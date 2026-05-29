## Context

`Mastemon (Tribal)` resolves to 93 unique cards across 55 decklists. The resolved best deck is a compact 20-card implementation target, but only five of those best-deck cards currently have Rust YAML. The most important core cards without production YAML are `P-187`, `BT23-102`, `BT14-033`, `ST10-04`, `BT23-031`, `BT23-067`, `BT11-042`, `BT11-083`, `BT11-094`, and `EX6-074`. `EX6-029` exists, but it is currently a Blast DNA shell with the printed effect body stubbed.

Current Rust substrate is stronger than the older Mastemon notes imply. Union hand/trash selection, selected-security play, selected-security digivolve sources, `dna_origin`, formula-backed `trash_top_security`, and Blast DNA pending-selection flow exist. The remaining work should therefore be scoped around a few reusable missing shapes, then card YAML and behavioral tests.

## Goals / Non-Goals

**Goals:**

- Make the resolved Mastemon best deck faithful in Rust DSL.
- Add reusable DSL/engine substrate for owner-routed security placement and security placement/trash cost gates.
- Implement the Mastemon boss line first: `EX6-029`, `P-187`, and `BT23-102`.
- Implement the high-frequency support core after substrate lands.
- Keep every legal player decision surfaced through pending selections and action masks.
- Update gap trackers based on verified code, not stale tracker assumptions.

**Non-Goals:**

- No `ACTION_SPACE_SIZE` expansion.
- No observation tensor/profile changes.
- No Python legacy script authoring.
- No raw-Rust card-effect escapes for cards whose behavior is expressible in DSL.
- No requirement to complete every low-frequency tech card in the 93-card pool before the best-deck core is playable.

## Decisions

### Decision: Treat the resolver output as the source pool, but implement in priority bands

The resolver output in `qa/archetype-qa/mastemon-tribal/deck_pool.json` is the authoritative pool for coverage accounting. Implementation should start with the resolved best deck and cards appearing in roughly 40+ of 55 decklists, then continue through medium-frequency tech and finally low-frequency tech.

Alternative considered: implement all 93 cards in one undifferentiated batch. That would mix reusable blockers with low-impact tech cards and make it harder to verify the archetype incrementally.

### Decision: Add owner-routed security placement as a reusable primitive

Mastemon effects often select a Digimon or Tamer, then place it into its owner's security stack. Existing DSL placement takes a static `of: you` or `of: opponent`, which is not faithful for any-player target selections. Add an owner-routed variant instead of splitting every card into friendly and enemy branches.

Alternative considered: author separate `you` and `opponent` selection branches per card. That duplicates logic, complicates action-mask expectations, and is easy to get wrong when selected targets can cross owners.

### Decision: Model security placement/trash gates as result-aware costs

`P-187` and `ST10-14` require tails that happen only if a placement succeeds. `P-187` also uses top-security trash as a cost for its hand/trash play effect. These should become explicit result/cost patterns so card YAML can express "if you did" without relying on unconditional follow-up steps.

Alternative considered: place/trash first in the process body, then continue regardless. That would over-fire tails when the cost cannot be paid or the placement is prevented.

### Decision: Confirm selected-security digivolve with card-local tests before adding new substrate

The engine already resolves `CardSourceRef::Security` through `effect_initiated_digivolve_from_source`, and `select_security` can bind a chosen card. `BT14-033` should first be tested against this current path. Add new substrate only if that test proves a missing behavior.

Alternative considered: implement a new `digivolve_from_security_at` verb up front. Current code may already support the needed behavior, so preemptive vocabulary would risk unnecessary surface area.

### Decision: Keep Blast DNA action/tensor contracts stable

Blast DNA flow already uses pending selections and existing action ranges. Mastemon card unlock work should not expand action space or change the active observation tensor.

Alternative considered: add new action IDs for Mastemon-specific choices. That would force synchronized updates across action specs, tensor metadata, RL wrappers, frontend constants, and model compatibility for no apparent need.

## Risks / Trade-offs

- Owner-routed placement touches leave-field and security-add replacement paths. Mitigation: add focused engine/DSL tests with both friendly and opponent targets before using it in boss cards.
- Result-aware placement costs can accidentally hide choices. Mitigation: tests must inspect pending-selection masks for every legal target and decline path.
- Stale gap docs may misclassify current code. Mitigation: each task verifies the current implementation before filing or closing a gap.
- Completing only the best-deck core leaves some resolved pool cards unimplemented. Mitigation: the Mastemon coverage spec distinguishes best-deck readiness from full 93-card pool coverage.
- Security-stack effects are order-sensitive. Mitigation: tests must assert final security order, top/bottom placement, and security-loss trigger dispatch where relevant.
