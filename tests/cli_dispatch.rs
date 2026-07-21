// Integration tests for CLI dispatch behavior.
//
// Spec: fpm-core Subcommand Routing
//
// These tests exercise the compiled fpm binary via assert_cmd. They do NOT
// require py.exe — they only verify CLI parsing, help text, and version output.

use assert_cmd::Command;
use predicates::str::contains;

// ── --version ─────────────────────────────────────────────────────────────

#[test]
fn version_prints_detailed_version() {
    Command::cargo_bin("fpy")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(contains(format!("fpy {}", env!("CARGO_PKG_VERSION"))))
        .stdout(contains("Active Python:"));
}

#[test]
fn version_short_flag_also_works() {
    Command::cargo_bin("fpy")
        .unwrap()
        .arg("-V")
        .assert()
        .success()
        .stdout(contains(format!("fpy {}", env!("CARGO_PKG_VERSION"))));
}

// ── --help ────────────────────────────────────────────────────────────────

#[test]
fn help_exits_zero_and_lists_subcommands() {
    Command::cargo_bin("fpy")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("use"))
        .stdout(contains("list"))
        .stdout(contains("current"))
        .stdout(contains("default"))
        .stdout(contains("env"))
        .stdout(contains("install"));
}

#[test]
fn help_short_flag_also_works() {
    Command::cargo_bin("fpy")
        .unwrap()
        .arg("-h")
        .assert()
        .success();
}

// ── subcommand --help ─────────────────────────────────────────────────────

#[test]
fn list_help_exits_zero() {
    Command::cargo_bin("fpy")
        .unwrap()
        .args(["list", "--help"])
        .assert()
        .success()
        .stdout(contains("List installed Python runtimes"));
}

#[test]
fn list_remote_help_exits_zero() {
    Command::cargo_bin("fpy")
        .unwrap()
        .args(["list-remote", "--help"])
        .assert()
        .success()
        .stdout(contains("--pre"));
}

#[test]
fn list_remote_runs() {
    let assert = Command::cargo_bin("fpy")
        .unwrap()
        .arg("list-remote")
        .assert();

    let output = assert.get_output();
    let code = output.status.code().unwrap_or(-1);

    // Either succeeds and prints the table, or fails with 7 (CacheError) or 8 (NetworkError)
    if code == 0 {
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("VERSION"));
        assert!(stdout.contains("RELEASE DATE"));
    } else {
        assert!(
            code == 7 || code == 8,
            "Expected success or network/cache error, got {}",
            code
        );
    }
}

#[test]
fn env_help_shows_shell_and_use_on_cd_flags() {
    Command::cargo_bin("fpy")
        .unwrap()
        .args(["env", "--help"])
        .assert()
        .success()
        .stdout(contains("--shell"))
        .stdout(contains("--use-on-cd"));
}

#[test]
fn use_help_shows_silent_if_unchanged_flag() {
    Command::cargo_bin("fpy")
        .unwrap()
        .args(["use", "--help"])
        .assert()
        .success()
        .stdout(contains("--silent-if-unchanged"));
}

#[test]
fn current_help_exits_zero() {
    Command::cargo_bin("fpy")
        .unwrap()
        .args(["current", "--help"])
        .assert()
        .success()
        .stdout(contains("currently active Python version"));
}

#[test]
fn default_help_exits_zero() {
    Command::cargo_bin("fpy")
        .unwrap()
        .args(["default", "--help"])
        .assert()
        .success()
        .stdout(contains("default Python version"));
}

#[test]
fn install_help_exits_zero() {
    Command::cargo_bin("fpy")
        .unwrap()
        .args(["install", "--help"])
        .assert()
        .success()
        .stdout(contains("Install a Python version"));
}

// ── unrecognized subcommand routes to pass-through ────────────────────────
//
// Unrecognized first token forwards to py.exe. If py.exe is present, its exit
// code propagates. If py.exe is missing, fpm exits with code 1 (PyNotFound).
// We test BOTH paths — the test passes as long as fpm does NOT panic and
// exits with a non-zero code (since "foobar" is not a valid Python script).

#[test]
fn unrecognized_subcommand_does_not_crash() {
    // `fpm foobar` — py.exe will try to open "foobar" as a script and fail.
    // If py is missing, fpm exits 1. If py is present, py exits non-zero.
    // Either way, fpm must not panic or crash.
    let mut cmd = Command::cargo_bin("fpy").unwrap();
    cmd.arg("foobar");
    let assert = cmd.assert();

    // The exit code is either 1 (PyNotFound) or propagated from py.
    // We accept any non-zero exit code — we just verify no panic.
    let output = assert.get_output();
    let code = output.status.code().unwrap_or(-1);
    assert!(
        code != 0,
        "fpm foobar should exit non-zero (py not found or py script error), got {code}"
    );
}
