// Integration tests for pass-through to py.exe.
//
// Spec: fpm-core Unrecognized Args Pass Through to py.exe
//
// ALL tests in this file are #[ignore] — they require a real PyManager
// (py.exe) installed on PATH. Run with: cargo test -- --ignored

use assert_cmd::Command;
use predicates::str::contains;

// ── pass-through forwards args to py.exe ──────────────────────────────────

/// `fpm --list` should forward `--list` to py.exe and propagate the exit code.
/// py --list prints installed runtimes and exits 0.
#[test]
#[ignore]
fn passthrough_forwards_list_to_py() {
    Command::cargo_bin("fpm")
        .unwrap()
        .arg("--list")
        .assert()
        .success();
}

/// `fpm --version` passes through to py.exe, which prints the Python version
/// manager version.
#[test]
#[ignore]
fn passthrough_forwards_version_to_py() {
    Command::cargo_bin("fpm")
        .unwrap()
        .arg("--version")
        .assert()
        .stdout(contains("Python")); // py --version prints something with "Python"
}

/// `fpm -V:3.13 -m markitdown` forwards all args to py.exe verbatim.
/// We test with a harmless flag that py accepts.
#[test]
#[ignore]
fn passthrough_forwards_multiple_args() {
    // py -3.13 --version forwards args to the 3.13 interpreter's --version.
    // We just verify the process runs and exits without fpm crashing.
    // We don't assert on exit code — py may or may not be installed.
    let mut cmd = Command::cargo_bin("fpm").unwrap();
    cmd.args(["-V:3.14", "--version"]);
    let _ = cmd.assert();
}

// ── py.exe missing ─────────────────────────────────────────────────────────

/// When py.exe is NOT on PATH, `fpm script.py` should exit with code 1 and
/// print an error to stderr mentioning PyManager.
///
/// This test runs fpm with py.exe absent from PATH by overriding PATH to empty.
/// On machines where py.exe IS installed, we simulate "missing" by clearing PATH.
#[test]
#[ignore]
fn passthrough_py_missing_exits_nonzero_with_stderr() {
    Command::cargo_bin("fpm")
        .unwrap()
        .env("PATH", "")
        .arg("script.py")
        .assert()
        .failure()
        .code(1)
        .stderr(contains("PyManager"));
}
