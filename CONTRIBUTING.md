# Contributing to fpy

Thanks for your interest in contributing to fpy! This guide covers everything
you need to get started.

## Prerequisites

- **Rust 1.96.0** — the project pins the toolchain via `rust-toolchain.toml`, so
  rustup will install the right version automatically.
- **Node.js 22+** and **pnpm 9+** — used for the tooling stack (commit hooks,
  conventional commits, changesets).
- **Git** — with the `main` branch as the default.

## Getting started

```sh
# Clone the repository
git clone https://github.com/creativelaides/fpy.git
cd fpy

# Install Node.js dev dependencies (husky, commitlint, commitizen, changesets)
pnpm install

# Build the project
cargo build

# Run the test suite
cargo test
```

The first `cargo build` will pull dependencies and compile them. The first
`pnpm install` will set up husky git hooks automatically.

## Rust tooling

### Formatting

```sh
cargo fmt
```

The project uses `rustfmt.toml` with edition 2021 and `max_width = 100`.
Run `cargo fmt` before committing. The pre-commit hook runs `cargo fmt --check`
and will block the commit if formatting is off.

### Linting

```sh
cargo clippy -- -D warnings
```

Clippy runs with `-D warnings` — all lints are treated as errors. Fix them
before committing. The pre-commit hook enforces this.

### Tests

```sh
cargo test
```

All unit tests must pass. The pre-commit hook runs the full test suite.

## Committing changes

fpy uses [Conventional Commits](https://www.conventionalcommits.org/) enforced
by [commitlint](https://commitlint.js.org/). The commit message format is:

```
type(scope): summary
```

### Using commitizen (recommended)

For interactive conventional commit authoring:

```sh
pnpm cz
```

This walks you through selecting a type, scope, summary, and optional body.

### Manual commits

If you commit manually, the `commit-msg` hook validates your message against
the commitlint rules. Allowed types and scopes:

| Type     | When to use                                  |
| -------- | -------------------------------------------- |
| feat     | New feature                                  |
| fix      | Bug fix                                      |
| chore    | Tooling, maintenance, deps                  |
| docs     | Documentation                                |
| refactor | Code restructuring without behavior change  |
| test     | Adding or updating tests                     |
| ci       | CI/CD changes                                |
| perf     | Performance improvement                      |
| style    | Formatting, whitespace, semicolons          |
| build    | Build system or dependencies                 |
| revert   | Reverting a previous commit                 |

Allowed scopes: `error`, `config`, `pymanager`, `version-file`, `shim`,
`shell`, `cli`, `commands`, `ci`, `docs`, `openspec`.

Example:

```
feat(config): add FPM_DIR environment variable override
```

## Changesets

fpy uses [Changesets](https://github.com/changesets/changesets) to manage
versions and changelogs.

### Adding a changeset

Whenever you make a change that should be released, add a changeset:

```sh
pnpm changeset
```

This prompts you to:

1. Select the package (only `fpy` for now).
2. Choose the bump type (`major`, `minor`, or `patch`).
3. Write a human-readable summary (goes into the changelog).

The command creates a markdown file under `.changeset/`. Commit that file with
your code changes.

See [`.changeset/README.md`](.changeset/README.md) for more details.

## CI

GitHub Actions runs on every pull request and push to `main`:

| Job         | Command                          | Runner            |
| ----------- | -------------------------------- | ----------------- |
| Formatting   | `cargo fmt --check`              | `windows-latest`  |
| Clippy       | `cargo clippy -- -D warnings`     | `windows-latest`  |
| Unit tests   | `cargo test`                     | `windows-latest`  |

fpy is Windows-only in its first slice, so all CI jobs run on Windows.

### Release flow

When a changeset is merged into `main`, the **Release** workflow runs
`pnpm version:prepare`, which:

1. Consumes pending changesets and bumps `package.json`.
2. Syncs the version into `Cargo.toml` via `.ci/sync-cargo-version.js`.
3. Opens a "Version Packages" PR.

Merging that PR publishes the release.

## Git hooks (husky)

The project uses [husky](https://typicode.github.io/husky/) for git hooks:

- **pre-commit** — runs `cargo fmt --check`, `cargo clippy -- -D warnings`, and
  `cargo test`.
- **commit-msg** — runs commitlint to validate the commit message.

If you need to bypass hooks in an emergency, use `git commit --no-verify`, but
do this sparingly — CI will still enforce the same checks.

## Dependency updates

[Renovate](https://docs.renovatebot.com/) is configured to open PRs for
outdated dependencies. It runs weekly (Monday before 6 AM) and auto-merges
patch updates and dev dependencies after CI passes.