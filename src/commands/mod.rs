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

use std::path::{Path, PathBuf};

use crate::config;
use crate::error::FpmError;
use crate::pymanager::PyManager;
use crate::shim;

pub mod current;
pub mod default;
pub mod env_cmd;
pub mod install;
pub mod list;
pub mod passthrough;
pub mod use_cmd;

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
    /// Stored for commands that may need direct config access.
    #[allow(dead_code)]
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

/// Activates a Python runtime for the current session.
///
/// Shared session-activation sequence used by `fpm use` and `fpm default` so
/// the two commands cannot drift (spec: python-version-switching — "Session
/// Activation Effects Are Reusable"). Performs the following steps:
///
/// 1. Resolves the executable path for `tag` via the PyManager (rejects
///    uninstalled tags with `FpmError::VersionNotInstalled` before any side
///    effect).
/// 2. Derives the install directory as the parent of the resolved exe.
/// 3. Canonicalizes the install directory (for stable junction comparison).
/// 4. Retargets the per-session shim junction at `session_dir` to the
///    install dir.
/// 5. Sets `PYTHON_MANAGER_DEFAULT` to `tag` for the current process.
///
/// `silent_if_unchanged` is `fpm use`-specific and stays in `use_cmd`; this
/// helper does only the resolve → derive → retarget → set env sequence.
///
/// # Parameters
/// - `pymanager`: the PyManager client (real or mock) used to resolve the exe.
/// - `tag`: the runtime tag to activate, e.g. `"3.14-64"`.
/// - `session_dir`: the per-session multishell directory (from
///   `FPM_MULTISHELL_PATH`). The caller is responsible for ensuring it is
///   present and valid; `fpm default` checks for it before any write so the
///   activation path only runs when a session exists.
///
/// # Returns
/// The canonicalized install directory so each caller can print its own
/// message (`fpm use` prints "Using Python {tag}", `fpm default` prints
/// "Default set to {tag}; session activated").
pub fn activate_session<M: crate::pymanager::PyManagerOps>(
    pymanager: &mut M,
    tag: &str,
    session_dir: &Path,
) -> Result<PathBuf, FpmError> {
    // 1. Resolve the exe path for this tag (rejects uninstalled tags first).
    let exe_path = pymanager.resolve_exe(tag)?;

    // 2. Derive the install directory (parent of the exe).
    let install_dir = exe_path.parent().ok_or_else(|| {
        FpmError::ShimError(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "resolved exe has no parent directory",
        ))
    })?;

    // 3. Canonicalize install_dir for comparison / stable junction target.
    let canonical_install = install_dir
        .canonicalize()
        .unwrap_or_else(|_| install_dir.to_path_buf());

    // 4. Retarget the junction to the install directory.
    shim::retarget(session_dir, &canonical_install)?;

    // 5. Set PYTHON_MANAGER_DEFAULT in-process.
    std::env::set_var(config::PYTHON_MANAGER_DEFAULT_ENV, tag);

    Ok(canonical_install)
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
