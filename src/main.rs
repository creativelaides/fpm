// fpm — a Windows-native Rust wrapper around PyManager.
//
// This is the minimal entry point for PR1. Full CLI parsing and subcommand
// dispatch arrive in PR4 (cli.rs + main.rs rewrite).

#![allow(dead_code)] // Modules wired in PR2-PR4; not consumed by main() yet.

mod config;
mod error;
mod pymanager;

fn main() {
    println!("fpm {}", env!("CARGO_PKG_VERSION"));
}
