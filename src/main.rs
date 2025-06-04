// Avoid musl's default allocator due to slower performance
// https://nickb.dev/blog/default-musl-allocator-considered-harmful-to-performance
#[cfg(target_env = "musl")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

mod base;
mod config;
mod dpt_file;
mod env;
mod pkg;
mod repo;
mod run;
mod store;

pub const PROGRESS_STYLE_BYTES: &str =
    "{msg} [{wide_bar:.green/blue}] {bytes}/{total_bytes} ({eta})";
pub const PROGRESS_STYLE: &str =
    "{msg} [{wide_bar:.green/blue}] {human_pos}/{human_len} ({eta})";
pub const PROGRESS_CHARS: &str = "##-";

use std::{
    fs::write,
    io::Read,
    path::{Path, PathBuf},
    process::exit,
    str::FromStr,
    sync::{Arc, Mutex},
};

use base::rebuild_base;
use dpt_file::read_dpt_file;
use indicatif::ProgressIterator;

use anyhow::{anyhow, Context, Result};
use colog::format::CologStyle;
use log::{error, warn, Level};
use pkg::{
    decompress_pkg_read, get_package_config, string_to_package, Package,
};
use repo::{
    get_all_available_packages, install_pkgs_and_dependencies,
    newest_package_from_name, package_to_onlinepackage, OnlinePackage,
    RepositoryIndex,
};
use run::run_multiple_packages;
use store::{get_dpt_dir, get_installed_packages, get_package_for_bin};
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
    builder.filter(Some("pubgrub"), log::LevelFilter::Warn);
    builder.filter(Some("reqwest"), log::LevelFilter::Warn);
    builder.init();

    let args = std::env::args().collect::<Vec<String>>();
    let argc = std::env::args().count();

    if get_effective_uid() != 0 {
        error!("dpt needs to be installed setuid or run as root!");
        exit(exitcode::USAGE);
    }

    let me = args[0]
        .split("/")
        .last()
        .context("Failed to get last part of $0 !")?;
    if me != "dpt"
        && args.get(1)
            != Some(&"chroot-not-intended-for-interactive-use".to_string())
    {
        let packages = get_installed_packages()?;
        let pkg = get_package_for_bin(me, &packages)?;
        let uid = get_current_uid();
        set_current_uid(0)?;
        exit(run::run_pkg(
            &pkg.to_package(),
            uid,
            args[1..].to_vec(),
            Some(me),
            false,
        )?);
    }

    if argc < 2 {
        error!("Not enough arguments!");
        print_help();
        exit(exitcode::USAGE);
    }

    if args[1] != "chroot-not-intended-for-interactive-use"
        && args[1] != "run"
        && args[1] != "run-multi"
        && args[1] != "dev-env"
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
        "rebuild" => {
            command_requires_root_uid();
            let dpt = read_dpt_file()?;
            let repo_packages = get_all_available_packages()?;

            let done_list = install_pkgs_and_dependencies(
                &dpt.packages
                    .iter()
                    .map(|package| {
                        newest_package_from_name(&package.name, &repo_packages)
                            .context(anyhow!(
                                "Package {} is not found in repository!",
                                package
                            ))
                            .unwrap()
                    })
                    .collect(),
                &repo_packages,
                false,
            )?;

            rebuild_base(&dpt).context("Failed to build base!")?;

            let mut dpt_lock = dpt.clone();

            let done_list = remove_duplicates(done_list);

            dpt_lock.packages = Vec::with_capacity(done_list.len());

            for x in done_list {
                dpt_lock.packages.push(Package::new(x.name, x.version));
            }

            write(
                get_dpt_dir().join("dpt.lock"),
                ron::ser::to_string_pretty(
                    &dpt_lock,
                    ron::ser::PrettyConfig::default(),
                )?,
            )
            .context("Failed to write dpt.lock file")?;
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
                warn!("When running `dpt run` using sudo, the inner package gets run as root. Use setuid instead of sudo to run it as yourself");
            }
            set_current_uid(0)?;
            let mut run_args = Vec::<String>::new();
            if argc > 3 {
                for arg in &args[3..] {
                    run_args.push(arg.clone());
                }
            }
            exit(run::run_pkg(&pkg, uid, run_args, None, false)?);
        }
        "run-multi" => {
            if argc < 3 {
                error!("Not enough arguments!");
                exit(exitcode::USAGE);
            }
            let packages = get_installed_packages()?;
            let mut packages_to_run = Vec::<Package>::new();
            let mut previous_was_cmd = false;
            let mut cmd: Option<&str> = None;
            for pkg in &args[2..] {
                if pkg == "--" {
                    break;
                }
                if previous_was_cmd {
                    cmd = Some(pkg);
                    continue;
                }
                if pkg == "--cmd" || pkg == "-c" {
                    previous_was_cmd = true;
                    continue;
                } else {
                    previous_was_cmd = false;
                }

                let version = friendly_str_to_package(pkg, &packages)
                    .context(anyhow!("Package `{}` not found!", pkg))?;
                packages_to_run.push(version);
            }
            let uid = get_current_uid();
            if uid == 0 && std::env::var("SUDO_USER").is_ok() {
                warn!("When running `dpt run` using sudo, the inner package gets run as root. Use setuid instead of sudo to run it as yourself");
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
            exit(run_multiple_packages(
                &packages_to_run,
                uid,
                run_args,
                cmd,
                false,
            )?);
        }
        "dev-env" => {
            if argc < 3 {
                error!("Not enough arguments!");
                exit(exitcode::USAGE);
            }

            let uid = get_current_uid();
            if uid == 0 && std::env::var("SUDO_USER").is_ok() {
                warn!("When running `dpt dev-env` using sudo, the inner package gets run as root. Use setuid instead of sudo to run it as yourself");
            }
            set_current_uid(0)?;

            let packages = get_all_available_packages()?;
            let mut packages_to_run = Vec::<Package>::new();
            let mut previous_was_cmd = false;
            let mut cmd: Option<&str> = None;
            for pkg in &args[2..] {
                if pkg == "--" {
                    break;
                }
                if previous_was_cmd {
                    cmd = Some(pkg);
                    continue;
                }
                if pkg == "--cmd" || pkg == "-c" {
                    previous_was_cmd = true;
                    continue;
                } else {
                    previous_was_cmd = false;
                }

                let version = friendly_str_to_package(pkg, &packages)
                    .context(anyhow!("Package `{}` not found!", pkg))?;
                packages_to_run.push(version);
            }

            install_pkgs_and_dependencies(
                &packages_to_run
                    .iter()
                    .map(|package| {
                        package_to_onlinepackage(&package, &packages)
                            .context(anyhow!(
                                "Package {} is not found on the filesystem!",
                                package.name
                            ))
                            .unwrap()
                    })
                    .collect(),
                &packages,
                false,
            )?;

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
            exit(run_multiple_packages(
                &packages_to_run,
                uid,
                run_args,
                cmd,
                true,
            )?);
        }
        "gen-index" => {
            set_effective_uid(get_current_uid())?;
            let mut out = RepositoryIndex { packages: vec![] };

            let dpts = walkdir::WalkDir::new(".")
                .follow_links(true)
                .into_iter()
                .filter(|x| {
                    x.is_ok()
                        && x.as_ref().unwrap().path().extension().is_some()
                        && x.as_ref()
                            .unwrap()
                            .path()
                            .extension()
                            .unwrap()
                            .to_str()
                            == "dpt".into()
                })
                .map(|x| x.unwrap().path().to_owned())
                .collect::<Vec<PathBuf>>();
            for ent in dpts.into_iter().progress().with_style(
                indicatif::ProgressStyle::default_bar()
                    .template(PROGRESS_STYLE)?
                    .progress_chars(PROGRESS_CHARS),
            ) {
                let mut pkg = decompress_pkg_read(std::fs::File::open(&ent)?)?;
                for pkg_ent in pkg.entries()? {
                    let mut pkg_ent = pkg_ent?;
                    if pkg_ent.path()? == Path::new("dpt/pkg.ron")
                        || pkg_ent.path()? == Path::new("./dpt/pkg.ron")
                    {
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

                        out.packages.push(OnlinePackage {
                            name: cfg.name,
                            version: cfg.version,
                            url: ent_path,
                            depends: cfg.depends,
                        });
                        break;
                    }
                }
            }

            std::fs::write(
                "index.ron",
                ron::ser::to_string_pretty(
                    &out,
                    ron::ser::PrettyConfig::default(),
                )?,
            )?;
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
            let p: Arc<Mutex<std::process::Child>> =
                Arc::new(Mutex::new(p.spawn()?));

            let proc_arc_clone = Arc::clone(&p);
            ctrlc::set_handler(move || {
                let l = proc_arc_clone.lock();
                if let Ok(mut x) = l {
                    let _ = x.kill();
                };
            })
            .context("Failed to register ctrlc singal handler")?;
            let l = p.lock();
            let exit_code = if let Ok(mut x) = l {
                x.wait()?.code().unwrap_or(89)
            } else {
                243
            };

            exit(exit_code);
        }
        cmd => {
            error!("Unknown command {}!", cmd);
            print_help();
            exit(exitcode::USAGE);
        }
    }

    Ok(())
}

fn remove_duplicates<T: Eq + std::hash::Hash + Clone>(mut l: Vec<T>) -> Vec<T> {
    let mut seen = std::collections::HashSet::new();
    l.retain(|c| seen.insert(c.clone()));
    l
}

fn friendly_str_to_package(
    arg: &str,
    pkgs: &Vec<OnlinePackage>,
) -> Result<Package> {
    let pkg = match string_to_package(arg) {
        Ok(x) => {
            if package_to_onlinepackage(&x, pkgs).is_ok() {
                x
            } else {
                newest_package_from_name(arg, pkgs)?.to_package()
            }
        }
        Err(_) => newest_package_from_name(arg, pkgs)?.to_package(),
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
        "Usage: dpt command [additional arguments]

Dpt, package management, done right.

Commands:
    rebuild         Rebuilds the environment according to the dpt file.
    run             Runs a program
    run-multi       Runs the first program specified in an env with the rest
    gen-index       Generates the index file for a package repository at PWD"
    );
}
