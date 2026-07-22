// Integration tests for `fpm env --shell powershell`.
//
// Spec: powershell-shell-integration fpm env Emits PowerShell Setup Script
//
// These tests do NOT require py.exe — `fpm env` only creates a session
// directory and emits a PowerShell script. We use a temp FPM_DIR to avoid
// touching the user's real fpm data directory.

use assert_cmd::Command;
use predicates::str::contains;
use tempfile::TempDir;

// ── fpm env --shell powershell (no --use-on-cd) ─────────────────────────────

#[test]
fn env_powershell_emits_expected_env_vars() {
    let temp = TempDir::new().unwrap();
    let temp_path = temp.path().to_str().unwrap();

    Command::cargo_bin("fpy")
        .unwrap()
        .env("FPY_DIR", temp_path)
        .args(["env", "--shell", "powershell"])
        .assert()
        .success()
        .stdout(contains("$env:FPY_DIR"))
        .stdout(contains("$env:FPY_MULTISHELL_PATH"))
        .stdout(contains("$env:PATH"));
}

#[test]
fn env_powershell_prepends_session_dir_to_path() {
    let temp = TempDir::new().unwrap();
    let temp_path = temp.path().to_str().unwrap();

    // The PATH prepend should reference the multishells session directory.
    Command::cargo_bin("fpy")
        .unwrap()
        .env("FPY_DIR", temp_path)
        .args(["env", "--shell", "powershell"])
        .assert()
        .success()
        .stdout(contains("multishells"))
        .stdout(contains(";$env:PATH"));
}

#[test]
fn env_powershell_creates_session_directory() {
    let temp = TempDir::new().unwrap();
    let temp_path = temp.path().to_str().unwrap();

    Command::cargo_bin("fpy")
        .unwrap()
        .env("FPY_DIR", temp_path)
        .args(["env", "--shell", "powershell"])
        .assert()
        .success();

    // Verify the session directory was created under temp/multishells/.
    let multishells = temp.path().join("multishells");
    assert!(
        multishells.exists(),
        "multishells directory should be created under FPM_DIR"
    );

    let entries: Vec<_> = std::fs::read_dir(&multishells)
        .unwrap()
        .map(|e| e.unwrap())
        .collect();
    assert!(
        !entries.is_empty(),
        "at least one session dir should exist under multishells/"
    );
}

#[test]
fn env_powershell_emits_cleanup_hook() {
    let temp = TempDir::new().unwrap();
    let temp_path = temp.path().to_str().unwrap();

    Command::cargo_bin("fpy")
        .unwrap()
        .env("FPY_DIR", temp_path)
        .args(["env", "--shell", "powershell"])
        .assert()
        .success()
        .stdout(contains("Register-EngineEvent PowerShell.Exiting"))
        .stdout(contains("Remove-Item"));
}

// ── fpm env --shell powershell --use-on-cd ─────────────────────────────────

#[test]
fn env_powershell_use_on_cd_emits_set_location_hook() {
    let temp = TempDir::new().unwrap();
    let temp_path = temp.path().to_str().unwrap();

    Command::cargo_bin("fpy")
        .unwrap()
        .env("FPY_DIR", temp_path)
        .args(["env", "--shell", "powershell", "--use-on-cd"])
        .assert()
        .success()
        .stdout(contains("function global:Set-Location"))
        .stdout(contains("fpy use --silent-if-unchanged"))
        .stdout(contains(".python-version"));
}

#[test]
fn env_powershell_use_on_cd_still_has_cleanup_hook() {
    let temp = TempDir::new().unwrap();
    let temp_path = temp.path().to_str().unwrap();

    Command::cargo_bin("fpy")
        .unwrap()
        .env("FPY_DIR", temp_path)
        .args(["env", "--shell", "powershell", "--use-on-cd"])
        .assert()
        .success()
        .stdout(contains("Register-EngineEvent PowerShell.Exiting"));
}

#[test]
fn env_powershell_without_use_on_cd_omits_set_location_hook() {
    let temp = TempDir::new().unwrap();
    let temp_path = temp.path().to_str().unwrap();

    Command::cargo_bin("fpy")
        .unwrap()
        .env("FPY_DIR", temp_path)
        .args(["env", "--shell", "powershell"])
        .assert()
        .success()
        // The Set-Location override should NOT be present without --use-on-cd.
        .stdout(contains("Register-EngineEvent PowerShell.Exiting"))
        .stdout(contains("Remove-Item"));
}

// ── session directory uniqueness ────────────────────────────────────────────

#[test]
fn env_creates_unique_session_each_invocation() {
    let temp = TempDir::new().unwrap();
    let temp_path = temp.path().to_str().unwrap();

    // Run fpm env twice — each should create a distinct session directory.
    Command::cargo_bin("fpy")
        .unwrap()
        .env("FPY_DIR", temp_path)
        .args(["env", "--shell", "powershell"])
        .assert()
        .success();

    Command::cargo_bin("fpy")
        .unwrap()
        .env("FPY_DIR", temp_path)
        .args(["env", "--shell", "powershell"])
        .assert()
        .success();

    let multishells = temp.path().join("multishells");
    let entries: Vec<_> = std::fs::read_dir(&multishells)
        .unwrap()
        .map(|e| e.unwrap())
        .collect();
    assert_eq!(
        entries.len(),
        2,
        "two env invocations should create two distinct session dirs"
    );
}
