# Archetype DSL Implementation: beatbreak (Glowing Dawn / BEATBREAK)
Date: 2026-06-06
Cards processed this run: 7 (BT25-088, BT25-090, BT25-035, BT25-049, BT25-081, BT25-041, BT25-057)
Pipeline: batch-implement-cards-rust-dsl (subagent run)

## Summary
- IMPLEMENTED: 1 (BT25-081)
- PARTIAL: 5 (BT25-088, BT25-090, BT25-049, BT25-035, BT25-041)
- BLOCKED (hybrid): 1 (BT25-057)

All shipped YAML + tests are green (verified via an isolated test binary while
the shared `cards_behavioral` binary was under concurrent sibling authoring;
the per-card modules are also registered in `tests/cards_behavioral/bt25/mod.rs`).

## Per-Card Verdicts
| Card ID | Name | Mode | Verdict | Tests | Notes |
|---------|------|------|---------|-------|-------|
| BT25-088 | Kyo Sawashiro | IMPLEMENT | PARTIAL | 9/9 | Tamer. cards.json text WRONG (only inherited line) — authored from card image + DCGO. set-3 / on-lose-security suspend+place-2-FD / [Security] play-self done; Glowing Dawn play -1 BLOCKED (interactive pay_cost). |
| BT25-090 | Tomoro Tenma | IMPLEMENT | PARTIAL | 9/9 | Tamer. set-3 / on-any-Digimon-suspend suspend+place-2-FD / [Security] play-self done; Glowing Dawn Option-use -1 BLOCKED. |
| BT25-035 | Cougarmon | IMPLEMENT | PARTIAL | 6/6 | -3000 DP / inherited Barrier / Glowing Dawn alt-digi done; trash-2 free-digivolve BLOCKED (G-TRASH-N). |
| BT25-049 | Armalizamon | IMPLEMENT | PARTIAL | 7/7 | optional suspend opp / inherited Piercing / alt-digi done; Glowing Dawn Option-use -3 BLOCKED. |
| BT25-081 | Fangmon | IMPLEMENT | IMPLEMENTED | 10/10 | All clauses: suspend non-purple Tamer, opp-Tamer-suspend +1 memory (OPT), inherited Retaliation. Widened DSL keyword map (Retaliation). |
| BT25-041 | Murasamemon | IMPLEMENT | PARTIAL | 5/5 | Alliance / alt-digi / inherited End-of-Attack trash-FD→unsuspend host done; main play/use-reduced clause BLOCKED. |
| BT25-057 | Monarchlizamon | IMPLEMENT | BLOCKED (hybrid) | 0 | DUAL card (cards.json mislabels as Digimon). DSL dual has no per-face effects + no Arts Digivolve authoring. |

## Substrate widened this run
- **DSL keyword map: `Retaliation`** — added to `code/digimon-engine/src/dsl_cards/modifier_map.rs`
  `lookup_keyword` and `code/digimon-dsl/src/validator.rs` `KNOWN_KEYWORD_KEYS`.
  `Keyword::Retaliation` was already wired behaviorally
  (`cards/keyword_effects.rs`, `tests/keyword_phase_e/retaliation.rs`) but the
  DSL `grant_keyword: Retaliation` path silently no-op'd (returned `None`). Now
  it installs the keyword — also retroactively fixes the previously-shipped
  BT25-078 inherited Retaliation. DSL parity tests (`--test dsl phase1c`) stay green.

## Engine-Gap Blocked Cards
### Glowing Dawn cost-reduction-by-trashing-face-down (BT25-088 c3, BT25-090 c3, BT25-049 c2, BT25-041 main)
- **Gap:** `G-COST-REDUCTION-INTERACTIVE-PAY-COST` (docs/RUST_ENGINE_GAPS.md, NEW this run).
- A `kind: cost_reduction` `pay_cost` that installs an interactive selection
  (the `trash_bottom_face_down_source_under_tamer` Tamer pick) parks → the
  engine drops the reduction `amount` credit while still paying the cost. The
  reduction is silently lost. Fix: defer the credit to the parked pay_cost's
  success continuation. (The same verb works as a *process activation cost* —
  see BT25-041 inherited unsuspend, BT25-057 De-Digivolve.)

## DSL-Vocab-Gap Blocked Cards
### BT25-035 Cougarmon — trash-2 free-digivolve
- `G-TRASH-N-BOTTOM-FACE-DOWN-UNDER-TAMER` (qa/dsl-vocab-gaps.md, pre-existing) +
  effect-driven free-digivolve-into-a-hand-card.

### BT25-057 Monarchlizamon — DUAL card
- `G-DSL-DUAL-PER-FACE-EFFECTS` (DualSpec carries no `effects:` per face) +
  Arts Digivolve authoring (`G-DSL-ARTS-DIGIVOLVE`). See qa/dsl-vocab-gaps.md.

## Notable data findings
- `data/cards.json` is WRONG for **BT25-088** (only carried the inherited
  [Security] line; the full 4-clause Tamer text is on the card image + DCGO) and
  **mislabels BT25-057** as a plain Digimon when it is a DUAL card. Both were
  authored/assessed against the card image + DCGO per CLAUDE.md source priority.
