# Delta for python-version-switching

## ADDED Requirements

### Requirement: Session Activation Effects Are Reusable

The session-activation effects produced by `fpm use` (per-session shim
retarget and setting `PYTHON_MANAGER_DEFAULT` for the current process) MAY
be produced by other `fpm` commands that require immediate session effect,
without changing `fpm use` semantics. `fpm use` SHALL remain session-only and
SHALL NOT write `pymanager.json`. Any command reusing these effects SHALL
obtain the session shim directory from `FPM_MULTISHELL_PATH` using the same
error path as `fpm use`.

#### Scenario: fpm default reuses session activation

- GIVEN `3.14-64` is installed and the session shim directory exists
- WHEN the user runs `fpm default 3.14`
- THEN the shim is retargeted to the 3.14 runtime
- AND `PYTHON_MANAGER_DEFAULT` is set to `3.14` for the current process
- AND the observable activation matches `fpm use 3.14`

#### Scenario: fpm use remains session-only when activation is shared

- GIVEN `pymanager.json` has `default_tag: "3.12"`
- WHEN the user runs `fpm use 3.14`
- THEN `pymanager.json` still has `default_tag: "3.12"`
- AND the shim and `PYTHON_MANAGER_DEFAULT` reflect `3.14` for the session only

#### Scenario: Reused activation fails without session shim directory

- GIVEN `FPM_MULTISHELL_PATH` is not set
- WHEN the user runs `fpm default 3.14`
- THEN `fpm` exits non-zero
- AND stderr reports the session shim directory is not available, with the same guidance as `fpm use`