# Archive Report: fpm-default-command

**Change slug**: fpm-default-command  
**Archive date**: 2026-07-17  
**Status**: ✅ Archived — SDD cycle complete  
**Archive mode**: hybrid (OpenSpec + Engram)

## Final Verdict

| Phase | Result |
|-------|--------|
| Proposal | Approved |
| Specification | Approved |
| Design | Approved |
| Tasks | 16/16 complete (Phase 1–3 implementation tasks + Phase 4 verification tasks) |
| Verification | **PASS**, 0 CRITICAL, 0 WARNING, 0 SUGGESTION |
| Merge to main | Done |

## What Changed

`fpm default` now behaves like `fnm default`: it persists `default_tag` to `pymanager.json` **and** immediately activates the current session (retargets the per-session shim and sets `PYTHON_MANAGER_DEFAULT`). It also adds `--unset` to remove the default and `--dry-run` to preview without side effects.

## Key Design Decisions

1. **Shared activation helper**: `commands::activate_session` extracted into `src/commands/mod.rs` so `fpm use` and `fpm default` cannot drift.
2. **`unset_default` returns `Result<bool, FpmError>`**: lets the command distinguish "removed" from "was already absent" in one IO operation.
3. **CLI constraints**: `--unset` conflicts with `tag`; `--dry-run` requires `tag` and conflicts with `--unset`.
4. **Validation ordering**: `resolve_exe` → require `session_dir` → `write_default` → `activate_session`.
5. **Partial failure handling**: if the write succeeds but retarget fails, do not roll back; print a stderr warning and exit with `ShimError` (code 5).

## Source Files Modified

| File | Role |
|------|------|
| `src/commands/mod.rs` | Added `activate_session` helper + tests |
| `src/commands/use_cmd.rs` | Refactored to call `activate_session`; behavior unchanged |
| `src/commands/default.rs` | Rewrote set/read/unset/dry-run logic |
| `src/cli.rs` | Added `--unset` and `--dry-run` flags to `Commands::Default` |
| `src/pymanager.rs` | Added `unset_default_tag` helper and `PyManagerOps::unset_default` |
| `src/main.rs` | Updated `Commands::Default` dispatch to pass `session_dir` |

## Specs Synced

| Domain | Action | Details |
|--------|--------|---------|
| `pymanager-delegation` | Updated | Replaced `fpm default Reads and Writes pymanager.json` with `fpm default Reads, Writes, and Activates pymanager.json`; added `--unset`, `--dry-run`, and tag-validation requirements (3 added, 1 modified) |
| `python-version-switching` | Updated | Added `Session Activation Effects Are Reusable` requirement with 3 scenarios |

## Verification Results

- `cargo fmt --check` → clean
- `cargo clippy --all-targets -- -D warnings` → clean
- `cargo test` → 140 passed / 0 failed / 9 ignored
- Spec compliance: 16/16 scenarios compliant

## Reconciliation Note

`tasks.md` originally left Phase 4 verification checkboxes (`4.1`, `4.2`) unchecked because verification ran after the tasks artifact was written. The verify-report proves both items are complete, so the archive process mechanically checked them to ensure the archived audit trail reflects final state.

## Artifacts Archived

All artifacts moved to `openspec/changes/archive/2026-07-17-fpm-default-command/`:

- `proposal.md` ✅
- `explore.md` ✅
- `specs/pymanager-delegation/spec.md` ✅
- `specs/python-version-switching/spec.md` ✅
- `design.md` ✅
- `tasks.md` ✅ (16/16 tasks complete)
- `verify-report.md` ✅
- `archive-report.md` ✅

## Source of Truth Updated

- `openspec/specs/pymanager-delegation/spec.md`
- `openspec/specs/python-version-switching/spec.md`

## SDD Cycle Complete

The change has been fully planned, implemented, verified, and archived. Ready for the next change.
