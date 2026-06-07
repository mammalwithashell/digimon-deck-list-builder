# Archetype DSL Implementation: Titan (BT25 slice)
Date: 2026-06-06
Total cards in this slice: 8
Processed this run: 8
Pipeline: batch-implement-cards-rust-dsl

## Summary
- IMPLEMENTED: 4
- PARTIAL: 0
- AUDITED-OK: 0
- AUDITED-MISSING-TESTS: 0
- AUDITED-DRIFT: 0
- BLOCKED (engine): 1
- BLOCKED (dsl): 1
- BLOCKED (hybrid): 2
- SKIPPED (prior verdict): 0

## Per-Card Verdicts
| Card ID | Name | Mode | Verdict | Review | Tests | Notes |
|---------|------|------|---------|--------|-------|-------|
| BT25-006 | Dorimon | IMPLEMENT | IMPLEMENTED | self | 5/5 | Inherited [Opp Turn][OPT] on opp-attack: trash 1 hand card → unsuspend 1 [Titan] Digimon |
| BT25-068 | Deltamon | IMPLEMENT | IMPLEMENTED | self | 9/9 | <Collision>; [All Turns][OPT] on self-suspend De-Digivolve 1 opp; inherited +1000 DP; TS alt-digivolve |
| BT25-071 | Orochimon | IMPLEMENT | IMPLEMENTED | self | 9/9 | OnPlay/Digivolve lock 1 opp Digimon/Tamer (CannotAttack until their turn ends); [All Turns][OPT] on-suspend reveal-3 play-1 cost<=4 TS, rest to deck bottom (face-up + inherited) |
| BT25-019 | UltimateBrachiomon | IMPLEMENT | IMPLEMENTED | self | 12/12 | <Reboot>+<Blocker>; OnPlay/Digivolve delete highest-DP opp; EoT[OPT] memory-gated Digimon/Option effect immunity |
| BT25-069 | Raremon | IMPLEMENT | BLOCKED (dsl) | self | 0/0 | link 1 [TS] card from trash to a Digimon free — no select-trash-card-and-link verb |
| BT25-073 | Dragomon | IMPLEMENT | BLOCKED (hybrid) | self | 0/0 | trash a link card as cost + would-leave-by-trashing-link replacement |
| BT25-080 | Witchmon | IMPLEMENT | BLOCKED (engine) | self | 0/0 | inherited OnDiscardHand trigger + main-clause played_by_effect condition |
| BT25-083 | LadyDevimon | IMPLEMENT | BLOCKED (hybrid) | self | 0/0 | bottom-source-from-hand/trash picker + trash-digivolution-option-as-cost + cost-reduced trash-option use |

Note: this slice was executed by a single orchestrator agent (no fan-out sub-agents),
so each "Review" is a self-audit against the hybrid checklist; all 35 behavioral
tests for the four IMPLEMENTED cards pass.

## Engine-Gap Blocked Cards
### BT25-080 Witchmon
- Effect text (inherited): "[All Turns][Once Per Turn] When your hand is trashed from, if this Digimon has the [Titan] trait, delete 1 of your opponent's level 4 or lower Digimon."
- Effect text (main): "[On Play][When Attacking][Once Per Turn] By trashing 1 card in your hand, you may return 1 [Titan] trait card from your trash to the hand. After, if played by an effect, delete 1 of your opponent's level 5 or lower Digimon."
- Missing engine API: `OnDiscardHand` (hand→trash) trigger timing + dispatch; `played_by_effect` play-context condition.
- Logged: `docs/RUST_ENGINE_GAPS.md` → "`OnDiscardHand` … trigger timing + 'played by an effect' condition".

## DSL-Vocab-Gap Blocked Cards
### BT25-069 Raremon
- Effect text: "[On Play][When Digivolving] You may link 1 [TS] trait card from your trash to 1 of your Digimon without paying the cost."
- Missing DSL verb: select-a-trash-card-and-link-it-to-an-own-Digimon (the shipping `link_to_own_digimon` only links the carrier Option).
- Lowers to engine API: link host substrate exists; cross-ref `docs/RUST_ENGINE_GAPS.md` `[Link]` subsystem (alternate-source linking from trash).
- Logged: `qa/dsl-vocab-gaps.md` → "BT25 titan slice — BLOCKED cards / BT25-069".

### BT25-073 Dragomon  (hybrid)
- Effect text (main): "[On Play][When Digivolving] By trashing 1 of your Digimon's link cards, you may play or use 1 [TS] trait card with a play or use cost of 5 or less from your hand without paying the cost."
- Effect text (inherited): "[All Turns] When this Digimon would leave the battle area, by trashing 1 of its link cards, it doesn't leave."
- Missing DSL verb: select+trash a specific link card from a permanent as an activation cost / as a replacement cost (`from: linked_cards`).
- Cross-ref existing `G-DSL-LINK-TRASH-AS-REPLACEMENT-COST` (BT25-066) + `[Link]` subsystem in `docs/RUST_ENGINE_GAPS.md`.
- Logged: `qa/dsl-vocab-gaps.md` → "BT25 titan slice / BT25-073".

### BT25-083 LadyDevimon  (hybrid)
- Effect text: see card; place [Three Musketeers] card from hand/trash as bottom digivolution source → Draw 1; trash an Option from digivolution cards to use a [Three Musketeers] Option from trash with cost reduced by 3.
- Missing DSL verbs: hand|trash zone-choice bottom-source placement; trash-an-Option-from-digivolution-stack-as-cost; reduced-cost use-Option-from-trash.
- Logged: `qa/dsl-vocab-gaps.md` → "BT25 titan slice / BT25-083".

## New Patterns Discovered
- reveal-3 + `play_from_revealed_free` + `place_remainder_on_deck` (BT25-071): the remainder
  bottom-placement installs a `select_ordered_permutation` selection when >1 card remains; a
  positive reveal-play test must resolve that follow-up ordering selection (not just the bucket
  pick). Worth a note in RUST_DSL_TEST_API.md §5 alongside the EX8-050 trash-remainder variant.
