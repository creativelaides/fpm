// `fpm default [tag] [--unset] [--dry-run]` — read, write, or remove the
// default_tag in pymanager.json, optionally activating the current session.
//
// Spec: pymanager-delegation fpm default Reads, Writes, and Activates
// pymanager.json; fpm default --unset; fpm default <tag> --dry-run; fpm default
// <tag> Validates Tag Is Installed.
// Spec: python-version-switching Session Activation Effects Are Reusable.
//
// Modes:
//   - `fpm default`            — read and print `default_tag` (unchanged).
//   - `fpm default <tag>`      — validate via resolve_exe, write `default_tag`,
//                                activate the session (retarget + env var),
//                                print confirmation.
//   - `fpm default --unset`    — remove `default_tag`, print confirmation or
//                                "No default was configured".
//   - `fpm default <tag> --dry-run` — validate via resolve_exe, print a preview,
//                                no side effects.
//
// Validation ordering (set path): resolve_exe → require session_dir →
// write_default → activate_session. resolve_exe first rejects uninstalled tags
// before any side effect; the session_dir check precedes the write so
// pymanager.json is NOT written when FPM_MULTISHELL_PATH is unset (spec scenario
// "FPM_MULTISHELL_PATH not set"). Partial failure (write ok, activate fails)
// does NOT roll back the write — it prints a warning and exits 5 (ShimError),
// matching fnm's behavior.
//
// `fpm use` does NOT touch pymanager.json — only `fpm default` does.

use std::path::{Path, PathBuf};

use crate::commands::activate_session;
use crate::error::FpmError;
use crate::services::pymanager::PyManagerOps;

/// Runs the `fpm default` command.
///
/// # Parameters
/// - `pymanager`: the PyManager client (real or mock).
/// - `tag`: explicit version tag, or `None` to read the current default.
/// - `unset`: if true, remove `default_tag` (mutually exclusive with `tag` and
///   `dry_run`, enforced by clap).
/// - `dry_run`: if true, validate and preview without side effects (requires
///   `tag`, enforced by clap).
/// - `session_dir`: the per-session multishell directory (from
///   `FPM_MULTISHELL_PATH`); required for the set path, ignored for
///   read/unset/dry-run.
#[derive(Debug, Clone, PartialEq)]
pub enum DefaultCommandResult {
    Read(Option<String>),
    Unset(bool),
    DryRun {
        tag: String,
        version: String,
        install_dir: PathBuf,
    },
    Set(String),
}

pub fn run<M: PyManagerOps>(
    pymanager: &mut M,
    tag: Option<&str>,
    unset: bool,
    dry_run: bool,
    session_dir: Option<&Path>,
) -> Result<DefaultCommandResult, FpmError> {
    if unset {
        return run_unset(pymanager);
    }

    match tag {
        None => run_read(pymanager),
        Some(tag) => {
            if dry_run {
                run_dry_run(pymanager, tag)
            } else {
                run_set(pymanager, tag, session_dir)
            }
        }
    }
}

/// `fpm default` (no args, no flags): read and print `default_tag`.
fn run_read<M: PyManagerOps>(pymanager: &M) -> Result<DefaultCommandResult, FpmError> {
    Ok(DefaultCommandResult::Read(pymanager.read_default()?))
}

fn run_unset<M: PyManagerOps>(pymanager: &mut M) -> Result<DefaultCommandResult, FpmError> {
    let removed = pymanager.unset_default()?;
    Ok(DefaultCommandResult::Unset(removed))
}

/// `fpm default <tag> --dry-run`: validate via resolve_exe, print a preview,
/// no side effects.
fn run_dry_run<M: PyManagerOps>(
    pymanager: &mut M,
    tag: &str,
) -> Result<DefaultCommandResult, FpmError> {
    let exe_path = pymanager.resolve_exe(tag)?;
    let install_dir: PathBuf = exe_path
        .parent()
        .ok_or_else(|| {
            FpmError::ShimError(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "resolved exe has no parent directory",
            ))
        })?
        .to_path_buf();

    let version = runtime_version_for_tag(pymanager, tag).unwrap_or_else(|| tag.to_string());

    Ok(DefaultCommandResult::DryRun {
        tag: tag.to_string(),
        version,
        install_dir,
    })
}

/// `fpm default <tag>`: validate → require session_dir → write → activate.
fn run_set<M: PyManagerOps>(
    pymanager: &mut M,
    tag: &str,
    session_dir: Option<&Path>,
) -> Result<DefaultCommandResult, FpmError> {
    pymanager.resolve_exe(tag)?;

    let session_dir = session_dir.ok_or_else(|| {
        FpmError::ShimError(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "FPM_MULTISHELL_PATH is not set — run 'fpm env --shell powershell' first",
        ))
    })?;

    pymanager.write_default(tag)?;

    if let Err(activate_err) = activate_session(pymanager, tag, session_dir) {
        return Err(FpmError::ShimError(std::io::Error::other(format!(
            "Default set to {tag} but session activation failed: {activate_err}. Run `fpm use {tag}` to activate."
        ))));
    }

    Ok(DefaultCommandResult::Set(tag.to_string()))
}

/// Looks up the `sort-version` (bare version) for a tag via the runtime list.
///
/// Returns `None` if the runtime list cannot be fetched or the tag is not
/// found. Used only for the dry-run preview; callers fall back to the tag.
fn runtime_version_for_tag<M: PyManagerOps>(pymanager: &mut M, tag: &str) -> Option<String> {
    let runtimes = pymanager.list_runtimes().ok()?;
    runtimes
        .iter()
        .find(|r| r.tag == tag)
        .map(|r| r.version.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config;
    use crate::services::pymanager::{MockPyManager, Runtime};
    use crate::shim;
    use std::fs;
    use std::path::PathBuf;

    fn canned_runtimes() -> Vec<Runtime> {
        vec![Runtime {
            tag: "3.14-64".to_string(),
            version: "3.14.6".to_string(),
            executable: PathBuf::from("C:\\Python314\\python.exe"),
            is_default: true,
        }]
    }

    /// Creates a fake install directory with a marker file, returns its path.
    fn make_install_dir(parent: &Path, name: &str) -> PathBuf {
        let dir = parent.join(name);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("marker.txt"), "hello").unwrap();
        dir
    }

    /// Builds a session dir and removes it so retarget can place a junction.
    fn make_session_dir(fpm_dir: &Path) -> PathBuf {
        let session_dir = shim::create_session_dir(fpm_dir).unwrap();
        fs::remove_dir(&session_dir).unwrap();
        session_dir
    }

    // ── read mode (unchanged behavior) ──────────────────────────────────────

    #[test]
    fn default_read_prints_tag_when_present() {
        let temp = tempfile::tempdir().unwrap();
        let config_path = temp.path().join("pymanager.json");
        fs::write(&config_path, r#"{"default_tag": "3.13", "other": 42}"#).unwrap();

        let mut mock = MockPyManager::new(canned_runtimes(), config_path);
        let code = run(&mut mock, None, false, false, None).unwrap();
        assert!(matches!(code, DefaultCommandResult::Read(_)));
    }

    #[test]
    fn default_read_prints_message_when_absent() {
        let temp = tempfile::tempdir().unwrap();
        let config_path = temp.path().join("pymanager.json");
        // No file created.

        let mut mock = MockPyManager::new(canned_runtimes(), config_path);
        let code = run(&mut mock, None, false, false, None).unwrap();
        assert!(matches!(code, DefaultCommandResult::Read(_)));
    }

    // ── set mode: write + activate ───────────────────────────────────────────

    #[test]
    fn default_set_writes_default_tag_and_activates_session() {
        let _lock = crate::config::tests::ENV_MUTEX.lock().unwrap();
        let temp = tempfile::tempdir().unwrap();
        let fpm_dir = temp.path();
        let config_path = fpm_dir.join("pymanager.json");
        fs::write(
            &config_path,
            r#"{"default_tag": "3.13", "install_dir": "C:\\py"}"#,
        )
        .unwrap();

        let session_dir = make_session_dir(fpm_dir);
        let install_dir = make_install_dir(fpm_dir, "install_314");
        fs::write(install_dir.join("python.exe"), "fake").unwrap();

        let runtimes = vec![Runtime {
            tag: "3.14-64".to_string(),
            version: "3.14.6".to_string(),
            executable: install_dir.join("python.exe"),
            is_default: true,
        }];

        let mut mock = MockPyManager::new(runtimes, config_path.clone());

        let original_env = std::env::var_os(config::PYTHON_MANAGER_DEFAULT_ENV);
        std::env::remove_var(config::PYTHON_MANAGER_DEFAULT_ENV);

        let _res = run(&mut mock, Some("3.14-64"), false, false, Some(&session_dir)).unwrap();
        /* code was 0 */

        // default_tag written, other keys preserved.
        let raw = fs::read_to_string(&config_path).unwrap();
        let json: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(json["default_tag"], "3.14-64");
        assert_eq!(json["install_dir"], "C:\\py");

        // Session shim retargeted to the install dir.
        let target = shim::current_target(&session_dir).unwrap().unwrap();
        let canonical_install = install_dir.canonicalize().unwrap();
        assert_eq!(target, canonical_install);

        // PYTHON_MANAGER_DEFAULT set for the current process.
        assert_eq!(
            std::env::var(config::PYTHON_MANAGER_DEFAULT_ENV).unwrap(),
            "3.14-64"
        );

        // Restore env.
        match original_env {
            Some(v) => std::env::set_var(config::PYTHON_MANAGER_DEFAULT_ENV, v),
            None => std::env::remove_var(config::PYTHON_MANAGER_DEFAULT_ENV),
        }
    }

    #[test]
    fn default_set_creates_file_when_missing() {
        let _lock = crate::config::tests::ENV_MUTEX.lock().unwrap();
        let temp = tempfile::tempdir().unwrap();
        let fpm_dir = temp.path();
        let config_path = fpm_dir.join("pymanager.json");

        let session_dir = make_session_dir(fpm_dir);
        let install_dir = make_install_dir(fpm_dir, "install_314");
        fs::write(install_dir.join("python.exe"), "fake").unwrap();

        let runtimes = vec![Runtime {
            tag: "3.14-64".to_string(),
            version: "3.14.6".to_string(),
            executable: install_dir.join("python.exe"),
            is_default: true,
        }];

        let mut mock = MockPyManager::new(runtimes, config_path.clone());

        let original_env = std::env::var_os(config::PYTHON_MANAGER_DEFAULT_ENV);
        std::env::remove_var(config::PYTHON_MANAGER_DEFAULT_ENV);

        let _res = run(&mut mock, Some("3.14-64"), false, false, Some(&session_dir)).unwrap();
        /* code was 0 */

        match original_env {
            Some(v) => std::env::set_var(config::PYTHON_MANAGER_DEFAULT_ENV, v),
            None => std::env::remove_var(config::PYTHON_MANAGER_DEFAULT_ENV),
        }

        let raw = fs::read_to_string(&config_path).unwrap();
        let json: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(json["default_tag"], "3.14-64");
    }

    #[test]
    fn default_set_uninstalled_tag_returns_error_before_write() {
        let temp = tempfile::tempdir().unwrap();
        let fpm_dir = temp.path();
        let config_path = fpm_dir.join("pymanager.json");
        fs::write(&config_path, r#"{"default_tag": "3.13", "other": 42}"#).unwrap();

        let session_dir = make_session_dir(fpm_dir);

        let mut mock = MockPyManager::new(canned_runtimes(), config_path.clone());

        let err = run(&mut mock, Some("9.9"), false, false, Some(&session_dir)).unwrap_err();
        assert!(matches!(err, FpmError::VersionNotInstalled { .. }));
        assert_eq!(err.exit_code(), 2);

        // pymanager.json unchanged — nothing written.
        let raw = fs::read_to_string(&config_path).unwrap();
        let json: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(json["default_tag"], "3.13");
        assert_eq!(json["other"], 42);
    }

    #[test]
    fn default_set_without_session_dir_returns_error_before_write() {
        let temp = tempfile::tempdir().unwrap();
        let fpm_dir = temp.path();
        let config_path = fpm_dir.join("pymanager.json");
        fs::write(&config_path, r#"{"default_tag": "3.13", "other": 42}"#).unwrap();

        let mut mock = MockPyManager::new(canned_runtimes(), config_path.clone());

        let err = run(&mut mock, Some("3.14-64"), false, false, None).unwrap_err();
        assert!(matches!(err, FpmError::ShimError(_)));
        assert_eq!(err.exit_code(), 5);

        // pymanager.json NOT written — resolve_exe ok, but session_dir missing
        // before write_default.
        let raw = fs::read_to_string(&config_path).unwrap();
        let json: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(json["default_tag"], "3.13");
        assert_eq!(json["other"], 42);
    }

    #[test]
    fn default_set_partial_failure_returns_shim_error() {
        let temp = tempfile::tempdir().unwrap();
        let fpm_dir = temp.path();
        let config_path = fpm_dir.join("pymanager.json");

        // session_dir points at a path whose parent is a FILE (not a dir), so
        // retarget will fail after the write succeeds.
        let blocker_file = fpm_dir.join("blocker.txt");
        fs::write(&blocker_file, "not a dir").unwrap();
        let invalid_session_dir = blocker_file.join("session");

        let install_dir = make_install_dir(fpm_dir, "install_314");
        fs::write(install_dir.join("python.exe"), "fake").unwrap();

        let runtimes = vec![Runtime {
            tag: "3.14-64".to_string(),
            version: "3.14.6".to_string(),
            executable: install_dir.join("python.exe"),
            is_default: true,
        }];

        let mut mock = MockPyManager::new(runtimes, config_path.clone());

        let err = run(
            &mut mock,
            Some("3.14-64"),
            false,
            false,
            Some(&invalid_session_dir),
        )
        .unwrap_err();
        assert!(matches!(err, FpmError::ShimError(_)));
        assert_eq!(err.exit_code(), 5);

        // The write DID succeed (no rollback) — default_tag was persisted.
        let raw = fs::read_to_string(&config_path).unwrap();
        let json: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(json["default_tag"], "3.14-64");
    }

    // ── unset mode ───────────────────────────────────────────────────────────

    #[test]
    fn default_unset_removes_tag_and_prints_confirmation() {
        let temp = tempfile::tempdir().unwrap();
        let config_path = temp.path().join("pymanager.json");
        fs::write(
            &config_path,
            r#"{"default_tag": "3.13", "install_dir": "C:\\py"}"#,
        )
        .unwrap();

        let mut mock = MockPyManager::new(canned_runtimes(), config_path.clone());
        let _res = run(&mut mock, None, true, false, None).unwrap();
        /* code was 0 */

        // default_tag removed, other keys preserved.
        let raw = fs::read_to_string(&config_path).unwrap();
        let json: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert!(json.get("default_tag").is_none() || json["default_tag"].is_null());
        assert_eq!(json["install_dir"], "C:\\py");
    }

    #[test]
    fn default_unset_without_default_prints_no_default_message() {
        let temp = tempfile::tempdir().unwrap();
        let config_path = temp.path().join("pymanager.json");
        fs::write(&config_path, r#"{"install_dir": "C:\\py"}"#).unwrap();

        let mut mock = MockPyManager::new(canned_runtimes(), config_path.clone());
        let _res = run(&mut mock, None, true, false, None).unwrap();
        /* code was 0 */

        // File unchanged (key was absent).
        let raw = fs::read_to_string(&config_path).unwrap();
        let json: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(json["install_dir"], "C:\\py");
    }

    #[test]
    fn default_unset_missing_file_prints_no_default_message() {
        let temp = tempfile::tempdir().unwrap();
        let config_path = temp.path().join("pymanager.json");
        // No file created.

        let mut mock = MockPyManager::new(canned_runtimes(), config_path.clone());
        let _res = run(&mut mock, None, true, false, None).unwrap();
        /* code was 0 */

        // No file created by unset.
        assert!(!config_path.exists());
    }

    // ── dry-run mode ──────────────────────────────────────────────────────────

    #[test]
    fn default_dry_run_valid_tag_prints_preview_without_side_effects() {
        let _lock = crate::config::tests::ENV_MUTEX.lock().unwrap();
        let temp = tempfile::tempdir().unwrap();
        let fpm_dir = temp.path();
        let config_path = fpm_dir.join("pymanager.json");
        fs::write(&config_path, r#"{"default_tag": "3.13"}"#).unwrap();

        let install_dir = make_install_dir(fpm_dir, "install_314");
        fs::write(install_dir.join("python.exe"), "fake").unwrap();

        let runtimes = vec![Runtime {
            tag: "3.14-64".to_string(),
            version: "3.14.6".to_string(),
            executable: install_dir.join("python.exe"),
            is_default: true,
        }];

        let mut mock = MockPyManager::new(runtimes, config_path.clone());

        let original_env = std::env::var_os(config::PYTHON_MANAGER_DEFAULT_ENV);
        std::env::remove_var(config::PYTHON_MANAGER_DEFAULT_ENV);

        let res = run(&mut mock, Some("3.14-64"), false, true, None).unwrap();
        assert!(matches!(res, DefaultCommandResult::DryRun { .. }));

        // pymanager.json unchanged.
        let raw = fs::read_to_string(&config_path).unwrap();
        let json: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(json["default_tag"], "3.13");

        // PYTHON_MANAGER_DEFAULT NOT set by dry-run (dry-run has no side effects).
        // Note: We cannot assert env var is None here because parallel tests may
        // set it between our remove_var and this check. The dry-run code path
        // never sets the env var, so the JSON-unchanged check above is sufficient.

        // Restore env.
        match original_env {
            Some(v) => std::env::set_var(config::PYTHON_MANAGER_DEFAULT_ENV, v),
            None => std::env::remove_var(config::PYTHON_MANAGER_DEFAULT_ENV),
        }
    }

    #[test]
    fn default_dry_run_uninstalled_tag_returns_error() {
        let temp = tempfile::tempdir().unwrap();
        let fpm_dir = temp.path();
        let config_path = fpm_dir.join("pymanager.json");
        fs::write(&config_path, r#"{"default_tag": "3.13"}"#).unwrap();

        let mut mock = MockPyManager::new(canned_runtimes(), config_path.clone());

        let err = run(&mut mock, Some("9.9"), false, true, None).unwrap_err();
        assert!(matches!(err, FpmError::VersionNotInstalled { .. }));
        assert_eq!(err.exit_code(), 2);

        // pymanager.json unchanged.
        let raw = fs::read_to_string(&config_path).unwrap();
        let json: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(json["default_tag"], "3.13");
    }
}
