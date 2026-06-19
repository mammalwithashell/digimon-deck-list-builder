# Tasks — collapse DSL step idioms

> **Implementation note (verified 2026-06-18, before starting):** the field/zone select steps (`SelectHand`/`SelectTrash`/`SelectOwnPermanent`/`SelectOpponentPermanent`) ALREADY receive the dispatcher's *implicit* `tail` (the rest of the process body) at their execution sites in `code/digimon-engine/src/dsl_cards/step/selections.rs` (e.g. `install_select_hand(..., tail.to_vec(), ...)`), so `select_*` + a following step already works via the process sequence. The 8 existing `then:`-bearing structs (`SelectOwnSources`, `SelectOpponentSources`, `DigiBurst`, `SelectOpponentDpBudget`, `SelectOpponentPlayCostBudget`, `SelectOwnBreedingPermanent`, `SelectUnionArgs`, + 1) carry an EXPLICIT scoped `then` that the dispatcher must compose with the implicit tail. **The subtlety to get right:** when a field/zone select gains an explicit `then`, the explicit tail must run scoped to the selection binding AND compose correctly with the implicit dispatcher tail WITHOUT double-running — read how `SelectOwnSources` (selections.rs:645, compiled.rs:1653) threads its `then` into the install helper + `selection_result` and mirror exactly. This is the cloneable VM's `ResumeFrame::RunTail` data, so keep it closure-free. CompiledStep variants (`SelectOwnPermanent`/`SelectHand`/`SelectTrash`/`SelectOpponentPermanent`) need a new `then: Vec<CompiledStep>` field (`#[serde(default)]` for bincode-pack compat — see [[reference_dsl_substrate_authoring_gotchas]]; build.rs regenerates the pack via the build-dep on digimon-dsl).

## 1. Generalize the `then:` action-tail (do first — lowest risk, highest reach)
- [x] 1.1 Added `#[serde(default)] then: Vec<StepSpec>` to `SelectFieldArgs` (select_own/opponent/any_permanent) and `SelectZoneArgs` (select_hand/trash/reveal/security). Because both arg structs are SHARED, the field is lowered for **all 7** variants that use them (incl. `SelectAnyPermanent` + `SelectReveal`) to avoid a silent-drop footgun and per rule 28 (widen uniformly) — `SelectReveal`'s low-level `then` is orthogonal to and forward-compatible with §2's `reveal_search`. Compiled variants gained `#[serde(default)] then: Vec<CompiledStep>` (bincode-pack compat). Lowered via new `compile_then_tail` helper (mirrors `SelectOwnSources::then`). Dispatcher composes `inner_tail = then ++ tail` via new `compose_then_tail` helper, keeping `selection_result` — `then` runs only on a pick (callback) or, for a non-cost optional decline, scoped to the empty binding (no-op, identical to the `SelectOwnSources` min==0 case); the implicit `tail` runs exactly once. Closure-free (cloneable VM `ResumeFrame::RunTail`). Patched 29 `CompiledStep::Select*` literal constructions across the dsl test files + 1 in `compile.rs` (the replacement-choose desugar).
- [x] 1.2 Behavioral parity tests in `tests/dsl/collapse_then_tail.rs`: `select_trash` + `then:[add_to_hand_from_trash]` (mirrors the implicit-tail form in `phase2b_end_to_end`), `select_own_permanent` + `then:[delete_permanent]`, `select_own_permanent` + `then:[suspend]`. All green; full `dsl` target 775/0.
- [x] 1.3 RL-visibility test `then_tail_inner_selection_stays_rl_visible`: a `select_own_permanent` whose `then` contains a `select_hand` installs a NEW `pending_selection` (with exposed candidate actions) after the first pick resolves — no auto-resolution (rule 17).
- [~] 1.4 SKIP `discard_from_hand` / `recover_from_trash` aliases — the `then:` form reads clearly and adding aliases would re-introduce the sprawl this change removes (the task's own guidance).

## 2. `reveal_search` composite
- [ ] 2.1 Make `add_to_hand_from_reveal` (+ `trash_from_reveal`/`return_to_deck_from_reveal` siblings) accept a multi-card `CardList` (`max: N`); fix EX5-015's single-card limitation.
- [ ] 2.2 Add `reveal_search { of, count, buckets: [{filter, to: hand|trash|deck, max, optional, prompt}...], remainder: top|bottom|choose }`; lower to reveal_top_deck → per-bucket select → per-bucket move → place_remainder.
- [ ] 2.3 Port the existing reveal-idiom corner-case tests (empty pool, all-optional declined, remainder ordering, no_duplicate_cards across buckets) onto `reveal_search`.
- [ ] 2.4 Migrate a few representative searcher cards (e.g. BT9-092, EX5-015) to `reveal_search` as parity fixtures.

## 3. Security-placement overhaul
- [ ] 3.1 Add `StackPosition::Choice` + `compile_stack_position` (`compile.rs:278-285`) handling; lower to a binary top/bottom `pending_selection`.
- [ ] 3.2 Behavioral test: `position: choice` offers top/bottom and places accordingly; migrate BT25-038 clause A to it and confirm the ~100→~20 line reduction with parity.
- [ ] 3.3 Consolidate the place-on-security arg structs (`step.rs:1587-1626` + `place_on_security` 1587-1595) into one source-polymorphic verb (`source:` = hand binding | permanent/material binding | self) with a `SecurityReplacementDisposition` enum (`none|cancel|handle|observed`).
- [ ] 3.4 Behavioral parity on EVERY current place-on-security user before deleting the old verbs; keep the replacement dispositions legal only inside `kind: replacement`. Fix the bottom+handle asymmetry.
- [ ] 3.5 Delete the superseded place-on-security verbs.

## 4. `link_card_to_self` → `link_cards` migration
- [ ] 4.1 Confirm `link_cards` covers single-card self-host + chosen-host; close `G-DSL-LINK-N-CARDS-PER-HOST` / `G-DSL-LINK-FROM-ANY-OWN-DIGIMON-SOURCES` if a residual gap exists.
- [ ] 4.2 Migrate the 11 cards (ST22-12, BT21-023/073/101, BT25-052/056/060/069/070/072/089) to `link_cards` with per-card parity tests.
- [ ] 4.3 Delete `link_card_to_self` + its lowerer.

## 4.5 Link substrate + EX11-027 (folded in from fix-dsl-substrate-rot-and-bugs)
- [ ] 4.5.1 Add `G-DSL-LINK-RELINK-STANDING-PERMANENT`: an engine primitive (DCGO `IPlacePermanentToLinkCards` analog) + DSL verb to move a standing battle-area permanent to become a link card on a chosen own Digimon.
- [ ] 4.5.2 Add `G-DSL-LINK-HOST-FILTER`: host filter + link-requirement enforcement on `link_card_to_self { to: ChosenOwnDigimon }` (exclude the source permanent; check `CanLinkToTargetPermanent`).
- [ ] 4.5.3 Add `G-DSL-LINK-HETEROGENEOUS-CHOICE`: model a single RL selection that is an either/or between two distinct link operations (self-permanent relink vs hand-card link), with a branch offered only when its precondition holds.
- [ ] 4.5.4 Add `G-DSL-REPLACEMENT-LINK-CARD-TO-BOTTOM-SOURCE`: a `kind: replacement` cost that places a chosen link card as the carrier's bottom digivolution card to cancel a leave (engine fn analogous to `trash_own_link_card_and_cancel_leave` but calling `AddDigivolutionCardsBottom`).
- [ ] 4.5.5 Migrate EX11-027 Maquinamon off raw_rust to the new vocab; behavioral tests for the on_play link choice + the leave-prevention replacement; remove `ex11_027_*` from the phase0/roundtrip `StubRegistry`.
- [ ] 4.5.6 Promote the `dsl-substrate-integrity` loader guard in `code/digimon-engine/src/cards.rs` from warn-mode to a hard error (panic on unregistered raw_rust ref) now that the pack has zero unregistered refs; add the engine-level "unregistered ref fails load" test (fix-dsl-substrate-rot-and-bugs §1.2).
- [ ] 4.5.7 Move the `G-DSL-LINK-*` entries from `qa/dsl-vocab-gaps.md` to `qa/resolved-gaps.md`.

## 5. Docs + verification
- [ ] 5.1 Regenerate the vocab block; confirm removed verbs drop out + new verbs appear; `dsl-vocab-doc-drift --check` green.
- [ ] 5.2 Update `RUST_DSL_AGENT_GUIDE.md` §5 (searcher pattern, `then:`-tail idiom, security placement).
- [ ] 5.3 RL guard: action-mask/tensor for a representative collapsed card matches its longhand equivalent (no action-space/tensor size change).
- [ ] 5.4 Full DSL + behavioral + action-mask suites green.
