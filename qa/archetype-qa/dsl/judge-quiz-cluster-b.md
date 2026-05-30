# Archetype DSL Implementation: Judge-Quiz Cluster B (Q6/Q8/Q13/Q14/Q24)
Date: 2026-05-30
Total cards in pool: 12
Pipeline: batch-implement-cards-rust-dsl (--implementer-model opus)

Cluster B is the ≤0-DP-rules-check family. Gap 1 (G-NO-GENERAL-ZERO-DP-RULES-CHECK)
was closed 2026-05-29, unblocking these scenarios at the engine level; this run authors
their cards so Q6/Q8/Q13/Q14/Q24 can flip BLOCKED-CARD → PASS.

## Summary (running)
- IMPLEMENTED: 7  (BT21-042, AD1-016, BT21-044, BT13-020, BT16-101, EX4-005, BT21-004)
- PARTIAL: 1  (ST17-07 — protection clause BLOCKED on a logged DSL gap)
- BLOCKED: 0
- SKIPPED: 0
- Remaining to process: 4 (Batch 3: BT23-101, BT23-037, EX6-004, BT8-109)

## Substrate widened during this run (rule 28) — second wave
- **0-DP deletion cause (`EventCause::Rule`)** — `Game::run_state_based_rules_check` now
  tags its ≤0-DP state-based deletions with `EventCause::Rule` (via the
  `current_deletion_event_cause_override` slot, the Overclock idiom), so deletion observers
  can distinguish "deleted by having 0 DP" from "deleted by an effect". This is the
  LOAD-BEARING cluster-B mechanic — it unblocked BT16-101's gain-2-memory clause
  (`any_of[battle_deletion, rule]`) and is the correct semantics for all observers
  (a 0-DP rule deletion is not an effect deletion). Refines observer payload only;
  replacement-window filtering still reads `current_deletion_cause`.

## Deferred substrate follow-up (logged, not done)
- **`G-OPPONENT-SCOPED-EFFECT-PROTECTION-DSL`** (ST17-07) — `add_modifier` installs
  cause-agnostic protection; opponent-only delete/return immunity is unrepresentable.
  Backward-compatible opt-in widening (`opponent_only: bool` on add_modifier) deferred as a
  deliberate cross-cutting change (affects BT18-064/P-215/EX8-070). See `qa/dsl-vocab-gaps.md`.

## Worktree-persistence incident (recovered)
Three Batch-2 sub-agents (EX4-005, BT21-004, ST17-07) reported passing tests but ran in
isolated sandboxes that did not persist to the main tree (their files were 0-byte/missing;
the tell: each "had to create mod.rs"). Caught by the reviewer. Re-authored INLINE by the
orchestrator against the same exemplars + DCGO; all green. BT16-101 (same batch) + all of
Batch 1 persisted correctly.

## Substrate widened during this run (rule 28)
- **`TreatAsDigimon` / `SynthIdentity` payload** — the DSL `add_modifier` step now accepts a
  structured `synth_identity:` block (dp required; kind defaults Digimon; level/colors/traits
  optional), lowering to the engine's pre-existing `ModifierPayload::SynthIdentity` via a new
  `EffectContext::add_modifier_with_payload`. Closes the Track C "treat a Tamer as a Digimon"
  slice. Used by BT21-044 (3000 DP) + BT13-020 (12000 DP) in pure DSL — no raw_rust.
  Pinned by `digimon-dsl` `parse_synth_identity` + `validator::tests`. Logged in
  `qa/dsl-vocab-gaps.md`.

## Per-Card Verdicts
| Card ID | Name | Mode | Verdict | Review | Tests | Notes |
|---------|------|------|---------|--------|-------|-------|
| BT21-042 | GeoGreymon | IMPLEMENT | IMPLEMENTED | APPROVED | 17 | on_enter_field_anyone Marcus observer → free self-digivolve into [RizeGreymon]; +2000 inherited aura; Agumon alt-path |
| AD1-016 | ShineGreymon | IMPLEMENT | IMPLEMENTED | NEEDS-FIX→fixed | 22 | Alliance/Blocker (runtime-installed, behaviorally tested); union hand/trash Marcus play + formula debuff; All-Turns delete observer |
| BT21-044 | RizeGreymon | IMPLEMENT | IMPLEMENTED | APPROVED | 20 | TreatAsDigimon(3000)+CannotDigivolve+Rush+Alliance on Marcus; may_attack_now; Tamer-deletion→Marcus-to-security |
| BT13-020 | ShineGreymon: Burst Mode | IMPLEMENT | IMPLEMENTED | NEEDS-FIX→fixed | 18 | TreatAsDigimon(12000)+Rush on played Marcus; own-Tamer-suspend→trash opp security; Burst Digivolve alt-path |
| BT16-101 | Rapidmon (X Antibody) | IMPLEMENT | IMPLEMENTED | APPROVED | 21 | Armor Purge; suspend-all+may-attack; conditional aura -4000 to opp suspended; gain 2 memory on battle/0-DP delete (EventCause::Rule substrate) |
| EX4-005 | Agumon | IMPLEMENT | IMPLEMENTED | re-authored inline | 13 | start-of-main conditional +1 memory; inherited red/yellow-Tamer-suspend→Draw 1; Koromon alt-path |
| BT21-004 | Koromon | IMPLEMENT | IMPLEMENTED | re-authored inline | 9 | egg; inherited yellow/red-Tamer-suspend→Draw 1 |
| ST17-07 | Rapidmon | IMPLEMENT | PARTIAL | gap diagnosis confirmed | 11 (+2 ⌀) | De-Digivolve 1 + inherited battle-delete→trash security DONE; opponent-scoped protection BLOCKED (G-OPPONENT-SCOPED-EFFECT-PROTECTION-DSL) |

## Engine-Gap Blocked Cards
(none this batch)

## DSL-Vocab-Gap Blocked Cards
(none this batch — the TreatAsDigimon payload gap was widened, not routed around)

## New Patterns Discovered
- "treat a Tamer as a Digimon for the turn": `select_own_permanent`/`play_from_hand_free bind_as`
  → `add_modifier { modifier: TreatAsDigimon, synth_identity: { dp: N } }` + `CannotDigivolve`
  + `grant_keyword` (Rush/Alliance), all `expiry: end_of_turn`. Canonical refs: BT21-044, BT13-020.
- `place_on_security` from a `{ binding: <select_trash>, zone: trash }` source, face-up (BT21-044) —
  confirms trash→top-security path end-to-end.
