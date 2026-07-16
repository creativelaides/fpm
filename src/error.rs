// fpm domain errors and exit-code mapping.
//
// Spec: fpm-core Exit Code Propagation
// Each variant maps to a distinct exit code (1-6) so callers and scripts can
// distinguish failure modes without parsing stderr.

use std::io;
use thiserror::Error;

/// Domain errors emitted by fpm.
///
/// Each variant maps to a stable exit code via [`FpmError::exit_code`].
/// Pass-through and delegated commands propagate the child process exit code
/// directly and never produce an `FpmError`.
#[derive(Debug, Error)]
pub enum FpmError {
    /// `py.exe` was not found on PATH.
    /// Exit code: 1
    #[error("PyManager 26.x+ is required (py.exe not found on PATH)")]
    PyNotFound,

    /// The requested version tag is not installed.
    /// Exit code: 2
    #[error("Version {tag} is not installed")]
    VersionNotInstalled { tag: String },

    /// No `.python-version` or `pyproject.toml` was found walking up from cwd.
    /// Exit code: 3
    #[error("No .python-version or pyproject.toml found walking up from cwd")]
    NoVersionFile,

    /// No installed runtime satisfies the version specifier.
    /// Exit code: 4
    #[error("No installed runtime satisfies {specifier}")]
    SpecNotSatisfied { specifier: String },

    /// Failed to create or retarget the session shim junction.
    /// Exit code: 5
    #[error("Failed to update session shim: {0}")]
    ShimError(#[source] io::Error),

    /// Failed to read or write `pymanager.json`.
    /// Exit code: 6
    #[error("Failed to read/write pymanager.json: {0}")]
    ConfigError(String),
}

impl FpmError {
    /// Maps this error to the stable exit code defined by the fpm-core spec.
    ///
    /// | Variant            | Code |
    /// |--------------------|------|
    /// | PyNotFound         | 1    |
    /// | VersionNotInstalled| 2    |
    /// | NoVersionFile      | 3    |
    /// | SpecNotSatisfied   | 4    |
    /// | ShimError          | 5    |
    /// | ConfigError        | 6    |
    pub fn exit_code(&self) -> i32 {
        match self {
            FpmError::PyNotFound => 1,
            FpmError::VersionNotInstalled { .. } => 2,
            FpmError::NoVersionFile => 3,
            FpmError::SpecNotSatisfied { .. } => 4,
            FpmError::ShimError(_) => 5,
            FpmError::ConfigError(_) => 6,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pynotfound_maps_to_1() {
        let err = FpmError::PyNotFound;
        assert_eq!(err.exit_code(), 1);
    }

    #[test]
    fn version_not_installed_maps_to_2() {
        let err = FpmError::VersionNotInstalled {
            tag: "3.14".to_string(),
        };
        assert_eq!(err.exit_code(), 2);
    }

    #[test]
    fn no_version_file_maps_to_3() {
        let err = FpmError::NoVersionFile;
        assert_eq!(err.exit_code(), 3);
    }

    #[test]
    fn spec_not_satisfied_maps_to_4() {
        let err = FpmError::SpecNotSatisfied {
            specifier: ">=3.15".to_string(),
        };
        assert_eq!(err.exit_code(), 4);
    }

    #[test]
    fn shim_error_maps_to_5() {
        let err = FpmError::ShimError(io::Error::new(io::ErrorKind::Other, "boom"));
        assert_eq!(err.exit_code(), 5);
    }

    #[test]
    fn config_error_maps_to_6() {
        let err = FpmError::ConfigError("permission denied".to_string());
        assert_eq!(err.exit_code(), 6);
    }

    #[test]
    fn all_exit_codes_are_distinct_and_in_range() {
        let codes = [
            FpmError::PyNotFound.exit_code(),
            FpmError::VersionNotInstalled { tag: String::new() }.exit_code(),
            FpmError::NoVersionFile.exit_code(),
            FpmError::SpecNotSatisfied { specifier: String::new() }.exit_code(),
            FpmError::ShimError(io::Error::new(io::ErrorKind::Other, "x")).exit_code(),
            FpmError::ConfigError(String::new()).exit_code(),
        ];
        // Each code is unique
        let unique: std::collections::HashSet<i32> = codes.iter().copied().collect();
        assert_eq!(unique.len(), 6, "exit codes must be distinct");
        // All in 1..=6
        for c in &codes {
            assert!(*c >= 1 && *c <= 6, "exit code {} out of range 1-6", c);
        }
    }

    #[test]
    fn error_messages_are_human_readable() {
        assert_eq!(
            FpmError::PyNotFound.to_string(),
            "PyManager 26.x+ is required (py.exe not found on PATH)"
        );
        assert_eq!(
            FpmError::VersionNotInstalled { tag: "3.14".into() }.to_string(),
            "Version 3.14 is not installed"
        );
        assert_eq!(
            FpmError::SpecNotSatisfied { specifier: ">=3.15".into() }.to_string(),
            "No installed runtime satisfies >=3.15"
        );
    }
}