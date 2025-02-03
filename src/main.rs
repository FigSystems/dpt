// Improve rust's default behavior
#![allow(dead_code)]
#![allow(unused_variables)]

mod gen_pkg;
mod pkg;

fn main() {
    for arg in std::env::args() {
        match arg.as_str() {
            "--help" | "-h" => {
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
                return;
            }
            "--version" | "-v" => {
                println!("{}", env!("CARGO_PKG_VERSION"));
                return;
            }
            _ => {} // It will just be handeled as a positional argument
        }
    }
}
