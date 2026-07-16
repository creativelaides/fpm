# fpm

A fast Python version manager for Windows, built in Rust. Wraps the official
Python Install Manager (`py`/`pymanager`) for per-session Python version
switching, inspired by [fnm](https://github.com/Schniz/fnm).

## Features

- **Windows-native**: Built for Windows with NTFS junction-based shim switching.
- **Per-session switching**: Switch Python versions per shell session without
  touching the global default.
- **Version file support**: Reads `.python-version` and `pyproject.toml`
  (`requires-python` / Poetry `python` dependency) with PEP 440 specifier
  matching.
- **Pass-through to `py`**: Any unrecognized command forwards verbatim to
  `py.exe`, so existing aliases and workflows keep working.
- **Shell integration**: Generates a PowerShell script that sets up
  `FPM_DIR`, `FPM_MULTISHELL_PATH`, and PATH — with optional `use-on-cd`
  automatic switching.

## Installation

### Prerequisites

- **PyManager 26.x+** installed on Windows (`py.exe` on PATH). Download from
  [python.org](https://www.python.org/downloads/) or run
  `winget install Python.Python.3` (the official launcher ships with Python
  installs).

### Build from source

```sh
git clone https://github.com/creativelaides/fpm.git
cd fpm
cargo build --release
```

Add the `target/release` directory to your PATH:

```powershell
# Add to your PowerShell profile (see Shell Setup below for profile path)
$env:PATH += ";$PWD\target\release"
```

> **Using cargo install (future)**: Once published to crates.io, you'll be able
> to run `cargo install fpm` to install the binary directly.

## Shell Setup

### PowerShell

Add the following to the end of your PowerShell profile file:

```powershell
fpm env --use-on-cd --shell powershell | Out-String | Invoke-Expression
```

This evaluates the `fpm env` script every time a new shell starts, setting up
`FPM_DIR`, prepending the session shim directory to `PATH`, and installing
the `Set-Location` hook for automatic version switching on `cd`.

#### Profile locations

| Shell version      | Profile path                                                                     |
| ------------------ | -------------------------------------------------------------------------------- |
| PowerShell 6+      | `%userprofile%\Documents\PowerShell\Microsoft.PowerShell_profile.ps1`            |
| Windows PowerShell | `%userprofile%\Documents\WindowsPowerShell\Microsoft.PowerShell_profile.ps1`    |

To create the profile if it doesn't exist:

```powershell
if (-not (Test-Path $profile)) { New-Item $profile -Force }
```

To edit the profile:

```powershell
Invoke-Item $profile
```

#### Without use-on-cd

If you prefer manual switching (no automatic `cd` hook), omit `--use-on-cd`:

```powershell
fpm env --shell powershell | Out-String | Invoke-Expression
```

## Usage

### Commands

| Command                      | Description                                                        |
| ---------------------------- | ------------------------------------------------------------------ |
| `fpm use [version]`          | Switch to a Python version for this session. Resolves from         |
|                              | `.python-version` or `pyproject.toml` if no version is given.       |
| `fpm list`                   | List installed Python runtimes.                                    |
| `fpm current`                | Print the currently active Python version.                         |
| `fpm default [tag]`          | Read or set the default Python version (writes `pymanager.json`).  |
| `fpm env --shell powershell` | Emit a shell integration script. Use `--use-on-cd` for automatic   |
|                              | switching on directory change.                                     |
| `fpm install <tag>`          | Install a Python version via `py install <tag>`.                   |

### Examples

```sh
# List installed Python versions
fpm list

# Switch to Python 3.14 for this session
fpm use 3.14

# Print the active version
fpm current

# Set 3.13 as the default (persists across sessions via pymanager.json)
fpm default 3.13

# Install a new Python version
fpm install 3.12

# Pass through to py.exe — all unrecognized args forward verbatim
fpm -m http.server 8000
fpm script.py
```

### Version file resolution

`fpm use` with no arguments walks up from the current directory looking for:

1. `.python-version` — contains a version tag (e.g. `3.14` or `3.14-64`).
2. `pyproject.toml` — `[project] requires-python` (PEP 621) or
   `[tool.poetry.dependencies] python` with a PEP 440 specifier (e.g.
   `>=3.12`, `~3.13`, `==3.14.*`).

The first match wins. For specifiers, fpm reduces against the installed
runtime list and selects the highest matching version.

```sh
# Create a .python-version file
echo "3.14" > .python-version

# Now `fpm use` (no args) switches to 3.14
fpm use
```

## Configuration

### FPM_DIR

The fpm data directory. Defaults to `%LocalAppData%\fpm`. Override with the
`FPM_DIR` environment variable:

```powershell
$env:FPM_DIR = "D:\my-fpm-data"
```

Session shim directories are created under `FPM_DIR/multishells/<session-id>/`.

### PYTHON_MANAGER_DEFAULT

Set in-process by `fpm use` to mark the active version for the current
session. Read by `fpm current` as the primary source of truth.

### pymanager.json

Located at `%AppData%\Python\pymanager.json`. Managed by `fpm default` (and
PyManager itself). `fpm use` does **not** write to this file — switching is
session-only.

## Session directory lifecycle

Each `fpm env` invocation creates a unique session directory under
`FPM_DIR/multishells/<pid>_<random>/`. The generated PowerShell script
registers a `PowerShell.Exiting` engine event that best-effort removes the
session directory on clean shell exit.

Stale directories from crashed shells do not break other sessions — the
unique session ID prevents collisions. You can safely clean up stale
directories manually:

```powershell
Remove-Item -Recurse -Force "$env:FPM_DIR\multishells\*"
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup, commit
conventions, and CI details.

For interactive conventional commits:

```sh
pnpm cz
```

For changesets (version bumps and changelog):

```sh
pnpm changeset
```

## License

MIT