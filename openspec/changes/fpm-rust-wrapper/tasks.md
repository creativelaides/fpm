# Tasks: fpm Rust Wrapper

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | ~1450 (17 src + Cargo.toml + README + tests) |
| 400-line budget risk | High |
| Chained PRs recommended | Yes |
| Suggested split | PR1 scaffold+error+config → PR2 pymanager+version_file → PR3 shim+shell → PR4 commands+cli → PR5 tests+README |
| Delivery strategy | ask-on-risk |
| Chain strategy | pending |

Decision needed before apply: Yes
Chained PRs recommended: Yes
Chain strategy: pending
400-line budget risk: High

### Suggested Work Units

| Unit | Goal | Likely PR | Notes |
|------|------|-----------|-------|
| 1 | Scaffold: git init, cargo init, Cargo.toml, .gitignore, hello main | PR1 | base=main; unblocks VCS for all later PRs |
| 2 | error.rs + config.rs | PR1 | depends on U1; ~80 lines |
| 3 | pymanager.rs + unit tests | PR2 | depends on U2; ~260 lines |
| 4 | version_file.rs + unit tests | PR2 | depends on U3; ~230 lines |
| 5 | shim.rs + unit tests | PR3 | depends on U2; ~130 lines |
| 6 | shell/mod.rs + shell/powershell.rs + tests | PR3 | depends on U5; ~160 lines |
| 7 | commands/mod.rs + passthrough + install | PR4 | depends on U3; ~100 lines |
| 8 | commands/list + current + default | PR4 | depends on U3; ~140 lines |
| 9 | commands/env_cmd | PR4 | depends on U5,U6; ~40 lines |
| 10 | commands/use_cmd | PR4 | depends on U3,U4,U5; ~70 lines |
| 11 | cli.rs + main.rs wiring | PR4 | depends on U7-U10; ~80 lines |
| 12 | integration tests (assert_cmd) | PR5 | depends on U11; ~180 lines |
| 13 | README.md | PR5 | depends on U11; ~80 lines |

## Phase 1: Foundation

- [x] 1.1 `git init`, `.gitignore` (target/), `cargo init --name fpm`, initial commit. Files: .gitignore, Cargo.toml, src/main.rs. Spec: fpm-core. Deps: none. ~60 lines.
- [x] 1.2 Create `src/error.rs`: `FpmError` enum (PyNotFound, VersionNotInstalled, NoVersionFile, SpecNotSatisfied, ShimError, ConfigError) + exit-code mapping (1-6). Spec: fpm-core Exit Code Propagation. Deps: 1.1. ~40 lines.
- [x] 1.3 Create `src/config.rs`: `fpm_dir()` via etcetera (default %LocalAppData%\fpm), `pymanager_json_path()` (%AppData%\Python\), constants. Spec: powershell-shell-integration, pymanager-delegation. Deps: 1.1. ~40 lines.

## Phase 2: Core Implementation

- [x] 2.1 Create `src/pymanager.rs`: `PyManagerOps` trait, `Runtime` struct, `PyManager` (OnceCell cache), `MockPyManager`, `list_runtimes()` (spawn `py list --format=json`, parse once), `resolve_exe(tag)` (`py list --one --format=exe`), `read_default()`, `write_default(tag)`, `install(tag)`. Spec: pymanager-delegation (all reqs). Deps: 1.2,1.3. ~180 lines.
- [x] 2.2 Unit tests in `src/pymanager.rs`: JSON parse from canned fixture, malformed JSON error, default read/write key preservation, missing pymanager.json creation. Spec: pymanager-delegation Parse/Default. Deps: 2.1. ~80 lines.
- [x] 2.3 Create `src/version_file.rs`: `resolve(cwd)` upward walk, `.python-version` parser (comments/blanks/empty), `pyproject.toml` parser (toml+pep440_rs, requires-python + tool.poetry), specifier reduction vs cached runtime list (highest match). Spec: version-file-resolution (all reqs). Deps: 2.1. ~140 lines.
- [x] 2.4 Unit tests in `src/version_file.rs`: fixture trees for cwd-wins, ancestor, no-file, comment/blank, empty file, specifier matching (>=, ~, ==.*), no-satisfies, malformed toml skip, .python-version precedence. Spec: version-file-resolution. Deps: 2.3. ~90 lines.
- [x] 2.5 Create `src/shim.rs`: `create_session_dir(fpm_dir)` (`<pid>_<random>`), `retarget(session_dir, install_dir)` (remove_dir + junction::create, NOT remove_dir_all), `current_target(session_dir)` (junction::get_target, canonicalize). Spec: python-version-switching, powershell-shell-integration. Deps: 1.2. ~80 lines.
- [x] 2.6 Unit tests in `src/shim.rs`: retarget+current_target round-trip on tempfile dirs, unique session ids across calls. Spec: powershell-shell-integration Session ID Unique. Deps: 2.5. ~50 lines.
- [x] 2.7 Create `src/shell/mod.rs`: `Shell` trait `render(session_dir, use_on_cd) -> String`. Spec: powershell-shell-integration. Deps: none. ~20 lines.
- [x] 2.8 Create `src/shell/powershell.rs`: script generator — `$env:FPM_DIR`, PATH prepend, `FPM_MULTISHELL_PATH`, `Set-Location` hook (use-on-cd → `fpm use --silent-if-unchanged`), `PowerShell.Exiting` cleanup hook. Spec: powershell-shell-integration (all reqs). Deps: 2.7. ~100 lines.
- [x] 2.9 Unit tests in `src/shell/powershell.rs`: output contains expected env vars, use-on-cd emits Set-Location override, no use-on-cd omits hook, exit cleanup present. Spec: powershell-shell-integration. Deps: 2.8. ~40 lines.

## Phase 3: Commands

- [x] 3.1 Create `src/commands/mod.rs`: `Context` (config, pymanager, session_dir from FPM_MULTISHELL_PATH), dispatch helper. Spec: fpm-core. Deps: 2.1. ~40 lines.
- [x] 3.2 Create `src/commands/passthrough.rs`: spawn `py.exe` with all args verbatim, inherit stdio, propagate exit code; py missing → PyNotFound. Spec: fpm-core Pass-through. Deps: 3.1. ~30 lines.
- [x] 3.3 Create `src/commands/install.rs`: spawn `py install <tag>`, stream stdout/stderr, propagate exit code. Spec: pymanager-delegation install. Deps: 3.1. ~30 lines.
- [x] 3.4 Create `src/commands/list.rs`: call `list_runtimes()`, render table (version, tag, path, default marker). Spec: fpm-core, pymanager-delegation. Deps: 3.1. ~50 lines.
- [x] 3.5 Create `src/commands/current.rs`: read `PYTHON_MANAGER_DEFAULT` or `default_tag`, spawn `py -V` for active version. Spec: pymanager-delegation current. Deps: 3.1. ~40 lines.
- [x] 3.6 Create `src/commands/default.rs`: read/print `default_tag` (no arg), write `default_tag` preserving other keys (with arg), create file if missing. Spec: pymanager-delegation default. Deps: 3.1. ~50 lines.
- [x] 3.7 Create `src/commands/env_cmd.rs`: `--shell powershell [--use-on-cd]` → create session dir, render shell script to stdout. Spec: powershell-shell-integration. Deps: 2.8,3.1. ~40 lines.
- [x] 3.8 Create `src/commands/use_cmd.rs`: resolve version (explicit or version_file::resolve), `--silent-if-unchanged` (compare canonical junction target vs install dir, suppress stdout if equal), retarget junction, set `PYTHON_MANAGER_DEFAULT`, print "Using Python X". Spec: python-version-switching (all reqs). Deps: 2.1,2.3,2.5,3.1. ~70 lines.

## Phase 4: CLI Wiring

- [x] 4.1 Create `src/cli.rs`: clap derive `Cli`, `Commands` enum (Use,List,Current,Default,Env,Install), `--shell`, `--use-on-cd`, `--silent-if-unchanged` flags. Spec: fpm-core Subcommand Routing. Deps: none. ~50 lines.
- [x] 4.2 Rewrite `src/main.rs`: parse args, dispatch recognized subcommand, detect unrecognized first token → passthrough, `--version` via clap cargo feature, FpmError→exit code mapping. Spec: fpm-core (all reqs). Deps: 4.1, all Phase 3. ~30 lines.

## Phase 5: Integration Tests

- [x] 5.1 `tests/cli_dispatch.rs` (assert_cmd): `fpm --version` prints crate version, `fpm --help` lists subcommands exit 0, recognized subcommand routes. Spec: fpm-core Subcommand Routing. Deps: 4.2. ~60 lines.
- [x] 5.2 `tests/passthrough.rs` `#[ignore]`: forwards args to py verbatim, propagates exit code, py missing → non-zero + stderr. Spec: fpm-core Pass-through. Deps: 4.2. ~40 lines.
- [x] 5.3 `tests/env_cmd.rs`: `fpm env --shell powershell` with temp FPM_DIR creates dir + emits script with expected env vars; `--use-on-cd` emits Set-Location. Spec: powershell-shell-integration. Deps: 4.2. ~50 lines.
- [x] 5.4 `tests/use_cmd.rs` `#[ignore]`: full `fpm use` flow (resolve, retarget, env var, silent-if-unchanged). Spec: python-version-switching. Deps: 4.2. ~30 lines.

## Phase 6: Documentation

- [x] 6.1 Create `README.md`: install snippet (`fpm env --use-on-cd --shell powershell | Out-String | Invoke-Expression`), $PROFILE locations (PS6+, PS5), supported commands table, cleanup note for stale multishells, PyManager 26.x+ requirement. Spec: powershell-shell-integration Install Snippet. Deps: 4.2. ~80 lines.