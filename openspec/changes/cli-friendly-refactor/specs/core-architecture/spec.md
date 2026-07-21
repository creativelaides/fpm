<core-architecture Specification>
## Purpose

Defines the "Clean Architecture Lite" pattern for the CLI, isolating domain logic from UI formatting and side effects to improve testability and maintainability.

## Requirements

### Requirement: Architecture Separation

All CLI commands MUST separate input parsing, domain logic execution, and UI output formatting.

#### Scenario: Command Execution

- GIVEN a user invokes a CLI command
- WHEN the command executes
- THEN the handler parses the input
- AND invokes an isolated domain function without side effects
- AND passes the domain result to a UI formatter for output

### Requirement: UI Independence

Domain functions MUST NOT directly print to stdout or stderr.

#### Scenario: Domain Function Output

- GIVEN a domain function is executing
- WHEN it needs to communicate a result or error
- THEN it returns a result object or error type
- AND the UI layer is responsible for rendering it to the terminal
</core-architecture Specification>
