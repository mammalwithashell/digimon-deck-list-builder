## Why

The second shape of DSL sprawl (the first being type-sprawl, handled by `unify-dsl-scalar-and-comparators`) is **step-sprawl**: the same multi-step authoring idiom is re-spelled in every card instead of being a single verb. The audit quantified four cases, all verified against source:

**1. The `select → act` pair is the dominant authoring shape, written longhand everywhere.** select+delete appears in 86 cards, select+suspend in 79, select+discard in 34, select-from-trash+add-to-hand in 14 — each a two-step `select_* { bind_as: t }` then `delete_permanent { target: t }` pair. The engine **already** supports an inline `then: Vec<StepSpec>` tail — it ships on **seven** select structs today (`SelectOwnSources`, `SelectOpponentSources`, `DigiBurst`, `SelectOpponentDpBudget`, `SelectOpponentPlayCostBudget`, `SelectOwnBreedingPermanent`, `SelectUnionArgs`). It is simply not exposed on the high-traffic field/zone selectors (`select_*_permanent`, `select_hand`, `select_trash`, `select_security`). Extending the existing mechanism — not inventing one — collapses the bind-then-act pair across 200+ clauses at near-zero semantic risk.

**2. The searcher idiom expands to 4–7 steps in 57 cards.** `reveal_top_deck → select_reveal[/buckets] → add_to_hand_from_reveal → place_remainder_on_deck` (often with two select+add pairs) is the single most common multi-step idiom. A `reveal_search` composite expresses it in one verb, each bucket remaining an RL-visible pick.

**3. Security placement is simultaneously sprawling and under-powered.** There are ~6 overlapping place-on-security verbs (`place_on_security`, `place_permanent_on_security` + `_and_handle_replacement`/`_bottom_and_cancel_replacement`/`_observed`, `place_self_at_security`, `place_self_option_at_security`) over 4 near-identical arg structs (`step.rs:1587-1626`), **yet none supports a player-elected top/bottom position** — `StackPosition` (`step.rs:1434-1438`) has only `Top`/`Bottom`/`Random`. So "as top OR bottom security" always explodes into `select_effect_choice` + two `if`-equals arms calling the same verb with different position. This amplifies into BT25-038's ~100-line clause A. The family also has an asymmetry (bottom+cancel exists but bottom+handle doesn't).

**4. `link_card_to_self` is superseded but the flywheel is running backwards.** It is marked in-source as superseded by the more general `link_cards`, yet usage has grown to 11 cards (up from the documented 5) while `link_cards` has only 3 — and new BT25 cards keep landing on the deprecated verb. Migrating the 11 and deleting the old verb stops a documented divergence from widening (the deprecation note itself is a quick win in `fix-dsl-substrate-rot-and-bugs`; the migration + deletion is here).

All four reduce per-card line count and the surface authors/sub-agents must reason over, while raising the ceiling for the shapes they cover.

## What Changes

- **Generalize the existing `then: Vec<StepSpec>` action-tail** to all selection verbs (`select_opponent_permanent`, `select_own_permanent`, `select_hand`, `select_trash`, `select_security`), so `select` + bind + act collapses into one clause. Optionally add thin composite aliases (`discard_from_hand`, `recover_from_trash`) for the most common bundles. Each tail step still surfaces its own RL-visible selections.
- **Add a `reveal_search` composite verb**: `reveal_search { of, count, buckets: [{ filter, to: hand|trash|deck, max, optional, prompt }...], remainder: top|bottom|choose }`. Also make `add_to_hand_from_reveal` accept a multi-card list so "add N" is one `max: N` bucket (fixes the EX5-015 single-card limitation).
- **Security-placement overhaul**: add `position: choice` (player-elected top/bottom) to the placement verbs so the top/bottom `select_effect_choice` + paired `if`-arms collapse into the step; and consolidate the ~6 place-on-security verbs into one source-polymorphic verb (source: hand binding OR material binding OR self) with a shared replacement-disposition enum, fixing the bottom+handle asymmetry.
- **Migrate `link_card_to_self` → `link_cards`** (after confirming `link_cards` covers single-card self-host and chosen-host; close `G-DSL-LINK-N-CARDS-PER-HOST` / `G-DSL-LINK-FROM-ANY-OWN-DIGIMON-SOURCES` if needed), then delete the deprecated verb and its lowerer.
- **Link substrate for relinking + EX11-027** (folded in from `fix-dsl-substrate-rot-and-bugs`, 2026-06-14). That change found EX11-027 Maquinamon cannot leave test-only raw_rust without 4 new link primitives (filed in `qa/dsl-vocab-gaps.md`): `G-DSL-LINK-RELINK-STANDING-PERMANENT` (move a standing battle-area permanent to become a link card on another own Digimon — DCGO `IPlacePermanentToLinkCards`), `G-DSL-LINK-HETEROGENEOUS-CHOICE` (a single RL selection that is an either/or between two distinct link operations), `G-DSL-LINK-HOST-FILTER` (host filter + link-requirement enforcement on `link_card_to_self { to: ChosenOwnDigimon }`, excluding the source), and `G-DSL-REPLACEMENT-LINK-CARD-TO-BOTTOM-SOURCE` (a `kind: replacement` cost that places a chosen link card as the carrier's bottom digivolution card to cancel a leave). Add these primitives, migrate EX11-027 off raw_rust, and **promote the `dsl-substrate-integrity` loader guard from warn-mode to a hard error** — EX11-027 is the last pack card on unregistered raw_rust, so once it migrates the guard can panic on any future unregistered ref.

## Capabilities

### Modified Capabilities
- `dsl-card-scripting-vocabulary`: all selection verbs accept an inline `then:` action tail (the bind-then-act pair collapses into one clause); a `reveal_search` composite verb expresses the reveal→bucket→remainder searcher idiom and `add_to_hand_from_reveal` accepts multiple cards; security placement supports a player-elected position and a single source-polymorphic verb replaces the per-source/per-disposition family; `link_card_to_self` is removed in favor of `link_cards`.

## Impact

- **DSL crate:** add `then` to the field/zone `Select*Args` structs (`step.rs`) + compile/lower; new `reveal_search` verb (`step.rs`, `compile.rs`, `compiled.rs`, lowerer); `StackPosition::Choice` (+ `compile_stack_position` `compile.rs:278-285`); consolidate the place-on-security arg structs (`step.rs:1587-1626`) behind one verb + a `SecurityReplacementDisposition` enum; delete `link_card_to_self`.
- **Engine lowering:** `dsl_cards/*` arms for the new/changed verbs; the `then`-tail lowering already exists for the seven structs — reuse it.
- **Cards:** optional mechanical migration of the longhand `select`+act pairs (no rewrite forced — old form still valid); BT25-038 + the security-placement cards simplify; the 11 `link_card_to_self` cards migrate to `link_cards`; EX5-015 uses the multi-card reveal-add.
- **Docs:** regenerate the vocab block; update `RUST_DSL_AGENT_GUIDE.md` §5 (the searcher pattern + the `then`-tail idiom + security placement).
- **Tests:** `then`-tail behavioral tests on each newly-supporting selector; `reveal_search` parser + behavioral; `position: choice` selection test; link migration parity tests on the 11 cards.
- **RL contract:** additive only. `then`-tail steps and `reveal_search` buckets resolve through existing selection machinery (each pick is a normal `pending_selection`); `position: choice` adds one ordinary binary selection. No action-space/tensor size change expected — confirm the selection-slot encoding is reused.

## Non-Goals

- Type unification (`FormulaSpec`/comparator/budget merge) — `unify-dsl-scalar-and-comparators`. (If that change lands first, new verbs here use the canonical `FormulaSpec` for any magnitude fields.)
- Bug fixes, loader guard, dead-vocab retirement, doc-rot, and the `link_card_to_self` *deprecation note* — `fix-dsl-substrate-rot-and-bugs`. (This change does the actual migration + deletion.)
- Tier-3 capability gaps — card-driven, authored when their archetypes come up.
