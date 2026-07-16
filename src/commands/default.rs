// `fpm default [tag]` — read or write the default_tag in pymanager.json.
//
// Spec: pymanager-delegation fpm default Reads and Writes pymanager.json
//
// With no argument: reads and prints `default_tag` from pymanager.json.
// With a tag argument: writes `default_tag` to pymanager.json, preserving all
// other keys. Creates the file if it doesn't exist.
//
// `fpm use` does NOT touch pymanager.json — only `fpm default` does.

use crate::error::FpmError;
use crate::pymanager::PyManagerOps;

/// Runs the `fpm default` command.
///
/// - `tag = None`: read and print the current `default_tag`.
/// - `tag = Some(tag)`: write `default_tag`, preserving other keys.
pub fn run<M: PyManagerOps>(pymanager: &mut M, tag: Option<&str>) -> Result<i32, FpmError> {
    match tag {
        None => {
            let current = pymanager.read_default()?;
            match current {
                Some(t) => {
                    println!("{t}");
                    Ok(0)
                }
                None => {
                    println!("No default Python configured.");
                    Ok(0)
                }
            }
        }
        Some(tag) => {
            pymanager.write_default(tag)?;
            println!("Default Python set to {tag}");
            Ok(0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pymanager::{MockPyManager, Runtime};
    use std::path::PathBuf;

    fn canned_runtimes() -> Vec<Runtime> {
        vec![Runtime {
            tag: "3.14-64".to_string(),
            version: "3.14.6".to_string(),
            executable: PathBuf::from("C:\\Python314\\python.exe"),
            is_default: true,
        }]
    }

    #[test]
    fn default_read_prints_tag_when_present() {
        let temp = tempfile::tempdir().unwrap();
        let config_path = temp.path().join("pymanager.json");
        std::fs::write(&config_path, r#"{"default_tag": "3.13", "other": 42}"#).unwrap();

        let mut mock = MockPyManager::new(canned_runtimes(), config_path);
        let code = run(&mut mock, None).unwrap();
        assert_eq!(code, 0);
    }

    #[test]
    fn default_read_prints_message_when_absent() {
        let temp = tempfile::tempdir().unwrap();
        let config_path = temp.path().join("pymanager.json");
        // No file created.

        let mut mock = MockPyManager::new(canned_runtimes(), config_path);
        let code = run(&mut mock, None).unwrap();
        assert_eq!(code, 0);
    }

    #[test]
    fn default_write_creates_file_when_missing() {
        let temp = tempfile::tempdir().unwrap();
        let config_path = temp.path().join("pymanager.json");

        let mut mock = MockPyManager::new(canned_runtimes(), config_path.clone());
        let code = run(&mut mock, Some("3.14")).unwrap();
        assert_eq!(code, 0);

        // Verify file was created with the right content.
        let raw = std::fs::read_to_string(&config_path).unwrap();
        let json: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(json["default_tag"], "3.14");
    }

    #[test]
    fn default_write_preserves_other_keys() {
        let temp = tempfile::tempdir().unwrap();
        let config_path = temp.path().join("pymanager.json");
        std::fs::write(
            &config_path,
            r#"{"default_tag": "3.13", "install_dir": "C:\\py"}"#,
        )
        .unwrap();

        let mut mock = MockPyManager::new(canned_runtimes(), config_path.clone());
        let code = run(&mut mock, Some("3.14")).unwrap();
        assert_eq!(code, 0);

        let raw = std::fs::read_to_string(&config_path).unwrap();
        let json: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(json["default_tag"], "3.14");
        assert_eq!(json["install_dir"], "C:\\py");
    }

    #[test]
    fn default_write_then_read_round_trips() {
        let temp = tempfile::tempdir().unwrap();
        let config_path = temp.path().join("pymanager.json");

        let mut mock = MockPyManager::new(canned_runtimes(), config_path);

        // Write
        let code = run(&mut mock, Some("3.12")).unwrap();
        assert_eq!(code, 0);

        // Read back
        let code = run(&mut mock, None).unwrap();
        assert_eq!(code, 0);
    }
}
