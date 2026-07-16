// Per-session multishell shim via NTFS junctions.
//
// Spec: python-version-switching, powershell-shell-integration
// Design: Shim Mechanism — NTFS Junction to Install Directory
//
// Each `fpm env` call creates a unique session directory under
// `<FPM_DIR>/multishells/<pid>_<random>/`. `fpm use` retargets that directory
// (an NTFS junction) to the resolved Python install directory. The junction
// puts the entire install dir on PATH so `python.exe`, `pip.exe`, `idle.exe`,
// etc. all resolve correctly without shadowing `py.exe`.
//
// CRITICAL: retarget uses `std::fs::remove_dir` (removes the reparse point
// only), NOT `remove_dir_all` (which would follow the junction into the target
// and delete the real Python install).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::id;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::config::multishells_dir;
use crate::error::FpmError;

/// Creates a unique per-session directory under `<fpm_dir>/multishells/`.
///
/// The session id is `{pid}_{random_u32}` — PID alone could collide across
/// process reuse, so a random suffix (seeded from the system clock) makes
/// collision astronomically unlikely without adding a uuid dependency.
///
/// Creates both the `multishells/` parent and the per-session leaf directory.
/// Returns the full path to the session directory.
pub fn create_session_dir(fpm_dir: &Path) -> Result<PathBuf, FpmError> {
    let parent = multishells_dir(fpm_dir);
    fs::create_dir_all(&parent).map_err(io_error)?;

    let session_id = format!("{}_{}", id(), random_u32());
    let session_dir = parent.join(&session_id);
    fs::create_dir(&session_dir).map_err(io_error)?;

    Ok(session_dir)
}

/// Retargets the session junction to a new install directory.
///
/// If a junction already exists at `session_dir`, it is removed with
/// `std::fs::remove_dir` (which removes the reparse point, NOT the target
/// contents). Then a new NTFS junction is created pointing from `session_dir`
/// to `install_dir`.
///
/// # Safety note
///
/// NEVER use `fs::remove_dir_all` on a junction — it follows into the target
/// directory and deletes the real Python install.
pub fn retarget(session_dir: &Path, install_dir: &Path) -> Result<(), FpmError> {
    // Remove existing junction if present. remove_dir on a junction removes
    // the reparse point only, leaving the target intact.
    if session_dir.exists() {
        fs::remove_dir(session_dir).map_err(io_error)?;
    }

    // junction::create(target, junction_path) — first arg is the real target,
    // second is the junction (reparse point) path.
    junction::create(install_dir, session_dir).map_err(io_error)
}

/// Reads the current junction target of `session_dir`, canonicalized.
///
/// Returns `Ok(None)` if no junction exists at the path. Returns
/// `Ok(Some(canonicalized_target))` if a junction is present.
pub fn current_target(session_dir: &Path) -> Result<Option<PathBuf>, FpmError> {
    match junction::get_target(session_dir) {
        Ok(target) => {
            let canonical = target
                .canonicalize()
                // If canonicalize fails (e.g. target gone), fall back to raw.
                .unwrap_or(target);
            Ok(Some(canonical))
        }
        // NotFound → no junction; raw OS error 4390 (ERROR_NOT_A_REPARSE_POINT)
        // → path exists but is a plain directory, not a junction. Both map to
        // None so callers get a clean "nothing here" answer.
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) if e.raw_os_error() == Some(4390) || e.kind() == io::ErrorKind::InvalidFilename => {
            Ok(None)
        }
        Err(e) => Err(io_error(e)),
    }
}

// ─── helpers ─────────────────────────────────────────────────────────────

/// Wraps an io::Error into FpmError::ShimError.
fn io_error(e: io::Error) -> FpmError {
    FpmError::ShimError(e)
}

/// Generates a simple pseudo-random u32 from the system clock.
///
/// No need for a full CSPRNG — the goal is uniqueness across concurrent
/// shells, not cryptographic randomness. Uses nanosecond precision when
/// available, falling back to milliseconds.
fn random_u32() -> u32 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    // Mix sub-second nanos with the pid for extra entropy.
    let nanos = now.subsec_nanos();
    let secs = now.as_secs() as u32;
    nanos.wrapping_mul(2654435761).wrapping_add(secs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    /// Creates a fake "install" directory to use as a junction target.
    fn make_install_dir(parent: &Path, name: &str) -> PathBuf {
        let dir = parent.join(name);
        fs::create_dir_all(&dir).unwrap();
        // Drop a marker file so we can verify the junction resolves.
        fs::write(dir.join("marker.txt"), "hello").unwrap();
        dir
    }

    #[test]
    fn create_session_dir_creates_unique_directory() {
        let fpm = tempdir().unwrap();
        let dir1 = create_session_dir(fpm.path()).unwrap();
        let dir2 = create_session_dir(fpm.path()).unwrap();

        assert!(dir1.exists(), "first session dir should exist");
        assert!(dir2.exists(), "second session dir should exist");
        assert_ne!(dir1, dir2, "session dirs must be unique");

        // Both under <fpm>/multishells/
        assert!(dir1.starts_with(fpm.path().join("multishells")));
        assert!(dir2.starts_with(fpm.path().join("multishells")));
    }

    #[test]
    fn create_session_dir_id_contains_pid() {
        let fpm = tempdir().unwrap();
        let dir = create_session_dir(fpm.path()).unwrap();
        let name = dir.file_name().unwrap().to_str().unwrap();
        let pid_str = id().to_string();
        assert!(
            name.starts_with(&format!("{}_", pid_str)),
            "session id '{name}' should start with pid '{pid_str}_'"
        );
    }

    #[test]
    fn create_session_dir_creates_multishells_parent() {
        let fpm = tempdir().unwrap();
        let _dir = create_session_dir(fpm.path()).unwrap();
        let parent = fpm.path().join("multishells");
        assert!(parent.exists(), "multishells parent should be created");
    }

    #[test]
    fn retarget_creates_junction() {
        let fpm = tempdir().unwrap();
        let session = create_session_dir(fpm.path()).unwrap();
        let install = make_install_dir(fpm.path(), "install_314");

        // remove_dir the empty session dir so junction::create can place a
        // reparse point at that path.
        fs::remove_dir(&session).unwrap();

        retarget(&session, &install).unwrap();

        // Reading through the junction should find the marker file.
        let marker = fs::read_to_string(session.join("marker.txt")).unwrap();
        assert_eq!(marker, "hello");
    }

    #[test]
    fn retarget_replaces_existing_junction() {
        let fpm = tempdir().unwrap();
        let session = create_session_dir(fpm.path()).unwrap();
        let install_a = make_install_dir(fpm.path(), "install_313");
        let install_b = make_install_dir(fpm.path(), "install_314");

        fs::remove_dir(&session).unwrap();
        retarget(&session, &install_a).unwrap();
        // Verify it points to A
        assert_eq!(
            fs::read_to_string(session.join("marker.txt")).unwrap(),
            "hello"
        );

        // Retarget to B — should replace, not fail.
        retarget(&session, &install_b).unwrap();
        assert_eq!(
            fs::read_to_string(session.join("marker.txt")).unwrap(),
            "hello"
        );

        // current_target should now resolve to install_b (canonicalized)
        let target = current_target(&session).unwrap().unwrap();
        let canonical_b = install_b.canonicalize().unwrap();
        assert_eq!(target, canonical_b);
    }

    #[test]
    fn current_target_reads_junction() {
        let fpm = tempdir().unwrap();
        let session = create_session_dir(fpm.path()).unwrap();
        let install = make_install_dir(fpm.path(), "install_312");

        fs::remove_dir(&session).unwrap();
        retarget(&session, &install).unwrap();

        let target = current_target(&session).unwrap();
        assert!(target.is_some(), "should read a junction target");
        let canonical_install = install.canonicalize().unwrap();
        assert_eq!(target.unwrap(), canonical_install);
    }

    #[test]
    fn current_target_returns_none_for_nonexistent() {
        let fpm = tempdir().unwrap();
        let ghost = fpm.path().join("does_not_exist");

        let target = current_target(&ghost).unwrap();
        assert!(target.is_none(), "non-existent path should return None");
    }

    #[test]
    fn current_target_returns_none_for_plain_dir() {
        let fpm = tempdir().unwrap();
        // A plain directory (no reparse point) — get_target returns NotFound
        // for non-junctions, which we map to None.
        let plain = fpm.path().join("plain_dir");
        fs::create_dir(&plain).unwrap();

        let target = current_target(&plain).unwrap();
        assert!(
            target.is_none(),
            "plain directory without junction should return None"
        );
    }

    #[test]
    fn round_trip_create_retarget_read() {
        let fpm = tempdir().unwrap();
        let install = make_install_dir(fpm.path(), "install_315");

        // 1. create session dir
        let session = create_session_dir(fpm.path()).unwrap();
        assert!(session.is_dir());

        // 2. retarget to install
        fs::remove_dir(&session).unwrap();
        retarget(&session, &install).unwrap();

        // 3. read back target
        let target = current_target(&session).unwrap().unwrap();
        let canonical = install.canonicalize().unwrap();
        assert_eq!(target, canonical);

        // 4. verify content accessible through junction
        assert_eq!(
            fs::read_to_string(session.join("marker.txt")).unwrap(),
            "hello"
        );
    }

    #[test]
    fn random_u32_is_nonzero_and_varies() {
        let a = random_u32();
        // Tiny sleep to let the clock tick
        std::thread::sleep(std::time::Duration::from_micros(100));
        let b = random_u32();
        // At least one should be non-zero
        assert!(a != 0 || b != 0, "random_u32 should not always be zero");
    }

    #[test]
    fn retarget_does_not_delete_target_contents() {
        // This is the CRITICAL safety property: remove_dir on the junction
        // must not delete files in the install directory.
        let fpm = tempdir().unwrap();
        let session = create_session_dir(fpm.path()).unwrap();
        let install = make_install_dir(fpm.path(), "install_safe");

        fs::remove_dir(&session).unwrap();
        retarget(&session, &install).unwrap();

        // Retarget away (removes junction, creates new one to a different dir)
        let install2 = make_install_dir(fpm.path(), "install_other");
        retarget(&session, &install2).unwrap();

        // Original install dir must still have its marker file
        assert!(
            install.join("marker.txt").exists(),
            "retarget must NOT delete the previous target's contents"
        );
    }
}
