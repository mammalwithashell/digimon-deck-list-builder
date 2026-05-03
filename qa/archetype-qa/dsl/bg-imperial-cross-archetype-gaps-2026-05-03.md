# BG Imperial Cross-Archetype DSL / Engine Gap Source

Date: 2026-05-03

Assessment source: `data/deck_library.json` archetype `BG Imperial`, using the two DigimonMeta 1st-place lists dated 2026-01-25 and 2026-02-14. This document distills the BG Imperial readiness assessment into reusable DSL/engine work items that can feed a later cross-archetype implementation spec.

This is not a card implementation plan. BG Imperial still needs production YAML and card-level tests for nearly the whole 25-card pool. The purpose here is to separate card authoring backlog from remaining reusable DSL/engine capability gaps.

## Current Verdict

`blocked`

The archetype is blocked primarily by missing card YAML and card-specific tests, not by the original April 2026 reusable primitives. Several previously blocking primitives now have reusable coverage:

| Capability | Current state | Evidence |
|---|---|---|
| Top-level authored DNA `alt_paths` populate runtime `CardData.dna_costs` | resolved as reusable capability | `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dna_digivolve_user_action authored_dna_alt_path_makes_dna_action_legal_for_bt20_016` |
| Partition source requirements and replacement-window source selection | resolved as reusable engine capability | `cargo test --manifest-path code/digimon-engine/Cargo.toml --test replacements bt16_025_partition` |
| Delay-as-deletion-replacement | resolved as reusable engine capability; BT17-097 Delay replacement prompt/continuation verified 2026-05-03 | `cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow -- replacement_integration::bt17_097 --nocapture` |
| Start-of-your-next-turn Delay timing | resolved as reusable timing capability | `docs/RUST_ENGINE_GAPS.md` Group 5 closure notes |
| Source-parametric effect digivolve and selected security-to-hand | resolved as reusable movement capability | `docs/RUST_ENGINE_GAPS.md` Group 4 closure notes |

## Card Authoring Backlog, Not Cross-Archetype Gaps

These are needed to make BG Imperial playable but should not become new shared gap specs unless card implementation proves a missing primitive:

| Area | BG Imperial cards | Required local work |
|---|---|---|
| Lv.2/Lv.3/Lv.4 inherited draw and keyword effects | `BT3-002`, `BT12-002`, `P-117`, `EX1-014`, `ST9-09`, `BT12-022`, `BT12-050` | Author production YAML and card behavioral tests |
| Searcher rookies and inherited end-of-turn DNA | `BT12-021`, `BT12-047` | Author YAML using reveal/add and inherited DNA registration; add card tests |
| Normal DNA Lv.5s | `ST9-05`, `BT12-028`, `BT16-025` | Author DNA `alt_paths`, When Digivolving/Attacking clauses, and DNA-origin tests |
| Imperialdramon boss line | `ST9-06`, `BT12-031`, `BT16-027`, `BT16-028`, `BT20-020`, `BT17-077` | Author production YAML and card tests around source counts, source names, ACE/Blast, and security/trash movement |
| Tamers/options with mostly known primitives | `BT3-093`, `LM-030`, `BT17-097` | BT17-097 Delay replacement continuation is fixed/verified; security Tamer-play production authoring remains open until production YAML and card-level tests prove it. |

## Remaining Reusable Gap Candidates

### G-BG-01: One-shot next-digivolve cost reduction with a selected suspend cost

- **Type:** hybrid `engine-gap` / `dsl-gap`
- **Blocks:** `BT3-103` Hidden Potential Discovered!
- **Cross-archetype value:** Training options, memory boosts, and older green/yellow options often install "the next time one of your Digimon would digivolve this turn" reducers with a paid condition.
- **Printed behavior:** `[Main] For the turn, when one of your green Digimon would next digivolve, by suspending 1 of your Digimon, reduce the digivolution cost by 5.`
- **Missing capability:** A player-scoped, one-shot future digivolve modifier that fires at cost calculation time, prompts/pays a selectable suspend cost, verifies target eligibility, consumes itself only when used, and leaves decline visible through pending selection/action masks.
- **Why it matters:** A static cost modifier or unconditional reduction hides the printed "by suspending 1 of your Digimon" choice and can apply to the wrong digivolution.
- **Spec should cover:** modifier lifecycle, target-card and base-permanent predicates, selectable cost payment before memory payment, decline behavior, once-only consumption, and interaction with stacked reducers.
- **First test:** Play `BT3-103`, attempt a green digivolution with one unsuspended own Digimon available, assert a suspend-cost prompt appears before the reduced cost is paid; decline keeps the unreduced cost, accept suspends the selected Digimon and applies `-5` only once.
- **Likely files:** `code/digimon-engine/src/cost_hooks/`, `code/digimon-engine/src/effect_context/`, `code/digimon-engine/src/action/`, `code/digimon-dsl/src/step.rs`, `code/digimon-engine/src/dsl_cards/step/`.

### G-BG-02: Scheduled delayed return for a just-played permanent

- **Type:** hybrid `engine-gap` / `dsl-gap`
- **Blocks:** `BT16-085` Davis Motomiya & Ken Ichijoji
- **Cross-archetype value:** Many effects temporarily play a card and schedule it to return, delete, trash, or expire at a later phase.
- **Printed behavior:** `[Start of Your Main Phase] You may play 1 [Veemon] or [Wormmon] from your hand without paying the cost. At the next end of your opponent's turn, return it to the hand.`
- **Missing capability:** A delayed zone-change marker tied to the exact permanent created by an earlier effect, with owner routing, survival checks, and correct expiry at the next opponent end turn.
- **Why it matters:** Auto-returning by card name can hit the wrong permanent; omitting the delayed return gives a free permanent with no printed drawback.
- **Spec should cover:** binding the played permanent handle, preserving identity through battle-area shifts, no-op if the permanent left before expiry, owner hand routing, and interaction with replacement/prevention effects when the delayed return occurs.
- **First test:** Trigger `BT16-085`, choose a `Veemon` from hand, assert it enters battle area for free and receives a scheduled return marker; advance to the next opponent end turn and assert that same permanent returns to owner hand if still present.
- **Likely files:** `code/digimon-engine/src/game.rs`, `code/digimon-engine/src/effect_queue.rs`, `code/digimon-engine/src/effect_context/`, `code/digimon-dsl/src/step.rs`, `code/digimon-engine/src/dsl_cards/lower_delay.rs`.

### G-BG-03: DNA-origin event context and rider gating

- **Type:** hybrid `engine-gap` / `dsl-gap`
- **Blocks:** `BT12-022`, `BT12-050`, `BT12-028`, `BT16-025`, `BT16-085`
- **Cross-archetype value:** DNA Omnimon and other DNA decks need effects that distinguish normal digivolution from DNA digivolution and apply extra riders only when the event was DNA.
- **Printed behavior examples:** `BT12-022` gains memory when it would DNA digivolve into green; `BT16-085` gains memory when a blue/green Digimon digivolves and trashes sources only if DNA digivolving.
- **Missing capability:** Stable event context exposing `is_dna_digivolve`, DNA materials, resulting permanent, and resulting card properties to triggered effects and inherited effects.
- **Why it matters:** Current code has signs of a DNA-origin field, but it is warned as unused in targeted tests. Card authoring needs proven runtime event context so DNA-only riders do not fire on normal digivolution or fail to fire after DNA.
- **Spec should cover:** event payload shape, inherited-source observer fan-out, timing order relative to draw bonus and memory payment, and predicates such as `event_is_dna_digivolve`, `event_card_color_has`, and `event_material_contains`.
- **First test:** Put `BT12-022` under one DNA material, DNA digivolve into a green card, and assert the memory gain fires; normal digivolve into the same green card must not trigger the DNA-only effect.
- **Likely files:** `code/digimon-engine/src/game.rs`, `code/digimon-engine/src/effect_queue.rs`, `code/digimon-engine/src/dsl_cards/lower_triggered.rs`, `code/digimon-dsl/src/predicate.rs`.

### G-BG-04: Source-count and source-name predicates in triggered selection filters

- **Type:** `dsl-gap`
- **Blocks:** `BT16-025`, `BT16-027`, `BT16-028`, `BT20-020`, `BT12-031`
- **Cross-archetype value:** Many boss Digimon compare source counts, check named sources, or scale effects based on source colors/names.
- **Printed behavior examples:** `BT16-027` returns an opponent's Digimon with as many or fewer digivolution cards as this Digimon; `BT20-020` checks whether `Imperialdramon: Dragon Mode` is in this Digimon's digivolution cards.
- **Missing capability:** Concise DSL predicates/formulas for source count comparisons against the effect source and for named-card/material presence in the source stack.
- **Why it matters:** Raw Rust or hand-coded card logic should not be needed for common source-count and source-name filters.
- **Spec should cover:** `source_count_lte_source`, `has_source_name`, `source_color_count`, and formula bindings that distinguish top card from inherited sources.
- **First test:** Author a DSL fixture where an opponent permanent with two sources is selectable only if the effect source has two or more sources, then verify one-source opponents and three-source opponents are filtered correctly.
- **Likely files:** `code/digimon-dsl/src/predicate.rs`, `code/digimon-dsl/src/spec.rs`, `code/digimon-engine/src/dsl_cards/step/`, `code/digimon-engine/tests/dsl/`.

### G-BG-05: Optional immediate attack generated by an effect

- **Type:** hybrid `engine-gap` / `dsl-gap`
- **Blocks:** `BT20-016` and may affect future BG Imperial tech; not in the two assessed BG Imperial lists, but appears in Imperialdramon-adjacent authored YAML.
- **Cross-archetype value:** Multiple modern cards say "this Digimon may attack" or "1 of your Digimon may attack" during effect resolution, sometimes even after digivolving or at negative memory.
- **Printed behavior:** `BT20-016` Paildramon: `[On Play] [When Digivolving] ... Then, this Digimon may attack.`
- **Missing capability:** A pending attack selection installed from effect resolution that uses the normal attack legality/mask machinery while preserving the effect-specific attacker and optional PASS.
- **Why it matters:** Omitting the attack is a fidelity loss; auto-attacking hides a player-visible choice.
- **Spec should cover:** player-visible optional attack prompt, target legality reuse, without-suspending variants, memory/turn-state legality, and interaction with "can't attack" modifiers.
- **First test:** Resolve a fixture effect granting "this Digimon may attack" with one valid opponent/player target and assert the action mask exposes PASS plus legal attack actions for that attacker only.
- **Likely files:** `code/digimon-engine/src/action/`, `code/digimon-engine/src/combat.rs`, `code/digimon-engine/src/effect_context/`, `code/digimon-dsl/src/step.rs`.

### G-BG-06: Hand/trash union-zone play or digivolve choices

- **Type:** `dsl-gap`
- **Blocks:** `BT17-097` security Tamer-play effect if authored literally; related to many option/security effects. This does not block the already verified BT17-097 Delay replacement continuation.
- **Cross-archetype value:** Many security and option effects let the player choose a card from hand or trash, then play/digivolve it.
- **Printed behavior:** `BT17-097` security effect plays 1 Tamer card with `Davis Motomiya` or `Ken Ichijoji` in its name from hand or trash, then places the option in battle area.
- **Missing capability:** A first-class union-zone selector that surfaces hand and trash candidates in one pending choice while preserving zone-specific movement semantics.
- **Why it matters:** Running separate hand-then-trash prompts changes choice ordering and can hide a legal card if the first prompt is declined incorrectly.
- **Spec should cover:** candidate identity includes zone, PASS behavior, ownership routing, and follow-up placement of the resolving option.
- **First test:** Put one valid Davis/Ken Tamer in hand and one in trash, resolve the security effect, and assert both candidates appear in the same pending selection.
- **Likely files:** `code/digimon-engine/src/selection.rs`, `code/digimon-engine/src/action/`, `code/digimon-engine/src/effect_context/`, `code/digimon-dsl/src/step.rs`.
- **Updated 2026-05-03:** Keep this open for BT17-097 security Tamer play. The passing `replacement_integration::bt17_097*` option-flow tests verify Delay replacement prompt/continuation only; this plan did not prove production security YAML or behavioral tests for the Tamer play clause.

## Cross-Archetype Spec Compile Notes

When compiling the next shared DSL/engine spec, do not include "missing BG Imperial YAML" as a reusable primitive. Treat the reusable gap candidates above as spec inputs and keep the card authoring backlog in a separate Group 9-style unlock pass.

Recommended ordering:

1. `G-BG-03` DNA-origin event context, because it validates whether current DNA plumbing is truly scriptable for both BG Imperial and DNA Omnimon.
2. `G-BG-01` one-shot future cost reducers, because it affects options beyond BG Imperial and interacts with cost hooks.
3. `G-BG-02` scheduled delayed return, because it introduces persistent effect markers tied to exact permanents.
4. `G-BG-04` source-count/source-name predicates, because it removes repeated raw-Rust pressure from boss card YAML.
5. `G-BG-06` union-zone selection, because it is broadly useful for security and option effects.
6. `G-BG-05` immediate effect attacks, because it is important but can be isolated behind attack-action reuse.

Acceptance criteria for any resulting spec:

- No `ACTION_SPACE_SIZE` or tensor contract changes unless the spec explicitly updates `docs/ACTION_SPEC.md`, `docs/TENSOR_SPEC.md`, Rust constants, PyO3 exports, RL wrappers, frontend constants, and model metadata together.
- Every player-visible decision must be surfaced through action masks or `PendingSelection`.
- Every reusable primitive must have at least one non-card fixture test and one card-shaped regression using a BG Imperial or adjacent card when possible.
- Tracker updates must distinguish `docs/RUST_ENGINE_GAPS.md` engine primitives from `qa/dsl-vocab-gaps.md` vocabulary/lowering work.
