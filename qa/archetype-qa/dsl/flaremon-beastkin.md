# Archetype DSL Implementation: Flaremon / beastkin (BT25 slice)
Date: 2026-06-06
Total cards in slice: 4
Processed this run: 4
Pipeline: batch-implement-cards-rust-dsl

## Summary
- IMPLEMENTED: 4
- PARTIAL: 0
- AUDITED-OK: 0
- AUDITED-MISSING-TESTS: 0
- AUDITED-DRIFT: 0
- BLOCKED (engine): 0
- BLOCKED (dsl): 0
- BLOCKED (hybrid): 0
- SKIPPED (prior verdict): 0

All four cards are fully implemented (no approximations, every choice surfaced
through pending selection). 37 behavioral tests across the slice, all green
(verified through an isolated test harness — see "Environment note" below).

## Per-Card Verdicts
| Card ID | Name | Mode | Verdict | Review | Tests | Notes |
|---------|------|------|---------|--------|-------|-------|
| BT25-024 | Lekismon | IMPLEMENT | IMPLEMENTED | self | 7/7 | Lv4 Blue. Draw1; YourTurn red-played/digivolve → self digivolve into Crescemon **in trash** cost-1; Jamming. |
| BT25-016 | GrapLeomon | IMPLEMENT | IMPLEMENTED | self | 9/9 | Lv5 Red. OnPlay +3000DP + may-attack; all-turns 13000+DP-attacker → free digivolve into Marsmon/Callismon; SecurityA+1. |
| BT25-017 | Flaremon | IMPLEMENT | IMPLEMENTED | self | 8/8 | Lv5 Red. OnPlay may-attack then trash-hand → delete opp ≤7000DP; YourTurn blue → self digivolve into Apollomon cost-2; SecurityA+1. |
| BT25-054 | GreatGrizzlymon | IMPLEMENT | IMPLEMENTED | self | 10/10 | Lv5 Green. Blocker; OnPlay taunt; win-battle → free digivolve into Callismon/Marsmon; inherited OPT win-battle trash-security. |

## Key implementation patterns (reusable for the rest of the BT25 TS/beastkin set)

- **Alt-digivolve "Lv.X w/ [TS] trait: Cost N"** (all four cards): a second
  `alt_paths: - kind: digivolve, from: { level_eq: X, trait_has: TS }, cost: N`
  alongside the printed standard `{ level_eq, color_is, cost }` evo block.
  (DCGO `AddSelfDigivolutionRequirementStaticEffect`.)

- **"When your Digimon are played or digivolve, if any are <color>, this Digimon
  may digivolve into [Named] … cost reduced by N"** (BT25-024, BT25-017):
  `when: [on_enter_field_anyone, on_digivolve]`, `active_when: { your_turn: true }`,
  `condition: { all_of: [ event_target_owner: you, event_card_color_has: [<color>] ] }`,
  `optional: true`, then `select_hand`(named) → `effect_initiated_digivolve
  { target: source, from_hand: evo, cost: { reduce: N } }`. Same shape as
  BT16-085 / BT16-028.

- **Digivolve into a named card *in the TRASH*** (BT25-024): the DSL supports this
  via `effect_initiated_digivolve`'s **`source:`** field (NOT `from_hand`) bound
  from a `select_trash` step — the BT16-040 Wormmon idiom. No DSL gap; the
  earlier suspicion (no `from_trash`) was refuted by `EffectDigivolveArgs.source`.

- **"This/1-of-your Digimon may attack"** (BT25-016, BT25-017): `may_attack_now
  { attacker: <source | bound-selection>, targets: any, optional: true }`. For
  "1 of your Digimon may attack" the attacker is a `select_own_permanent`
  (optional) binding so the player both chooses the attacker and may decline.

- **"by trashing 1 card in your hand, <effect>"** (BT25-017): optional cost via
  `select_hand { optional: true, bind_as }` → `trash_from_hand_by_index` then
  the payoff guarded by `if { binding_present: <bind> }` (no half-paid cost).

- **Taunt — "Give 1 opp Digimon '[Start of Your Main Phase] This Digimon
  attacks.' until their turn ends"** (BT25-054): `select_opponent_permanent`
  (mandatory) → `grant_triggered_effect { timing: start_of_your_main_phase,
  expiry: end_of_opponents_turn, body: [ force_attack { attacker: carrier } ] }`.
  Identical to EX10-034 Blastmon Clause A.

- **"When this Digimon wins a battle …"** (BT25-054, main + inherited):
  `when: [on_any_deletion]`, `condition: { source_deleted_battle_opponent: true }`,
  `active_when: { all_turns: true }`. The resolved `source_deleted_battle_opponent`
  predicate (driver ST4-11) = carrier deleted its direct battle opponent and
  survived. Used both for the free digivolve and the inherited OPT trash-security.

## Source-priority resolution

- **BT25-024 "in the hand" vs "in the trash":** `data/cards.json` says
  `[Crescemon] in the hand`, but the **card image** (`BT25-024.webp`) and DCGO
  C# both say **trash** (`DigivolveIntoHandOrTrashCard(..., isHand:false)`).
  Per CLAUDE.md source priority, the printed card face outranks API-ingested
  `cards.json`. Implemented as TRASH and documented in the YAML header.

## Engine-Gap Blocked Cards
None.

## DSL-Vocab-Gap Blocked Cards
None. (The suspected `effect_initiated_digivolve`-from-trash gap was refuted:
the `source:` field already supports a trash-bound evolution card.)

## New Patterns Discovered
- None requiring new RUST_DSL_TEST_API documentation; all patterns reuse existing
  idioms (BT16-040/085/028, EX10-034, ST4-11, BT20-016).

## Environment note (not a card issue)
At implementation time the shared `cards_behavioral` test binary was
non-compiling due to several **other concurrent sessions'** in-flight files
(e.g. `bt25_078.yaml` invalid DSL; `bt25_006/019/068/071.rs`, `bt12_021.rs`,
`p_117.rs`, etc. referencing changed/absent engine APIs). The four cards in this
slice were validated through a temporary isolated harness that included only the
four slice modules; that harness reported **37 passed / 0 failed** and was then
removed. The four production YAMLs each compile cleanly into the engine's
`build.rs` card pack (no parse errors/warnings). Re-running the full
`cards_behavioral` suite green requires the other sessions' files to land.
