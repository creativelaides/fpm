// Command dispatch infrastructure.
//
// Spec: fpm-core
//
// `CommandContext` carries everything a command handler needs: config paths,
// the PyManager instance (for runtime queries), and the per-session directory
// (read from the FPM_MULTISHELL_PATH env var set by `fpm env`).
//
// Each submodule implements one fpm subcommand. The dispatch from clap args to
// the correct handler lives in `main.rs`.

use std::path::PathBuf;

use crate::config;
use crate::error::FpmError;
use crate::pymanager::PyManager;

pub mod current;
pub mod default;
pub mod install;
pub mod list;
pub mod passthrough;

/// Shared context passed to every command handler.
///
/// `config_dir` and `pymanager_json_path` are resolved at startup. `pymanager`
/// is the real PyManager client (caches `py list` lazily). `session_dir` is
/// read from `FPM_MULTISHELL_PATH` — only set inside a shell that ran
/// `fpm env`; commands that need it (e.g. `use`) error if it's missing.
pub struct CommandContext {
    /// fpm data directory (FPM_DIR or %LocalAppData%\fpm).
    pub fpm_dir: PathBuf,
    /// Path to pymanager.json (%AppData%\Python\pymanager.json).
    pub pymanager_json_path: PathBuf,
    /// PyManager client — lazily spawns `py` and caches results.
    pub pymanager: PyManager,
    /// Per-session multishell directory from FPM_MULTISHELL_PATH.
    /// `None` when not running inside an fpm-integrated shell.
    pub session_dir: Option<PathBuf>,
}

impl CommandContext {
    /// Builds a `CommandContext` from the live environment.
    ///
    /// Resolves `fpm_dir` and `pymanager_json_path` via `config`, and reads
    /// `FPM_MULTISHELL_PATH` to find the per-session shim directory.
    pub fn from_env() -> Result<Self, FpmError> {
        let fpm_dir = config::fpm_dir().map_err(|e| FpmError::ConfigError(e.to_string()))?;
        let pymanager_json_path =
            config::pymanager_json_path().map_err(|e| FpmError::ConfigError(e.to_string()))?;

        let session_dir = std::env::var_os(config::FPM_MULTISHELL_PATH_ENV).map(PathBuf::from);

        Ok(CommandContext {
            fpm_dir,
            pymanager_json_path,
            pymanager: PyManager::new(),
            session_dir,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    /// Helper: run a closure with an env var set, then restore it.
    fn with_env<F>(key: &str, value: Option<&str>, f: F)
    where
        F: FnOnce(),
    {
        let original = env::var_os(key);
        match value {
            Some(v) => env::set_var(key, v),
            None => env::remove_var(key),
        }
        f();
        match original {
            Some(v) => env::set_var(key, v),
            None => env::remove_var(key),
        }
    }

    #[test]
    fn from_env_resolves_fpm_dir() {
        let temp = tempfile::tempdir().unwrap();
        let temp_path = temp.path().to_path_buf();

        with_env(
            config::FPM_DIR_ENV,
            Some(temp_path.to_str().unwrap()),
            || {
                let ctx = CommandContext::from_env().unwrap();
                assert_eq!(ctx.fpm_dir, temp_path);
            },
        );
    }

    #[test]
    fn from_env_reads_session_dir_from_env() {
        let temp = tempfile::tempdir().unwrap();
        let session = temp.path().join("multishells").join("1234_5678");
        fs::create_dir_all(&session).unwrap();

        with_env(
            config::FPM_MULTISHELL_PATH_ENV,
            Some(session.to_str().unwrap()),
            || {
                let ctx = CommandContext::from_env().unwrap();
                assert_eq!(ctx.session_dir.as_ref().unwrap(), &session);
            },
        );
    }

    #[test]
    fn from_env_session_dir_none_when_unset() {
        with_env(config::FPM_MULTISHELL_PATH_ENV, None, || {
            let ctx = CommandContext::from_env().unwrap();
            assert!(ctx.session_dir.is_none());
        });
    }

    use std::fs;
}
