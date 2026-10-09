## ADDED Requirements

### Requirement: One worker contract across model vendors
Every worker adapter SHALL accept a task packet (stage, item, attempt id, schema path, worktree, references, budget, model and effort) and SHALL return a status, a schema-validated result, its artifacts as a worktree diff plus manifest, and usage (tokens, cost, wall time), so stages are vendor-agnostic.

#### Scenario: Same stage on either vendor
- **WHEN** the router assigns an authoring item to Claude in one run and to Codex in another
- **THEN** the driver consumes both results through the same contract without vendor-specific branches in the stage

#### Scenario: Driver tests without a model
- **WHEN** driver tests run
- **THEN** a fake worker replays canned results through the same contract

### Requirement: The Codex binary is resolved at runtime and always sandboxed
The Codex adapter SHALL locate `codex.exe` from the installed Codex package at runtime rather than a hardcoded path, and SHALL pass an explicit `workspace-write` sandbox and approval policy on every call; it MUST refuse to run if the effective sandbox would be `danger-full-access`.

#### Scenario: Store update moves the binary
- **WHEN** the Codex package version directory changes
- **THEN** the next run resolves the new path without configuration changes

#### Scenario: Global config is full access
- **WHEN** the user's Codex config defaults to `danger-full-access`
- **THEN** the adapter's explicit overrides still run the worker under `workspace-write`

### Requirement: Workers run in driver-managed, pinned worktrees
The driver SHALL create and pool worker worktrees itself at a pinned base commit, each with its own `CARGO_TARGET_DIR`, and SHALL NOT rely on either CLI's built-in worktree option.

#### Scenario: Base drift
- **WHEN** a pooled worktree's HEAD differs from the run's expected base before an item starts
- **THEN** the driver resets it to the base before dispatch

#### Scenario: Isolated builds
- **WHEN** two workers build the engine concurrently
- **THEN** they use different cargo target directories

### Requirement: Failures are retried by class
The adapters SHALL retry transient failures with backoff, SHALL retry a schema-invalid result once with the validation error, and on quota or credit exhaustion SHALL stop using that vendor for the rest of the run and report it to the router.

#### Scenario: Rate limited
- **WHEN** a call fails with a rate-limit error
- **THEN** it is retried with increasing delay up to the configured limit

#### Scenario: Credits exhausted
- **WHEN** a vendor reports exhausted credits or quota
- **THEN** no further calls go to that vendor in the run and its pending items are re-routed or paused

### Requirement: Usage is captured per attempt
Every worker call SHALL record tokens, cost in USD (from the CLI where it reports cost, otherwise from a configured price table) and wall time on its attempt.

#### Scenario: Vendor reports only tokens
- **WHEN** a Codex call returns token counts but no cost
- **THEN** the attempt's cost is computed from the configured price table and marked as derived
