# version-file-resolution Specification

## Purpose

Resolve a Python version for `fpm use` (no args) by walking the directory tree
upward from cwd, reading `.python-version` first, then `pyproject.toml`'s
`requires.python-version` / `project.requires-python`.

## Requirements

### Requirement: Walk Upward From cwd

Version resolution SHALL start at cwd and walk parent directories up to the
filesystem root, checking each directory for a version file in a defined order.

#### Scenario: File in cwd wins

- GIVEN cwd contains `.python-version` with `3.13`
- AND a parent contains `pyproject.toml` with `requires-python`
- WHEN `fpm use` resolves a version
- THEN `3.13` is selected (cwd `.python-version` wins)

#### Scenario: File in ancestor

- GIVEN cwd has no version file
- AND cwd's grandparent contains `.python-version` with `3.12`
- WHEN `fpm use` resolves a version
- THEN `3.12` is selected

#### Scenario: No file upward to root

- GIVEN no `.python-version` or `pyproject.toml` exists from cwd to root
- WHEN `fpm use` resolves a version
- THEN resolution fails
- AND `fpm use` exits non-zero with a "no version file found" error

### Requirement: .python-version Format

A `.python-version` file SHALL be read as UTF-8 text. The version is the first
non-empty, non-comment line trimmed of whitespace. Lines beginning with `#` are
comments.

#### Scenario: Plain version line

- GIVEN `.python-version` contains `3.13\n`
- WHEN the file is parsed
- THEN the resolved version is `3.13`

#### Scenario: Comment and blank lines ignored

- GIVEN `.python-version` contains `# project python\n\n3.14\n`
- WHEN the file is parsed
- THEN the resolved version is `3.14`

#### Scenario: Empty file

- GIVEN `.python-version` exists but is empty
- WHEN resolution reads it
- THEN the file is treated as not declaring a version
- AND resolution continues to the next source/parent directory

### Requirement: pyproject.toml python-version Constraint

When no `.python-version` is found at a directory, `fpm` SHALL read
`pyproject.toml` in that directory and extract the Python version from
`requires.python-version` or `project.requires-python`. Specifiers like `>=3.12`,
`~3.13`, or `==3.14.*` SHALL be reduced to the highest installed runtime matching
the specifier (queried via the cached `py list`).

#### Scenario: Lower-bound specifier

- GIVEN `pyproject.toml` has `requires-python = ">=3.12"`
- AND installed runtimes are `3.11`, `3.13`, `3.14`
- WHEN `fpm use` resolves via `pyproject.toml`
- THEN `3.14` is selected (highest installed satisfying `>=3.12`)

#### Scenario: Pinned specifier

- GIVEN `pyproject.toml` has `requires-python = "==3.13.*"`
- AND `3.13-64` is installed
- WHEN `fpm use` resolves
- THEN `3.13` is selected

#### Scenario: No installed runtime satisfies specifier

- GIVEN `pyproject.toml` has `requires-python = ">=3.20"`
- AND no runtime `>=3.20` is installed
- WHEN `fpm use` resolves
- THEN `fpm` exits non-zero
- AND stderr reports no installed runtime satisfies the constraint

#### Scenario: Malformed pyproject.toml

- GIVEN `pyproject.toml` is not valid TOML
- WHEN resolution tries to read it
- THEN `fpm` skips that file without crashing
- AND resolution continues upward

### Requirement: .python-version Takes Precedence Over pyproject.toml

Within the same directory, if both `.python-version` and `pyproject.toml` exist,
`.python-version` SHALL win. `pyproject.toml` is only consulted when
`.python-version` is absent or empty.

#### Scenario: Both files in same directory

- GIVEN a directory has `.python-version`=`3.13` and `pyproject.toml`=`requires-python=">=3.14"`
- WHEN `fpm use` resolves at that directory
- THEN `3.13` is selected