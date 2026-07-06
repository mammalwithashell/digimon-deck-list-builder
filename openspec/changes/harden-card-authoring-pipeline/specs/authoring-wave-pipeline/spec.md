# Spec: authoring-wave-pipeline

## ADDED Requirements

### Requirement: Worker deliverables are files plus a manifest
Card-authoring workers SHALL write their YAML spec and behavioral test files at canonical repo paths inside their isolated worktree and SHALL return a structured manifest — card id, verdict, file paths with content hashes, pasted `test result:` (or `cargo check`) evidence lines, notes, and gap citations — instead of embedding file bodies in structured output.

#### Scenario: Manifest-based merge
- **WHEN** a worker completes a card and the orchestrator merges the wave
- **THEN** the card's files are obtained from the worker's worktree via a base-relative git diff filtered to the manifest's paths, and no file content is parsed out of the structured-output payload

#### Scenario: Out-of-manifest changes are quarantined
- **WHEN** a worker's worktree diff contains paths not listed in its manifest
- **THEN** those paths are reported to the orchestrator and are not applied without explicit adjudication

### Requirement: Reviewers judge the worker's actual files
Wave reviewers SHALL read the worker's worktree (read-only) to judge the actual authored files, and a missing-file-on-the-main-tree observation SHALL NOT be a valid rejection reason.

#### Scenario: Review before merge
- **WHEN** a reviewer evaluates a card whose files exist only in the worker worktree
- **THEN** the review proceeds against those files and worktree cleanup is deferred until after review verdicts and merge

### Requirement: Wave merges run through the merge tool
Wave merges SHALL be performed by a single idempotent tool that applies the manifest diffs, verifies each card's YAML compiles into the pack, verifies test-module registration end-to-end (mod wiring present AND the module contributes at least one discovered test), records verdicts, runs the scoped behavioral filter, and emits a wave summary. Each verification failure MUST abort loudly rather than degrade.

#### Scenario: Dead test module is impossible
- **WHEN** a merged card's test file is not reachable from the test binary (missing `mod` wiring or zero discovered tests)
- **THEN** the merge tool fails the wave for that card instead of completing silently

#### Scenario: Idempotent re-run
- **WHEN** the merge tool is re-run over an already-merged wave
- **THEN** it makes no changes and reports the wave as already merged

### Requirement: Agent bases are pinned mechanically
Worker and fix-agent worktrees SHALL be verified against an expected base commit before work begins, via a shared sync script invoked in the agent preamble (and/or orchestrator-side verification at dispatch), replacing per-prompt ad-hoc reset instructions.

#### Scenario: Stale base is corrected or refused
- **WHEN** an agent's worktree HEAD does not match the expected base commit
- **THEN** the sync script hard-resets the throwaway branch to the expected commit or fails the agent fast, before any files are read or written

### Requirement: Transient agent failures retry in-script
Campaign workflow scripts SHALL wrap per-item agent calls with a bounded retry-with-backoff for transient API failures, and SHALL abort the run fast on sustained failure bursts (credit exhaustion) so a resume restarts cleanly.

#### Scenario: Rate-limit wave absorbed
- **WHEN** an implementer call fails with a transient API error
- **THEN** the script retries it after a backoff before recording AGENT-FAILED, without requiring a full workflow relaunch

### Requirement: Mechanical review fixes are auto-applied
Reviewers SHALL classify each issue as mechanical or structural; mechanical directives with a machine-checkable target SHALL be applied by the merge tool and re-verified by the scoped suite, while structural rejections dispatch fix agents.

#### Scenario: Single-key directive auto-applied
- **WHEN** a reviewer directive specifies an exact single-edit change (e.g. one field value flip) and the target text is present
- **THEN** the merge tool applies it, re-runs the card's suite, and records the fix without dispatching a fix agent

#### Scenario: Ambiguous directive falls through
- **WHEN** a mechanical directive's target cannot be matched exactly
- **THEN** the card routes to a fix agent instead of a best-effort edit

### Requirement: Review batching preserves per-card scrutiny
Simple cards (low clause count and short printed text) MAY share one reviewer call in groups of up to four; complex cards (DUALs, multi-clause Lv.5+, Options with alt-plays) MUST retain a dedicated per-card review. A batch reviewer MUST be able to escalate any card to a solo re-review.

#### Scenario: Complex card is never batch-reviewed
- **WHEN** a wave contains a DUAL or multi-clause boss card
- **THEN** that card receives a dedicated reviewer regardless of batch grouping
