mod config;
mod env;
mod gen_pkg;
mod pkg;
mod pool;
mod repo;
mod run;

pub const CONFIG_LOCATION: &str = "/etc/fpkg/";

use std::{path::PathBuf, process::exit};

use anyhow::{Context, Result};
use env::{generate_environment_for_package, pool_to_env_location};
use log::{error, info, warn};
use pkg::{onlinepackage_to_package, string_to_package, Package};
use pool::{get_installed_packages, package_to_pool_location};
use repo::{
    install_pkg_and_dependencies, newest_package_from_name, package_to_onlinepackage, OnlinePackage,
};
use uzers::{
    self, get_current_uid, get_effective_uid,
    switch::{set_current_uid, set_effective_uid},
};

fn main() -> Result<()> {
    colog::init();
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

    if args[1] != "chroot-not-intended-for-interactive-use" && args[1] != "run" {
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
            if argc < 3 {
                error!("Not enough arguments!");
                exit(exitcode::USAGE);
            }
            let path = PathBuf::from(&format!("{}", &args[2]));
            let err = gen_pkg::gen_pkg(&path, &path.clone().with_extension("fpkg"));
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

                let out_path =
                    PathBuf::from(package_to_onlinepackage(pkg, &installed_packages)?.url);
                let out_path = pool_to_env_location(&out_path)?;

                env::generate_environment_for_package(
                    pkg,
                    &installed_packages,
                    &out_path,
                    &mut Vec::new(),
                )?;
            }
        }
        "list" => {
            command_requires_root_uid();
            match repo::get_all_available_packages() {
                Ok(x) => {
                    for pkg in x {
                        info!("{:#?}", pkg);
                    }
                }
                Err(e) => {
                    error!("{}", e.to_string());
                    exit(exitcode::UNAVAILABLE)
                }
            }
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
                        error!("Failed to find package {}: {}", pkg, e.to_string());
                        exit(exitcode::UNAVAILABLE);
                    }
                };
                let version = package_to_onlinepackage(&version, &packages)?;

                let pkgs = repo::get_all_available_packages()?;
                let mut done_list = Vec::<OnlinePackage>::new();

                install_pkg_and_dependencies(&version, &pkgs, &mut done_list)?;

                let pkgs = get_installed_packages()?;

                for done in done_list {
                    generate_environment_for_package(
                        &onlinepackage_to_package(&done),
                        &pkgs,
                        &pool_to_env_location(&package_to_pool_location(
                            &onlinepackage_to_package(&done),
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
            let pkg = friendly_str_to_package(&args[2], &get_installed_packages()?)?;
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
            run::run_pkg(&pkg, uid, run_args)?;
        }
        "chroot-not-intended-for-interactive-use" => {
            command_requires_root_uid();
            if argc < 5 {
                error!("Not enough arguments!");
                exit(exitcode::USAGE);
            }
            let uid: u32 = args[3].parse()?;
            let prev_dir = std::env::current_dir()?;
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
                exit_code
                    .code()
                    .ok_or(anyhow::anyhow!("Failed to get process exit code!"))?,
            );
        }
        "uninstall" | "rm" => {}
        cmd => {
            error!("Unknown command {}!", cmd);
            print_help();
            exit(exitcode::USAGE);
        }
    }

    info!("Done!");
    Ok(())
}

fn friendly_str_to_package(arg: &str, pkgs: &Vec<OnlinePackage>) -> Result<Package> {
    let pkg = match string_to_package(arg) {
        Ok(x) => x,
        Err(_) => onlinepackage_to_package(&newest_package_from_name(arg, pkgs)?),
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
    install/add    Installs packages
    uninstall/rm   Uninstalls packages
    run            Runs a program
    gen-pkg        Generates a package from a directory
    build-env      Build or refreshes a packages environment"
    );
}
