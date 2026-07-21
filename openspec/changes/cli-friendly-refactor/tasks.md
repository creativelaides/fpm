## Review Workload Forecast

Decision needed before apply: No
Chained PRs recommended: Yes
Chain strategy: stacked-to-main
400-line budget risk: High

### Suggested Work Units

| Unit | Goal | Likely PR | Notes |
|------|------|-----------|-------|
| 1 | Foundation & CLI flags | PR 1 | Add dependencies, setup `src/services/` & `src/ui/`, custom `--version` in `main.rs` & `cli.rs`. |
| 2 | Service Layer & `list-remote` | PR 2 | Implement `src/services/remote.rs` (ureq, etcetera) & `list_remote` command. |
| 3 | Core Refactor | PR 3 | Refactor existing commands (`list`, `use`, `current`, `default`, `env`, `install`) to Clean Arch Lite (Services + UI Formatters). |
| 4 | Testing & Cleanup | PR 4 | Add unit & E2E tests, clean up unused imports. |

## Phase 1: Foundation / Infrastructure

- [x] 1.1 Update `Cargo.toml` to add `ureq` and `etcetera` dependencies.
- [x] 1.2 Create `src/services/mod.rs` and `src/ui/mod.rs` to establish the new architecture modules.
- [x] 1.3 Create `src/ui/formatters.rs` with basic error and output formatting functions (e.g. `print_detailed_version`).
- [x] 1.4 Update `src/cli.rs` to disable clap's default version and define a custom `--version` boolean flag. Add `ListRemote` subcommand.
- [x] 1.5 Update `src/main.rs` to handle the custom `--version` flag, executing `py --help` and `py --version` to build the detailed version string, and printing it via `ui/formatters.rs`.

## Phase 2: Core / Services

- [x] 2.1 Write failing unit tests in `src/services/remote.rs` for remote version fetching and caching logic.
- [x] 2.2 Implement `src/services/remote.rs` to fetch Python versions from python.org using `ureq`.
- [x] 2.3 Implement caching in `src/services/remote.rs` using `etcetera` for the cache directory and a 24-hour TTL.
- [x] 2.4 Implement offline fallback in `src/services/remote.rs` to return cached versions with a warning if the network request fails.
- [x] 2.5 Refactor `src/pymanager.rs` into `src/services/pymanager.rs` to serve as the core service for local Python management.

## Phase 3: CLI Controllers / UI wiring

- [x] 3.1 Write failing tests in `src/ui/formatters.rs` for `print_remote_versions` and existing command outputs.
- [x] 3.2 Create `src/commands/list_remote.rs` to wire the `list-remote` command, calling `services/remote.rs` and rendering via `ui/formatters.rs`.
- [x] 3.3 Refactor `src/commands/list.rs` to return `Result<DomainType, Error>` instead of using `println!`, and render output via `src/ui/formatters.rs`.
- [x] 3.4 Refactor `src/commands/current.rs` and `src/commands/default.rs` to use Clean Architecture Lite.
- [x] 3.5 Refactor `src/commands/use_cmd.rs` and `src/commands/env_cmd.rs` to use Clean Architecture Lite.
- [x] 3.6 Refactor `src/commands/install.rs` and `src/commands/passthrough.rs` to use Clean Architecture Lite.

## Phase 4: Testing / Verification

- [x] 4.1 Write unit tests for the UI Formatters in `src/ui/formatters.rs` to ensure strings format correctly without side effects.
- [x] 4.2 Write integration tests for `list-remote` offline cache fallback behavior.
- [x] 4.3 Write integration tests for the `--version` override output.

## Phase 5: Cleanup / Documentation

- [x] 5.1 Remove all lingering `println!` and `eprintln!` calls from `src/commands/*.rs`.
- [x] 5.2 Update `README.md` to document the new `list-remote` command and custom `--version` output.
