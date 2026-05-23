# Archetype DSL Implementation: BG Imperial — Blocker Mitigation
Date: 2026-05-23
Total cards in pool: 5 (explicit `--cards` list — top blockers for the BG Imperial RL training pool)
Processed this run: 5
Pipeline: batch-implement-cards-rust-dsl

## Summary
- IMPLEMENTED: 5
- PARTIAL: 0
- AUDITED-OK: 0
- AUDITED-MISSING-TESTS: 0
- AUDITED-DRIFT: 0
- BLOCKED (engine): 0
- BLOCKED (dsl): 0
- BLOCKED (hybrid): 0
- SKIPPED (prior verdict): 0

## Pool-impact result
| Archetype     | Before | After  |
|---------------|--------|--------|
| BG Imperial   | 8 / 38 | 26 / 38 |
| Puppets       | 34 / 34 | 34 / 34 |
| DNA Omnimon   | 66 / 66 | 66 / 66 |
| Medusamon     | 110 / 111 | 110 / 111 |

**BG Imperial: 8 → 26 decks (+18; target was 20+).**

## Per-Card Verdicts
| Card ID   | Name                | Mode      | Verdict     | Tests | Blocks (pre) | Notes |
|-----------|---------------------|-----------|-------------|-------|--------------|-------|
| LM-028    | Blue Scramble       | IMPLEMENT | IMPLEMENTED | 16/16 | 14/36        | Color-swap of LM-029 (yellow → blue). |
| EX7-023   | Hexeblaumon         | IMPLEMENT | IMPLEMENTED | 17/17 | 13/36        | Iceclad sourced from cards.json effect_text auto-parser. Flood-gate CannotSuspend comparator works against carrier's own divi count. |
| P-118     | Wormmon             | IMPLEMENT | IMPLEMENTED | 15/15 |  9/36        | Two-bucket reveal with `self_color_count_gte: 2` + `name_contains "Ken Ichijoji"`. Inherited EoT DNA digivolve via `alt_path_registration`. |
| LM-036    | Jade Memory Boost!  | IMPLEMENT | IMPLEMENTED | 10/10 |  7/36        | Color-swap of the LM-034/035/037 Memory Boost! family (yellow/purple → green/blue). |
| BT16-002  | DemiVeemon          | IMPLEMENT | IMPLEMENTED | 13/13 |  6/36        | Inherited declarative DP aura with `while_condition: self_color_count_gte: 2`. |

Total new tests: **71** across 5 card files. All passing.

## Engine-Gap Blocked Cards
None.

## DSL-Vocab-Gap Blocked Cards
None. One informational gap was *catalogued* during EX7-023 implementation but did not block — see "Notable findings."

## Notable findings

### EX7-023 — `Keyword::Iceclad` not exposed via `kind: grant_keyword`
- `Keyword::Iceclad` is supported in the engine combat math (`Phase F §F2`, RULES_CONTEXT 16-34) and is parsed automatically from `cards.json` `effect_text` via `parse_printed_keywords` and consumed by `combat::resolve_battle` via `has_keyword`.
- The DSL `grant_keyword` lookup (`lookup_keyword`) does not include an Iceclad arm, so authors cannot grant Iceclad via a declarative clause. **This did not block EX7-023** — the auto-parse from printed text is sufficient. Catalogued informally in the EX7-023 YAML header.
- If a future card grants Iceclad via an effect (e.g., "this Digimon and that one gain Iceclad until end of turn"), the DSL gap would need to be closed.

### P-118 — bucket negative-test mask vs hand-state semantics
- The agent's initial negative test asserted that a non-matching card's reveal-pick action was absent from `valid_action_ids`. Test failed because the action mask still surfaces all reveal slots when a bucket has no candidates (the engine auto-skips at commit rather than masking at selection time).
- Test was relaxed to check the behavioral truth (hand contents after `auto_resolve`): a single-color green Digimon does NOT end up in hand. All 15 P-118 tests now pass.
- Future bucket negative tests should follow this pattern: assert hand/zone state after `auto_resolve`, not action-mask shape.

## New Patterns Worth Documenting
- **`while_condition` vs `active_when` for inherited auras** (BT16-002): `while_condition` evaluates with `PredicateSubject::Permanent` (carrier-aware) — required for `self_color_count_gte` predicates. `active_when` evaluates with `PredicateSubject::None` → always false for color-count predicates. Existing exemplar: `BT12-031.yaml`.
- **Bucket negative tests should target hand-state, not action-mask** (P-118): documented in this artifact.
