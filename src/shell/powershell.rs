// PowerShell shell integration script generator.
//
// Spec: powershell-shell-integration (all requirements)
// Design: Data Flow for `fpm env --shell powershell`
//
// `render` produces the complete PowerShell script that `fpm env --shell
// powershell` prints to stdout. The user evaluates it with
// `| Out-String | Invoke-Expression`.
//
// The script:
//   1. Sets `$env:FPM_DIR` to the fpm data directory.
//   2. Prepends the session shim directory to `$env:PATH`.
//   3. Sets `$env:FPM_MULTISHELL_PATH` to the session directory.
//   4. (with --use-on-cd) Overrides `Set-Location` to call
//      `fpm use --silent-if-unchanged` when a `.python-version` or
//      `pyproject.toml` exists in the new location.
//   5. Registers a `PowerShell.Exiting` engine event for best-effort
//      cleanup of the session directory on shell exit.

use std::path::Path;

use super::Shell;
use crate::config::{FPM_DIR_ENV, FPM_MULTISHELL_PATH_ENV};

/// PowerShell shell backend.
pub struct PowerShell;

impl PowerShell {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PowerShell {
    fn default() -> Self {
        Self::new()
    }
}

impl Shell for PowerShell {
    fn render(&self, session_dir: &Path, use_on_cd: bool) -> String {
        // session_dir is the per-session multishell directory. We need the
        // fpm_dir (parent of multishells/) for $env:FPM_DIR. Walk up two
        // levels: <fpm_dir>/multishells/<session-id> -> <fpm_dir>.
        let fpm_dir = session_dir
            .parent() // multishells/
            .and_then(|p| p.parent()) // fpm_dir
            .map(|p| p.to_path_buf())
            .unwrap_or_default();

        let session_str = session_dir.display();
        let fpm_dir_str = fpm_dir.display();

        let mut script = String::new();

        // ── Environment variables ─────────────────────────────────────
        script.push_str(&format!("$env:{FPM_DIR_ENV} = \"{fpm_dir_str}\"\n"));
        script.push_str(&format!(
            "$env:{FPM_MULTISHELL_PATH_ENV} = \"{session_str}\"\n"
        ));
        // Prepend session shim dir to PATH so python.exe, pip.exe, etc.
        // resolve from the junction'd install directory.
        script.push_str(&format!("$env:PATH = \"{session_str};$env:PATH\"\n"));

        // ── use-on-cd hook ────────────────────────────────────────────
        if use_on_cd {
            script.push_str(&use_on_cd_hook());
        }

        // ── exit cleanup hook ─────────────────────────────────────────
        script.push_str(&exit_cleanup_hook(session_dir));

        script
    }
}

/// Generates the `Set-Location` override hook for `--use-on-cd`.
///
/// Follows fnm's PowerShell pattern: override the `Set-Location` function to
/// call the original logic, then check if a `.python-version` or
/// `pyproject.toml` exists in the new location (walking upward). If found,
/// invoke `fpm use --silent-if-unchanged`.
fn use_on_cd_hook() -> String {
    r#"
# fpm use-on-cd: override Set-Location to auto-switch Python on dir change
$__fpmOriginalSetLocation = Get-Command Set-Location -CommandType Function
function global:Set-Location {
    param(
        [Parameter(ValueFromPipeline = $true, Position = 0)]
        [string]$Path,
        [Parameter(Position = 1)]
        [string]$LiteralPath
    )
    end {
        if ($Path) {
            & $__fpmOriginalSetLocation $Path
        } elseif ($LiteralPath) {
            & $__fpmOriginalSetLocation -LiteralPath $LiteralPath
        } else {
            & $__fpmOriginalSetLocation
        }

        # After changing directory, check for a version file.
        # Walk upward from the new cwd looking for .python-version or
        # pyproject.toml. fpm use does the real resolution; we just gate on
        # file presence to avoid spawning fpm on every cd.
        $dir = Get-Location
        while ($dir) {
            if (Test-Path (Join-Path $dir '.python-version') -or
                Test-Path (Join-Path $dir 'pyproject.toml')) {
                fpm use --silent-if-unchanged 2>$null
                break
            }
            $parent = Split-Path $dir -Parent
            if ($parent -eq $dir) { break }
            $dir = $parent
        }
    }
}
"#
    .to_string()
}

/// Generates the `PowerShell.Exiting` engine event hook for best-effort
/// cleanup of the session directory on shell exit.
///
/// This is best-effort and non-blocking: if the directory cannot be removed
/// (e.g. a process holds a handle), the error is silently ignored. Stale
/// directories do not break other sessions because session IDs are unique.
fn exit_cleanup_hook(session_dir: &Path) -> String {
    let session_str = session_dir.display();
    format!(
        r#"
# fpm cleanup: remove session directory on shell exit (best-effort)
$global:__FpmSessionDir = "{session_str}"
$global:__FpmCleanup = Register-EngineEvent PowerShell.Exiting -Action {{
    if ($global:__FpmSessionDir -and (Test-Path $global:__FpmSessionDir)) {{
        Remove-Item -LiteralPath $global:__FpmSessionDir -Force -ErrorAction SilentlyContinue
    }}
}}
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn session_dir(fpm_dir: &Path) -> PathBuf {
        fpm_dir.join("multishells").join("1234_5678")
    }

    fn render(fpm_dir: &Path, use_on_cd: bool) -> String {
        let ps = PowerShell::new();
        ps.render(&session_dir(fpm_dir), use_on_cd)
    }

    #[test]
    fn output_contains_fpm_dir() {
        let fpm = PathBuf::from(r"C:\Users\test\fpm");
        let out = render(&fpm, false);
        assert!(
            out.contains("$env:FPM_DIR ="),
            "output should set FPM_DIR, got:\n{out}"
        );
        assert!(
            out.contains(r"C:\Users\test\fpm"),
            "output should contain the fpm_dir path, got:\n{out}"
        );
    }

    #[test]
    fn output_contains_path_prepend() {
        let fpm = PathBuf::from(r"C:\Users\test\fpm");
        let out = render(&fpm, false);
        assert!(
            out.contains("$env:PATH = \"") && out.contains(";$env:PATH\""),
            "output should prepend session dir to PATH, got:\n{out}"
        );
        // The session dir must appear in the PATH prepend
        let session = session_dir(&fpm);
        assert!(
            out.contains(&format!("{}", session.display())),
            "PATH prepend should include the session dir, got:\n{out}"
        );
    }

    #[test]
    fn output_contains_fpm_multishell_path() {
        let fpm = PathBuf::from(r"C:\Users\test\fpm");
        let out = render(&fpm, false);
        assert!(
            out.contains("$env:FPM_MULTISHELL_PATH ="),
            "output should set FPM_MULTISHELL_PATH, got:\n{out}"
        );
        let session = session_dir(&fpm);
        assert!(
            out.contains(&format!("{}", session.display())),
            "FPM_MULTISHELL_PATH should contain the session dir, got:\n{out}"
        );
    }

    #[test]
    fn use_on_cd_adds_set_location_hook() {
        let fpm = PathBuf::from(r"C:\Users\test\fpm");
        let out = render(&fpm, true);
        assert!(
            out.contains("function global:Set-Location"),
            "use-on-cd should emit a Set-Location override, got:\n{out}"
        );
        assert!(
            out.contains("fpm use --silent-if-unchanged"),
            "use-on-cd hook should invoke fpm use --silent-if-unchanged, got:\n{out}"
        );
        assert!(
            out.contains(".python-version"),
            "use-on-cd hook should check for .python-version, got:\n{out}"
        );
        assert!(
            out.contains("pyproject.toml"),
            "use-on-cd hook should check for pyproject.toml, got:\n{out}"
        );
    }

    #[test]
    fn no_use_on_cd_omits_hook() {
        let fpm = PathBuf::from(r"C:\Users\test\fpm");
        let out = render(&fpm, false);
        assert!(
            !out.contains("function global:Set-Location"),
            "without --use-on-cd the Set-Location override should be absent, got:\n{out}"
        );
        assert!(
            !out.contains("fpm use --silent-if-unchanged"),
            "without --use-on-cd the fpm use call should be absent, got:\n{out}"
        );
    }

    #[test]
    fn cleanup_hook_present() {
        let fpm = PathBuf::from(r"C:\Users\test\fpm");
        let out = render(&fpm, false);
        assert!(
            out.contains("Register-EngineEvent PowerShell.Exiting"),
            "output should register a PowerShell.Exiting cleanup hook, got:\n{out}"
        );
        assert!(
            out.contains("Remove-Item"),
            "cleanup hook should remove the session dir, got:\n{out}"
        );
        assert!(
            out.contains("ErrorAction SilentlyContinue"),
            "cleanup should be best-effort (silently continue on error), got:\n{out}"
        );
    }

    #[test]
    fn cleanup_hook_contains_session_dir() {
        let fpm = PathBuf::from(r"C:\Users\test\fpm");
        let out = render(&fpm, false);
        let session = session_dir(&fpm);
        // The session dir should appear at least twice: PATH prepend + cleanup
        let session_str = format!("{}", session.display());
        let count = out.matches(&session_str).count();
        assert!(
            count >= 2,
            "session dir should appear in PATH and cleanup, found {count} times"
        );
    }

    #[test]
    fn cleanup_hook_present_with_use_on_cd() {
        let fpm = PathBuf::from(r"C:\Users\test\fpm");
        let out = render(&fpm, true);
        assert!(
            out.contains("Register-EngineEvent PowerShell.Exiting"),
            "cleanup hook should be present even with --use-on-cd, got:\n{out}"
        );
    }

    #[test]
    fn output_starts_with_fpm_dir_assignment() {
        let fpm = PathBuf::from(r"C:\test\fpm");
        let out = render(&fpm, false);
        let trimmed = out.trim_start();
        assert!(
            trimmed.starts_with("$env:FPM_DIR ="),
            "first meaningful line should be FPM_DIR assignment, got:\n{}",
            &trimmed[..50.min(trimmed.len())]
        );
    }

    #[test]
    fn fpm_dir_resolved_from_session_dir_parent() {
        // session_dir = <fpm_dir>/multishells/<id>
        // render should derive fpm_dir by walking up two levels.
        let fpm = PathBuf::from(r"C:\data\fpm");
        let out = render(&fpm, false);
        assert!(
            out.contains(r#"$env:FPM_DIR = "C:\data\fpm""#),
            "FPM_DIR should be derived from session_dir parent, got:\n{out}"
        );
    }
}
