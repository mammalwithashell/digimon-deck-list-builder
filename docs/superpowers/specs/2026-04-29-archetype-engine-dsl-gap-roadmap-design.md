# Archetype Engine and DSL Gap Roadmap

**Date:** 2026-04-29
**Status:** Design approved; implementation plans pending

## Context

Recent archetype readiness passes have produced a large set of Rust engine and DSL gaps. The gaps are documented across:

- `docs/RUST_ENGINE_GAPS.md`
- `qa/archetype-qa/engine-gaps.md`
- `qa/dsl-vocab-gaps.md`
- `qa/archetype-qa/*.md`
- `qa/archetype-qa/dsl/*.md`
- `docs/RUST_ENGINE_API.md`
- `docs/RUST_DSL_TEST_API.md`
- `docs/ACTION_SPEC.md`
- `docs/TENSOR_SPEC.md`

The common pattern is not that individual cards need special handling. Most blocked cards need reusable engine primitives, pending-selection surfaces, action-mask ranges, or DSL lowering vocabulary. The roadmap should therefore be capability-first, with archetype unlock checkpoints after each group.

The no-approximations policy remains the controlling rule: every player-visible choice must flow through engine actions or pending selections. The roadmap must not introduce hidden auto-picks, card-effect stubs, broad raw-Rust escapes, or UI-only decisions.

## Goals

1. Convert the documented gap pile into logical implementation groups that can become separate, testable implementation plans.
2. Sequence foundational engine work before dependent DSL/card-authoring work.
3. Preserve Rust as the source of truth while keeping action-mask, tensor, PyO3, and RL contracts stable.
4. Define acceptance gates so each group can be closed with behavioral evidence, tracker updates, and archetype readiness improvements.
5. Keep individual card implementation out of this roadmap except where a card is named as the first regression or acceptance fixture for a reusable primitive.

## Non-Goals

- This spec does not implement any card, engine primitive, or DSL verb.
- This spec does not replace `docs/RUST_ENGINE_GAPS.md`, `qa/archetype-qa/engine-gaps.md`, or `qa/dsl-vocab-gaps.md`; it organizes them into execution groups.
- This spec does not bless raw-Rust no-op placeholders as an acceptable final state.
- This spec does not retire Python parity tracking. `docs/RUST_PYTHON_PARITY.md` remains relevant until the Rust engine fully owns production behavior.

## Architecture

The roadmap is capability-first with archetype unlock checkpoints. Each group should land as one or more focused implementation plans. A group is complete only when it adds failing tests first, implements the reusable primitive, wires DSL syntax where applicable, updates docs/trackers, and proves the relevant archetype blockers have moved.

Each implementation plan should follow this loop:

1. Pick a narrow primitive from one roadmap group.
2. Write a failing Rust engine or DSL behavioral test using the smallest real-card fixture that exposed the gap.
3. Implement the engine primitive and action/pending-selection surface.
4. Add or update DSL schema/lowering only after the engine behavior exists.
5. Run targeted Rust tests and any DSL lint/schema checks.
6. Update gap trackers and readiness docs.
7. Re-run the relevant archetype assessment or card behavioral test slice.

## Roadmap Groups

### Group 1: Event Context and Dispatch Foundation

Purpose: normalize when effects fire and what event payload they receive.

Representative gaps:

- `G-INHERITED-DISPATCH`
- `G-OPT-TRIGGERED`
- `G-ON-MOVE`
- `G-GAME-EVENT-DIGIVOLVE`
- `G-ON-DIGIVOLVE-TRAIT-FILTER`
- `G-ON-ENTER-FIELD-ANYONE-TRAIT-FILTER`
- `G-OPTION-PLACED-TIMING`
- `G-BREEDING-TRIGGER-DISPATCH`
- `OnDigivolutionCardTrashed`
- `OnAllyAttack` / `OnOpponentAttack`
- `OnOwnSecurityRemoved`
- `OnPlaceSecurity` / `OnAddedToSecurity`
- `OnDiscardSecurity`
- `EndOfOpponentsTurn`

Key design:

- Introduce a consistent trigger payload model that can carry source permanent, event card, entering permanent, digivolving target, trashed source, source index, cause player, owner/controller, attack participant, and security event context.
- Ensure `effect_queue::enqueue_from_permanent` scans top-card, inherited stack sources, linked cards, Training/Delay option states, and breeding permanents where the timing explicitly allows it.
- Enforce once-per-turn and max-per-turn counters for all triggered dispatch paths, not just manually activated effects.
- Keep event fan-out explicit and narrow. Do not use broad global dispatch when a timing is side-specific or zone-specific.

Acceptance:

- Inherited Medusamon/Rocks security-removed effects fire from under a carrier.
- Royal Knights breeding King Drasil effects can fire without moving the permanent to battle.
- Option placement can trigger an inherited observer with the placed option's trait in context.
- `OnDigivolutionCardTrashed` fires the exact trashed source card, not a later trash-zone scan.

### Group 2: Selection and Action-Mask Primitives

Purpose: make every new player choice visible to RL through pending selections and masks.

Representative gaps:

- Cross-permanent count-capped source selection
- `G-ROCKS-SOURCE-SELECTION-DSL`
- `G-MULTI-SELECT-OPP-DP-SUM`
- Ordered permutation selection
- Effect-choice branch selection
- Opponent-as-selecting-player
- Union-zone hand-or-trash selection
- Multi-pick from reveal
- DNA-pair selection
- `G-BREEDING-PERMANENT-SELECTION`
- `G-SELECT-EMPTY-OUTER-TAIL`
- `G-FOR-EACH-EXCLUDE-BINDING`

Key design:

- Add stable selection references for battle-area permanents, breeding permanents, source cards under permanents, revealed cards, security cards, and branch choices.
- Avoid position-shift bugs by using stable identifiers or reverse-order deletion when iterating over mutable battle areas.
- Add PASS/decline terminators for up-to-N prompts, optional costs, and optional triggered effects.
- Support aggregate constraints: DP budget, exact N, up-to-N, one-per-predicate, and ordered return-to-deck permutations.

Acceptance:

- Rocks can ask the player to choose source cards across all own Digimon stacks and trash exactly those choices.
- DP-budget deletion can select multiple opponent Digimon until the budget is exhausted or the player stops.
- Breeding-area permanents can be selected without being encoded as fake battle-area handles.
- Empty inner selections continue outer tails when card text says "then" behavior still happens.

### Group 3: Cost and Replacement Framework

Purpose: provide faithful cost-before-effect and would-leave prevention semantics.

Representative gaps:

- Generic `.pay_cost()` / `.activation_cost(...)` for triggered abilities
- `G-REPLACEMENT-CAUSE-GATE`
- `G-PARTITION-SOURCE-ENFORCEMENT`
- `G-DELAY-REPLACEMENT-PREVENT-DELETION`
- `<Fragment (N)>`
- `<Barrier>`
- `<Scapegoat>`
- `<Armor Purge>`
- `<Decode>`
- Source-scoped return immunity
- Effect-driven attack cancellation
- Deletion optionality

Key design:

- Generalize `EffectBuilder::pay_cost` so every timing can gate process execution on a cost result.
- Costs that require choices must create selections before the process body runs.
- Replacement evaluation must carry cause/controller/source-player context and evaluate `active_when` before offering prevention.
- Prevent/cancel results should be explicit values in the leave-field or attack state machine.

Acceptance:

- Puppet "other than by your effects" prevention does not offer illegal prevention against own effects.
- Partition checks required sources, asks the player for legal source choices, plays the selected sources, and prevents the original leave-field event only when successful.
- EX10-003-style attack cancellation can end the pending attack after the cost is paid.
- Optional deletion observers can be declined.

### Group 4: Zone Movement and Stack Operations

Purpose: make effect-initiated movement between zones complete and scriptable.

Representative gaps:

- Play from hand/trash/security/materials without paying cost
- Effect-initiated digivolve from hand/trash/security/materials
- Return to hand/deck top/deck bottom
- Trash from hand
- Security trash/place/recover/shuffle operations
- Bottom-source placement
- Stack reorder and source extraction
- `ctx.move_from_breeding()`
- Play to empty breeding slot
- Cast-time stack construction for cost reduction
- Search own security stack
- Add pending security option to hand

Key design:

- Movement helpers must accept source-player/cause where prevention modifiers depend on it.
- Helpers must centralize event emission, including OnPlay suppression flags, played-by-effect flags, digivolved-by-this-effect flags, and source-leave triggers.
- Stack mutation helpers must preserve source ownership, ordering, and trigger semantics.
- Security helpers must distinguish pending security resolution, face-up security search, and the security stack itself.

Acceptance:

- Options resolving from security can move the currently resolving card to hand instead of trash.
- Digivolve-from-trash and digivolve-from-security use the same legality and event hooks as hand digivolve, with source-zone differences explicit.
- Security placement respects `CannotAddSecurityByEffect`.
- Moving from breeding fires the correct movement/enter-field observers.

### Group 5: Option, Delay, Plug-In, Link, and Training State

Purpose: make non-Digimon battle-area states first-class.

Representative gaps:

- Option play flow and disposition
- Event-gated Delay
- Start-of-turn Delay
- Delay-as-replacement
- Inherited-security option placement
- Option placed/trashed observers
- Plug-In / Link registration and re-linking
- Linked-card effect scope
- Training option behavior
- Scheduled end-of-turn option effects

Key design:

- Model placed Options with explicit state: Delay, Training, Plug-In free, Plug-In linked, transient, and other future field-option states.
- Record placement turn so Delay cannot activate on the turn it was placed.
- Dispatch Option-specific events after placement, activation, trashing, re-linking, or carrier loss.
- Keep Plug-In linked cards distinct from digivolution sources and regular battle-area permanents.

Acceptance:

- Royal Knights option placement can trigger King Drasil inherited effects.
- Red/Green Scramble start-of-turn Delay fires at the correct turn boundary.
- Unique Emblem event-gated Delay activates only when the named Tamer event occurs and only after the placement turn.
- Plug-In cards can link from hand or from battle area and preserve linked-scope effects.

### Group 6: Modifiers, Auras, and Keywords

Purpose: make continuous effects and printed keywords enforceable in masks, combat, and queries.

Representative gaps:

- Player-scoped modifier registry
- Source-scoped return/de-digivolve immunity
- Declarative aura to player-scoped modifiers
- Named-target declarative auras
- Security-zone auras
- Dynamic DP formulas
- Dynamic security attack modifiers
- Ignore color requirement in Rust option masks
- Collision, Piercing, Reboot, Retaliation, Progress, Overclock, Digi-Burst, Decoy, DigiXros alias, Ace Overflow
- Permanent-scoped activation suppression
- Fixed attack target and redirect immunity

Key design:

- Separate permanent-scoped, player-scoped, card-source-scoped, and query-time declarative modifiers.
- Prefer query-time aura evaluation for field-state-dependent effects to avoid stale materialized modifiers.
- Every keyword that changes legality must be reflected in both action masks and execution validation.
- Dynamic formulas must be evaluated at query time when printed text is continuous.

Acceptance:

- `IgnoreColorRequirement` affects Rust option action masks.
- Reboot unsuspends during the opponent's unsuspend phase.
- Piercing continues to security only after a winning battle where Piercing is active.
- Collision forces or constrains blocker behavior through combat, not only through display keywords.
- Dynamic DP and Security A. modifiers update after stack depth or board state changes.

### Group 7: DSL Predicate, Formula, and Lowering Coverage

Purpose: expose existing and new engine primitives to YAML without raw-Rust escapes.

Representative gaps:

- Event-card predicates for Mineral/Rock observers
- Replacement cause predicates and `active_when`
- `dp_lte` / `dp_gte` candidate evaluation
- `event_target_owner`
- `[All Turns]` triggered filters
- Board-color cross-reference predicates
- Play-cost filters
- Opponent security count predicates
- Binding DP formulas
- Same-level pair formulas
- Formula filters for counted battle-area cards
- Lowest-level aggregate predicates
- Shared-trash formula thresholds
- `dna_costs` authoring / production population
- Alt-path conditions
- `not_in_binding`
- Dynamic aura formula fields

Key design:

- DSL predicates should evaluate against an explicit subject: candidate card, permanent, event payload, source permanent, replacement context, or binding.
- Formula evaluation should accept bindings and filtered zone scans rather than ad hoc formula variants for each card.
- The compiler should reject unsupported constructs clearly instead of lowering to no-op placeholders.
- YAML syntax should remain declarative and reusable; avoid one-card verb names.

Acceptance:

- Medusamon, Rocks, Royal Knights, Puppets, and BG Imperial YAML can remove current raw-Rust placeholders for predicate/formula-only blockers.
- `qa/dsl-vocab-gaps.md` entries get closed only when schema, compiler, lowerer, tests, and docs are all updated.
- DSL-generated effects preserve the same action/pending-selection behavior as hand-written Rust effects.

### Group 8: Token and Card-Data Completion

Purpose: finish metadata and generated object support needed by the above groups.

Representative gaps:

- Familiar Token On Deletion
- Petrification Token and token definitions
- `CardKind::Token`
- DigiXros scoped aliases
- Reveal-zone overlays
- Ace Overflow metadata and inherited penalty
- `CardData.dna_costs` population
- Native keyword parsing from printed/card data

Key design:

- Tokens must be real card definitions with effects, not special-cased empty permanents.
- Metadata that affects action masks should live in `CardData` or generated card registry data, not in one-off effect closures.
- Alias and overlay semantics must be scoped to the subsystem that prints them. DigiXros aliases must not leak into generic name matching.

Acceptance:

- Familiar token deletion creates the opponent-target choice and applies -3000 DP.
- DNA action masks work from authored/loaded `dna_costs`.
- Ace Overflow is applied when the card leaves from field or under-card states.
- DigiXros alias matching is limited to DigiXros material checks.

### Group 9: Archetype Unlock Passes

Purpose: validate the capability roadmap against real decks instead of abstract coverage.

Checkpoint order:

1. **Medusamon** — inherited dispatch, security-removed observers, option/security disposition, DP predicates.
2. **Rocks** — source selection, source trash triggers, pay-cost ordering, Collision, source immunity, Delay/Option state.
3. **Royal Knights** — breeding trigger dispatch, breeding selection, option placement, option trait predicates.
4. **Puppets** — Overclock sacrifice predicates, Familiar token effects, event-gated Delay, replacement cause gates.
5. **BG Imperial** — DNA costs, end-of-turn DNA registration, Partition, Delay replacement prevention.
6. **Chaos Control / DNA Omnimon** — effect-initiated digivolve from non-hand zones, self-stack predicates, branch choices.
7. **Dark Masters and remaining audits** — global observer coverage, deletion/enter-field observers, large-scale regression.

Each checkpoint should:

- Re-run the appropriate `assess-rust-engine-archetype` workflow or equivalent readiness review.
- Update `docs/RUST_ENGINE_GAPS.md`, `qa/archetype-qa/engine-gaps.md`, and `qa/dsl-vocab-gaps.md`.
- Move cards from raw-Rust/no-op placeholders to DSL or hand-written Rust only when behavior is complete.
- Add behavioral tests for the representative real cards.

### Group 10: Acceptance and Regression Gates

Purpose: keep the roadmap from closing gaps on paper while behavior remains partial.

Every implementation group must satisfy:

- Failing test committed before implementation work in that group.
- Passing targeted `cargo test` for the changed Rust engine/test surface.
- DSL schema/lint/generation tests when YAML syntax changes.
- Action-mask tests for every new legal choice or prevention choice.
- PyO3/RL contract review when action space, tensor shape, or exposed runner behavior changes.
- Tracker updates that move gaps to resolved or narrow the remaining blocker precisely.
- No new production dependency on `engine_py_legacy`.
- No auto-selection of a player-visible choice.
- No raw network/UI-only handling for game rules.

## Sequencing

| Phase | Groups | Dependency Notes |
|---|---|---|
| 1 | Event Context and Dispatch | Required before most observer, inherited, Delay, and archetype readiness work. |
| 2 | Selection and Action-Mask Primitives | Required before source costs, replacement choices, branch choices, breeding targets, and DP-budget effects. |
| 3 | Cost and Replacement Framework | Depends on selection primitives; unlocks Partition, Fragment, Barrier, Scapegoat, and Delay prevention. |
| 4 | Zone Movement and Stack Operations | Can start after event context exists; some source/security helpers depend on selection. |
| 5 | Option/Delay/Plug-In/Training State | Depends on event context and zone helpers; some Delay replacement work depends on Group 3. |
| 6 | Modifiers, Auras, and Keywords | Can proceed in slices, but mask-affecting keywords must coordinate with Group 2. |
| 7 | DSL Predicate, Formula, and Lowering Coverage | Should follow the engine primitive for each construct; pure predicate/formula fixes can run earlier. |
| 8 | Token and Card-Data Completion | Can proceed in parallel except where metadata feeds action masks or keyword enforcement. |
| 9 | Archetype Unlock Passes | Runs after each capability group, not only at the end. |
| 10 | Acceptance and Regression Gates | Applies to every phase. |

## Parallelization Strategy

The safest parallel lanes are:

- Event dispatch sub-slices with disjoint timings.
- Pure DSL predicate/formula fixes that do not alter shared selection/action internals.
- Token/card-data completion.
- Keyword enforcement slices with separate combat/mask surfaces.
- Archetype readiness reassessments after a group lands.

The riskiest shared surfaces should be serialized:

- `PendingSelection` representation and action decoder changes.
- `PermanentHandle` / breeding permanent addressing.
- `EffectContext` movement helpers that mutate multiple zones.
- Replacement-window semantics.
- Option/Delay battle-area state model.
- Player-scoped modifier registry shape.

## First Implementation Plan Candidates

1. **Inherited Dispatch + OPT Enforcement**
   - Small, high-leverage, and already sharply documented.
   - First fixtures: BT21-008 Elizamon inherited security-removed, BT13-007 King Drasil breeding if breeding support is included.

2. **Cross-Permanent Source Selection + `OnDigivolutionCardTrashed`**
   - Directly unlocks Rocks core.
   - First fixtures: EX10-032 Proganomon or P-167 Landramon.

3. **Delay Timing Slice**
   - Separates start-of-turn Delay from event-gated Delay and replacement Delay.
   - First fixtures: LM-027 Red Scramble, BT22-098 Unique Emblem: Fable Waltz, BT17-097 Return to the Primogenitor.

4. **Breeding Permanent Support**
   - Cleanly unlocks Royal Knights support and surfaces permanent-addressing decisions.
   - First fixtures: BT13-007 King Drasil_7D6 and BT20-083 Omekamon.

## Done Criteria for This Roadmap

The roadmap is complete when:

- Every open gap in the three primary trackers is either resolved, moved into a clearly scoped remaining blocker, or explicitly deferred with a reason.
- The DSL can express the representative card patterns without no-op raw-Rust placeholders.
- Archetype assessments for the target set produce no `engine-gap` or `dsl-gap` verdicts for the core deck shells.
- Action masks and pending selections expose all legal choices required by the implemented card text.
- Rust engine tests cover the card patterns that originally exposed the gaps.
