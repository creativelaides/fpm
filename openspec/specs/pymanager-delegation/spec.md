# pymanager-delegation Specification

## Purpose

The abstraction over the official PyManager (`py`/`pymanager`): parsing
`py list --format=json`, reading and writing `%AppData%\Python\pymanager.json`,
and PEP 514 registry reads for install-path resolution. `fpm` delegates all
runtime management to `py`; it does not download Python itself.

## Requirements

### Requirement: Parse py list --format=json

`fpm` SHALL obtain the installed-runtimes list by spawning
`py list --format=json` and parsing the JSON into a runtime collection. The
parsed result SHALL be cached for the lifetime of the process.

#### Scenario: Successful parse

- GIVEN `py list --format=json` returns valid JSON with runtimes
- WHEN `fpm` needs the runtime list
- THEN the JSON is parsed exactly once per process
- AND runtimes are available for `list`, `use`, and `current`

#### Scenario: py list returns malformed JSON

- GIVEN `py list --format=json` returns invalid JSON
- WHEN `fpm` parses it
- THEN `fpm` exits non-zero
- AND stderr reports the parse failure

### Requirement: Resolve Single Runtime via py list --one

To resolve one runtime for a tag, `fpm` SHALL call
`py list --one --format=exe <tag>` and use the returned executable path. It SHALL
NOT reimplement tag-matching itself.

#### Scenario: Resolve installed tag

- GIVEN `3.14-64` is installed
- WHEN `fpm use 3.14` resolves the runtime
- THEN `py list --one --format=exe 3.14` is the source of the executable path
- AND `fpm` does not compute the path from the registry alone

#### Scenario: Tag resolves to nothing

- GIVEN no runtime matches `9.9`
- WHEN `fpm` calls `py list --one --format=exe 9.9`
- THEN `py` returns no executable
- AND `fpm` surfaces a "version not installed" error

### Requirement: fpm install Delegates to py install

`fpm install <version>` SHALL spawn `py install <version>` and stream its
stdout/stderr to the terminal. `fpm` SHALL NOT implement its own download logic.
The child's exit code SHALL become `fpm`'s exit code.

#### Scenario: Successful install

- GIVEN `3.13` is not yet installed
- WHEN the user runs `fpm install 3.13`
- THEN `py install 3.13` is spawned
- AND its output streams through to the terminal
- AND `fpm` exits with `py install`'s exit code

### Requirement: fpm default Reads, Writes, and Activates pymanager.json

`fpm default` with no argument SHALL read and print `default_tag` from
`%AppData%\Python\pymanager.json`. `fpm default <version>` SHALL validate the
tag is installed, write `default_tag` to that file (preserving all other
keys), retarget the per-session shim to the resolved runtime, and set
`PYTHON_MANAGER_DEFAULT` for the current process. `fpm` SHALL NOT touch
`pymanager.json` from `fpm use`.

#### Scenario: Read current default

- GIVEN `pymanager.json` contains `"default_tag": "3.13"`
- WHEN the user runs `fpm default`
- THEN stdout contains `3.13`

#### Scenario: Read when no default configured

- GIVEN `pymanager.json` has no `default_tag` key (or is absent)
- WHEN the user runs `fpm default`
- THEN stdout reports no default is configured
- AND the exit code is 0

#### Scenario: Write new default and activate session

- GIVEN `3.14-64` is installed, the session shim directory exists, and `pymanager.json` has other keys
- WHEN the user runs `fpm default 3.14`
- THEN `default_tag` becomes `3.14` in `pymanager.json`
- AND all other keys in `pymanager.json` are preserved
- AND the per-session shim resolves to the 3.14 runtime executable
- AND `PYTHON_MANAGER_DEFAULT` is set to `3.14` for the current process
- AND stdout reports the default was set and the session activated

#### Scenario: pymanager.json missing

- GIVEN `%AppData%\Python\pymanager.json` does not exist and `3.14-64` is installed
- WHEN the user runs `fpm default 3.14`
- THEN `fpm` creates the file with `default_tag: "3.14"`
- AND the shim is retargeted and `PYTHON_MANAGER_DEFAULT` is set
- AND the exit code is 0

#### Scenario: FPM_MULTISHELL_PATH not set

- GIVEN `3.14-64` is installed but `FPM_MULTISHELL_PATH` is not set
- WHEN the user runs `fpm default 3.14`
- THEN `fpm` exits non-zero
- AND stderr reports the session shim directory is not available
- AND `pymanager.json` is not written

#### Scenario: Tag not installed

- GIVEN `9.9` is not installed
- WHEN the user runs `fpm default 9.9`
- THEN `fpm` exits non-zero
- AND stderr reports `9.9` is not installed
- AND `pymanager.json` and the shim are unchanged

#### Scenario: fpm current reports active version

- GIVEN the active runtime is determined by `PYTHON_MANAGER_DEFAULT` or `default_tag`
- WHEN the user runs `fpm current`
- THEN stdout shows the active version

### Requirement: fpm default --unset Removes default_tag

`fpm default --unset` SHALL remove the `default_tag` key from
`%AppData%\Python\pymanager.json`, preserving all other keys. It SHALL NOT
retarget the session shim or modify `PYTHON_MANAGER_DEFAULT`. If the file or
key is absent, `fpm` SHALL exit 0 and report that no default was configured.

#### Scenario: Unset existing default

- GIVEN `pymanager.json` contains `"default_tag": "3.13"` and `"install_dir": "C:\\py"`
- WHEN the user runs `fpm default --unset`
- THEN `default_tag` is removed from `pymanager.json`
- AND `install_dir` is preserved
- AND the exit code is 0

#### Scenario: Unset when no default configured

- GIVEN `pymanager.json` exists without a `default_tag` key
- WHEN the user runs `fpm default --unset`
- THEN `fpm` exits 0
- AND stdout reports no default was configured
- AND `pymanager.json` is otherwise unchanged

#### Scenario: Unset when pymanager.json missing

- GIVEN `%AppData%\Python\pymanager.json` does not exist
- WHEN the user runs `fpm default --unset`
- THEN `fpm` exits 0
- AND stdout reports no default was configured
- AND no file is created

### Requirement: fpm default <tag> --dry-run Previews Without Side Effects

`fpm default <tag> --dry-run` SHALL print the tag that would be written to
`default_tag` and SHALL NOT write `pymanager.json`, retarget the shim, or set
`PYTHON_MANAGER_DEFAULT`. The tag SHALL be validated via `resolve_exe` before
the preview is printed.

#### Scenario: Dry-run for an installed tag

- GIVEN `3.14-64` is installed and `pymanager.json` has `default_tag: "3.13"`
- WHEN the user runs `fpm default 3.14 --dry-run`
- THEN stdout reports `3.14` as the would-be default
- AND `pymanager.json` still has `default_tag: "3.13"`
- AND the shim is not retargeted
- AND `PYTHON_MANAGER_DEFAULT` is unchanged

#### Scenario: Dry-run for a tag not installed

- GIVEN `9.9` is not installed
- WHEN the user runs `fpm default 9.9 --dry-run`
- THEN `fpm` exits non-zero
- AND stderr reports `9.9` is not installed
- AND no preview is printed

### Requirement: fpm default <tag> Validates Tag Is Installed

`fpm default <tag>` SHALL validate that `<tag>` resolves to an installed
runtime via `resolve_exe` (equivalent to `fpm use`) before writing
`pymanager.json` or retargeting the shim. If the tag does not resolve, `fpm`
SHALL exit non-zero with a "version not installed" error and SHALL NOT modify
`pymanager.json`, the shim, or `PYTHON_MANAGER_DEFAULT`.

#### Scenario: Reject uninstalled tag before any side effect

- GIVEN `9.9` is not installed and `pymanager.json` has `default_tag: "3.13"`
- WHEN the user runs `fpm default 9.9`
- THEN `fpm` exits non-zero
- AND stderr reports `9.9` is not installed
- AND `pymanager.json` still has `default_tag: "3.13"`
- AND the shim is not retargeted

### Requirement: PEP 514 Registry Reads for Install Path

`fpm` MAY read `HKEY_CURRENT_USER\Software\Python\<Company>\<Tag>\InstallPath`
(PEP 514) as a fallback to resolve an install path when `py list --format=exe`
is unavailable. The registry read SHALL be read-only.

#### Scenario: Registry fallback

- GIVEN `py list --one --format=exe` is unavailable but the runtime is registered
- WHEN `fpm` resolves the install path
- THEN `fpm` reads the `InstallPath` value from PEP 514 registry
- AND no registry value is written

### Requirement: PyManager 26.x+ Assumption

`fpm` SHALL assume PyManager 26.x+ is installed (`py.exe`/`pymanager.exe` on
PATH). Older legacy launchers are out of scope and need not be supported.

#### Scenario: py.exe missing

- GIVEN `py.exe` is not on PATH
- WHEN any `fpm` command requiring PyManager runs
- THEN `fpm` exits non-zero with a message stating PyManager 26.x+ is required