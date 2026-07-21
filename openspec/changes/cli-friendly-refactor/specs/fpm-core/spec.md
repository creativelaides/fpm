<Delta for fpm-core>
## MODIFIED Requirements

### Requirement: Subcommand Routing

`fpm` SHALL recognize exactly the subcommands `use`, `list`, `current`,
`default`, `install`, `env`, and `list-remote`. Each SHALL dispatch to its dedicated
implementation. `--version` SHALL print the crate version; `--help` SHALL print
clap-generated help.
(Previously: fpm SHALL recognize exactly the subcommands use, list, current, default, install, and env.)

#### Scenario: Recognized subcommand dispatches

- GIVEN `fpm` is on PATH
- WHEN the user runs `fpm list`
- THEN the `list` implementation runs
- AND no other subcommand implementation is invoked

#### Scenario: --version prints detailed tool versions

- GIVEN `fpm` is installed
- WHEN the user runs `fpm --version`
- THEN stdout MUST contain `fpm <crate-version>`
- AND stdout MUST contain the installed Python Launcher / PyManager version
- AND stdout MUST contain the active Python version (if any)

#### Scenario: --help prints usage

- GIVEN `fpm` is on PATH
- WHEN the user runs `fpm --help`
- THEN stdout lists all recognized subcommands
- AND the exit code is 0
</Delta for fpm-core>
