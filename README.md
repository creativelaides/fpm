<div align="center">
<div style="display: inline-block; border-radius: 50%; overflow: hidden; width: 200px; height: 200px;">
  <img src="assets/fpy_logo_python.png" width="200" height="200" alt="fpy — Friendly Python" />
</div>

# fpy (Friendly Python)

A fast Python version manager for Windows, built in Rust. Wraps the official
Python Install Manager (`py`/`pymanager`) for per-session and global Python
version switching, inspired by [fnm](https://github.com/Schniz/fnm).

[![Crates.io](https://img.shields.io/crates/v/fpy?style=flat-square)](https://crates.io/crates/fpy)
[![npm](https://img.shields.io/npm/v/@kwak-projects/fpy?style=flat-square)](https://www.npmjs.com/package/@kwak-projects/fpy)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue?style=flat-square)](LICENSE)
[![CI](https://img.shields.io/github/actions/workflow/status/creativelaides/fpy/ci.yml?style=flat-square&label=CI)](https://github.com/creativelaides/fpy/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/creativelaides/fpy?style=flat-square&label=Release)](https://github.com/creativelaides/fpy/releases/latest)

</div>

> [English](README.md) | [Espanol](lang/README.es.md)

## Features

- **Windows-native**: Built for Windows with NTFS junction-based shim switching.
- **Per-session switching**: `fpy use` switches Python versions per shell
  session without touching the global default.
- **Global default**: `fpy default` sets the global default Python version,
  persisting to `pymanager.json` and activating it in the current session
  immediately.
- **Version file support**: Reads `.python-version` and `pyproject.toml`
  (`requires-python` / Poetry `python` dependency) with PEP 440 specifier
  matching.
- **Pass-through to `py`**: Any unrecognized command forwards verbatim to
  `py.exe`, so existing aliases and workflows keep working.
- **Shell integration**: Generates a PowerShell script that sets up
  `FPY_DIR`, `FPY_MULTISHELL_PATH`, and PATH — with optional `use-on-cd`
  automatic switching.

## Installation

### Prerequisites

- **PyManager 26.x+** installed on Windows (`py.exe` on PATH). Download from
  [python.org](https://www.python.org/downloads/) or run
  `winget install Python.Python.3` (the official launcher ships with Python
  installs).

### Build from source

```sh
git clone https://github.com/creativelaides/fpy.git
cd fpy
cargo build --release
```

Add the `target/release` directory to your PATH:

```powershell
# Add to your PowerShell profile (see Shell Setup below for profile path)
$env:PATH += ";$PWD\target\release"
```

> **Using cargo install (future)**: Once published to crates.io, you'll be able
> to run `cargo install fpy` to install the binary directly.

## Shell Setup

### PowerShell

Add the following to the end of your PowerShell profile file:

```powershell
fpy env --use-on-cd --shell powershell | Out-String | Invoke-Expression
```

This evaluates the `fpy env` script every time a new shell starts, setting up
`FPY_DIR`, prepending the session shim directory to `PATH`, and installing
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
fpy env --shell powershell | Out-String | Invoke-Expression
```

## Usage

### Commands

| Command                      | Description                                                        |
| ---------------------------- | ------------------------------------------------------------------ |
| `fpy use [version]`          | Switch to a Python version for this session. Resolves from         |
|                              | `.python-version` or `pyproject.toml` if no version is given.       |
| `fpy list`                   | List installed Python runtimes.                                    |
| `fpy list-remote [--pre]`    | Fetches python.org versions, filters pre-releases unless `--pre`   |
|                              | is specified, caches for 24 hours locally using etcetera in JSON.  |
| `fpy current`                | Print the currently active Python version.                         |
| `fpy default [tag]`          | Set the global default Python version (writes `pymanager.json` and  |
|                              | activates it in the current session). Use `--unset` to remove or   |
|                              | `--dry-run` to preview.                                             |
| `fpy env --shell powershell` | Emit a shell integration script. Use `--use-on-cd` for automatic   |
|                              | switching on directory change.                                     |
| `fpy install <tag>`          | Install a Python version via `py install <tag>`.                   |
| `fpy --version`              | Displays fpy crate version, launcher version, and active python version. |

### Examples

```sh
# List installed Python versions
fpy list

# Switch to Python 3.14 for this session
fpy use 3.14

# Print the active version
fpy current

# Set 3.13 as the global default (persists + activates immediately)
fpy default 3.13

# Preview setting a default without making changes
fpy default 3.14 --dry-run

# Remove the global default
fpy default --unset

# Install a new Python version
fpy install 3.12

# Pass through to py.exe — all unrecognized args forward verbatim
fpy -m http.server 8000
fpy script.py
```

### Version file resolution

`fpy use` with no arguments walks up from the current directory looking for:

1. `.python-version` — contains a version tag (e.g. `3.14` or `3.14-64`).
2. `pyproject.toml` — `[project] requires-python` (PEP 621) or
   `[tool.poetry.dependencies] python` with a PEP 440 specifier (e.g.
   `>=3.12`, `~3.13`, `==3.14.*`).

The first match wins. For specifiers, fpy reduces against the installed
runtime list and selects the highest matching version.

```sh
# Create a .python-version file
echo "3.14" > .python-version

# Now `fpy use` (no args) switches to 3.14
fpy use
```

## Configuration

### FPY_DIR

The fpy data directory. Defaults to `%LocalAppData%\fpy`. Override with the
`FPY_DIR` environment variable:

```powershell
$env:FPY_DIR = "D:\my-fpy-data"
```

Session shim directories are created under `FPY_DIR/multishells/<session-id>/`.

### PYTHON_MANAGER_DEFAULT

Set in-process by `fpy use` to mark the active version for the current
session. Read by `fpy current` as the primary source of truth.

### pymanager.json

Located at `%AppData%\Python\pymanager.json`. Managed by `fpy default` (and
PyManager itself). `fpy use` does **not** write to this file — switching is
session-only.

## Session directory lifecycle

Each `fpy env` invocation creates a unique session directory under
`FPY_DIR/multishells/<pid>_<random>/`. The generated PowerShell script
registers a `PowerShell.Exiting` engine event that best-effort removes the
session directory on clean shell exit.

Stale directories from crashed shells do not break other sessions — the
unique session ID prevents collisions. You can safely clean up stale
directories manually:

```powershell
Remove-Item -Recurse -Force "$env:FPY_DIR\multishells\*"
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

---

<div align="center">
<div style="display: inline-block; border-radius: 50%; overflow: hidden; width: 80px; height: 80px;">
  <img src="assets/kwak_logo_sponsor.jpg" width="80" height="80" alt="KWAK — Kit for Windows Application Kickstart" />
</div>

<sub>Part of <strong>KWAK</strong> — <em>Kit for Windows Application Kickstart</em></sub>
</div>
