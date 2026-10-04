# DSL Implementation: BT26 meta blockers
Date: 2026-10-03
Total cards in pool: 28 (22 last-mile/Training cards + 4 engine-gap cards + 2 adjacent fixes found in review: BT25-059 Ceresmon, BT5-092 Nokia)
Pipeline: batch-implement-cards-rust-dsl (scout folded into implementer; Opus review wave over all cards)

## Summary
- IMPLEMENTED: 28
- PARTIAL: 0
- BLOCKED: 0
- Reprint check: no promo in scope is a text-identical reprint (P-200≠BT26-090, P-209≠BT25-084, P-210≠BT24-083/BT26-088, BT19-001≠BT10-003, BT12-011≠BT10-111, BT24-033≠other Salamons).

## Per-Card Verdicts
| Card ID | Name | Mode | Verdict | Review | Tests | Notes |
|---------|------|------|---------|--------|-------|-------|
| BT25-077 | Bacchusmon | AUDIT/FIX | IMPLEMENTED | APPROVED (after fixes) | 19 | Cost -5 via new level_sum_gte (G-DSL-BOARD-LEVEL-SUM resolved); Green Lv.5 circle added. Review fixes: [All Turns] clause no longer skippabl |
| BT25-087 | Thomas H. Norstein | AUDIT/FIX | IMPLEMENTED | APPROVED (after fixes) | 21 | Both former blockers (OnAddToHand trigger, selection-bearing BeforePayCost pay_cost) were already resolved; YAML narrowed to DATA SQUAD Digi |
| BT25-103 | GraceNovamon | IMPLEMENT | IMPLEMENTED | APPROVED (after fixes) | 23 | Closed G-ENGINE-DSL-FIELD-COUNTER-WINDOW (DSL [Counter] lowers with Effect.counter; Counter window on every attack incl. player; Counter bod |
| BT25-050 | Kiwimon | AUDIT/FIX | IMPLEMENTED | APPROVED (after fixes) | 13 | G-ENGINE-IF-AFTER-SELECTION-NOT-RESUMED was a misdiagnosis: count_gte defaults to owner: you; gate is board-wide (owner: any). Review fixes: |
| BT25-059 | Ceresmon | AUDIT/FIX | IMPLEMENTED | APPROVED (after fixes) | 16 | Fixes: colors Green/Yellow + Yellow Lv.5 route; self cost reduction now actually applies (before_pay_cost + when_playing_this) and counts bo |
| BT5-092 | Nokia Shibuya | AUDIT/FIX | IMPLEMENTED | APPROVED (after fixes) | 24 | Digivolve reducer is optional (DCGO isOptional) via G-COST-REDUCTION-OPTIONAL-SYNC-PAY-COST-DIGIVOLVE; [On Play] exact names; DoruGreymon/Bu |
| EX10-070 | God Grade Unleashed | IMPLEMENT | IMPLEMENTED | APPROVED (after fixes) | 11 | Color bypass, [Main] draw+place, [Security] place, and the link-card-trashed <Delay> (G-DSL-ON-LINK-CARD-TRASHED-DELAY). |
| BT24-099 | Super Hacking | IMPLEMENT | IMPLEMENTED | APPROVED (after fixes) | 15 | Cost-gated placement: official Q&A (no trash -> no draw, no placement, card trashed) via G-ENGINE-DELAY-OPTION-CONDITIONAL-PLACEMENT. Delay  |
| EX1-072 | Emergency Program Shutdown! | IMPLEMENT | IMPLEMENTED | APPROVED (after fixes) | 11 | New ModifierType::CannotUseOptionCards (G-ENGINE-CANNOT-USE-OPTION-CARDS): [Main] until end of opponent's next turn, [Security] this turn, t |
| BT2-041 | ShineGreymon | IMPLEMENT | IMPLEMENTED | APPROVED (after fixes) | 12 | Suspend all own yellow Tamers, then repeat once per Tamer actually suspended (new per: effect_suspended_count); [Your Turn] +1000 per Tamer. |
| P-209 | Titamon | IMPLEMENT | IMPLEMENTED | APPROVED (after fixes) | 14 | Promo printing (not a reprint of BT25-084). Alliance; trash-cost suspend + CannotUnsuspend; OPT play Lv<=4 [Demon]/[Titan] from trash on han |
| BT4-111 | Jack Raid | IMPLEMENT | IMPLEMENTED | APPROVED (after fixes) | 8 | Gain 1 memory per 10 trash cards; [Security] gain 2. |
| P-200 | Kanan Yuki | IMPLEMENT | IMPLEMENTED | APPROVED (after fixes) | 10 | Not a reprint of BT26-090. Optional suspend-self TS digivolve reducer via G-COST-REDUCTION-OPTIONAL-SYNC-PAY-COST-DIGIVOLVE. |
| P-210 | Hiroko Sagisaka | IMPLEMENT | IMPLEMENTED | APPROVED (after fixes) | 7 | Not a reprint of BT24-083/BT26-088. |
| BT24-033 | Salamon | IMPLEMENT | IMPLEMENTED | APPROVED (after fixes) | 12 | Red Lv.2 circle restored (ingest dropped it). |
| BT8-108 | Mist Memory Boost! | IMPLEMENT | IMPLEMENTED | APPROVED (after fixes) | 7 | Trash 2 + Draw 1 then place (implicit); Delay gain 2; [Security] place. |
| P-240 | Arcturusmon | IMPLEMENT | IMPLEMENTED | APPROVED (after fixes) | 14 | Printed data corrected in cards.json/card_overrides.json (ingest had cost 0, no DP/text). |
| P-244 | Unique Emblem: Ragnarok Attainer | IMPLEMENT | IMPLEMENTED | APPROVED (after fixes) | 12 | Delay on effect-placed [Vemmon] fixed via G-ENGINE-EVENT-DELAY-ON-ADD-DIGIVOLUTION-CARDS. [Your Turn] per card image + DCGO. Printed data co |
| EX11-066 | Xeno | IMPLEMENT | IMPLEMENTED | APPROVED (after fixes) | 15 | also_treated_as Zenith (Rule) in YAML. |
| BT21-098 | Ragnarok Cannon | IMPLEMENT | IMPLEMENTED | APPROVED (after fixes) | 11 | Galacticmon attack gate uses event_target_name_contains (exact for current pool; G-DSL-EVENT-TARGET-NAME-IS logged). |
| BT8-095 | Fire Rocket | IMPLEMENT | IMPLEMENTED | APPROVED (after fixes) | 8 |  |
| BT19-001 | Pickmons | IMPLEMENT | IMPLEMENTED | APPROVED (after fixes) | 10 | Declining the optional hand pick refunds the OPT (DCGO optional activation). |
| BT12-011 | Shoutmon (King Version) | IMPLEMENT | IMPLEMENTED | APPROVED (after fixes) | 21 | Closed G-ENGINE-INNATE-SAVE-SEQUENCED-DUPLICATE and G-ENGINE-DIGIXROS-SLOT-PREDICATE. |
| LM-057 | Wall Training | IMPLEMENT | IMPLEMENTED | APPROVED (after fixes) | 20 | Shared Training-option shape (red/blue). Delay target gating fixed via G-HAS-DIGIVOLVE-CANDIDATE-COMPOSITE-FILTER. |
| LM-058 | Parkour Training | IMPLEMENT | IMPLEMENTED | APPROVED (after fixes) | 10 | Shared Training-option shape (blue/green). Delay target gating fixed via G-HAS-DIGIVOLVE-CANDIDATE-COMPOSITE-FILTER. |
| LM-059 | Heat Training | IMPLEMENT | IMPLEMENTED | APPROVED (after fixes) | 10 | Shared Training-option shape (yellow/red). Delay target gating fixed via G-HAS-DIGIVOLVE-CANDIDATE-COMPOSITE-FILTER. |
| LM-060 | Shadow Training | IMPLEMENT | IMPLEMENTED | APPROVED (after fixes) | 10 | Shared Training-option shape (green/purple). Delay target gating fixed via G-HAS-DIGIVOLVE-CANDIDATE-COMPOSITE-FILTER. |
| LM-061 | Punching Training | IMPLEMENT | IMPLEMENTED | APPROVED (after fixes) | 10 | Shared Training-option shape (black/red). Delay target gating fixed via G-HAS-DIGIVOLVE-CANDIDATE-COMPOSITE-FILTER. |

## Substrate widened (rule 28)
See qa/resolved-gaps.md (2026-10-03 entries): G-DSL-BOARD-LEVEL-SUM, G-ENGINE-IF-AFTER-SELECTION-NOT-RESUMED (misdiagnosis), G-ENGINE-DSL-FIELD-COUNTER-WINDOW, G-ENGINE-OPP-SOURCES-HOST-KIND, G-HAS-DIGIVOLVE-CANDIDATE-COMPOSITE-FILTER, G-DSL-FORMULA-EFFECT-SUSPENDED-COUNT, G-ENGINE-INNATE-SAVE-SEQUENCED-DUPLICATE, G-ENGINE-DIGIXROS-SLOT-PREDICATE, G-ENGINE-EVENT-DELAY-ON-ADD-DIGIVOLUTION-CARDS, G-COST-REDUCTION-OPTIONAL-SYNC-PAY-COST-DIGIVOLVE, G-ENGINE-CANNOT-USE-OPTION-CARDS, G-ENGINE-DELAY-OPTION-CONDITIONAL-PLACEMENT, G-DSL-ON-LINK-CARD-TRASHED-DELAY, G-DSL-COST-TARGET-FROM-HAND.

## New Patterns Discovered
- `select_*_permanent` with `optional: true` ends the whole clause on PASS unless `continue_on_decline: true`; DCGO canNoSelect continues. "You may <pick>. Then, …" cards need `continue_on_decline` (BT25-077, BT25-050 fixed; pool-wide audit recommended).
- An optional activation whose first step is a declinable pick still spends OPT on decline unless the body `refund_opt`s (BT19-001, BT25-077).
- Self play-cost reductions need `reduction_timing: before_pay_cost` + `when_playing_this: true`; a bare `cost_reduction` with `condition:` never applies (BT25-059 shipped that way untested).
- `count_gte` defaults its filter owner to `you`; board-wide "if there are N … Digimon" needs `owner: any`.
- Fast TDD loop: compile one card test as its own test target with the draft YAML inlined via `from_dsl_yaml(include_str!)` (~5 s vs ~5 min for the full binary).
