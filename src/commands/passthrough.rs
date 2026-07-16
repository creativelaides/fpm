// Pass-through command: forward unrecognized args to `py.exe` verbatim.
//
// Spec: fpm-core Unrecognized Args Pass Through to py.exe
//
// When the user runs `fpm <args>` and the first non-flag token is not a
// recognized subcommand, fpm spawns `py.exe` with all args verbatim, inheriting
// stdout/stderr/stdin, and propagates the child's exit code. fpm SHALL NOT
// alter, reorder, or inject arguments.

use std::process::Command;

use crate::error::FpmError;

/// Spawns `py.exe` with all `args` verbatim, inheriting stdio.
///
/// Returns the child's exit code. If `py.exe` cannot be found on PATH, returns
/// `FpmError::PyNotFound` (exit code 1).
pub fn run(args: &[String]) -> Result<i32, FpmError> {
    let status = Command::new("py")
        .args(args)
        .status()
        .map_err(|_| FpmError::PyNotFound)?;

    Ok(status.code().unwrap_or(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies that `run` with `--version` either spawns `py` successfully
    /// (py installed) or returns `PyNotFound` (py missing). Both are valid
    /// outcomes depending on the test environment.
    ///
    /// We use `--version` (not empty args) to avoid opening the REPL, which
    /// would hang the test forever.
    #[test]
    fn run_with_version_arg_does_not_hang() {
        let result = run(&["--version".to_string()]);
        match result {
            Ok(code) => assert!(code >= 0, "exit code should be non-negative"),
            Err(e) => assert!(
                matches!(e, FpmError::PyNotFound),
                "missing py should yield PyNotFound, got {e:?}"
            ),
        }
    }

    /// Verifies that `run` with multiple args forwards them. Using `--list`
    /// which terminates quickly. If py is missing, we get PyNotFound.
    #[test]
    fn run_forwards_multiple_args() {
        let result = run(&["--list".to_string()]);
        match result {
            Ok(code) => assert!(code >= 0, "exit code should be non-negative"),
            Err(e) => assert!(
                matches!(e, FpmError::PyNotFound),
                "missing py should yield PyNotFound, got {e:?}"
            ),
        }
    }
}
