# DSL Vocabulary Gaps Tracker

Resolved DSL gaps have been moved to [qa/resolved-gaps.md](resolved-gaps.md). This file tracks only open gaps and partial slices with remaining follow-up work.

This file accumulates `BLOCKED` verdicts whose `gap_kind` is `dsl` (the engine has the primitive but the DSL lacks a verb that lowers to it). Entries are appended by `/batch-implement-cards-rust-dsl`.

Format per entry:

```
## <CARD_ID> — <clause name>
- Effect text: "..."
- Missing DSL verb / step kind / predicate: ...
- Lowers to engine API: <method on EffectContext that already exists>
- Suggested DSL syntax: <YAML shape>
- First reported: YYYY-MM-DD
```

## Zephagamon / Vortexdramon — remaining battle-engine prep gaps
- Status: partial readiness slice added 2026-05-03. `EX11-074.yaml` now covers static `<Piercing>`, `<Vortex>`, `<Blocker>`, and a focused `battle:` pathway. The regression in `tests/cards_behavioral/ex11/ex11_074.rs` proves that an effect battle deletes the defender through DP battle but is not an attack: it must not trigger Piercing/security and must not leave `pending_attack` populated.
- Rule boundary: `battle:` is the correct DSL step for effects that say a Digimon battles another Digimon. Do not model these as `attack` or force-follow-up attack effects. Attack-only timings and Piercing security continuation remain tied to declared attacks, not effect battles.
- EX11-074 remaining gap: the printed "[When Digivolving] [When Attacking] You may suspend 1 Digimon. If this effect suspended your Digimon..." branch needs a binding/condition result from the suspend step. The DSL can select and suspend, but cannot yet branch on "this effect suspended your Digimon" and bind that cost/result into the follow-up +6000 DP and immunity-until-opponent-turn-ends clause.
- EX11-074 remaining gap: full `[All Turns] [Once Per Turn] When any Digimon suspend, this Digimon may unsuspend. Then, this Digimon may battle 1 opponent Digimon` still needs faithful optional trigger ordering and the unsuspend-then-optional-battle branch. The readiness fixture keeps the battle path focused instead of auto-implementing the whole printed clause.
- BT20-101 remaining gap: Zephagamon needs a formula that counts suspended Digimon, divides that count by 2, and uses the capped result as the number of opponent Digimon selected to place at the bottom of the deck. Existing count-capped multi-select support needs this suspended-count / division formula vocabulary and bottom-deck target movement wiring for the full clause.
- EX11-035 remaining gap: the green Avian/Bird play effect needs a formula DP cap for the target card. The DSL needs a predicate/formula shape that computes the allowed play target's DP ceiling from the printed condition rather than a fixed literal.
- EX11-062 remaining gap: the card needs a conditional `VortexCanAttackPlayer` aura while the opponent has no unsuspended Digimon. The engine has a `VortexCanAttackPlayer` modifier type, but the DSL needs an aura/active_when form that grants it only under that board-state condition.
- Gap kind: hybrid. Some engine primitives exist (`battle:`, static keyword grants, `ModifierType::VortexCanAttackPlayer`), but the remaining Zephagamon clauses need DSL result bindings, formulas, conditional aura lowering, and card-specific faithful branch wiring.
- First reported: 2026-05-03 (Zephagamon Battle Engine Prep Task 4)

## BT22-098 / P-229 — event-gated Delay activation windows
- Effect text: BT22-098: "[Your Turn] When any of your [Arisa Kinosaki] suspend, <Delay> ... 1 of your [Puppet] trait Digimon may digivolve into a [Puppet] and [LIBERATOR] trait Digimon card in the hand with the digivolution cost reduced by 3." P-229: "[Your Turn] When any of your [Mirai Kinosaki]s are played, <Delay> ... 1 of your Digimon may digivolve into a level 6 or lower [LIBERATOR] trait card in the hand with the digivolution cost reduced by 3."
- Status: partially resolved 2026-05-02 for the BT22-098 "when Arisa suspends" slice. `kind: delay` now accepts body-level `active_when`, preserves `event_card_name_contains`, and lowers `trigger: on_suspend` to `DelayTrigger::OnEvent(EffectTiming::OnSuspend)` with the condition evaluated when the event fires.
- Remaining missing DSL verb / step kind / predicate: `on_ally_played` for P-229 is still virtual/skipped by the timing map, and the process body still depends on faithful `effect_initiated_digivolve` support for hand-zone targets, Puppet/LIBERATOR trait filters, and cost reduction.
- Lowers to engine API: `DelayTrigger::OnEvent(EffectTiming::OnSuspend)` plus an event predicate on the delayed Option's `DelayEffect`, preserving the rule that Delay effects cannot activate the turn the option was placed.
- Suggested DSL syntax:
  ```yaml
  - kind: delay
    trigger: on_suspend
    active_when:
      event_card_name_contains: "Arisa Kinosaki"
    process:
      - effect_initiated_digivolve:
          target: { trait: Puppet }
          into: { trait_all: [Puppet, LIBERATOR], zone: hand }
          cost_delta: -3

  - kind: delay
    trigger: on_ally_played
    condition: { event_card_name_is: "Mirai Kinosaki" }
    process:
      - effect_digivolve:
          from: hand
          target_filter: { trait_has: LIBERATOR, level_lte: 6 }
          cost_reduction: 3
  ```
- Gap kind: hybrid (BT22-098 event trigger/predicate lowering is covered; remaining process vocabulary/lowering and P-229 timing support stay open).
- First reported: 2026-04-28 (Puppets archetype assessment)

## EX9-032 / EX7-027 / BT22-036 — replacement cause predicate and `active_when` lowering
- Effect text: "[All Turns] [Once Per Turn] When this Digimon would leave the battle area other than by your effects, by deleting 1 of your Tokens or other [Puppet] trait Digimon, prevent it from leaving."
- Status: PARTIALLY RESOLVED on 2026-05-03. Replacement clauses now preserve replacement subject/source/cause predicates through lowering, apply `active_when`, and can protect a different subject than the replacement source. This is verified for `BT24-040`/`BT24-101`-style TS protection and `BT17-097` Delay replacement continuation.
- Remaining missing DSL verb / step kind / predicate: Puppet/token-specific cost bodies and card-specific EX9-032 / EX7-027 / BT22-036 production YAML/tests remain open until authored and verified.
- Lowers to engine API: replacement evaluator context plus `EffectContext` replacement outcome setters such as `cancel_leave`.
- Suggested DSL syntax:
  ```yaml
  - kind: replacement
    trigger: when_would_leave_battle_area
    active_when:
      replacement_cause_not: own_effect
    process:
      - select_own_permanent:
          as: cost
          filter:
            any_of:
              - kind: token
              - trait_has: Puppet
            other_than_source: true
      - delete_permanent: { target: cost }
      - cancel_replacement: {}
  ```
- Gap kind: partially resolved hybrid. The reusable replacement-context predicate/lowering slice is closed; unimplemented card bodies remain card-authoring work unless they surface new reusable primitives.
- Verification: `cargo test --manifest-path code\digimon-engine\Cargo.toml --test replacements -- cross_permanent context_predicates route_replacements nested_select_substrate --nocapture`; `cargo test --manifest-path code\digimon-engine\Cargo.toml --test option_flow -- replacement_integration::bt17_097 --nocapture`; `cargo test --manifest-path code\digimon-engine\Cargo.toml --test cards_behavioral -- bt24_040 bt24_101 --nocapture`.
- First reported: 2026-04-28 (Puppets archetype assessment)

## BT24-080 — delete all opponent Digimon with the lowest level
- Status: PARTIALLY RESOLVED for the reusable lowest-level permanent predicate on 2026-05-02. `CompiledPredicate::level_matches_aggregate` can match permanents whose top card level equals `CompiledAggregateSelector::LowestLevel` for a player scope, skipping Tamers/Options with no top-card level. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- level_is_lowest_among_opponent_digimon_filters_only_lowest_level_digimon`.
- Effect text: "[On Play] [When Digivolving] [On Deletion] Delete all of your opponent's lowest level Digimon."
- Remaining DSL verb / step kind / predicate: card-specific authoring still needs to wire the aggregate predicate through the surrounding delete-all flow. Repeat target-selection blockers elsewhere are unrelated and remain open.
- Lowers to engine API: engine-side iteration over opponent battle-area permanents plus `delete_permanent` is sufficient once the minimum-level candidate set can be computed.
- Suggested DSL syntax:
  ```yaml
  - delete_all:
      of: opponent
      filter:
        kind: digimon
        level_is: { aggregate: minimum, over: opponent_battle_area }
  ```
- First reported: 2026-04-28

## Rocks archetype refresh — source-selection and cost-payment DSL surface  [G-ROCKS-SOURCE-SELECTION-DSL]
- Effect text: Rocks core repeatedly uses "by trashing any 1/3 [Mineral] or [Rock] trait card(s) from your Digimon's digivolution cards" and "place up to N [Mineral]/[Rock] cards from your trash as bottom digivolution cards." Examples: `EX10-032`, `P-167`, `EX10-036`, `EX10-033`, `EX8-055`, `EX10-028`, `EX8-070`, `EX10-025`.
- Missing DSL verb / step kind / predicate: First-class source-zone selectors for digivolution cards across all of your own stacks, including exact-N, up-to-N with PASS terminator, and single-pick forms. Current DSL has `place_as_bottom_source` and `trash_top_source`, but no `select_source_across_own_permanents` / `select_n_sources_across_own_permanents` step that can bind `(PermanentHandle, source_index)` choices and then trash/place exactly the selected cards.
- Companion engine gap: `docs/RUST_ENGINE_GAPS.md` tracks the engine half under "Cross-permanent count-capped multi-select" and the cost-ordering half under "`.pay_cost()` builder hook for triggered non-cost-reduction effects." This entry tracks the YAML vocabulary and lowering shape that should sit on top of those primitives once available.
- Lowers to engine API: proposed `ctx.select_source_across_own_permanents(...)`, `ctx.select_n_sources_across_own_permanents(...)`, and `EffectBuilder::pay_cost_trash_n_own_sources_by_trait(...)`.
- Suggested DSL syntax:
  ```yaml
  - pay_cost:
      select_sources:
        of: you
        from: any_own_digimon
        count: 1
        filter:
          any_of:
            - trait_has: Mineral
            - trait_has: Rock
        bind_as: trashed_sources
      then:
        - trash_selected_sources: trashed_sources
  ```
  Up-to-N variants should use `max_count: 3` and surface PASS as a legal terminator so RL sees the "stop selecting" choice.
- Gap kind: hybrid (engine selection/action support is still required; DSL needs the reusable vocabulary and lowering once that lands).
- Workaround: Do not auto-pick sources. The Rocks assessment on 2026-04-28 found this to be the core no-approximations blocker for the archetype.
- First reported: 2026-04-28 (Rocks Rust-engine assessment refresh)

---

## Rocks archetype refresh — event-card predicates for Mineral/Rock observers  [G-ROCKS-EVENT-CARD-PREDICATES]
- Effect text: Rocks Tamers and inherited effects gate on the card or host involved in a just-fired event, for example "when any of your Digimon digivolve into a [Mineral] or [Rock] trait Digimon" (`EX8-067`) and "when effects trash digivolution cards of any of your [Mineral] or [Rock] trait Digimon" (`EX10-063`, `P-169`, `EX11-065`).
- DSL predicate coverage: reusable predicate leaves for `trashed_source_trait_has`, `trashed_source_card_id_is`, and `host_permanent_trait_has` are implemented for event payloads with host/source context. Broader aliases such as `digivolving_card_trait_has` remain vocabulary work if card authors need that spelling; existing source-relative leaves such as `source_permanent_trait_has` are not enough unless the lowering receives the correct event subject and distinguishes observer permanent, host permanent, and trashed source card.
- Companion engine gap: the engine still needs full `OnDigivolutionCardTrashed` fan-out with host/source context; see `docs/RUST_ENGINE_GAPS.md` "OnDigivolutionCardTrashed observer timing" and related Rocks entries.
- Updated 2026-04-29: the OnDigivolve half now has runtime event-card and event-target context for normal `Game::digivolve_from_hand`; `event_card_trait_has` reads the new top card, and `target: event_target` binds the just-digivolved permanent. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- on_digivolve_event_card_trait_predicate_matches_new_top_card` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- on_digivolve_event_target_binding_resolves_digivolved_permanent`.
- Updated 2026-04-29: `Game::return_to_hand` source disposition now carries `event_card` / `event_source_card` for the trashed source and `event_host_card` for the former host top card, so `event_card_trait_has` can match sources trashed by that path. Runtime `event_host_permanent()` only exposes the stored host handle if it still resolves to that same card. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- on_digivolution_card_trashed_context_carries_host_and_trashed_source source_trash_host_context_does_not_alias_shifted_permanent` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- on_digivolution_card_trashed_event_card_trait_predicate_matches_trashed_source`. Remaining source-trash gaps include cross-permanent source selection, source-trash paths other than `return_to_hand`, and first-class DSL leaves for trashed-source / host-permanent predicates.
- Updated 2026-05-02: first-class predicate leaves now compile for `event_target_owner`, `host_permanent_trait_has`, `trashed_source_trait_has`, and `trashed_source_card_id_is`; runtime coverage exercises `TriggerSource::SourceTrashedFromStack` with live host/trashed-source context. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- phase3d_event_context`. Remaining source-trash producer paths not covered here should stay open until each producer proves it supplies host/source context rather than relying on fallback guessing.
- Updated 2026-05-03: Task 6 audit found the reusable source-trash payload and DSL predicate leaves already implemented. Added focused regression coverage that an actual `EffectContext::trash_card_source` producer supplies the exact trashed source card and live host into `trashed_source_trait_has`, `trashed_source_card_id_is`, `host_permanent_trait_has`, and `event_target_owner`. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- event_context_bindings group6_dynamic_formulas group7_predicate_batch --nocapture`. No new event payload, predicate, formula, action, or tensor primitive was added.
- Lowers to engine API: `TriggerContext` / event payload fields containing `{host_permanent, trashed_card, trashed_source_index, cause_player}` plus predicate evaluation against those fields.
- Suggested DSL syntax:
  ```yaml
  condition:
    all_of:
      - host_permanent_trait_has: Mineral
      - trashed_source_trait_has: Rock
  ```
  Trait alternatives should compose through existing `any_of`.
- Gap kind: hybrid (requires engine event context plus DSL predicate leaves).
- Workaround: None faithful. Scanning trash after the fact loses which source card was trashed from which host, and can trigger the wrong inherited card.
- First reported: 2026-04-28 (Rocks Rust-engine assessment refresh)

---

## Rocks archetype refresh — authored YAML coverage note
- Assessment target: the `Rocks` / `RockClose` archetype in `data/deck_library.json`, refreshed on 2026-04-28.
- Finding: only `BT14-009`, `BT16-082`, `EX7-074`, and `P-206` currently have Rust YAML under `code/digimon-engine/cards/`; the main `EX8`/`EX10`/`EX11`/`P-167` Rocks shell is not authored in DSL yet.
- Existing DSL gaps reaffirmed by the refresh:
  - `EX11-008 — [When Moving] timing` no longer blocks on the `on_move` token or moved-card event context as of 2026-04-29; card bodies may still need separate target-selection, reveal, or follow-up action primitives.
  - `P-189 — play cost <= filter` was closed on 2026-05-01 for static `play_cost_lte` filters on `select_hand` / `select_trash`; remaining Rocks blockers are tracked separately.
  - `P-206 — Board-color cross-reference predicate` was closed on 2026-05-02 for dynamic `color_matches_any_field_digimon` card predicates; any remaining P-206 Delay, Option, or action-flow blockers are separate.
  - `P-107 — place_self_as_delay_option` remains relevant to `P-107`, `P-039`, `BT23-096`, and related Delay/security disposition effects.
- First reported: 2026-04-28 (Rocks Rust-engine assessment refresh)

---

## BT22-015 — grant "this Digimon may attack" after When Digivolving
- Effect text: "[When Digivolving] ... Then, this Digimon may attack."
- Missing DSL verb / step kind / predicate: `ModifierType::MayAttack` / immediate attack permission is not exposed by the DSL modifier map, and there is no declarative step that lowers to the engine's attack-permission helper once the effect resolves.
- Lowers to engine API: `ModifierType::MayAttack` / `ModifierType::CanAttackUnsuspended` or the force-follow-up attack helper tracked in `docs/RUST_ENGINE_GAPS.md`.
- Suggested DSL syntax: `grant_attack_permission: { target: self, scope: player_or_digimon, expiry: end_of_turn }` for persistent permission, and a distinct `offer_follow_up_attack: { target: self }` when the printed text creates an immediate action prompt.
- First reported: 2026-04-28

## BT22-015 — count same-level pairs in own stack
- Status: PARTIALLY RESOLVED for the reusable same-level source pair formula on 2026-05-02. `CompiledPerSelector::SameLevelPairsInSources` counts source cards below the top card by level and sums `count / 2` per level bucket. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- same_level_pair_count_formula_reads_source_stack_levels`.
- Effect text: "[When Digivolving] For every 2 cards with the same level in this Digimon's digivolution cards, return 1 of your opponent's Digimon to the bottom of the deck."
- Remaining DSL verb / step kind / predicate: repeat-count target selection derived from a formula is still open.
- Lowers to engine API: stack inspection plus repeated `return_to_deck(..., DeckEnd::Bottom)` after each player-visible target selection.
- Suggested DSL syntax: `formula: { aggregate: same_level_pairs, zone: self_sources }` feeding `repeat: <formula>` around a `select_opponent_permanent` + `return_to_deck_bottom` step.
- First reported: 2026-04-28

## BT17-078 — bottom-deck all opponent Digimon sharing chosen level
- Effect text: "[On Play] [When Digivolving] ... place all of your opponent's Digimon with the same level as 1 of their Digimon at the bottom of the deck."
- Missing DSL verb / step kind / predicate: Binding one selected opponent Digimon's level and applying a mass same-level filter to every opponent permanent. The DSL has selection and aggregate helpers, but lacks a reusable "bind selected property, then for-each matching permanents" pattern.
- Lowers to engine API: select opponent permanent, read selected level, then call `return_to_deck(..., DeckEnd::Bottom)` for each opponent permanent whose top card has that level.
- Suggested DSL syntax: `bind_selected: { name: chosen_level, selector: opponent_digimon, property: level }` followed by `for_each_opponent_permanent: { where: { level_eq: "$chosen_level" }, do: return_to_deck_bottom }`.
- First reported: 2026-04-28
---

## BT23-005 — [Your Turn] cost reduction when digivolving into Reptile/Dragonkin
- Effect text: "[Your Turn] When this Digimon would digivolve into a Digimon card with the [Reptile] or [Dragonkin] trait, reduce the digivolution cost by 1."
- Missing DSL verb / step kind / predicate: `CostReductionBody` in `digimon-dsl/src/clause.rs` has no `when_this_digivolves_into` + `target_trait_has` trigger form. Existing variants are `when_playing_this: bool` and `when_any_ally_played: Option<PredicateSpec>`. Neither captures "THIS permanent is the digivolution source AND the target Digimon card has trait X".
- Companion engine gap: `scan_before_pay_cost_reduction` in `game_actions.rs` constructs `EffectReadContext` from the source permanent only — no reference to the digivolution-target hand card is threaded to the condition closure, so a predicate cannot inspect the target's traits.
- Lowers to engine API: `BeforePayCost` timing exists; fixed-amount cost reduction exists. Missing: trigger-predicate variant + target-card threading in `scan_before_pay_cost_reduction`.
- Suggested DSL syntax:
  ```yaml
  - scope: own
    kind: cost_reduction
    reduction_timing: before_pay_cost
    when_this_digivolves_into:
      target_trait_has: [Reptile, Dragonkin]
    active_when: { your_turn: true }
    amount: 1
  ```
- First reported: 2026-04-27 (BT23-005 batch-implement-cards-rust-dsl)

---

## BT21-025 — `attacker_trait_has` predicate on `on_attack_target_change` clauses  [G-ATK-TRAIT-FILTER]
- Effect text: "[Your Turn][Once Per Turn] When any of your [Reptile] or [Dragonkin] trait Digimon's attack targets change, trash your opponent's top security card."
- Missing DSL verb / step kind / predicate: `attacker_trait_has` (and likely `attacker_owner_is_you`) predicates to gate `on_attack_target_change` clauses by the attacking permanent's traits/owner.
- Lowers to engine API: `TriggerContext` already carries `source_permanent` for `PlayerBattleArea` triggers; a predicate could inspect `ctx.trigger_context.source_permanent.traits()`. No new engine API needed.
- Suggested DSL syntax:
  ```yaml
  condition:
    attacker_trait_has: Reptile
    # or any_of: [{ attacker_trait_has: Reptile }, { attacker_trait_has: Dragonkin }]
  ```
- Workaround used: `any_permanent` filter over your battle area with `trait_has: Reptile/Dragonkin` — necessary but not sufficient (over-fires when a non-matching attacker switches target while a matching ally is on board).
- First reported: 2026-04-27 (BT21-025 batch-implement-cards-rust-dsl)

---

## BT24-016 — `condition:` field on `AltPathSpec` (alt-digivolve activation gates)  [G-ALT-PATH-CONDITION]
- Effect text: "[Hand] [Main] If you have [Owen Dreadnought], by placing 1 [Dimetromon] from your trash as any of your [Elizamon]'s bottom digivolution card, it digivolves into this card for a digivolution cost of 3, ignoring digivolution requirements."
- Missing DSL verb / step kind / predicate: `AltPathSpec` (in `digimon-dsl`) has `#[serde(deny_unknown_fields)]` and no `condition:` field. The "If you have [Owen Dreadnought]" gate cannot be expressed; the activated alt-path becomes available whenever the source filter (Elizamon on field) plus the extra_cost (Dimetromon in trash) are satisfied, regardless of Owen presence.
- Lowers to engine API: The activated-digivolve activation check already evaluates predicates via the standard `PredicateSpec` path; the gap is that `AltPathSpec` doesn't carry an extra activation predicate.
- Suggested DSL syntax: add `condition: Option<PredicateSpec>` to `AltPathSpec` (evaluated on top of the existing source-filter / extra-cost gates).
  ```yaml
  alt_paths:
    - kind: activated_digivolve
      condition:
        all_of:
          - exists: { of: you, zone: [battle_area], kind: tamer, name_contains: "Owen Dreadnought" }
      from: { name_contains: "Elizamon" }
      cost: 3
      ignore_requirements: true
      extra_cost: ...
  ```
- First reported: 2026-04-27 (BT24-016 batch-implement-cards-rust-dsl)

---

## EX11-054 — [All Turns] entering-permanent trait gate  [G-ENTERING-PERMANENT-TRAIT]

- Effect text: "[All Turns] When your Digimon are played or digivolve, if any of them have the [Reptile] or [Dragonkin] trait, by suspending this Tamer, <Draw 1>. After, 1 of your Digimon with <Progress> gets +3000 DP for the turn."
- Missing DSL verb / step kind / predicate: `entering_permanent_trait_has` / `digivolving_permanent_trait_has` — BoolPredicate leaves to gate an observer clause on the traits of the card that JUST entered the field or digivolved. The `event_target_trait_has` predicate evaluates `TriggerContext.target_permanent`, which for `OnEnterFieldAnyone` / `OnDigivolve` observers is the OBSERVER's own permanent handle (not the entering/digivolving card).
- Companion engine gap: `trigger_context_for_source` in `effect_queue.rs` sets `target_permanent = source_permanent` (the observer itself) when iterating `TriggerSource::PlayerBattleArea(pid)`. The entering card's handle is not threaded into `TriggerContext`. Additionally, `GameEvent::Digivolve` is "defined for future wiring — not emitted yet" (events.rs), blocking event-log-based detection of the digivolving permanent.
- Updated 2026-04-29: the digivolve half is now partially closed for normal `Game::digivolve_from_hand`: `GameEvent::Digivolve` is emitted and `TriggerSource::Digivolved` populates `TriggerContext.event_permanent` / `event_card` with the just-digivolved permanent and new top card. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- game_event_digivolve_is_emitted_with_new_top_card_and_field_index`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- on_digivolve_event_card_trait_predicate_matches_new_top_card`, and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- on_digivolve_event_target_binding_resolves_digivolved_permanent`. `OnEnterFieldAnyone`, effect-initiated digivolve, DNA digivolve, and breeding-area digivolve remain open.
- Updated 2026-04-29: the enter-field half is now partially closed for normal hand-played battle-area permanents: `TriggerSource::EnteredField` populates `TriggerContext.event_permanent` / `event_card` with the entering permanent and card. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- on_enter_field_anyone_event_card_trait_predicate_matches_entering_card`. Effect-created permanents, token play, option placement, play-from-trash context, and breeding-area observer fan-out remain open.
- Lowers to engine API: covered enter-field and digivolve paths now use `TriggerContext.event_permanent` / `event_card`; remaining dedicated `entering_permanent_trait_has` / `digivolving_permanent_trait_has` syntax, if added, should lower to those fields and keep untested entry/digivolve paths gated until separate dispatch tests exist.
- Suggested DSL syntax:
  ```yaml
  condition:
    any_of:
      - entering_permanent_trait_has: Reptile
      - entering_permanent_trait_has: Dragonkin
  # (same shape for digivolve half with digivolving_permanent_trait_has)
  ```
- Gap kind: hybrid (engine doesn't thread the entering-permanent handle through TriggerContext; DSL has no predicate leaf to read it even if it did).
- Workaround: `kind: raw_rust` no-op placeholder (`ex11_054_all_turns_noop`). All related tests `#[ignore]`'d with `entering_permanent_trigger_context` tag.
- First reported: 2026-04-27 (EX11-054 batch-implement-cards-rust-dsl)

---

## BT21-024 — Opponent security count condition  [G-OPP-SECURITY-COUNT-LTE]

- Effect text: "[On Play][When Digivolving] If your opponent has 5 or fewer security cards, they place 1 card from their hand as the bottom security card. Then, trash their top security card."
- Missing DSL verb / step kind / predicate: `opponent_security_count_lte` — a `PredicateSpec` / `BoolPredicate` leaf that checks the OPPONENT's (not controller's) security stack count. The existing `security_count_lte: u8` field in `PredicateSpec` evaluates `rctx.security_count(rctx.player)` (controller's security). No `of:` field exists on the predicate to redirect the player lookup. A separate `opponent_security_count_lte: Option<u8>` field is needed.
- Lowers to engine API: `rctx.security_count(rctx.opponent())` — `security_count(player_id)` already exists on `EffectReadContext`. The gap is that the predicate evaluator has no branch to call it with the opponent ID.
- Suggested DSL syntax:
  ```yaml
  condition:
    opponent_security_count_lte: 5
  ```
  Alternatively, extend `security_count_lte` to accept an `of:` modifier:
  ```yaml
  condition:
    security_count_lte: { count: 5, of: opponent }
  ```
- Gap kind: dsl (engine primitive exists; predicate evaluator just needs the branch and an `of:` routing parameter or a sibling field).
- Workaround: Clause runs unconditionally (matching DCGO behavior where `trash_top_security` runs outside the inner `if (SecurityCards.Count <= 5)` block). The condition gates only the `select_hand` + `place_on_security` sub-step in DCGO. Negative condition test is `#[ignore = "pending: G-OPP-SECURITY-COUNT-LTE"]`.
- First reported: 2026-04-27 (BT21-024 batch-implement-cards-rust-dsl, Medusamon Batch 8)

---

## BT21-024 — Outer-tail continuation lost when `select_hand` has no candidates  [G-SELECT-EMPTY-OUTER-TAIL]

- Effect text: "[On Play][When Digivolving] ... Then, trash their top security card." — the `trash_top_security` step after `as_selecting_player` must fire even when the opponent has no hand cards.
- Engine gap: `install_select_hand` in `code/digimon-engine/src/effect_context/selections.rs` (lines 177–179) returns early without installing a `PendingSelection` when `valid_action_ids.is_empty()` (opponent has no hand cards). When this early-return fires, no selection callback is ever installed, so `drain_dsl_outer_tail` (which is called from the selection callback in `selections.rs:47`) is never executed. Steps that `park_outer_tail` placed after the `as_selecting_player` block — specifically `trash_top_security` — are silently discarded.
- Root cause: the outer-tail drain relies on the inner select completing through its callback. An empty-selection skip short-circuits before the callback is installed.
- Lowers to engine API: no new method needed. Fix options: (1) in `install_select_hand`, when `valid_action_ids.is_empty()` and the call is not optional, immediately call `drain_dsl_outer_tail(ctx)` before returning; (2) alternatively, make the outer-tail drain happen in the park/skip path rather than only in the callback; (3) add an `on_skip` path analogous to `on_decline` for optional selections that fires the continuation.
- Suggested fix path: option (1) — cheapest, no new API surface:
  ```rust
  if valid_action_ids.is_empty() {
      // No candidates: skip the selection but still drain the outer tail.
      drain_dsl_outer_tail(ctx);
      return;
  }
  ```
- Gap kind: engine (the DSL YAML is correctly structured; the lowering engine loses the continuation in the empty-hand case).
- Workaround: Test for the empty-hand case is `#[ignore = "pending: G-SELECT-EMPTY-OUTER-TAIL"]`. In practice, the YAML behavior deviates from printed card text only when the opponent has an empty hand (rare competitive scenario).
- First reported: 2026-04-27 (BT21-024 batch-implement-cards-rust-dsl, Medusamon Batch 8)

---

## BT17-018 — `lose_count_bound` step verb (count-driven security trash loop)  [G-LOSE-COUNT-BOUND]

- Effect text: "[When Attacking][Once Per Turn] For every 10 cards in both players' trash, trash 1 card from the top of your opponent's security stack."
- Missing DSL verb / step kind / predicate: `lose_count_bound` — a step that calls `trash_top_security(opponent)` N times where N = `floor(combined_trash_count / 10)`. This is a pure count-driven loop with no player choice. The DSL spec draft describes a `lose_count_bound` verb but it is not implemented in `digimon-dsl/src/step.rs` (`StepSpec` enum). The closest existing verb `trash_top_security: { of: opponent }` fires exactly once; there is no `repeat: { count_fn: ... }` combinator in the DSL.
- Lowers to engine API: `EffectContext::trash_top_security(player)` — already exists. The gap is a DSL-side `repeat` or `lose_count_bound` verb that loops based on a computed integer (e.g., `floor_div(card_count_in_zone(trash, any), 10)`).
- Suggested DSL syntax (when `lose_count_bound` is implemented):
  ```yaml
  - lose_count_bound:
      count:
        floor_div:
          - card_count_in_zone: { zone: trash, of: any }
          - 10
      of: opponent
  ```
  Alternatively, a `repeat_n` step verb:
  ```yaml
  - repeat_n:
      count:
        floor_div:
          - card_count_in_zone: { zone: trash, of: any }
          - 10
      body:
        - trash_top_security: { of: opponent }
  ```
- Gap kind: dsl (engine has `trash_top_security`; DSL has no count-computed loop combinator).
- Workaround: `raw_rust: { fn: bt17_018_trash_security_per_ten_trash }` — reads both players' trash sizes, computes `floor((p0 + p1) / 10)`, loops `ctx.trash_top_security(opponent)` that many times. When closed: replace the `raw_rust:` step with the native DSL verb.
- First reported: 2026-04-27 (BT17-018 batch-implement-cards-rust-dsl)

---

## Royal Knights — `on_option_placed` timing lowerer  [G-OPTION-PLACED-TIMING]

- Effect text: `BT13-007` King Drasil_7D6 inherited: "[Breeding] [Your Turn] [Once Per Turn] When an Option card with the [Royal Knight] trait is placed in the battle area, gain 1 memory."
- Missing DSL verb / step kind / predicate: `when: on_option_placed` is accepted by the DSL compiler as `CompiledTiming::OnOptionPlaced`, but the Rust engine timing map returns `None` for it, so no `EffectTiming` is emitted and no clause can fire.
- Companion engine gap: the Rust engine has no `EffectTiming::OnOptionPlaced` dispatch site when a Delay/Training/field Option is placed in the battle area. `BT13-110` Royal Knights of the Purge and `BT20-100` The Last Guardian both make this timing matter for the Royal Knights loop.
- Lowers to engine API: needs a new `EffectTiming::OnOptionPlaced` (or equivalent observer timing) plus a dispatch after Option placement in `Game::dispose_option` / option placement helpers. The trigger context should identify the placed Option card and controller so `event_card_trait_has: "Royal Knight"` can be evaluated.
- Suggested DSL syntax:
  ```yaml
  - scope: inherited
    when: on_option_placed
    active_when: { in_breeding: true }
    once_per_turn: true
    condition: { event_card_trait_has: "Royal Knight" }
    process:
      - gain_memory: 1
  ```
- Gap kind: hybrid (DSL has the token but no lowering target; engine lacks the timing dispatch).
- Workaround: None faithful. The memory-gain trigger is omitted at runtime.
- First reported: 2026-04-28 (Royal Knights archetype assessment)
- Updated 2026-04-29: `when: on_option_placed` now lowers to `EffectTiming::OnOptionPlaced`, and Delay-style Option placement through `Game::play_option_from_hand` supplies the placed Option through `event_card` / `event_permanent`. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- on_option_placed_fires_after_delay_option_enters_battle_area` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- on_option_placed_event_card_trait_predicate_matches_placed_option`.
- Updated 2026-05-02: Group 5 Task 4 covers Link, Training, inherited/security self-placement, and top-card plus inherited breeding-area observer fan-out for `OnOptionPlaced`, with placed Option context available via `event_card` and Link host context via `event_host_permanent` / `event_host_card`. Link placement resumes `OnLink` after placed-option selections settle, and breeding-source `max_per_turn` accounting is covered for this queued observer path. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow -- on_option_placed_fires_for_training_link_and_security_placement_with_event_card link_on_option_placed_selection_resumes_on_link_after_choice_resolves on_option_placed_scans_inherited_sources_under_breeding_top_card once_per_turn_breeding_on_option_placed_observer_fires_once_not_zero`. Transient Standard options remain open because they are not battle-area placements.

---

## Royal Knights — selecting permanents in the breeding area  [G-BREEDING-PERMANENT-SELECTION]

- Effect text: `BT20-083` Omekamon: "[On Deletion] You may place this card as the bottom digivolution card of your [King Drasil_7D6] in the breeding area." Similar Royal Knights effects target or play from the breeding-area King Drasil stack (`BT13-093`, `BT13-110`, `BT13-112`, `EX11-053`, `BT23-072`).
- Status: selection is resolved; effect movement support is partially resolved. `select_own_breeding_permanent` now installs a breeding-specific pending selection and binding without fake battle-area handles. Group 4 also lets `place_as_bottom_source` target the real breeding slot via `BREEDING_TARGET`.
- Companion engine state: `SelectionKind::BreedingPermanent`, `BreedingPermanentSelectionRef`, and phase-scoped breeding select actions cover the player-visible choice. `EffectContext::move_from_breeding_by_effect` and `play_to_breeding_from_hand` cover direct effect movement to/from the real breeding slot.
- Lowers to engine API: `select_own_breeding_permanent` for the choice, `place_as_bottom_source` for tucking under the selected breeding stack, and source-parametric `effect_initiated_digivolve` for non-hand result cards once a source binding is available.
- Suggested DSL syntax:
  ```yaml
  - select_own_permanent:
      bind_as: kd
      filter:
        all_of:
          - name_is: "King Drasil_7D6"
          - zone: [breeding]
      prompt: "Choose your King Drasil_7D6 in breeding"
  ```
  Alternatively, add an explicit sugar step:
  ```yaml
  - select_own_breeding_permanent:
      bind_as: kd
      filter: { name_is: "King Drasil_7D6" }
  ```
- Gap kind: hybrid (the YAML shape exists, but lowering/runtime selection ignore breeding).
- Workaround: None faithful. Auto-targeting the only breeding permanent would hide a player-visible selection and violates the no-approximations policy.
- First reported: 2026-04-28 (Royal Knights archetype assessment)
- Updated 2026-05-02: remaining open follow-ups are breeding-area trigger fan-out (`G-BREEDING-TRIGGER-DISPATCH`) and card-specific optional/filter wrappers, not the basic breeding selection or real-zone movement primitives.

---
