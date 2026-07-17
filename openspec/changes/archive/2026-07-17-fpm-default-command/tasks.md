# Tasks: Improve `fpm default` Command

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | ~400 (refactor moves + new logic + tests) |
| 400-line budget risk | Medium |
| Chained PRs recommended | Yes |
| Suggested split | PR 1 (foundation: activate_session + unset_default) → PR 2 (default.rs + cli + main + tests) |
| Delivery strategy | ask-on-risk |
| Chain strategy | pending |

Decision needed before apply: Yes
Chained PRs recommended: Yes
Chain strategy: pending
400-line budget risk: Medium

### Suggested Work Units

| Unit | Goal | Likely PR | Notes |
|------|------|-----------|-------|
| 1 | Extract `activate_session` into `commands/mod.rs`; refactor `use_cmd.rs`; add `unset_default` to trait/impls/helper in `pymanager.rs`; unit tests for both | PR 1 | base = main; no behavior change, tests green |
| 2 | Rewrite `default.rs` (set+activate/read/unset/dry-run/validation ordering/partial-failure warning); add `--unset`/`--dry-run` to `cli.rs`; wire `main.rs` dispatch with `session_dir`; new default.rs tests | PR 2 | base = PR 1 branch; behavior lands here |

## Phase 1: Foundation (no behavior change)

- [x] 1.1 Add `pub fn activate_session<M: PyManagerOps>(pymanager, tag, session_dir) -> Result<PathBuf, FpmError>` to `src/commands/mod.rs` doing resolve_exe → parent() → canonicalize → `shim::retarget` → `config::PYTHON_MANAGER_DEFAULT_ENV` set_var; return canonical install dir.
- [x] 1.2 Refactor `src/commands/use_cmd.rs::run` to call `activate_session` (replace lines 54-89 block); keep `silent_if_unchanged` check before the call; behavior unchanged.
- [x] 1.3 Add `fn unset_default(&mut self) -> Result<bool, FpmError>` to `PyManagerOps` in `src/pymanager.rs`; implement on `PyManager` and `MockPyManager` delegating to a new `unset_default_tag(path) -> Result<bool, FpmError>` helper (remove key, preserve others; false if file/key absent; no file created).
- [x] 1.4 Move the resolve+retarget assertions out of `use_cmd` tests into a new `activate_session` test block in `src/commands/mod.rs`; add `unset_default_tag` unit tests in `src/pymanager.rs` (removes+preserves, false-on-missing-file, false-on-missing-key, round-trips with `read_default`).

## Phase 2: Core Implementation

- [x] 2.1 Rewrite `src/commands/default.rs::run` to take `(pymanager, tag: Option<&str>, unset: bool, dry_run: bool, session_dir: Option<&Path>) -> Result<i32, FpmError>` with dispatch: dry_run → resolve_exe then print "Would set default to {tag} and activate Python at {install_dir}", exit 0, no writes; unset → `unset_default`, print "Removed default" or "No default configured", exit 0; no args → read (unchanged); tag → set+activate path.
- [x] 2.2 Implement set+activate path in `default.rs`: resolve_exe (exit 2 before any side effect) → require `session_dir` (exit 5 before write) → `write_default` → `activate_session`; on activate failure, print stderr warning "Default set to {tag} but session activation failed: {err}. Run `fpm use {tag}` to activate." and return `ShimError` (exit 5); on success print "Default set to {tag}; session activated".
- [x] 2.3 Add `unset: bool` (`#[arg(long, conflicts_with = "tag")]`) and `dry_run: bool` (`#[arg(long, requires = "tag", conflicts_with = "unset")]`) to `Commands::Default` in `src/cli.rs`.
- [x] 2.4 Update `Commands::Default` dispatch in `src/main.rs` to resolve `session_dir` (reuse the `ctx.session_dir.ok_or_else(ShimError...)` block from `Commands::Use`) and pass `tag`, `unset`, `dry_run`, `session_dir` to `default::run`.

## Phase 3: Testing

- [x] 3.1 `src/commands/default.rs` tests: set writes+retargets+sets env (scenario Write new default and activate session); read unchanged (Read current default / Read when no default); unset removes + prints "Removed default" (Unset existing default); unset no-key prints "No default configured" (Unset when no default configured); unset missing file → no file created, exit 0 (Unset when pymanager.json missing); dry-run installed → prints preview, no write/retarget/env (Dry-run installed); dry-run uninstalled → exit 2, no preview (Dry-run not installed); uninstalled tag → exit 2 before any write (Reject uninstalled tag); missing `session_dir` → exit 5 before write (FPM_MULTISHELL_PATH not set); partial failure (write ok, retarget fails) → warning + exit 5.
- [x] 3.2 `src/commands/mod.rs` tests: `activate_session` retargets + sets env + returns canonical path; errors on uninstalled tag (exit 2); errors on invalid `session_dir` (exit 5).
- [x] 3.3 `src/pymanager.rs` tests for `unset_default_tag`: removes key + preserves others; false on missing file; false on missing key; round-trip with `read_default`; mock `unset_default` via trait.
- [x] 3.4 Verify `fpm use` tests still green after `activate_session` extraction (silent_if_unchanged + version-file resolution remain in `use_cmd`).

## Phase 4: Verification

- [x] 4.1 Run `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test` (124 existing + new tests) — all green.
- [x] 4.2 Confirm spec scenarios map 1:1 to tests: pymanager-delegation (set+activate, read, unset x3, dry-run x2, reject uninstalled, FPM_MULTISHELL_PATH) and python-version-switching (reuse, use stays session-only, reused activation fails without shim dir).
