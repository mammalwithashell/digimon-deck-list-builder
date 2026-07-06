# Spec: qa-tracker-hygiene

## ADDED Requirements

### Requirement: Gap trackers are single-copy
`qa/dsl-vocab-gaps.md` SHALL contain exactly one copy of its content (the current file holds two full duplicated copies that agents update in parallel), and a lightweight check SHALL prevent re-duplication.

#### Scenario: Dedupe lands with a guard
- **WHEN** the tracker is deduplicated
- **THEN** a check (script or test) fails if the file's section headings ever appear twice, so the duplication cannot silently return

### Requirement: Trackers are orchestrator-write-only during waves
During authoring waves, worker and reviewer agents SHALL report gap entries and verdicts through their structured outputs/manifests; only the orchestrator (or its merge tool) writes `qa/dsl-vocab-gaps.md`, `docs/RUST_ENGINE_GAPS.md`, and `qa/qa-reports/validated_cards_dsl.json`.

#### Scenario: Worker reports, orchestrator writes
- **WHEN** a worker discovers a new vocabulary gap mid-implementation
- **THEN** the gap arrives in the worker's manifest and the orchestrator's merge step appends the tracker entry exactly once, avoiding cross-agent merge conflicts on the shared files
