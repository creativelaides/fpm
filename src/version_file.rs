// Version file resolution for `fpm use` (no explicit tag).
//
// Spec: version-file-resolution
//
// Walks upward from cwd, checking each directory for version sources in order:
//   1. `.python-version` — exact tag (first non-empty, non-comment line)
//   2. `pyproject.toml` — specifier from `requires-python` (PEP 621) or
//      `[tool.poetry.dependencies] python` (Poetry); reduced to the highest
//      installed runtime matching the specifier
//
// `.python-version` takes precedence over `pyproject.toml` within the same
// directory. An empty `.python-version` is treated as "no version declared"
// and the walk continues. A malformed `pyproject.toml` is skipped and the walk
// continues.

use std::path::Path;
use std::str::FromStr;

use pep440_rs::{Version, VersionSpecifiers};

use crate::error::FpmError;
use crate::pymanager::{PyManagerOps, Runtime};

/// Resolves a Python version tag for `fpm use` (no explicit argument).
///
/// Walks upward from `cwd`. At each directory, checks `.python-version` first
/// (exact tag), then `pyproject.toml` (specifier reduced to the highest
/// installed runtime). Returns the first match found, or
/// `FpmError::NoVersionFile` if none exists up to the filesystem root.
///
/// `pymanager` supplies the installed-runtimes list used to reduce
/// `pyproject.toml` specifiers to a concrete tag.
pub fn resolve<M: PyManagerOps>(cwd: &Path, pymanager: &mut M) -> Result<String, FpmError> {
    let mut dir: Option<&Path> = Some(cwd);

    while let Some(current) = dir {
        // 1. .python-version (exact tag, precedence)
        if let Some(tag) = parse_python_version_file(current)? {
            return Ok(tag);
        }

        // 2. pyproject.toml (specifier → highest matching runtime)
        if let Some(tag) = resolve_from_pyproject(current, pymanager)? {
            return Ok(tag);
        }

        dir = current.parent();
    }

    Err(FpmError::NoVersionFile)
}

/// Reads `.python-version` in `dir` and returns the exact tag, if declared.
///
/// Returns `Ok(None)` when the file is absent OR empty/whitespace/comments
/// only (treated as "no version declared").
fn parse_python_version_file(dir: &Path) -> Result<Option<String>, FpmError> {
    let path = dir.join(".python-version");
    let contents = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(FpmError::ConfigError(e.to_string())),
    };

    Ok(parse_python_version_text(&contents))
}

/// Extracts the first non-empty, non-comment line from `.python-version`
/// content, trimmed of whitespace. Returns `None` if no such line exists.
fn parse_python_version_text(contents: &str) -> Option<String> {
    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with('#') {
            continue;
        }
        return Some(trimmed.to_string());
    }
    None
}

/// Reads `pyproject.toml` in `dir` and reduces the specifier to the highest
/// matching installed runtime tag.
///
/// Returns `Ok(None)` when the file is absent, has no python specifier, or is
/// malformed (malformed TOML is skipped without error per the spec).
fn resolve_from_pyproject<M: PyManagerOps>(
    dir: &Path,
    pymanager: &mut M,
) -> Result<Option<String>, FpmError> {
    let path = dir.join("pyproject.toml");
    let contents = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        // Malformed/unreadable: skip and continue upward.
        Err(_) => return Ok(None),
    };

    let specifier = match extract_python_specifier(&contents) {
        Ok(Some(s)) => s,
        Ok(None) => return Ok(None),
        // Malformed TOML: skip this file, keep walking.
        Err(_) => return Ok(None),
    };

    let runtimes = pymanager.list_runtimes()?;
    let tag = reduce_specifier(&specifier, runtimes)?;
    Ok(tag)
}

/// Extracts the python version specifier from `pyproject.toml` content.
///
/// Checks PEP 621 `[project] requires-python` first, then Poetry
/// `[tool.poetry.dependencies] python`. Returns `Ok(None)` if neither is
/// present. Returns `Err` if the content is not valid TOML.
fn extract_python_specifier(contents: &str) -> Result<Option<String>, FpmError> {
    let doc: toml::Value =
        toml::from_str(contents).map_err(|e| FpmError::ConfigError(e.to_string()))?;

    // PEP 621: [project] requires-python
    if let Some(requires_python) = doc
        .get("project")
        .and_then(|p| p.get("requires-python"))
        .and_then(|v| v.as_str())
    {
        return Ok(Some(requires_python.to_string()));
    }

    // Poetry: [tool.poetry.dependencies] python
    if let Some(python) = doc
        .get("tool")
        .and_then(|t| t.get("poetry"))
        .and_then(|p| p.get("dependencies"))
        .and_then(|d| d.get("python"))
    {
        // Poetry python can be a string (specifier) or a table with "version".
        if let Some(s) = python.as_str() {
            return Ok(Some(s.to_string()));
        }
        if let Some(version) = python.get("version").and_then(|v| v.as_str()) {
            return Ok(Some(version.to_string()));
        }
    }

    Ok(None)
}

/// Reduces a PEP 440 specifier against the installed runtimes, returning the
/// tag of the highest matching runtime.
///
/// Each runtime's `version` field is parsed to a `pep440_rs::Version`. The
/// highest version satisfying the specifier wins. Returns
/// `FpmError::SpecNotSatisfied` when no runtime matches.
fn reduce_specifier(specifier: &str, runtimes: &[Runtime]) -> Result<Option<String>, FpmError> {
    let specifiers =
        VersionSpecifiers::from_str(specifier).map_err(|_| FpmError::SpecNotSatisfied {
            specifier: specifier.to_string(),
        })?;

    let mut best: Option<(Version, String)> = None;

    for runtime in runtimes {
        // Runtime tags like "3.14-64" can't be parsed directly as a Version;
        // use the bare version field ("3.14.6") for specifier matching.
        let Ok(version) = Version::from_str(&runtime.version) else {
            continue;
        };

        if !specifiers.contains(&version) {
            continue;
        }

        let is_higher = best
            .as_ref()
            .map(|(best_version, _)| version > *best_version)
            .unwrap_or(true);

        if is_higher {
            best = Some((version, runtime.tag.clone()));
        }
    }

    match best {
        Some((_, tag)) => Ok(Some(tag)),
        None => Err(FpmError::SpecNotSatisfied {
            specifier: specifier.to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pymanager::MockPyManager;
    use std::fs;
    use std::path::PathBuf;

    /// Canned runtimes simulating `py list` output for specifier tests.
    fn canned_runtimes() -> Vec<Runtime> {
        vec![
            Runtime {
                tag: "3.11-64".to_string(),
                version: "3.11.9".to_string(),
                executable: PathBuf::from("C:\\Python311\\python.exe"),
                is_default: false,
            },
            Runtime {
                tag: "3.13-64".to_string(),
                version: "3.13.7".to_string(),
                executable: PathBuf::from("C:\\Python313\\python.exe"),
                is_default: false,
            },
            Runtime {
                tag: "3.14-64".to_string(),
                version: "3.14.6".to_string(),
                executable: PathBuf::from("C:\\Python314\\python.exe"),
                is_default: true,
            },
        ]
    }

    fn mock_manager(dir: &Path) -> MockPyManager {
        MockPyManager::new(canned_runtimes(), dir.join("pymanager.json"))
    }

    // -------------------------------------------------------------------------
    // .python-version parsing
    // -------------------------------------------------------------------------

    #[test]
    fn python_version_plain_line() {
        assert_eq!(
            parse_python_version_text("3.13\n"),
            Some("3.13".to_string())
        );
    }

    #[test]
    fn python_version_ignores_comments_and_blanks() {
        assert_eq!(
            parse_python_version_text("# project python\n\n3.14\n"),
            Some("3.14".to_string())
        );
    }

    #[test]
    fn python_version_empty_file_returns_none() {
        assert_eq!(parse_python_version_text(""), None);
    }

    #[test]
    fn python_version_only_comments_returns_none() {
        assert_eq!(
            parse_python_version_text("# just a comment\n# another\n"),
            None
        );
    }

    #[test]
    fn python_version_trims_whitespace() {
        assert_eq!(
            parse_python_version_text("   3.12   \n"),
            Some("3.12".to_string())
        );
    }

    #[test]
    fn python_version_inline_comment_not_stripped() {
        // A `#` not at line start is part of the value (uncommon but literal).
        assert_eq!(
            parse_python_version_text("3.12 # not stripped\n"),
            Some("3.12 # not stripped".to_string())
        );
    }

    // -------------------------------------------------------------------------
    // pyproject.toml specifier extraction
    // -------------------------------------------------------------------------

    #[test]
    fn pyproject_extracts_pep621_requires_python() {
        let toml = r#"
[project]
name = "demo"
requires-python = ">=3.12"
"#;
        assert_eq!(
            extract_python_specifier(toml).unwrap(),
            Some(">=3.12".to_string())
        );
    }

    #[test]
    fn pyproject_extracts_poetry_string_dependency() {
        let toml = r#"
[tool.poetry.dependencies]
python = ">=3.12,<4.0"
"#;
        assert_eq!(
            extract_python_specifier(toml).unwrap(),
            Some(">=3.12,<4.0".to_string())
        );
    }

    #[test]
    fn pyproject_extracts_poetry_table_dependency() {
        let toml = r#"
[tool.poetry.dependencies]
python = { version = ">=3.12,<4.0", markers = "sys_platform == 'win32'" }
"#;
        assert_eq!(
            extract_python_specifier(toml).unwrap(),
            Some(">=3.12,<4.0".to_string())
        );
    }

    #[test]
    fn pyproject_no_specifier_returns_none() {
        let toml = r#"
[project]
name = "demo"
"#;
        assert_eq!(extract_python_specifier(toml).unwrap(), None);
    }

    #[test]
    fn pyproject_malformed_toml_errors() {
        let result = extract_python_specifier("not = valid = toml =");
        assert!(result.is_err(), "malformed TOML should error");
    }

    // -------------------------------------------------------------------------
    // specifier reduction
    // -------------------------------------------------------------------------

    #[test]
    fn reduce_lower_bound_selects_highest() {
        let tag = reduce_specifier(">=3.12", &canned_runtimes()).unwrap();
        assert_eq!(tag, Some("3.14-64".to_string()));
    }

    #[test]
    fn reduce_pinned_wildcard_selects_match() {
        let tag = reduce_specifier("==3.13.*", &canned_runtimes()).unwrap();
        assert_eq!(tag, Some("3.13-64".to_string()));
    }

    #[test]
    fn reduce_tilde_selects_minor_floor() {
        // ~=3.13.0 means >=3.13.0, <3.14.0 — pins to the 3.13 line.
        let tag = reduce_specifier("~=3.13.0", &canned_runtimes()).unwrap();
        assert_eq!(tag, Some("3.13-64".to_string()));
    }

    #[test]
    fn reduce_unsatisfiable_errors() {
        let err = reduce_specifier(">=3.20", &canned_runtimes()).unwrap_err();
        assert!(matches!(err, FpmError::SpecNotSatisfied { .. }));
        assert_eq!(err.exit_code(), 4);
    }

    // -------------------------------------------------------------------------
    // resolve() upward walk
    // -------------------------------------------------------------------------

    #[test]
    fn resolve_cwd_python_version_wins() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();

        // cwd has .python-version; parent has pyproject.toml.
        let cwd = root.join("child");
        fs::create_dir_all(&cwd).unwrap();
        fs::write(cwd.join(".python-version"), "3.13\n").unwrap();
        fs::write(
            root.join("pyproject.toml"),
            r#"[project]
requires-python = ">=3.14"
"#,
        )
        .unwrap();

        let mut mgr = mock_manager(root);
        let tag = resolve(&cwd, &mut mgr).unwrap();
        assert_eq!(tag, "3.13");
    }

    #[test]
    fn resolve_finds_python_version_in_ancestor() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();

        // Grandparent has .python-version; cwd and parent are empty.
        let cwd = root.join("a").join("b").join("c");
        fs::create_dir_all(&cwd).unwrap();
        fs::write(root.join(".python-version"), "3.12\n").unwrap();

        let mut mgr = mock_manager(root);
        let tag = resolve(&cwd, &mut mgr).unwrap();
        assert_eq!(tag, "3.12");
    }

    #[test]
    fn resolve_falls_back_to_pyproject_in_ancestor() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();

        let cwd = root.join("child");
        fs::create_dir_all(&cwd).unwrap();
        fs::write(
            root.join("pyproject.toml"),
            r#"[project]
requires-python = ">=3.12"
"#,
        )
        .unwrap();

        let mut mgr = mock_manager(root);
        let tag = resolve(&cwd, &mut mgr).unwrap();
        assert_eq!(tag, "3.14-64");
    }

    #[test]
    fn resolve_no_file_returns_no_version_file_error() {
        let temp = tempfile::tempdir().unwrap();
        let cwd = temp.path().join("deep");
        fs::create_dir_all(&cwd).unwrap();

        let mut mgr = mock_manager(temp.path());
        let err = resolve(&cwd, &mut mgr).unwrap_err();
        assert!(matches!(err, FpmError::NoVersionFile));
        assert_eq!(err.exit_code(), 3);
    }

    #[test]
    fn resolve_python_version_precedence_over_pyproject_same_dir() {
        let temp = tempfile::tempdir().unwrap();
        let dir = temp.path();

        fs::write(dir.join(".python-version"), "3.13\n").unwrap();
        fs::write(
            dir.join("pyproject.toml"),
            r#"[project]
requires-python = ">=3.14"
"#,
        )
        .unwrap();

        let mut mgr = mock_manager(dir);
        let tag = resolve(dir, &mut mgr).unwrap();
        assert_eq!(tag, "3.13");
    }

    #[test]
    fn resolve_empty_python_version_falls_through_to_pyproject() {
        let temp = tempfile::tempdir().unwrap();
        let dir = temp.path();

        // Empty .python-version → treat as not declaring → check pyproject.toml.
        fs::write(dir.join(".python-version"), "\n# only a comment\n").unwrap();
        fs::write(
            dir.join("pyproject.toml"),
            r#"[project]
requires-python = ">=3.12"
"#,
        )
        .unwrap();

        let mut mgr = mock_manager(dir);
        let tag = resolve(dir, &mut mgr).unwrap();
        assert_eq!(tag, "3.14-64");
    }

    #[test]
    fn resolve_malformed_pyproject_skipped_continues_upward() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();

        let cwd = root.join("child");
        fs::create_dir_all(&cwd).unwrap();
        // Malformed pyproject in cwd should be skipped...
        fs::write(cwd.join("pyproject.toml"), "not = valid = toml =").unwrap();
        // ...and .python-version in root should be found.
        fs::write(root.join(".python-version"), "3.12\n").unwrap();

        let mut mgr = mock_manager(root);
        let tag = resolve(&cwd, &mut mgr).unwrap();
        assert_eq!(tag, "3.12");
    }

    #[test]
    fn resolve_unsatisfiable_specifier_errors() {
        let temp = tempfile::tempdir().unwrap();
        let dir = temp.path();

        fs::write(
            dir.join("pyproject.toml"),
            r#"[project]
requires-python = ">=3.20"
"#,
        )
        .unwrap();

        let mut mgr = mock_manager(dir);
        let err = resolve(dir, &mut mgr).unwrap_err();
        assert!(matches!(err, FpmError::SpecNotSatisfied { .. }));
        assert_eq!(err.exit_code(), 4);
    }

    #[test]
    fn resolve_poetry_specifier_in_cwd() {
        let temp = tempfile::tempdir().unwrap();
        let dir = temp.path();

        fs::write(
            dir.join("pyproject.toml"),
            r#"
[tool.poetry.dependencies]
python = ">=3.12,<4.0"
"#,
        )
        .unwrap();

        let mut mgr = mock_manager(dir);
        let tag = resolve(dir, &mut mgr).unwrap();
        assert_eq!(tag, "3.14-64");
    }
}
