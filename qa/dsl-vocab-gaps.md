# DSL Vocabulary Gaps Tracker

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
- Missing DSL verb / step kind / predicate: replacement clauses accept `active_when`, but `lower_replacement` ignores it. The DSL also lacks a replacement-context predicate for "other than by your effects", so this prevention effect cannot be faithfully gated by cause/controller.
- Lowers to engine API: the replacement evaluator needs to expose replacement cause/controller to DSL predicates and apply `active_when` before presenting the cost/selection branch.
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
- Gap kind: hybrid (DSL predicate/lowering plus engine replacement-context plumbing).
- First reported: 2026-04-28 (Puppets archetype assessment)

## Chaos Control — effect-initiated digivolve from trash
- Effect text: EX11-005 Yaamon / EX11-069 Yuuki / BT21-100 The Digimon I Designed / BT24-080 Megidramon all digivolve a battle-area Digimon into a card in trash, sometimes for free and sometimes with a reduced cost.
- Status: resolved for selected live card bindings in Group 4 (2026-05-02). `effect_initiated_digivolve` now accepts `source:` as a source-zone-parametric binding while preserving legacy `from_hand:`.
- Lowers to engine API: `EffectContext::effect_initiated_digivolve_from_source` / `Game::effect_initiated_digivolve_from_source`, with card-handle bindings resolved from hand, trash, security, material stack, or reveal pool.
- Suggested DSL syntax:
  ```yaml
  - effect_initiated_digivolve:
      target: base
      from:
        zone: trash
        of: you
        card: evo
      cost: { reduce: 1 }
      ignore_requirements: false
  ```
- First reported: 2026-04-28
- Regression coverage: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test effect_context -- effect_digivolve_from_zones`, plus `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- group4_zone_movement`.

## BT24-080 — delete all opponent Digimon with the lowest level
- Effect text: "[On Play] [When Digivolving] [On Deletion] Delete all of your opponent's lowest level Digimon."
- Missing DSL verb / step kind / predicate: aggregate minimum-level predicate plus mass delete. `for_each` can delete all permanents matching a predicate, but the DSL has no predicate for "level equals the minimum level among opponent Digimon."
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

## EX4-011 — DP deletion threshold from shared trash count
- Status: PARTIAL after 2026-05-01. Runtime evaluation for `dp_lte` / `dp_gte` permanent predicates is closed for literal thresholds and existing `FormulaSpec` variants. This entry remains open only for the missing `shared_trash_count` / bucket formula vocabulary shown below.
- Effect text: "For every 10 total cards in both player's trashes, add 2000 to the maximum DP you can choose with DP-based deletion effects."
- Missing DSL verb / step kind / predicate: formula support for cross-player trash count buckets inside a `dp_lte` selection predicate. Existing formula vocabulary covers some modifier values, but not a target-filter threshold derived from `floor((your_trash + opponent_trash) / 10) * 2000`.
- Lowers to engine API: read both players' trash lengths, compute the threshold, then install the normal opponent-permanent selection and `delete_permanent` callback.
- Suggested DSL syntax:
  ```yaml
  dp_lte:
    formula:
      base: 7000
      per: shared_trash_count
      bucket: 10
      delta: 2000
  ```
- First reported: 2026-04-28

---

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
- Missing DSL verb / step kind / predicate: Reusable predicate leaves for the event payload: `digivolving_card_trait_has`, `trashed_source_trait_has`, `trashed_source_card_id_is`, and `host_permanent_trait_has`. Existing source-relative leaves such as `source_permanent_trait_has` are not enough unless the lowering receives the correct event subject and distinguishes observer permanent, host permanent, and trashed source card.
- Companion engine gap: the engine still needs full `OnDigivolutionCardTrashed` fan-out with host/source context; see `docs/RUST_ENGINE_GAPS.md` "OnDigivolutionCardTrashed observer timing" and related Rocks entries.
- Updated 2026-04-29: the OnDigivolve half now has runtime event-card and event-target context for normal `Game::digivolve_from_hand`; `event_card_trait_has` reads the new top card, and `target: event_target` binds the just-digivolved permanent. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- on_digivolve_event_card_trait_predicate_matches_new_top_card` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- on_digivolve_event_target_binding_resolves_digivolved_permanent`.
- Updated 2026-04-29: `Game::return_to_hand` source disposition now carries `event_card` / `event_source_card` for the trashed source and `event_host_card` for the former host top card, so `event_card_trait_has` can match sources trashed by that path. Runtime `event_host_permanent()` only exposes the stored host handle if it still resolves to that same card. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- on_digivolution_card_trashed_context_carries_host_and_trashed_source source_trash_host_context_does_not_alias_shifted_permanent` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- on_digivolution_card_trashed_event_card_trait_predicate_matches_trashed_source`. Remaining source-trash gaps include cross-permanent source selection, source-trash paths other than `return_to_hand`, and first-class DSL leaves for trashed-source / host-permanent predicates.
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
  - `P-206 — Board-color cross-reference predicate` remains the specific blocker for `P-206` Delay filtering.
  - `P-107 — place_self_as_delay_option` remains relevant to `P-107`, `P-039`, `BT23-096`, and related Delay/security disposition effects.
- First reported: 2026-04-28 (Rocks Rust-engine assessment refresh)

---

## BT22-008 / BT22-017 — inherited end-of-turn DNA digivolve registration
- Effect text: "[End of Your Turn] This Digimon and another of your Digimon may DNA digivolve into a Digimon card in your hand."
- Missing DSL verb / step kind / predicate: Lowering for existing `alt_path_registration` declarative clauses with `kind: dna_digivolve`. YAML examples can spell the clause, but `code/digimon-engine/src/dsl_cards/mod.rs` currently leaves this declarative form in the unlowered catch-all branch.
- Lowers to engine API: the same alternate-path registration and action-mask channel used by normal DNA digivolve costs, producing a player-visible pending/action path rather than an automatic end-of-turn digivolve.
- Suggested DSL syntax: keep the existing `alt_path_registration` shape and require lowering for inherited clauses, including `timing: end_of_your_turn`, `kind: dna_digivolve`, material filters, target hand-card filter, and cost override.
- Also blocks: `BT12-021` Veemon and `BT12-047` Wormmon in BG Imperial. Their inherited text is "[End of Your Turn] This Digimon and any of your other Digimon may DNA digivolve into a Digimon card in the hand."
- First reported: 2026-04-28

## BG Imperial DNA cards — YAML `dna_costs` authoring / production data population
- Effect text: "[DNA Digivolve] Blue Lv.4 + Green Lv.4 : Cost 0" and equivalent BG Imperial DNA requirements.
- Missing DSL verb / step kind / predicate: RESOLVED 2026-05-02 for top-level `alt_paths: [{ kind: dna_digivolve, ... }]` authoring into runtime `CardData.dna_costs`.
- Lowers to engine API: `CardData.dna_costs`, consumed by the DNA digivolve action-mask branch and `Game::initiate_dna_digivolve`.
- Covered by: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- dsl_dna_alt_path_enriches_card_data_dna_costs`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dna_digivolve_user_action -- authored_dna_alt_path_makes_dna_action_legal_for_bt20_016`, and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt20_016_has_dna_digivolve_alt_path`.
- Full verification: `cargo test --manifest-path code/digimon-engine/Cargo.toml`, `DIGIMON_BACKEND=rust python -m pytest code/engine_py_legacy/tests/engine/test_rust_backend_parity.py -v`, and `python -m pytest code/tests/rl -v`.
- Remaining limits: inherited/end-of-turn `alt_path_registration` DNA clauses are still tracked by the preceding entry; this closure covers top-level printed DNA digivolve card data.
- First reported: 2026-04-28 (BG Imperial assess-rust-engine-archetype)

## Group 8 — scoped DigiXros aliases, ACE Overflow metadata, and reveal overlays
- Effect text: "also treated as [X] for DigiXros", `<ACE> Overflow -N`, and reveal effects whose revealed cards should be evaluated with temporary name/kind identity only while in the reveal zone.
- Status: RESOLVED 2026-05-02 for the planned vocabulary/data slices.
- Lowered runtime surfaces: `CardData.digixros_aliases` / compiled `digixros_aliases` for DigiXros-only material matching; `CardData.ace_overflow` for memory loss when ACE cards leave battle-area stacks or are removed from under a stack; and `RevealOverlay` on `CardSource` for reveal-zone predicate evaluation until destination movement clears the overlay.
- Covered by: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- digixros`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test ace_overflow`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- reveal`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test keyword_parsing`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test mask_and_tensor`, and full `cargo test --manifest-path code/digimon-engine/Cargo.toml`.
- Remaining limits: full DigiXros play flow, reveal overlays outside explicit reveal-zone predicates, and unrelated immediate-attack/declarative-keyword gaps remain separate entries.
- First reported: consolidated from Group 8 token/card-data gap planning.

## BT22-015 — grant "this Digimon may attack" after When Digivolving
- Effect text: "[When Digivolving] ... Then, this Digimon may attack."
- Missing DSL verb / step kind / predicate: `ModifierType::MayAttack` / immediate attack permission is not exposed by the DSL modifier map, and there is no declarative step that lowers to the engine's attack-permission helper once the effect resolves.
- Lowers to engine API: `ModifierType::MayAttack` / `ModifierType::CanAttackUnsuspended` or the force-follow-up attack helper tracked in `docs/RUST_ENGINE_GAPS.md`.
- Suggested DSL syntax: `grant_attack_permission: { target: self, scope: player_or_digimon, expiry: end_of_turn }` for persistent permission, and a distinct `offer_follow_up_attack: { target: self }` when the printed text creates an immediate action prompt.
- First reported: 2026-04-28

## BT22-015 — count same-level pairs in own stack
- Effect text: "[When Digivolving] For every 2 cards with the same level in this Digimon's digivolution cards, return 1 of your opponent's Digimon to the bottom of the deck."
- Missing DSL verb / step kind / predicate: Formula support for "number of same-level pairs in this Digimon's digivolution cards" and repeat-count target selection derived from that formula.
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

## EX11-008 — [When Moving] timing (DSL half — see engine-gaps.md for engine half)

- Effect text: "[When Moving] [On Play] 1 of your Digimon with the [Reptile] or [Dragonkin] trait gains <Raid> and +3000 DP for the turn."
- Status: fixed for the timing token and direct runtime event context on 2026-04-29. `when: on_move` lowers to `EffectTiming::OnMove`; `event_target_trait_has` can inspect the moved permanent/card from `TriggerSource::MovedFromBreeding`. Verified by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- on_move_event_target_trait_predicate_matches_moved_permanent`.
- Remaining DSL work: card-specific bodies still need any additional step verbs/predicates they print, such as EX11-008's target grant body and BT16-082's reveal/add-to-hand/hatch tail.
- Gap kind: hybrid (this entry tracks the DSL half; engine half tracked separately).
- First reported: 2026-04-27 (EX11-008 batch-implement-cards-rust-dsl)

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

## BT24-082 / BT21-081 — Immediate optional attack within effect resolution  [G-MAY-ATTACK-NOW]
- Effect text (BT24-082): "[Your Turn] When any of your Digimon digivolve into a [Reptile] or [Dragonkin] Digimon, by suspending this Tamer, that Digimon gets +3000 DP for the turn. Then, it may attack."
- Effect text (BT21-081): "[End of Your Turn] By suspending this Tamer, 1 of your Digimon with the [Reptile] or [Dragonkin] trait gains <Piercing> for the turn. Then, that Digimon attacks."
- Missing DSL verb / step kind / predicate: No DSL verb for an immediate attack (optional or mandatory) on a specific named Digimon mid-effect-resolution. The DCGO fires `SelectAttackEffect` (BT24-082) or `SetCanNotSelectNotAttack` (BT21-081) within the effect coroutine — i.e., an in-effect attack, not an end-of-turn attack-window action.
- Engine gap: `ModifierType::MayAttack` and `ModifierType::ForceAttack` exist in `enums.rs` but are NOT in `lookup_modifier_type` (`modifier_map.rs`), so they cannot be granted via `add_modifier:`. Even if registered, these modifiers target the EOT Execute/Vortex attack window — they don't trigger an immediate, mid-effect attack on a specific permanent.
- Lowers to engine API: A new `EffectContext::may_attack_now(target: PermanentHandle)` / `force_attack_now(target: PermanentHandle)` primitive would be needed, plus a corresponding DSL step verb.
- Suggested DSL syntax:
  ```yaml
  - may_attack_now: { target: tgt }          # optional — player may choose to attack
  - force_attack_now: { target: tgt }         # mandatory — Digimon must attack
  ```
- Gap kind: hybrid (DSL lacks the verb AND engine lacks the mid-effect attack primitive).
- Workaround: Omit the "may attack" / "then attacks" sub-clause. Test `#[ignore]`'d with `G-MAY-ATTACK-NOW` tag.
- First reported: 2026-04-27 (BT24-082 batch-implement-cards-rust-dsl)

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

## P-189 — [Security] play cost ≤ 4 filter on select_hand / select_trash  [G-PLAY-COST-LTE]

- Status: CLOSED on 2026-05-01. `play_cost_lte` is now parsed, compiled, evaluated against `CardData::play_cost`, and wired into `select_hand` / `select_trash` valid-action filtering. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- group7_predicate_batch parse_group7_predicate_leaves`.
- Effect text: "[Security] You may play 1 card with the [LIBERATOR] trait and a play cost of 4 or less from your hand or trash without paying the cost."
- Missing DSL verb / step kind / predicate: `play_cost_lte` (or `cost_lte`) — a `PredicateSpec` leaf that checks `CardData::play_cost <= N`. `PredicateSpec` in `digimon-dsl/src/predicate.rs` has no cost-comparison field. The `eval_card_fields` function in `code/digimon-engine/src/dsl_cards/predicate.rs` handles `level_eq`, `level_lte`, `level_gte`, `color_is`, `trait_has`, `name_*`, `card_number_is` — but no `play_cost` / `cost_lte` / `cost_gte` variant.
- Companion issue: `install_select_hand` and `install_select_trash` in `code/digimon-engine/src/dsl_cards/step/selections.rs` currently use `|_game, _idx| true` (accept-all filter, Phase 2b), so even if `play_cost_lte` were added to `PredicateSpec`, it would not be evaluated until Phase 2b filter wiring is completed.
- Lowers to engine API: no new engine method needed. Fix requires: (1) add `play_cost_lte: Option<u32>` (and optionally `play_cost_gte`) to `PredicateSpec`; (2) add a branch in `eval_card_fields` to check `card_data.play_cost <= n`; (3) wire the filter predicate into `install_select_hand` and `install_select_trash`.
- Suggested DSL syntax:
  ```yaml
  filter:
    all_of:
      - trait_has: LIBERATOR
      - play_cost_lte: 4
  ```
- Gap kind: dsl (engine already stores `play_cost` on `CardData`; the DSL/lowering path just lacks the predicate leaf).
- Workaround: none needed for static play-cost caps after 2026-05-01. Previously ignored tests for incorrect candidate filtering can be unignored when updating affected card suites.
- First reported: 2026-04-27 (P-189 batch-implement-cards-rust-dsl)

---

## BT5-008 — `other: true` predicate not evaluated in `eval_permanent_fields`  [G-OTHER-PREDICATE-UNEVALUATED]

- Effect text: "[Your Turn] Your other [Gaossmon] all get +3000 DP."
- Missing DSL verb / step kind / predicate: `other: true` is present in `PredicateSpec` (field `pub other: Option<bool>` at line 81) and compiles to `CompiledPredicate.other`, but `eval_permanent_fields` in `code/digimon-engine/src/dsl_cards/predicate.rs` (lines 381–436) does NOT check `pred.other`. The field is silently ignored at evaluation time, so a filtered aura with `other: true` would also buffer the source card itself (over-fires).
- Lowers to engine API: no new engine method needed. Fix requires: add a check in `eval_permanent_fields` after the zone/owner checks: `if pred.other == Some(true) { if let Some(src) = rctx.source_permanent { if handle == src { return false; } } }`.
- Suggested DSL syntax: already in the DSL spec as `other: true`; the evaluator just needs the additional guard.
- Gap kind: dsl (the predicate field compiles correctly; the runtime evaluator doesn't act on it).
- Workaround: Aura will over-fire (also buffs the source card). The self-exclusion behavioral test is `#[ignore = "BLOCKED: G-OTHER-PREDICATE-UNEVALUATED"]`. Secondary to G-DECLARATIVE-KEYWORD (declarative tick gap blocks ALL filtered aura runtime; self-exclusion only matters once that is fixed).
- First reported: 2026-04-27 (BT5-008 batch-implement-cards-rust-dsl, Medusamon archetype)

---

## BT5-008 — Player-level flood-gate modifier not installable from DSL  [G-PLAYER-FLOOD-GATE-DSL]

- Effect text: "[Opponent's Turn] Your opponent can't reduce digivolution costs."
- Status: CLOSED for DSL vocabulary and direct runtime primitives as of 2026-05-01. The DSL now supports `target_player` on `kind: flood_gate` and an explicit `add_player_modifier` step, and the engine has a `CannotReduceDigivolveCost` modifier with digivolve-only cost-reduction enforcement.
- Remaining related blocker: passive/static field effects still depend on G-DECLARATIVE-KEYWORD. BT5-008 compiles to the native player-targeted flood_gate below, but the engine still needs the global declarative field-effect dispatcher before face-up static rookies install their modifiers from board state.
- Implemented DSL syntax:
  ```yaml
  - kind: flood_gate
    active_when: { opponents_turn: true }
    target_player: opponent
    modifier: CannotReduceDigivolveCost
  ```
- Gap kind: closed vocabulary/runtime primitive gap; remaining passive dispatch gap is tracked separately by G-DECLARATIVE-KEYWORD.
- Former workaround removed for BT5-008: raw_rust no-op placeholder (`bt5_008_opp_cannot_reduce_digivolve_cost`).
- First reported: 2026-04-27 (BT5-008 batch-implement-cards-rust-dsl, Medusamon archetype). Closed: 2026-05-01 (floodgate DSL flexibility pass).

---

## P-137 — Opponent adds top security card to hand  [G-ADD-TOP-SECURITY-TO-HAND]
- Effect text: "[Your Turn][Once Per Turn] When this Digimon's attack target is switched, your opponent adds the top card of their security stack to the hand."
- Missing DSL verb / step kind / predicate: `add_top_security_to_hand` — a verb to move the top security card to the owner's hand (as opposed to `trash_top_security` which trashes it). No `add_top_security_to_hand` step exists in `digimon-dsl/src/step.rs` (`StepSpec` enum).
- Companion engine gap: `EffectContext` has `trash_top_security(player)` but no `add_top_security_to_hand(player)`. The engine move primitive (pop from security vec, push to hand vec, fire `OnLoseSecurity` + `OnOpponentSecurityRemoved` events) must be implemented as a new method on `EffectContext` before a DSL verb can lower to it.
- Lowers to engine API (proposed): `EffectContext::add_top_security_to_hand(player: PlayerId) -> bool` — pops `security.last()`, pushes to `hand`, fires `EffectTiming::OnLoseSecurity` via `SecurityRevealed` and `EffectTiming::OnOpponentSecurityRemoved` via `PlayerBattleArea(controller)`.
- Suggested DSL syntax:
  ```yaml
  - add_top_security_to_hand: { of: opponent }
  ```
- Gap kind: hybrid (DSL lacks the verb AND engine lacks the `EffectContext` method; only `trash_top_security` exists).
- Workaround: `raw_rust: { fn: p_137_opp_adds_top_security_to_hand }` registered in `src/cards/raw_rust/mod.rs`. When closed: replace the `raw_rust:` step with the native DSL verb.
- First reported: 2026-04-27 (P-137 batch-implement-cards-rust-dsl, Medusamon Batch 8)

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

## ST22-08 — Link Registration Clause (Plug-In / Link Card Mechanic)  [G-DSL-LINK-VERB]
- Status: closed 2026-05-02. DSL now supports `kind: link_requirement` lowering to `Effect::link(cost, filter)` and `link_to_own_digimon` process steps that surface the existing Link host-selection `PendingSelection` without adding action IDs.
- Effect text: "Inherited: Link Requirements [Link] Lv.3 or higher: Cost 2 (Plug this card from the hand or battle area sideways into the specified Digimon in the battle area.)"
- Also: "[Main] You may link this card to 1 of your Digimon without paying the cost."
- Missing DSL verb / step kind / predicate: Two missing DSL constructs:
  (a) A declarative clause kind for declaring link requirements — no `kind: link_requirement` or equivalent in `TypedDeclarativeBody`. The closest existing cards (EX11-027 Maquinamon) use `kind: raw_rust fn: ex11_027_link_requirements triggers: [main_from_hand, main_on_field]`.
  (b) An optional link-action step within a `process:` body — no `link_to_digimon:` or similar step verb. DCGO's `SelectPermanentEffect` with `Mode.Custom` + `card.CanLinkToTargetPermanent(permanent, false)` + `canNoSelect: true` drives this. The engine has `OptionSubtype::Link`, `Effect::link(cost, filter)`, and `attach_linked_card()`, but these are reachable only via hand-written `CardEffect` or raw_rust functions.
- Lowers to engine API: `Effect::link(cost, filter_fn)` on the declarative side; `ctx.game.attach_linked_card(host_handle)` on the step side. Both exist in the engine; neither is accessible from the DSL step vocabulary.
- Suggested DSL syntax:
  ```yaml
  # Declaration form (inherited link requirements):
  - kind: link_requirement
    scope: inherited
    cost: 2
    filter: { level_gte: 3 }
  
  # Step form (optional free link in process body):
  - link_to_own_digimon:
      optional: true
      cost_delta: -99   # or free: true
      filter: { kind: digimon }
      bind_as: linked_host
  ```
- Gap kind: dsl (engine has the primitive; DSL lacks both the clause kind and the step verb).
- First reported: 2026-04-27 (ST22-08 batch-implement-cards-rust-dsl, Medusamon Batch 11)

---

## ST22-08 — Linked-Card Effect Scope  [G-DSL-LINKED-SCOPE]
- Status: closed 2026-05-02. DSL now accepts `scope: linked`; triggered lowering marks the emitted effect with `.linked()` so the existing linked-card dispatch path can fire it from the host.
- Effect text: DCGO shows EndOfTurnLinkedEffect with `activateClass.SetIsLinkedEffect(true)` — an effect that fires only when the card is linked to a Digimon in the battle area, and the linked Digimon may attack at the controller's end of turn.
- Missing DSL verb / step kind / predicate: `scope: linked` — a clause scope for effects that fire as if they were part of the Digimon the card is linked to. `CompiledScope` in `digimon-dsl/src/compiled.rs` has `FaceUp` and `Inherited` variants; there is no `Linked` variant. The effect-queue (`effect_queue.rs`) already handles linked cards in `enqueue_from_permanent` (the Phase 8 Task 4 linked_cards branch), but the scope is expressed as a raw `linked_cards` list on `Permanent`, not as a DSL-compiled clause with `scope: linked`.
- Lowers to engine API: the engine already fires effects for linked cards via the `linked_cards` loop in `enqueue_from_permanent`. The DSL lowering layer would need to detect `scope: linked` on a clause and install the resulting `Effect` via `Effect::declarative(card)` with a flag indicating it should be enqueued from the linked-card path rather than the top-card path.
- Suggested DSL syntax:
  ```yaml
  - scope: linked
    when: end_of_your_turn
    optional: true
    once_per_turn: true
    process:
      - raw_rust: { fn: st22_08_linked_eot_may_attack }
  ```
- Gap kind: dsl (engine fires linked-card effects; DSL has no `scope: linked` clause kind that lowers into the linked-card effect list).
- First reported: 2026-04-27 (ST22-08 batch-implement-cards-rust-dsl, Medusamon Batch 11)

---

## ST22-08 — Named-Binding DP Reference in Formula  [G-BINDING-DP-FORMULA]
- Effect text: "[Main] … delete 1 of your opponent's Digimon with as much or less DP as 1 of your Digimon."
- Missing DSL verb / step kind / predicate: `binding_dp` — a formula primitive that reads the effective DP of a named binding (a `PermanentHandle` stored by `bind_as:` from a prior `select_own_permanent`). The formula system (`formula.rs` + `formula_eval.rs`) can read `source_permanent`'s DP via `{ of: source_permanent, value: dp }` (see DSL spec §3.10), but there is no form to read an arbitrary named binding's DP — which is required for "DP ≤ chosen own Digimon's DP" where the comparator is player-selected mid-effect.
- Lowers to engine API: `ctx.game.effective_dp(handle)` — already exists. The gap is that `CompiledFormula` has no `BindingDp(String)` variant that reads `bindings.get_permanent(name)` and calls `effective_dp`. 
- Suggested DSL syntax:
  ```yaml
  # In dp_lte formula, reference a named binding:
  dp_lte:
    formula:
      binding_dp: ally   # resolves bindings["ally"] as PermanentHandle, calls effective_dp
  ```
  Requires: (1) add `BindingDp(String)` to `FormulaSpec` and `CompiledFormula`; (2) add evaluation branch in `formula_eval.rs` that resolves the binding from `Bindings` and calls `ctx.game.effective_dp(h)`; (3) pass `Bindings` into the formula evaluator call chain.
- Gap kind: dsl (engine has `effective_dp`; DSL formula system has no binding-reference form).
- Workaround: None for `binding_dp` itself. Static and existing formula-backed `dp_lte` / `dp_gte` permanent predicates are now evaluated as of 2026-05-01, but this gap remains open until formulas can read a named binding's effective DP.
- First reported: 2026-04-27 (ST22-08 batch-implement-cards-rust-dsl, Medusamon Batch 11)

---

## ST22-08 — Return Played Option Card to Hand Post-Security  [G-ADD-OPTION-SELF-TO-HAND]
- Effect text: "[Security] Delete 1 of your opponent's Digimon with the lowest DP. Then, add this card to the hand."
- Status: Resolved for the narrow pending-security disposition slice on 2026-05-01. DSL now supports a deterministic `add_this_option_to_hand: {}` step that lowers to `EffectContext::add_pending_security_to_hand()`, consuming `Game.pending_security` and moving the revealed card to the defender/controller hand instead of letting security dispose trash it.
- DSL syntax:
  ```yaml
  - add_this_option_to_hand: {}
  ```
- Coverage: `debug_runner_dsl::security_dsl_adds_currently_resolving_option_to_hand` and `lm_027_security_adds_card_to_hand_after_play`.
- Remaining related gaps: cards may still be blocked by other predicates or Option mechanics, such as lowest-DP selection, Plug-In/Link, Delay timing, or play-cost filters.
- First reported: 2026-04-27 (ST22-08 batch-implement-cards-rust-dsl, Medusamon Batch 11)

---

## BT21-072 — [All Turns] +1000 DP per digivolution card (dynamic formula aura)  [G-AURA-DP-FORMULA]

- Effect text: "[All Turns] This Digimon gets +1000 DP for each of its digivolution cards."
- Missing DSL verb / step kind / predicate: `dp_modifier_fn` / `dp_modifier_formula` — a formula-based variant of `AuraBody.dp_modifier`. The DCGO implements this via `ChangeSelfDPStaticEffect(changeValue: 1000 * count(), ...)` at `EffectTiming.None`, where `count()` is a live lambda returning `PermanentOfThisCard().DigivolutionCards.Count()` (= material_count = stack_size - 1). This is a **continuously-recomputed** aura that updates dynamically each tick, including after `de_digivolve` operations that pop digivolution cards from the stack. The DSL `kind: aura` with self-target accepts only `dp_modifier: Option<i32>` — a static literal with no formula variant. The `FormulaSpec` type (with `per: material_count, delta: 1000`) exists for step-level `add_dp_modifier` verbs, but `add_dp_modifier` only snapshots the formula's value at event-fire time, not continuously. Storing a snapshot in `Effect.dp_modifier` cannot model the dynamic behaviour required.
- Lowers to engine API: `source_dp_contribution(perm_handle, source_index)` reads `Effect.dp_modifier` continuously — the engine query mechanism already supports live reads. The gap is that `AuraBody` has no formula field to store a `FormulaSpec` that `lower_aura.rs` could evaluate at read-time rather than compile-time.
- Suggested DSL syntax:
  ```yaml
  - kind: aura
    active_when: { all_turns: true }   # or omit for always-on
    target: {}                          # self
    dp_modifier_fn:                     # NEW: formula-based dynamic variant
      base: 0
      per: material_count               # CompiledPerSelector::MaterialCount = stack_size - 1
      delta: 1000
  ```
  Implementation notes: (1) add `dp_modifier_fn: Option<FormulaSpec>` to `AuraBody` in `digimon-dsl/src/clause.rs`; (2) compile to `CompiledAuraBody.dp_modifier_fn: Option<CompiledFormula>` in `digimon-dsl/src/compiler/clause.rs`; (3) in `lower_aura.rs`, when `dp_modifier_fn` is set, store the `CompiledFormula` in a new `Effect.dp_modifier_formula` field instead of `Effect.dp_modifier`; (4) have `source_dp_contribution` evaluate the formula against the current stack size when `dp_modifier_formula` is present.
- Gap kind: dsl (engine `source_dp_contribution` already reads continuously; `AuraBody` just lacks the formula field, and `Effect` lacks a formula storage slot for dynamic evaluation).
- Workaround: Clause omitted from YAML. Test `#[ignore = "pending: G-AURA-DP-FORMULA — AuraBody.dp_modifier does not accept a formula"]`.
- First reported: 2026-04-27 (BT21-072 batch-implement-cards-rust-dsl, Medusamon Batch 11)

---

## BT20-102 — Omnimon (X Antibody) self-digivolution-stack name check  [G-SELF-DIGIVOLUTION-CONTAINS-NAME]

- Effect text: "[On Play][When Digivolving] If [Omnimon] or [X Antibody] is in this Digimon's digivolution cards, ..."
- Missing DSL verb / step kind / predicate: `self_digivolution_contains_name` — a `BoolPredicate` leaf that evaluates `rctx.source_permanent()?.contains_card_name(name, &rctx.game.card_data)` from within a triggered clause's `condition:` block. The DSL predicate `source_name_contains` applies to the SOURCE PERMANENT (the Digimon this card is stacked under, in inherited contexts) — not to this card's own digivolution stack at runtime. Additionally, `lower_triggered.rs` passes `PredicateSubject::None` to condition closures, so any subject-requiring predicate silently passes.
- Engine gap component (hybrid): `Permanent::contains_card_name(name, data)` exists in `code/digimon-engine/src/permanent.rs` and scans the full stack. The gap is that `lower_triggered.rs` does not pass a `PredicateSubject::Permanent(source_h)` to the condition closure (currently passes `PredicateSubject::None`).
- Lowers to engine API: `Permanent::contains_card_name(name, &game.card_data)` on `rctx.source_permanent()`.
- Suggested DSL syntax:
  ```yaml
  condition:
    self_digivolution_contains_name: "Omnimon"
    # or: any_of: [{ self_digivolution_contains_name: "Omnimon" }, { self_digivolution_contains_name: "X Antibody" }]
  ```
  Implementation: add `self_digivolution_contains_name: Option<String>` to `BoolPredicateSpec` in `digimon-dsl/src/predicate.rs`, compile to `CompiledPredicate` field, evaluate in `eval_predicate(p, rctx, PredicateSubject::Permanent(source_h))` where `source_h` is the triggering permanent's handle — requires threading the source handle into the triggered-clause condition closure in `lower_triggered.rs`.
- Gap kind: hybrid (engine has the method; DSL needs predicate leaf + `lower_triggered.rs` subject threading).
- Workaround: entire boardwipe clause routed through `raw_rust: { fn: bt20_102_boardwipe_and_return }` which calls `perm.contains_card_name(...)` directly. Over-wide: top card name "Omnimon (X Antibody)" contains "X Antibody" so condition is always true for BT20-102 even with no digivolution source.
- First reported: 2026-04-27 (BT20-102 batch-implement-cards-rust-dsl, Medusamon Batch 11)

---

## BT20-102 — Exclude-from-binding filter in `for_each`  [G-FOR-EACH-EXCLUDE-BINDING]

- Status: CLOSED on 2026-05-01. `not_in_binding` is now parsed, compiled, evaluated against `Permanent` / `PermanentList` bindings, and `for_each` threads current bindings into its permanent scan. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- group7_predicate_batch parse_group7_predicate_leaves`.
- Effect text: "[On Play][When Digivolving] ... choose 1 of both players' Digimon and delete all other Digimon."
- Missing DSL verb / step kind / predicate: `not_in_binding` — a `CandidatePredicate` leaf in `for_each { over, filter, body }` that excludes permanents whose handle appears in a named binding (a prior selection). Without it, "delete all OTHER Digimon" (all except the two saved by selection) cannot be expressed purely in DSL.
- Engine API: the engine can iterate `battle_area` handles and compare against a collected `Vec<PermanentHandle>`. No new API needed — gap is purely in the DSL filter vocabulary.
- Suggested DSL syntax:
  ```yaml
  - for_each:
      over: { owner: any, kind: digimon, not_in_binding: saved }
      bind_as: candidate
      body:
        - delete_permanent: { target: candidate }
  ```
  Implementation: add `not_in_binding: Option<String>` to `CandidatePredicateSpec` in `digimon-dsl/src/predicate.rs`, compile, and evaluate by looking up the named binding in `Bindings` and comparing handle equality.
- Gap kind: dsl (engine can express this in a raw_rust loop; DSL has no filter for handle-set exclusion).
- Workaround: none needed for handle-set exclusion after 2026-05-01. Card YAML/tests may still need separate migration away from any raw_rust bridge if other blockers remain.
- First reported: 2026-04-27 (BT20-102 batch-implement-cards-rust-dsl, Medusamon Batch 11)

---

## P-035 / P-103 / BT24-089 — Option-as-permanent placement (inherited security)  [G-PLACE-SELF-AS-OPTION-PERMANENT]

- Effect text (P-035): "[Main] … Then, place this card in your battle area." and "[Security] Place this card in the battle area." (inherited)
- Status: IMPLEMENTED for inherited-security source Options as of 2026-05-02. The DSL verb / step kind is `place_self_as_delay_option: {}` — a step that places the currently-resolving Option card into the battle area as an `OptionState::Delayed` permanent from within an inherited security context. Two contexts exist:
  1. **Main clause** ("Then, place this card in your battle area."): DCGO calls `PlaceDelayOptionCards(card, activateClass)`. In the Rust engine, `dispose_option` + `classify_option_subtype` detect the `kind: delay` clause and auto-place the card at the `MainEffectDrain` phase — no explicit DSL step is needed. The engine handles placement implicitly.
  2. **Inherited security clause** ("[Security] Place this card in the battle area."): DCGO calls `CardEffectFactory.PlaceSelfDelayOptionSecurityEffect(card)`. Rust now exposes `EffectContext::place_self_as_delay_option_permanent`, which removes the matching non-top source Option from its host stack, places it in the owner's battle area as `OptionState::Delayed`, and dispatches `OnOptionPlaced`.
- Lowers to engine API: `EffectContext::place_self_as_delay_option_permanent(&mut self)`. In the inherited-security context, this method: (1) identifies the source Option card (the digivolution source that triggered this effect), (2) removes it from its current location (digivolution stack), and (3) places it in `self.game.players[owner].battle_area` as a `Permanent` with `OptionState::Delayed`.
- Suggested DSL syntax:
  ```yaml
  - place_self_as_delay_option: {}
  ```
  Used in the `process:` of the inherited security clause. Not needed in the Main clause (engine auto-placement via `dispose_option` suffices).
- Gap kind: resolved dsl vocabulary/engine API gap for inherited security context. Engine already handled the Main clause auto-placement; inherited-security-context placement now uses the explicit DSL step because `dispose_option` is not called in that path.
- Verification: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow -- inherited_security_places_source_option_as_delay_permanent`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- place_self_as_delay_option`.
- Workaround: no longer needed for new YAML/tests. Existing `process: []` placeholders for P-035, P-103, and BT24-089 can migrate to `place_self_as_delay_option: {}` when those card YAMLs/tests are revisited.
- First reported: 2026-04-28 (P-035 Red Memory Boost! batch-implement-cards-rust-dsl, Medusamon Batch 12). Same gap pre-existed in P-103.yaml and BT24-089.yaml without a tracker entry.

---

## P-206 — Board-color cross-reference predicate in Delay clause  [G-COLOR-MATCH-AGAINST-BOARD]

- Effect text: "[Main] ＜Delay＞ … You may play 1 Tamer card with the same color as any of your Digimon on the field from your hand with the play cost reduced by 4."
- Missing DSL verb / step kind / predicate: `color_matches_any_field_digimon` (or an equivalent dynamic board-color filter) — a `PredicateSpec` leaf that checks whether a candidate card's colors share at least one color with ANY Digimon currently in the controller's battle area. The existing `color_is` predicate accepts a fixed literal color token (e.g. `red`, `blue`, `white`) — it cannot perform a dynamic cross-reference against the set of colors currently present on field Digimon top cards. No `any_of_field_colors` or `matches_board_color` predicate variant exists in `PredicateSpec` in `digimon-dsl/src/predicate.rs`.
- Root cause: the filter predicate must inspect runtime game state (the controller's battle area, specifically the colors of permanent top cards) during `select_hand` candidate evaluation — a dynamic query, not a static literal comparison. `eval_card_fields` in `code/digimon-engine/src/dsl_cards/predicate.rs` has no branch for this pattern.
- Lowers to engine API: `CardData::card_colors` (already on `CardData`) read against the controller's `battle_area.iter().map(|p| card_data[p.top_card().data_index].card_colors)` collected set. No new engine method is needed — the gap is purely in the DSL predicate vocabulary.
- Suggested DSL syntax:
  ```yaml
  filter:
    all_of:
      - kind: tamer
      - color_matches_any_field_digimon: { of: you }   # NEW predicate leaf
  ```
  Implementation notes: (1) add `color_matches_any_field_digimon: Option<PlayerRef>` to `BoolPredicateSpec` in `digimon-dsl/src/predicate.rs`; (2) compile to `CompiledPredicate.color_matches_any_field_digimon`; (3) in `eval_card_fields`, collect the union of colors from the relevant player's battle-area top cards, then check if the candidate card's colors overlap. Requires threading `rctx.game` (already available in `eval_card_fields` via `EffectReadContext`) rather than a static literal.
- Gap kind: dsl (engine stores `card_colors` on `CardData` and has full battle-area access; the DSL/lowering path just lacks the predicate leaf for a dynamic color-set intersection).
- Workaround: `filter: { kind: tamer }` only — the "same color as any of your Digimon on the field" constraint is not enforced at selection time. Tests asserting color-mismatch rejection are `#[ignore = "pending: G-COLOR-MATCH-AGAINST-BOARD"]`. First card affected: P-206 Digital Gate Open Delay clause.
- First reported: 2026-04-28 (P-206 Digital Gate Open batch-implement-cards-rust-dsl, Medusamon Batch 14)

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

## BT8-097 / Royal Knights — formula filters for counted battle-area cards  [G-FORMULA-KIND-FILTER]

- Effect text: `BT8-097` Crimson Blaze: "Reduce the memory cost of this card in your hand by 1 for each Digimon your opponent has in play."
- Missing DSL verb / step kind / predicate: `card_count_in_zone` formulas can count a player's `battle_area`, but cannot apply a `kind: digimon` filter. The authored YAML therefore counts all opponent battle-area permanents, including Tamers and Option permanents, when computing the cost reduction.
- Lowers to engine API: the engine can inspect each battle-area permanent and test `Permanent::is_digimon(&card_data)`; the formula DSL needs a filtered-count form that passes a compiled predicate into formula evaluation.
- Suggested DSL syntax:
  ```yaml
  amount_fn:
    base: 0
    per:
      card_count_in_zone:
        of: opponent
        zone: battle_area
        filter: { kind: digimon }
    delta: 1
  ```
- Gap kind: dsl (engine has the data; formula vocabulary lacks the filter).
- Workaround: current YAML over-reduces when the opponent controls non-Digimon permanents.
- First reported: 2026-04-28 (Royal Knights archetype assessment; surfaced by BT8-097 in Royal Knights lists)
