# Archetype DSL Implementation: Aegiomon (slice aegiomon-1)
Date: 2026-06-06
Total cards in slice: 6
Processed this run: 6
Pipeline: batch-implement-cards-rust-dsl

## Summary
- IMPLEMENTED: 4
- PARTIAL: 0
- AUDITED-OK: 0
- AUDITED-MISSING-TESTS: 0
- AUDITED-DRIFT: 0
- BLOCKED (engine): 0
- BLOCKED (dsl): 2
- BLOCKED (hybrid): 0
- SKIPPED (prior verdict): 0

## Per-Card Verdicts
| Card ID | Name | Mode | Verdict | Review | Tests | Notes |
|---------|------|------|---------|--------|-------|-------|
| BT25-033 | Aegiomon | IMPLEMENT | IMPLEMENTED | self | 6/6 | Barrier own+inherited; optional [OP][WD] add-top-sec-to-hand → 1 opp -5000 DP (turn) |
| BT25-025 | Aegiochusmon: Blue | IMPLEMENT | IMPLEMENTED | self | 8/8 | Blocker; special Decode([Aegiomon]) replacement; [OP][WD] De-Digivolve 1 + sec≤3 unsuspend; inherited sec-removed Shaman unsuspend |
| BT25-053 | Aegiochusmon: Green | IMPLEMENT | IMPLEMENTED | self | 7/7 | Vortex; Decode([Aegiomon]); [OP][WD] suspend+freeze opp + sec≤3 Piercing+5000; inherited sec-removed suspend |
| BT25-018 | Apollomon | IMPLEMENT | IMPLEMENTED | self | 7/7 | Cost reduction; [OP][WD] all opp -2000/own-Digimon + delete DP≤this; EOT DNA→GraceNovamon + may attack; inherited [WA][OPT] delete DP≤this |
| BT25-039 | Sirenmon | IMPLEMENT | BLOCKED (dsl) | self | 0 | board-wide protect-other + security-EOT-play-and-place-self gaps |
| BT25-020 | Marsmon | IMPLEMENT | BLOCKED (dsl) | self | 0 | board-wide battle-winner predicate gap |

Tests: 28/28 green via `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt25_018 bt25_025 bt25_033 bt25_053`.

## Notes on the special <Decode ([Aegiomon])>
Aegiochusmon Blue/Green print `<Decode ([Aegiomon])>` = "play 1 [Aegiomon] from
its digivolution cards without paying the cost" on a non-battle leave. This is
NOT the generic engine `Keyword::Decode` (which redirects the leaving permanent
to hand). It is the BT22-015-class special-Decode: modeled as a
`kind: replacement` + `trigger: when_would_leave_battle_area` + non-battle cause
filter + `select_material` (name_contains Aegiomon) + `play_from_materials`
(free), with the original leave proceeding.

## DSL-Vocab-Gap Blocked Cards
### BT25-039 Sirenmon
- G-DSL-PROTECT-OTHER-BY-SELF-DELETE — "[All Turns] when another of your
  [Shaman]/[Iliad] Digimon/Tamer would leave (other than by your effects), by
  deleting this Digimon, they don't leave." Needs a would-leave replacement
  whose subject is a filtered set of OTHER owner permanents, plus a self-delete
  cost and cancel-leave for each protected permanent.
- G-DSL-SECURITY-EOT-PLAY-AND-PLACE-SELF-UNDER — "[Security] [End of Your Turn]
  play 1 [Ceresmon] reduced by 7; then may place this card as the played
  Digimon's bottom digivolution card." Needs a security-resident EOT trigger +
  play-from-hand-reduced bound result + place-self-as-bottom-source.
- Expressible (not shipped because the card is blocked overall): inherited
  [Opponent's Turn][OPT] redirect attack to a suspended own Digimon
  (Deramon idiom); [On Deletion] place self face-up as bottom security
  (place_self_at_security).

### BT25-020 Marsmon
- G-DSL-BATTLE-WINNER-BOARDWIDE — "[All Turns] [OPT] when any of your [TS]
  trait Digimon win a battle, trash your opponent's top security card." No
  board-wide battle-winner predicate (`source_deleted_battle_opponent` is
  carrier-only; `event_target_*` on `on_any_deletion` describes the loser).
- Expressible (not shipped): mandatory cost reduction (-5 if a Digimon with
  13000+ DP exists); [OP][WD][WA] +3000 DP to 1 own Digimon then 1 own may
  battle 1 opp.
