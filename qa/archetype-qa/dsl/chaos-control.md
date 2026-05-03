# Chaos Control Rust DSL / Engine Gap Assessment

Date: 2026-05-03

Assessment workflow: `.codex/skills/assess-rust-engine-archetype/`

Target: `data/deck_library.json` archetype `Chaos Control`, using the local `digimoncard_io` list. The list has 24 unique card IDs and is a purple trash / low-hand-size control shell around Guilmon, Megidramon, ChaosGallantmon, delayed Options, trash recursion, and effect-initiated digivolution from trash.

This document is an archetype-level gap inventory for compiling a later cross-archetype DSL / engine gap spec. It separates reusable DSL or engine gaps from ordinary card-authoring work. Old Python QA in `qa/archetype-qa/chaos_control.md` remains useful as behavioral reference only; it is not Rust DSL readiness evidence.

## Verdict

`blocked`

Chaos Control is not currently ready as executable Rust DSL. Several reusable primitives landed on 2026-05-02, including non-hand effect digivolution, source/security movement slices, Delay timing slices, player-scoped modifiers, dynamic formulas, and keyword/aura improvements. The remaining blocker is now mostly card authoring plus a small set of cross-archetype DSL / engine gaps that should be specified once and reused.

The biggest concrete issue is coverage: 23 of the 24 card IDs in the archetype list have no YAML under `code/digimon-engine/cards/**/`. `BT7-107` exists only in `_examples/`, and its security clause still points at an unregistered raw-rust function instead of the native `add_this_option_to_hand` step.

## Card Pool

| Card | Required behavior | Rust DSL status | Reusable gap / next step |
|---|---|---|---|
| `EX11-005` Yaamon | Inherited start-main digivolve host into Dark Dragon / Evil Dragon from trash, cost -1; if successful, trash 2 from hand | card-yaml / test-gap | Author YAML using source-zone-parametric `effect_initiated_digivolve`; add success/failure ordering tests |
| `BT24-066` Guilmon | Reveal top 3; choose add and trash among Evil / Dark Dragon / Evil Dragon / Dark Knight / purple Tamer; trash 1 hand; inherited delete level 3 | card-yaml / test-gap | Needs two-pass reveal selection coverage and hand-trash ordering |
| `EX11-047` Impmon | Start-main trash 1 hand, then gain 1 memory; inherited +2000 DP | card-yaml / test-gap | Straightforward after hand-trash cost/order test |
| `BT24-070` Growlmon | If hand <=4, optional play purple Tamer cost <=4 from trash; inherited delete level 3 | card-yaml / test-gap | Needs play-from-trash filter by card kind/color/play cost |
| `BT20-069` Punkmon | Trash hand, grant Blocker + Retaliation to selected own Digimon until opponent turn ends | card-yaml / test-gap | Uses existing keyword grants; needs expiry/target tests |
| `BT24-076` WarGrowlmon | Trash-main play self cost -2 if hand <=4; on play/when digivolving delete level <=4; inherited on deletion play level <=4 Dark Dragon / Evil Dragon from trash | card-yaml / test-gap | Needs trash-main mask, self-play cost override, and inherited play-from-trash coverage |
| `BT24-080` Megidramon | Also treated as ChaosGallantmon; trash EOT effect-digivolve a Dark Dragon / Evil Dragon into this from trash for free if hand <=4; Blocker; delete all opponent lowest-level Digimon on play/digivolve/deletion | card-yaml / dsl-gap / test-gap | Lowest-level predicate exists, but card-specific delete-all flow and name-alias coverage need YAML/tests |
| `EX4-011` ChaosGallantmon | Trash EOT delete own Gallantmon with source as cost to play self free; on play delete opponent by DP cap 7000 + 2000 per 10 total trash | card-yaml / test-gap | Shared-trash formula primitive is resolved; needs cost-before-play and DP formula tests |
| `EX7-060` Nidhoggmon | Trash-main optional play self from trash cost -4 if hand <=4; Blocker; on deletion play level <=5 Dark Dragon / Evil Dragon from trash | card-yaml / test-gap | Needs optional trash-main play and filtered trash play |
| `EX3-072` Megiddo Flame | Main delete level <=4, or by deleting own Digimon delete level <=6 instead; security play Guilmon from trash | card-yaml / test-gap | Needs branch choice / cost-paid upgraded branch |
| `BT20-096` Black Sabbath | Trash-main pay 6, return self to deck bottom, delete unsuspended opponent Digimon; main trash hand then delete level <=4; security delete level <=6 | card-yaml / test-gap | Needs trash-main Option activation with self-return cost |
| `BT21-100` The Digimon I Designed | Ignore color with Takato; main draw/trash/place self; delayed trigger when effects delete Digimon; Delay digivolves Guilmon/Growlmon-family Digimon into Growlmon/Gallantmon/Megidramon from trash for free; security memory + place self | card-yaml / test-gap | Uses resolved Delay and non-hand effect-digivolve primitives; needs trigger timing and placement-turn gating tests |
| `BT7-107` Calling From the Darkness | Main delete own Digimon, then return up to 2 purple Digimon from trash; security add self to hand | partial / dsl-gap | Move YAML from `_examples` or duplicate into production set; replace unregistered raw-rust security shim with `add_this_option_to_hand` |
| `ST10-15` Darkness Wave | Main trash top 3; if yellow Digimon in play, return yellow or purple Digimon from trash; security activate main | card-yaml / test-gap | Needs activate-main-from-security and conditional trash recursion |
| `EX11-069` Yuuki | On play/start-main optional trash hand for memory; attack trigger if hand <=4 effect-digivolve attacker into Dark Dragon / Evil Dragon from trash cost -1; end all turns suspend to recur trait card from trash | card-yaml / test-gap | Uses effect-digivolve-from-trash; needs attack-trigger action/pending flow and end-all-turn optional suspend cost |
| `ST6-14` Matt Ishida | When own Digimon deleted, optional suspend for memory; security play | card-yaml / test-gap | Needs tamer event observer and suspend-as-cost |
| `EX1-066` Analog Youth | On play reveal top 3, add Digimon, trash rest; deletion observer for level >=5 Digimon with sources: suspend, gain memory, hatch | card-yaml / test-gap | Needs hatch helper and event-target/source-count predicates in card coverage |
| `EX7-056` Orochimon | Blocker; on deletion trash hand then delete opponent level 3 and level 4; inherited Retaliation | card-yaml / test-gap | Straightforward if multiple independent target deletes are authored |
| `EX7-053` Eyesmon: Scatter Mode | On play trash hand then optional return trait Digimon from trash; inherited Retaliation | card-yaml / test-gap | Needs filtered optional trash-to-hand |
| `EX11-050` Loudmon | Trash 2 hand, select own Dark Dragon / Evil Dragon as DP reference, delete opponent Digimon with DP <= reference; aura grants Scapegoat to trait Digimon while hand <=4; inherited Security A +1 aura while hand <=4 | card-yaml / engine-gap / test-gap | Scapegoat is a reusable replacement/keyword capability; dynamic target-DP reference needs card tests |
| `ST16-14` Matt Ishida | Start-turn memory set; when hand trashed by own effect, suspend for memory; security play | card-yaml / test-gap | Needs hand-trash event observer with cause/controller |
| `P-205` Insane Synthetic Monster | Ignore color with DM; main draw 2/trash 2/place self; Delay deletes own cost <=7 Digimon to play Kimeramon/Millenniummon from trash cost -3; security same draw/trash/place self | card-yaml / test-gap | Uses resolved Delay placement and play-from-trash cost override; needs cost target and security placement tests |
| `EX4-006` Guilmon | On play gains Rush if total trashes >=20 | card-yaml / test-gap | Uses shared trash count predicate/formula; needs temporary keyword grant coverage |
| `EX10-040` DemiDevimon | Start-main if opponent trash <=10, trash top 2 of both decks, then if opponent trash >=10 gain memory; inherited attack mills both players | card-yaml / test-gap | Needs opponent trash-count predicates and deck-mill ordering |

## Reusable Cross-Archetype Gaps

### G-CHAOS-DELETE-ALL-AGGREGATE

- **Gap:** Declarative "delete all matching permanents" authoring pattern needs a tested DSL shape for aggregate predicates such as lowest level.
- **Type:** `dsl-gap` / card-yaml gap
- **Tracker:** `qa/dsl-vocab-gaps.md`
- **Blocks:** `BT24-080`; also any "delete all lowest/highest X" effects in other archetypes.
- **Why it matters:** `BT24-080` must delete every opponent Digimon tied for the lowest level. A single target selection would hide mandatory choices and produce wrong board states.
- **Evidence:** `qa/dsl-vocab-gaps.md` marks the reusable lowest-level predicate as partially resolved, but says card-specific authoring still needs to wire the aggregate predicate through the surrounding delete-all flow.
- **First test:** `BT24-080` on play with opponent level 3, level 3, and level 4 Digimon deletes both level 3 Digimon and leaves level 4.
- **Implementation hint:** Prefer a `for_each` over opponent Digimon filtered by `level_matches_aggregate: lowest_level` plus `delete_permanent`, or add a native `delete_all` step if the pattern repeats enough to justify vocabulary.

### G-CHAOS-EFFECT-DIGIVOLVE-FROM-TRASH-CARD-COVERAGE

- **Gap:** The reusable non-hand effect-digivolve primitive is implemented, but Chaos needs card-specific YAML patterns and regression coverage for inherited, Tamer-triggered, Delay-triggered, attack-triggered, and trash-EOT variants.
- **Type:** `test-gap` / card-yaml gap
- **Tracker:** none for engine; card coverage belongs under `code/digimon-engine/tests/cards_behavioral/**`
- **Blocks:** `EX11-005`, `EX11-069`, `BT21-100`, `BT24-080`
- **Why it matters:** These effects are core to the archetype and differ in trigger timing, target/source selection, cost reduction, free digivolution, and post-success hand-trash follow-up.
- **Evidence:** `qa/dsl-vocab-gaps.md` and `docs/RUST_ENGINE_GAPS.md` mark non-hand source-zone effect digivolution resolved in Group 4.
- **First test:** `EX11-005` inherited effect selects a trash `BT24-080`, grows the host stack, spends reduced memory, and only then trashes two cards from hand.
- **Implementation hint:** Use `effect_initiated_digivolve` with a `source:` binding from `select_trash`; keep any "if this effect digivolve" continuation behind success.

### G-CHAOS-DELAY-OPTION-PLACEMENT-AND-TRIGGER-COVERAGE

- **Gap:** Delay option placement and timing primitives exist, but Chaos needs reusable examples for "place this card in battle area" from main/security and for non-end-turn trigger conditions tied to effect deletion events.
- **Type:** `test-gap` / possible `dsl-gap` if deletion-event Delay trigger cannot be expressed
- **Tracker:** `docs/RUST_ENGINE_GAPS.md` if a new Delay trigger variant is needed; otherwise card tests only
- **Blocks:** `BT21-100`, `P-205`
- **Why it matters:** Both Options persist in the battle area and activate later. Auto-trashing them after resolution or firing on the wrong turn loses the actual player-visible Delay choice.
- **Evidence:** `docs/RUST_ENGINE_GAPS.md` records Group 5 Delay timing resolution, but the Chaos cards are not authored.
- **First test:** `BT21-100` main places itself as a delayed Option and does not allow its Delay on the placement turn; after an effect deletes a Digimon, it offers a valid trash digivolve activation.
- **Implementation hint:** Reuse `kind: delay`, `place_self_as_delay_option`, and source-zone `effect_initiated_digivolve`. If deletion-event Delay triggers are not expressible, write the first failing test before expanding `DelayTrigger`.

### G-CHAOS-TRASH-MAIN-OPTION-ACTION

- **Gap:** Trash-zone main activations need consistent DSL authoring and action-mask coverage for cards that can be used from trash with explicit costs and self-disposition.
- **Type:** `test-gap` / possible `engine-gap`
- **Tracker:** `docs/RUST_ENGINE_GAPS.md` only if the action mask cannot expose the activation
- **Blocks:** `BT24-076`, `EX7-060`, `BT20-096`
- **Why it matters:** The archetype depends on low-hand-size trash activations. These must be legal actions surfaced from trash, not scripted auto-actions.
- **Evidence:** older Python QA lists Trash Main behavior as fixed, but there is no Rust YAML for these cards.
- **First test:** With `BT20-096` in trash, hand size <=4, and enough memory, assert the action mask exposes its trash-main activation; resolving it pays 6, returns the Option to deck bottom, and deletes an unsuspended opponent Digimon.
- **Implementation hint:** Reuse existing trash-main action range and DSL trigger forms; add card-level regression before treating the pattern as ready.

### G-CHAOS-BRANCH-CHOICE-WITH-COST-UPGRADE

- **Gap:** Option effects that present a branch where one branch pays a cost to upgrade the removal ceiling need a reusable DSL pattern with explicit player choice.
- **Type:** `dsl-gap` / test-gap
- **Tracker:** `qa/dsl-vocab-gaps.md` if current `select_effect_choice` cannot express the branch cleanly
- **Blocks:** `EX3-072`
- **Why it matters:** `EX3-072` can delete level <=4 by default, or by deleting one of your Digimon, delete level <=6 instead. The player must choose whether to pay the cost and which own Digimon to delete.
- **Evidence:** Chaos old QA says Megiddo Flame was faithful in Python; no Rust YAML exists.
- **First test:** With one own Digimon and opponent level 5/6 targets, `EX3-072` offers the upgraded branch; declining keeps only level <=4 targets legal.
- **Implementation hint:** Use `select_effect_choice` with separate process bodies. The cost branch should install `select_own_permanent` before the upgraded opponent selection.

### G-CHAOS-SCAPEGOAT-KEYWORD-REPLACEMENT

- **Gap:** Scapegoat needs a reusable keyword / replacement implementation that filters "would be deleted other than by your effects" and prompts the controller to delete another Digimon to prevent that deletion.
- **Type:** `engine-gap` / `dsl-gap`
- **Tracker:** `docs/RUST_ENGINE_GAPS.md`
- **Blocks:** `EX11-050`; also future cards granting Scapegoat-like prevention.
- **Why it matters:** `EX11-050` grants Scapegoat to all own Dark Dragon / Evil Dragon Digimon while hand <=4. This is a real replacement choice with a cost, not an aura-only keyword label.
- **Evidence:** No Chaos YAML exists; replacement/cost infrastructure has improved, but a dedicated Scapegoat route is not evidenced by local Chaos coverage.
- **First test:** With `EX11-050` active and hand <=4, opponent effect would delete a trait Digimon. The mask should offer Scapegoat; accepting deletes another own Digimon and prevents the original deletion. Own-effect deletion should not offer it.
- **Implementation hint:** Implement as a granted replacement effect or native keyword effect using replacement cause/controller predicates and `select_own_permanent` as a cost.

### G-CHAOS-HAND-TRASH-EVENT-OBSERVERS

- **Gap:** Effects that trigger when cards are trashed from hand by your effects need cause/controller-aware hand-trash event payloads.
- **Type:** `engine-gap` / test-gap
- **Tracker:** `docs/RUST_ENGINE_GAPS.md` if the event payload is missing
- **Blocks:** `ST16-14`; supports several Chaos hand-management cards indirectly.
- **Why it matters:** `ST16-14` must trigger when the controller trashes a card in hand using one of their effects, and not for opponent effects, costs outside the effect text, or ordinary discard-like moves.
- **Evidence:** The archetype has many "trash card(s) in hand" effects, but no Rust YAML or behavioral test tying those events to `ST16-14`.
- **First test:** `EX11-047` start-main trashes a hand card while `ST16-14` is unsuspended; `ST16-14` may suspend to gain memory. Opponent-caused hand trash should not trigger it.
- **Implementation hint:** Ensure hand-trash helpers emit a structured event with cause player/source effect, then lower Tamer observer predicates against that event.

## Spec Compilation Notes

When compiling the cross-archetype DSL / engine gap spec, route findings this way:

- Put missing runtime primitives, action-mask surfaces, replacement windows, event payloads, and keyword implementations in `docs/RUST_ENGINE_GAPS.md`.
- Put missing YAML schema, predicates, step verbs, lowering routes, and reusable authoring vocabulary in `qa/dsl-vocab-gaps.md`.
- Treat missing Chaos production YAML as card-authoring backlog, not a reusable gap, unless at least one of the gaps above prevents faithful authoring.
- Do not expand `ACTION_SPACE_SIZE` or tensor contracts for these gaps without a separate action/tensor contract plan. The known Chaos blockers should fit current pending-selection and action-mask surfaces unless Scapegoat or trash-main activation proves otherwise.

## Suggested First Regression Batch

1. `EX11-005` inherited effect-digivolves from trash and gates the hand-trash continuation on success.
2. `BT24-080` deletes all tied lowest-level opponent Digimon on each printed trigger.
3. `BT21-100` places itself as a delayed Option, respects placement-turn gating, and activates after an effect deletion.
4. `BT20-096` exposes and resolves trash-main activation from trash.
5. `EX11-050` proves Scapegoat replacement behavior or files the missing engine primitive.

Passing those five tests would separate remaining bulk YAML work from true cross-archetype DSL / engine gaps.
