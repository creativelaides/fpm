## Intent
Refactor the `fpm` CLI architecture to separate UI, side effects, and domain logic ("Clean Architecture Lite") to improve maintainability and enable easier additions of fnm-like functionality.

## Scope

### In Scope
- Refactor all existing commands to the new "Clean Architecture Lite" structure.
- Implement a new `list-remote` command that fetches available Python versions directly from python.org.
- Implement local caching of remote version lists in a simple JSON file (resolved via `etcetera` cache directory) to speed up repeated queries and provide offline fallback.

### Out of Scope
- Implementing an `alias` command (omitted for now).
- Using SQLite for caching (avoiding extra dependency overhead).

## Capabilities

### New Capabilities
- `list-remote`: Fetch and display remote Python versions from python.org with local JSON caching and offline fallback.

### Modified Capabilities
- `core-architecture`: Complete refactor of all CLI commands to isolate domain logic from UI and side-effects.

## Approach
Adopt a "Clean Architecture Lite" pattern across the CLI. Handlers will parse input, invoke isolated domain functions, and pass results to UI formatters. The `list-remote` command will make direct HTTP requests to python.org's release listings, caching the result into a JSON file stored in the appropriate Windows cache directory using the `etcetera` library. Offline invocations will gracefully fall back to reading from this cache and warn the user.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `src/main.rs` | Modified | Update CLI command definitions and routing |
| `src/commands/*` | Modified | Refactor existing commands into domain logic and UI separation |
| `src/commands/list_remote.rs` | New | Implement new `list-remote` command logic |
| `src/cache.rs` | New | Implement JSON-based local caching using `etcetera` |
| `Cargo.toml` | Modified | Add HTTP client and `etcetera` if not already present |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Regressions in existing commands | Medium | Ensure existing integration or unit tests pass before merging |
| Unreliable API format | Low | Implement robust parsing and fallback to cache on errors |

## Rollback Plan
Revert the Git commit or branch out from the prior stable commit, as this is a full architectural refactor that affects all commands.

## Dependencies
- HTTP client library
- `etcetera` crate for cross-platform cache directory resolution
- `serde_json` for JSON caching

## Success Criteria
- [ ] All existing commands function identically to their pre-refactor state under the new architecture.
- [ ] `list-remote` outputs remote versions retrieved from python.org.
- [ ] Repeated `list-remote` calls are visibly faster due to JSON caching.
- [ ] Running `list-remote` offline successfully reads from cache and displays a warning.
