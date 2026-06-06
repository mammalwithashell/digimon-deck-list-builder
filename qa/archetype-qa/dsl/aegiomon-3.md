# Archetype DSL Implementation: Aegiomon (slice aegiomon-3)
Date: 2026-06-06
Total cards in slice: 5
Processed this run: 5
Pipeline: batch-implement-cards-rust-dsl

## Summary
- IMPLEMENTED: 2
- PARTIAL: 0
- AUDITED-OK: 0
- AUDITED-MISSING-TESTS: 0
- AUDITED-DRIFT: 0
- BLOCKED (engine): 2
- BLOCKED (dsl): 1
- BLOCKED (hybrid): 0
- SKIPPED (prior verdict): 0

## Per-Card Verdicts
| Card ID | Name | Mode | Verdict | Review | Tests | Notes |
|---------|------|------|---------|--------|-------|-------|
| BT25-103 | GraceNovamon | IMPLEMENT | BLOCKED (dsl) | n/a | 0 | Counter clause needs dynamic-count cross-permanent player-choice digivolution-source trash (G-DSL-SELECT-OPP-SOURCES-DYNAMIC-CROSS-PERMANENT). Clauses 1–6 expressible. |
| BT25-094 | Cosmic Area | IMPLEMENT | IMPLEMENTED | self | 5/5 | Area option; floodgate color-ignore; security aura grant Alliance + conditional Rush (Apollomon/Dianamon); Main replace-bottom-sec + play red/blue TS -3; inherited security play lvl4- free. Mirror of BT25-095. |
| BT25-097 | Guardian Palace | IMPLEMENT | BLOCKED (engine) | n/a | 0 | Alliance primary OK; conditional <Scapegoat> (Junomon) aura-grant to target set not behaviorally installed (G-ENGINE-AURA-GRANT-REPLACEMENT-KEYWORD). |
| BT25-099 | Gear Forest Village | IMPLEMENT | IMPLEMENTED | self | 5/5 | Area option; floodgate color-ignore; security aura grant Alliance + conditional Piercing (Bacchusmon/Ceresmon); Main replace-bottom-sec + play green/black TS -3; inherited security play lvl4- free. |
| BT25-102 | Factorial Area | IMPLEMENT | BLOCKED (engine) | n/a | 0 | Blocker primary OK; conditional <Link +1> (Vulcanusmon) aura-grant not expressible — aura modifier value hardcoded to 0; no grantable Link keyword (G-ENGINE-AURA-GRANT-LINK-MAX). |

## Engine-Gap Blocked Cards
### BT25-097 Guardian Palace
- Effect text: "[Security] [All Turns] All of your yellow or purple [TS] trait Digimon gain ＜Alliance＞. While you have a Digimon with [Junomon] in its name, they also gain ＜Scapegoat＞."
- Missing engine API: aura-granted replacement-type keywords (Scapegoat) do not install their `when_would_be_deleted` auto-effect on the target set; `Game::effects_for_card` only synthesizes the source card's own registry grant and skips conditional grants.
- Gap tag: G-ENGINE-AURA-GRANT-REPLACEMENT-KEYWORD (qa/archetype-qa/engine-gaps.md)

### BT25-102 Factorial Area
- Effect text: "[Security] [All Turns] All of your Black or Red [TS] trait Digimon gain ＜Blocker＞. While you have [Vulcanusmon], they also gain ＜Link +1＞."
- Missing engine API: aura cannot grant a numeric max-link increase — aura `modifier:` path applies a hardcoded value of 0; no grantable `Link` keyword variant.
- Gap tag: G-ENGINE-AURA-GRANT-LINK-MAX (qa/archetype-qa/engine-gaps.md)

## DSL-Vocab-Gap Blocked Cards
### BT25-103 GraceNovamon
- Effect text: "[When Attacking] [Counter] [Once Per Turn] For each of this Digimon's digivolution cards, you may trash any 1 digivolution card from your opponent's Digimon. Then, you may end this attack."
- Missing DSL verb: `select_opponent_sources` with a formula-resolved max (= source_material_count) AND cross-permanent candidate set (isFromOnly1Permanent: false).
- Lowers to engine API: opponent-digivolution-source selection machinery (single-permanent path exists); needs dynamic max + cross-permanent enumeration.
- Suggested DSL syntax: `select_opponent_sources: { max_fn: { source_material_count: {} }, min: 0, cross_permanent: true, bind_as: trashed, then: [ { trash_selected_sources: { source_refs: trashed } } ] }`
- Gap tag: G-DSL-SELECT-OPP-SOURCES-DYNAMIC-CROSS-PERMANENT (qa/dsl-vocab-gaps.md)

## New Patterns Discovered
- Area-option family (BT25-094/097/099/102) is the keyword-aura sister of BT25-095's +DP aura: the primary security aura grants a **combat** keyword (`has_keyword`-gated: Alliance/Blocker/Piercing/Rush) and works via aura `grant_keyword`; the **conditional secondary** keyword is the discriminator — combat keywords ship, but replacement-keywords (Scapegoat) and numeric keywords (Link +1) hit engine gaps. Worth a note in RUST_DSL_TEST_API.md that aura `grant_keyword` is faithful only for has_keyword-gated combat keywords.
