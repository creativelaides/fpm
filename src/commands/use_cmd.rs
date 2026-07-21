// `fpm use [version] [--silent-if-unchanged]` — switch Python version per session.
//
// Spec: python-version-switching (all requirements)
//
// Resolution:
//   1. If `version` is given: resolve via `pymanager.resolve_exe(version)`.
//   2. If no `version`: call `version_file::resolve(cwd)` to get a tag from
//      `.python-version` or `pyproject.toml`.
//
// After resolution:
//   - Get the exe path, derive the install dir (parent of exe).
//   - Get session_dir from FPM_MULTISHELL_PATH env var (must be set).
//   - If --silent-if-unchanged: compare current_target(session_dir) vs the
//     resolved install_dir; if equal, suppress output and exit 0.
//   - Delegates retarget + PYTHON_MANAGER_DEFAULT set to the shared
//     `commands::activate_session` helper so `fpm use` and `fpm default`
//     cannot drift (spec: python-version-switching).
//   - Print "Using Python <version>".
//
// Session-only: does NOT write pymanager.json.

use std::path::{Path, PathBuf};

use crate::commands::activate_session;
use crate::config;
use crate::error::FpmError;
use crate::services::pymanager::PyManagerOps;
use crate::shim;
use crate::version_file;

/// Runs the `fpm use` command.
///
/// Parameters:
/// - `pymanager`: the PyManager client (real or mock)
/// - `version`: explicit version tag, or None to resolve from version files
/// - `silent_if_unchanged`: if true, suppress stdout when the version is
///   already active
/// - `cwd`: current working directory (for version file resolution)
/// - `session_dir`: the per-session multishell directory (from
///   FPM_MULTISHELL_PATH); required — `fpm use` only works inside an
///   fpm-integrated shell
pub fn run<M: PyManagerOps>(
    pymanager: &mut M,
    version: Option<&str>,
    silent_if_unchanged: bool,
    cwd: &Path,
    session_dir: &Path,
) -> Result<Option<String>, FpmError> {
    // 1. Resolve the version tag.
    let tag = match version {
        Some(v) => v.to_string(),
        None => version_file::resolve(cwd, pymanager)?,
    };

    // 2. Resolve the exe path for this tag so we can derive the install dir
    //    and (when --silent-if-unchanged) compare against the current target.
    let exe_path = pymanager.resolve_exe(&tag)?;

    // 3. Derive the install directory (parent of the exe).
    let install_dir = exe_path.parent().ok_or_else(|| {
        FpmError::ShimError(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "resolved exe has no parent directory",
        ))
    })?;

    // 4. Canonicalize install_dir for comparison.
    let canonical_install = install_dir
        .canonicalize()
        .unwrap_or_else(|_| install_dir.to_path_buf());

    // 5. If --silent-if-unchanged, check if the junction already points here.
    if silent_if_unchanged {
        if let Some(current) = shim::current_target(session_dir)? {
            if current == canonical_install {
                // Already active — suppress output, exit 0.
                // Still set PYTHON_MANAGER_DEFAULT for correctness.
                std::env::set_var(config::PYTHON_MANAGER_DEFAULT_ENV, &tag);
                return Ok(None);
            }
        }
    }

    // 6. Activate the session via the shared helper (retarget + set env).
    activate_session(pymanager, &tag, session_dir)?;

    // 7. Print the switch message.

    Ok(Some(tag))
}

/// Reads `FPM_MULTISHELL_PATH` from the environment and returns the session dir.
///
/// Returns `FpmError::ShimError` if the env var is not set (fpm use only works
/// inside an fpm-integrated shell).
#[allow(dead_code)]
pub fn session_dir_from_env() -> Result<PathBuf, FpmError> {
    std::env::var_os(config::FPM_MULTISHELL_PATH_ENV)
        .map(PathBuf::from)
        .ok_or_else(|| {
            FpmError::ShimError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "FPM_MULTISHELL_PATH is not set — run 'fpm env --shell powershell' first",
            ))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::pymanager::{MockPyManager, Runtime};
    use std::fs;
    use std::path::PathBuf;

    fn canned_runtimes() -> Vec<Runtime> {
        vec![
            Runtime {
                tag: "3.14-64".to_string(),
                version: "3.14.6".to_string(),
                executable: PathBuf::from("C:\\Python314\\python.exe"),
                is_default: true,
            },
            Runtime {
                tag: "3.13-64".to_string(),
                version: "3.13.7".to_string(),
                executable: PathBuf::from("C:\\Python313\\python.exe"),
                is_default: false,
            },
        ]
    }

    /// Creates a fake install directory with a marker file, returns its path.
    fn make_install_dir(parent: &Path, name: &str) -> PathBuf {
        let dir = parent.join(name);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("marker.txt"), "hello").unwrap();
        dir
    }

    #[test]
    fn use_with_explicit_version_resolves_and_retargets() {
        let temp = tempfile::tempdir().unwrap();
        let fpm_dir = temp.path();

        // Create a session dir (as fpm env would).
        let session_dir = shim::create_session_dir(fpm_dir).unwrap();
        // Remove it so retarget can create the junction.
        fs::remove_dir(&session_dir).unwrap();

        // Create a fake install dir.
        let install_dir = make_install_dir(fpm_dir, "install_314");

        // Build runtimes pointing at the fake install dir.
        let runtimes = vec![Runtime {
            tag: "3.14-64".to_string(),
            version: "3.14.6".to_string(),
            executable: install_dir.join("python.exe"),
            is_default: true,
        }];
        // Create the python.exe file so parent() resolves.
        fs::write(install_dir.join("python.exe"), "fake").unwrap();

        let mut mock = MockPyManager::new(runtimes, fpm_dir.join("pymanager.json"));

        let res = run(&mut mock, Some("3.14-64"), false, fpm_dir, &session_dir).unwrap();
        assert!(res.is_some());

        // Verify the junction points to the install dir.
        let target = shim::current_target(&session_dir).unwrap().unwrap();
        let canonical_install = install_dir.canonicalize().unwrap();
        assert_eq!(target, canonical_install);
    }

    #[test]
    fn use_version_not_installed_returns_error() {
        let temp = tempfile::tempdir().unwrap();
        let fpm_dir = temp.path();

        let session_dir = shim::create_session_dir(fpm_dir).unwrap();
        fs::remove_dir(&session_dir).unwrap();

        let mut mock = MockPyManager::new(canned_runtimes(), fpm_dir.join("pymanager.json"));

        let err = run(&mut mock, Some("9.9"), false, fpm_dir, &session_dir).unwrap_err();
        assert!(matches!(err, FpmError::VersionNotInstalled { .. }));
        assert_eq!(err.exit_code(), 2);
    }

    #[test]
    fn use_no_version_no_file_returns_no_version_file_error() {
        let temp = tempfile::tempdir().unwrap();
        let fpm_dir = temp.path();

        let session_dir = shim::create_session_dir(fpm_dir).unwrap();
        fs::remove_dir(&session_dir).unwrap();

        // No version file in the temp dir tree.
        let mut mock = MockPyManager::new(canned_runtimes(), fpm_dir.join("pymanager.json"));

        let err = run(&mut mock, None, false, fpm_dir, &session_dir).unwrap_err();
        assert!(matches!(err, FpmError::NoVersionFile));
        assert_eq!(err.exit_code(), 3);
    }

    #[test]
    fn use_no_version_resolves_from_python_version_file() {
        let temp = tempfile::tempdir().unwrap();
        let fpm_dir = temp.path();

        // Write .python-version in cwd.
        fs::write(fpm_dir.join(".python-version"), "3.13-64\n").unwrap();

        let session_dir = shim::create_session_dir(fpm_dir).unwrap();
        fs::remove_dir(&session_dir).unwrap();

        // Create a fake install dir for 3.13.
        let install_dir = make_install_dir(fpm_dir, "install_313");
        fs::write(install_dir.join("python.exe"), "fake").unwrap();

        let runtimes = vec![
            Runtime {
                tag: "3.14-64".to_string(),
                version: "3.14.6".to_string(),
                executable: make_install_dir(fpm_dir, "install_314").join("python.exe"),
                is_default: true,
            },
            Runtime {
                tag: "3.13-64".to_string(),
                version: "3.13.7".to_string(),
                executable: install_dir.join("python.exe"),
                is_default: false,
            },
        ];
        // Create python.exe for the 3.14 install too.
        fs::write(fpm_dir.join("install_314").join("python.exe"), "fake").unwrap();

        let mut mock = MockPyManager::new(runtimes, fpm_dir.join("pymanager.json"));

        let res = run(&mut mock, None, false, fpm_dir, &session_dir).unwrap();
        assert!(res.is_some());

        // Verify the junction points to 3.13.
        let target = shim::current_target(&session_dir).unwrap().unwrap();
        let canonical_install = install_dir.canonicalize().unwrap();
        assert_eq!(target, canonical_install);
    }

    #[test]
    fn use_silent_if_unchanged_suppresses_when_already_active() {
        let temp = tempfile::tempdir().unwrap();
        let fpm_dir = temp.path();

        let session_dir = shim::create_session_dir(fpm_dir).unwrap();
        fs::remove_dir(&session_dir).unwrap();

        // Create install dir and retarget to it first.
        let install_dir = make_install_dir(fpm_dir, "install_314");
        fs::write(install_dir.join("python.exe"), "fake").unwrap();

        let runtimes = vec![Runtime {
            tag: "3.14-64".to_string(),
            version: "3.14.6".to_string(),
            executable: install_dir.join("python.exe"),
            is_default: true,
        }];

        let mut mock = MockPyManager::new(runtimes, fpm_dir.join("pymanager.json"));

        // First, set the junction to 3.14.
        run(&mut mock, Some("3.14-64"), false, fpm_dir, &session_dir).unwrap();

        // Now run with --silent-if-unchanged for the same version.
        let res = run(&mut mock, Some("3.14-64"), true, fpm_dir, &session_dir).unwrap();
        assert!(res.is_none());
    }

    #[test]
    fn use_silent_if_unchanged_switches_when_different() {
        let temp = tempfile::tempdir().unwrap();
        let fpm_dir = temp.path();

        let session_dir = shim::create_session_dir(fpm_dir).unwrap();
        fs::remove_dir(&session_dir).unwrap();

        // Create two install dirs.
        let install_313 = make_install_dir(fpm_dir, "install_313");
        fs::write(install_313.join("python.exe"), "fake").unwrap();
        let install_314 = make_install_dir(fpm_dir, "install_314");
        fs::write(install_314.join("python.exe"), "fake").unwrap();

        let runtimes = vec![
            Runtime {
                tag: "3.13-64".to_string(),
                version: "3.13.7".to_string(),
                executable: install_313.join("python.exe"),
                is_default: false,
            },
            Runtime {
                tag: "3.14-64".to_string(),
                version: "3.14.6".to_string(),
                executable: install_314.join("python.exe"),
                is_default: true,
            },
        ];

        let mut mock = MockPyManager::new(runtimes, fpm_dir.join("pymanager.json"));

        // First switch to 3.13.
        run(&mut mock, Some("3.13-64"), false, fpm_dir, &session_dir).unwrap();

        // Now run silent-if-unchanged for 3.14 (different) — should switch.
        let res = run(&mut mock, Some("3.14-64"), true, fpm_dir, &session_dir).unwrap();
        assert!(res.is_some());

        // Verify junction now points to 3.14.
        let target = shim::current_target(&session_dir).unwrap().unwrap();
        let canonical_314 = install_314.canonicalize().unwrap();
        assert_eq!(target, canonical_314);
    }

    #[test]
    fn use_does_not_write_pymanager_json() {
        let temp = tempfile::tempdir().unwrap();
        let fpm_dir = temp.path();

        let config_path = fpm_dir.join("pymanager.json");
        fs::write(&config_path, r#"{"default_tag": "3.12", "other_key": 99}"#).unwrap();

        let session_dir = shim::create_session_dir(fpm_dir).unwrap();
        fs::remove_dir(&session_dir).unwrap();

        let install_dir = make_install_dir(fpm_dir, "install_314");
        fs::write(install_dir.join("python.exe"), "fake").unwrap();

        let runtimes = vec![Runtime {
            tag: "3.14-64".to_string(),
            version: "3.14.6".to_string(),
            executable: install_dir.join("python.exe"),
            is_default: true,
        }];

        let mut mock = MockPyManager::new(runtimes, config_path.clone());

        run(&mut mock, Some("3.14-64"), false, fpm_dir, &session_dir).unwrap();

        // pymanager.json should be unchanged.
        let raw = fs::read_to_string(&config_path).unwrap();
        let json: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(
            json["default_tag"], "3.12",
            "use must NOT change default_tag"
        );
        assert_eq!(json["other_key"], 99, "other keys must be preserved");
    }

    #[test]
    fn session_dir_from_env_returns_path_when_set() {
        let _lock = crate::config::tests::ENV_MUTEX.lock().unwrap();
        let temp = tempfile::tempdir().unwrap();
        let session = temp.path().join("test_session");
        fs::create_dir_all(&session).unwrap();

        std::env::set_var(config::FPM_MULTISHELL_PATH_ENV, &session);
        let result = session_dir_from_env().unwrap();
        assert_eq!(result, session);
        std::env::remove_var(config::FPM_MULTISHELL_PATH_ENV);
    }

    #[test]
    fn session_dir_from_env_errors_when_unset() {
        let _lock = crate::config::tests::ENV_MUTEX.lock().unwrap();
        std::env::remove_var(config::FPM_MULTISHELL_PATH_ENV);
        let err = session_dir_from_env().unwrap_err();
        assert!(matches!(err, FpmError::ShimError(_)));
    }
}
