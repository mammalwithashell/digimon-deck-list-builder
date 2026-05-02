# BG Imperial Rust DSL Readiness Assessment

Date: 2026-04-28

Assessment workflow: `.codex/skills/assess-rust-engine-archetype/`

Target: `data/deck_library.json` archetype `BG Imperial`, using the two DigimonMeta 1st-place lists dated 2026-01-25 and 2026-02-14. The combined card pool has 25 unique card IDs.

## Verdict

`blocked`

BG Imperial is not currently implementable faithfully as executable Rust YAML DSL. The card metadata exists in `data/cards.json`, but no production YAML specs exist under `code/digimon-engine/cards/**/` for the assessed card IDs. More importantly, several core effects require missing DSL/data or engine behavior:

- DNA digivolve cost authoring/data population for the normal DNA action mask.
- Inherited end-of-turn DNA digivolve registration.
- Partition source-list enforcement and source selection in a replacement window.
- Delay effects that act as deletion-prevention replacements.
- Start-of-turn Delay activation for Green Scramble.

Older `qa/archetype-qa/bg-imperial.md` notes are Python-era QA and should not be read as Rust DSL readiness.

## Card Pool

| Card | Required behavior | Status | Evidence | Gap / next step |
|---|---|---|---|---|
| `BT12-002` DemiVeemon | Inherited once-per-turn attack draw when green Digimon is in play | data-gap / test-gap | `data/cards.json`; no matching YAML under `code/digimon-engine/cards/` | Author YAML and add inherited behavioral coverage |
| `BT3-002` DemiVeemon | Inherited attack draw when this Digimon has Jamming | data-gap / test-gap | `data/cards.json`; `modifier_map.rs` has `Jamming` | Author YAML and add inherited condition coverage |
| `P-117` Veemon | Free-trait digivolution cost reduction when a Tamer is present; inherited two-color attack draw | dsl-gap / test-gap | `data/cards.json`; `docs/RUST_PYTHON_PARITY.md` notes context-aware cost hooks still have gaps | Add target-card-aware cost-reduction coverage before scripting |
| `BT12-021` Veemon | Search top 3; inherited end-of-turn DNA digivolve from hand | dsl-gap | `qa/dsl-vocab-gaps.md` inherited end-of-turn DNA entry | Reuse and close inherited `alt_path_registration` lowering gap |
| `BT12-047` Wormmon | Search top 3; inherited end-of-turn DNA digivolve from hand | dsl-gap | `qa/dsl-vocab-gaps.md` inherited end-of-turn DNA entry | Reuse and close inherited `alt_path_registration` lowering gap |
| `BT16-040` Wormmon | Start-main/on-play digivolve into Lv.4 Insectoid/Free from trash with reduced cost | test-gap | `data/cards.json`; no YAML | Needs effect-initiated digivolve-from-trash test |
| `ST9-09` Stingmon | Hand play-cost reduction when blue Digimon is in play; inherited attack draw | test-gap | `data/cards.json`; no YAML | Needs cost-reduction and inherited draw tests |
| `EX1-014` ExVeemon | Jamming; inherited grants Jamming while Imperialdramon/Free | test-gap | `modifier_map.rs` maps `Jamming`; no YAML | Needs inherited keyword-grant coverage |
| `BT12-022` ExVeemon | Gain memory when this would DNA digivolve into green; inherited Jamming aura | dsl-gap / test-gap | `data/cards.json`; DNA path blocked by `dna_costs` data | Needs DNA event/cost data before the trigger can matter |
| `BT12-050` Stingmon | Gain memory when this would DNA digivolve into blue; inherited Piercing aura | dsl-gap / test-gap | `modifier_map.rs` maps `Piercing`; DNA path blocked by `dna_costs` data | Needs DNA event/cost data before the trigger can matter |
| `BT21-037` Lighdramon | Piercing, Armor Purge, suspend target, DP buff | test-gap | `modifier_map.rs` maps `Piercing` and `ArmorPurge`; no YAML | Author YAML and verify Armor Purge replacement behavior |
| `ST9-05` Paildramon | DNA Lv.4 blue + Lv.4 green, WD bottom-deck if DNA, WA unsuspend | dsl-gap / data-gap | `docs/RUST_PYTHON_PARITY.md` 4.5b and known DSL schema gap | Add DNA cost data/authoring, then card test |
| `BT12-028` Paildramon | DNA Lv.4 blue + Lv.4 green, trash sources, DNA-gated attack lock | dsl-gap / data-gap | `docs/RUST_PYTHON_PARITY.md` 4.5b and known DSL schema gap | Add DNA cost data/authoring, then card test |
| `BT16-025` Paildramon | DNA path, Partition, WD suspend by source count, DNA-gated unsuspend lock, WA suspend/unsuspend | engine-gap / dsl-gap | `lower_partition.rs` accepts but ignores `sources`; `docs/RUST_PYTHON_PARITY.md` notes Partition replacement work | Implement partition source enforcement and DNA cost authoring |
| `ST9-06` Imperialdramon Dragon Mode | WD play one blue Lv.4-or-lower and one green Lv.4-or-lower from sources | test-gap | `data/cards.json`; no YAML | Needs source selection and play-from-sources test |
| `BT12-031` Imperialdramon: Fighter Mode | WD suspend/return; alternate bottom-deck if returning Dragon Mode source; dynamic DP/security/blocker by source colors | dsl-gap / test-gap | `data/cards.json`; no YAML | Needs source-return cost and dynamic source-color formula coverage |
| `BT16-027` Imperialdramon: Fighter Mode | Blast Digivolve, OP/WD bottom-deck by source count, EOA unsuspend and source-gated bottom-deck | test-gap | `docs/RUST_PYTHON_PARITY.md` says blast support landed; no YAML | Author YAML and add BG Imperial blast regression |
| `BT16-028` Imperialdramon: Dragon Mode | WD unsuspend lock on Digimon/Tamer; suspend them to unsuspend own; all-turns free digivolve when opponent effect plays/digivolves | dsl-gap / test-gap | `data/cards.json`; no YAML | Needs event predicate for opponent effect plays/digivolves and optional digivolve from hand |
| `BT20-020` Imperialdramon: Fighter Mode | Raid/Piercing, effect-play lock, Dragon Mode source check, security-removal observer delete | test-gap | `modifier_map.rs` maps `Raid` and `Piercing`; no YAML | Author YAML after source-name and security-removal observer coverage |
| `BT17-077` Imperialdramon: Paladin Mode | Blast Digivolve, trash all opponent sources, return trash to deck, white Lv.7 memory rider, WA bottom-deck cost to unsuspend | test-gap | `docs/RUST_PYTHON_PARITY.md` says blast support landed; no YAML | Author YAML and add trash-return rider tests |
| `BT3-093` Davis Motomiya | Start-turn memory setter; search blue and green Digimon; security play | test-gap | `data/cards.json`; no YAML | Author Tamer YAML and search/security tests |
| `BT16-085` Davis Motomiya & Ken Ichijoji | Start-main optional play Veemon/Wormmon with delayed return; suspend for memory on blue/green digivolve; DNA rider trash 3 sources | dsl-gap / test-gap | `data/cards.json`; no YAML | Needs delayed return scheduling and DNA-specific rider coverage |
| `BT3-103` Hidden Potential Discovered! | One-shot next green digivolution cost reduction by suspending own Digimon | dsl-gap / test-gap | `data/cards.json`; no YAML | Needs one-shot cost hook expressed in YAML |
| `LM-030` Green Scramble | Main reduced digivolve then place option; start-of-turn Delay; security play from trash and add self to hand | engine-gap / dsl-gap | `qa/archetype-qa/engine-gaps.md` `G-DELAY-START-OF-TURN`; `G-ADD-OPTION-SELF-TO-HAND` | Add start-of-turn Delay trigger and pending-security-to-hand primitive |
| `BT17-097` Return to the Primogenitor | Main reduced Free digivolve then place option; all-turns Delay as deletion-prevention replacement | engine-gap / dsl-gap | `lower_delay.rs` only schedules Delay effects; no replacement activation path | Add Delay replacement window activation and effect digivolve/prevent deletion flow |

## Reusable Gaps

### DNA cost data and YAML authoring

- **Gap:** Production Rust card data cannot currently supply DNA digivolve costs from YAML or `cards.json` ingest.
- **Type:** `dsl-gap` / `data-gap`
- **Blocks:** `ST9-05`, `BT12-028`, `BT16-025`, and any BG Imperial card whose printed behavior depends on a successful DNA digivolve.
- **Why it matters:** The engine action mask can emit DNA actions only when the hand card has populated `CardData.dna_costs`. Without that data, the legal player choice never appears.
- **Evidence:** `docs/RUST_PYTHON_PARITY.md` 4.5b says production ingest leaves `dna_costs = []`; the same doc records the known DSL schema gap that `CardSpec` cannot author `dna_costs`. `code/digimon-dsl/src/spec.rs` has `CardSpec` fields for `alt_paths` and `effects`, but no `dna_costs` field.
- **First test:** Put `BT16-025` in hand with a blue Lv.4 and green Lv.4 in battle, enough memory, and assert the main action mask exposes DNA digivolve. Selecting the pair should produce a stack with both materials.
- **Implementation hint:** Add a YAML/data path into `CardData.dna_costs`, either directly on `CardSpec` or by lowering `alt_paths: kind: dna_digivolve` into runtime DNA cost data before action-mask evaluation.

### Inherited end-of-turn DNA digivolve registration

- **Gap:** Inherited clauses can spell an end-of-turn DNA path, but the lowering path does not register that alternate path as a player-visible action/pending selection.
- **Type:** `dsl-gap`
- **Blocks:** `BT12-021`, `BT12-047`
- **Why it matters:** The inherited effect is one of the archetype's main ways to DNA digivolve outside the normal main phase. Auto-resolving or omitting it would violate the no-approximations policy.
- **Evidence:** `qa/dsl-vocab-gaps.md` tracks the same missing `alt_path_registration` lowering for `BT22-008` / `BT22-017`; this assessment extends that reusable gap to the BG Imperial inherited Veemon/Wormmon pair.
- **First test:** With `BT12-021` under a Digimon and a valid partner on field, end the turn and assert a player-visible end-of-turn DNA action into a matching hand card is offered.
- **Implementation hint:** Lower inherited `alt_path_registration` clauses with `timing: end_of_your_turn` and `kind: dna_digivolve` into the same alternate-path/action-mask channel used by normal DNA digivolve.

### Partition source enforcement

- **Gap:** Partition currently grants a marker/keyword, but the configured source predicates are not enforced and no replacement-window source selection is installed.
- **Type:** `engine-gap` / `dsl-gap`
- **Blocks:** `BT16-025`
- **Why it matters:** `BT16-025` must play exactly one blue Lv.4 and one green Lv.4 from its sources when the leave-field condition is met, and it must not fire for battle or the controller's own effects.
- **Evidence:** `code/digimon-engine/src/dsl_cards/lower_partition.rs` says Phase 1 only grants `Keyword::Partition`; `active_when` and `sources` are accepted but ignored. `docs/RUST_PYTHON_PARITY.md` notes Partition needs nested `PendingSelection::Source` in a replacement window.
- **First test:** Build a `BT16-025` stack with one blue Lv.4 and one green Lv.4 source, delete it by an opponent effect, then assert the player may play exactly those source cards without paying costs and the original leave-field outcome is handled correctly.
- **Implementation hint:** Install a `WhenWouldLeaveBattleArea` replacement for Partition that validates source predicates, prompts source choices, and resumes the original removal only after the replacement decision.

### Delay as deletion-prevention replacement

- **Gap:** Delay effects are scheduled option activations, but `BT17-097` needs a Delay option to activate inside a deletion replacement window and prevent the deletion after a successful free digivolve.
- **Type:** `engine-gap` / `dsl-gap`
- **Blocks:** `BT17-097`
- **Why it matters:** The option is not a start/end-turn delayed trigger. It watches a Free trait Digimon that would be deleted, lets the player trash the option, performs an effect digivolve into Imperialdramon from hand, and prevents that deletion.
- **Evidence:** `code/digimon-engine/src/dsl_cards/lower_delay.rs` lowers Delay into `EffectTiming::DelayEffect` with `EndOfThisTurn` or `EndOfYourNextTurn`. There is no route for Delay activation inside `WhenWouldBeDeleted`.
- **First test:** With `BT17-097` in battle area and a Free trait Digimon about to be deleted by the opponent, assert the player can trash the option, choose an Imperialdramon in hand, digivolve without cost, and cancel the deletion.
- **Implementation hint:** Add a replacement-window Delay activation path, likely through a pending choice over eligible Delay options, then a reusable `EffectContext` helper that pays the trash cost, effect-digivolves the threatened permanent, and marks the deletion as prevented.

### Start-of-turn Delay for Scramble options

- **Gap:** Delay triggers cannot fire at the start of the controller's next turn.
- **Type:** `engine-gap` / `dsl-gap`
- **Blocks:** `LM-030`
- **Why it matters:** Green Scramble's Delay is a start-of-turn action, not an end-of-turn scheduled effect.
- **Evidence:** `qa/archetype-qa/engine-gaps.md` already tracks `G-DELAY-START-OF-TURN` for `LM-027`; the same mechanic applies to `LM-030`.
- **First test:** Place `LM-030` in battle area, advance to the controller's next start of turn with the opponent controlling a Digimon, and assert the Delay activation is offered before main phase actions.
- **Implementation hint:** Add `DelayTrigger::StartOfYourNextTurn`, lower a `start_of_your_turn` token to it, and fire it from the start-turn hook.

## Implementation Order

1. Close DNA cost authoring/data population, because it unlocks the normal DNA action surface.
2. Close inherited end-of-turn DNA registration for the Lv.3 searchers.
3. Implement Partition source enforcement for `BT16-025`.
4. Implement Delay replacement activation for `BT17-097`.
5. Add start-of-turn Delay and pending-security-to-hand support for `LM-030`.
6. Author card YAML and TDD coverage card by card, treating old Python QA as behavioral reference only.
