# Proposal: Improve `fpm default` Command

## Intent

`fpm default <tag>` currently only writes `default_tag` to `pymanager.json`; it does not retarget the session shim. Setting a global default therefore has no immediate effect in the current shell — the user must also run `fpm use`. This change makes `fpm default` behave like `fnm default`: global persistence plus immediate session effect.

## Scope

### In Scope
- `fpm default <tag>`: validate tag, persist to `pymanager.json`, retarget session shim, set `PYTHON_MANAGER_DEFAULT`.
- `fpm default --unset`: remove `default_tag` from `pymanager.json`, preserving other keys.
- `fpm default <tag> --dry-run`: preview without writing.
- `fpm default` (no args): unchanged read behavior.

### Out of Scope
- `--platform`, `--list`, `--verbose` flags.
- Deprecating or changing `fpm use` semantics.

## Capabilities

### New Capabilities
- None.

### Modified Capabilities
- `pymanager-delegation`: `fpm default <tag>` now also activates the version for the current session by retargeting the shim and setting `PYTHON_MANAGER_DEFAULT`. Adds `--unset` and `--dry-run`.
- `python-version-switching`: shared shim/env logic is reused by `fpm default <tag>`; `fpm use` behavior is unchanged.

## Approach

1. Add `--unset` and `--dry-run` flags to the `Default` clap variant in `src/cli.rs`.
2. Add `unset_default_tag` helper in `src/pymanager.rs` and `unset_default` to `PyManagerOps`.
3. Rewrite `src/commands/default.rs`:
   - `--unset`: call `unset_default`, print confirmation.
   - no args: read and print current `default_tag` (unchanged).
   - `<tag>`: validate via `resolve_exe`, write `default_tag`, retarget shim using the same logic as `use_cmd.rs`, set `PYTHON_MANAGER_DEFAULT` env var, print combined message.
   - `--dry-run`: print the target `default_tag` without writing or retargeting.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `src/commands/default.rs` | Modified | Core command logic, validation, shim retarget, unset, dry-run. |
| `src/cli.rs` | Modified | Add `--unset` and `--dry-run` to `Default` variant. |
| `src/pymanager.rs` | Modified | Add `unset_default_tag` helper and `PyManagerOps::unset_default`. |
| `src/commands/use_cmd.rs` | Read-only reference | Shim/env logic reused; no changes. |
| `openspec/specs/pymanager-delegation/spec.md` | Delta | Update `fpm default` requirements. |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Tag validation via `resolve_exe` rejects tags PyManager would auto-install. | Low | Validation matches the existing `fpm use` behavior; users can `fpm install` first if needed. |
| Retarget requires `FPM_MULTISHELL_PATH` to be set, same as `fpm use`. | Med | Reuse the same error path; print the same guidance. |
| `--unset` may expose different PyManager fallback than setting `default_tag` to `""`. | Low | Remove the key entirely; behavior aligns with `pymanager.json` docs. |

## Rollback Plan

Revert the three source files (`default.rs`, `cli.rs`, `pymanager.rs`) and the spec delta to their previous versions. Users who ran the new `fpm default <tag>` are left in a consistent state: `pymanager.json` and the shim both point to the same version, so reverting code does not create config drift.

## Dependencies
- None beyond existing `pymanager-delegation` and `python-version-switching` specs.

## Success Criteria

- [ ] `fpm default 3.14` writes `default_tag`, retargets the shim, sets `PYTHON_MANAGER_DEFAULT`, and prints the combined message.
- [ ] `fpm default` (no args) still prints the current default.
- [ ] `fpm default --unset` removes `default_tag` from `pymanager.json` while preserving other keys.
- [ ] `fpm default 3.14 --dry-run` prints the would-be default without side effects.
- [ ] `fpm use` remains session-only and unchanged.
