## Why

The Medusamon archetype DSL run finished with 47/54 cards `IMPLEMENTED` and 7 stuck at `PARTIAL`. None of the 7 are blocked on card-authoring — their YAML is already as faithful as the DSL allows. They are blocked on 7 engine/DSL **substrate** gaps logged in `qa/archetype-qa/engine-gaps.md` and `qa/dsl-vocab-gaps.md`. Closing the substrate is the only way to make these cards faithful, and several gaps also block cards in other sets, so the work pays for itself beyond Medusamon.

## What Changes

Five substrate gaps are committed work in this change (no RL action-space or observation-tensor impact):

- **G-SECURITY-SKILL-RESUME-REFIRE** (engine bug) — `combat.rs::drive_security_resolution`'s `SecuritySkillDrain` arm re-enqueues `SecuritySkill` on resume without recording the drain fired; declining an optional `[Security]` "you may" effect infinite-loops. Record drain-fired state so resume advances. Unblocks **P-189**; also fixes **P-206**, **ST19-08**.
- **G-ZONE-SELECTED-TRASH-TO-DECK-TOP** (engine + DSL) — all trash→deck `EffectContext` methods hard-code `deck.insert(0, …)` (bottom). Add `return_trash_cards_to_deck_top` plus a `destination: top|bottom` DSL parameter. Unblocks **LM-027**; also **LM-029/030/031**.
- **G-TRASH-SELECTED-SECURITY** (engine + DSL) — the engine has `trash_top_security` / `trash_bottom_security` but no arbitrary-index variant, and no DSL verb consumes a `select_security` binding. Add `trash_security_at_index` + a `trash_selected_security` DSL verb. Unblocks **BT24-018**.
- **G-ACTIVATION-COST-TRASH-SELF** (DSL vocab) — `activation_cost:` accepts only `suspend_self` / `return_self_to_deck_bottom`. Add a declinable `trash_self: true` variant so `<Delay>` "by trashing this card" is a true (declinable) activation cost per Comprehensive Rules 16-16-2. Unblocks **BT21-093**.
- **G-ALT-PATH-SAVE-IN-TEXT** (DSL vocab) — alt-path `from:` filters have no predicate for "source card has a keyword printed in its effect text". Add a `save_in_text` / `keyword_in_text` predicate leaf. Unblocks **BT21-072**.

Two further gaps are scoped here as a **design spike only** — both require a new action ID that moves `ACTION_SPACE_SIZE`, rippling into the observation tensor, mask, decoder, and both spec docs (working rules 1 & 4):

- **G-ACTIVATED-DIGIVOLVE-EXECUTION** — `CompiledAltPathKind::ActivatedDigivolve` has no engine execution route. Blocks **BT24-016**; also BT22-013, BT22-026, BT16-027.
- **G-LINK-OPTION-DUAL-PLAY-MODE** — `classify_option_subtype` is first-match-wins, so a Plug-In Option cannot be both a `[Main]` Option and a Link Option. Blocks **ST22-08**.

The spike produces a written action-space decision (reuse existing IDs vs. grow the space) and a follow-up proposal; it does **not** implement Tier 3 here.

## Capabilities

### New Capabilities
- `security-card-effects`: declinable `[Security]` triggered effects resolve to completion without re-firing on resume; effects can trash a chosen non-top security card.
- `trash-to-deck-top-return`: effects can return selected trash cards to the **top** of the owner's deck, not only the bottom.
- `dsl-card-scripting-vocabulary`: new DSL authoring vocabulary — a declinable `activation_cost: { trash_self: true }` and a `save_in_text` alt-path `from:` predicate.

### Modified Capabilities
<!-- None. The existing specs (dna-omnimon-archetype-coverage, dsl-inherited-substitute-trash) are unaffected at the requirement level. -->

## Impact

- **Engine** (`code/digimon-engine/`): `combat.rs` (security resolution state machine), `selection.rs` (resolution state field), `effect_context/mod.rs` (new `return_trash_cards_to_deck_top`, `trash_security_at_index`).
- **DSL crate** (`code/digimon-dsl/`): `step.rs` / `compile.rs` / `compiled.rs` — new `destination` zone-move param, `trash_selected_security` verb, `trash_self` activation-cost variant, `save_in_text` predicate leaf; matching lowering in `code/digimon-engine/src/dsl_cards/`.
- **No RL-contract impact** for the committed Tier 1+2 work: `ACTION_SPACE_SIZE` and `TENSOR_SIZE` are unchanged; no new pending-selection masks.
- **Cards**: re-running `/batch-implement-cards-rust-dsl Medusamon` after this change moves 5 of the 7 `PARTIAL` cards to `IMPLEMENTED` and also unblocks LM-029/030/031, P-206, ST19-08.
- **Trackers**: `qa/archetype-qa/engine-gaps.md` and `qa/dsl-vocab-gaps.md` entries for the 5 closed gaps move to `qa/resolved-gaps.md`.
- **Out of scope**: Tier 3 (G-ACTIVATED-DIGIVOLVE-EXECUTION, G-LINK-OPTION-DUAL-PLAY-MODE) implementation — spike output only.
