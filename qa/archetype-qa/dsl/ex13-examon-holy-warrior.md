# Archetype DSL Implementation: EX13 — Examon / holy warrior (Royal Knights Assembly)
Date: 2026-10-01
Total cards in pool: 15
Processed this run: 15
Pipeline: batch-implement-cards-rust-dsl (author-set EX13 slice)

## Summary
- IMPLEMENTED: 11
- PARTIAL: 1
- AUDITED-OK: 0
- AUDITED-MISSING-TESTS: 0
- AUDITED-DRIFT: 0
- BLOCKED (engine): 1
- BLOCKED (dsl): 0
- BLOCKED (hybrid): 2
- SKIPPED (prior verdict): 0

## Per-Card Verdicts
| Card ID | Name | Mode | Verdict | Review | Tests | Notes |
|---------|------|------|---------|--------|-------|-------|
| EX13-001 | Gigimon | IMPLEMENT | IMPLEMENTED | self-reviewed | 10/10 | inherited [Your Turn][OPT] red Tamer played → digivolve into [Growlmon]/[Gallantmon] −2 |
| EX13-020 | Magnamon | IMPLEMENT | IMPLEMENTED | self-reviewed | 15/15 | Blocker/Armor Purge; +1000 per trash colour then `multiply(floor_div(DP,5000),−4000)`; face + inherited EoT unsuspend (OPT refund); Assembly −2 [Veemon] |
| EX13-014 | Jesmon | IMPLEMENT | IMPLEMENTED | self-reviewed | 13/13 | WD/WA union hand|sources free [Huckmon] Option; own-Digimon-played → may delete lowest DP, then yes/no Atho token |
| EX13-015 | Gallantmon | IMPLEMENT | IMPLEMENTED | self-reviewed | 15/15 | OP/WD/WA/Counter delete ≥12000 (DCGO filter inverted — printed text governs) else trash security; leave-replacement by deleting ≤9000 |
| EX13-023 | UlforceVeedramon | IMPLEMENT | IMPLEMENTED (2026-10-01) | self-reviewed | 12/12 | change orientation (OPT refund on decline); fewest-source deck bounce; opponent-scoped unsuspended protections via aura `modifier_from` (G-ENGINE-STACKED-CARD-RETURN-PROTECTION RESOLVED) |
| EX13-036 | Kentaurosmon | IMPLEMENT | IMPLEMENTED | self-reviewed | 12/12 | [Security][On Play] "instead" mass −7000; WD most-security trash → re-run the [Security] body; WD/EoA/Counter place 1 of each player's Digimon on top security |
| EX13-037 | Dynasmon | IMPLEMENT | IMPLEMENTED | self-reviewed | 11/11 | trash own security + 10000, ≤3 → trash theirs; security-removed observer −12000, ≤3 → Recovery (needed the 15-8-3-2 engine fix) |
| EX13-043 | Leopardmon | IMPLEMENT | IMPLEMENTED | self-reviewed | 13/13 | any-side suspend + lowest-DP bottom deck; play-or-use −(4+suspended); suspended-ally leave-replacement by unsuspending |
| EX13-060 | Alphamon | IMPLEMENT | IMPLEMENTED | self-reviewed | 16/16 | WD −8000, opponent ≥5 memory → +2; [Chronicle] played → attack + refire WD; EoT play [Chronicle] −6 + Rush |
| EX13-061 | Gankoomon | IMPLEMENT | IMPLEMENTED | self-reviewed | 12/12 | yes/no Hinukamuy + white Digimon opponent-Digimon immunity; white-suspend → free [Huckmon] Option; Assembly 3 distinct names (engine fix) |
| EX13-062 | Craniamon | IMPLEMENT | PARTIAL | self-reviewed | 15/15 | +3000 "when this unsuspends" fires for effect unsuspension only (phase/Reboot unsuspension gap) |
| EX13-064 | LordKnightmon | IMPLEMENT | IMPLEMENTED | self-reviewed | 14/14 | [Rie Kishibe] Tamer route ≤3 security (engine fix); WD union hand|trash play-or-use; [Knightmon]-text aura; Rush+Collision attack |
| EX13-016 | Omnimon | IMPLEMENT | IMPLEMENTED (2026-10-01) | self-reviewed | 11/11 | same-level pair save via `select_materials { min: 2, same_by: level }` (G-ENGINE-SAME-LEVEL-SOURCE-PAIR-SELECTION RESOLVED) |
| EX13-045 | Examon | IMPLEMENT | IMPLEMENTED | self-reviewed | 12/12 | DNA → mass +10000 + forced attack, then may battle; win-battle (incl. effect battles) → play/use [Dracomon]/[Examon]-text ≤12 free |
| EX13-077 | Omnimon: Merciful Mode | IMPLEMENT | IMPLEMENTED (2026-10-01) | self-reviewed | 20/20 | Assembly "w/different colors" via `distinct_by: color` (G-ASSEMBLY-DISTINCT-BY-COLOR RESOLVED) |

## Engine-Gap Blocked Cards
### EX13-023 UlforceVeedramon (engine) — RESOLVED 2026-10-01 (IMPLEMENTED)
- Effect text: "[All Turns] Your opponent's effects can't reduce this unsuspended Digimon's DP, return its stacked cards to the hand or deck, or trash them."
- Missing engine API: a "stacked cards can't be returned to hand/deck by opponent effects" modifier, plus a continuous aura form of opponent-scoped `ImmuneFromDPMinus` / `ImmuneFromStackTrashing` gated on "this Digimon is unsuspended".
- Suggested addition: `ModifierType::ImmuneFromStackReturn` + aura `modifier:` payload with an opponent `effect_immunity_filter`. See `qa/archetype-qa/engine-gaps.md` §G-ENGINE-STACKED-CARD-RETURN-PROTECTION.

### EX13-062 Craniamon (PARTIAL, engine)
- Effect text: "[All Turns] When this Digimon unsuspends, it gets +3000 DP until your turn ends."
- Missing engine API: unsuspend-phase / <Reboot> unsuspension firing `OnUnsuspend` (DCGO does, via `IUnsuspendPermanents`). See §G-ENGINE-PHASE-UNSUSPEND-NO-ONUNSUSPEND.

## DSL-Vocab-Gap Blocked Cards
### EX13-016 Omnimon (hybrid) — RESOLVED 2026-10-01 (IMPLEMENTED)
- Effect text: "[All Turns] When this Digimon would leave the battle area, by trashing 2 same-level cards from its digivolution cards, it doesn't leave."
- Missing DSL verb: `select_materials { same_by: level }`.
- Lowers to engine API: none yet — a SAME-key mode on the count-capped multi-pick (`DistinctByMode::SameLevel`).
- Suggested DSL syntax: `select_materials { of_permanent: replacement_subject, max: 2, same_by: level, bind_as: pair }` → trash → `cancel_replacement`.

### EX13-077 Omnimon: Merciful Mode (hybrid)
- Effect text: "Assembly -8: 6 [ADVENTURE] trait Digimon cards w/different colors".
- Missing DSL verb: `distinct_by: color`.
- Lowers to engine API: none yet — colour-distinct assignment (bipartite matching) in the Assembly SDR + pick mask.
- Suggested DSL syntax: `materials: [ { filter: { kind: digimon, trait_has: ADVENTURE }, repeat: {min: 6, max: 6}, distinct_by: color, zones: [trash], stack_under: true } ]`.

## Substrate widened this run
- DSL: `multiply` formula; `printed_keyword` predicate; `event_winner_is_source` predicate; `binding_card_kind` over union bindings; `select_any_permanent.continue_on_decline`.
- Engine: security-removed / lose-security / discard-security observers deferred inside a resolving effect (15-8-3-2); `<Evade>` candidate self-scope; Tamer-named digivolve route; plain Assembly `distinct_by`.

## New Patterns Discovered
- Mid-clause "you may" (e.g. "Then, you may play 1 token"): an `optional:` step only prompts when it LEADS a clause — use an explicit 2-label `select_effect_choice` instead.
- `select_own/opponent/any_permanent { optional: true }` DROPS the rest of the clause on decline unless `continue_on_decline: true` (needed for independent "Then" legs and for `refund_opt`).
- `DebugRunner` memory is clamped to 10: playing a cost-12+ card from hand passes the turn and expires "for the turn" modifiers — drive [On Play] with `enqueue_triggered` on a placed copy instead.
- To end a turn without the next player's turn auto-ending, `set_memory(-3)` before `end_turn()` (the opponent then starts with +3).
- "When this Digimon wins a battle" that must also cover effect battles: `on_ally_won_battle` + `event_winner_is_source: true`.
