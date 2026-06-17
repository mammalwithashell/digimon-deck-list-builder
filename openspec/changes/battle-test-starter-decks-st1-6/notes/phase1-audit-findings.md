# Phase 1 — Faithfulness re-audit findings (2026-06-14)

Six parallel read-only Opus auditors re-derived every verdict from the **card image + DCGO C# + general_rule.pdf** (templated 2026-05-29 verdicts treated as untrusted). Orchestrator independently verified every BUG/GAP against DCGO before deciding.

## Headline
**90 / 96 cards cleanly OK.** No missing clauses, wrong outcomes, wrong numerics, name-vs-trait confusion, auto-selections, or crashes were found. All flagged items are *minor action-space-fidelity* nuances. The decks are in strong shape.

Per-deck: ST-1 16 OK · ST-2 14 OK / 2 BUG · ST-3 16 OK · ST-4 14 OK / 2 GAP · ST-5 16 OK · ST-6 14 OK / 2 GAP(flagged).

## Triage decisions

### FIX now (clean, DCGO-backed, low-risk)
- **ST2-06 Garurumon** — inherited [When Attacking] "trash the bottom digivolution card of 1 of your opponent's Digimon". YAML filter has `materials_count_gte: 1`; DCGO `ST2_06.cs` `CanSelectPermanentCondition` checks ONLY `IsPermanentExistsOnOpponentBattleAreaDigimon` (no source check). Over-restriction → fix by dropping `materials_count_gte: 1`. Verified: ST2-03/ST2-09 DCGO *do* require the source count, so their YAML filters are correct and stay.

### REJECT (false positive)
- **ST6-12 VenomMyotismon** — flagged because YAML `optional_zero: true` lets you pick 0 while DCGO `ST6_12.cs` forces ≥1 (`CanEndSelectCondition` rejects empty). This is the SAME "Up to 2 → gain <keyword>" pattern the ST-5 audit correctly ruled OK for ST5-12 (Reboot) and ST5-15 (Laser Eye). Per CLAUDE.md, "can you pick 0 for *up to N*" is a RULES question → `general_rule.pdf` rule 15-10-2-2 (permits 0) outranks DCGO's UI-side force-≥1, and the project's `reference_dsl_optional_mandatory_selection_pitfall` memory says use `optional_zero` for "up to N". **No change; behavior is correct and consistent with ST5.**

### DEFER + document (out of scope / risky / negligible)
- **ST4-13 HerculesKabuterimon** & **ST4-15 Needle Spray** — suspend target filtered `is_unsuspended: true`; DCGO + rule 15-15-6-3 allow choosing any opponent Digimon (incl. already-suspended). This is a **substrate-wide convention shared by 46+ cards**, not an ST-4 authoring error. Changing it is a cross-cutting action-space decision out of scope for an ST-1..6 change. Logged to gap tracker; flagged for the user. (Suspending an already-suspended Digimon is a no-op, so gameplay impact is nil; only RL action-set breadth differs.)
- **ST6-13 CresGarurumon** — [Main] `<Digi-Burst 2>` activation `condition` requires a valid Lv3 purple Digimon already in trash; DCGO gates only on `CanDigiBurst()` (≥2 sources) and plays nothing if no target. The removed line (pay Digi-Burst with nothing to play) is never correct play; dropping the gate risks a soft-lock on the mandatory inner `select_trash` with no candidates. Current behavior is strictly safe. Logged; defer unless the user wants strict action-space parity (would need the inner play step made skippable-when-empty + a soft-lock test).
- **ST2-15 Kaiser Nail** — source-selection filter doesn't enforce DCGO's `CanPlayAsNewPermanent` playability gate (unplayable sources are selectable, then the play silently fizzles). Behavior converges (you can't play it anyway). Likely needs a new DSL "playable-as-new-permanent" filter predicate (DSL-vocab gap). Logged; cosmetic.

## Cosmetic notes (no action)
- ST5-09 / ST5-12 / ST5-13: YAML uses `expiry: end_of_opponents_turn` rather than `_next_turn`; for own-turn installs these are identical (verified by passing tests). Fine.
- ST1-07: YAML omits inert `form`/`attribute` fields (resolved from metadata). Fine.
- ST1-07 ignored test (`G-DECLARATIVE-KEYWORD`): legitimately retained to document a generic engine gap (own-scope declarative keyword install); ST1-07's real inherited grant works via stack-walk + combat tick-fresh strike. Not a card bug.

## Data-quality (cards.json) — informational, YAML is faithful
- ST3-15 Holy Flame: cards.json text says "checks 3 *additional*" / "their turn"; image+DCGO say "3 *fewer*" / "your opponent's *next* turn". YAML follows image/DCGO (correct).
- ST2-15/ST2-16: cards.json dropped "as another Digimon" / "to its owner's hand" + trash parenthetical; YAML correct per image/DCGO.

## Net fix list for Phase 2
1. ST2-06: drop `materials_count_gte: 1` (TDD).

Everything else is REJECT (ST6-12) or DEFER-with-documentation (ST4-13, ST4-15, ST6-13, ST2-15) — surfaced to the user, logged to the gap tracker. These do not block training-readiness (no crashes/soft-locks/wrong outcomes).
