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
mod pymanager;
mod shell;
mod shim;
mod version_file;

use std::process::ExitCode;

use clap::Parser;
use error::FpmError;

use crate::cli::{Cli, Commands, ShellKind};

fn main() -> ExitCode {
    let cli = Cli::parse();

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
            commands::use_cmd::run(
                &mut ctx.pymanager,
                version.as_deref(),
                silent_if_unchanged,
                &cwd,
                &session_dir,
            )
        }

        Some(Commands::List) => {
            let mut ctx = commands::CommandContext::from_env()?;
            commands::list::run(&mut ctx.pymanager)
        }

        Some(Commands::Current) => {
            let mut ctx = commands::CommandContext::from_env()?;
            commands::current::run(&mut ctx.pymanager)
        }

        Some(Commands::Default {
            tag,
            unset,
            dry_run,
        }) => {
            let mut ctx = commands::CommandContext::from_env()?;
            // Pass session_dir through as Option; default::run only requires it
            // on the set path (validate → require session_dir → write →
            // activate). Read, unset, and dry-run do not need it, so we must
            // NOT error here when FPM_MULTISHELL_PATH is unset (spec: read/unset
            // must work outside an fpm-integrated shell).
            commands::default::run(
                &mut ctx.pymanager,
                tag.as_deref(),
                unset,
                dry_run,
                ctx.session_dir.as_deref(),
            )
        }

        Some(Commands::Env { shell, use_on_cd }) => match shell {
            ShellKind::Powershell => {
                let ctx = commands::CommandContext::from_env()?;
                commands::env_cmd::run(&ctx.fpm_dir, use_on_cd)
            }
        },

        Some(Commands::Install { tag }) => commands::install::run(&tag),

        None => {
            // No recognized subcommand — forward to py.exe.
            commands::passthrough::run(&cli.passthrough_args)
        }
    }
}
