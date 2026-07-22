// Abstraction over the official PyManager (`py`/`pymanager`).
//
// Spec: pymanager-delegation
//
// `fpm` delegates ALL runtime management to `py`; it never downloads Python
// itself. This module:
//   - parses `py list --format=json` into a runtime collection (cached once per
//     process via `OnceCell`),
//   - resolves a single runtime's executable via `py list --one --format=exe`,
//   - reads and writes `%AppData%\Python\pymanager.json` (default_tag),
//   - spawns `py install <tag>` and streams its output.
//
// The `PyManagerOps` trait abstracts `py` calls so unit tests can run without
// PyManager installed (`MockPyManager` returns canned data).

use std::path::{Path, PathBuf};
use std::process::Command;

use once_cell::sync::OnceCell;
use serde::Deserialize;

use crate::config;
use crate::error::FpmError;

/// Wrapper for the `py list --format=json` output shape: `{"versions": [...]}`.
#[derive(Debug, Deserialize)]
struct PyListResponse {
    versions: Vec<Runtime>,
}

/// A single installed Python runtime as reported by `py list`.
///
/// Mirrors the JSON shape emitted by `py list --format=json`. Unknown keys are
/// ignored so future PyManager versions don't break parsing.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Runtime {
    /// PyManager tag, e.g. `"3.14-64"` (version plus architecture suffix).
    pub tag: String,
    /// Bare version string, e.g. `"3.14.6"`. Maps to `sort-version` in the JSON.
    #[serde(rename = "sort-version")]
    pub version: String,
    /// Full path to the runtime's `python.exe`.
    pub executable: PathBuf,
    /// Whether PyManager marks this runtime as the default.
    #[serde(rename = "default", default)]
    pub is_default: bool,
}

/// Operations fpm needs from PyManager.
///
/// The real implementation (`PyManager`) spawns `py.exe`. The test
/// implementation (`MockPyManager`) returns canned data, allowing unit tests to
/// run without PyManager installed.
pub trait PyManagerOps {
    /// Returns the installed runtimes (cached after the first call).
    fn list_runtimes(&mut self) -> Result<&[Runtime], FpmError>;

    /// Resolves the executable path for a single runtime tag.
    ///
    /// Delegates to `py list --one --format=exe <tag>` — fpm does NOT
    /// reimplement tag matching.
    fn resolve_exe(&mut self, tag: &str) -> Result<PathBuf, FpmError>;

    /// Reads `default_tag` from `pymanager.json`, if present.
    fn read_default(&self) -> Result<Option<String>, FpmError>;

    /// Writes `default_tag` to `pymanager.json`, preserving all other keys.
    fn write_default(&mut self, tag: &str) -> Result<(), FpmError>;

    /// Removes `default_tag` from `pymanager.json`, preserving all other keys.
    ///
    /// Returns `Ok(true)` if the key was present and removed, `Ok(false)` if
    /// the file or key is absent (no-op, no file created). This lets the caller
    /// print "Removed default" vs "No default configured" in a single call
    /// without a read-then-unset race.
    fn unset_default(&mut self) -> Result<bool, FpmError>;

    /// Spawns `py install <tag>` and returns the child exit code.
    #[allow(dead_code)]
    fn install(&mut self, tag: &str) -> Result<i32, FpmError>;
}

/// Real PyManager client. Caches `py list --format=json` for the process
/// lifetime (populated lazily on the first `list_runtimes` call).
pub struct PyManager {
    /// Cached runtime list. Empty until the first `list_runtimes` call.
    runtimes: OnceCell<Vec<Runtime>>,
}

impl PyManager {
    /// Creates a new `PyManager` with an empty cache.
    pub fn new() -> Self {
        PyManager {
            runtimes: OnceCell::new(),
        }
    }

    /// Spawns `py list --format=json` and parses the output into runtimes.
    fn fetch_runtimes(&self) -> Result<Vec<Runtime>, FpmError> {
        let output = Command::new("py")
            .args(["list", "-f", "json"])
            .output()
            .map_err(|_| FpmError::PyNotFound)?;

        if !output.status.success() {
            return Err(FpmError::PyNotFound);
        }

        let response: PyListResponse = serde_json::from_slice(&output.stdout)
            .map_err(|e| FpmError::ConfigError(e.to_string()))?;

        Ok(response.versions)
    }
}

impl Default for PyManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PyManagerOps for PyManager {
    fn list_runtimes(&mut self) -> Result<&[Runtime], FpmError> {
        let runtimes = self.runtimes.get_or_try_init(|| self.fetch_runtimes())?;
        Ok(runtimes)
    }

    fn resolve_exe(&mut self, tag: &str) -> Result<PathBuf, FpmError> {
        let output = Command::new("py")
            .args(["list", "--one", "-f", "exe", tag])
            .output()
            .map_err(|_| FpmError::PyNotFound)?;

        if !output.status.success() {
            return Err(FpmError::VersionNotInstalled {
                tag: tag.to_string(),
            });
        }

        let exe = String::from_utf8_lossy(&output.stdout);
        let exe = exe.trim();
        if exe.is_empty() {
            return Err(FpmError::VersionNotInstalled {
                tag: tag.to_string(),
            });
        }

        Ok(PathBuf::from(exe))
    }

    fn read_default(&self) -> Result<Option<String>, FpmError> {
        let path =
            config::pymanager_json_path().map_err(|e| FpmError::ConfigError(e.to_string()))?;
        read_default_tag(&path)
    }

    fn write_default(&mut self, tag: &str) -> Result<(), FpmError> {
        let path =
            config::pymanager_json_path().map_err(|e| FpmError::ConfigError(e.to_string()))?;
        write_default_tag(&path, tag)
    }

    fn unset_default(&mut self) -> Result<bool, FpmError> {
        let path =
            config::pymanager_json_path().map_err(|e| FpmError::ConfigError(e.to_string()))?;
        unset_default_tag(&path)
    }

    fn install(&mut self, tag: &str) -> Result<i32, FpmError> {
        let status = Command::new("py")
            .args(["install", tag])
            .status()
            .map_err(|_| FpmError::PyNotFound)?;

        Ok(status.code().unwrap_or(1))
    }
}

/// Mock PyManager for unit tests. Returns canned data without spawning `py`.
#[allow(dead_code)]
pub struct MockPyManager {
    /// Canned runtimes returned by `list_runtimes`.
    pub runtimes: Vec<Runtime>,
    /// Path to a `pymanager.json` fixture used by `read_default`/`write_default`.
    pub config_path: PathBuf,
}

impl MockPyManager {
    /// Creates a mock with the given runtimes and config path.
    #[allow(dead_code)]
    pub fn new(runtimes: Vec<Runtime>, config_path: PathBuf) -> Self {
        MockPyManager {
            runtimes,
            config_path,
        }
    }
}

impl PyManagerOps for MockPyManager {
    fn list_runtimes(&mut self) -> Result<&[Runtime], FpmError> {
        Ok(&self.runtimes)
    }

    fn resolve_exe(&mut self, tag: &str) -> Result<PathBuf, FpmError> {
        self.runtimes
            .iter()
            .find(|r| r.tag == tag)
            .map(|r| r.executable.clone())
            .ok_or_else(|| FpmError::VersionNotInstalled {
                tag: tag.to_string(),
            })
    }

    fn read_default(&self) -> Result<Option<String>, FpmError> {
        read_default_tag(&self.config_path)
    }

    fn write_default(&mut self, tag: &str) -> Result<(), FpmError> {
        write_default_tag(&self.config_path, tag)
    }

    fn unset_default(&mut self) -> Result<bool, FpmError> {
        unset_default_tag(&self.config_path)
    }

    fn install(&mut self, _tag: &str) -> Result<i32, FpmError> {
        // Mock install always succeeds without doing anything.
        Ok(0)
    }
}

// ---------------------------------------------------------------------------
// pymanager.json helpers (shared by real and mock impls)
// ---------------------------------------------------------------------------

/// Reads `default_tag` from a JSON file, returning `Ok(None)` if the file is
/// missing or the key is absent.
fn read_default_tag(path: &Path) -> Result<Option<String>, FpmError> {
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(FpmError::ConfigError(e.to_string())),
    };

    let json: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|e| FpmError::ConfigError(e.to_string()))?;

    Ok(json
        .get("default_tag")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string()))
}

/// Writes `default_tag` into a JSON file, preserving all other keys.
///
/// If the file does not exist, it is created with `{"default_tag": "<tag>"}`.
fn write_default_tag(path: &Path, tag: &str) -> Result<(), FpmError> {
    let mut json: serde_json::Value = match std::fs::read(path) {
        Ok(bytes) => {
            serde_json::from_slice(&bytes).map_err(|e| FpmError::ConfigError(e.to_string()))?
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            serde_json::json!({})
        }
        Err(e) => return Err(FpmError::ConfigError(e.to_string())),
    };

    // Ensure we're mutating an object.
    if !json.is_object() {
        json = serde_json::json!({});
    }

    if let Some(obj) = json.as_object_mut() {
        obj.insert("default_tag".to_string(), serde_json::json!(tag));
    }

    let parent = path.parent();
    if let Some(parent) = parent {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| FpmError::ConfigError(e.to_string()))?;
        }
    }

    let pretty =
        serde_json::to_string_pretty(&json).map_err(|e| FpmError::ConfigError(e.to_string()))?;
    std::fs::write(path, pretty).map_err(|e| FpmError::ConfigError(e.to_string()))?;

    Ok(())
}

/// Removes `default_tag` from a JSON file, preserving all other keys.
///
/// Returns:
/// - `Ok(true)`  — the `default_tag` key was present and removed (file and
///   other keys preserved).
/// - `Ok(false)` — the file is missing OR the key is absent (no-op; no file
///   is created).
/// - `Err`       — IO or parse failure.
///
/// This is the symmetric counterpart of `write_default_tag`: `write_default`
/// inserts/overwrites the key, `unset_default` removes it. Used by
/// `fpm default --unset` to print "Removed default" vs "No default
/// configured" in a single call without a read-then-unset race.
fn unset_default_tag(path: &Path) -> Result<bool, FpmError> {
    // Missing file → nothing to unset.
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(FpmError::ConfigError(e.to_string())),
    };

    let mut json: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|e| FpmError::ConfigError(e.to_string()))?;

    // Only an object can hold a `default_tag` key. A non-object file (e.g. an
    // array or scalar) has no key to remove — return false without rewriting
    // it, mirroring read_default_tag's silent skip.
    let removed = match json.as_object_mut() {
        Some(obj) => obj.remove("default_tag").is_some(),
        None => false,
    };

    if !removed {
        return Ok(false);
    }

    let pretty =
        serde_json::to_string_pretty(&json).map_err(|e| FpmError::ConfigError(e.to_string()))?;
    std::fs::write(path, pretty).map_err(|e| FpmError::ConfigError(e.to_string()))?;

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;

    /// Canned `py list --format=json` fixture resembling real PyManager output.
    /// The real output is a JSON object with a "versions" array, not a bare array.
    const JSON_FIXTURE: &str = r#"{
        "versions": [
            {
                "tag": "3.14-64",
                "sort-version": "3.14.6",
                "executable": "C:\\Python314\\python.exe",
                "default": true
            },
            {
                "tag": "3.13-64",
                "sort-version": "3.13.7",
                "executable": "C:\\Python313\\python.exe",
                "default": false
            },
            {
                "tag": "3.12-64",
                "sort-version": "3.12.11",
                "executable": "C:\\Python312\\python.exe"
            }
        ]
    }"#;

    fn canned_runtimes() -> Vec<Runtime> {
        let response: PyListResponse =
            serde_json::from_str(JSON_FIXTURE).expect("fixture must parse");
        response.versions
    }

    #[test]
    fn parses_py_list_json_into_runtimes() {
        let runtimes = canned_runtimes();
        assert_eq!(runtimes.len(), 3);

        let first = &runtimes[0];
        assert_eq!(first.tag, "3.14-64");
        assert_eq!(first.version, "3.14.6");
        assert_eq!(first.executable, PathBuf::from("C:\\Python314\\python.exe"));
        assert!(first.is_default);
    }

    #[test]
    fn missing_default_field_defaults_to_false() {
        let runtimes = canned_runtimes();
        // The third entry omits "default" — serde default should be false.
        assert!(!runtimes[2].is_default);
    }

    #[test]
    fn malformed_json_produces_config_error() {
        let result: Result<Vec<Runtime>, _> = serde_json::from_str("{ not json");
        assert!(result.is_err(), "malformed JSON should fail to parse");

        // Verify the same error surfaces through FpmError::ConfigError shape.
        let err = result.unwrap_err();
        assert!(!err.to_string().is_empty());
    }

    #[test]
    fn read_default_returns_tag_when_present() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("pymanager.json");
        fs::write(&path, r#"{"default_tag": "3.13", "other_key": 42}"#).unwrap();

        let tag = read_default_tag(&path).unwrap();
        assert_eq!(tag, Some("3.13".to_string()));
    }

    #[test]
    fn read_default_returns_none_when_key_absent() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("pymanager.json");
        fs::write(&path, r#"{"other_key": 42}"#).unwrap();

        let tag = read_default_tag(&path).unwrap();
        assert_eq!(tag, None);
    }

    #[test]
    fn read_default_returns_none_when_file_missing() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("does-not-exist.json");

        let tag = read_default_tag(&path).unwrap();
        assert_eq!(tag, None);
    }

    #[test]
    fn write_default_preserves_other_keys() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("pymanager.json");
        fs::write(&path, r#"{"default_tag": "3.13", "other_key": 42}"#).unwrap();

        write_default_tag(&path, "3.14").unwrap();

        let raw = fs::read_to_string(&path).unwrap();
        let json: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(json["default_tag"], "3.14");
        assert_eq!(json["other_key"], 42, "other keys must be preserved");
    }

    #[test]
    fn write_default_creates_file_when_missing() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("pymanager.json");

        write_default_tag(&path, "3.14").unwrap();

        assert!(path.exists(), "file should be created");
        let tag = read_default_tag(&path).unwrap();
        assert_eq!(tag, Some("3.14".to_string()));
    }

    #[test]
    fn write_default_round_trips_read() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("pymanager.json");

        write_default_tag(&path, "3.12").unwrap();
        assert_eq!(read_default_tag(&path).unwrap(), Some("3.12".to_string()));

        write_default_tag(&path, "3.14").unwrap();
        assert_eq!(read_default_tag(&path).unwrap(), Some("3.14".to_string()));
    }

    #[test]
    fn mock_list_runtimes_returns_canned_data() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("pymanager.json");
        let mut mock = MockPyManager::new(canned_runtimes(), path);

        let runtimes = mock.list_runtimes().unwrap();
        assert_eq!(runtimes.len(), 3);
        assert_eq!(runtimes[0].tag, "3.14-64");
    }

    #[test]
    fn mock_resolve_exe_finds_existing_tag() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("pymanager.json");
        let mut mock = MockPyManager::new(canned_runtimes(), path);

        let exe = mock.resolve_exe("3.13-64").unwrap();
        assert_eq!(exe, PathBuf::from("C:\\Python313\\python.exe"));
    }

    #[test]
    fn mock_resolve_exe_errors_for_missing_tag() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("pymanager.json");
        let mut mock = MockPyManager::new(canned_runtimes(), path);

        let err = mock.resolve_exe("9.9").unwrap_err();
        assert!(matches!(err, FpmError::VersionNotInstalled { .. }));
        assert_eq!(err.exit_code(), 2);
    }

    #[test]
    fn mock_install_returns_zero() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("pymanager.json");
        let mut mock = MockPyManager::new(canned_runtimes(), path);

        let code = mock.install("3.13").unwrap();
        assert_eq!(code, 0);
    }

    #[test]
    fn mock_default_read_write_via_trait() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("pymanager.json");
        let mut mock = MockPyManager::new(canned_runtimes(), path.clone());

        // No file yet.
        assert_eq!(mock.read_default().unwrap(), None);

        mock.write_default("3.14").unwrap();
        assert_eq!(mock.read_default().unwrap(), Some("3.14".to_string()));
    }

    #[test]
    fn trait_can_be_used_generically() {
        fn use_manager<M: PyManagerOps>(manager: &mut M) -> Result<usize, FpmError> {
            Ok(manager.list_runtimes()?.len())
        }

        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("pymanager.json");
        let mut mock = MockPyManager::new(canned_runtimes(), path);

        assert_eq!(use_manager(&mut mock).unwrap(), 3);
    }

    #[test]
    fn write_default_overwrites_non_object_file() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("pymanager.json");
        // Write a non-object (array) — should be replaced, not merged.
        {
            let mut f = fs::File::create(&path).unwrap();
            f.write_all(b"[1, 2, 3]").unwrap();
        }

        write_default_tag(&path, "3.14").unwrap();

        let raw = fs::read_to_string(&path).unwrap();
        let json: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(json["default_tag"], "3.14");
        assert!(json.is_object(), "file should now be a JSON object");
    }

    // ── unset_default_tag ───────────────────────────────────────────────────

    #[test]
    fn unset_default_tag_removes_key_and_preserves_others() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("pymanager.json");
        fs::write(&path, r#"{"default_tag": "3.13", "install_dir": "C:\\py"}"#).unwrap();

        let removed = unset_default_tag(&path).unwrap();
        assert!(removed, "key was present and removed");

        let raw = fs::read_to_string(&path).unwrap();
        let json: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert!(
            json.get("default_tag").is_none() || json["default_tag"].is_null(),
            "default_tag must be absent"
        );
        assert_eq!(json["install_dir"], "C:\\py", "other keys preserved");
    }

    #[test]
    fn unset_default_tag_returns_false_when_file_missing() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("does-not-exist.json");

        let removed = unset_default_tag(&path).unwrap();
        assert!(!removed, "missing file → false");
        // No file created.
        assert!(!path.exists());
    }

    #[test]
    fn unset_default_tag_returns_false_when_key_absent() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("pymanager.json");
        fs::write(&path, r#"{"install_dir": "C:\\py"}"#).unwrap();

        let removed = unset_default_tag(&path).unwrap();
        assert!(!removed, "key absent → false");

        // File unchanged.
        let raw = fs::read_to_string(&path).unwrap();
        let json: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(json["install_dir"], "C:\\py");
    }

    #[test]
    fn unset_default_tag_round_trips_with_read() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("pymanager.json");

        // write then unset then read.
        write_default_tag(&path, "3.14").unwrap();
        assert_eq!(read_default_tag(&path).unwrap(), Some("3.14".to_string()));

        assert!(unset_default_tag(&path).unwrap());
        assert_eq!(read_default_tag(&path).unwrap(), None);
    }

    #[test]
    fn unset_default_tag_returns_false_for_non_object_file() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("pymanager.json");
        fs::write(&path, r#"[1, 2, 3]"#).unwrap();

        let removed = unset_default_tag(&path).unwrap();
        assert!(!removed, "non-object file → no key to remove");
    }

    #[test]
    fn mock_unset_default_via_trait() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("pymanager.json");
        fs::write(&path, r#"{"default_tag": "3.13", "other": 1}"#).unwrap();

        let mut mock = MockPyManager::new(canned_runtimes(), path.clone());
        assert!(mock.unset_default().unwrap(), "key was present");
        assert!(
            !mock.unset_default().unwrap(),
            "second call: key now absent"
        );
    }
}
