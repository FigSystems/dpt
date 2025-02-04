// Improve rust's default behavior
#![allow(dead_code)]
#![allow(unused_variables)]

mod gen_pkg;
mod pkg;

use std::path::Path;

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
            info!("gen-pkg");
            if argc < 3 {
                error!("Not enough arguments!");
                std::process::exit(exitcode::USAGE);
            }
            let path = std::path::PathBuf::from(&format!("{}", &args[2]));
            info!("{}", &path.display());
            let err = gen_pkg::gen_pkg(&path, &path.clone().join(Path::new("fpkg/pkg.kdl")));
            if let Err(e) = err {
                error!("{}", e);
                std::process::exit(1);
            }
        }
        cmd => {
            error!("Unknown command {}!", cmd);
            print_help();
            std::process::exit(exitcode::USAGE);
        }
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
