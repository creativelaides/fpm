// fpm — a Windows-native Rust wrapper around PyManager.
//
// Entry point: parse CLI args, dispatch recognized subcommands, or forward
// unrecognized args verbatim to `py.exe` (pass-through mode).
//
// Spec: fpm-core (all requirements)

mod cli;
mod commands;
mod config;
mod error;
pub mod services;
mod shell;
mod shim;
pub mod ui;
mod version_file;

use std::process::ExitCode;

use clap::Parser;
use error::FpmError;

use crate::cli::{Cli, Commands, ShellKind};

fn main() -> ExitCode {
    let cli = Cli::parse();

    if cli.version {
        let fpm_version = env!("CARGO_PKG_VERSION");
        // Get py --help output to extract launcher version
        let py_help = std::process::Command::new("py")
            .arg("--help")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
            .unwrap_or_else(|_| "Unavailable".to_string());
        let launcher_version = py_help.lines().next().unwrap_or("Unavailable");

        // Get py --version output
        let py_version = std::process::Command::new("py")
            .arg("--version")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
            .unwrap_or_else(|_| "Unavailable".to_string());

        let output =
            ui::formatters::print_detailed_version(fpm_version, launcher_version, &py_version);
        println!("{}", output);
        return ExitCode::SUCCESS;
    }

    match dispatch(cli) {
        Ok(code) => ExitCode::from(code as u8),
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(e.exit_code() as u8)
        }
    }
}

/// Dispatches the parsed CLI to the appropriate command handler.
///
/// If a recognized subcommand is present, runs it. Otherwise, forwards all
/// passthrough args to `py.exe`.
fn dispatch(cli: Cli) -> Result<i32, FpmError> {
    match cli.command {
        Some(Commands::Use {
            version,
            silent_if_unchanged,
        }) => {
            let mut ctx = commands::CommandContext::from_env()?;
            let session_dir = ctx.session_dir.ok_or_else(|| {
                FpmError::ShimError(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "FPM_MULTISHELL_PATH is not set — run 'fpm env --shell powershell' first",
                ))
            })?;
            let cwd = std::env::current_dir()
                .map_err(|e| FpmError::ShimError(std::io::Error::other(e)))?;

            let res = commands::use_cmd::run(
                &mut ctx.pymanager,
                version.as_deref(),
                silent_if_unchanged,
                &cwd,
                &session_dir,
            )?;

            if let Some(tag) = res {
                println!("{}", ui::formatters::format_use_success(&tag));
            }
            Ok(0)
        }

        Some(Commands::List) => {
            let mut ctx = commands::CommandContext::from_env()?;
            let runtimes = commands::list::run(&mut ctx.pymanager)?;
            print!("{}", ui::formatters::format_local_runtimes(&runtimes));
            Ok(0)
        }

        Some(Commands::ListRemote) => {
            let fetcher = crate::services::remote::DefaultRemoteFetcher::new()?;
            let (versions, offline) = commands::list_remote::run(&fetcher)?;
            // Assuming we pass `false` for show_pre for now (would need CLI flag)
            print!(
                "{}",
                ui::formatters::print_remote_versions(&versions, false, offline)
            );
            Ok(0)
        }

        Some(Commands::Current) => {
            let mut ctx = commands::CommandContext::from_env()?;
            let (active_tag, py_version_line) = commands::current::run(&mut ctx.pymanager)?;
            println!(
                "{}",
                ui::formatters::format_current_version(
                    active_tag.as_deref(),
                    py_version_line.as_deref()
                )
            );
            let exit_code = if py_version_line.is_none() { 1 } else { 0 };
            Ok(exit_code)
        }

        Some(Commands::Default {
            tag,
            unset,
            dry_run,
        }) => {
            let mut ctx = commands::CommandContext::from_env()?;
            let res = commands::default::run(
                &mut ctx.pymanager,
                tag.as_deref(),
                unset,
                dry_run,
                ctx.session_dir.as_deref(),
            )?;

            match res {
                commands::default::DefaultCommandResult::Read(tag) => {
                    println!("{}", ui::formatters::format_default_read(tag.as_deref()));
                }
                commands::default::DefaultCommandResult::Unset(removed) => {
                    println!("{}", ui::formatters::format_default_unset(removed));
                }
                commands::default::DefaultCommandResult::DryRun {
                    tag,
                    version,
                    install_dir,
                } => {
                    println!(
                        "{}",
                        ui::formatters::format_default_dry_run(&tag, &version, &install_dir)
                    );
                }
                commands::default::DefaultCommandResult::Set(tag) => {
                    println!("{}", ui::formatters::format_default_set_success(&tag));
                }
            }
            Ok(0)
        }

        Some(Commands::Env { shell, use_on_cd }) => match shell {
            ShellKind::Powershell => {
                let ctx = commands::CommandContext::from_env()?;
                let script = commands::env_cmd::run(&ctx.fpm_dir, use_on_cd)?;
                print!("{}", script);
                Ok(0)
            }
        },

        Some(Commands::Install { tag }) => commands::install::run(&tag),

        None => {
            // No recognized subcommand — forward to py.exe.
            commands::passthrough::run(&cli.passthrough_args)
        }
    }
}
