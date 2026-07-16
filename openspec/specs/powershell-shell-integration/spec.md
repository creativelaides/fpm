# powershell-shell-integration Specification

## Purpose

`fpm env --shell powershell` generates the PowerShell script the user evaluates
to integrate `fpm` into their shell: it exports `FPM_DIR`, prepends the
per-session shim directory to `PATH`, and optionally installs a `Set-Location`
hook for automatic `use-on-cd`. The apply phase does NOT auto-edit `$profile`; it
emits a documented install snippet.

## Requirements

### Requirement: fpm env Emits PowerShell Setup Script

`fpm env --shell powershell` SHALL create a per-session multishell directory
under `FPM_DIR/multishells/<session-id>/` and print to stdout a PowerShell
script that sets `$env:FPM_DIR`, prepends the multishell directory to
`$env:PATH`, and exports `FPM_MULTISHELL_PATH`.

#### Scenario: Fresh env invocation

- GIVEN `FPM_DIR` resolves to a writable directory
- WHEN the user runs `fpm env --shell powershell`
- THEN a unique `multishells/<session-id>/` directory is created
- AND stdout contains `$env:FPM_DIR = "..."`
- AND stdout contains `$env:PATH = "<multishell-dir>;$env:PATH"`
- AND stdout contains `$env:FPM_MULTISHELL_PATH = "<multishell-dir>"`
- AND the exit code is 0

#### Scenario: FPM_DIR not writable

- GIVEN `FPM_DIR` points to a read-only location
- WHEN the user runs `fpm env --shell powershell`
- THEN `fpm` exits non-zero
- AND stderr reports the multishell directory could not be created

### Requirement: --use-on-cd Adds Set-Location Hook

When invoked with `--use-on-cd`, `fpm env --shell powershell` SHALL additionally
emit a PowerShell `Set-Location` hook that, on every directory change, calls
`fpm use --silent-if-unchanged` when a `.python-version` file exists in the new
location (resolved upward per `version-file-resolution`).

#### Scenario: use-on-cd enabled

- GIVEN `fpm env --shell powershell --use-on-cd` is run
- WHEN the output is inspected
- THEN it contains a `Set-Location` function override
- AND the override invokes `fpm use --silent-if-unchanged` when a version file is found

### Requirement: Session ID Is Unique Per Shell

Each `fpm env --shell powershell` invocation SHALL produce a unique session id
for the multishell directory so concurrent shells do not collide.

#### Scenario: Two concurrent shells

- GIVEN two PowerShell sessions run `fpm env --shell powershell`
- WHEN each evaluates the output
- THEN each gets a distinct `FPM_MULTISHELL_PATH`
- AND neither overwrites the other's shim

### Requirement: Install Snippet Is Documented, Not Auto-Applied

The apply phase SHALL emit a documented PowerShell install snippet and the
relevant `$PROFILE` locations (PowerShell 6+ and Windows PowerShell 5). It SHALL
NOT mutate `$PROFILE` automatically.

#### Scenario: Install instructions output

- GIVEN the apply phase runs
- WHEN install instructions are emitted
- THEN they contain the snippet `fpm env --use-on-cd --shell powershell | Out-String | Invoke-Expression`
- AND they list the `$PROFILE` path for PowerShell 6+ and for Windows PowerShell 5
- AND no `$PROFILE` file is modified by `fpm` itself

### Requirement: Shim Directory Lifecycle

`fpm` SHALL document cleanup of leftover `multishells/<session-id>/` directories
and SHOULD remove the current session's directory on clean shell exit when
feasible. Stale directories from crashed shells SHALL NOT break other sessions.

#### Scenario: Stale session directory ignored

- GIVEN a stale `multishells/<dead-id>/` directory exists
- WHEN a new shell runs `fpm env --shell powershell`
- THEN the new shell gets a fresh unique directory
- AND the stale directory does not affect the new session's PATH