<div align="center">

# fpm (Fast Python Manager)

A fast Python version manager for Windows, built in Rust. Wraps the official
Python Install Manager (`py`/`pymanager`) for per-session and global Python
version switching, inspired by [fnm](https://github.com/Schniz/fnm).

[![Crates.io](https://img.shields.io/crates/v/fpm?style=flat-square)](https://crates.io/crates/fpm)
[![npm](https://img.shields.io/npm/v/fpm?style=flat-square)](https://www.npmjs.com/package/fpm)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue?style=flat-square)](LICENSE)
[![CI](https://img.shields.io/github/actions/workflow/status/creativelaides/fpm/ci.yml?style=flat-square&label=CI)](https://github.com/creativelaides/fpm/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/creativelaides/fpm?style=flat-square&label=Release)](https://github.com/creativelaides/fpm/releases/latest)

<br>

<div style="display: inline-block; border-radius: 50%; overflow: hidden; width: 125px; height: 125px;">
  <img src="assets/kwak_logo_sponsor.jpg" width="125" height="125" alt="KWAK — Kit for Windows Application Kickstart" />
</div>

<sub>Part of <strong>KWAK</strong> — <em>Kit for Windows Application Kickstart</em></sub>

</div>

> [English](README.md) | [Espanol](lang/README.es.md)

## Features

- **Windows-native**: Built for Windows with NTFS junction-based shim switching.
- **Per-session switching**: `fpm use` switches Python versions per shell
  session without touching the global default.
- **Global default**: `fpm default` sets the global default Python version,
  persisting to `pymanager.json` and activating it in the current session
  immediately.
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
| `fpm default [tag]`          | Set the global default Python version (writes `pymanager.json` and  |
|                              | activates it in the current session). Use `--unset` to remove or   |
|                              | `--dry-run` to preview.                                             |
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

# Set 3.13 as the global default (persists + activates immediately)
fpm default 3.13

# Preview setting a default without making changes
fpm default 3.14 --dry-run

# Remove the global default
fpm default --unset

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