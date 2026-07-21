<Design: cli-friendly-refactor>
## Technical Approach

Refactor the CLI into a "Clean Architecture Lite" pattern. We will separate concerns by keeping argument parsing in controllers (`src/cli.rs`, `src/commands/*.rs`), moving domain logic to a new `src/services/` module, and extracting output generation into a `src/ui/` module. Domain functions will return structured `Result<T, E>` types rather than using `println!` directly, allowing the UI module to format and render the data.

We will add a new `list-remote` command that fetches Python versions from python.org using an HTTP client (e.g., `ureq`), caches them locally in a JSON file managed via the `etcetera` crate, and provides a graceful fallback to the cache (with a warning) when offline. 

Finally, the `--version` option will be overridden to present a detailed string containing the `fpm` crate version, the PyManager version (parsed from `py --help`), and the active Python version (from `py --version`).

## Architecture Decisions

### Decision: Clean Architecture Split
**Choice**: Split application into Controllers (`commands`), Services (`services`), and Formatters (`ui`).
**Alternatives considered**: Traditional MVC or Hexagonal Architecture.
**Rationale**: Clean Architecture Lite provides enough separation of concerns to make the CLI testable and decoupled from `println!` side effects, without introducing the heavy abstractions of Hexagonal architecture.

### Decision: Caching Strategy for `list-remote`
**Choice**: Use a simple JSON file stored in the platform-native cache directory using `etcetera`, with a cache expiration of 24 hours. Pre-releases (alphas, betas, RCs) are filtered out of the output by default unless a `--pre` argument is passed.
**Alternatives considered**: SQLite, embedded document database, or no local caching.
**Rationale**: JSON is lightweight, avoids adding heavy database dependencies, and is perfectly sufficient for caching a simple list of versions. `etcetera` ensures correct cross-platform paths (like AppData on Windows). A 24-hour expiration avoids unnecessary network calls on multiple daily runs, while the `--pre` flag keeps the output clean and focused on stable versions by default.

### Decision: Overriding `--version` Behavior
**Choice**: Disable clap's default version flag and implement a custom `--version` boolean argument handled in `main.rs`.
**Alternatives considered**: Generate the version dynamically in `clap`'s `#[command(version = ...)]`.
**Rationale**: Computing the detailed version involves running `py` commands, which can be slow or fail. A custom flag handler allows for proper error handling and fallback behavior outside of clap's static definition.

## Data Flow

    User Input (CLI Args)
            │
            ▼
    Controller (src/commands/) 
            │
            ├─► Service (src/services/) ──► External (python.org / PyManager)
            │        │
            │        ▼ (Returns Result<DomainObject, Error>)
            ▼
    UI Formatter (src/ui/)
            │
            ▼
      Console (stdout/stderr)

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `Cargo.toml` | Modify | Add `ureq` for HTTP requests for `list-remote`. |
| `src/cli.rs` | Modify | Disable clap's default version, add custom `--version` flag, add `ListRemote` subcommand. |
| `src/main.rs` | Modify | Handle custom `--version` logic (execute `py --help` & `py --version`), route `ListRemote`. |
| `src/commands/*.rs` | Modify | Remove `println!` statements, delegate logic to `services/`, pass results to `ui/`. |
| `src/services/mod.rs` | Create | Root for domain services. |
| `src/services/remote.rs` | Create | Service for fetching/caching python.org versions using `ureq` and `etcetera`. |
| `src/services/pymanager.rs` | Create | Service encapsulating PyManager (`py.exe`) interactions. |
| `src/ui/mod.rs` | Create | UI root module. |
| `src/ui/formatters.rs` | Create | Functions for printing versions, errors, and detailed version info. |

## Interfaces / Contracts

```rust
// In src/services/remote.rs
pub struct RemoteVersion {
    pub version: String,
    pub release_date: Option<String>,
}

pub trait RemoteFetcher {
    fn fetch_versions(&self) -> Result<Vec<RemoteVersion>, error::FpmError>;
    fn get_cached_versions(&self) -> Result<Vec<RemoteVersion>, error::FpmError>;
}

// In src/ui/formatters.rs
pub fn print_remote_versions(versions: &[RemoteVersion], from_cache: bool) {
    // ...
}
pub fn print_detailed_version(fpm_ver: &str, pymanager_ver: &str, active_py_ver: &str) {
    // ...
}
```

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| Unit | Services | Mock external HTTP requests and PyManager CLI outputs to test domain logic and parsing. |
| Unit | UI Formatters | Verify output strings match expected formatting without printing side effects. |
| Integration | Commands | Test the full flow from controller to service to UI rendering. |
| E2E | `list-remote` offline | Simulate network failure to ensure cache fallback triggers correctly and prints the offline warning. |

## Migration / Rollout

No data migration required. The JSON cache will be generated on the first successful run of `list-remote`.

## Open Questions

None. (Pre-releases are filtered by default unless `--pre` is supplied; Cache expiration is set to 24 hours).
</Design: cli-friendly-refactor>
