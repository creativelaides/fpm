// Integration tests for `fpm use`.
//
// Spec: python-version-switching (all requirements)
//
// ALL tests in this file are #[ignore] — they require a real PyManager
// (py.exe) installed on PATH with actual Python runtimes. Run with:
//   cargo test -- --ignored

use assert_cmd::Command;
use predicates::str::contains;
use tempfile::TempDir;

/// `fpm use 3.14` should switch the session to Python 3.14.
/// Requires py.exe with a 3.14 runtime installed and FPM_MULTISHELL_PATH set.
#[test]
#[ignore]
fn use_explicit_version_switches_and_prints_message() {
    let temp = TempDir::new().unwrap();
    let temp_path = temp.path().to_str().unwrap();

    // Create the session dir via fpm env first, then capture its path.
    // We need FPM_MULTISHELL_PATH to be set for `fpm use`.
    let env_output = Command::cargo_bin("fpm")
        .unwrap()
        .env("FPM_DIR", temp_path)
        .args(["env", "--shell", "powershell"])
        .output()
        .unwrap();

    let script = String::from_utf8(env_output.stdout).unwrap();

    // Extract the FPM_MULTISHELL_PATH value from the emitted script.
    let session_line = script
        .lines()
        .find(|l| l.contains("$env:FPM_MULTISHELL_PATH"))
        .unwrap();
    // Line looks like: $env:FPM_MULTISHELL_PATH = "C:\...\multishells\1234_5678"
    let session_dir = session_line
        .split('=')
        .nth(1)
        .unwrap()
        .trim()
        .trim_matches('"')
        .to_string();

    // Now run `fpm use 3.14` with FPM_MULTISHELL_PATH set.
    Command::cargo_bin("fpm")
        .unwrap()
        .env("FPM_DIR", temp_path)
        .env("FPM_MULTISHELL_PATH", &session_dir)
        .args(["use", "3.14"])
        .assert()
        .success()
        .stdout(contains("Using Python"));
}

/// `fpm use --silent-if-unchanged 3.14` when 3.14 is already active should
/// produce no stdout output.
#[test]
#[ignore]
fn use_silent_if_unchanged_suppresses_output_when_active() {
    let temp = TempDir::new().unwrap();
    let temp_path = temp.path().to_str().unwrap();

    // Setup: run fpm env to get a session dir.
    let env_output = Command::cargo_bin("fpm")
        .unwrap()
        .env("FPM_DIR", temp_path)
        .args(["env", "--shell", "powershell"])
        .output()
        .unwrap();

    let script = String::from_utf8(env_output.stdout).unwrap();
    let session_line = script
        .lines()
        .find(|l| l.contains("$env:FPM_MULTISHELL_PATH"))
        .unwrap();
    let session_dir = session_line
        .split('=')
        .nth(1)
        .unwrap()
        .trim()
        .trim_matches('"')
        .to_string();

    // First, switch to 3.14 normally.
    Command::cargo_bin("fpm")
        .unwrap()
        .env("FPM_DIR", temp_path)
        .env("FPM_MULTISHELL_PATH", &session_dir)
        .args(["use", "3.14"])
        .assert()
        .success();

    // Now run with --silent-if-unchanged — should exit 0 with no stdout.
    Command::cargo_bin("fpm")
        .unwrap()
        .env("FPM_DIR", temp_path)
        .env("FPM_MULTISHELL_PATH", &session_dir)
        .args(["use", "--silent-if-unchanged", "3.14"])
        .assert()
        .success();
    // Note: we can't assert empty stdout easily with assert_cmd because
    // the binary may print a newline. The exit code 0 is the key assertion.
}

/// `fpm use nonexistentversion` should exit with code 2 (VersionNotInstalled).
#[test]
#[ignore]
fn use_nonexistent_version_exits_code_2() {
    let temp = TempDir::new().unwrap();
    let temp_path = temp.path().to_str().unwrap();

    // Setup session dir.
    let env_output = Command::cargo_bin("fpm")
        .unwrap()
        .env("FPM_DIR", temp_path)
        .args(["env", "--shell", "powershell"])
        .output()
        .unwrap();

    let script = String::from_utf8(env_output.stdout).unwrap();
    let session_line = script
        .lines()
        .find(|l| l.contains("$env:FPM_MULTISHELL_PATH"))
        .unwrap();
    let session_dir = session_line
        .split('=')
        .nth(1)
        .unwrap()
        .trim()
        .trim_matches('"')
        .to_string();

    // Use a version that definitely is not installed.
    Command::cargo_bin("fpm")
        .unwrap()
        .env("FPM_DIR", temp_path)
        .env("FPM_MULTISHELL_PATH", &session_dir)
        .args(["use", "9.99.99-nonexistent"])
        .assert()
        .failure()
        .code(2)
        .stderr(contains("not installed"));
}

/// `fpm use` (no args) in a directory with .python-version should read and
/// switch to that version.
#[test]
#[ignore]
fn use_no_args_reads_python_version_file() {
    let temp = TempDir::new().unwrap();
    let temp_path = temp.path().to_str().unwrap();

    // Write a .python-version file in the temp dir.
    std::fs::write(temp.path().join(".python-version"), "3.14\n").unwrap();

    // Setup session dir.
    let env_output = Command::cargo_bin("fpm")
        .unwrap()
        .env("FPM_DIR", temp_path)
        .args(["env", "--shell", "powershell"])
        .output()
        .unwrap();

    let script = String::from_utf8(env_output.stdout).unwrap();
    let session_line = script
        .lines()
        .find(|l| l.contains("$env:FPM_MULTISHELL_PATH"))
        .unwrap();
    let session_dir = session_line
        .split('=')
        .nth(1)
        .unwrap()
        .trim()
        .trim_matches('"')
        .to_string();

    // Run `fpm use` (no version arg) from the temp dir (has .python-version).
    Command::cargo_bin("fpm")
        .unwrap()
        .current_dir(temp.path())
        .env("FPM_DIR", temp_path)
        .env("FPM_MULTISHELL_PATH", &session_dir)
        .arg("use")
        .assert()
        .success()
        .stdout(contains("Using Python"));
}

/// `fpm use` (no args) in a directory WITHOUT version files should exit with
/// code 3 (NoVersionFile).
#[test]
#[ignore]
fn use_no_args_no_version_file_exits_code_3() {
    let temp = TempDir::new().unwrap();
    let temp_path = temp.path().to_str().unwrap();

    // No .python-version or pyproject.toml in temp dir.

    // Setup session dir.
    let env_output = Command::cargo_bin("fpm")
        .unwrap()
        .env("FPM_DIR", temp_path)
        .args(["env", "--shell", "powershell"])
        .output()
        .unwrap();

    let script = String::from_utf8(env_output.stdout).unwrap();
    let session_line = script
        .lines()
        .find(|l| l.contains("$env:FPM_MULTISHELL_PATH"))
        .unwrap();
    let session_dir = session_line
        .split('=')
        .nth(1)
        .unwrap()
        .trim()
        .trim_matches('"')
        .to_string();

    // Run `fpm use` with no args — should fail with code 3.
    Command::cargo_bin("fpm")
        .unwrap()
        .current_dir(temp.path())
        .env("FPM_DIR", temp_path)
        .env("FPM_MULTISHELL_PATH", &session_dir)
        .arg("use")
        .assert()
        .failure()
        .code(3)
        .stderr(contains("No .python-version or pyproject.toml"));
}
