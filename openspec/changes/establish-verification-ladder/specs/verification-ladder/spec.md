# Spec: verification-ladder

## ADDED Requirements

### Requirement: Tiered verification entrypoint
The repository SHALL provide a single verification entrypoint (`scripts/verify`) with four tiers: tier 0 static gates (lint/drift/citation checks), tier 1 fast semantic canaries (dsl binary, judge quiz, FAQ conformance, keyword matrix, parity guards, determinism guard), tier 2 change-scoped behavior (impact-scoped behavioral filter, golden-replay diff, invariant fuzz), and tier 3 full seal (full behavioral suite, archetype-static tests, dcgo-replay corpus). Each tier MUST complete within its budget class (tier 0 seconds; tier 1 ≈2 minutes; tier 2 ≈5 minutes) on the reference dev machine.

#### Scenario: One command per tier
- **WHEN** a developer or agent runs `scripts/verify --tier 1`
- **THEN** all tier-1 suites run with the correct environment (stack size, thread caps) and a single pass/fail summary is emitted

#### Scenario: Ladder guidance for engine changes
- **WHEN** an engine or DSL change is prepared for commit
- **THEN** the documented workflow requires tiers 0–2 green locally, with tier 3 reserved for pre-merge/nightly seals

### Requirement: Tiers 1 and 2 run in CI
Tier-1 and tier-2 suites SHALL run as CI gates on engine-affecting changes, ending the class of test binaries that run in no CI gate and rot on main.

#### Scenario: Side binary can no longer rot silently
- **WHEN** a change breaks a tier-1 binary (e.g. a parity guard or the judge quiz)
- **THEN** CI fails on the PR rather than the breakage being discovered weeks later by a local run
