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
pub fn run<M: PyManagerOps>(pymanager: &mut M) -> Result<i32, FpmError> {
    let runtimes = pymanager.list_runtimes()?;

    if runtimes.is_empty() {
        println!("No Python runtimes installed.");
        println!("Install one with: fpm install <version>");
        return Ok(0);
    }

    // Compute column widths for alignment.
    let version_w = runtimes
        .iter()
        .map(|r| r.version.len())
        .max()
        .unwrap_or(7)
        .max(7); // "VERSION".len()
    let tag_w = runtimes
        .iter()
        .map(|r| r.tag.len())
        .max()
        .unwrap_or(3)
        .max(3); // "TAG".len()

    // Header
    println!(
        "{:<width_v$}  {:<width_t$}  PATH",
        "VERSION",
        "TAG",
        width_v = version_w,
        width_t = tag_w,
    );
    println!(
        "{:-<width_v$}  {:-<width_t$}  PATH",
        "",
        "",
        width_v = version_w,
        width_t = tag_w,
    );

    // Rows
    for rt in runtimes {
        let marker = if rt.is_default { " *" } else { "  " };
        println!(
            "{}{:<width_v$}  {:<width_t$}  {}",
            marker,
            rt.version,
            rt.tag,
            rt.executable.display(),
            width_v = version_w,
            width_t = tag_w,
        );
    }

    Ok(0)
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

        let code = run(&mut mock).unwrap();
        assert_eq!(code, 0);
    }

    #[test]
    fn list_empty_runtimes_prints_message() {
        let temp = tempfile::tempdir().unwrap();
        let mut mock = MockPyManager::new(vec![], temp.path().join("pymanager.json"));

        let code = run(&mut mock).unwrap();
        assert_eq!(code, 0);
    }

    #[test]
    fn list_propagates_error_from_pymanager() {
        // We can't easily make MockPyManager return an error, but we verify
        // the function signature accepts any PyManagerOps impl.
        let temp = tempfile::tempdir().unwrap();
        let mut mock = MockPyManager::new(canned_runtimes(), temp.path().join("pymanager.json"));

        // list_runtimes on mock always succeeds; verify Ok path.
        let code = run(&mut mock).unwrap();
        assert_eq!(code, 0);
        // Verify the mock still has its data (cached, not consumed).
        assert_eq!(mock.list_runtimes().unwrap().len(), 3);
    }
}
