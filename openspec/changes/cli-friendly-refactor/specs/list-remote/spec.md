<list-remote Specification>
## Purpose

Fetches and displays remote Python versions from python.org with local JSON caching and offline fallback.

## Requirements

### Requirement: Fetch Remote Versions

The system SHALL fetch the list of available Python versions from python.org when the cache is empty or expired.

#### Scenario: Successful fetch

- GIVEN the cache is empty
- AND the system is online
- WHEN the user runs `list-remote`
- THEN the system fetches the list from python.org
- AND the system displays the available versions

### Requirement: Local Caching

The system MUST cache the fetched version list in a JSON file using the `etcetera` cache directory resolution.

#### Scenario: Cache hit

- GIVEN a valid cache file exists
- WHEN the user runs `list-remote`
- THEN the system reads the versions from the cache
- AND the system displays the versions without making an HTTP request

### Requirement: Offline Fallback

The system SHALL use the cached version list if an HTTP request fails, and MUST warn the user about the offline state.

#### Scenario: Offline with cache

- GIVEN a valid cache file exists
- AND the system is offline
- WHEN the user runs `list-remote`
- THEN the system displays the cached versions
- AND a warning message is shown indicating the offline fallback

#### Scenario: Offline without cache

- GIVEN no cache file exists
- AND the system is offline
- WHEN the user runs `list-remote`
- THEN the system fails gracefully with an error indicating no internet and no cache
</list-remote Specification>
