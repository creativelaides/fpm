# Archive Report: fpm-rust-wrapper

**Change slug**: `fpm-rust-wrapper`
**Archive date**: 2026-07-16
**Archived to**: `openspec/changes/archive/2026-07-16-fpm-rust-wrapper/`
**Artifact store mode**: hybrid (Engram + OpenSpec)

## Final Status

| Field | Value |
|---|---|
| Change | fpm-rust-wrapper |
| Intent | Replace the hand-written PowerShell `py { ... }` `$profile` block with a fast, hardened Rust CLI named `fpm` that mirrors fnm's ergonomics for Python on Windows. |
| Final state | Implemented, verified, merged to main |
| Verdict | PASS WITH WARNINGS (0 CRITICAL) |
| Tasks | 24 / 24 complete |
| Tests | 133 total, 124 pass, 9 ignored |

## What Changed

The change delivered a Windows-only Rust CLI wrapping PyManager (`py.exe`):

- 6 `clap` subcommands: `use`, `list`, `current`, `default`, `env`, `install`.
- NTFS junction-based per-session shim under `%LocalAppData%\fpm\multishells\<session-id>\`.
- `fpm env --shell powershell` emits a PowerShell script for PATH prepend + optional `Set-Location` hook.
- Version-file resolution walking upward from cwd: `.python-version` then `pyproject.toml` (`requires-python` / `[tool.poetry.dependencies] python`), reduced via `pep440_rs`.
- Pass-through of unrecognized `fpm <args>` to `py.exe`.
- `fpm default` reads/writes `%AppData%\Python\pymanager.json`.
- 5 chained PRs (PR1 scaffold → PR2 pymanager/version-file → PR3 shim/shell → PR4 commands/CLI → PR5 tests/README).

## Key Decisions

1. **Shim mechanism**: NTFS directory junction to the resolved Python install directory (parent of `py list --one --format=exe` output). Safer than symlinks on Windows and lets `pip.exe`/`idle.exe` resolve.
2. **Skip PEP 514 registry fallback for v1**: deferred per design; removes `winreg` dependency.
3. **--silent-if-unchanged**: compare canonicalized junction target against the newly resolved install dir.
4. **pyproject.toml specifier matching via `pep440_rs`**: avoids hand-rolling PEP 440.
5. **Session ID**: `<pid>_<random_u32>` for uniqueness without extra UUID crate.
6. **Test strategy**: `assert_cmd` + predicates; `PyManagerOps` trait with real + mock impl; `#[ignore]` for tests requiring real `py.exe`.

## Artifacts Produced

### OpenSpec

| Artifact | Status | Path |
|---|---|---|
| Exploration | Archived | `openspec/changes/archive/2026-07-16-fpm-rust-wrapper/explore.md` |
| Proposal | Archived | `openspec/changes/archive/2026-07-16-fpm-rust-wrapper/proposal.md` |
| Design | Archived | `openspec/changes/archive/2026-07-16-fpm-rust-wrapper/design.md` |
| Tasks | Archived | `openspec/changes/archive/2026-07-16-fpm-rust-wrapper/tasks.md` |
| Verify report | Archived | `openspec/changes/archive/2026-07-16-fpm-rust-wrapper/verify-report.md` |
| fpm-core spec | Copied to main spec tree | `openspec/specs/fpm-core/spec.md` |
| python-version-switching spec | Copied to main spec tree | `openspec/specs/python-version-switching/spec.md` |
| powershell-shell-integration spec | Copied to main spec tree | `openspec/specs/powershell-shell-integration/spec.md` |
| pymanager-delegation spec | Copied to main spec tree | `openspec/specs/pymanager-delegation/spec.md` |
| version-file-resolution spec | Copied to main spec tree | `openspec/specs/version-file-resolution/spec.md` |

### Engram Observations (traceability)

| Topic | Observation ID | Purpose |
|---|---|---|
| sdd/fpm-rust-wrapper/tasks | #64 | Task breakdown and completion tracking |
| sdd/fpm-rust-wrapper/apply-progress | #65 | PR1-PR5 cumulative apply progress |
| sdd/fpm-rust-wrapper/verify-report | #67 | Verification verdict and results |

## Test Results

| Suite | Tests | Status |
|---|---|---|
| Unit tests (src/) | 105 | pass |
| tests/cli_dispatch.rs | 11 | pass |
| tests/env_cmd.rs | 8 | pass |
| tests/passthrough.rs | 4 | ignored (require `py.exe`) |
| tests/use_cmd.rs | 5 | ignored (require `py.exe`) |
| **Total** | **133** | **124 pass, 9 ignored** |

- `cargo fmt --check`: PASS
- `cargo clippy --tests -- -D warnings`: PASS
- `cargo build`: clean

## Verify Verdict

**PASS WITH WARNINGS**, 0 CRITICAL issues.

### Warning resolved post-verify

- **W-01** (`passthrough_forwards_version_to_py` ignored test failure): The test used `fpm --version` expecting pass-through to `py --version`; clap correctly intercepts `--version` per the fpm-core spec. The test argument was changed to `-0p` to avoid clap interception and the fix was committed (`becc3ef`).

### Suggestions (non-blocking)

| ID | Description | Severity |
|---|---|---|
| S-01 | `OnceCell` → `Option` for pymanager cache (functionally equivalent) | SUGGESTION |
| S-02 | `junction::create` arg order in design doc (implementation is correct) | SUGGESTION |
| S-03 | `pep440_rs` `~=` example in design doc (use `~=3.13.0`) | SUGGESTION |
| S-04 | No coverage tool configured | SUGGESTION |
| S-05 | Pre-existing clippy `io_other_error` fix in `src/error.rs` unit tests | SUGGESTION |

## Deviations from Design

3 minor deviations, all SUGGESTION level:

1. `OnceCell<Vec<Runtime>>` cache became `Option<Vec<Runtime>>` — functionally equivalent.
2. Design doc showed `junction::create(session_dir, install_dir)`; actual `junction` v1.2 API is `junction::create(target, junction_path)`, and implementation uses the correct order.
3. Design doc mentioned `~3.13`; `pep440_rs` requires full `~=3.13.0` form, which implementation tests use.

## Spec Compliance

| Capability | Requirements | Compliant | Deferred |
|---|---|---|---|
| fpm-core | 8 | 8 | 0 |
| python-version-switching | 7 | 7 | 0 |
| powershell-shell-integration | 6 | 6 | 0 |
| pymanager-delegation | 11 | 10 | 1 (PEP 514 registry fallback deferred by design) |
| version-file-resolution | 11 | 11 | 0 |
| **Total** | **43** | **42** | **1** |

## Source of Truth Updated

The main spec tree now contains the 5 capability specs as baseline:

- `openspec/specs/fpm-core/spec.md`
- `openspec/specs/python-version-switching/spec.md`
- `openspec/specs/powershell-shell-integration/spec.md`
- `openspec/specs/pymanager-delegation/spec.md`
- `openspec/specs/version-file-resolution/spec.md`

## Task Completion Gate

All 24 tasks in the archived `tasks.md` are marked `[x]`. No unchecked implementation tasks remain. No stale-checkbox reconciliation was required.

## Archive Verification

- [x] Main specs updated correctly (5 specs copied to `openspec/specs/`)
- [x] Change folder moved to archive (`openspec/changes/archive/2026-07-16-fpm-rust-wrapper/`)
- [x] Archive contains all artifacts (explore, proposal, specs, design, tasks, verify-report)
- [x] Archived `tasks.md` has no unchecked implementation tasks
- [x] Active changes directory no longer has this change

## SDD Cycle Complete

The change has been fully planned, proposed, designed, specified, implemented, verified, and archived. Ready for the next change.
