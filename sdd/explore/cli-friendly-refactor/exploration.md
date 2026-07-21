## Exploration: cli-friendly-refactor

### Current State
`fpm` is a fast, friendly wrapper around PyManager (`py.exe`) designed with fnm-like UX. Currently, it supports `use`, `list`, `current`, `default`, `env`, and `install` commands. The current architecture employs a straightforward command-dispatch pattern: `src/main.rs` and `src/cli.rs` handle argument parsing via `clap` and delegate directly to handlers in `src/commands/*.rs`. Domain logic (like junction retargeting and session activation) is partly mixed with side-effects (environment variable mutations) and presentation logic (CLI printing) within the command handlers. While functional, it lacks some of `fnm`'s friendlier CLI behaviors (e.g., `alias`, `list-remote`).

### Affected Areas
- `src/commands/*` — Command modules tightly couple UI, side effects, and domain logic.
- `src/cli.rs` — Needs updates to route new commands (`alias`, `list-remote`).
- `src/pymanager.rs` — Needs extensions to query remote versions (if possible via PyManager or external APIs).
- `src/error.rs` — Needs refactoring to separate domain errors from UI/presentation errors.

### Approaches
1. **Domain-Driven Reorganization with UI Abstraction (Ports & Adapters)**
   - **Description**: Move core business rules into a pure `domain` module. Introduce traits for UI (`Logger`/`Printer`), FileSystem, and Environment to fully decouple side effects.
   - **Pros**: Highly testable, fully decouples presentation from domain logic, makes future additions like JSON output or TUI trivial.
   - **Cons**: High initial refactoring effort, adds boilerplate which may contradict the project's goal of being a simple wrapper.
   - **Effort**: High

2. **Clean Architecture "Lite" (Command/Service Separation)**
   - **Description**: Extract printing and formatting into a `ui` module, and move business logic from `commands/` into `services/`. Commands act merely as controllers that parse args, call a service, and pass the result to a UI formatter. Add new commands like `alias` and `list-remote`.
   - **Pros**: Easy to implement, clears up the coupling in `commands/*.rs`, keeps the codebase fast and simple without over-engineering.
   - **Cons**: Still relies somewhat on direct OS interactions during testing.
   - **Effort**: Medium

3. **Incremental Feature Addition without Refactor**
   - **Description**: Simply add `alias.rs` and `list_remote.rs` in the current `commands/` structure.
   - **Pros**: Fastest time to market.
   - **Cons**: Continues the pattern of mixing UI and domain logic; increases technical debt.
   - **Effort**: Low

### Recommendation
**Approach 2 (Clean Architecture "Lite")** combined with adding `fnm`-equivalent features (`alias`, `list-remote`). This strikes the optimal balance between improving developer/user experience and avoiding over-engineering. Extracting a `ui` module will allow for richer CLI output uniformly across commands, while isolating business logic makes adding features like aliases cleaner.

### Risks
- **PyManager Limitations**: Implementing `list-remote` might be difficult if `py.exe` does not expose available remote versions; we may need to directly query Python's release APIs or the Nuget feed.
- **Config Drift**: Implementing `alias` will likely require an `fpm`-specific config (or extending `pymanager.json`), which could diverge from PyManager's native behavior.
- **Performance**: Excessive abstraction could slightly impact startup time, though keeping it "Lite" mitigates this risk.

### Ready for Proposal
Yes — the architectural direction is clear and actionable.
