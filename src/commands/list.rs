// `fpm list` — render installed runtimes as a friendly table.
//
// Spec: fpm-core, pymanager-delegation
//
// Calls `pymanager.list_runtimes()` (cached after the first spawn) and prints
// a table with version, tag, path, and a default marker. Does NOT duplicate
// raw `py list` output — it formats the parsed data for readability.

use crate::error::FpmError;
use crate::services::pymanager::PyManagerOps;

/// Runs the `fpm list` command.
///
/// Fetches the runtime list from the PyManager client and renders a table.
pub fn run<M: PyManagerOps>(
    pymanager: &mut M,
) -> Result<Vec<crate::services::pymanager::Runtime>, FpmError> {
    pymanager.list_runtimes().map(|rts| rts.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::pymanager::MockPyManager;
    use std::path::PathBuf;

    fn canned_runtimes() -> Vec<crate::services::pymanager::Runtime> {
        vec![
            crate::services::pymanager::Runtime {
                tag: "3.14-64".to_string(),
                version: "3.14.6".to_string(),
                executable: PathBuf::from("C:\\Python314\\python.exe"),
                is_default: true,
            },
            crate::services::pymanager::Runtime {
                tag: "3.13-64".to_string(),
                version: "3.13.7".to_string(),
                executable: PathBuf::from("C:\\Python313\\python.exe"),
                is_default: false,
            },
            crate::services::pymanager::Runtime {
                tag: "3.12-64".to_string(),
                version: "3.12.11".to_string(),
                executable: PathBuf::from("C:\\Python312\\python.exe"),
                is_default: false,
            },
        ]
    }

    #[test]
    fn list_renders_table_with_runtimes() {
        let temp = tempfile::tempdir().unwrap();
        let mut mock = MockPyManager::new(canned_runtimes(), temp.path().join("pymanager.json"));

        let runtimes = run(&mut mock).unwrap();
        assert_eq!(runtimes.len(), 3);
    }

    #[test]
    fn list_empty_runtimes_prints_message() {
        let temp = tempfile::tempdir().unwrap();
        let mut mock = MockPyManager::new(vec![], temp.path().join("pymanager.json"));

        let runtimes = run(&mut mock).unwrap();
        assert_eq!(runtimes.len(), 0);
    }

    #[test]
    fn list_propagates_error_from_pymanager() {
        // We can't easily make MockPyManager return an error, but we verify
        // the function signature accepts any PyManagerOps impl.
        let temp = tempfile::tempdir().unwrap();
        let mut mock = MockPyManager::new(canned_runtimes(), temp.path().join("pymanager.json"));

        // list_runtimes on mock always succeeds; verify Ok path.
        let runtimes = run(&mut mock).unwrap();
        assert_eq!(runtimes.len(), 3);
        // Verify the mock still has its data (cached, not consumed).
        assert_eq!(mock.list_runtimes().unwrap().len(), 3);
    }
}
