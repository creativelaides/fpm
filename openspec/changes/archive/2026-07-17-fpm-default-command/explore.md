# Exploration: fpm default command

## Executive Summary

The existing `fpm default [tag]` command is a thin read/write wrapper around the `default_tag` key in `%AppData%\Python\pymanager.json`. It already satisfies the current `pymanager-delegation` spec for reading, writing, and key preservation. This exploration identifies several UX and safety gaps compared to real-world version managers and PyManager's own configuration surface, and flags the need for user direction before a proposal is written.

## Current State

### Implementation (`src/commands/default.rs`)

- With no argument, calls `pymanager.read_default()` and prints:
  - the tag if present (`println!("{t}")`), or
  - `No default Python configured.` if absent.
- With a tag argument, calls `pymanager.write_default(tag)`, prints `Default Python set to {tag}`, and exits 0.
- Uses `PyManagerOps` (real `py` calls are abstracted; mock impls enable unit tests).

### Underlying `pymanager.json` helpers (`src/pymanager.rs`)

- `read_default_tag(path)` returns `Ok(None)` if the file is missing or `default_tag` is absent; returns `Err(FpmError::ConfigError)` for malformed JSON or I/O errors.
- `write_default_tag(path, tag)` reads existing JSON, preserves all other keys, overwrites `default_tag`, creates the file if missing, and writes pretty-printed JSON.

### CLI surface (`src/cli.rs`)

```rust
Default {
    /// Version tag to set as default. If omitted, prints the current default.
    tag: Option<String>,
}
```

Only one positional argument is accepted; no flags are defined for `--unset`, `--list`, validation, or verbose display.

### Main dispatch (`src/main.rs`)

The `Default` variant directly calls `commands::default::run(&mut ctx.pymanager, tag.as_deref())`.

## Spec and Design Context

### Current `pymanager-delegation` spec (openspec/specs/pymanager-delegation/spec.md)

Requirement: `fpm default` Reads and Writes `pymanager.json`.

- `fpm default` (no arg) SHALL read and print `default_tag`.
- `fpm default <version>` SHALL write `default_tag` to `%AppData%\Python\pymanager.json`, preserving all other keys.
- `fpm` SHALL NOT touch `pymanager.json` from `fpm use`.

Scenarios cover: read current default, write new default, missing config file creation, and `fpm current` active-version reporting.

### Archived design (openspec/changes/archive/2026-07-16-fpm-rust-wrapper/design.md)

Design explicitly states:

> `fpm default [tag]`: read/write `default_tag` in `pymanager.json`, preserve other keys.

It also clarifies the architectural separation:

> `fpm use` sets `PYTHON_MANAGER_DEFAULT` for the current session; `fpm default` persists it to `pymanager.json`.

No extended default command features were planned in v1.

## PyManager Default Surface (per python.org docs)

From `https://docs.python.org/3/using/windows.html#configuration`:

- `default_tag` (config key) / `PYTHON_MANAGER_DEFAULT` (env var): the preferred default version to launch or install. By default, this is interpreted as the most recent non-prerelease version from the CPython team.
- `default_platform` / `PYTHON_MANAGER_DEFAULT_PLATFORM`: preferred platform suffix, e.g. `3.14-64` is preferred over `3.14` when `default_platform` is `-64`.
- Precedence in PyManager: active virtual environment > shebang > `PYTHON_MANAGER_DEFAULT` / `default_tag` > latest installed.
- The user config file is `%AppData%\Python\pymanager.json`.

Key implications for `fpm default`:

1. Setting `default_tag` does **not** validate that the tag is installed. PyManager will fall back or attempt an install depending on context.
2. `default_tag` can be a partial tag (e.g. `3.14`) and combine with `default_platform` (`-64`).
3. `fpm current` already reports the effective version by spawning `py -V`, while `fpm default` only reports the configured preference.
4. `default_tag` is only one of several configuration keys. There is no documented `--unset` command in PyManager itself; to unset, a user would presumably remove the `default_tag` key or set an empty value.

## Affected Areas

| File | Why affected |
|------|--------------|
| `src/commands/default.rs` | Core command logic and unit tests. |
| `src/cli.rs` | Adds flags/arguments to the `Default` subcommand. |
| `src/pymanager.rs` | May need new `PyManagerOps` methods for unsetting, validation, or richer display. |
| `src/commands/list.rs` | Source of candidate tags if `fpm default --list-aliases` is implemented. |
| `src/commands/current.rs` | Overlaps conceptually if `fpm default` shows resolved effective version. |
| `openspec/specs/pymanager-delegation/spec.md` | Delta spec additions/modifications for any new behavior. |

## Gaps and Candidate Improvements

| # | Gap / Improvement | Notes | Effort |
|---|-------------------|-------|--------|
| 1 | **Unset default** (`fpm default --unset`) | Remove `default_tag` from `pymanager.json`, preserving other keys. PyManager would then fall back to latest installed. No documented PyManager `--unset` flag exists, so this would be an `fpm` convenience. | Low |
| 2 | **Validate tag exists before setting** | Reuse `pymanager.resolve_exe(tag)` or `list_runtimes()` to ensure the tag is installed. Prevents typos and silent invalid defaults. | Low-Medium |
| 3 | **Show resolved version info** (`fpm default --verbose` or always-on) | Print both the configured tag and the runtime it resolves to (version, path, active-marker). | Medium |
| 4 | **List candidate tags** (`fpm default --list` or integrate with `fpm list`) | Suggest installed tags when the user runs `fpm default` with no tag and no default configured, or add a dedicated flag. | Low-Medium |
| 5 | **Support `default_platform` awareness** | `fpm default 3.14` might imply `3.14-64` when `default_platform` is set. Could display platform hint or accept `--platform`. | Medium |
| 6 | **Atomic or safer JSON edit** | Current helper reads, mutates, and writes. Adding a temporary file + rename would reduce corruption risk. | Low |
| 7 | **Dry-run / preview** (`fpm default <tag> --dry-run`) | Show what `default_tag` would be without writing. Useful for scripting. | Low |
| 8 | **Return code consistency for no default** | Today `fpm default` with no configured default exits 0. Some tools would return 1 to signal "not set". | Low |

## Approaches

### Approach A: Minimal UX polish

Add `--unset` and validate the tag exists before writing.

- Pros: small scope, high practical value, minimal risk, fits current test strategy.
- Cons: does not address display/resolution ambiguity; still a thin wrapper.
- Effort: Low.

### Approach B: Richer read and validation

- When reading (`fpm default` no arg), show whether the default is active, what version it resolves to, and where it comes from (`pymanager.json` vs `PYTHON_MANAGER_DEFAULT` env var).
- When writing, validate the tag is installed (via `py list --one --format=exe <tag>`) and reject unknown tags.
- Add `--unset`.

- Pros: more informative, prevents misconfiguration, closer to `fnm`/`nvm` expectations.
- Cons: more I/O on read (requires `py list`); `fpm default` currently spawns no `py` process. Adds coupling between `default` and runtime resolution.
- Effort: Medium.

### Approach C: Full default management subcommand suite

Add `--unset`, `--list` (list installed tags), `--verbose`, `--dry-run`, `--platform`, and possibly `--global` vs `--local` semantics.

- Pros: very discoverable, feature-complete for power users.
- Cons: largest change, may exceed the 400-line review budget, overlaps with `fpm list`/`fpm current`, and introduces behavior not requested by the user.
- Effort: High.

## Recommendation

Recommend **Approach A** as the safe starting point, with optional additive elements from Approach B if the user wants richer output. The core value of `fpm default` is "persist the version I use by default"; `--unset` and basic validation are the lowest-risk improvements that directly improve correctness without redefining the command.

## Risks

1. **Scope creep**: `default` touches `pymanager.json`, the user's persistent Python configuration. Aggressive features like `--platform` or effective-version resolution risk confusion with `fpm use` and `fpm current`.
2. **Validation fragility**: If `fpm default <tag>` validates the tag against `py list`, it may reject tags that PyManager would later install automatically or resolve through `default_platform`. Validation should probably be a warning, not a hard error, or only error when no runtime matches.
3. **PyManager precedence**: Changing display to include "active" state could mislead users because `PYTHON_MANAGER_DEFAULT` (set by `fpm use`) overrides `pymanager.json` for the session. `fpm default` must stay focused on the persisted config.
4. **Unset semantics**: Removing `default_tag` entirely vs setting it to `""` may behave differently in PyManager. Need to confirm by testing or accept the documented fallback path (latest installed).

## Open Questions (need user clarification)

1. **What is the primary improvement you want?** (`--unset`, validation, richer display, or something else?)
2. **Should `fpm default <tag>` validate that the tag is installed?** Strict error, warning, or no validation (keep current PyManager-aligned behavior)?
3. **Should `fpm default` without arguments show just the tag (current behavior) or also the resolved version/path?**
4. **Should the output format be stable/scriptable?** If richer display is added, a `--porcelain`/machine-readable mode may be needed.
5. **Should unsetting remove the `default_tag` key entirely, or set it to an empty string?**
6. **Is there a review-line budget or delivery strategy?** If the user wants several features, the work may need to be split across chained PRs to stay within the 400-line budget.

## Ready for Proposal

**No.** The exploration is complete, but the user's intent is too broad. The orchestrator should ask the user which of the candidate improvements they want and confirm the validation/output behavior before writing a proposal/spec.
