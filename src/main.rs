// Improve rust's default behavior
#![allow(dead_code)]
#![allow(unused_variables)]

mod gen_pkg;
mod pkg;

use log::error;

fn main() {
    for arg in std::env::args() {
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

    if std::env::args().count() < 2 {
        error!("Not enough arguments!");
        print_help();
        std::process::exit(exitcode::USAGE);
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
