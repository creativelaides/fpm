// fpm — a Windows-native Rust wrapper around PyManager.
//
// This is the minimal entry point for PR1. Full CLI parsing and subcommand
// dispatch arrive in PR4 (cli.rs + main.rs rewrite).

mod error;
mod config;

fn main() {
    println!("fpm {}", env!("CARGO_PKG_VERSION"));
}