# Spec: impact-scoped-testing

## ADDED Requirements

### Requirement: Diff-to-filter impact map
The repository SHALL provide a tool that, given a git diff, computes the affected behavioral-test scope: card YAMLs are indexed by the DSL verbs/predicates/timings they use, engine lowering files are mapped to the verbs they lower, and the tool emits the `cards_behavioral` name-filter plus the list of affected side test binaries.

#### Scenario: Lowering change scopes to consumers
- **WHEN** a change touches one DSL step's lowering arm
- **THEN** the tool emits a filter containing exactly the cards whose YAML uses that step (plus its DSL-level test file), and the scoped run completes in minutes instead of the full-suite hour

#### Scenario: Unmapped core files escalate honestly
- **WHEN** a change touches core engine files with no verb mapping (e.g. combat or turn machinery)
- **THEN** the tool answers "full suite required" (tier 3) rather than emitting an under-scoped filter

### Requirement: The map cannot drift silently
The engine-file→verb mapping SHALL be guarded by a coverage check (in the spirit of the existing eval-arm coverage test) so that adding a new DSL verb or lowering arm without a map entry fails a tier-1 check.

#### Scenario: New verb forces a map entry
- **WHEN** a new DSL step is added with its lowering
- **THEN** the map-coverage check fails until the impact map knows the new verb's lowering site
