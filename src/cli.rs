// CLI argument parsing via clap derive.
//
// Spec: fpm-core Subcommand Routing
//
// Top-level `Cli` struct with an optional `Commands` subcommand enum. When no
// recognized subcommand is present, the remaining args are forwarded verbatim
// to `py.exe` (pass-through mode).

use clap::{Parser, Subcommand};

/// fpy — a Windows-native Rust wrapper around PyManager.
#[derive(Parser, Debug)]
#[command(
    name = "fpy",
    disable_version_flag = true,
    about = "Per-session Python version switching via PyManager"
)]
pub struct Cli {
    /// Print version information
    #[arg(short = 'V', long = "version")]
    pub version: bool,

    /// Subcommand to run. If absent or unrecognized, args forward to py.exe.
    #[command(subcommand)]
    pub command: Option<Commands>,

    // Catch-all for pass-through: clap stores trailing args here when no
    // subcommand matches. We use `allow_external_subcommands` semantics by
    // collecting remaining args into this Vec.
    /// Remaining args forwarded to py.exe when no subcommand matches.
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub passthrough_args: Vec<String>,
}

/// Recognized fpm subcommands.
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Switch to a Python version for this session.
    Use {
        /// Version tag to switch to (e.g. "3.14"). If omitted, resolves from
        /// .python-version or pyproject.toml.
        version: Option<String>,
        /// Suppress output when the version is already active.
        #[arg(long)]
        silent_if_unchanged: bool,
    },

    /// List installed Python runtimes.
    List,

    /// List available Python versions from python.org.
    ListRemote {
        /// Include pre-release versions (alpha, beta, rc).
        #[arg(long)]
        pre: bool,
    },

    /// Print the currently active Python version.
    Current,

    /// Read or set the default Python version (writes pymanager.json).
    Default {
        /// Version tag to set as default. If omitted, prints the current default.
        tag: Option<String>,
        /// Remove the default_tag from pymanager.json (no session change).
        #[arg(long, conflicts_with = "tag")]
        unset: bool,
        /// Validate and preview the would-be default without side effects.
        /// Requires a tag; mutually exclusive with --unset.
        #[arg(long, requires = "tag", conflicts_with = "unset")]
        dry_run: bool,
    },

    /// Emit a shell integration script for the given shell.
    Env {
        /// Target shell.
        #[arg(long)]
        shell: ShellKind,
        /// Emit a Set-Location hook for automatic use-on-cd.
        #[arg(long)]
        use_on_cd: bool,
    },

    /// Install a Python version via `py install <tag>`.
    Install {
        /// Version tag to install.
        tag: String,
    },
}

/// Supported shell backends for `fpm env`.
#[derive(clap::ValueEnum, Debug, Clone)]
pub enum ShellKind {
    /// PowerShell (Windows PowerShell 5+ and PowerShell 6+).
    Powershell,
}
