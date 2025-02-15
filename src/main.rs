mod config;
mod env;
mod gen_pkg;
mod pkg;
mod pool;
mod repo;

pub const CONFIG_LOCATION: &str = "/etc/fpkg/";

use std::{path::PathBuf, process::exit};

use anyhow::Result;
use env::{generate_environment_for_package, pool_to_env_location};
use log::{error, info};
use pkg::{onlinepackage_to_package, string_to_package, Package};
use pool::{get_installed_packages, package_to_pool_location};
use repo::{install_pkg_and_dependencies, package_to_onlinepackage, OnlinePackage};
use users;

fn main() -> Result<()> {
    colog::init();
    let args = std::env::args().collect::<Vec<String>>();
    let argc = std::env::args().count();

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
            command_requires_root();
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
        "list" => {
            command_requires_root();
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
            command_requires_root();
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

                let pkgs = repo::get_all_available_packages()?;
                let mut done_list = Vec::<OnlinePackage>::new();

                install_pkg_and_dependencies(&newest_version, &pkgs, &mut done_list)?;

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
        // Add 'rm'
        cmd => {
            error!("Unknown command {}!", cmd);
            print_help();
            exit(exitcode::USAGE);
        }
    }

    info!("Done!");
    Ok(())
}

fn command_requires_root() {
    if users::get_current_uid() != 0 {
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
