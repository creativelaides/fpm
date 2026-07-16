# python-version-switching Specification

## Purpose

Per-session Python version switching via a rewriteable shim, fnm-style. `fpm use`
resolves a runtime, rewrites the per-session `python.exe` shim to point at the
real executable, and sets `PYTHON_MANAGER_DEFAULT` for the session. Effects are
session-only; `pymanager.json` is NOT written by `use`.

## Requirements

### Requirement: fpm use With Explicit Version

`fpm use <version>` SHALL resolve the runtime matching `<version>` via
`py list --one --format=exe <version>`, set the `PYTHON_MANAGER_DEFAULT`
environment variable to `<version>` for the current process, and rewrite the
per-session shim's `python.exe` to point at the resolved executable path. It
SHALL NOT write `pymanager.json`.

#### Scenario: Switch to an installed version

- GIVEN version `3.14` is installed and the session shim directory exists
- WHEN the user runs `fpm use 3.14`
- THEN `PYTHON_MANAGER_DEFAULT` is set to `3.14` in the process environment
- AND the per-session `python.exe` shim resolves to the 3.14 runtime executable
- AND `pymanager.json` is unchanged

#### Scenario: Requested version not installed

- GIVEN version `9.9` is not installed
- WHEN the user runs `fpm use 9.9`
- THEN `fpm` exits non-zero
- AND stderr reports that version `9.9` is not installed
- AND no shim is rewritten

#### Scenario: Ambiguous version tag

- GIVEN multiple runtimes match the tag `3.13` (e.g. 64 and arm64)
- WHEN the user runs `fpm use 3.13`
- THEN `fpm` resolves via `py list --one`, picking the PyManager-preferred match
- AND the shim points at that single resolved executable

### Requirement: fpm use Without Args Resolves a Version File

`fpm use` with no version argument SHALL resolve a version via
`version-file-resolution` and behave as `fpm use <resolved>`. If no version file
is found, it SHALL exit non-zero with a clear error.

#### Scenario: .python-version present in cwd

- GIVEN cwd contains `.python-version` with content `3.13`
- WHEN the user runs `fpm use`
- THEN the effect is equivalent to `fpm use 3.13`

#### Scenario: No version file anywhere upward

- GIVEN no `.python-version` or `pyproject.toml` exists from cwd up to the root
- WHEN the user runs `fpm use`
- THEN `fpm` exits non-zero
- AND stderr reports no version file was found

### Requirement: --silent-if-unchanged Flag

`fpm use --silent-if-unchanged <version>` SHALL produce no stdout when the
selected version is already active. It SHALL still ensure the shim and
`PYTHON_MANAGER_DEFAULT` are correct.

#### Scenario: Same version already active

- GIVEN the shim already points at 3.14 and `PYTHON_MANAGER_DEFAULT=3.14`
- WHEN the user runs `fpm use --silent-if-unchanged 3.14`
- THEN stdout is empty
- AND the exit code is 0

### Requirement: Session-Only Effect

`fpm use` SHALL NOT modify `pymanager.json`, the registry, or any persistent
config. Only the per-session shim directory and the current process environment
are affected.

#### Scenario: Persistence untouched

- GIVEN `pymanager.json` has `default_tag: "3.12"`
- WHEN the user runs `fpm use 3.14`
- THEN `pymanager.json` still has `default_tag: "3.12"`