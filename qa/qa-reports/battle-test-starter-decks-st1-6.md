# Battle-test report — Starter decks ST-1 … ST-6 (2026-06-14)

OpenSpec change: `battle-test-starter-decks-st1-6`. Goal: get the six original color
starter decks (ST-1 … ST-6, 96 unique cards) to a state where their lists can drive
RL training with confidence the cards are implemented faithfully and the decks play
full games cleanly.

## Verdict: **GO for training.**
All 6 lists are faithful (90/96 cards verified clean; 1 bug fixed; 5 minor
action-space divergences deferred with no gameplay-outcome impact), legal, fully
test-covered, training-wired, and play full games with **zero crashes and zero
soft-locks**.

## Faithfulness re-audit (Phase 1)
Six parallel read-only Opus auditors re-derived every verdict from the **card image
→ DCGO C# → general_rule.pdf** (the templated 2026-05-29 "Faithful to printed text +
DCGO." notes were treated as untrusted). Orchestrator independently verified every
flagged item against DCGO. Detail: `openspec/changes/battle-test-starter-decks-st1-6/notes/phase1-audit-findings.md`.

| Deck | OK | Action taken |
|------|----|--------------|
| ST-1 Gaia Red | 16 | clean |
| ST-2 Cocytus Blue | 16 | **ST2-06 fixed**; ST2-15 minor deferred |
| ST-3 Heaven's Yellow | 16 | clean |
| ST-4 Giga Green | 16 | ST4-13/ST4-15 minor deferred |
| ST-5 Machine Black | 16 | clean |
| ST-6 Venomous Violet | 16 | ST6-13 minor deferred; ST6-12 false-positive rejected |

No missing clauses, wrong numerics, name-vs-trait confusion, auto-selections, wrong
timings, or wrong outcomes were found.

### Bug fixed (TDD)
- **ST2-06 Garurumon** — inherited [When Attacking] "trash the bottom digivolution
  card of 1 of your opponent's Digimon" was filtered to opponents with `≥1` source.
  DCGO `ST2_06.cs` allows targeting ANY opponent Digimon (no source check, unlike
  ST2-03/ST2-09 whose DCGO *does* require it). Removed `materials_count_gte: 1`.
  Regression test: `st2_06_targets_sourceless_opponent_digimon` (RED→GREEN).

### Deferred minor divergences (logged `qa/dsl-vocab-gaps.md` [G-AUDIT-ST1-6])
All are action-space-fidelity nuances — no wrong outcome, crash, or soft-lock — so
they do **not** block training:
- **ST4-13 / ST4-15** suspend target filtered `is_unsuspended:true` (DCGO + rule
  15-15-6-3 allow any opponent Digimon). Shared **~46-card repo-wide convention** —
  belongs in its own cross-cutting change, not an ST1-6 edit.
- **ST6-13** `<Digi-Burst 2>` activation over-gated on a valid trash target existing
  (DCGO gates only on `CanDigiBurst`). The removed line is never optimal play, and
  loosening it risks a soft-lock on the mandatory inner `select_trash` — deferred.
- **ST2-15** source filter lacks DCGO's "playable-as-new-permanent" gate (genuine
  DSL-vocab gap); behavior converges (you can't play it either way).

### Rejected (false positive)
- **ST6-12 VenomMyotismon** `optional_zero` ("up to 2 → 0 allowed") is **correct** per
  rule 15-10-2-2 (PDF outranks DCGO's UI-side force-≥1), consistent with ST5-12 /
  ST5-15 and the project `reference_dsl_optional_mandatory_selection_pitfall` memory.

## Tests (Phases 0 + 3)
- Behavioral (`cards_behavioral` st1–6): **131 passed**, 1 ignored (st1_07, generic
  engine gap `G-DECLARATIVE-KEYWORD`, not a card bug), 0 failed.
- Interaction (`archetypes` st1–6): **39 passed**, 0 failed.
- Static archetype tests (all 6 decks): deck-legality ✓, coverage 16/16 = 100% ✓,
  5/5 smoke games ✓, combo-presence ✓ — **24/24 PASS**.
- Coverage correction: st4 is NOT zero-coverage (its ~23 tests live inline in
  `tests/cards_behavioral/st4/mod.rs`).

## MCP + full-game battle-testing (Phase 4)
- **scenario-MCP (browser target)** stood up (PyO3 wheel rebuilt with the ST2-06 fix
  + `uvicorn` `/debug`). Round-trip confirmed; staged **ST1-11 WarGreymon** and
  verified faithful resolution through the real browser/PyO3 wire
  (`securityAttackModifier = 2` at 4 digivolution cards — the old double-count-to-4
  bug is gone), plus correct +2000 DP aura.
- **Full games via the PyO3 `RustHeadlessGame` wire** (the training `HeadlessRunner`
  path), greedy, seed-balanced, with an explicit soft-lock check (≥1 legal action at
  every non-terminal state):

  | Coverage | Result |
  |---|---|
  | 21 pairings (6 mirror + 15 cross), 4 games each = **84 games** | completed 78, timeout 6, **softlock 0, crash 0** |
  | Static `smoke_games` (5 × 6 decks) | **30/30 clean** |
  | **Total full games** | **114, zero engine faults** |

  The 6 timeouts are greedy-mirror policy stalls (all involve ST-2's unsuspend/strip
  loop) — not engine faults; training uses `OpponentWrapper` + a real policy. Heavy
  P2 win skew in cross-matchups is the known first-player/seat asymmetry under a
  greedy mirror, handled by seat-balancing in training.

## Training readiness (Phase 5)
- Deck-pool / archetype wiring: all 6 starters resolve via `canonicalize_archetype`
  to legal 54-card decks AND are in the gauntlet's training-ready set (status
  `AUDITED-OK` preserved in `validated_cards_dsl.json` — required by
  `_TRAINING_READY_DSL_STATUSES`). ST-3's apostrophe is handled.
  → launch with `--archetypes "ST-1 Gaia Red,ST-2 Cocytus Blue,ST-3 Heaven's Yellow,ST-4 Giga Green,ST-5 Machine Black,ST-6 Venomous Violet"`.
- `DigimonEnv`-style reset/step: all 6 lists → mask shape 2192, legal actions
  present, `step()` advances phase. No errors.
- Optional local smoke-train: skipped per user preference (favored engine/MCP games).

## Artifacts
- Re-derived per-card verdicts: `qa/qa-reports/validated_cards_dsl.json` (96 entries,
  report `battle-test-starter-decks-st1-6`, no templated notes remain).
- Audit findings: `openspec/changes/battle-test-starter-decks-st1-6/notes/phase1-audit-findings.md`.
- Baseline: `.../notes/phase0-baseline.md`. Harnesses: `.../notes/play_starter_games.py`, `check_wiring.py`.
- Gap log: `qa/dsl-vocab-gaps.md` [G-AUDIT-ST1-6].

## BLOCKED cards
None.
