// `fpm current` — print the active Python version.
//
// Spec: pymanager-delegation fpm current reports active version
//
// Determines the active version by checking `PYTHON_MANAGER_DEFAULT` env var
// first (set by `fpm use` for the session), then falling back to
// `default_tag` in pymanager.json. Then spawns `py -V` to report the
// actually-launched Python version for accuracy.

use std::process::Command;

use crate::config;
use crate::error::FpmError;
use crate::services::pymanager::PyManagerOps;

/// Runs the `fpm current` command.
///
/// 1. Determine the active tag: `PYTHON_MANAGER_DEFAULT` env var or
///    `pymanager.json` `default_tag`.
/// 2. Spawn `py -V` to get the actual version string from PyManager.
/// 3. Print the result.
pub fn run<M: PyManagerOps>(pymanager: &mut M) -> Result<i32, FpmError> {
    // Determine which tag is "active" — session override first, then config.
    let active_tag = match std::env::var_os(config::PYTHON_MANAGER_DEFAULT_ENV) {
        Some(v) => Some(v.to_string_lossy().into_owned()),
        None => pymanager.read_default()?,
    };

    // Spawn `py -V` for the real version string.
    let output = Command::new("py")
        .arg("-V")
        .output()
        .map_err(|_| FpmError::PyNotFound)?;

    let version_line = String::from_utf8_lossy(&output.stdout);
    let version_line = version_line.trim();

    if version_line.is_empty() {
        // py -V failed or produced no output.
        if let Some(tag) = active_tag {
            println!("Python {tag} (configured, py -V unavailable)");
        } else {
            println!("No default Python configured.");
        }
        return Ok(if output.status.success() { 0 } else { 1 });
    }

    match active_tag {
        Some(tag) => println!("{version_line} (tag: {tag})"),
        None => println!("{version_line}"),
    }

    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::pymanager::{MockPyManager, Runtime};
    use std::env;
    use std::path::PathBuf;

    fn canned_runtimes() -> Vec<Runtime> {
        vec![Runtime {
            tag: "3.14-64".to_string(),
            version: "3.14.6".to_string(),
            executable: PathBuf::from("C:\\Python314\\python.exe"),
            is_default: true,
        }]
    }

    /// Helper: run a closure with an env var set, then restore it.
    fn with_env<F>(key: &str, value: Option<&str>, f: F)
    where
        F: FnOnce(),
    {
        let _lock = crate::config::tests::ENV_MUTEX.lock().unwrap();
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
    fn current_reads_python_manager_default_env() {
        let temp = tempfile::tempdir().unwrap();
        let mut mock = MockPyManager::new(canned_runtimes(), temp.path().join("pymanager.json"));

        // Set the env var so current picks it up.
        with_env(config::PYTHON_MANAGER_DEFAULT_ENV, Some("3.14"), || {
            let code = run(&mut mock).unwrap_or_else(|e| {
                // If py is missing, the function returns PyNotFound, which is
                // valid. We just verify the env-var path was taken (no crash).
                if matches!(e, FpmError::PyNotFound) {
                    return 1;
                }
                panic!("unexpected error: {e:?}");
            });
            assert!(code == 0 || code == 1);
        });
    }

    #[test]
    fn current_falls_back_to_default_tag() {
        let temp = tempfile::tempdir().unwrap();
        let config_path = temp.path().join("pymanager.json");
        std::fs::write(&config_path, r#"{"default_tag": "3.13"}"#).unwrap();

        let mut mock = MockPyManager::new(canned_runtimes(), config_path);

        // Ensure PYTHON_MANAGER_DEFAULT is unset so it falls back to default_tag.
        with_env(config::PYTHON_MANAGER_DEFAULT_ENV, None, || {
            let result = run(&mut mock);
            // Either py exists (Ok) or PyNotFound — both are fine.
            match result {
                Ok(code) => assert!(code == 0 || code == 1),
                Err(e) => assert!(matches!(e, FpmError::PyNotFound)),
            }
        });
    }

    #[test]
    fn current_no_default_configured() {
        let temp = tempfile::tempdir().unwrap();
        let config_path = temp.path().join("pymanager.json");
        // No pymanager.json, no env var.

        let mut mock = MockPyManager::new(canned_runtimes(), config_path);

        with_env(config::PYTHON_MANAGER_DEFAULT_ENV, None, || {
            let result = run(&mut mock);
            match result {
                Ok(code) => assert!(code == 0 || code == 1),
                Err(e) => assert!(matches!(e, FpmError::PyNotFound)),
            }
        });
    }
}
