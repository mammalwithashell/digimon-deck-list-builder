# Latest Archetype DSL and Engine Gap Closure Spec

**Date:** 2026-05-03
**Status:** Draft for implementation planning

## Context

Late on 2026-05-02 local time, the repository gained a new batch of archetype Rust DSL and engine gap inputs. The source files themselves are dated 2026-05-03, so this spec uses the source artifact date while treating the "today" set as the late 2026-05-02 local commits.

Primary source inputs:

- `qa/archetype-qa/dsl/2026-05-03-dna-omnimon-dsl-engine-gaps.md`
- `qa/archetype-qa/dsl/2026-05-03-medusamon-cross-archetype-gaps.md`
- `qa/archetype-qa/dsl/alter-s-ladder-2026-05-03.md`
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
- `docs/RUST_ENGINE_GAPS.md`
- `qa/archetype-qa/engine-gaps.md`
- `qa/dsl-vocab-gaps.md`

The recurring pattern is clear: the archetypes are not blocked by isolated card text. They are blocked by reusable pending-selection surfaces, event payloads, source-stack operations, replacement semantics, effect-granted attacks, option/security disposition, and DSL vocabulary that can lower those primitives without raw-Rust placeholders.

## Goals

1. Deduplicate the latest archetype gap inputs into reusable implementation slices.
2. Prioritize slices by cross-archetype reuse and dependency order.
3. Preserve the no-approximations rule: every player-visible choice must be exposed through action masks or `PendingSelection`.
4. Keep `ACTION_SPACE_SIZE`, active tensor layouts, PyO3 exports, RL wrappers, and frontend constants stable unless a separate contract-change plan updates all of them together.
5. Define first fixtures, likely implementation files, DSL shape, and acceptance gates for each slice.

## Non-Goals

- This spec does not implement engine or DSL code.
- This spec does not author broad archetype card batches.
- This spec does not close tracker entries on paper.
- This spec does not permit no-op YAML, raw-Rust escape hatches, hidden auto-selections, or UI-only rules handling.
- This spec does not replace the existing roadmap in `docs/superpowers/specs/2026-04-29-archetype-engine-dsl-gap-roadmap-design.md`; it is the next source-driven consolidation pass.

## Cross-Cutting Contracts

Every implementation slice below must follow these rules:

- Add failing Rust engine or DSL tests before implementation.
- Assert action masks or `PendingSelection` candidates for each player-visible choice, including optional declines and PASS terminators.
- Use printed card text from `data/cards.json` as the rule source, then `docs/RULES_CONTEXT.md`, then ruling references if needed.
- Keep card fixtures narrow: one reusable primitive test plus one card-shaped regression is enough for a capability slice.
- Update `docs/RUST_ENGINE_GAPS.md`, `qa/archetype-qa/engine-gaps.md`, `qa/dsl-vocab-gaps.md`, and the source archetype note when a gap closes, splits, or is demoted to card-local authoring.
- Do not expand `ACTION_SPACE_SIZE` or tensor contracts as a side effect. If a choice cannot fit existing pending-selection action ranges, split the work into an action/tensor contract spec.

## Current Implementation Audit

**Audit date:** 2026-05-03

This audit compares the consolidated gap list against the current Rust DSL schema, lowering code, engine primitives, and tests. It changes the roadmap from "build every named primitive" to "finish only the missing capability edges."

| Area | Current status | Work still needed |
| --- | --- | --- |
| Reveal and remainder selection | Existing DSL/engine primitives cover `reveal_top_deck`, `select_reveal`, `select_count_capped_multi`, `select_ordered_permutation`, `place_remainder_on_deck`, and reveal-pool destination moves. | Keep Slice 1 only for true multi-bucket reveal selection with per-bucket min/max, duplicate prevention across buckets, and bucket-specific result bindings. Do not rebuild ordered remainders. |
| Effect-granted attacks | Attack masks already cover Raid, Collision, Vortex, MayAttack, ForceAttack, attack redirect/cancel, and effect battles. | Keep Slice 2 for immediate attack-as-an-attack flows that install a prompt/action context and preserve attack restrictions. Do not treat `battle:` as sufficient for attack text. |
| Replacement/prevention predicates | Replacement cause/source/subject predicates and core replacement dispatch exist. | Keep Slice 3 for generic cross-permanent DSL replacement/prevention where the source permanent, protected subject, cause, cost, and cancellation all compose correctly. |
| Source-stack movement and aggregates | Source/material counts, stack size, same-level source-pair counts, source selection, trash selected source, and some play-from-material flows exist. | Keep Slice 4 for residual full-stack movement, selected-source play flows, observer-safe aggregate scopes, and source-stack property sums not covered by current formulas. |
| Hybrid/Tamer and union-zone digivolve | Union-zone selection and effect digivolve from hand/source zones exist. `source_treated_as` exists in compiled alt-path data but is not consumed by engine lowering. | Keep Slice 5 for Tamer-as-base/hybrid lowering plus union-zone selection chained into effect digivolve. |
| Event fan-out and bindings | Many event timings, event predicates, dynamic formulas, and binding reads are already present. | Keep Slice 6 only for missing event payload/result binding edges surfaced by card text, especially when several triggers or result values derive from one event. |
| Option, Delay, and security disposition | Delay, event-gated Delay, place-self-as-Delay, `OnOptionPlaced`, and option play/use-requirement flow exist. | Narrow Slice 7 to selected security/option disposition, security-effect suppression, and field-option residuals. Do not rebuild Delay. |
| Formula and predicate vocabulary | Group 6/7-style formula and predicate primitives cover lowest/highest DP/level, binding DP, card counts, material counts, stack size, suspended/security/event predicates, and several source-relative predicates. | Treat Slice 8 as residual vocabulary only. Add nouns/operators only when a specific blocked card needs them. |
| Cross-card effect re-firing | No reusable effect enumeration or "fire another card's effect" primitive was found. | Keep Slice 9 open. |
| Production YAML gates | This is a process/authoring slice, not an engine primitive. | Defer broad YAML conversion until prerequisite primitives close, then demote resolved archetype notes to card-local authoring work. |

## Prioritized Capability Slices

### Slice 1: Multi-Bucket Reveal Selection and Ordered Remainders

**Why first:** This recurs across TS Olympos, Red Hybrid, DNA Omnimon, Alter-S Ladder, Rocks, and many searcher cards. It is mostly selection/DSL work and unlocks a lot of card authoring without changing global contracts.

**Source gaps:**

- `G-TS-MULTI-BUCKET-REVEAL-SEARCH`
- `RH-05`
- DNA Omnimon gap 10
- `G-ASL-05`
- `G-ROCKS-REVEAL-ORDERING`

**Files likely touched:**

- `code/digimon-dsl/src/step.rs`
- `code/digimon-dsl/src/compile.rs`
- `code/digimon-dsl/src/compiled.rs`
- `code/digimon-engine/src/selection.rs`
- `code/digimon-engine/src/action/mask.rs`
- `code/digimon-engine/src/action/decode.rs`
- `code/digimon-engine/src/dsl_cards/step/selections.rs`
- `code/digimon-engine/src/dsl_cards/step/zone_moves.rs`
- `code/digimon-engine/tests/dsl/reveal_buckets.rs`
- `code/digimon-engine/tests/selection/reveal_buckets.rs`

**Capability:** Select one or more cards from a single reveal pool into named buckets, prevent selecting the same reveal card twice, support optional and mandatory bucket semantics, and place the remainder on the top or bottom of deck in printed order or selected order.

**Candidate DSL shape:**

```yaml
- reveal_top_deck: { of: you, count: 3, bind_as: revealed }
- select_reveal_buckets:
    from: revealed
    buckets:
      - bind_as: hybrid
        filter:
          any_of:
            - trait_has: Hybrid
            - trait_has: "Ten Warriors"
        max: 1
      - bind_as: tamer
        filter:
          all_of:
            - kind: tamer
            - has_inherited: true
        max: 1
    no_duplicate_cards: true
    prompt: "Choose cards to add"
- add_to_hand_from_reveal: { card: hybrid }
- add_to_hand_from_reveal: { card: tamer }
- place_remainder_on_deck:
    from: revealed
    position: bottom
    order: chosen
```

**First fixtures:**

- `BT17-009` Flamemon for Red Hybrid: add one Hybrid/Ten Warriors and one inherited-effect Tamer, then bottom the rest.
- `BT24-031` Elecmon for TS Olympos: one Iliad bucket and one TS bucket with a dual-match card that cannot be chosen twice.

**Acceptance gates:**

- The same revealed card cannot satisfy two buckets.
- PASS is legal only when the printed minimum has been satisfied.
- Ordered remainder handling is deterministic and covered by tests.
- Empty or single-bucket reveal effects still run existing outer tails correctly.

**Current status:** Partially implemented. Reveal-pool moves, `select_reveal`, count-capped multi-pick, ordered permutation, and `place_remainder_on_deck` already cover the ordered-remainder half. This slice should only implement bucketed reveal selection with independent result bindings and cross-bucket duplicate prevention.

### Slice 2: Effect-Granted Attacks and Attack-Context Restrictions

**Why second:** Immediate may-attack and attack-without-suspending appear in DNA Omnimon, Medusamon, Alter-S Ladder, BG Imperial, Red Hybrid, Royal Knights, TS Olympos, Zephagamon, and Puppets/Overclock-adjacent paths.

**Source gaps:**

- DNA Omnimon gap 4
- `MED-GAP-03`
- `G-ASL-02`
- `G-BG-05`
- `RH-07`
- Royal Knights immediate may-attack
- `G-TS-IMMEDIATE-MAY-ATTACK`
- `ZEPH-G005`

**Files likely touched:**

- `code/digimon-engine/src/combat.rs`
- `code/digimon-engine/src/game_actions.rs`
- `code/digimon-engine/src/action/mask.rs`
- `code/digimon-engine/src/action/decode.rs`
- `code/digimon-engine/src/effect_context/mod.rs`
- `code/digimon-engine/src/effect_context/selections.rs`
- `code/digimon-engine/src/dsl_cards/step/combat.rs`
- `code/digimon-dsl/src/step.rs`
- `code/digimon-engine/tests/combat/effect_granted_attack.rs`
- `code/digimon-engine/tests/dsl/effect_granted_attack.rs`

**Capability:** Effects can open an optional attack flow after resolution, optionally restrict targets to players or Digimon, optionally attack without suspending, and reuse normal combat legality without pretending the attack is a main-phase action.

**Candidate DSL shape:**

```yaml
- select_own_permanent:
    filter: { kind: digimon, trait_has: Hybrid }
    bind_as: attacker
    optional: true
    prompt: "Choose a Digimon to attack"
- may_attack_now:
    attacker: attacker
    targets: player
    without_suspending: false
```

**First fixtures:**

- `BT18-088` inherited Red Hybrid end-turn player attack.
- `BT24-037` Silphymon or `BT24-085` Dan Yuki & Kanan Yuki for TS Olympos after an effect body.
- `EX9-013` BlitzGreymon for Alter-S Ladder end-turn DNA into optional attack.

**Acceptance gates:**

- PASS/decline is visible before the attack is committed.
- Target restrictions are enforced in the pending attack mask.
- Attack-only timings fire only for attacks, while existing `battle:` effect battles remain non-attacks.
- The pending attack resumes correctly after counter/block/security windows or after selections created by attack-triggered effects.

**Current status:** Partially implemented. Raid, Collision, Vortex, MayAttack, ForceAttack, target-mask replacement, attack redirect/cancel, and `battle:` effect battles already exist. The remaining gap is a reusable immediate attack flow that installs attack-context choices and restrictions rather than resolving as an effect battle.

### Slice 3: Cross-Permanent Replacement, Prevention, and Cause Predicates

**Why third:** Replacement subject/source/cause separation blocks TS Olympos, Medusamon, Puppets, Royal Knights, Alter-S Ladder, and several protection shells. It depends on existing Group 3 replacement work but needs broader subject/source semantics.

**Source gaps:**

- `G-TS-CROSS-PERMANENT-REPLACEMENT-PREVENTION`
- `MED-GAP-04`
- `PUPPETS-G003`
- Royal Knights leave-field prevention
- `G-ASL-01`
- Zephagamon protection/immunity follow-ups

**Files likely touched:**

- `code/digimon-engine/src/effect.rs`
- `code/digimon-engine/src/effect_context/mod.rs`
- `code/digimon-engine/src/effect_queue.rs`
- `code/digimon-engine/src/game.rs`
- `code/digimon-engine/src/game_actions.rs`
- `code/digimon-engine/src/dsl_cards/lower_replacement.rs`
- `code/digimon-engine/src/dsl_cards/predicate.rs`
- `code/digimon-dsl/src/predicate.rs`
- `code/digimon-engine/tests/replacements/cross_permanent.rs`
- `code/digimon-engine/tests/dsl/replacement_context.rs`

**Capability:** Replacement effects can evaluate the would-leave subject, replacement source permanent, cause player/controller, cause kind, and "other than by your effects" filters before offering a cost. Prevention cancels exactly the pending zone move that triggered it.

**Candidate DSL shape:**

```yaml
- kind: replacement
  trigger: when_would_leave_battle_area
  active_when:
    replacement_subject_owner: you
    replacement_subject_trait_has: TS
    replacement_cause_not: own_effect
  cost:
    - trash_top_security: { of: you }
  process:
    - cancel_replacement: {}
```

**First fixtures:**

- `BT24-101` Jupitermon or `BT24-040` Venusmon protects another TS permanent.
- `EX9-020` CresGarurumon plays a Lv6 source instead when another Lv6 would leave.
- `EX9-032` Puppet-style prevention with token or other Puppet cost.

**Acceptance gates:**

- Own-effect exclusions suppress the prompt.
- Declining prevention lets the original move resolve.
- Paying the cost cancels only the pending move that matched the predicate.
- Source permanent and subject permanent may be different.

**Current status:** Partially implemented. Replacement cause/source/subject predicates and the core replacement dispatcher exist, but the DSL still needs a generic cross-permanent authoring path where the replacement source can differ from the protected subject and still cancel exactly the matched event.

### Slice 4: Source-Stack Aggregates, Source-Stack Play, and Material Movement

**Why fourth:** Source-stack operations underpin TS Olympos, Alter-S Ladder, Royal Knights, DNA Omnimon, Rocks, Millenniummon, and Decode/DigiXros follow-ups.

**Source gaps:**

- `G-TS-SOURCE-STACK-AGGREGATES`
- `G-ASL-01`
- `G-ASL-07`
- Royal Knights stack-source multi-selection/play
- DNA Omnimon gaps 3 and 7
- `G-MILL-EFFECT-DNA-TRASH-MATERIAL`
- `G-MILL-DIGIXROS-PLAY-FLOW`
- `G-ROCKS-SOURCE-TRASH-CONTEXT-COMPLETE`

**Files likely touched:**

- `code/digimon-engine/src/permanent.rs`
- `code/digimon-engine/src/game.rs`
- `code/digimon-engine/src/effect_context/mod.rs`
- `code/digimon-engine/src/effect_queue.rs`
- `code/digimon-engine/src/dsl_cards/step/permanent_mutations.rs`
- `code/digimon-engine/src/dsl_cards/step/zone_moves.rs`
- `code/digimon-engine/src/dsl_cards/formula_eval.rs`
- `code/digimon-dsl/src/formula.rs`
- `code/digimon-engine/tests/effect_context/source_stack_operations.rs`
- `code/digimon-engine/tests/dsl/source_stack_aggregates.rs`

**Capability:** Provide reusable operations for trashing all sources under a target, selecting and playing named/material sources, computing source-count aggregates, and moving a full stack or selected sources while preserving owner, order, and event context.

**Candidate DSL shape:**

```yaml
- select_opponent_permanent:
    filter: { kind: digimon }
    bind_as: target
- trash_all_sources: { target: target }

- select_own_sources:
    filter: { name_contains: BlitzGreymon }
    min: 1
    max: 1
    bind_as: blitz_source
- play_selected_sources_free: { source_refs: blitz_source }
```

**First fixtures:**

- `BT24-040` Venusmon trashes all digivolution cards of an opponent stack.
- `EX4-060` Omnimon Alter-S plays one BlitzGreymon source and one CresGarurumon source, then places self bottom security.
- King Drasil/Royal Knights source play from breeding stack after the breeding fan-out slice lands.

**Acceptance gates:**

- Trashing all sources preserves the top card and fires source-trash observers for each removed source.
- Source selection uses stable source references and cannot be invalidated by earlier movement.
- Playing from sources marks the card as played by effect and suppresses/dispatches On Play according to printed text.
- Full-stack return routes sources to the correct owner zones.

**Current status:** Partially implemented. Material counts, stack size, same-level-pair counts, source selection, selected-source trashing, and some play-from-material behavior are already present. Keep this slice focused on full-stack moves, selected-source play flows, observer-safe aggregate scopes, and property-sum formulas still missing from card-shaped tests.

### Slice 5: Hybrid/Tamer Digivolution and Union-Zone Effect Digivolve

**Why fifth:** Red Hybrid makes Tamer-as-base and Digimon-or-Tamer effect digivolve foundational. BG Imperial, Puppets, Chaos Control, and Rocks add union-zone and event-gated variants.

**Source gaps:**

- `RH-01`
- `RH-02`
- `RH-03`
- `G-BG-01`
- `G-BG-06`
- `PUPPETS-G005`
- `G-CHAOS-EFFECT-DIGIVOLVE-FROM-TRASH-CARD-COVERAGE`
- `G-ROCKS-DELAY-EVENT-DIGIVOLVE`

**Files likely touched:**

- `code/digimon-engine/src/game.rs`
- `code/digimon-engine/src/game_actions.rs`
- `code/digimon-engine/src/action/mask.rs`
- `code/digimon-engine/src/action/decode.rs`
- `code/digimon-engine/src/dsl_cards/lower_alt_path_registration.rs`
- `code/digimon-engine/src/dsl_cards/step/play_digivolve.rs`
- `code/digimon-engine/src/effect_context/mod.rs`
- `code/digimon-dsl/src/spec.rs`
- `code/digimon-dsl/src/step.rs`
- `code/digimon-engine/tests/dsl/hybrid_tamer_digivolve.rs`
- `code/digimon-engine/tests/effect_context/effect_digivolve_union_zones.rs`

**Capability:** Normal action masks can expose Tamer-as-level-N digivolution bases for Hybrid routes, and effect-initiated digivolve can choose from Digimon or Tamer bases with hand/trash/reveal/source candidates and dynamic cost formulas.

**Candidate DSL shape:**

```yaml
alt_paths:
  - kind: digivolve
    from:
      zone: battle_area
      filter:
        all_of:
          - kind: tamer
          - color_has: red
    as_level: 3
    cost: 2

- effect_initiated_digivolve:
    base:
      owner: you
      any_of:
        - kind: digimon
        - kind: tamer
    into:
      zone: hand
      filter:
        any_of:
          - trait_has: Hybrid
          - trait_has: Hero
    cost:
      base: printed
      reduce:
        per:
          distinct_tamer_names:
            of: you
            color: red
        delta: 1
```

**First fixtures:**

- `BT17-012` BurningGreymon normal Tamer-base digivolve.
- `BT21-082` Takuya Kanbara effect digivolve from Digimon or Tamer into Hybrid/Hero with dynamic cost reduction.
- `BT17-011` scheduled self-delete tied to the exact permanent created by the effect.

**Acceptance gates:**

- Tamer bases appear in the action mask only for cards with valid alt paths.
- Effect digivolve offers all legal base/card pairs and supports decline when printed.
- Delayed cleanup tracks the exact permanent, not a battle-area index.
- Cost formulas are visible in mask legality and execution rejects stale illegal choices.

**Current status:** Partially implemented. Union-zone selection and effect digivolve from hand/source zones exist, but `source_treated_as` is currently compile data rather than engine-lowered behavior. This slice remains open for Tamer-as-base/hybrid alt-path lowering and an end-to-end union-zone selection into effect digivolve flow.

### Slice 6: Event Fan-Out, Result Bindings, and Trigger Context Completion

**Why sixth:** Several gaps are not new actions; they are missing event payloads or result bindings. They block Zephagamon branches, Puppet triggers, Rocks source-trash observers, Chaos hand/trash observers, and Royal Knights breeding triggers.

**Source gaps:**

- `ZEPH-G002`
- `ZEPH-G008`
- `PUPPETS-G002`
- `PUPPETS-G004`
- `PUPPETS-G005`
- `G-ROCKS-SOURCE-TRASH-CONTEXT-COMPLETE`
- `G-CHAOS-HAND-TRASH-EVENT-OBSERVERS`
- Royal Knights breeding-area trigger fan-out
- `MED-GAP-05`
- `MED-GAP-06`

**Files likely touched:**

- `code/digimon-engine/src/enums.rs`
- `code/digimon-engine/src/effect_queue.rs`
- `code/digimon-engine/src/effect_context/mod.rs`
- `code/digimon-engine/src/game.rs`
- `code/digimon-engine/src/combat.rs`
- `code/digimon-engine/src/dsl_cards/lower_triggered.rs`
- `code/digimon-engine/src/dsl_cards/predicate.rs`
- `code/digimon-dsl/src/predicate.rs`
- `code/digimon-engine/tests/timing_dispatch.rs`
- `code/digimon-engine/tests/dsl/event_context_bindings.rs`

**Capability:** Trigger contexts carry exact event subjects, cards, hosts, causes, attack participants, and action results. DSL predicates can ask about those event fields, and later steps can branch on result bindings such as "this effect suspended your Digimon."

**Candidate DSL shape:**

```yaml
- select_and_suspend:
    target:
      owner: you
      kind: digimon
    bind_result_as: suspended_own
- if_result:
    result: suspended_own
    equals: true
    then:
      - add_dp_modifier:
          target: source
          amount: 6000
          expires: end_of_opponents_turn
```

**First fixtures:**

- `EX11-074` Vortexdramon: branch only if this effect suspended your Digimon.
- `BT13-007` King Drasil in breeding: start-main trigger fan-out while remaining in breeding.
- Rocks source-trash producer paths outside `return_to_hand` carry host and trashed source context.

**Acceptance gates:**

- Result-bound branches do not infer from board state after the fact.
- Breeding-area triggers preserve source-card and controller attribution.
- Event predicates identify the specific moved/played/deleted/trashed card, not a later zone scan.
- Optional triggered effects and once-per-turn counters work through the same queue path.

**Current status:** Partially implemented. Event timings, event predicates, dynamic formulas, and several binding reads already exist. Keep this slice limited to missing event payload fan-out and result-binding fields that are demonstrated by blocked card text.

### Slice 7: Option, Delay, Security Disposition, and Field-Option Completion

**Why seventh:** Group 5 closed major Delay/Link primitives, but the latest archetype docs still call out specific option and security disposition gaps. These should be resolved as reusable option-flow completions, not card-level workarounds.

**Source gaps:**

- DNA Omnimon gap 5
- `RH-10`
- `G-ROCKS-OPTION-SELF-DISPOSITION`
- `ZEPH-G007`
- `G-TS-OPTION-USE-FROM-HAND-BY-COST-CEILING`
- `G-CHAOS-DELAY-OPTION-PLACEMENT-AND-TRIGGER-COVERAGE`
- `G-CHAOS-TRASH-MAIN-OPTION-ACTION`

**Files likely touched:**

- `code/digimon-engine/src/game.rs`
- `code/digimon-engine/src/permanent.rs`
- `code/digimon-engine/src/effect_context/mod.rs`
- `code/digimon-engine/src/dsl_cards/lower_delay.rs`
- `code/digimon-engine/src/dsl_cards/timing_map.rs`
- `code/digimon-engine/src/dsl_cards/step/zone_moves.rs`
- `code/digimon-engine/src/action/mask.rs`
- `code/digimon-dsl/src/step.rs`
- `code/digimon-engine/tests/option_flow/`
- `code/digimon-engine/tests/dsl/option_disposition.rs`

**Capability:** Options can be used from hand by effect with dynamic cost ceilings, move the resolving option to hand/security/battle as printed, activate Delay from event windows, and expose trash-main actions where printed.

**Candidate DSL shape:**

```yaml
- select_hand:
    of: you
    filter:
      kind: option
      trait_has: TS
      play_cost_lte:
        formula:
          per: opponent_memory
          delta: 1
    bind_as: option
    optional: true
- use_option_from_hand_free: { hand_index: option }
- add_this_option_to_hand: {}
```

**First fixtures:**

- `BT24-085` Dan Yuki & Kanan Yuki uses a TS Option from hand under a dynamic ceiling.
- `BT17-094` Ancient Guardian Deity conditional color bypass and security self-to-hand.
- Chaos Control trash-main Option action fixture.

**Acceptance gates:**

- Option disposition is centralized so the same card cannot be both trashed and moved.
- Dynamic option filters affect both masks and decode rejection.
- Delay activation windows respect placement turn and event predicates.
- Security option self-movement works only for the currently resolving security option unless a different card is explicitly selected.

**Current status:** Partially implemented and narrower than originally scoped. Delay lowering, event-gated Delay, self-placement as Delay, `OnOptionPlaced`, and option play/use-requirement flow already exist. Remaining work should target selected security/option disposition, security-effect suppression, and field-option residuals.

### Slice 8: Formula, Predicate, and Aggregate Extensions

**Why eighth:** Several latest gaps need small formula/predicate vocabulary once the larger action surfaces exist. They should land in batches with focused parse/eval tests.

**Source gaps:**

- `ZEPH-G003`
- `G-TS-SOURCE-STACK-AGGREGATES`
- DNA Omnimon gaps 6 and 7
- `MED-GAP-08`
- `G-ASL-09`
- `G-BG-04`
- Zephagamon DP ceiling and conditional Vortex aura notes

**Files likely touched:**

- `code/digimon-dsl/src/formula.rs`
- `code/digimon-dsl/src/predicate.rs`
- `code/digimon-dsl/src/compile.rs`
- `code/digimon-engine/src/dsl_cards/formula_eval.rs`
- `code/digimon-engine/src/dsl_cards/predicate.rs`
- `code/digimon-engine/src/dsl_cards/lower_aura.rs`
- `code/digimon-engine/tests/dsl/formula_aggregates.rs`
- `code/digimon-engine/tests/dsl/predicate_aggregates.rs`

**Capability:** DSL can express suspended-count division, capped multi-select counts, source-count aggregates, highest/lowest property sets, selected-property binding, same-level pair counts, and conditional auras such as `VortexCanAttackPlayer` while the opponent has no unsuspended Digimon.

**Candidate DSL shape:**

```yaml
count:
  formula:
    per:
      card_count_in_zone:
        of: any
        zone: battle_area
        filter: { is_suspended: true }
    divide_by: 2
    cap: 3

filter:
  source_count_matches_aggregate:
    selector: lowest
    of: opponent
```

**First fixtures:**

- `BT20-101` Zephagamon suspended-count divide-by-2 capped bottom-deck selection.
- `BT24-030` Neptunemon bottom-decks opponent Digimon with the fewest sources.
- Medusamon highest/lowest aggregate target operation.

**Acceptance gates:**

- Formula parse tests cover every new leaf and operator.
- Runtime evaluation uses live board state and does not snapshot dynamic auras incorrectly.
- Aggregate ties include every tied permanent where printed.
- Selection masks derive their count ceilings from the same formula used by execution.

**Current status:** Partially implemented and mostly residual. Existing formula/predicate batches cover lowest/highest DP/level, binding DP, card counts, material counts, stack size, same-level source pairs, suspended/security/event predicates, and several source-relative predicates. Add only the remaining nouns/operators required by currently blocked cards.

### Slice 9: Cross-Card Effect Re-Firing and Effect Enumeration

**Why ninth:** This is less broad than selection/attack/replacement, but it blocks TS Olympos Homeros, Puppets/Nyabootmon, Jesmon-style effects, and Apocalymon/Dark Masters-like future work.

**Source gaps:**

- `G-TS-CROSS-CARD-EFFECT-REFIRING`
- `PUPPETS-G002`
- `qa/archetype-qa/engine-gaps.md` "Activate Another Card's When Digivolving Effect"

**Files likely touched:**

- `code/digimon-engine/src/effect.rs`
- `code/digimon-engine/src/effect_queue.rs`
- `code/digimon-engine/src/effect_context/mod.rs`
- `code/digimon-engine/src/selection.rs`
- `code/digimon-engine/src/action/mask.rs`
- `code/digimon-engine/src/dsl_cards/step/mod.rs`
- `code/digimon-dsl/src/step.rs`
- `code/digimon-engine/tests/effect_context/effect_refiring.rs`
- `code/digimon-engine/tests/dsl/effect_refiring.rs`

**Capability:** Select a permanent or source card, enumerate eligible effects by timing, present a player-visible effect choice when multiple are available, and enqueue the selected effect with correct source attribution and once-per-turn handling.

**Candidate DSL shape:**

```yaml
- select_own_permanent:
    filter: { trait_has: "Olympos XII" }
    bind_as: olympus
    optional: true
- activate_effect_of:
    target: olympus
    timings: [on_play, when_digivolving]
    attribution: target_source
    optional: true
```

**First fixtures:**

- `BT24-102` Homeros activates an Olympos XII `[On Play]` or `[When Digivolving]` effect at end of turn.
- `BT22-042` Nyabootmon activates one of its own `[When Digivolving]` effects from another trigger.

**Acceptance gates:**

- The chosen effect does not pretend the target card was just played or digivolved unless the refired effect itself requires that event.
- Once-per-turn accounting is explicit.
- Multiple eligible effects produce a mask-visible choice.
- No raw iteration over `effect_list()` remains in new card YAML.

**Current status:** Open. No reusable cross-card effect enumeration or effect re-firing primitive was found in the current DSL/engine tests.

### Slice 10: Production YAML Authoring Gates

**Why last:** Several source docs include "production YAML missing" as a gap. That is real readiness work, but it should follow reusable primitive closure so card YAML does not fossilize approximations.

**Source gaps:**

- DNA Omnimon gap 11
- `PUPPETS-G001`
- `ZEPH-G001`
- `G-MILL-CARD-YAML-REGRESSION-BATCH`
- Red Hybrid card-local authoring backlog
- TS Olympos card-local authoring backlog
- Alter-S Ladder authored-card coverage table

**Files likely touched:**

- `code/digimon-engine/cards/**`
- `code/digimon-engine/tests/cards_behavioral/**`
- `qa/archetype-qa/dsl/*.md`
- `qa/qa-reports/validated_cards_dsl.json`

**Capability:** Once reusable primitives land, migrate high-frequency cards to production Rust YAML with behavioral tests and no raw-Rust placeholders.

**Acceptance gates:**

- Each card starts with a failing `cards_behavioral` test that names the printed clause being covered.
- YAML uses only supported DSL vocabulary or documented card-owned Rust effects when the card truly requires bespoke behavior.
- Existing ignored tests are either unignored and passing or updated to reference the still-open reusable gap.
- Readiness docs distinguish `card-local authoring` from `engine-gap` and `dsl-gap`.

**Current status:** Open as an authoring/process gate. This is not an engine primitive. Defer broad production YAML conversion until prerequisite capability slices close, then convert only cards whose remaining gaps are card-local authoring work.

## Execution Order

Remaining work after the current implementation audit:

1. True multi-bucket reveal selection with independent bucket bindings and duplicate prevention across buckets.
2. Immediate effect-granted attack flow that installs attack-context legal choices and restrictions.
3. Generic cross-permanent replacement/prevention completion.
4. Source-stack residuals: full-stack movement, selected-source play, and missing aggregate scopes.
5. Hybrid/Tamer alt-path lowering and union-zone selection chained into effect digivolve.
6. Missing event payload/result bindings, limited to blocked card text.
7. Narrow option/security disposition cleanup.
8. Residual formula/predicate vocabulary not already covered by existing Group 6/7 tests.
9. Cross-card effect re-firing and effect enumeration.
10. Production YAML authoring gates after prerequisite capability closure.

Already-covered primitives should not receive new roadmap groups unless a card-shaped regression exposes a behavioral bug in that primitive.

## Tracker Update Policy

When implementing a slice:

- Add a dated status note to the relevant open entry in `docs/RUST_ENGINE_GAPS.md`.
- Add or update the corresponding DSL entry in `qa/dsl-vocab-gaps.md`.
- Update `qa/archetype-qa/engine-gaps.md` only when the legacy/Python-facing tracker still owns the gap or references the Rust blocker.
- Update every source archetype doc listed in the slice with either "resolved by <test command>" or "demoted to card-local authoring after <primitive> landed."
- Do not mark an archetype `ready` until its card-shaped tests pass and its production YAML is present.

## Verification Matrix

Each slice should end with targeted commands first, then broader gates when the touched surface warrants it:

```powershell
cargo test --manifest-path code\digimon-engine\Cargo.toml --test dsl -- <focused_filter>
cargo test --manifest-path code\digimon-engine\Cargo.toml --test <engine_test_binary> -- <focused_filter>
cargo test --manifest-path code\digimon-engine\Cargo.toml --test cards_behavioral -- <card_filter>
cargo test --manifest-path code\digimon-engine\Cargo.toml --test mask_and_tensor -- <mask_filter>
```

Run RL/PyO3 checks only when a slice changes exported constants, observation/action metadata, Python bindings, or training wrapper assumptions:

```powershell
$env:DIGIMON_BACKEND='rust'; python -m pytest code\tests\rl -v
$env:DIGIMON_BACKEND='rust'; python -m pytest code\engine_py_legacy\tests\engine\test_rust_backend_parity.py -v
```

If any slice changes `ACTION_SPACE_SIZE`, tensor shape, layout metadata, or PyO3 exports, stop and create a separate action/tensor contract spec before merging.

## Self-Review

- **Implementation audit:** Existing primitives are called out so roadmap work targets only missing capability edges.
- **Spec coverage:** The source docs' recurring gap families are represented: reveal buckets, effect attacks, replacements, source stacks, Hybrid/Tamer digivolve, event fan-out/result bindings, options/security disposition, formulas/predicates, effect re-firing, and production YAML authoring.
- **No placeholders:** This spec avoids `TBD`/`TODO` placeholders and names concrete files, fixtures, DSL shapes, and acceptance gates.
- **Type/name consistency:** Gap names are preserved from the source docs where useful, and new slice names are capability-centric rather than card-centric.
- **No-approximations compliance:** Every slice explicitly requires action masks or pending selections for choices and forbids hidden auto-selection or raw-Rust readiness claims.
