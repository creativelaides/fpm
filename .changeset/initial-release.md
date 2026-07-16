---
"fpm": minor
---

Initial release of fpm (Fast Python Manager) v0.1.0.

A fast Python version manager for Windows, built in Rust. Wraps the official Python Install Manager (`py`/`pymanager`) for per-session Python version switching, inspired by fnm.

## Features

- Windows-native with NTFS junction-based shim switching
- Per-session version switching via `fpm use <version>`
- Version file resolution from `.python-version` and `pyproject.toml` (PEP 621 + Poetry)
- PowerShell shell integration via `fpm env --shell powershell`
- Pass-through to `py.exe` for unrecognized commands
- `fpm install` delegates to `py install`
- `fpm default` reads/writes `pymanager.json`