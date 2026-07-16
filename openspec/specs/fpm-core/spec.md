# fpm-core Specification

## Purpose

The `fpm` Rust CLI entry point and subcommand router. Recognizes a fixed set of
subcommands (`use`, `list`, `current`, `default`, `install`, `env`) and global
flags (`--version`, `--help`). Any unrecognized first token forwards verbatim to
`py.exe` so existing aliases keep working.

## Requirements

### Requirement: Subcommand Routing

`fpm` SHALL recognize exactly the subcommands `use`, `list`, `current`,
`default`, `install`, and `env`. Each SHALL dispatch to its dedicated
implementation. `--version` SHALL print the crate version; `--help` SHALL print
clap-generated help.

#### Scenario: Recognized subcommand dispatches

- GIVEN `fpm` is on PATH
- WHEN the user runs `fpm list`
- THEN the `list` implementation runs
- AND no other subcommand implementation is invoked

#### Scenario: --version prints crate version

- GIVEN `fpm` was built with `clap` `cargo` feature
- WHEN the user runs `fpm --version`
- THEN stdout contains `fpm <crate-version>`

#### Scenario: --help prints usage

- GIVEN `fpm` is on PATH
- WHEN the user runs `fpm --help`
- THEN stdout lists all recognized subcommands
- AND the exit code is 0

### Requirement: Unrecognized Args Pass Through to py.exe

Any `fpm <args>` whose first non-flag token is not a recognized subcommand SHALL
be forwarded verbatim to `py.exe`, inheriting stdout, stderr, and exit code. `fpm`
SHALL NOT alter, reorder, or inject arguments.

#### Scenario: Pass-through of raw Python invocation

- GIVEN `py.exe` is on PATH
- WHEN the user runs `fpm -3.13 -m markitdown file.md`
- THEN `py.exe` is invoked with args `["-3.13", "-m", "markitdown", "file.md"]`
- AND `py.exe`'s stdout/stderr stream to the terminal unchanged
- AND `fpm` exits with `py.exe`'s exit code

#### Scenario: Pass-through of unknown subcommand

- GIVEN `py.exe` is on PATH
- WHEN the user runs `fpm foobar --x`
- THEN `py.exe` is invoked with args `["foobar", "--x"]`
- AND `fpm` exits with `py.exe`'s exit code

#### Scenario: py.exe missing from PATH

- GIVEN `py.exe` is NOT on PATH
- WHEN the user runs `fpm script.py`
- THEN `fpm` exits with a non-zero code
- AND stderr reports that `py.exe` could not be found

### Requirement: Single py list Cache Per Invocation

A single `fpm` process invocation SHALL spawn `py list --format=json` at most
once and reuse the parsed result for any command needing the installed-runtimes
list.

#### Scenario: list then current share one spawn

- GIVEN `fpm list` internally needs the runtime list and version resolution
- WHEN the user runs `fpm list`
- THEN `py.exe list --format=json` is spawned at most once during that process

### Requirement: Exit Code Propagation

`fpm` SHALL exit 0 on success and non-zero on error. For pass-through and
delegated commands, `fpm` SHALL propagate the child process exit code.

#### Scenario: Delegated command failure

- GIVEN `fpm install <version>` delegates to `py install`
- WHEN `py install` exits with code 1
- THEN `fpm` exits with code 1