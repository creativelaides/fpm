# Design: Improve `fpm default` Command

## Technical Approach

Make `fpm default <tag>` behave like `fnm default`: persist `default_tag` to `pymanager.json` AND immediately activate the current session (retarget shim + set `PYTHON_MANAGER_DEFAULT`). Add `--unset` (remove key) and `--dry-run` (validate + preview, no side effects). Extract the shared session-activation sequence from `use_cmd.rs` into a `commands::activate_session` helper so `default.rs` and `use_cmd.rs` cannot drift. Add `PyManagerOps::unset_default` returning whether a key was actually removed, so the command can print the right message. CLI uses clap `conflicts_with` / `requires` to make `--unset` and `--dry-run` mutually exclusive and to force `--dry-run` to take a tag.

## Architecture Decisions

| Decision | Choice | Alternatives | Rationale |
|---|---|---|---|
| Retarget reuse | Extract `commands::activate_session(pymanager, tag, session_dir) -> Result<PathBuf, FpmError>` into `commands/mod.rs`; refactor `use_cmd.rs` to call it; `default.rs` calls it too | Inline duplicate in `default.rs`; new `commands/activate.rs` module | The `python-version-switching` delta explicitly requires activation effects be reusable with the same error path. One shared helper enforces that; two copies invite drift. `mod.rs` already hosts the shared `CommandContext` and is the natural home; a new module is overkill for one function. `silent_if_unchanged` stays in `use_cmd` (it is use-specific) — the helper does only resolve→derive install→retarget→set env, returning the canonical install path so each caller prints its own message. |
| `unset_default` signature | `fn unset_default(&mut self) -> Result<bool, FpmError>` — `true` if a `default_tag` key was removed, `false` if file/key absent (no-op) | Return `()` like `write_default`; have caller `read_default` first | `read_default`-then-`unset` is two IO ops and a race. The bool lets `default.rs` print "Removed default" vs "No default was configured" (spec scenarios 2 & 3) in one call. Matches the informative-return precedent of `read_default -> Option<String>`. |
| `--unset` vs tag | clap `#[arg(long, conflicts_with = "tag")] unset: bool` — `fpm default --unset 3.14` is a parse error | Allow tag with `--unset` (ignore it) | Explicit error is safer than silently dropping the tag; matches user mental model ("unset takes no argument"). |
| `--dry-run` vs tag | `#[arg(long, requires = "tag", conflicts_with = "unset")] dry_run: bool` — bare `fpm default --dry-run` is a parse error; `--dry-run --unset` is a parse error | Allow bare `--dry-run` to preview the current default | The spec defines `--dry-run` only as "the tag that would be written" — it needs a tag. Previewing the current default is just `fpm default` (read mode), so no extra meaning is lost. |
| Validation ordering | resolve_exe → require `session_dir` → write_default → activate_session (retarget + env) | retarget before write; write+retarget in one transaction | Spec scenario "FPM_MULTISHELL_PATH not set" requires `pymanager.json` NOT written, so the session-dir check must precede the write. resolve_exe first satisfies "reject uninstalled tag before any side effect". |
| Partial failure (write ok, retarget fails) | Do NOT auto-rollback the write. Print a stderr warning: "Default set to {tag} but session activation failed: {err}. Run `fpm use {tag}` to activate." Exit with `ShimError` (code 5). | Roll back `default_tag` on retarget failure | Rollback has its own failure modes and hides the user's intent. The persisted default is still correct global state; only the session is stale. Surfacing the warning + exit code lets scripts detect it and the user recover with one command. Matches fnm's behavior. |
| `--dry-run` output | Human-readable single line: `Would set default to {tag} and activate Python at {install_dir}` | Machine-parseable key=value | No existing fpm command uses machine-parseable output (`fpm current` prints human form). Spec scenario only requires the tag to appear. |

## Data Flow

```
fpm default <tag>            fpm default --unset        fpm default <tag> --dry-run
      │                            │                              │
 resolve_exe(tag) ──err──→ exit 2  │                       resolve_exe(tag) ──err──→ exit 2
      │ ok                          │                             │ ok
 session_dir? ──no──→ exit 5       read_default?                 print "Would set default to {tag}..."
      │ yes                          │                            exit 0 (no writes, no retarget, no env)
 write_default(tag)                 unset_default()
      │                              │
 activate_session()                 print "Removed default" / "No default configured"
      │                              exit 0
 print "Default set to {tag}; session activated"
 exit 0
```

`activate_session` (shared, in `commands/mod.rs`): `resolve_exe → parent() → canonicalize → shim::retarget → set_var(PYTHON_MANAGER_DEFAULT)`. Returns the canonical install path.

## File Changes

| File | Action | Description |
|---|---|---|
| `src/commands/mod.rs` | Modify | Add `pub fn activate_session<M: PyManagerOps>(pymanager, tag, session_dir) -> Result<PathBuf, FpmError>` extracting steps 2-7 of `use_cmd::run`. |
| `src/commands/use_cmd.rs` | Modify | Replace inline resolve→retarget→env block (lines 54-89) with `activate_session` call; keep `silent_if_unchanged` check before it. Behavior unchanged. |
| `src/commands/default.rs` | Modify | Rewrite `run` to take `DefaultArgs { tag, unset, dry_run, session_dir }`; dispatch set/read/unset/dry-run; call `activate_session` for set; validation ordering per decision. |
| `src/cli.rs` | Modify | Add `unset: bool` (`conflicts_with = "tag"`) and `dry_run: bool` (`requires = "tag"`, `conflicts_with = "unset"`) to `Commands::Default`. |
| `src/pymanager.rs` | Modify | Add `unset_default(&mut self) -> Result<bool, FpmError>` to `PyManagerOps`; implement for `PyManager` and `MockPyManager`; add shared helper `unset_default_tag(path) -> Result<bool, FpmError>`. |
| `src/main.rs` | Modify | Update `Commands::Default` dispatch to pass `session_dir` (reuse the `ctx.session_dir.ok_or_else(...)` block from `Commands::Use`). |

## Interfaces / Contracts

```rust
// commands/mod.rs
pub fn activate_session<M: PyManagerOps>(
    pymanager: &mut M,
    tag: &str,
    session_dir: &Path,
) -> Result<PathBuf, FpmError>;
// resolves exe, derives+canonicalizes install dir, retargets junction,
// sets PYTHON_MANAGER_DEFAULT. Returns canonical install dir.

// pymanager.rs — PyManagerOps
fn unset_default(&mut self) -> Result<bool, FpmError>;
// true  → default_tag key was present and removed (file + other keys preserved)
// false → file missing OR key absent (no-op, no file created)
// Err   → IO/parse failure

// cli.rs — Commands::Default
Default {
    tag: Option<String>,
    #[arg(long, conflicts_with = "tag")]
    unset: bool,
    #[arg(long, requires = "tag", conflicts_with = "unset")]
    dry_run: bool,
}
```

## Testing Strategy

| Layer | What | Approach |
|---|---|---|
| Unit (pymanager.rs) | `unset_default_tag` removes key + preserves others; returns false on missing file/key; returns true on present key; round-trips with `read_default` | tempfile + serde_json assertions, mirroring existing `write_default_tag` tests. |
| Unit (default.rs) | set retargets + writes + sets env; read unchanged; unset removes + prints right message for both branches; dry-run validates but writes nothing; uninstalled tag exits 2 before any write; missing `session_dir` exits 5 before any write; write-then-retarget-failure prints warning + exits 5 | `MockPyManager` + fake install dirs + `shim::create_session_dir`, reusing the `use_cmd` test harness. For the retarget-failure case, pass a `session_dir` whose parent is read-only or a file path so `shim::retarget` errors. |
| Unit (commands/mod.rs) | `activate_session` retargets and sets env; returns canonical path; errors when tag uninstalled; errors when `session_dir` invalid | Extracted from `use_cmd` tests — move the resolve+retarget assertions here, have `use_cmd` tests focus on `silent_if_unchanged` + version-file resolution. |
| Integration | `fpm default 3.14` then `fpm current` shows 3.14; `fpm default --unset` then `fpm default` prints "No default" | Manual or ignored — no integration harness in repo today. |

## Migration / Rollout

No migration required. Revert the six source files; users who ran the new `fpm default <tag>` are left consistent (`pymanager.json` and shim point at the same version).

## Open Questions

- [ ] Should `--dry-run` also report whether the session would actually change (i.e. compare against `current_target`)? Out of scope unless requested — spec only requires the tag preview.
- [ ] Confirm exit code for the write-ok/retarget-failed warning path: `ShimError` (5) is proposed; acceptable?