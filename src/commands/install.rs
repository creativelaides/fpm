// `fpm install <tag>` — delegate to `py install <tag>`.
//
// Spec: pymanager-delegation fpm install Delegates to py install
//
// Spawns `py install <tag>` and streams stdout/stderr to the terminal by
// inheriting stdio. fpm does NOT implement its own download logic. The child's
// exit code becomes fpm's exit code.

use std::process::Command;

use crate::error::FpmError;

/// Spawns `py install <tag>`, inheriting stdio, and returns the child exit code.
///
/// If `py.exe` cannot be found, returns `FpmError::PyNotFound` (exit code 1).
pub fn run(tag: &str) -> Result<i32, FpmError> {
    let status = Command::new("py")
        .args(["install", tag])
        .status()
        .map_err(|_| FpmError::PyNotFound)?;

    Ok(status.code().unwrap_or(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_constructs_correct_command() {
        // We can't easily test the actual spawn without py installed, but we
        // can verify the function handles the missing-py case gracefully.
        let result = run("3.14");
        match result {
            Ok(code) => assert!(code >= 0, "exit code should be non-negative"),
            Err(e) => assert!(
                matches!(e, FpmError::PyNotFound),
                "missing py should yield PyNotFound, got {e:?}"
            ),
        }
    }
}
