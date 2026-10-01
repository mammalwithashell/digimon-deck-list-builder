# Archetype DSL Implementation: EX13 — Mutant
Date: 2026-10-01
Total cards in pool: 6
Processed this run: 6
Pipeline: batch-implement-cards-rust-dsl (author-set EX13 slice)

## Summary
- IMPLEMENTED: 4
- PARTIAL: 0
- AUDITED-OK: 0
- AUDITED-MISSING-TESTS: 0
- AUDITED-DRIFT: 0
- BLOCKED (engine): 1
- BLOCKED (dsl): 1
- BLOCKED (hybrid): 0
- SKIPPED (prior verdict): 0

None of the six cards has a DCGO script at the submodule (b9a0638cd); the
official Bandai DB bundles (`data/card_bundles/`) and `general_rule.pdf` were
the references. Per-card JSONs were added from `data/cards.json` +
`data/card_overrides.json`.

## Per-Card Verdicts
| Card ID | Name | Mode | Verdict | Review | Tests | Notes |
|---------|------|------|---------|--------|-------|-------|
| EX13-050 | Bokomon | IMPLEMENT | IMPLEMENTED | self-reviewed | 11/11 | `flood_gate` CannotGainMemoryExceptFromTamers on BOTH players (`target_player: any`, official Q&A); Tamer gains still allowed; inherited <Blocker> |
| EX13-053 | Thundermon | IMPLEMENT | IMPLEMENTED | self-reviewed | 15/15 | up to 3 chained optional trash picks → deck top in pick order; mandatory delete, cap 3 + returned; (Rule) name alias `Mamemon Thundermon` (DCGO EX12_041 idiom); inherited [On Deletion] De-Digivolve 1 |
| EX13-054 | Nanimon | IMPLEMENT | IMPLEMENTED | self-reviewed | 12/12 | [Security] play after battle (its [On Play] fires); [OP][OD] CannotAttackPlayer until the opponent's turn ends; inherited +1000 DP; Rule [Mutant] trait |
| EX13-031 | KingSukamon | IMPLEMENT | BLOCKED (dsl) | — | 0 | timed ChangeBaseCardName / ChangeBaseCardColor need an `add_modifier` payload (G-DSL-ADD-MODIFIER-NAME-COLOR-PAYLOAD); engine has the modifiers |
| EX13-059 | BigMamemon | IMPLEMENT | IMPLEMENTED | self-reviewed | 19/19 | [WD][OD] reveal 3 → free cost<=7 [Mamemon]-name/[Mutant] Digimon; [EoYT][OPT] optional "by deleting a [Mamemon]" → delete 1 lowest play cost (ties = choice); inherited → delete ALL lowest |
| EX13-063 | PrinceMamemon | IMPLEMENT | BLOCKED (engine) | self-reviewed | 17/19 (2 ignored) | G-ASSEMBLY-NO-DISTINCT-BY: plain Assembly accepts same-name copies for "w/different names". All other clauses green (reveal/free play cost<=10, [On Deletion] delete highest, <Blocker>+<Guard> aura incl. a granted-Guard save) |

## Engine-Gap Blocked Cards
### EX13-063 PrinceMamemon
- Effect text: "Assembly -4: 3 Lv.5 or lower [Mamemon] text cards w/different names"
- Missing engine API: `distinct_by` enforcement on the plain `kind: assembly` path (`resolve_eligible_assembly` / `assembly_can_fulfill` / `install_assembly_element`)
- Suggested addition: thread `distinct_by` per element into `select_count_capped_multi_min` + a name-distinct SDR, or route `kind: assembly` through the `cast_time_assembly` DigiXros transaction (see `docs/RUST_ENGINE_GAPS.md` §G-ASSEMBLY-NO-DISTINCT-BY)

## DSL-Vocab-Gap Blocked Cards
### EX13-031 KingSukamon
- Effect text: "... you may change the base name, color and DP of 1 of your opponent's Digimon to [Sukamon], white and 3000 until their turn ends."
- Missing DSL verb: `add_modifier` payload for `ChangeBaseCardName` (Name) / `ChangeBaseCardColor` (Colors)
- Lowers to engine API: `ModifierType::ChangeBaseCardName` / `ChangeBaseCardColor` (typed `ModifierPayload`, consulted by `Permanent::synth_identity`)
- Suggested DSL syntax: `add_modifier: { target: victim, modifier: ChangeBaseCardName, name: Sukamon, expiry: end_of_opponents_turn }` (+ `colors: [white]`)

## New Patterns Discovered
- "Up to N, to the top of the deck, player-ordered": chain N optional `select_union_zone` picks, each nested under `if: binding_present` of the previous and immediately `return_union_bound_to_deck: top`; branch the follow-up cap on which bindings are present (EX13-053).
- `end_turn()` at non-negative memory does not rotate the turn after an [End of Your Turn] prompt resolves; drive EoYT tests with `pass_turn()` (EX13-059).
