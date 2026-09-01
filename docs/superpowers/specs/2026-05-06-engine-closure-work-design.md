# Engine Closure Work Spec

**Date:** 2026-05-06
**Status:** Draft for implementation planning

## Context

Recent Rust DSL implementation runs have converged on the same set of reusable engine blockers. The important signal is not that individual archetypes need bespoke card code. The important signal is that several shared engine domains still do not expose every printed Digimon TCG choice through `PendingSelection`, action masks, stable event context, and observer-safe zone movement.

This spec consolidates those blockers into engine closure domains. It is intended to supersede one-off card-gap triage for the next implementation wave, while still preserving the live gap trackers as the source of truth for status updates.

Primary input artifacts:

- `docs/RUST_ENGINE_GAPS.md`
- `qa/archetype-qa/engine-gaps.md`
- `qa/dsl-vocab-gaps.md`
- `qa/archetype-qa/dsl/2026-05-03-dna-omnimon-dsl-engine-gaps.md`
- `qa/archetype-qa/dsl/2026-05-03-medusamon-cross-archetype-gaps.md`
- `qa/archetype-qa/dsl/alter-s-ladder-cross-archetype-gaps-2026-05-03.md`
- `qa/archetype-qa/dsl/bg-imperial-cross-archetype-gaps-2026-05-03.md`
- `qa/archetype-qa/dsl/chaos-control.md`
- `qa/archetype-qa/dsl/millenniummon.md`
- `qa/archetype-qa/dsl/puppets-2026-05-03-engine-dsl-gaps.md`
- `qa/archetype-qa/dsl/red-hybrid-ancientgreymon-2026-05-03-dsl-engine-gaps.md`
- `qa/archetype-qa/dsl/rocks-gap-inputs-2026-05-03.md`
- `qa/archetype-qa/dsl/royal-knights-2026-05-03-dsl-engine-gaps.md`
- `qa/archetype-qa/dsl/ts-olympos-2026-05-03-dsl-engine-gaps.md`
- `qa/archetype-qa/dsl/zephagamon-2026-05-03-dsl-engine-gaps.md`
- `docs/superpowers/specs/2026-05-03-latest-archetype-dsl-engine-gap-closure-design.md`

## Goals

1. Define the minimum engine capabilities needed to close the highest-impact Rust archetype blockers.
2. Group work by reusable engine domain and implementation dependency, not by archetype or card ID.
3. Preserve the no-approximations rule: every gameplay choice that affects legal outcomes must be surfaced through an engine action or `PendingSelection`.
4. Keep `ACTION_SPACE_SIZE`, active tensor profiles, PyO3 exports, frontend constants, and RL wrappers stable unless a separate contract-change spec updates them together.
5. Provide concrete first fixtures and acceptance gates for each closure domain.
6. Make tracker hygiene part of closure: a primitive is not closed until its tracker entries and source archetype reports are updated.

## Non-Goals

- This spec does not implement any engine, DSL, YAML, or test changes.
- This spec does not authorize card-effect stubs, hidden auto-selection, raw-Rust no-op placeholders, or UI-owned rules logic.
- This spec does not attempt broad archetype YAML authoring. Production card migration happens only after the required reusable primitive exists.
- This spec does not expand the action or tensor contract. Any primitive that cannot fit the existing pending-selection/action infrastructure must be split into a separate action/tensor contract spec.
- This spec does not treat legacy Python or DCGO as authoritative. Printed text and local rules docs remain higher priority.

## Closure Model

Engine closure proceeds in vertical slices. Each slice starts with one engine fixture that proves the primitive, then one card-shaped regression from an archetype report. DSL schema and lowering are included only when they are required to prove the primitive is usable without raw Rust.

The completion loop for every slice is:

1. Read printed text for the chosen fixture in `data/cards.json`.
2. Write a failing Rust test under `code/digimon-engine/tests/`.
3. Implement the engine primitive through existing subsystems: action masks, `PendingSelection`, `EffectContext`, replacement dispatch, effect queue, zone movement, or modifier enforcement.
4. Add DSL schema/lowering only after the engine behavior exists.
5. Add one card-shaped behavioral test, preferably for a card already carrying an ignored gap test or YAML comment.
6. Run targeted Rust tests.
7. Update `docs/RUST_ENGINE_GAPS.md`, `qa/archetype-qa/engine-gaps.md`, `qa/dsl-vocab-gaps.md`, and the relevant `qa/archetype-qa/dsl/*.md` source note.

## Impact Priority

| Priority | Domain | Why it comes first |
|---|---|---|
| P0 | Event context, provenance, and observer fan-out | Many later primitives depend on knowing what happened, who caused it, what card/permanent moved, and where the observer lives. |
| P0 | Replacement and would-leave decisions | Prevention, Armor Purge, Decode, Partition, Barrier, Scapegoat, and Puppet/Royal Knights protection all require pre-move choices. |
| P0 | Effect-created attack flows and combat interrupts | Immediate attacks, attack without suspending, Raid/Collision target changes, and redirect effects are action-mask critical. |
| P1 | Zone movement and source/material operations | Source-stack play, hand/trash/security free-play, mass source cleanup, and security movement unlock many high-frequency archetypes. |
| P1 | Selection primitives | DP-budget multi-select, union-zone choices, ordered reveal buckets, and source/security selections preserve RL-visible choices. |
| P1 | Option, Delay, and security disposition | Search/options are deck glue; field Options and security disposition must not bypass events or cleanup. |
| P2 | Formula, predicate, and modifier enforcement | These close card-specific expressiveness edges after the engine can carry event and movement data. |
| P2 | Production YAML/test migration gates | This converts primitives into cards but should not lead engine design. |

## Domain 1: Event Context, Provenance, and Observer Fan-Out

**Impact:** Critical

**Archetypes hit:** Medusamon, Royal Knights, Puppets, Zephagamon, Red Hybrid, DNA Omnimon, Rocks, Chaos Control, BG Imperial, TS Olympos.

**Problem:** Existing event dispatch has improved, but several archetypes still need complete payloads and broader zone fan-out. Effects must distinguish event card, event permanent, former controller, source player, cause, source zone, whether the event was effect-initiated, whether it was DNA, whether a suspend/delete came from Overclock, and whether an observer lives in battle, breeding, hand, trash, source stack, security, or field Option state.

**Required capabilities:**

- A trigger payload model that can carry event card, event permanent, former controller, affected player, source player, cause category, source effect/card, old/new attack targets, selected result bindings, and moved-card sets.
- Fan-out policies for battle-area, inherited stack, breeding-area, hand-resident, trash-resident, security-resident, linked/field Option, and Delay observers.
- Deleted-object snapshots for `OnAnyDeletion` predicates such as owner, kind, trait, level, DP, and cause.
- Entered-play and digivolved payloads that identify the entering/resulting permanent, not the observer.
- Suspend/unsuspend payloads that identify the suspended permanent, whether an effect suspended it, and which effect/card caused it.
- Stable provenance tokens returned by effect-play and effect-digivolve helpers for later cleanup, suppression, and result predicates.

**Representative first fixtures:**

- `EX11-060` Arisa Kinosaki: deletion observer must distinguish ordinary Token/Puppet deletion from Overclock-caused deletion and require a visible suspend-this-Tamer cost.
- `BT20-084` Sistermon Ciel (Awakened): trash-resident observer must see a Digimon played event and digivolve a field Sistermon Ciel into the trash card.
- `BT13-007` Royal Knights breeding fixture: a breeding-source observer must fire without moving King Drasil into battle.

**Acceptance gates:**

- Event predicates read the event subject, not the carrier or observer.
- Former controller and deletion cause are available after the permanent has left the battle area.
- Fan-out does not double-trigger the same effect through multiple zones.
- Once-per-turn and max-per-turn accounting still apply to inherited and off-field observers.
- The action mask exposes any optional follow-up choice created by the observer.

## Domain 2: Replacement and Would-Leave Decisions

**Impact:** Critical

**Archetypes hit:** Royal Knights, Puppets, TS Olympos, DNA Omnimon, Alter-S Ladder, Medusamon, Chaos Control, Red Hybrid.

**Problem:** Cards that say "would leave", "would be deleted", "prevent it from leaving", "other than by your effects", Armor Purge, Decode, Partition, Barrier, Scapegoat, and Delay-as-prevention all need a pre-movement replacement window. Many require the source of the replacement to be different from the permanent being protected.

**Required capabilities:**

- Centralized leave-field attempts before destination mutation.
- Replacement context with threatened subject, source permanent/card, cause player, cause category, destination, battle/non-battle flag, and effect source kind.
- Pending-selection cost payment inside replacement resolution.
- Non-cancelling would-leave observers that react to the same pre-move event and then allow the original move to continue.
- Inherited replacement scanning under the threatened permanent.
- Keyword-provided replacement emitters for Armor Purge, Decode, Partition-adjacent source play, and future Barrier/Scapegoat variants.

**Representative first fixtures:**

- `BT22-036` or `EX11-022` as inherited Puppet source: when the carrier would leave by an opponent effect, offer a Token/other Puppet deletion cost and prevent only that leave event.
- `BT20-100` The Last Guardian: Delay replacement prevents a matching Omnimon from leaving by trashing the Option.
- `BT20-091` Cool Boy: when a Royal Knight would leave, optionally play Omekamon without cancelling the leave event.

**Acceptance gates:**

- Declining an optional replacement allows the original leave event to proceed.
- Paying a replacement cost cancels only the specific pending leave event.
- Cause filters distinguish own effects, opponent effects, battle deletion, non-battle deletion, return, deck bottom, and security placement where printed text requires it.
- Inherited replacement effects preserve source-card attribution and carrier identity.
- No replacement auto-pays a cost or auto-selects a sacrifice.

## Domain 3: Effect-Created Attacks and Combat Interrupts

**Impact:** Critical

**Archetypes hit:** TS Olympos, Royal Knights, Zephagamon, Red Hybrid, DNA Omnimon, Alter-S Ladder, Medusamon, BG Imperial.

**Problem:** Many cards create an attack from inside effect resolution, allow a Digimon to attack without suspending, force an opponent's Digimon to attack, redirect an existing attack target, or observe attack-target changes. Treating these as simple keyword grants or automatic attacks hides legal choices and breaks combat timing.

**Required capabilities:**

- `EffectContext` helper for optional and mandatory effect-created attacks.
- Attack flow state that can be opened from an effect even outside the normal main-phase attack window.
- Flags for `without_suspending`, player-only targets, Digimon-only targets, selected-attacker-only, and forced-opponent attack.
- Combat interrupt for retargeting an existing attack, including Raid, Collision, Blocker-adjacent redirects, and card-specific target switches.
- `OnAttackTargetChange` payload with attacker, old target, new target, controller, and reason.
- Distinction between `battle:` effects and attacks; effect battles must not trigger attack-only security continuation or Piercing.

**Representative first fixtures:**

- `BT24-037` TS Olympos may-attack branch: after the effect resolves, select one eligible Digimon and expose a normal attack flow with PASS available before attack commitment.
- `BT20-102` Royal Knights: end-of-turn effect allows an attack without suspending.
- `ST18-14` Zephagamon: suspend-this-Tamer cost redirects an existing attack to another Digimon or the player.

**Acceptance gates:**

- Optional effect-created attacks expose decline/PASS.
- Mandatory attacks still use target legality and pending selection where a target choice exists.
- Attack-state restrictions do not leak into ordinary future attacks.
- `OnAttackTargetChange` fires once per successful retarget and carries enough context for inherited predicates.
- Effect battles remain separate from attacks.

## Domain 4: Zone Movement and Source/Material Operations

**Impact:** High

**Archetypes hit:** Royal Knights, Alter-S Ladder, DNA Omnimon, TS Olympos, Red Hybrid, Puppets, Rocks, Millenniummon.

**Problem:** Effects need to move cards between hand, trash, security, deck, breeding, source stacks, and battle area while preserving ownership, event dispatch, trigger provenance, and player-visible choices. Raw list mutation and index-based references are not acceptable because they skip observers and can move the wrong card after battle-area shifts.

**Required capabilities:**

- Stable source/material selectors for battle and breeding carriers.
- Play from source/materials with name uniqueness, source-zone identity, and On Play suppression flags.
- Effect play from hand/trash/security with `PlayOptions`, including `suppress_on_play`.
- Effect digivolve from hand, trash, source, and mixed material zones with source/result provenance.
- Security movement helpers for selected security, top/bottom placement, top security to hand, self to security, and stacked-card-to-security.
- Bulk zone moves such as return all trash to deck bottom and trash top N sources of every opponent Digimon.
- Owner-routed deck-bottom/top placement for cards moved from shared or opponent-controlled zones.

**Representative first fixtures:**

- `BT13-112` Royal Knights: choose distinct-name Royal Knight sources under King Drasil, play them, suppress On Play where printed, trash King Drasil, and grant Rush.
- `BT5-106` Demonic Disaster: Security plays a level 3 purple Digimon from trash while suppressing only that played Digimon's On Play effects.
- `BT17-077` Imperialdramon: Paladin Mode: trash all opponent sources, return a chosen player's trash to deck bottom, and bind moved-card results for memory gain.

**Acceptance gates:**

- Moving security cards fires security-removed observers exactly when it should.
- Source/material movement removes the exact selected source from the original carrier.
- Effect-created permanents return a stable handle for scheduled cleanup or later branch predicates.
- Owner routing is correct even when control differs from ownership.
- No helper mutates raw vectors without routing through observer-safe movement APIs.

## Domain 5: Selection Primitives and Action-Mask Surfaces

**Impact:** High

**Archetypes hit:** Royal Knights, DNA Omnimon, Alter-S Ladder, Medusamon, Red Hybrid, Puppets, Rocks, Zephagamon.

**Problem:** The engine needs a complete set of reusable pending-selection shapes for printed choices: aggregate DP-budget multi-select, exact-N trash selection, up-to-N partial selection, reveal buckets, ordered remainders, opponent-as-selecting-player, union-zone choices, security selection, source-stack selection, DNA pair selection, and selected-property binding.

**Required capabilities:**

- Aggregate-sum multi-select with a running DP or play-cost budget.
- Exact-N and up-to-N multi-select with PASS/finish semantics.
- Ordered permutation over revealed or deck-remainder cards.
- Union-zone selectors that preserve origin zone identity.
- Opponent-controlled selections over opponent hand/security/trash where printed.
- Security selectors that respect visibility and stable index behavior.
- DNA and Blast DNA pair selectors, including mixed field plus hand materials.
- Binding selected properties such as level, play cost, color, trait, DP, and source count for later mass application.

**Representative first fixtures:**

- `BT17-018` Gallantmon: Crimson Mode: choose any number of opponent Digimon whose total DP is 15000 or less.
- `BT24-017` Medusamon: exact-count opponent trash selection gates follow-up token creation.
- `BT17-078` Omnimon: select one opponent Digimon, then bottom-deck all opponent Digimon with the same level.

**Acceptance gates:**

- The mask exposes all legal combinations through a sequence of legal choices and a finish action.
- PASS is illegal before a mandatory minimum is met and legal once optional completion is allowed.
- Candidate identities remain stable after earlier selected candidates move.
- Separate zones are not decomposed into sequential prompts when printed text offers one combined choice.

## Domain 6: Option, Delay, Plug-In, and Security Disposition

**Impact:** High

**Archetypes hit:** Zephagamon, TS Olympos, DNA Omnimon, Rocks, Chaos Control, Red Hybrid, Royal Knights, Puppets.

**Problem:** Options can resolve then trash, place themselves in battle as Delay, link as Plug-Ins, activate from security, add themselves to hand, stay on field, trash as cost, or re-link from battle. These flows must preserve placement-turn Delay gating, event-gated Delay windows, Option trash observers, color/use requirements, and security disposition.

**Required capabilities:**

- Field Option lifecycle that distinguishes Delay, Plug-In/link, orphaned Plug-In, and ordinary field Option states.
- Option use from hand by effect, with cost ceiling formulas and color-requirement bypass where printed.
- Security activation disposition: add resolving Option to hand, place in battle, trash after resolution, or play a Tamer/Digimon from hand/trash/security.
- Global `OnOptionTrashed` observer context for field Options and linked Options.
- Plug-In link/re-link source zones: hand, free field Option, and linked card on a carrier.
- Delay activation as a visible main-phase or event-gated action with placement-turn restrictions.

**Representative first fixtures:**

- `EX11-072` Zephagamon option: play from hand/trash, place itself in battle, later activate Delay on a Shoto suspend event.
- `BT24-085` TS Olympos Tamer: use one TS Option from hand with use cost less than or equal to opponent memory, then open may-attack.
- `ST22-11` Defense Plug-In F: link from hand or battle area into the specified Digimon.

**Acceptance gates:**

- Delay cannot activate the turn the Option was placed unless printed text explicitly permits it.
- Option self-disposition does not skip security/option trash observers.
- Option effects invoked by another card still resolve through the same Option flow as ordinary use where rules require it.
- Plug-In state survives carrier loss according to printed rules and can become a later selection candidate.

## Domain 7: Formula, Predicate, and Modifier Enforcement

**Impact:** Medium

**Archetypes hit:** Zephagamon, Puppets, TS Olympos, DNA Omnimon, BG Imperial, Alter-S Ladder, Medusamon, Red Hybrid, Rocks.

**Problem:** Several card blockers are no longer missing broad engine movement, but still lack runtime predicates, result bindings, formulas, or enforcement checks. These should be implemented only when attached to a concrete blocked card fixture, not as speculative vocabulary.

**Required capabilities:**

- Result-bound predicates such as "if this effect suspended your Digimon", `any_returned_card`, and binding-present/absent.
- Formula-backed target counts and DP ceilings, including suspended-count floor division and source-stack aggregate formulas.
- Source-relative predicates such as `stack_size_lte_source`, self stack contains name/trait, carrier has keyword, and rules text contains.
- Event predicates such as event target is source/not source, event is DNA, event is effect-initiated, and deletion cause equals Overclock.
- Modifier enforcement for narrow protection: opponent DP reduction, opponent De-Digivolve, source-kind immunity, timing suppression, Vortex can attack players, security-effect suppression, and player-scoped security-placement blocks.

**Representative first fixtures:**

- `EX11-074` Zephagamon: branch only if this effect suspended your own Digimon, then grant DP/immunity.
- `BT16-055` Namakemon: immune to opponent DP reduction and De-Digivolve while security count is at least 3.
- `BT20-101` Zephagamon: suspended-count divided by 2 drives count-capped bottom-deck selection.

**Acceptance gates:**

- Predicates are evaluated at runtime against the intended subject, not parsed and ignored.
- Formula values can feed selection counts, mutation amounts, and effect ceilings.
- Modifiers are enforced at every mutation path they claim to block.
- Broad `CannotBeAffected` is not used for narrow category-specific protection.

## Domain 8: Cross-Card Effect Re-Firing

**Impact:** Medium

**Archetypes hit:** TS Olympos, Dark Masters, Apocalymon-style decks, Royal Knights.

**Problem:** Some effects choose another permanent and activate one of that card's `[On Play]` or `[When Digivolving]` effects outside its normal timing. This is neither a fake play nor a fake digivolution. It must enumerate eligible effects, expose a choice if more than one exists, preserve source attribution, and define once-per-turn interaction.

**Required capabilities:**

- Enumerate registered effects on a selected permanent by timing.
- Present a pending choice when multiple eligible effects exist.
- Re-run the chosen effect with explicit attribution to the card that caused the refire and the card whose text is being activated.
- Preserve once-per-turn semantics unless printed text explicitly permits bypassing them.

**Representative first fixture:**

- `BT24-102` Homeros: choose an Olympos XII Digimon and activate one of its On Play or When Digivolving effects at end of turn after paying the printed suspend cost.

**Acceptance gates:**

- The target permanent is not treated as newly played or newly digivolved.
- Effects already consumed by once-per-turn limits do not refire unless the printed source permits it.
- The mask exposes both target choice and effect choice where applicable.

## Domain 9: Production YAML and Regression Gates

**Impact:** Medium

**Archetypes hit:** Zephagamon, DNA Omnimon, Puppets, Royal Knights, TS Olympos, Red Hybrid.

**Problem:** Some reports list production YAML absence as a gap. That is a valid readiness blocker but not always an engine blocker. YAML authoring should proceed only after the primitive required by the omitted printed text exists.

**Required capabilities:**

- Structural tests that reject load-only stubs for cards marked ready.
- Behavioral tests for every player-visible choice in a card's claimed implemented slices.
- Clear YAML comments for omitted blocked clauses, naming the exact gap ID.
- Tracker update discipline that demotes a reusable gap to card-local authoring once engine and DSL support land.

**Representative first fixtures:**

- `BT20-101` Zephagamon structural readiness test before full behavior.
- One Royal Knights high-frequency card currently carrying gap comments after its source-play primitive lands.
- One Puppets core card after deletion context and replacement dispatch are complete.

**Acceptance gates:**

- No production YAML claims full card readiness while printed text remains omitted.
- Ignored tests name the blocking gap and become active when the primitive lands.
- Raw-Rust helpers are retired or fenced to behavior that cannot yet be expressed, with no no-op placeholders.

## Implementation Order

1. Domain 1: Event context, provenance, and observer fan-out.
2. Domain 2: Replacement and would-leave decisions.
3. Domain 3: Effect-created attacks and combat interrupts.
4. Domain 4: Zone movement and source/material operations.
5. Domain 5: Selection primitives and action-mask surfaces.
6. Domain 6: Option, Delay, Plug-In, and security disposition.
7. Domain 7: Formula, predicate, and modifier enforcement.
8. Domain 8: Cross-card effect re-firing.
9. Domain 9: Production YAML and regression gates.

This order is dependency-aware, not strict waterfall. For example, a narrow DP-budget multi-select can land before all zone movement work if it has a disjoint write set and uses existing pending-selection infrastructure. However, production YAML migration should wait until its required engine primitive is tested.

## Verification Matrix

| Domain | Minimum targeted tests |
|---|---|
| Event context | `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- event_context`; card-shaped Puppets/Royal Knights event tests |
| Replacement | `cargo test --manifest-path code/digimon-engine/Cargo.toml --test replacements -- context`; one inherited replacement card test |
| Combat attack flow | `cargo test --manifest-path code/digimon-engine/Cargo.toml --test combat -- effect_granted_attack`; one DSL may-attack test |
| Zone movement | `cargo test --manifest-path code/digimon-engine/Cargo.toml --test effect_context -- zone_movement`; one source/material play card test |
| Selection | `cargo test --manifest-path code/digimon-engine/Cargo.toml --test selection`; one DP-budget or exact-N card test |
| Option/security | `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- option`; one field Option or security-disposition card test |
| Formula/modifier | `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- predicates formula`; one modifier enforcement card test |
| Effect re-firing | `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- effect_refire`; Homeros card-shaped test |
| YAML gates | `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- <card_filter>` for each card promoted from blocked to ready |

## Tracker Policy

Every closure PR must update the tracker that originally named the gap:

- Engine primitives: `docs/RUST_ENGINE_GAPS.md`
- Legacy archetype-engine tracker entries: `qa/archetype-qa/engine-gaps.md`
- DSL vocabulary and lowering entries: `qa/dsl-vocab-gaps.md`
- Source implementation-run rollups: `qa/archetype-qa/dsl/*.md`
- API authoring details when public helpers are added: `docs/RUST_ENGINE_API.md` or `docs/RUST_DSL_TEST_API.md`

Updates should mark entries as closed, partially closed, narrowed, or demoted to card-local authoring. They should include the test command that proves the new status.

## Success Criteria

The engine closure wave is successful when:

- P0 domains have at least one reusable primitive test and one card-shaped regression each.
- No P0 gap requires hidden auto-selection or raw vector mutation.
- Implemented primitives are reachable from DSL/YAML or documented as engine-only helpers with a follow-up DSL gap.
- Action and tensor contracts remain unchanged.
- At least one high-frequency archetype moves from blocked to mostly-card-authoring after tracker refresh.
- Every newly active card behavior has a targeted Rust test.

## Open Risks

- Some pending-selection shapes may exceed the practical capacity of the current generic action IDs. If that happens, pause and create a contract-change spec instead of expanding action space ad hoc.
- Event fan-out can accidentally double-trigger inherited effects if zone scanners overlap. Tests must cover one observer reachable through exactly one path.
- Stable permanent handles may expose old index assumptions in existing code. Handle-based code should coexist with current indices only where tests prove the index remains stable.
- Broad immunity/modifier checks are easy to over-apply. Category-specific protection must be tested against both blocked and allowed effect kinds.
- Production YAML migration can look like progress while leaving omitted printed choices. Readiness claims must remain tied to behavioral tests, not file count.

## Spec Self-Review

- Placeholder scan: no TBD/TODO placeholder sections remain.
- Scope check: this is a master design spec for engine closure domains, not an implementation plan. Each domain can become its own plan or sub-plan.
- Contract check: the spec explicitly forbids incidental action/tensor expansion.
- Ambiguity check: each domain names required capabilities, first fixtures, and acceptance gates.
