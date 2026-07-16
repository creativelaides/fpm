# Proposal: fpm Rust Wrapper

## Intent

Replace the hand-written PowerShell `py { ... }` block in `$profile` with a fast, hardened Rust CLI named `fpm` that mirrors fnm's ergonomics for Python on Windows. It delegates all runtime management to the official PyManager (`py`/`pymanager`) instead of reimplementing downloads or registry logic.

## Scope

### In Scope
- Windows-only Rust CLI: `fpm use`, `fpm list`, `fpm current`, `fpm default`, `fpm env --shell powershell`, `fpm install`, `--version`/`--help`.
- Shim-based switching: `fpm env` emits a PowerShell script that prepends a per-session shim directory to PATH; `fpm use` rewrites the `python.exe` shim to point to the resolved real executable.
- Pass-through of unrecognized `fpm <args>` to `py.exe`.
- `fpm use` (no args) resolves version from `.python-version` or `pyproject.toml` (`requires.python-version`), walking up the directory tree.
- Documented manual PowerShell profile snippet (no auto profile mutation).

### Out of Scope
- macOS/Linux, cmd/bash/zsh/fish, venv management, multi-arch/32-bit switching.
- Direct Python downloads; `fpm install` only delegates to `py install`.
- Persistent effects for `fpm use`; `fpm default` is the persistent variant.
- `fpm completions`, `fpm alias`, and `fpm exec` in the first slice.

## Capabilities

### New Capabilities
- `fpm-core`: CLI subcommand routing and pass-through to `py.exe`.
- `python-version-switching`: per-session shim rewrite on `fpm use` plus `PYTHON_MANAGER_DEFAULT` session variable.
- `powershell-shell-integration`: `fpm env` script generator and documented install snippet.
- `pymanager-delegation`: parsing `py list --format=json`, reading/writing `pymanager.json`, PEP 514 registry reads.
- `version-file-resolution`: `.python-version` and `pyproject.toml` lookup walking upward.

### Modified Capabilities
- None.

## Approach

Scaffold a Rust project with `clap` (derive), `serde_json`, `winreg`, `anyhow`, `thiserror`, `etcetera`, and `tempfile`. Use synchronous `std::process::Command` to spawn `py`.

- `fpm use <version>` resolves via `py list --one --format=exe <version>`, sets `$env:PYTHON_MANAGER_DEFAULT`, and rewrites the per-session `python.exe` shim in `FPM_DIR/multishells/<session-id>/`.
- `fpm use` (no args) walks cwd upward for `.python-version`, then `pyproject.toml`.
- `fpm env --shell powershell` creates the multishell directory and emits `$env:FPM_DIR`, `$env:PATH = "<shim-dir>;$env:PATH"`, plus a `Set-Location` hook for `--use-on-cd`.
- `fpm list` caches one `py list --format=json` call per invocation and renders a friendly table.
- `fpm default` reads/writes `default_tag` in `%AppData%\Python\pymanager.json`.
- `fpm install <version>` delegates to `py install <version>` with output streamed through.
- Pass-through forwards unrecognized args to `py.exe` verbatim.

Install documentation mirrors fnm's README style. Provide the snippet:

```powershell
if (-not (Test-Path $profile)) { New-Item $profile -Force }
fpm env --use-on-cd --shell powershell | Out-String | Invoke-Expression
```

Profile locations: PowerShell 6+ uses `$PROFILE` (commonly `%UserProfile%\Documents\PowerShell\Microsoft.PowerShell_profile.ps1`); PowerShell 5 uses `$PROFILE` under `WindowsPowerShell`. The apply phase emits these instructions; it does not mutate `$profile` automatically.

## Affected Areas

| Area | Impact | Description |
|---|---|---|
| `Cargo.toml` | New | Rust project scaffold |
| `src/main.rs`, `src/cli.rs` | New | Entry point and clap subcommands |
| `src/commands/*.rs` | New | `use`, `list`, `current`, `default`, `install`, `env` implementations |
| `src/shell/powershell.rs` | New | PowerShell env script generator |
| `src/pymanager.rs` | New | PyManager JSON/registry abstraction |
| `README.md` | New | Install snippet and supported commands |

## Risks

| Risk | Likelihood | Mitigation |
|---|---|---|
| PATH shadowing of `py` app aliases | Med | Shim directory is only prepended by the emitted `env` script; keep `%WindowsApps%` later in PATH |
| `py list` spawn cost (~150 ms) | High | Cache parsed JSON for the lifetime of each `fpm` invocation; avoid repeated spawns |
| Shim directory lifecycle left behind | Med | Use `tempfile` per-session dirs; document cleanup |
| Per-user vs all-users python.org installs | Low | Rely on `py list` merging; document PyManager 26.x+ support only |

## Rollback Plan

1. Remove the `fpm env` line from `$profile`.
2. Delete the `fpm` binary from PATH.
3. Clear any leftover `$env:PYTHON_MANAGER_DEFAULT` value in the session.
4. Restore the previous PowerShell wrapper block from backup or version control.

## Dependencies

- PyManager 26.x+ installed on Windows (`py.exe`/`pymanager.exe` on PATH).
- `cargo`/`rustc` 1.96+ for build.

## Success Criteria

- [ ] `fpm use 3.14` switches the active Python version in the current PowerShell session.
- [ ] `fpm use` without args selects the version declared in `.python-version` or `pyproject.toml`.
- [ ] `fpm list` shows installed runtimes without duplicating `py list` output.
- [ ] Unrecognized `fpm <args>` pass through to `py.exe`.
- [ ] `fpm env --shell powershell` emits a valid script and the documented install snippet works.
