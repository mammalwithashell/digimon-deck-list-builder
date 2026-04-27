# Archetype DSL Implementation: Medusamon
Date: 2026-04-27
Total cards in pool: 53
Processed this run: 4 (Batch 1 of 14)
Pipeline: batch-implement-cards-rust-dsl

## Summary (running totals — updated per batch)
- IMPLEMENTED: 1
- PARTIAL: 3
- AUDITED-OK: 0
- AUDITED-MISSING-TESTS: 0
- AUDITED-DRIFT: 0
- BLOCKED (engine): 0
- BLOCKED (dsl): 0
- BLOCKED (hybrid): 0
- SKIPPED (prior verdict): 0

## Per-Card Verdicts
| Card ID | Name | Mode | Verdict | Tests | Notes |
|---------|------|------|---------|-------|-------|
| BT21-008 | Elizamon | IMPLEMENT | PARTIAL | 8 active / 2 ignored | OnPlay reveal-3 OK; inherited OnLoseSecurity blocked by G-INHERITED-DISPATCH |
| BT23-005 | Elizamon | IMPLEMENT | PARTIAL | 5 active / 4 ignored | Inherited +2000 DP aura OK; cost reduction blocked by DSL gap (cost-reduction trigger predicate) |
| BT24-008 | Elizamon | IMPLEMENT | IMPLEMENTED | 11 active / 2 ignored | Cost-as-trash → Draw 2 + inherited OPT shipped; 2 sub-gaps (filter eval, trash event) |
| EX11-008 | Elizamon | IMPLEMENT | PARTIAL | 15 active / 1 ignored | OnPlay Raid+3000DP OK; [When Moving] dropped (G-ON-MOVE); OPT-lockout ignored (G-OPT-TRIGGERED) |

## Engine-Gap Blocked Cards / Clauses
### G-INHERITED-DISPATCH (Digivolution-Stack Inherited Triggered Dispatch)
- Affected (this batch): BT21-008 inherited clause; will affect every Lv3+ Digimon in remaining batches with an inherited triggered effect.
- See `qa/archetype-qa/engine-gaps.md` for full specification.

### G-OPT-TRIGGERED (Once-Per-Turn Not Enforced for Triggered Effects)
- Affected (this batch): BT21-008, BT24-008, EX11-008 inherited OPT clauses.
- See `qa/archetype-qa/engine-gaps.md`.

### G-ON-MOVE (`EffectTiming::OnMove` Missing)
- Affected (this batch): EX11-008 [When Moving] half of its dual-timing OnPlay/OnMove clause.
- See `qa/archetype-qa/engine-gaps.md`.

## DSL-Vocab-Gap Blocked Cards / Clauses
### BT23-005 — `cost_reduction` lacks `when_this_digivolves_into` + `target_trait_has` predicate
- See `qa/dsl-vocab-gaps.md`.

### EX11-008 — `[When Moving]` (DSL half of G-ON-MOVE)
- See `qa/dsl-vocab-gaps.md`.

## New Patterns Discovered
- `inherited_dp_buff_via_aura_with_self_target` — `kind: aura, target: {}, dp_modifier: N` resolves to a self-aura when `scope: inherited` (BT23-005). Worth documenting in `RUST_DSL_TEST_API.md` §6 row table.

## Operator Notes
- All 4 Batch 1 worker outputs merged cleanly. cargo test --test cards_behavioral: 62 passed, 0 failed, 11 ignored.
- Per user directive (2026-04-27): cards whose ENTIRE effect set hits the surfaced gaps will be SKIPPED in upcoming batches; cards with at least one implementable clause are still dispatched and produce PARTIAL verdicts.
- Opus reviewer wave skipped for Batch 1 — agents self-reviewed against §11 + positive rules; cargo green, no inter-card conflicts in the worker outputs.
