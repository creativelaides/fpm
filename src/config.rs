// fpy configuration: directory and path resolution.
//
// Spec: powershell-shell-integration, pymanager-delegation
//
// FPY_DIR defaults to %LocalAppData%\fpy (via etcetera's Windows base strategy
// cache_dir, which maps to LOCALAPPDATA). The user may override it by setting
// the FPY_DIR environment variable.
//
// pymanager.json lives at %AppData%\Python\pymanager.json (etcetera config_dir,
// which maps to APPDATA/Roaming).

use etcetera::base_strategy::BaseStrategy;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Subdirectory under FPY_DIR used for per-session multishell junctions.
pub const MULTISHELLS_DIR: &str = "multishells";

/// Environment variable name for the per-session multishell path.
/// Set by `fpy env --shell powershell`, read by `fpy use` to locate the session dir.
pub const FPY_MULTISHELL_PATH_ENV: &str = "FPY_MULTISHELL_PATH";

/// Environment variable name for the fpy data directory.
/// Set by `fpy env --shell powershell`, read by commands needing the fpy root.
pub const FPY_DIR_ENV: &str = "FPY_DIR";

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

/// Resolves the fpy data directory.
///
/// Checks the `FPM_DIR` environment variable first (user override).
/// Falls back to `%LocalAppData%\fpy` via etcetera's Windows base strategy
/// (cache_dir on Windows = LOCALAPPDATA).
///
/// The returned path is NOT created — callers are responsible for creation
/// (e.g. `shim::create_session_dir` creates `<fpy_dir>/multishells/...`).
pub fn fpy_dir() -> Result<PathBuf, ConfigError> {
    // Honor explicit override.
    if let Ok(dir) = std::env::var(FPY_DIR_ENV) {
        let dir = PathBuf::from(dir);
        return Ok(dir);
    }

    // Default: %LocalAppData%\fpy
    let strategy = etcetera::base_strategy::choose_base_strategy()
        .map_err(|e| ConfigError::HomeDirError(e.to_string()))?;

    Ok(strategy.cache_dir().join("fpy"))
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
/// This is `<fpy_dir>/multishells/`. The directory is NOT created here.
pub fn multishells_dir(fpy_dir: &Path) -> PathBuf {
    fpy_dir.join(MULTISHELLS_DIR)
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
    fn fpy_dir_honors_env_override() {
        let temp = tempfile::tempdir().unwrap();
        let temp_path = temp.path().to_path_buf();

        with_env(FPY_DIR_ENV, Some(temp_path.to_str().unwrap()), || {
            let dir = fpy_dir().unwrap();
            assert_eq!(dir, temp_path);
        });
    }

    #[test]
    fn fpy_dir_falls_back_to_default_without_env() {
        with_env(FPY_DIR_ENV, None, || {
            let dir = fpy_dir().unwrap();
            // Should end with "fpy"
            assert_eq!(
                dir.file_name().unwrap(),
                "fpy",
                "default fpy_dir should end with 'fpm', got {:?}",
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
    fn multishells_dir_is_relative_to_fpy_dir() {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().to_path_buf();
        let ms = multishells_dir(&base);
        assert!(ms.starts_with(&base));
        assert_eq!(ms.file_name().unwrap(), MULTISHELLS_DIR);
    }

    #[test]
    fn constants_are_stable() {
        assert_eq!(MULTISHELLS_DIR, "multishells");
        assert_eq!(FPY_MULTISHELL_PATH_ENV, "FPY_MULTISHELL_PATH");
        assert_eq!(FPY_DIR_ENV, "FPY_DIR");
        assert_eq!(PYTHON_MANAGER_DEFAULT_ENV, "PYTHON_MANAGER_DEFAULT");
        assert_eq!(PYMANAGER_JSON, "pymanager.json");
        assert_eq!(PYMANAGER_DIR, "Python");
    }
}
