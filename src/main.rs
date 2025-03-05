mod config;
mod env;
mod gen_pkg;
mod info;
mod pkg;
mod repo;
mod run;
mod store;
mod uninstall;

pub const CONFIG_LOCATION: &str = "/etc/fpkg/";

use std::{
    io::Read,
    path::{Path, PathBuf},
    process::exit,
    str::FromStr,
};

use anyhow::{anyhow, Context, Result};
use colog::format::CologStyle;
use env::{generate_environment_for_package, package_to_env_location};
use log::{error, info, warn, Level};
use pkg::{
    decompress_pkg_read, get_package_config, onlinepackage_to_package,
    string_to_package, Package,
};
use repo::{
    install_pkg_and_dependencies, newest_package_from_name,
    package_to_onlinepackage, OnlinePackage,
};
use run::run_multiple_packages;
use store::get_installed_packages;
use uninstall::uninstall_package_and_deps;
use uzers::{
    self, get_current_uid, get_effective_uid,
    switch::{set_current_uid, set_effective_uid},
};

pub struct CustomLevelToken;

// implement CologStyle on our type, and override `level_token`
impl CologStyle for CustomLevelToken {
    fn level_token(&self, level: &Level) -> &str {
        match *level {
            Level::Error => "E",
            Level::Warn => "W",
            Level::Info => "+",
            Level::Debug => "D",
            Level::Trace => "T",
        }
    }
}

fn main() -> Result<()> {
    let mut builder = colog::basic_builder();
    builder.format(colog::formatter(CustomLevelToken));
    if cfg!(debug_assertions) {
        builder.filter(None, log::LevelFilter::Debug);
    } else {
        builder.filter(None, log::LevelFilter::Info);
    }
    builder.init();
    let args = std::env::args().collect::<Vec<String>>();
    let argc = std::env::args().count();

    if argc < 2 {
        error!("Not enough arguments!");
        print_help();
        exit(exitcode::USAGE);
    }

    if get_effective_uid() != 0 {
        error!("FPKG needs to be installed setuid or run as root!");
        exit(exitcode::USAGE);
    }

    if args[1] != "chroot-not-intended-for-interactive-use"
        && args[1] != "run"
        && args[1] != "run-multi"
    {
        for arg in &args {
            match arg.as_str() {
                "--help" | "-h" => {
                    print_help();
                    return Ok(());
                }
                "--version" | "-v" => {
                    println!("{}", env!("CARGO_PKG_VERSION"));
                    return Ok(());
                }
                _ => {} // It will just be handled as a positional argument
            }
        }
    }

    match &args.get(1).unwrap() as &str {
        "gen-pkg" => {
            // You are allowed to generate a package as non-root
            set_effective_uid(get_current_uid())?;
            if argc < 3 {
                error!("Not enough arguments!");
                exit(exitcode::USAGE);
            }

            let path = PathBuf::from(&format!("{}", &args[2]));
            let mut out = PathBuf::from_str(
                &(path
                    .clone()
                    .to_str()
                    .ok_or(anyhow!("Invalid path '{}'!", path.display()))?
                    .to_string()
                    + ".fpkg"),
            )?;

            if argc > 3 {
                out = PathBuf::from_str(&args[3])?;
            }
            let err = gen_pkg::gen_pkg(&path, &out);
            if let Err(e) = err {
                error!("{}", e);
                exit(1);
            }
        }
        "build-env" => {
            command_requires_root_uid();
            if argc < 3 {
                error!("Not enough arguments!");
                exit(exitcode::USAGE);
            }

            let installed_packages = get_installed_packages()?;

            for pkg in &args[2..] {
                let pkg = &string_to_package(pkg)?;

                let out_path = package_to_env_location(&pkg)?;

                env::generate_environment_for_package(
                    pkg,
                    &installed_packages,
                    &out_path,
                    &mut Vec::new(),
                )?;
            }
        }
        "list" => {
            set_effective_uid(get_current_uid())?;
            let mut message = String::new();
            match repo::get_all_available_packages() {
                Ok(x) => {
                    for pkg in x {
                        // info!("{}", pkg);
                        message.push_str(&format!(
                            "\n{}-{}",
                            pkg.name, pkg.version
                        ));
                    }
                    info!("{}\n", message);
                }
                Err(e) => {
                    error!("{}", e);
                    exit(exitcode::UNAVAILABLE)
                }
            }
        }
        "list-installed" => {
            command_requires_root_uid();
            let mut message = String::new();
            let packages = store::get_installed_packages()?;
            for pkg in packages {
                message.push_str(&format!("\n{}-{}", pkg.name, pkg.version));
            }
            info!("{}\n", message);
        }
        "install" | "add" => {
            command_requires_root_uid();
            if argc < 3 {
                error!("Not enough arguments!");
                exit(exitcode::USAGE);
            }
            let packages = match repo::get_all_available_packages() {
                Ok(x) => x,
                Err(e) => {
                    error!("{}", e.to_string());
                    exit(exitcode::UNAVAILABLE);
                }
            };

            for pkg in &args[2..] {
                let version = match friendly_str_to_package(pkg, &packages) {
                    Ok(x) => x,
                    Err(e) => {
                        error!(
                            "Failed to find package {}: {}",
                            pkg,
                            e.to_string()
                        );
                        exit(exitcode::UNAVAILABLE);
                    }
                };
                let version = package_to_onlinepackage(&version, &packages)?;

                let mut done_list = Vec::<OnlinePackage>::new();

                install_pkg_and_dependencies(
                    &version,
                    &packages,
                    &mut done_list,
                    true,
                )?;

                let pkgs = get_installed_packages()?;

                for done in done_list {
                    generate_environment_for_package(
                        &onlinepackage_to_package(&done),
                        &pkgs,
                        &package_to_env_location(&onlinepackage_to_package(
                            &done,
                        ))?,
                        &mut Vec::<Package>::new(),
                    )?;
                }
            }
        }
        "run" => {
            if argc < 3 {
                error!("Not enough arguments!");
                exit(exitcode::USAGE);
            }
            let pkg =
                friendly_str_to_package(&args[2], &get_installed_packages()?)?;
            let uid = get_current_uid();
            if uid == 0 && std::env::var("SUDO_USER").is_ok() {
                warn!("When running `fpkg run` using sudo, the inner package gets run as root. Use setuid instead of sudo to run it as yourself");
            }
            set_current_uid(0)?;
            let mut run_args = Vec::<String>::new();
            if argc > 3 {
                for arg in &args[3..] {
                    run_args.push(arg.clone());
                }
            }
            exit(run::run_pkg(&pkg, uid, run_args, None)?);
        }
        "run-multi" => {
            if argc < 3 {
                error!("Not enough arguments!");
                exit(exitcode::USAGE);
            }
            let packages = get_installed_packages()?;
            let mut packages_to_run = Vec::<Package>::new();
            for pkg in &args[2..] {
                if pkg == "--" {
                    break;
                }
                let version = friendly_str_to_package(pkg, &packages)?;
                packages_to_run.push(version);
            }
            let uid = get_current_uid();
            if uid == 0 && std::env::var("SUDO_USER").is_ok() {
                warn!("When running `fpkg run` using sudo, the inner package gets run as root. Use setuid instead of sudo to run it as yourself");
            }
            set_current_uid(0)?;

            let mut run_args = Vec::<String>::new();
            if argc > 3 {
                let mut active = false;
                for arg in &args[3..] {
                    if active {
                        run_args.push(arg.clone());
                    } else {
                        if arg == "--" {
                            active = true;
                        }
                    }
                }
            }
            exit(run_multiple_packages(&packages_to_run, uid, run_args)?);
        }
        "gen-index" => {
            set_effective_uid(get_current_uid())?;
            let mut out_str = String::new();

            for ent in std::fs::read_dir(".")? {
                let ent = ent?.path();
                match ent.extension() {
                    Some(x) => {
                        if x.to_str() != Some("fpkg") {
                            continue;
                        }
                    }
                    _ => {
                        continue;
                    }
                }
                let mut pkg = decompress_pkg_read(std::fs::File::open(&ent)?)?;
                for pkg_ent in pkg.entries()? {
                    let mut pkg_ent = pkg_ent?;
                    if pkg_ent.path()? == Path::new("fpkg/pkg.kdl") {
                        let mut buf = String::new();
                        pkg_ent.read_to_string(&mut buf)?;
                        let cfg = get_package_config(&buf)?;

                        let ent_path = match ent.strip_prefix("./") {
                            Ok(x) => x,
                            Err(_) => &ent,
                        };
                        let ent_path = ent_path
                            .to_str()
                            .ok_or(anyhow!(
                                "Failed to convert file path into a str"
                            ))?
                            .to_string();

                        out_str.push_str(&format!(
                            "package name=\"{}\" version=\"{}\" path=\"{}\"",
                            cfg.name.clone(),
                            cfg.version.clone(),
                            ent_path
                        ));

                        if cfg.depends.is_empty() {
                            out_str.push('\n');
                        } else {
                            // We have dependencies! Yay!
                            out_str.push_str(&format!(" {{\n"));
                            for depend in cfg.depends {
                                out_str.push_str(&format!(
                                    "    depends \"{}\"{}",
                                    depend.name,
                                    if &depend.version_mask != "" {
                                        format!(
                                            "{{\n        version_mask {}",
                                            depend.version_mask
                                        )
                                    } else {
                                        "\n".to_string()
                                    }
                                ));
                            }
                            out_str.push_str("}\n");
                        }
                    }
                }
            }

            print!("{}", &out_str);
        }
        "chroot-not-intended-for-interactive-use" => {
            command_requires_root_uid();
            if argc < 5 {
                error!("Not enough arguments!");
                exit(exitcode::USAGE);
            }
            let uid: u32 = args[3].parse()?;
            let prev_dir =
                std::env::current_dir().unwrap_or(PathBuf::from_str("/")?);
            std::env::set_current_dir(&args[2])?;
            std::os::unix::fs::chroot(".")?;

            if prev_dir.is_dir() {
                std::env::set_current_dir(prev_dir)?;
            } else {
                std::env::set_current_dir("/")?;
            }
            set_current_uid(uid)?;
            set_effective_uid(get_current_uid())?;
            let mut p = std::process::Command::new(&args[4]);
            if argc > 5 {
                for a in &args[5..] {
                    p.arg(a);
                }
            }

            let exit_code = p
                .spawn()
                .context("In spawning")?
                .wait()
                .context("In waiting")?;
            exit(
                exit_code.code().ok_or(anyhow::anyhow!(
                    "Failed to get process exit code!"
                ))?,
            );
        }
        "uninstall" | "rm" => {
            command_requires_root_uid();
            if argc < 3 {
                error!("Not enough arguments!");
                exit(exitcode::USAGE);
            }

            let packages = get_installed_packages()?;

            for pkg in &args[2..] {
                uninstall_package_and_deps(&friendly_str_to_package(
                    &pkg, &packages,
                )?)?;
            }
        }
        cmd => {
            error!("Unknown command {}!", cmd);
            print_help();
            exit(exitcode::USAGE);
        }
    }

    info!("Done!");
    Ok(())
}

fn friendly_str_to_package(
    arg: &str,
    pkgs: &Vec<OnlinePackage>,
) -> Result<Package> {
    let pkg = match string_to_package(arg) {
        Ok(x) => x,
        Err(_) => {
            onlinepackage_to_package(&newest_package_from_name(arg, pkgs)?)
        }
    };
    Ok(pkg)
}

fn command_requires_root_uid() {
    if uzers::get_current_uid() != 0 {
        error!("You need to be root to run this!");
        exit(exitcode::USAGE);
    }
}

fn print_help() {
    println!(
        "Usage: fpkg command [additional arguments]

Fpkg, package management, done right.

Commands:
    install/add     Installs packages
    uninstall/rm    Uninstalls packages
    list            Lists available packages from the repo
    list-installed  Lists all installed packages
    run             Runs a program
    run-multi       Runs the first program specified in an env with the rest
    gen-pkg         Generates a package from a directory
    build-env       Build or refreshes a packages environment
    gen-index       Generates the index file for a package repository at CWD"
    );
}
