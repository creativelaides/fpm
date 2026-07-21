# Implementation Progress

**Change**: cli-friendly-refactor
**Mode**: Strict TDD

### TDD Cycle Evidence
| Task | RED (Tests Fail) | GREEN (Tests Pass) | REFACTOR |
|------|------------------|--------------------|----------|
| 2.1  | Added failing tests | Tests implemented and fixed | N/A |
| 2.2  | Wrote `test_fetch_versions` | Implemented `fetch_from_network` | Added cache path logic |
| 2.3  | Wrote cache ttl tests | Implemented `read_cache` / `write_cache` | Used `etcetera` correctly |
| 2.4  | Added offline fallback test | Offline cache fallback added | N/A |

### Completed Tasks
- [x] 2.1 Write failing unit tests in `src/services/remote.rs` for remote version fetching and caching logic.
- [x] 2.2 Implement `src/services/remote.rs` to fetch Python versions from python.org using `ureq`.
- [x] 2.3 Implement caching in `src/services/remote.rs` using `etcetera` for the cache directory and a 24-hour TTL.
- [x] 2.4 Implement offline fallback in `src/services/remote.rs` to return cached versions with a warning if the network request fails.

### Files Changed
| File | Action | What Was Done |
|------|--------|---------------|
| `src/error.rs` | Modified | Added `NetworkError` and `CacheError` with exit codes 7 and 8. Updated tests. |
| `src/services/remote.rs` | Created | Implemented `RemoteFetcher` trait and `DefaultRemoteFetcher` using `ureq` and `etcetera`. Added tests. |
| `src/services/mod.rs` | Modified | Added `pub mod remote;` |
| `openspec/changes/cli-friendly-refactor/tasks.md` | Modified | Checked off tasks 2.1-2.4 |

### Deviations from Design
The design trait for `RemoteFetcher` did not include a way to return whether the response was from the cache during an offline fallback inside `fetch_versions`. I have kept the trait signature returning `Result<Vec<RemoteVersion>, FpmError>` and implemented the offline fallback directly in `fetch_versions`. The controller or UI formatter may need to deduce the `from_cache` status (e.g., by checking if network failed or comparing with the cache TTL directly), or we will update the trait in the next phase.

### Issues Found
None.

### Workload / PR Boundary
- Mode: stacked PR slice
- Current work unit: Unit 2 ("Remote Caching & Offline Fallback")
- Boundary: Phase 2, Tasks 2.1 - 2.4
- Estimated review budget impact: Small (added one new service file and updated error types)
