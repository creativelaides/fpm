# Design: fpm Rust Wrapper

## Technical Approach

Windows-only Rust CLI wrapping PyManager (`py.exe`) via synchronous `std::process::Command`. Per-session version switching uses NTFS junctions (unprivileged directory reparse points) to a Python install directory, prepended to PATH via a generated PowerShell script. All runtime management is delegated to `py`; fpm never downloads Python. Version resolution walks the directory tree for `.python-version` or `pyproject.toml`, reducing PEP 440 specifiers against a cached `py list` runtime list.

## Architecture Decisions

### Decision: Shim Mechanism — NTFS Junction to Install Directory

**Choice**: Create a junction `<FPM_DIR>/multishells/<session-id>/` pointing to the resolved Python install directory (parent of `py list --one --format=exe` output). `fpm use` deletes the old junction (`std::fs::remove_dir` — NOT `remove_dir_all` which would follow into the target) and creates a new one.

**Alternatives**: File symlink (`symlink_file` — requires SeCreateSymbolicLinkPrivilege or Developer Mode); copy `python.exe` (breaks — exe needs sibling DLLs/Lib); forwarding-exe/fpm-as-shim (hardlink fpm.exe as python.exe, detect via `current_exe()` — adds invocation detection to main.rs, more complex).

**Rationale**: Junctions are unprivileged on NTFS (unlike symlinks), handle `pythonw.exe` automatically (it's in the install dir), put the entire install dir on PATH so `pip.exe`/`idle.exe` resolve correctly, and don't shadow `py.exe` (install dirs have no `py.exe` — that lives in `%WindowsApps%`). This is the closest adaptation of fnm's symlink model to Windows constraints. Requires the `junction` crate for creation; std only supports symlinks, not junctions.

### Decision: Skip PEP 514 Registry Fallback for v1

**Choice**: Rely solely on `py list --one --format=exe`. If it fails, error with "version not installed."

**Alternatives**: Read `HKEY_CURRENT_USER\Software\Python\<Company>\<Tag>\InstallPath` via `winreg` as fallback.

**Rationale**: PyManager 26.x+ is required and registers all runtimes. The registry fallback adds the `winreg` dependency and PEP 514 enumeration logic for an edge case that shouldn't occur in the supported environment. Ship without `winreg`; add later if real users hit unregistered runtimes.

### Decision: --silent-if-unchanged — Compare Junction Target Paths

**Choice**: Read the current junction's destination (`junction::get_target`), canonicalize it, and compare against the canonicalized install dir of the newly resolved runtime. If equal → already active, suppress stdout.

**Alternatives**: Compare tag strings against `PYTHON_MANAGER_DEFAULT` (fragile — same tag can resolve to different exe after update); compare exe paths directly (junction points to dir, not exe).

**Rationale**: Path comparison is the source of truth. The junction target IS the active install dir. Comparing canonical paths handles case/separator differences without string fragility.

### Decision: pyproject.toml Specifier Matching via pep440_rs

**Choice**: Use `pep440_rs` crate (astral/uv team) for `VersionSpecifiers` parsing and `Version` comparison. Parse `pyproject.toml` with the `toml` crate. Extract `[project] requires-python` (PEP 621) or `[tool.poetry.dependencies] python` (Poetry). Reduce specifier against cached runtime list: parse each runtime's tag to `pep440_rs::Version`, filter by specifier, select highest.

**Alternatives**: Hand-rolled specifier parser (bug-prone, reinvents PEP 440); `python-pkginfo` equivalent (none exists in Rust).

**Rationale**: PEP 440 specifier grammar is non-trivial (`>=3.12`, `~=3.13.0`, `==3.14.*`, `!=3.13.1`). `pep440_rs` is battle-tested by `uv`. Keeps matching logic correct with minimal code.

### Decision: Multishell Session Directory

**Choice**: `<FPM_DIR>/multishells/<session-id>/` where `session-id` = `{pid}_{random_u32}` (PID + random suffix for uniqueness). `FPM_DIR` defaults to `%LocalAppData%\fpm` via `etcetera`. Created by `fpm env`. Cleaned best-effort via a PowerShell `PowerShell.Exiting` engine event hook emitted in the env script. Stale dirs from crashed sessions are ignored (unique IDs prevent collision).

**Alternatives**: `tempfile::tempdir` (auto-deletes — we need persistent for session lifetime); UUID (adds `uuid` crate for marginal benefit over PID+random).

**Rationale**: PID alone could collide across process reuse; adding random suffix makes collision astronomically unlikely. No extra crate needed (`std::process::id()` + simple RNG). The exit hook is best-effort; the spec only requires stale dirs don't break other sessions.

### Decision: Test Strategy — assert_cmd + predicates, PyManager Trait Mocking

**Choice**: `assert_cmd` + `predicates` for CLI integration tests. `PyManagerOps` trait abstracts `py` calls; real impl spawns `py.exe`, test impl returns canned JSON. Unit tests for pure logic (version_file, specifier matching, JSON parse) use `tempfile` fixtures. Integration tests requiring real `py` are `#[ignore]` (run manually locally).

**Alternatives**: `rstest` (adds complexity); mock `py.exe` binary on PATH (fragile across CI environments).

**Rationale**: Trait injection lets unit tests run without PyManager installed. `#[ignore]` gates integration tests for machines with real `py`. No CI dependency on PyManager.

## Data Flow

### fpm env --shell powershell

```
fpm env ──→ config::fpm_dir() ──→ shim::create_session_dir(fpm_dir)
                                    │
                                    └─→ shell::powershell::render(session_dir, use_on_cd)
                                          │
                                          ├─ stdout: $env:FPM_DIR, $env:PATH prepend, $env:FPM_MULTISHELL_PATH
                                          └─ stdout: Set-Location hook (if --use-on-cd)
```

### fpm use 3.14

```
fpm use 3.14 ──→ pymanager::resolve_exe("3.14")
                    │  spawns: py list --one --format=exe 3.14
                    └─→ exe_path
                         │
                 shim::retarget(session_dir, exe_path.parent())
                     │  remove_dir(old junction) → junction::create(install_dir, session_dir)
                     │  // junction::create(target, junction_path) — target is the real dir, junction_path is the link
                     └─→ set PYTHON_MANAGER_DEFAULT=3.14 in-process
                         │
                    stdout: "Using Python 3.14"
```

### fpm use (no args)

```
fpm use ──→ version_file::resolve(cwd)
               │  walk up: .python-version (exact tag) → pyproject.toml (specifier)
               │           ├─ exact tag → return tag
               │           └─ specifier → pymanager::list_runtimes() (cached)
               │                           → filter by pep440_rs spec → highest match
               └─→ resolved_tag
                    │
              (proceed as fpm use <resolved_tag>)
```

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `Cargo.toml` | Create | Project manifest with dependencies (see below) |
| `src/main.rs` | Create | Entry point: parse args, dispatch or pass-through |
| `src/cli.rs` | Create | `clap` derive: `Cli`, `Commands` enum (`Use`, `List`, `Current`, `Default`, `Env`, `Install`), `--shell`, `--use-on-cd`, `--silent-if-unchanged` flags |
| `src/config.rs` | Create | `FPM_DIR` resolution via `etcetera`, `pymanager.json` path (`%AppData%\Python\`), constants |
| `src/error.rs` | Create | `thiserror` domain errors: `FpmError` enum (`PyNotFound`, `VersionNotInstalled`, `NoVersionFile`, `ShimError`, `ConfigError`, `SpecNotSatisfied`) |
| `src/pymanager.rs` | Create | `PyManager` struct with `OnceCell`-cached `Vec<Runtime>`, `resolve_exe(tag)`, `list_runtimes()`, `read_default()`, `write_default(tag)`, `install(tag)`; `PyManagerOps` trait for testing |
| `src/version_file.rs` | Create | `resolve(cwd) -> VersionSpec`, upward walk, `.python-version` parser, `pyproject.toml` parser (`toml` + `pep440_rs`), specifier reduction |
| `src/shim.rs` | Create | `create_session_dir(fpm_dir) -> PathBuf`, `retarget(session_dir, install_dir)`, `current_target(session_dir) -> Option<PathBuf>` via `junction` crate |
| `src/shell/mod.rs` | Create | `Shell` trait with `render(session_dir, use_on_cd) -> String` |
| `src/shell/powershell.rs` | Create | PowerShell env script generator: `$env:FPM_DIR`, PATH prepend, `FPM_MULTISHELL_PATH`, `Set-Location` hook for `--use-on-cd`, `PowerShell.Exiting` cleanup hook |
| `src/commands/mod.rs` | Create | Command dispatch, shared context (config, pymanager, session_dir from `FPM_MULTISHELL_PATH`) |
| `src/commands/use_cmd.rs` | Create | `fpm use` logic: resolve version (explicit or file), `--silent-if-unchanged` comparison, retarget junction, set env var |
| `src/commands/list.rs` | Create | `fpm list`: call `pymanager.list_runtimes()`, render table (version, tag, path, default marker) |
| `src/commands/current.rs` | Create | `fpm current`: read `PYTHON_MANAGER_DEFAULT` or `pymanager.json` default_tag, spawn `py -V` for active version |
| `src/commands/default.rs` | Create | `fpm default [tag]`: read/write `default_tag` in `pymanager.json`, preserve other keys |
| `src/commands/env_cmd.rs` | Create | `fpm env --shell powershell [--use-on-cd]`: create session dir, render shell script |
| `src/commands/install.rs` | Create | `fpm install <tag>`: spawn `py install <tag>`, stream stdout/stderr, propagate exit code |
| `src/commands/passthrough.rs` | Create | Spawn `py.exe` with all args verbatim, inherit stdio, propagate exit code |
| `README.md` | Create | Install snippet, supported commands, `$PROFILE` locations |

## Interfaces / Contracts

### PyManagerOps Trait

```rust
trait PyManagerOps {
    fn list_runtimes(&mut self) -> Result<&[Runtime]>;
    fn resolve_exe(&mut self, tag: &str) -> Result<PathBuf>;
    fn read_default(&self) -> Result<Option<String>>;
    fn write_default(&mut self, tag: &str) -> Result<()>;
    fn install(&mut self, tag: &str) -> Result<i32>;
}

struct Runtime {
    tag: String,       // e.g. "3.14-64"
    version: String,   // e.g. "3.14.6"
    executable: PathBuf,
    is_default: bool,
}

struct PyManager { runtimes: OnceCell<Vec<Runtime>> }  // real impl
struct MockPyManager { fixtures: Vec<Runtime> }        // test impl
```

### Shim Module

```rust
fn create_session_dir(fpm_dir: &Path) -> Result<PathBuf>;
// Creates <fpm_dir>/multishells/<pid>_<random>/, returns path

fn retarget(session_dir: &Path, install_dir: &Path) -> Result<()>;
// remove_dir(session_dir) if junction exists, then junction::create(install_dir, session_dir)
// junction::create(target, junction_path) — target is the real dir, junction_path is the link

fn current_target(session_dir: &Path) -> Result<Option<PathBuf>>;
// junction::get_target(session_dir), canonicalized
```

### Exit Code Mapping

| Error | Exit Code | Message |
|-------|-----------|---------|
| `PyNotFound` | 1 | "PyManager 26.x+ is required (py.exe not found on PATH)" |
| `VersionNotInstalled` | 2 | "Version <tag> is not installed" |
| `NoVersionFile` | 3 | "No .python-version or pyproject.toml found walking up from cwd" |
| `SpecNotSatisfied` | 4 | "No installed runtime satisfies <specifier>" |
| `ShimError` | 5 | "Failed to update session shim: <detail>" |
| `ConfigError` | 6 | "Failed to read/write pymanager.json: <detail>" |
| Pass-through/delegated | child code | (propagated) |

## Crate Dependencies

```toml
[dependencies]
clap = { version = "4.5", features = ["derive", "env", "cargo"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "2.0"
anyhow = "1.0"
etcetera = "0.8"
tempfile = "3.10"
junction = "1.2"
toml = "0.8"
pep440_rs = "0.7"

[dev-dependencies]
assert_cmd = "2.0"
predicates = "3.1"
```

**Dropped from exploration list**: `winreg` (registry fallback deferred), `colored` (defer to follow-up), `tokio` (synchronous is sufficient), `which` (use `std::env::var("PATH")` + manual scan or let `Command::new("py")` fail naturally).

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| Unit | `.python-version` parsing (comments, blanks, empty) | Fixture files in `tempfile` dirs |
| Unit | `pyproject.toml` specifier extraction + PEP 440 matching | Canned TOML fixtures, `pep440_rs` spec reduction |
| Unit | Upward walk algorithm | `tempfile` dir tree, verify stop at first match |
| Unit | `py list` JSON parsing into `Vec<Runtime>` | Canned JSON fixtures |
| Unit | `pymanager.json` read/write, key preservation | `tempfile` config files |
| Unit | Junction retarget + `current_target` round-trip | `tempfile` dirs, `junction` crate |
| Unit | PowerShell script generation (contains expected env vars) | String assertions on output |
| Integration | CLI dispatch: recognized subcommands route correctly | `assert_cmd` against compiled binary |
| Integration | Pass-through forwards args to `py.exe` verbatim | `assert_cmd` (requires `py` — `#[ignore]`) |
| Integration | `fpm env --shell powershell` creates dir + emits script | `assert_cmd` + temp `FPM_DIR` |
| Integration | `fpm use` full flow (resolve, retarget, env var) | `#[ignore]` — requires real PyManager |
| E2E | Install snippet works in real PowerShell | Manual; documented in README |

## Migration / Rollout

No migration required (greenfield). Rollout: `cargo install --path .` puts `fpm.exe` on PATH, then user adds the documented snippet to `$PROFILE`. Rollback: remove snippet, remove binary.

## Open Questions

- [ ] Should `fpm current` spawn `py -V` (adds ~190ms) or read `PYTHON_MANAGER_DEFAULT`/`default_tag` and resolve from cached list (faster but may differ from what `py` actually launches)? Design recommends: spawn `py -V` for accuracy, cache result is secondary.
- [ ] Poetry `[tool.poetry.dependencies] python` support — include in v1 or defer? Design includes it; minimal extra code after `toml` parse.