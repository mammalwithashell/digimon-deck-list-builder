# Archetype DSL Implementation: EX13 slice — Witchelny
Date: 2026-10-01
Total cards in pool: 3
Processed this run: 3
Pipeline: batch-implement-cards-rust-dsl

## Summary
- IMPLEMENTED: 3
- PARTIAL: 0
- AUDITED-OK: 0
- AUDITED-MISSING-TESTS: 0
- AUDITED-DRIFT: 0
- BLOCKED (engine): 0
- BLOCKED (dsl): 0
- BLOCKED (hybrid): 0
- SKIPPED (prior verdict): 0

## Per-Card Verdicts
| Card ID | Name | Mode | Verdict | Review | Tests | Notes |
|---------|------|------|---------|--------|-------|-------|
| EX13-025 | Candlemon | IMPLEMENT | IMPLEMENTED | self-review | 20/20 | Start of Main: 3+ security → trash top OR bottom (EffectChoice), Draw 1, +1 memory; then ≤2 security → optional [Witchelny]-text card from hand to bottom security. Inherited OPT leave save (opponent effects, trash top security), carrier gated on [Dynasmon]/[Witchelny] in text. |
| EX13-029 | FlameWizardmon | IMPLEMENT | IMPLEMENTED | self-review | 16/16 | <Armor Purge>; WD/WA shared OPT optional "by trashing top security" (outer prompt) → -4000 for the turn, then ≤3 security → delete 1 opp Digimon ≤4000 DP; also treated as [Wizardmon]; same inherited save as EX13-025. |
| EX13-033 | Mistymon | IMPLEMENT | IMPLEMENTED | self-review | 19/19 | <Barrier>; OP/WD optional [Witchelny]-text card from hand to bottom security, then optional pay (EffectChoice) trash top security → 1 own Digimon may attack; [All Turns][OPT] own security removed → -6000 for the turn, then ≤3 security → delete ≤6000 DP; inherited OPT may unsuspend. |

## Interpretation notes
- EX13-025 "If 3 or more … Then, if 2 or fewer …": the two processing conditions are evaluated independently (15-6-2). The second is re-read after the trash, so exactly 3 security → trash → 2 → placement offered, and starting at ≤2 runs only the placement half.
- "By trashing your top security card" without "you may" is still an optional processing condition (15-7-4). It is gated on security ≥ 1 (15-7-2) and can be paid even if the result does nothing (15-7-5).
- Inherited save (EX13-025/029) follows DCGO BT18_030.cs: WhenRemoveField, optional, OPT, IsOpponentEffect, top-card gate, SecurityCards ≥ 1.
- DCGO has no EX13_025/029/033.cs. Printed text from the official Bandai DB bundles.
- The per-card JSONs `cards/ex13/EX13-0{25,29,33}.json` were missing and were extracted verbatim from `data/cards.json`.

## Engine-Gap Blocked Cards
None.

## DSL-Vocab-Gap Blocked Cards
None.

## New Patterns Discovered
- Second-half optional "by" cost inside a multi-part clause: `select_effect_choice` (pay / don't) under a `security_count_gte: 1` gate. Use this when a clause-level `outer_prompt` can't carry the cost.
- Self-scoped leave replacement with a carrier-text gate: `in_text_contains` inside `active_when` does not flip the replacement to cross-permanent subject matching, so it stays "this Digimon".
