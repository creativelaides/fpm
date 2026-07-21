// fpm configuration: directory and path resolution.
//
// Spec: powershell-shell-integration, pymanager-delegation
//
// FPM_DIR defaults to %LocalAppData%\fpm (via etcetera's Windows base strategy
// cache_dir, which maps to LOCALAPPDATA). The user may override it by setting
// the FPM_DIR environment variable.
//
// pymanager.json lives at %AppData%\Python\pymanager.json (etcetera config_dir,
// which maps to APPDATA/Roaming).

use etcetera::base_strategy::BaseStrategy;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Subdirectory under FPM_DIR used for per-session multishell junctions.
pub const MULTISHELLS_DIR: &str = "multishells";

/// Environment variable name for the per-session multishell path.
/// Set by `fpm env --shell powershell`, read by `fpm use` to locate the session dir.
pub const FPM_MULTISHELL_PATH_ENV: &str = "FPM_MULTISHELL_PATH";

/// Environment variable name for the fpm data directory.
/// Set by `fpm env --shell powershell`, read by commands needing the fpm root.
pub const FPM_DIR_ENV: &str = "FPM_DIR";

/// Environment variable for the Python manager default override.
pub const PYTHON_MANAGER_DEFAULT_ENV: &str = "PYTHON_MANAGER_DEFAULT";

/// Filename of the PyManager configuration JSON.
pub const PYMANAGER_JSON: &str = "pymanager.json";

/// Subdirectory under %AppData% where PyManager stores pymanager.json.
pub const PYMANAGER_DIR: &str = "Python";

/// Errors that can occur during configuration resolution.
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Could not determine user data directory: {0}")]
    HomeDirError(String),
}

/// Resolves the fpm data directory.
///
/// Checks the `FPM_DIR` environment variable first (user override).
/// Falls back to `%LocalAppData%\fpm` via etcetera's Windows base strategy
/// (cache_dir on Windows = LOCALAPPDATA).
///
/// The returned path is NOT created — callers are responsible for creation
/// (e.g. `shim::create_session_dir` creates `<fpm_dir>/multishells/...`).
pub fn fpm_dir() -> Result<PathBuf, ConfigError> {
    // Honor explicit override.
    if let Ok(dir) = std::env::var(FPM_DIR_ENV) {
        let dir = PathBuf::from(dir);
        return Ok(dir);
    }

    // Default: %LocalAppData%\fpm
    let strategy = etcetera::base_strategy::choose_base_strategy()
        .map_err(|e| ConfigError::HomeDirError(e.to_string()))?;

    Ok(strategy.cache_dir().join("fpm"))
}

/// Resolves the path to PyManager's `pymanager.json`.
///
/// Returns `%AppData%\Python\pymanager.json` via etcetera's Windows base
/// strategy config_dir (which maps to APPDATA/Roaming on Windows).
pub fn pymanager_json_path() -> Result<PathBuf, ConfigError> {
    let strategy = etcetera::base_strategy::choose_base_strategy()
        .map_err(|e| ConfigError::HomeDirError(e.to_string()))?;

    Ok(strategy
        .config_dir()
        .join(PYMANAGER_DIR)
        .join(PYMANAGER_JSON))
}

/// Returns the multishells directory path under the given fpm dir.
///
/// This is `<fpm_dir>/multishells/`. The directory is NOT created here.
pub fn multishells_dir(fpm_dir: &Path) -> PathBuf {
    fpm_dir.join(MULTISHELLS_DIR)
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::env;

    pub static ENV_MUTEX: once_cell::sync::Lazy<std::sync::Mutex<()>> =
        once_cell::sync::Lazy::new(|| std::sync::Mutex::new(()));

    /// Helper: run a closure with an env var set, then restore it.
    fn with_env<F>(key: &str, value: Option<&str>, f: F)
    where
        F: FnOnce(),
    {
        let _lock = ENV_MUTEX.lock().unwrap();
        let original = env::var_os(key);
        match value {
            Some(v) => env::set_var(key, v),
            None => env::remove_var(key),
        }
        f();
        // Restore
        match original {
            Some(v) => env::set_var(key, v),
            None => env::remove_var(key),
        }
    }

    #[test]
    fn fpm_dir_honors_env_override() {
        let temp = tempfile::tempdir().unwrap();
        let temp_path = temp.path().to_path_buf();

        with_env(FPM_DIR_ENV, Some(temp_path.to_str().unwrap()), || {
            let dir = fpm_dir().unwrap();
            assert_eq!(dir, temp_path);
        });
    }

    #[test]
    fn fpm_dir_falls_back_to_default_without_env() {
        with_env(FPM_DIR_ENV, None, || {
            let dir = fpm_dir().unwrap();
            // Should end with "fpm"
            assert_eq!(
                dir.file_name().unwrap(),
                "fpm",
                "default fpm_dir should end with 'fpm', got {:?}",
                dir
            );
        });
    }

    #[test]
    fn pymanager_json_path_ends_with_python_pymanager_json() {
        // Don't depend on env override; just check the suffix structure.
        let path = pymanager_json_path().unwrap();
        assert_eq!(path.file_name().unwrap(), PYMANAGER_JSON);
        assert_eq!(path.parent().unwrap().file_name().unwrap(), PYMANAGER_DIR,);
    }

    #[test]
    fn multishells_dir_appends_subdir() {
        let base = Path::new("C:\\fake\\fpm");
        let ms = multishells_dir(base);
        assert_eq!(ms, Path::new("C:\\fake\\fpm\\multishells"));
    }

    #[test]
    fn multishells_dir_is_relative_to_fpm_dir() {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().to_path_buf();
        let ms = multishells_dir(&base);
        assert!(ms.starts_with(&base));
        assert_eq!(ms.file_name().unwrap(), MULTISHELLS_DIR);
    }

    #[test]
    fn constants_are_stable() {
        assert_eq!(MULTISHELLS_DIR, "multishells");
        assert_eq!(FPM_MULTISHELL_PATH_ENV, "FPM_MULTISHELL_PATH");
        assert_eq!(FPM_DIR_ENV, "FPM_DIR");
        assert_eq!(PYTHON_MANAGER_DEFAULT_ENV, "PYTHON_MANAGER_DEFAULT");
        assert_eq!(PYMANAGER_JSON, "pymanager.json");
        assert_eq!(PYMANAGER_DIR, "Python");
    }
}
