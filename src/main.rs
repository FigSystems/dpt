// Improve rust's default behavior
#![allow(dead_code)]

mod config;
mod gen_pkg;
mod pkg;
mod pool;
mod repo;

pub const CONFIG_LOCATION: &str = "/etc/fpkg/";

use log::{error, info};

fn main() {
    colog::init();
    let args = std::env::args().collect::<Vec<String>>();
    let argc = std::env::args().count();

    for arg in &args {
        match arg.as_str() {
            "--help" | "-h" => {
                print_help();
                return;
            }
            "--version" | "-v" => {
                println!("{}", env!("CARGO_PKG_VERSION"));
                return;
            }
            _ => {} // It will just be handeled as a positional argument
        }
    }

    if argc < 2 {
        error!("Not enough arguments!");
        print_help();
        std::process::exit(exitcode::USAGE);
    }

    match &args.get(1).unwrap() as &str {
        "gen-pkg" => {
            if argc < 3 {
                error!("Not enough arguments!");
                std::process::exit(exitcode::USAGE);
            }
            let path = std::path::PathBuf::from(&format!("{}", &args[2]));
            let err = gen_pkg::gen_pkg(&path, &path.clone().with_extension("fpkg"));
            if let Err(e) = err {
                error!("{}", e);
                std::process::exit(1);
            }
        }
        "fetch" => {
            if argc < 3 {
                error!("Not enough arguments!");
                std::process::exit(exitcode::USAGE);
            }

            match repo::fetch_file(args[2].to_string()) {
                Err(e) => {
                    error!("{}", e);
                    std::process::exit(1);
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
        cmd => {
            error!("Unknown command {}!", cmd);
            print_help();
            std::process::exit(exitcode::USAGE);
        }
    }

    info!("Done!");
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
