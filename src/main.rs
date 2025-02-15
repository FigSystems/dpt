// Improve rust's default behavior
#![allow(dead_code)]

mod config;
mod env;
mod gen_pkg;
mod pkg;
mod pool;
mod repo;

pub const CONFIG_LOCATION: &str = "/etc/fpkg/";

use std::{path::PathBuf, process::exit};

use anyhow::{Context, Result};
use env::get_env_location;
use log::{debug, error, info};
use pkg::string_to_package;
use pool::{get_installed_packages, get_pool_location};
use repo::package_to_onlinepackage;
use users;

fn main() -> Result<()> {
    colog::init();
    let args = std::env::args().collect::<Vec<String>>();
    let argc = std::env::args().count();

    if users::get_current_uid() != 0 {
        error!("You need to be root to run this!");
        exit(exitcode::USAGE);
    }

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
            _ => {} // It will just be handeled as a positional argument
        }
    }

    if argc < 2 {
        error!("Not enough arguments!");
        print_help();
        exit(exitcode::USAGE);
    }

    match &args.get(1).unwrap() as &str {
        "gen-pkg" => {
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
            if argc < 3 {
                error!("Not enough arguments!");
                exit(exitcode::USAGE);
            }

            let installed_packages = get_installed_packages()?;

            for pkg in &args[2..] {
                let pkg = &string_to_package(pkg)?;

                let out_path =
                    PathBuf::from(package_to_onlinepackage(pkg, &installed_packages)?.url);
                let out_path = out_path.strip_prefix(get_pool_location())?;
                let out_path = get_env_location().join(out_path);

                env::generate_environment_for_package(
                    pkg,
                    &installed_packages,
                    &out_path,
                    &mut Vec::new(),
                )?;
            }
        }
        "fetch" => {
            if argc < 3 {
                error!("Not enough arguments!");
                exit(exitcode::USAGE);
            }

            match repo::fetch_file(&args[2]) {
                Err(e) => {
                    error!("{}", e);
                    exit(1);
                }
                Ok(v) => {
                    println!(
                        "{}",
                        std::str::from_utf8(&v[..])
                            .unwrap_or("Failed to convert fetched bytes to utf-8")
                    );
                }
            }
        }
        "list" => match repo::get_all_available_packages() {
            Ok(x) => {
                for pkg in x {
                    info!("{:#?}", pkg);
                }
            }
            Err(e) => {
                error!("{}", e.to_string());
                exit(exitcode::UNAVAILABLE)
            }
        },
        "install" => {
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
                let newest_version = match repo::newest_package_from_name(pkg, &packages) {
                    Ok(x) => x,
                    Err(e) => {
                        error!("Failed to find package {}: {}", pkg, e.to_string());
                        exit(exitcode::UNAVAILABLE);
                    }
                };
                let dependencies = repo::resolve_dependencies_for_package(
                    &packages,
                    pkg::Package {
                        name: newest_version.name,
                        version: newest_version.version,
                    },
                );
                if let Err(e) = dependencies {
                    error!("Failed to resolve dependencies for package {}: {}", pkg, e);
                    exit(exitcode::UNAVAILABLE);
                }
                let dependencies = dependencies.unwrap();

                for depencency in &dependencies {
                    repo::install_pkg(&depencency).context("Failed to install package")?;
                }

                debug!("Package {}:\n{:#?}", pkg, &dependencies);
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
