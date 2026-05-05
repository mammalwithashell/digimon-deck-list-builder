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

## Royal Knights — filtered breeding permanent target  [RK-G001]
- Effect text: BT13-093: "[On Deletion] Place 1 Digimon card with the [Royal Knight] trait from your hand under a [King Drasil_7D6] in the breeding area as its bottom digivolution card." BT20-083: "[On Deletion] You may place this card as the bottom digivolution card of your [King Drasil_7D6] in the breeding area."
- Missing DSL verb / step kind / predicate: `select_own_breeding_permanent` has no `filter` field, so YAML cannot require that the selected breeding permanent is actually `[King Drasil_7D6]`.
- Lowers to engine API: existing breeding pending-selection and `place_as_bottom_source` flow once the selected breeding permanent can be filtered by top-card name/card id.
- Suggested DSL syntax:
  ```yaml
  - select_own_breeding_permanent:
      bind_as: kd
      filter: { name_is: "King Drasil_7D6" }
      prompt: "Choose your [King Drasil_7D6]"
      then:
        - place_as_bottom_source: { source: ..., target: kd }
  ```
- First reported: 2026-05-05 Royal Knights batch 1 implementation pass.

## Royal Knights — source-bound return-self cost into reduced-cost hand play  [RK-G002]
- Effect text: EX11-071: "[Main] By returning this Tamer to the bottom of the deck, you may play 1 play cost 4 or higher [Royal Knight] or [LIBERATOR] trait card from your hand with the play cost reduced by 2."
- Missing DSL verb / step kind / predicate: a Main-phase activation that pays a source-bound `return_to_deck { target: source, position: bottom }` cost and then opens a player-visible hand play selection whose actual payment is reduced by 2.
- Lowers to engine API: existing source permanent binding, hand selection, and pay-cost flow need a reusable action/pending-selection wrapper so the return cost and reduced play payment stay one legal choice.
- Suggested DSL syntax:
  ```yaml
  - when: main
    optional: true
    pay_cost:
      - return_to_deck: { target: source, position: bottom }
    process:
      - select_hand:
          bind_as: played
          filter:
            all_of:
              - play_cost_gte: 4
              - any_of:
                  - trait_has: "Royal Knight"
                  - trait_has: LIBERATOR
          prompt: "Play a cost 4+ Royal Knight/LIBERATOR"
      - play_from_hand:
          target: played
          cost: { reduce: 2 }
  ```
- First reported: 2026-05-05 Royal Knights batch 1 implementation pass.

## Royal Knights full pool pass — residual reusable DSL/engine gaps  [RK-G005]
- Status: PARTIAL pool pass completed on 2026-05-05. The Royal Knights resolver pool has 72 unique cards and now has 72 Rust DSL YAML entries. Fully unsupported clauses were left as explicit YAML comments plus ignored Rust tests instead of hidden approximations.
- Newly routed or reaffirmed blocked cards/clauses: `BT13-019`, `BT13-030`, `BT13-075`, `BT13-087`, `BT13-102`, `BT13-111`, `BT13-112`, `BT15-092`, `BT17-077`, `BT19-093`, `BT20-017`, `BT20-021`, `BT20-045`, `BT20-056`, `BT22-025`, `BT22-041`, `BT22-052`, `BT23-013`, `BT23-035`, `BT23-047`, `BT23-057`, `BT23-072`, `EX8-073`, `EX10-068`, and `EX11-053`.
- Missing DSL/engine areas: union selection across hand/trash/breeding/source zones; play from King Drasil or other source stacks with uniqueness/name-exclusion filters; hand-main source placement; opponent hidden-hand choices; result-dependent fallback branches; combined trash/security/color/source-count formulas; token registration for Atho/Rene/Por and Hinukamuy; Blast Digivolve and Blast DNA action paths; Option battle-area carrier lifecycle for non-Delay options; security-trash self-dispatch; security search/play and self-to-security placement; global security-removed observers; immediate may-attack/action prompts; Partition; and replacement/security-trash costs tied atomically to prevention.
- Workaround policy: no approximations were used for these blockers. If a printed clause required one of the missing primitives, the YAML either implemented an independent faithful slice such as a keyword/security play/simple trigger, or used a load-only gap stub.
- Verification: targeted `cargo test --manifest-path code\digimon-engine\Cargo.toml --test cards_behavioral -- <card_filter> --nocapture` passed for the final 25 filters, with one active load test and one ignored gap test per card.
- First reported: 2026-05-05 Royal Knights full pool implementation pass.

## Rocks pool pass residual DSL/engine gaps
- Status: PARTIAL pool pass completed on 2026-05-04. After pulling main, production YAML/test slices now exist for 40 of 47 Rocks pool cards; the remaining 7 were explicitly routed as blocked rather than no-op authored.
- Remaining blocked cards: `BT20-055`, `BT21-021`, `BT9-103`, `EX10-003`, `EX11-065`, `EX8-070`, `P-130`. Main now carries newer `P-123` coverage, so the stale Rocks blocked record was not retained.
- Missing DSL/engine areas: face-up security lifecycle and security end-of-opponent-turn timing; conditional inherited keyword grants based on host traits; Save/Xros routing; attack-cancel effects; hand-or-source costs with source trait filtering; source placement from hand/trash; lowest-play-cost delete; effect move-from-breeding and same-side/costed `[When Moving]` follow-up shapes beyond the resolved base OnMove timing.
- First reported: 2026-05-04 Rocks pool implementation pass.

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
- Finding: as of the 2026-05-04 Rocks batch plus the pulled main updates, 40 of 47 Rocks pool cards have Rust YAML under `code/digimon-engine/cards/`. New Rocks pass coverage added or audited the `EX8`/`EX10`/`EX11`/`P-167` shell; the remaining missing cards are tracked in the residual gap entry above.
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
- Also blocks: P-117 clause 0 — "[Your Turn][OPT] When this Digimon would digivolve into a card with the [Free] trait, if you have a Tamer, reduce the digivolution cost by 1." Same structural gap: need `target_trait_has: Free` in a `when_this_digivolves_into` trigger form. Cross-listed 2026-05-04.

---

## P-117 — inherited When Attacking color-count predicate  [G-DSL-SELF-COLOR-COUNT-GTE]
- Effect text: "[When Attacking] If this Digimon has 2 or more colors, ＜Draw 1＞ (Draw 1 card from your deck.)"
- Missing DSL verb / step kind / predicate: `self_color_count_gte: N` boolean predicate (or equivalent) evaluating whether the carrier permanent's top card has N or more distinct colors. No such predicate exists in the DSL's `PredicateSpec` / `CompiledPredicate` hierarchy.
- DCGO reference: `P_117.cs` lines 203-211 — `card.PermanentOfThisCard().TopCard.CardColors.Count >= 2`. Note: DCGO checks ONLY the top card's colors, not the union of the full digivolution stack. The DSL predicate should align with DCGO behavior: count the top card's colors only.
- Lowers to engine API: `Game::player(p).battle_area[i].top_card()` → `card_data[idx].colors.len()` comparison; no new engine primitive needed, only a DSL predicate leaf that invokes `ctx.source_permanent` top-card color count.
- Suggested DSL syntax:
  ```yaml
  condition:
    self_color_count_gte: 2
  ```
  Evaluates as: `ctx.source_permanent.and_then(|h| perm.top_card().colors().len()).unwrap_or(0) >= 2`.
  Alternative: `source_top_card_color_count_gte: 2` if the naming convention favors explicit subject.
- Workaround: omit condition (over-fires — Draw fires unconditionally on all carriers including mono-color). Negative-condition tests are `#[ignore = "pending: G-DSL-SELF-COLOR-COUNT-GTE from qa/dsl-vocab-gaps.md"]`.
- Gap kind: DSL only (engine has the data; only the predicate leaf is missing).
- Cards blocked: P-117 clause 1 (inherited When Attacking); BT12-031 clause 1b ([All Turns] SecurityAttackPlus+Blocker conditional on 2+ colors in digi-cards — same predicate needed, but evaluated against the FULL digivolution stack's union of colors, not just the top card).
- First reported: 2026-05-04 (P-117 batch-implement-cards-rust-dsl)

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

## BT8-097 / Royal Knights — formula filters for counted battle-area cards  [G-FORMULA-KIND-FILTER]

- Status: RESOLVED for reusable formula-zone count filters on 2026-05-02. `card_count_in_zone` payloads now accept `filter: { ... }`; the compiler carries the predicate into filtered count IR, and runtime evaluation counts only representable subjects that satisfy the predicate instead of falling back to an unfiltered count.
- Effect text: `BT8-097` Crimson Blaze: "Reduce the memory cost of this card in your hand by 1 for each Digimon your opponent has in play."
- Implemented DSL form: `card_count_in_zone` formulas can now apply a `kind: digimon` filter. `BT8-097.yaml` uses this filtered form so Tamers and Option permanents no longer reduce Crimson Blaze's play cost.
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
- Gap kind: resolved dsl vocabulary/evaluator gap for filtered zone-count formulas.
- Workaround: no longer needed for BT8-097 or other `card_count_in_zone` formulas with simple predicate filters.
- First reported: 2026-04-28 (Royal Knights archetype assessment; surfaced by BT8-097 in Royal Knights lists)
- Verification: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- group7_formula_batch phase3d_formula_zone_count`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt8_097`.

---

## AD1-012 — `on_opponent_attack` Timing variant on triggered clauses  [G-DSL-ON-OPPONENT-ATTACK]
- Effect text: AD1-012 CresGarurumon: "[Opponent's Turn][Once Per Turn] When one of your opponent's Digimon attacks, 2 of your Digimon may DNA digivolve into [Omnimon Alter-S] in the hand. Then, you may change the attack target to 1 of your Digimon."
- Missing DSL verb / step kind / predicate: `Timing::OnOpponentAttack` variant on `digimon_dsl::clause::Timing` (`code/digimon-dsl/src/clause.rs:83-125`); no mapping in `compile_timing` (`code/digimon-dsl/src/compile.rs:173-216`).
- Lowers to engine API: `Effect::on_opponent_attack` (`code/digimon-engine/src/effect.rs:427`) — engine timing dispatch already handles `EffectTiming::OnOpponentAttack` (`lower_triggered.rs:181`) and the combat state machine fires it (`combat.rs:2237-2242`). The hybrid declared-attack-observer engine slice closed 2026-04-29 unblocks the engine half; DSL just lacks the timing token.
- Suggested DSL syntax:
  ```yaml
  - when: on_opponent_attack
    active_when: { opponents_turn: true }
    once_per_turn: true
    optional: true
    process: [...]
  ```
- Implementation: add `Timing::OnOpponentAttack` variant + serde wiring + `compile_timing` arm; the existing `lower_triggered.rs` already routes `EffectTiming::OnOpponentAttack`, so no new lowering code needed.
- Gap kind: dsl. Engine has the primitive.
- Workaround: None faithful. AD1-012's Opp-Turn clause is documented as commented stub in YAML; behavioral test `#[ignore = "pending: G-DSL-ON-OPPONENT-ATTACK"]`.
- First reported: 2026-05-03 (AD1-012 batch-implement-cards-rust-dsl, DNA Omnimon Batch 1)

---

## AD1-012 — `redirect_attack_target` step verb  [G-DSL-REDIRECT-ATTACK-TARGET]
- Effect text: AD1-012 CresGarurumon (sub-step of the Opp-Turn clause): "Then, you may change the attack target to 1 of your Digimon."
- Missing DSL verb / step kind / predicate: No `redirect_attack_target` entry in the `StepSpec` enum / serde tag table at `code/digimon-dsl/src/step.rs`. No `CompiledStep::RedirectAttackTarget` variant.
- Lowers to engine API: `EffectContext::redirect_attack(new_target_perm)` (`code/digimon-engine/src/effect_context/mod.rs:3099`) — exists and is used by hand-written cards (BT22-061, EX11-042, P-094 in legacy Python).
- Suggested DSL syntax:
  ```yaml
  - select_own_permanent:
      bind_as: redirect_target
      optional: true
      filter: { kind: digimon }
      prompt: "Change attack target to 1 of your Digimon"
  - redirect_attack_target: { new_target: redirect_target }
  ```
- Implementation: add `StepSpec::RedirectAttackTarget { new_target: BindingRef }` + serde + `CompiledStep` variant + lowering arm in `dsl_cards/step/combat.rs` that resolves the binding to a `PermanentHandle` and calls `ctx.redirect_attack(perm_handle)`.
- Gap kind: dsl. Engine has the primitive.
- Workaround: None faithful. AD1-012 Opp-Turn redirect substep is BLOCKED behind the timing gap above as well.
- First reported: 2026-05-03 (AD1-012 batch-implement-cards-rust-dsl, DNA Omnimon Batch 1)

---

## BT15-101 — Self-target predicate for event triggers (`event_target_is_source`)  [G-DSL-EVENT-TARGET-IS-SELF]
- Effect text: BT15-101 MetalGarurumon: "[All Turns] [Once Per Turn] When this Digimon becomes suspended, you may unsuspend it."
- Missing DSL verb / step kind / predicate: No `event_target_is_source` (or equivalent `event_target_is_self`) BoolPredicate leaf that evaluates whether the suspended/affected permanent equals the source permanent. The existing event predicates (`event_target_owner`, `event_target_kind`, `event_target_trait_has`) only inspect the target's owner/kind/traits. The DSL `equals: [...]` predicate compares only integers (literals + integer bindings via `Bindings::get_literal`) — it cannot compare permanent handles.
- Lowers to engine API: `event_target_card(rctx)` already returns the `CardHandle` of the suspended permanent's top card; `rctx.source_permanent` carries the source permanent handle. A new predicate could compare `current_trigger_context.event_permanent` against `rctx.source_permanent_handle()`.
- Suggested DSL syntax: add `event_target_is_source: bool` BoolPredicate leaf evaluating `rctx.game.current_trigger_context?.event_permanent == Some(rctx.source_permanent_handle()?)`.
  ```yaml
  - when: on_suspend
    active_when: { all_turns: true }
    once_per_turn: true
    optional: true
    condition: { event_target_is_source: true }
    process:
      - unsuspend: { target: source }
  ```
- Implementation: add `event_target_is_source: Option<bool>` to `PredicateSpec`, compile to a new `CompiledPredicate` field, evaluate inside `eval_event_fields` in `dsl_cards/predicate.rs`.
- Gap kind: dsl. Engine has the comparison primitive (handles are equality-comparable).
- Workaround: AD1-014 pattern (`event_target_owner: you, event_target_kind: digimon`) — over-fires when ANY of the controller's Digimon (allies) suspend, so OPT may be consumed at the wrong moment and a "may unsuspend" prompt may appear when the source is not actually suspended. Faithful for "any of your Digimon"-style triggers (AD1-014, BT13-012); approximation-only for "this Digimon" triggers (BT15-101).
- First reported: 2026-05-03 (BT15-101 batch-implement-cards-rust-dsl)

## BT21-102 — `on_ally_attack` / `on_opponent_attack` timings missing from DSL
- Effect text: BT21-102 Tai Kamiya — "[Your Turn] When one of your Digimon attacks, by suspending this Tamer, ＜Draw 1＞."
- Missing DSL verb / step kind / predicate: `digimon_dsl::clause::Timing` enum (`code/digimon-dsl/src/clause.rs`) does not include `OnAllyAttack` or `OnOpponentAttack`. Both timings exist as `EffectTiming` variants in the engine and are dispatched correctly by `combat.rs` §Phase 9 Task 8, and `lower_triggered.rs` line 180 even maps `EffectTiming::OnAllyAttack`, but no `CompiledTiming::OnAllyAttack` exists so the mapping is unreachable from YAML.
- Lowers to engine API: `Effect::on_ally_attack(card)` / `Effect::on_opponent_attack(card)` already exist (`code/digimon-engine/src/effect.rs` line 421+).
- Suggested DSL syntax:
  ```yaml
  - when: on_ally_attack
    optional: true
    active_when: { your_turn: true }
    process:
      - suspend: { target: source }
      - draw: { of: you, count: 1 }
  ```
- Implementation: add `OnAllyAttack` and `OnOpponentAttack` to both `digimon_dsl::clause::Timing` (with `#[serde(rename = "on_ally_attack")]` / `on_opponent_attack`) and `digimon_dsl::compiled::CompiledTiming`, plus the `compile.rs` and `timing_map.rs` entries. The lowering already exists.
- Gap kind: dsl. Engine and dispatch are complete; only the DSL surface is missing.
- Workaround used in BT21-102: `when: when_attacking`. Faithful only when the source is a Tamer (since Tamers never declare attacks, the source firing the clause is never the attacker, mirroring OnAllyAttack semantics). For Digimon sources this workaround is over-permissive (would also fire on the Digimon's own attacks). Track BT21-102 as PARTIAL until the timing tokens land.
- First reported: 2026-05-03 (BT21-102 Tai Kamiya, batch-implement-cards-rust-dsl)

## BT21-102 — `play_cost_lte` formula-valued variant
- Effect text: BT21-102 Tai Kamiya — "[Main] [Once Per Turn] You may play 1 [ADVENTURE] or [Hero] trait card with a play cost of 2 or less from your hand without paying the cost. For each of your Tamers' colors, add 1 to this effect's play cost maximum."
- Missing DSL verb / step kind / predicate: `PredicateSpec::play_cost_lte` is `Option<i32>` (literal only — `code/digimon-dsl/src/predicate.rs` line 59). It cannot accept a formula expression so dynamic play-cost ceilings ("cost ≤ 2 + N") cannot be expressed in selection filters. Closely related to existing G-PLAY-COST-LTE entry but for the formula-valued variant rather than the literal predicate.
- Lowers to engine API: `card.play_cost <= rctx.eval_formula(formula)` — engine already has formula evaluation and per-card play_cost reads.
- Suggested DSL syntax:
  ```yaml
  filter:
    play_cost_lte:
      formula:
        base: 2
        per:
          distinct_colors_count:
            of: you
            zone: [battle_area]
            filter: { kind: tamer }
        delta: 0
  ```
- Implementation: change `PredicateSpec::play_cost_lte` to a sum type accepting either `i32` (literal) or `{ formula: FormulaSpec }`; thread compiled formula through `eval_card_fields`.
- Gap kind: dsl. Companion to G-DSL-DISTINCT-TAMER-COLORS-FORMULA — both must close together to faithfully implement BT21-102's [Main] OPT clause.
- First reported: 2026-05-03 (BT21-102 Tai Kamiya, batch-implement-cards-rust-dsl)

## EX9-066 — Binding-presence predicate (`binding_present`/`binding_absent`)  [G-DSL-BIND-PRESENT]
- Effect text: EX9-066 Tai Kamiya & Matt Ishida — "[On Play] You may return 1 Digimon card with [Greymon], [Garurumon] or [Omnimon] in its name from your trash to the hand. If this effect didn't return, ＜Draw 1＞." Also EX11-074 — "[When Digivolving] [When Attacking] You may suspend 1 Digimon. If this effect suspended your Digimon, ..."
- Status: OPEN (filed 2026-05-03 during EX9-066 batch-implement-cards-rust-dsl). Sibling of the EX11-074 gap noted at line 61 of this file (Zephagamon section), restated here as a standalone reusable primitive.
- Missing DSL verb / step kind / predicate: no `binding_present: <name>` or `binding_absent: <name>` BoolPredicate leaf that evaluates whether a prior `bind_as:` step (e.g. an optional `select_trash` / `select_hand` / `select_own_permanent` that the player may have declined) actually produced a value. The existing `equals: [<binding>, <literal>]` compare on `CompiledBindingCompare` only supports integer-valued bindings (literals + integer bindings via `Bindings::get_literal`) — it cannot distinguish a permanent/card binding that was set vs absent.
- Lowers to engine API: `Bindings::get_card(name).is_some()` / `Bindings::get_permanent(name).is_some()` / `Bindings::get_literal(name).is_some()` — engine already has these read paths through `digimon_dsl::compiled::Bindings` and `effect_context::Bindings`.
- Suggested DSL syntax:
  ```yaml
  - select_trash:
      bind_as: pick
      optional: true
      filter: { ... }
  - if:
      condition: { binding_present: pick }
      then: [ add_to_hand_from_trash: { card: pick } ]
      else: [ draw: 1 ]
  ```
- Implementation: add `binding_present: Option<String>` and `binding_absent: Option<String>` BoolPredicate leaves to `PredicateSpec`, compile to a `CompiledPredicate` field, evaluate inside `eval_predicate_with_bindings` in `dsl_cards/predicate.rs` by checking the named binding in the threaded `Bindings`.
- Gap kind: dsl. Engine has the comparison primitive (binding presence is a trivial Option check).
- Workaround used in EX9-066: drop the binding-result check entirely; present a binary `select_effect_choice [Return / Draw]` so the player explicitly picks the branch up front. The Return branch's inner `select_trash` is `optional: true` so it degrades gracefully when no eligible cards exist. Case C (no eligible card + player picked Return) becomes a no-op rather than a forced draw — diverges from DCGO but the action mask still surfaces the Decline → Draw alternative, so a faithful RL agent learns to pick Decline in case C. No auto-selection is performed on the agent's behalf; the no-approximations policy is preserved.
- First reported: 2026-05-03 (EX9-066 Tai Kamiya & Matt Ishida, batch-implement-cards-rust-dsl)

## BT24-008 / EX9-066 — General `count_gte` / `count_lte` predicate not evaluated  [G-COUNT-GTE-NOT-EVALUATED]
- Effect text: BT24-008 Lv4 Reptile/Dragonkin/LIBERATOR — "[On Play] By trashing 1 card with the [Reptile], [Dragonkin] or [LIBERATOR] trait from your hand, <Draw 2>." (condition gates on `count_gte` over hand). EX9-066 — needs gating on `count_gte` over trash zone for the trash-or-draw branch.
- Status: OPEN (filed 2026-05-03 during EX9-066 batch-implement-cards-rust-dsl). Previously documented inline in BT24-008.yaml header but not as a standalone gap entry.
- Missing engine evaluation: `PredicateSpec::count_gte: Option<CountAggregate>` and `count_lte: Option<CountAggregate>` parse correctly into `CompiledPredicate.count_gte` / `count_lte` (`compiled.rs` lines 223-224), but `dsl_cards/predicate.rs::eval_predicate_with_bindings` does NOT consult these fields — only the specialized `security_count_gte` / `security_count_lte` (predicate.rs lines 73-82) and `materials_count_gte` / `materials_count_lte` (predicate.rs lines 834-842) are wired. So `condition: { count_gte: { filter: ..., n: 1 } }` is a no-op that always evaluates as TRUE, which means `if count_gte ≥ 1 then [...] else [...]` always takes the `then` branch regardless of the actual card count.
- Lowers to engine API: needs a generic `count_matching_in_zone` walker that takes a `CompiledPredicate` filter (with `zone:` constraints) and counts matches across the named player's hand / trash / battle_area / security / deck. The existing `existential_any` walker (predicate.rs:279) only iterates `battle_area` and stops at first match — needs to be generalized to iterate the requested zones and count instead of short-circuit.
- Suggested DSL syntax (already accepted by the parser — only evaluation is missing):
  ```yaml
  condition:
    count_gte:
      filter:
        of: you
        zone: [trash]
        kind: digimon
        any_of:
          - name_contains: "Greymon"
          - name_contains: "Garurumon"
          - name_contains: "Omnimon"
      n: 1
  ```
- Implementation: add a `count_in_zones(filter: &CompiledPredicate, target: PlayerRef, rctx, bindings) -> u32` helper in `dsl_cards/predicate.rs` that iterates the player's hand / trash / battle_area / security / deck per the filter's `zone:` field and counts matches via per-card / per-permanent predicate evaluation. Then check `count >= agg.n` (gte) / `count <= agg.n` (lte) inside `eval_predicate_with_bindings`.
- Gap kind: engine evaluation gap (DSL surface complete; runtime evaluation missing).
- Workaround used in EX9-066: drop the count_gte pre-gate entirely; always present the binary [Return / Draw] choice and rely on the inner `select_trash` being `optional: true`. Acceptable because the action mask still surfaces both branches faithfully. BT24-008 has the same pending workaround documented in its YAML header.
- First reported: 2026-05-03 (EX9-066 Tai Kamiya & Matt Ishida, batch-implement-cards-rust-dsl)

## BT22-017 — `text_contains` (effect-text scan) predicate  [G-DSL-PREDICATE-TEXT-CONTAINS]
- Effect text: BT22-017 [On Play] "Reveal the top 3 cards of your deck. Add 1 card with [Omnimon] in its TEXT and 1 card with the [CS] trait among them to the hand."
- Missing DSL verb / step kind / predicate: `text_contains: Option<String>` leaf on `predicate::PredicateSpec`. The DSL exposes `name_contains` / `name_is` / `name_in` for card-name scans, but has no leaf that scans a candidate's printed `effect_text` / `inherited_text` / `security_text`. DCGO uses `source.HasText("Omnimon")` (BT22_017.cs line 63) which scans the card's effect text for the literal substring.
- Engine data IS present: `code/digimon-engine/src/card_data.rs` carries `effect_text`, `inherited_text`, and `security_text` fields on `CardData` (lines 87, 99, 124). Only the DSL predicate verb is missing.
- Lowers to engine API: a new `text_contains` leaf compiled through `CompiledPredicate` and evaluated in `dsl_cards/predicate.rs` by case-insensitive substring scan against the candidate's combined text. The existing `name_contains` evaluator at `dsl_cards/predicate.rs:705` is the lookalike to clone.
- Suggested DSL syntax:
  ```yaml
  filter:
    text_contains: "Omnimon"
  ```
- Approximation used in BT22-017 today: `name_contains: "Omnimon"`. Narrows correctly for printed Omnimon-named cards (BT12-085, BT22-015, etc.) because their card_name itself carries "Omnimon", but WRONGLY excludes cards that mention `[Omnimon]` only in their effect_text without carrying it in their name (e.g. tutors / supports printed "search for [Omnimon]"). Faithfulness divergence is asserted-and-#[ignore]'d in `bt22_017_on_play_bucket1_admits_card_with_omnimon_only_in_text`.
- Also blocks: any future card whose printed text uses an `in its text` (rather than `in its name`) bucket-filter — including BT12-059's bucket 1 if it were to switch from name-based to text-based per a future erratum.
- Gap kind: DSL vocabulary gap (engine data is present; no DSL surface to filter on it).
- First reported: 2026-05-03 (BT22-017 Gabumon, batch-implement-cards-rust-dsl)

## EX1-068 — grant a triggered effect to opponent's permanent  [G-DSL-GRANT-TRIGGERED-EFFECT-TO-OPPONENT]
- Effect text: EX1-068 [Main] "All of your opponent's Digimon gain '[When Attacking] lose 2 memory' until the end of their next turn."
- Missing DSL verb / step kind / predicate: A `grant_triggered_effect` step that installs a NEW triggered clause (timing + process body) on a SET of cross-permanent targets with a turn-scoped expiry. The DSL today exposes grants for STATIC effects only — `grant_keyword`, `add_modifier` / `add_dp_modifier`, `grant_effect_immunity`. None of those install a clause that itself fires on a future trigger (`when_attacking`, `when_digivolving`, `on_deletion`, ...) on the granted permanent.
- Engine substrate: the Python engine handles this via `permanent.grant_temp_effect(effect, expiry_turn)` + `clear_expired_effects()` (see `qa/archetype-qa/engine-gaps.md` line 33, RESOLVED 2026-03-14 in Python). The Rust engine has the modifier-registry + expiry-tick substrate (`ModifierRegistry` carries per-permanent typed modifiers with `Expiry`), but it does NOT carry a typed `GrantedTriggeredEffect` slot, and there is no `CompiledStep::GrantTriggeredEffect`.
- Lowers to engine API: needs (a) a new `ModifierRegistry` slot (or sibling registry) for per-permanent granted clauses with expiry; (b) the runtime clause dispatcher to consult granted slots when firing a timing on a permanent; (c) a `CompiledStep::GrantTriggeredEffect` whose payload is an inline `CompiledTriggeredClause` (or a registry-keyed template name) lowered against the granted permanent, NOT the source permanent.
- Suggested DSL syntax (option A — inline body):
  ```yaml
  - grant_triggered_effect:
      target:
        of: opponent
        zone: [battle_area]
        kind: digimon
      when: when_attacking
      process:
        - lose_memory: 2     # affects the granted permanent's controller
      expiry: end_of_opponents_turn
  ```
  (Option B — named template: `grant_named_effect: { id: "MemoryMinus2WhenAttacking", target: ..., expiry: ... }` with templates living in a new `code/digimon-engine/src/cards/granted_effects/` registry.)
- Approximation that would VIOLATE no-approximations: a clause that subtracts 2 memory whenever the opponent declares any attack within the expiry window. This over-fires on opponent Digimon played AFTER this Option resolves (DCGO's per-Permanent foreach loop runs ONCE at resolution time and snapshots the eligible Digimon set, so a Digimon played later does not carry the granted clause). Per no-approximations, EX1-068's [Main] clause is OMITTED entirely until the gap closes.
- Also blocks: any "[Main|On Play|When Digivolving] all (your|opponent's) Digimon gain '<bracketed-timing> <body>' until <expiry>" card text. DCGO grep for `UntilOpponentTurnEndEffects.Add` and `UntilOwnerTurnEndEffects.Add` returns ~20+ cards across sets — examples include several Memory-control Options and Tamer support effects across blue/yellow/black.
- Companion engine gap: tracked in `qa/archetype-qa/engine-gaps.md` line 33 as RESOLVED for Python; OPEN for the Rust engine's modifier registry.
- Gap kind: hybrid (Rust engine modifier registry needs a typed grant slot; DSL needs the verb + lowering).
- First reported: 2026-05-03 (EX1-068 Ice Wall!, batch-implement-cards-rust-dsl)

## EX1-021 — Formula-valued `gain_memory` step  [G-DSL-GAIN-MEMORY-FN]
- Effect text: EX1-021 MetalGarurumon — "[When Digivolving] Gain 1 memory for every 4 cards in your hand." DCGO: `count() = card.Owner.HandCards.Count / 4; AddMemory(count())`.
- Status: OPEN (filed 2026-05-03 during EX1-021 batch-implement-cards-rust-dsl).
- Missing DSL verb / step kind / predicate: `StepSpec::GainMemory(i32)` (`code/digimon-dsl/src/step.rs` line 67) is literal-only. There is no `gain_memory_fn:` variant that consumes a `FormulaSpec`. The same shape already exists for cost-reduction declarative bodies (`amount_fn:` on `kind: cost_reduction`, see BT8-097 / BT21-026 / BT24-017) — this gap is about extending the pattern to imperative `process:` steps.
- Lowers to engine API: `EffectContext::add_memory(player, n)` already accepts a runtime-computed integer. The lowering path needs to evaluate the formula via `formula_eval::evaluate_read_with_bindings(&formula, rctx, source_handle, bindings)` then pass the result to `add_memory`.
- Suggested DSL syntax:
  ```yaml
  - gain_memory_fn:
      formula:
        floor_div:
          - card_count_in_zone: { of: you, zone: hand }
          - 4
  ```
- Implementation: add `StepSpec::GainMemoryFn { formula: FormulaSpec }` + serde + `CompiledStep` variant; lowering arm in `dsl_cards/step/memory.rs` (or wherever `GainMemory` lowers today) that evaluates the formula and calls `ctx.add_memory(ctx.source_player(), result)`. Mirror the same shape for `LoseMemoryFn` for symmetry (no current cards request it, but it costs nothing to ship together).
- Workaround attempted: chained `if count_gte hand n: 4k then [gain_memory: 1]` blocks. BLOCKED at runtime by the pre-existing **G-COUNT-GTE-NOT-EVALUATED** gap — generic `count_gte` always evaluates TRUE, so the chained-`if` workaround would always award the full +N memory regardless of hand size. EX1-021 falls back to `process: []` until either gap closes.
- Also blocks: any `gain X memory for every Y of Z` printed-text family. DCGO grep for `AddMemory(.* / .*)` and `AddMemory(.*Count.*)` returns multiple cards across sets including BT5-095 (gain N where N depends on board state), several Tamer EOT memory grants tied to suspended-tamer counts, etc.
- Gap kind: dsl. Engine has `add_memory` and formula evaluation; only the DSL surface is missing.
- First reported: 2026-05-03 (EX1-021 MetalGarurumon, batch-implement-cards-rust-dsl)

## EX1-021 — `has_on_deletion_effect` permanent predicate  [G-DSL-HAS-ON-DELETION-EFFECT]
- Effect text: EX1-021 MetalGarurumon — "[When Attacking] If you have 8 or more cards in your hand and a Tamer in play, return 1 of your opponent's Digimon **that has an [On Deletion] effect** to the bottom of its owners deck." DCGO: `permanent.HasOnDeletionEffect`.
- Status: OPEN (filed 2026-05-03 during EX1-021 batch-implement-cards-rust-dsl).
- Missing DSL verb / step kind / predicate: `PredicateSpec` has no leaf that asks "does this permanent's top card (or any card in its digivolution stack) carry a triggered effect with `EffectTiming::OnDeletion`?" The closest existing leaf is `has_keyword` (which inspects `Keyword` modifiers on the permanent, not effect timings on the underlying card data).
- Engine data IS present: each `CardData` carries the compiled `CompiledCard` (when DSL-authored) with its `effects: Vec<CompiledClause>`; the `CompiledTriggered` clauses include a `when: Vec<CompiledTiming>` that encodes `OnDeletion`. Hand-written `CardEffect` impls expose effects through `card_effects(EffectTiming::OnDeletion, &card)` returning a non-empty list. A new evaluator could walk both surfaces.
- Lowers to engine API: a new `permanent_top_or_sources_have_timing(perm, EffectTiming::OnDeletion)` walker in `dsl_cards/predicate.rs` that checks every card in the permanent's stack (top + sources) for either:
  (a) a compiled DSL clause with `CompiledTiming::OnDeletion` in `when`, or
  (b) a hand-written `CardEffect` impl whose `card_effects(EffectTiming::OnDeletion, ...)` returns non-empty.
  Per the printed text the gate is on the existence of the timing in the card's printed text, not the runtime-active effect set; checking compiled clauses + hand-written impls covers both authoring paths.
- Suggested DSL syntax:
  ```yaml
  filter:
    all_of:
      - kind: digimon
      - has_on_deletion_effect: true
  ```
- Implementation: add `has_on_deletion_effect: Option<bool>` to `PredicateSpec` + `CompiledPredicate`. Evaluate inside `eval_permanent_fields` by walking `perm.card_sources` and consulting each card's `compiled_card` (DSL path) or registry-resolved `CardEffect` (hand-written path) for `OnDeletion`-timed clauses.
- Workaround: omit the `[On Deletion]` filter entirely. NOT acceptable per no-approximations — over-includes opponent Digimon without [On Deletion], so the player would be forced to pick a non-printed-text-eligible target. EX1-021 falls back to `process: []` until the gap closes.
- Also blocks: any "your opponent's Digimon that has an [On Deletion] effect" or "Digimon with a [When Attacking] effect" / "Tamer with a [Your Turn] effect" printed-text family. DCGO grep for `HasOnDeletionEffect` returns ~5 cards; `Has<Timing>Effect` patterns across all timings extend the impact.
- Gap kind: dsl. Engine data is present; only the DSL surface and walker are missing.
- First reported: 2026-05-03 (EX1-021 MetalGarurumon, batch-implement-cards-rust-dsl)

## EX4-060 / BT22-015 — Play card from own digivolution sources  [G-PLAY-FROM-OWN-DIGIVOLUTION-SOURCES]
- Effect text: EX4-060 Omnimon Alter-S — "[All Turns] When this Digimon would leave the battle area other than by one of your effects, play 1 [BlitzGreymon] and 1 [CresGarurumon] from this Digimon's digivolution cards without paying the costs." BT22-015 Omnimon — "<Decode (Red/Black Lv.3)> / <Decode (Blue/Yellow Lv.3)> (When this Digimon would leave the battle area other than in battle, you may play 1 [color] [level] Digimon card from its digivolution cards without paying the cost.)"
- Status: OPEN (filed 2026-05-03 during EX4-060 batch-implement-cards-rust-dsl). Sibling of the BT22-015 Decode entry already documented inline in `code/digimon-engine/cards/bt22/BT22-015.yaml` as `G-DECODE-PLAY-FROM-OWN-DIGIVOLUTION-SOURCES`. Restated here as a standalone reusable primitive — EX4-060 surfaces the SAME engine substrate without going through the Decode keyword, so the gap is naming-agnostic to Decode.
- Missing DSL verb / step kind / predicate: no DSL bind step that selects a card from THIS permanent's digivolution stack with an inline `card_filter:` (kind / name / level / color), and no DSL play step that consumes such a binding and routes the played card from the source stack. `select_hand` / `select_trash` / `select_opponent_permanent` / `select_own_permanent` cover hand, trash, and battle area; there is no `select_self_digivolution_source` step. The play verbs `play_from_hand_free` / `play_from_trash_free` require zone-specific HandIndex / TrashIndex bindings — there is no `play_from_own_digivolution_free` step that consumes a source-stack binding.
- Engine substrate likely needed: `EffectContext::play_from_own_digivolution_cards(source_perm: PermanentHandle, candidate_filter: ..., pay_cost: bool)` that (a) walks the carrier's source stack to find candidates, (b) installs a `SelectionKind` variant surfacing the source picks, (c) on resolution removes the picked source from the stack and uses the existing play-as-new-permanent path with payCost overridden. DCGO models this via `SelectCardEffect.SetUp(customRootCardList: card.PermanentOfThisCard().DigivolutionCards, root: SelectCardEffect.Root.Custom, ...)` followed by `PlayPermanentCards(..., root: SelectCardEffect.Root.DigivolutionCards, payCost: false, activateETB: true)`.
- Suggested DSL syntax (option A — split bind + play):
  ```yaml
  - select_self_digivolution_source:
      bind_as: blitz
      filter:
        all_of:
          - kind: digimon
          - name_contains: "BlitzGreymon"
      prompt: "Select 1 [BlitzGreymon] from this Digimon's digivolution cards to play"
  - play_from_own_digivolution_free: { source: blitz }
  ```
  (Option B — combined: `play_from_own_digivolution_cards: { filter: ..., free: true, optional: false }` that fuses the two; loses the explicit selection-stage binding but matches the printed text more compactly.)
- Workaround that would VIOLATE no-approximations: auto-pick the first matching source card and play it without surfacing the choice. Even when only 1 candidate exists this still leaks an action-selection that the RL action space should observe. Per no-approximations, the entire arm is OMITTED until the gap closes.
- Also blocks: every printed-text use of "play X from this Digimon's digivolution cards" — examples include the BT22-015 Decode clauses (Red/Black + Blue/Yellow Lv.3), EX4-060's [All Turns] arm, and any future "stack reanimator" effects. DCGO grep for `customRootCardList: card.PermanentOfThisCard().DigivolutionCards` returns multiple entries beyond these.
- Gap kind: dsl + engine. DSL needs the bind + play verbs; engine needs the new `EffectContext::play_from_own_digivolution_cards` substrate (zone-scoped selection + source-stack-rooted play path).
- First reported: 2026-05-03 (EX4-060 Omnimon Alter-S, batch-implement-cards-rust-dsl). Sibling clause documented earlier under BT22-015 Decode.

## EX4-060 — Place self at bottom of own security stack face down  [G-PLACE-SELF-AT-SECURITY-BOTTOM]
- Effect text: EX4-060 Omnimon Alter-S — "[All Turns] When this Digimon would leave the battle area other than by one of your effects, ... Then, place this Digimon at the bottom of your security stack face down."
- Status: OPEN (filed 2026-05-03 during EX4-060 batch-implement-cards-rust-dsl). Sibling of the engine-side "Zone-manipulation: security stack operations" gap in `docs/RUST_ENGINE_GAPS.md` (line 262) which calls out `place_security_bottom(player, card)` as a missing `EffectContext` helper for ANY card. EX4-060 surfaces the SELF-DISPOSITION specialisation: the leaving permanent is the subject AND the destination card.
- Missing DSL verb / step kind / predicate: no `place_self_at_security_bottom: {}` step that, when fired from a `kind: replacement` clause whose `replacement_subject` IS this permanent, reroutes the leaving permanent's destination from trash to the controller's security stack bottom face-down. The closest existing primitives are:
  - `place_self_as_delay_option: {}` — places an Option-card self into the battle area as a Delay permanent. Wrong destination zone (battle area, not security) and wrong subject scope (Option only).
  - `add_this_option_to_hand: {}` — routes an Option from security-resolution staging to hand. Wrong destination zone and wrong subject scope.
  - `place_permanent_bottom_security_and_cancel_replacement` — targets ANOTHER permanent (selected via a binding) and CANCELS the replacement. Wrong subject (binding-selected, not self) and wrong outcome (cancel vs proceed-with-reroute).
- Engine substrate likely needed: `EffectContext::place_self_at_security_bottom(&mut self)` that (a) consumes the leaving permanent (top + sources), (b) consults the player-scoped `CannotAddSecurityByEffect` modifier from `docs/RUST_ENGINE_GAPS.md` (when source_player != target_player), (c) places the bundle at the bottom of the controller's security stack face-down, (d) fires `OnAddToSecurity` if any such observer exists, (e) updates `face_up_security` bookkeeping (the placed bundle is face-down, so the slot stays unset). DCGO models this via `IPutSecurityPermanent(card.PermanentOfThisCard(), CardEffectHashtable(activateClass), toTop: false).PutSecurity()`.
- Replacement-outcome semantics: the leave PROCEEDS (default Proceed) but with destination rerouted. Today's replacement clauses default to Proceed-with-trash; this needs either a new Proceed-with-rerouted-zone outcome variant, OR the step internally consumes the leave and routes the cards itself (in which case the outer replacement still "proceeds" but the cards are gone-to-security before trash).
- Suggested DSL syntax:
  ```yaml
  - kind: replacement
    trigger: when_would_leave_battle_area
    active_when:
      all_of:
        - replacement_subject_is_source: true
        - none_of:
            - replacement_cause: own_effect
    process:
      # ... other steps ...
      - place_self_at_security_bottom: {}
  ```
- Workaround that would VIOLATE no-approximations: cancel the replacement and manually `place_self_as_delay_option` (wrong zone). Or fire `place_permanent_bottom_security_and_cancel_replacement` against the source-bound subject (still routes to TOP / cancels rather than proceeding). Both produce wrong observer fan-out and wrong end-state. Per no-approximations the entire clause is OMITTED until both this gap and `G-PLAY-FROM-OWN-DIGIVOLUTION-SOURCES` (the head arm of the same clause) close.
- Also blocks: any future card whose printed text reads "place this Digimon at the bottom of your security stack" or "place this card on top of your security stack" as a self-disposition reroute. DCGO grep for `IPutSecurityPermanent(card.PermanentOfThisCard()` returns multiple entries beyond EX4-060 (notably various endgame "place-self-as-security" recovery effects).
- Gap kind: dsl + engine. DSL needs the verb; engine needs the substrate (face-down placement + zone reroute on Proceed replacement outcome + CannotAddSecurityByEffect modifier check). Companion engine gap tracked in `docs/RUST_ENGINE_GAPS.md` under "Zone-manipulation: security stack operations" — the EX4-060 case is the SELF-subject specialisation, not addressed by the generic helper alone.
- First reported: 2026-05-03 (EX4-060 Omnimon Alter-S, batch-implement-cards-rust-dsl)

## EX4-039 / EX4-038 — Event-target-not-source predicate for OnDigivolve  [G-EVENT-TARGET-NOT-SOURCE]
- Effect text (both): "[Your Turn] [Once Per Turn] When one of your **other** Digimon digivolves, gain 1 memory."
- Status: OPEN as of 2026-05-03. EX4-039 surfaces it; EX4-038 has the same printed-text family.
- Missing DSL verb / step kind / predicate: a `CompiledPredicate` leaf such as `event_target_not_source: true` (or equivalently `event_permanent_not_source: true`) that returns false when the OnDigivolve trigger's `event_permanent` equals the inherited clause's `source_permanent` (the carrier permanent EX4-039 sits under). DCGO encodes this as `permanent != card.PermanentOfThisCard()` inside `CanTriggerWhenPermanentDigivolving`'s `PermanentCondition`.
- Lowers to engine API: `EffectReadContext::source_permanent()` already returns `Option<&Permanent>`; the trigger context's `event_permanent: Option<PermanentHandle>` is populated by `TriggerSource::Digivolved`. Comparing the two handles is a pure read — no new engine method needed.
- Suggested DSL syntax:
  ```yaml
  condition:
    all_of:
      - event_target_owner: you
      - event_target_kind: digimon
      - event_target_not_source: true
  ```
- Workaround applied today: `event_target_owner: you` + `event_target_kind: digimon`. Over-fires when the carrier permanent itself digivolves further (e.g. CARRIER-Lv4 → CARRIER-Lv5 while EX4-039 is a source under CARRIER). `once_per_turn: true` softens the impact to at most +1 spurious memory per turn. The negative-case behavioral test (`ex4_039_inherited_does_not_fire_when_carrier_itself_digivolves`) is `#[ignore]`'d pending closure.
- Also blocks: EX4-038 Agumon (sister card, identical inherited text). Other "When one of your other Digimon ..." printed-text families across EX4 and BT5/BT12 will reuse the same predicate. DCGO grep for `permanent != card.PermanentOfThisCard()` inside `OnDigivolve` / `OnEnterFieldAnyone` PermanentCondition shows the pattern recurs across cards.
- Gap kind: dsl. Engine already has both data points (`event_permanent` on `TriggerContext` for `Digivolved`, `source_permanent` on `EffectReadContext`); only the DSL predicate surface and its evaluator branch in `eval_event_fields` are missing.
- First reported: 2026-05-03 (EX4-039 Gabumon, batch-implement-cards-rust-dsl)

## EX9-021 — `is_dna_digivolving` predicate on triggered clauses  [G-DSL-IS-DNA-DIGIVOLVING]
- Effect text: EX9-021 Omnimon Alter-S — "[When Digivolving] **If DNA digivolving**, your opponent's effects don't affect this Digimon for the turn. Then, delete all of their Digimon with the highest level." DCGO splits the body on `CardEffectCommons.IsJogress(_hashtable)` — a per-trigger hashtable flag set when the digivolve was a DNA / jogress path.
- Status: OPEN (filed 2026-05-03 during EX9-021 batch-implement-cards-rust-dsl). Hybrid engine + DSL gap.
- Missing DSL verb / step kind / predicate: `PredicateSpec` exposes no `is_dna_digivolving: bool` leaf, and the `condition:` shape on a triggered clause has no equivalent. There is also no clause-level `if:` form (matches in `process:` body) that can branch on the DNA-vs-standard digivolve origin.
- Engine substrate also missing: `TriggerSource::Digivolved { player, permanent, card }` (`code/digimon-engine/src/selection.rs:352`) has NO `via_dna` / `from_dna_pair` flag. The DNA digivolve action path (`Game::initiate_dna_digivolve` etc.) does not currently enqueue a distinct trigger source for the DNA case. The dispatch code that lifts `Digivolved { ... }` into `TriggerContext` (`effect_queue.rs` around line 479) builds a context with `event_permanent` / `event_card` / `source_player` but no DNA discriminator.
- Lowers to engine API: needs (a) `via_dna: bool` (or `dna_pair: Option<(CardHandle, CardHandle)>`) field on `TriggerSource::Digivolved`, populated from the DNA-digivolve action handler; (b) surfacing on `TriggerContext` so DSL predicates can read it; (c) DSL `is_dna_digivolving: Option<bool>` leaf on `PredicateSpec` + `CompiledPredicate` with an evaluator that consults the trigger context flag (false at non-trigger-time, same convention as `event_target_owner`).
- Suggested DSL syntax:
  ```yaml
  - when: when_digivolving
    condition:
      is_dna_digivolving: true
    process:
      - grant_effect_immunity:
          target: source
          source_kind: any
          source_controller: opponent
          expiry: end_of_turn
  ```
  (Optional symmetric dual: `is_standard_digivolving: true` for "[If standard digivolving] X" forms.)
- Workaround that would VIOLATE no-approximations: always grant the immunity (over-fires on the standard-digivolve path), or never grant it (under-fires on DNA — the printed protection is lost). Both are unfaithful. Per no-approximations the DNA-gated immunity arm is OMITTED. The unconditional delete-highest tail of EX9-021's [When Digivolving] IS implemented (printed grammar + DCGO sequencing both confirm the delete fires regardless of the DNA gate).
- Also blocks: any future card with "[When Digivolving] If DNA digivolving, X" or "[When Digivolving] If you DNA digivolved, X" style printed text. DCGO grep for `IsJogress(` returns multiple cards across sets (notably Omnimon-family / DNA-archetype cards). Sibling-but-distinct from AD1-001's `dna_origin: true` predicate, which reads card-data origin metadata rather than per-trigger event metadata.
- Gap kind: hybrid (engine TriggerSource needs the flag + dispatch wiring; DSL needs the predicate). Tests `ex9_021_when_digivolving_dna_path_grants_self_opp_effect_immunity` and `ex9_021_when_digivolving_standard_path_does_not_grant_immunity` are `#[ignore]`'d under this gap tag.
- First reported: 2026-05-03 (EX9-021 Omnimon Alter-S, batch-implement-cards-rust-dsl).

## EX9-021 — Place self at TOP of own security stack face-up  [G-PLACE-SELF-AT-SECURITY-TOP]
- Effect text: EX9-021 Omnimon Alter-S — "[End of Attack] ... If this effect played, place this Digimon as your top security card." DCGO: `IPutSecurityPermanent(card.PermanentOfThisCard(), CardEffectHashtable, toTop: true).PutSecurity()` — places this permanent (top + sources) at the TOP of the controller's security stack (face-up; printed text does not specify face-down).
- Status: OPEN (filed 2026-05-03 during EX9-021 batch-implement-cards-rust-dsl). Sibling of `G-PLACE-SELF-AT-SECURITY-BOTTOM` (EX4-060) — same engine substrate, different placement target (TOP vs bottom) and different trigger-clause kind (end-of-attack tail vs would-leave replacement).
- Missing DSL verb / step kind / predicate: no `place_self_at_security_top: {}` step. Closest existing primitives are `place_self_as_delay_option` (places an Option-card self into the battle area as a Delay; wrong destination + Option-only), `add_this_option_to_hand` (routes Option to hand; wrong zone), and `place_permanent_bottom_security_and_cancel_replacement` (targets a binding-selected permanent and cancels the replacement; wrong subject + wrong outcome).
- Engine substrate likely needed: `EffectContext::place_self_at_security(top_or_bottom: SecurityPlacement, face_orientation: SecurityFace)` that consumes the source permanent's top card + sources, places the bundle at the specified end of the controller's security stack with the specified orientation, fires `OnAddToSecurity`, and updates `face_up_security` bookkeeping. EX4-060 (bottom face-down) and EX9-021 (top face-up) cover both axes of the helper, so this gap should be closed in one engine slice alongside `G-PLACE-SELF-AT-SECURITY-BOTTOM`.
- Suggested DSL syntax (option A — separate verbs):
  ```yaml
  - place_self_at_security_top: {}           # face-up by default
  ```
  (Option B — unified):
  ```yaml
  - place_self_at_security:
      position: top                          # top | bottom
      face: up                               # up | down (printed default
                                             # for top is up; for bottom is down)
  ```
- Workaround that would VIOLATE no-approximations: cancel the leave / keep self in battle area; both produce the wrong board state. Per no-approximations the entire [End of Attack] clause is OMITTED until both `G-PLAY-FROM-OWN-DIGIVOLUTION-SOURCES` (the head arm) AND this gap close (the "If this effect played" tail is gated on the head arm having executed at least one play, so neither half can stand alone in a faithful encoding).
- Also blocks: any future "place this Digimon as your top security card" or "place this card on top of your security stack face up" self-disposition tail. DCGO grep for `IPutSecurityPermanent(card.PermanentOfThisCard(...toTop: true)` returns multiple entries beyond EX9-021. Companion engine gap tracked in `docs/RUST_ENGINE_GAPS.md` under "Zone-manipulation: security stack operations" — a unified helper covering both top/bottom placements would close both DSL gaps simultaneously.
- Gap kind: hybrid (engine substrate + DSL verb). Test `ex9_021_end_of_attack_plays_two_from_stack_then_places_self_top_security` is `#[ignore]`'d under this gap tag (combined with `G-PLAY-FROM-OWN-DIGIVOLUTION-SOURCES`).
- First reported: 2026-05-03 (EX9-021 Omnimon Alter-S, batch-implement-cards-rust-dsl). Sibling clause tracked at `G-PLACE-SELF-AT-SECURITY-BOTTOM` (EX4-060).

## ST20-10 — Inverse alt-path direction: "this card may digivolve INTO X"  [G-ALT-PATH-DIRECTION-INTO]
- Effect text: ST20-10 Agumon — "[Your Turn] While your opponent has a Digimon with 10000 DP or more, or your Tamers have 3 or more total colors, this Digimon can digivolve into [WarGreymon] in the hand for a digivolution cost of 4, ignoring digivolution requirements." Other warp-style printed effects with the "this Digimon can digivolve into [Card] in the hand" shape are likely siblings (DCGO grep for `cardCondition: ... CardSource.EqualsCardName(...)` paired with `permanentCondition: ... == card.PermanentOfThisCard()` inside `AddSelfDigivolutionRequirementStaticEffect`).
- Status: OPEN (filed 2026-05-03 during ST20-10 batch-implement-cards-rust-dsl).
- Missing DSL verb / step kind / predicate: `AltPathSpec` (in `digimon-dsl/src/alt_path.rs`) is implicitly source-directed — `from:` filters the SOURCE permanent / hand card that may digivolve INTO the carrier. There is no inverse form for "this card grants ITSELF the ability to digivolve into card X in hand." Authoring the alt-path on the destination card (WarGreymon's YAML) would over-broadcast: every Lv3 Agumon-named card on the field would be presented the path, and the destination YAML would have to enumerate every "warp into me" effect across the card pool. Authoring on the source (ST20-10) is the natural printed-text home but the DSL has no syntax for it.
- Lowers to engine API: the engine's activated-digivolve mechanism already supports both `cardCondition` (target hand-card filter) and `permanentCondition: target == self` (source = this card) in DCGO's `AddSelfDigivolutionRequirementStaticEffect`. The gap is purely DSL-side: a new `AltPathSpec` direction flag (or a new `kind: warp_into_hand` variant) needs to flip the semantic of `from:` to filter the destination instead of the source.
- Suggested DSL syntax (option A — direction flag):
  ```yaml
  alt_paths:
    - kind: activated_digivolve
      direction: into            # NEW: source = self, target = `into:` filter
      into:
        zone: [hand]
        of: you
        name_is: "WarGreymon"
      cost: 4
      ignore_requirements: true
  ```
  (Option B — dedicated kind): `kind: warp_into_hand` with required `into:` field (no `from:`); same lowering on the engine side.
- Workaround that would VIOLATE no-approximations: silently move the alt-path to WarGreymon's YAML (over-broadcasts to every Lv3 controller) or omit the gating predicate (path always available regardless of opp DP / Tamer colours). Per no-approximations the warp clause is OMITTED until this gap closes. Five behavioral tests in `code/digimon-engine/tests/cards_behavioral/st20/st20_10.rs` are `#[ignore]`'d under this gap tag (paired with `G-ALT-PATH-CONDITION` and either `G-PRED-DP-LTE` or `G-DSL-DISTINCT-TAMER-COLORS`).
- Also blocks: any future "this Digimon can digivolve into [Card] in the hand for cost N" warp effect printed on the source card with a self-controller-state gate.
- Gap kind: dsl. Engine substrate already exists (DCGO uses the same `AddSelfDigivolutionRequirementStaticEffect` factory regardless of direction).
- First reported: 2026-05-03 (ST20-10 Agumon, batch-implement-cards-rust-dsl). Companion gap to `G-ALT-PATH-CONDITION` (BT24-016) — the gating predicate hole and the inverse-direction hole both block ST20-10 independently.

## ST20-10 — Distinct-Tamer-colours-on-field BoolPredicate  [G-DSL-DISTINCT-TAMER-COLORS]
- Effect text: ST20-10 Agumon — "...or your Tamers have 3 or more total colors..." (gating disjunct of the [Your Turn] warp clause). Sibling form of BT21-102 Tai Kamiya's "For each of your Tamers' colors, add 1 to this effect's play cost maximum" — both reference the same per-colour-count computation, but BT21-102 needs the value as a `FormulaSpec::per` aggregate (tracked under `G-DSL-DISTINCT-TAMER-COLORS-FORMULA`) while ST20-10 needs it as a BoolPredicate threshold ("3 or more").
- Status: OPEN (filed 2026-05-03 during ST20-10 batch-implement-cards-rust-dsl).
- Missing DSL verb / step kind / predicate: no `distinct_tamer_colors_gte: <N>` (or generalised `distinct_colors_count_gte: <N>` over a controller / kind / zone selector) BoolPredicate leaf on `PredicateSpec`. The existing `distinct_colors_count` (added under `G-DSL-DISTINCT-TAMER-COLORS-FORMULA`) is only available inside `FormulaSpec::per` — it cannot appear as a standalone boolean condition. `color_only` / `color_is` filter individual permanents by colour but do not aggregate colour counts across a permanent set.
- Lowers to engine API: DCGO's `Combinations.GetDifferenetColorCardCount(tamerCards) >= 3` returns the count of distinct colours present across the supplied permanent set, then thresholds. The engine's `eval_aggregate` (already used by `FormulaSpec::per: distinct_colors_count`) covers the count primitive — only the BoolPredicate wrapping is missing.
- Suggested DSL syntax (option A — dedicated leaf):
  ```yaml
  condition:
    distinct_tamer_colors_gte: 3
  ```
  (Option B — generalised over a permanent selector):
  ```yaml
  condition:
    distinct_colors_count:
      of: you
      zone: [battle_area]
      filter: { kind: tamer }
      gte: 3
  ```
- Workaround that would VIOLATE no-approximations: drop the disjunct entirely (gate fires only on opp ≥10000 DP, never on Tamer colours), or replace with a coarser proxy like "you have 3+ Tamers" (over-fires on three same-colour Tamers, under-fires on 3 distinct-colour Tamers some of which are deleted). Per no-approximations the entire warp clause is OMITTED until this gap (paired with `G-ALT-PATH-DIRECTION-INTO` and `G-ALT-PATH-CONDITION`) closes.
- Also blocks: any future "while your Tamers have N or more total colours" or "if you have N or more distinct-colour Tamers" gate. Sibling to `G-DSL-DISTINCT-TAMER-COLORS-FORMULA` (BT21-102) — the formula-aggregate form lands the underlying primitive; this gap closes the BoolPredicate wrapping. Both should land together once the formula primitive is generalised to also expose its result as a comparable scalar.
- Gap kind: dsl. Engine has the count primitive via `eval_aggregate`.
- First reported: 2026-05-03 (ST20-10 Agumon, batch-implement-cards-rust-dsl). Sibling of `G-DSL-DISTINCT-TAMER-COLORS-FORMULA` (BT21-102).

## Puppets Resolver Residual DSL/Hybrid Gaps (2026-05-04)

## BT13-101 / P-136 — event predicates with suspend-this-Tamer cost  [PUPPETS-G023]

- Effect text: `BT13-101`: "[All Turns] When you play a 2-color black/yellow Digimon, by suspending this Tamer, <Draw 1> and gain 1 memory." `P-136`: "[Your Turn] [Once Per Turn] When one of your Digimon digivolves into a Digimon with the [Puppet] trait, by suspending this Tamer, gain 1 memory."
- Missing DSL verb / step kind / predicate: event-card predicates for exact color sets and color count, event-target owner/trait predicates for digivolve observers where needed, plus declarative source-bound triggered activation costs.
- Companion engine state: the generic triggered activation-cost hook is tracked in `docs/RUST_ENGINE_GAPS.md`; DSL must be able to bind it to "suspend this Tamer" and preflight availability before exposing a prompt.
- Suggested DSL syntax:
  ```yaml
  condition:
    all:
      - event_card_kind: digimon
      - event_card_color_only: [black, yellow]
      - event_card_color_count: 2
      # or, for P-136-style digivolve observers:
      - event_target_owner: you
      - event_card_trait_has: Puppet
  activation_cost:
    suspend_this_tamer: {}
  ```
- Gap kind: hybrid. Event-card color predicates are DSL/evaluator vocabulary; source-bound triggered cost preflight needs the engine cost surface.
- Workaround: None faithful. Name, trait, or broad color-includes filters would admit illegal cards for `BT13-101`, and auto-suspending the Tamer would hide a player-visible cost for both cards.
- First reported: 2026-05-04 (Puppets resolver Batch 8, BT13-101). Updated 2026-05-04 by Batch 11 for `P-136`.

---

## BT16-055 — narrow protection and inherited rules-text predicate  [PUPPETS-G024/PUPPETS-G025]

- Effect text: "While you have 3 or more security cards, this Digimon isn't affected by your opponent's DP reduction effects and can't be de-digivolved by their effects." / "[Your Turn] While this Digimon has [Pulsemon] in its text, it gets +1000 DP."
- Missing DSL verb / step kind / predicate: category-scoped protection modifiers for opponent DP reduction and opponent De-Digivolve; inherited predicate over the carrier stack's printed rules text.
- Companion engine state: broad `CannotBeAffected` is too strong for the protection branch, and current inherited predicates do not inspect rules text on the carrier.
- Suggested DSL syntax:
  ```yaml
  protection:
    from: opponent
    categories: [dp_reduction, de_digivolve]
    while: { security_count_gte: 3 }

  active_when:
    carrier_text_contains: "Pulsemon"
  ```
- Gap kind: hybrid for narrow protection, DSL for rules-text contains predicate.
- Workaround: None faithful. Broad immunity or name predicates would over- or under-match printed behavior.
- First reported: 2026-05-04 (Puppets resolver Batch 8, BT16-055)

---

## BT20-084 — trash-resident effect digivolve and stacked-card-to-security  [PUPPETS-G026/PUPPETS-G027]

- Effect text: "[Trash] [All Turns] When any of your Digimon are played, 1 of your [Sistermon Ciel]s may digivolve into this card without paying the cost." / "[End of All Turns] Place this Digimon's top stacked card as the top security card."
- Missing DSL verb / step kind / predicate: trash-resident global observer dispatch; effect-initiated digivolve from a trash source card; curated top-stacked-card extraction and security-top placement.
- Companion engine state: existing effect-digivolve paths are not source-parametric for trash cards, and the reusable stack/security gap currently covers selected digivolution sources, not active top-card extraction with legal permanent cleanup.
- Suggested DSL syntax:
  ```yaml
  - scope: trash
    when: on_ally_played
    optional: true
    process:
      - select_own_permanent:
          bind_as: ciel
          filter: { name_is: "Sistermon Ciel" }
      - effect_digivolve_from_trash:
          target: ciel
          card: self
          cost: 0

  - place_top_stacked_card_as_security_top: { of: you }
  ```
- Gap kind: hybrid. The observer/source move needs engine support; DSL needs vocabulary for the new source and stack movement operations.
- Workaround: None faithful. Hand-based digivolve or raw stack mutation would move the wrong card or bypass action-mask and cleanup contracts.
- First reported: 2026-05-04 (Puppets resolver Batch 8, BT20-084)

---

## BT22-088 — return-this-Tamer cost before branch free-play  [PUPPETS-G028]

- Effect text: "[Start of Your Main Phase] By returning this Tamer to the bottom of the deck, you may play 1 [Arisa Kinosaki] with a different card number in your hand without paying the cost, or play 1 [Shoemon] from your hand or trash without paying the cost."
- Missing DSL verb / step kind / predicate: optional triggered activation cost that moves the source permanent to the bottom of deck, then an in-effect branch selector with origin-preserving hand/trash play consumers.
- Companion engine state: the generic triggered activation-cost hook is tracked in `docs/RUST_ENGINE_GAPS.md`; this card also needs a source-zone move as the cost and a follow-on branch selector.
- Suggested DSL syntax:
  ```yaml
  activation_cost:
    return_this_tamer_to_bottom_deck: {}
  choose_one:
    - play_from_hand_free:
        filter:
          all_of:
            - name_is: "Arisa Kinosaki"
            - card_id_not: "BT22-088"
    - play_from_hand_or_trash_free:
        filter: { name_is: "Shoemon" }
  ```
- Gap kind: hybrid. The cost/preflight is engine-facing; branch and origin-preserving selection need DSL vocabulary.
- Workaround: None faithful. Auto-returning the Tamer or auto-selecting Shoemon/Arisa would hide printed player-visible choices.
- First reported: 2026-05-04 (Puppets resolver Batch 8, BT22-088)

---

## BT23-077 — self-scoped OnSuspend event predicate  [PUPPETS-G029]

- Effect text: "[All Turns] When this Digimon suspends, <De-Digivolve 1> 1 of your opponent's Digimon."
- Missing DSL verb / step kind / predicate: a predicate over `OnSuspend` event context that proves the suspending event permanent is the same permanent as the effect source.
- Companion engine state: `OnSuspend` dispatch exists and event context is available for observed suspend events, but YAML cannot currently express `event_permanent_is_source`.
- Suggested DSL syntax:
  ```yaml
  - when: on_suspend
    condition: { event_permanent_is_source: true }
    process:
      - select_opponent_permanent:
          bind_as: target
          filter: { kind: digimon }
      - de_digivolve: { target: target, count: 1 }
  ```
- Gap kind: dsl predicate/evaluator gap.
- Workaround: None faithful. A broad `on_suspend` trigger would over-fire when any other permanent suspends.
- First reported: 2026-05-04 (Puppets resolver Batch 9, BT23-077)

---

## BT5-106 — effect-play On Play suppression provenance  [PUPPETS-G030]

- Effect text: "[Security] You may play 1 level 3 purple Digimon card from your trash without paying its memory cost. Any [On Play] effects on Digimon played with this effect don't activate."
- Missing DSL verb / step kind / predicate: a play-from-trash/free-play consumer that carries `suppress_on_play: true` provenance for the played Digimon only.
- Companion engine state: ordinary effect play from trash can enter the Digimon and normally fire On Play; this card needs the same player-visible trash selection but must skip the played permanent's On Play enqueue for that play event.
- Suggested DSL syntax:
  ```yaml
  - play_from_trash_free:
      target: chosen
      suppress_on_play: true
  ```
- Gap kind: hybrid. Engine play provenance needs an On Play suppression flag, and DSL needs vocabulary to request it.
- Workaround: None faithful. Omitting the play hides a legal security choice; ordinary play-from-trash would illegally fire the played Digimon's On Play effects.
- First reported: 2026-05-04 (Puppets resolver Batch 9, BT5-106)

## BT3-002 — `carrier_has_keyword` predicate for inherited clause conditions  [G-DSL-CARRIER-HAS-KEYWORD]

- Effect text: "Inherited Effect [When Attacking] [Once Per Turn] If this Digimon has <Jamming>, <Draw 1> (Draw 1 card from your deck.)"
- Card first discovered in: BT3-002 DemiVeemon (Digi-Egg, Lv.2, Blue)
- Missing DSL verb / step kind / predicate: `carrier_has_keyword` — a `PredicateSpec` / `BoolPredicate` leaf for inherited triggered clauses that checks whether the TOP CARD of the permanent carrying the egg source has a given keyword (printed OR modifier-granted). The existing `has_keyword` predicate in `CompiledPredicate` evaluates on `source_permanent` (the egg slot itself), not the carrier permanent. For inherited effects, `source_permanent` is the bottom-of-stack source card, not the carrier Digimon.
- Lowers to engine API: `Game::has_keyword(carrier_handle, Keyword::Jamming)` — the engine has this method (used in `combat.rs`, `game.rs`). The gap is that the DSL predicate evaluator has no path to resolve the carrier handle from `EffectReadContext` for inherited clauses. The carrier handle is `EffectReadContext.source_permanent` (if it exists) but only when the source IS the top card; for sub-stack inherited sources, the context's `source_permanent` is the egg, not the carrier.
- Suggested DSL syntax:
  ```yaml
  - scope: inherited
    when: when_attacking
    once_per_turn: true
    optional: true
    condition: { carrier_has_keyword: Jamming }
    process:
      - draw: { of: you, count: 1 }
  ```
- Gap kind: dsl (engine has `Game::has_keyword` and modifier tracking; DSL lowering just needs a new predicate leaf that reads the carrier handle from the inherited-effect dispatch context rather than the source permanent).
- Workaround: Omit the `condition` from the YAML entirely (preferred). The clause over-fires without the Jamming gate — any carrier with BT3-002 in its digivolution cards will draw on attack regardless of Jamming. The over-fire is documented in BT3-002.yaml. The negative-condition test `bt3_002_does_not_fire_without_jamming` is `#[ignore = "pending: G-DSL-CARRIER-HAS-KEYWORD from qa/dsl-vocab-gaps.md"]`.
- Trade-off of omission vs. un-gated clause: omission is preferred because the Draw 1 step is safe (no permanent game-state harm), the positive case (carrier has Jamming → draw) is the common path this egg was designed for, and over-firing without Jamming is a minor accuracy loss rather than a silent break.

---

## BT12-022 — `active_when` on `kind: grant_keyword` declarative clauses is not consumed  [G-DSL-GRANT-KEYWORD-ACTIVE-WHEN-NOT-CONSUMED]

- Effect text: "[Your Turn] While this Digimon has [Imperialdramon] in its name or the [Free] trait, it gains ＜Jamming＞" (BT12-022 ExVeemon, inherited)
- Missing DSL verb / step kind / predicate: `DeclarativeClause.active_when` is compiled into `CompiledDeclarativeClause::GrantKeyword { active_when, .. }` but is silently discarded by `lower_grant_keyword::lower` in `code/digimon-engine/src/dsl_cards/mod.rs` (line 82-98 uses `..` to destructure, ignoring `active_when`). The `lower_grant_keyword::lower` function signature has no `active_when` parameter.
- Companion state: `CompiledDeclarativeClause::GrantKeyword` does carry the `active_when: Option<CompiledPredicate>` field (compiled.rs:432). The `lower_aura::lower` function accepts and uses `active_when` correctly. The gap is that `lower_grant_keyword::lower` does not accept or apply it.
- Consequence: any `kind: grant_keyword` clause with `active_when:` specified will grant the keyword unconditionally — the condition is silently dropped. Cards relying on `active_when` to gate keyword grants over-fire.
- Lowers to engine API: `Effect::declarative(card).condition(move |rctx| eval_predicate(&aw, rctx, PredicateSubject::None))` — the condition closure already exists in `lower_aura::lower`; the same pattern needs to be applied in `lower_grant_keyword::lower`. Additionally, `Game::has_keyword` checks `effect.condition` for inherited declarative effects (game.rs lines 1717-1727) — so adding the condition to the `Effect` struct (not only the modifier tick) would gate the keyword check correctly without a declarative tick.
- Suggested fix:
  1. Add `active_when: Option<CompiledPredicate>` parameter to `lower_grant_keyword::lower`.
  2. In `mod.rs`, pass `active_when.clone()` to the call.
  3. Inside `lower_grant_keyword::lower`, add `if let Some(aw) = active_when { builder = builder.condition(move |rctx| eval_predicate(&aw, rctx, PredicateSubject::None)); }`.
- Gap kind: dsl (engine has condition support on `Effect` struct; only the lowering wire-up is missing).
- Workaround: Ship the clause without `active_when` (unconditional keyword grant, over-fires). Or omit the clause entirely. BT12-022 ships with `active_when` specified but unconditionally firing. Negative-condition tests are `#[ignore = "pending: G-DSL-GRANT-KEYWORD-ACTIVE-WHEN-NOT-CONSUMED from qa/dsl-vocab-gaps.md"]`.
- Cards affected: BT12-022 ExVeemon (inherited conditional Jamming).
- First reported: 2026-05-04 (BT12-022 batch-implement-cards-rust-dsl)

---

## BT12-022 — BeforePayCost triggered gain_memory for "would DNA digivolve into" target  [G-BEFORE-PAY-COST-GAIN-MEMORY]

- Effect text: "[Your Turn] When this Digimon would DNA digivolve into a green Digimon card, gain 1 memory." (BT12-022 ExVeemon)
- Missing DSL verb / step kind / predicate: The DSL `kind: cost_reduction` with `reduction_timing: before_pay_cost` models only cost reductions (integer decrements to `memory_cost`). There is no triggered declarative form for `gain_memory` at `BeforePayCost` timing. DCGO uses `EffectTiming.BeforePayCost` with `CanTriggerWhenPermanentWouldDigivolveOfCard + IsJogress` + `card.Owner.AddMemory(1)` — the memory gain is an arbitrary effect (not a cost reduction) triggered at pre-pay-cost time.
- Companion gap: G-BEFORE-PAY-COST-DIGIVOLVE-TARGET (already in qa/dsl-vocab-gaps.md) — the target-card threading (checking the would-digivolve-into card's color) is also missing. Both gaps must close before BT12-022 clause 0 can be implemented.
- Companion note on `on_dna_digivolve` alternative: `on_dna_digivolve` timing fires AFTER DNA digivolve completes, so it could not faithfully model the "would" semantics. Also, no `event_card_color_is` predicate exists in `PredicateSpec` for filtering by the result card's color.
- Lowers to engine API: `BeforePayCost` timing dispatch exists in `scan_before_pay_cost_reduction`; the gap is that it only updates `cost_delta`, not an arbitrary `gain_memory` side effect. A new DSL form (e.g., `kind: before_pay_cost_trigger`) with a `process:` body (not a `CostReductionBody`) would be needed.
- Suggested DSL syntax (once G-BEFORE-PAY-COST-DIGIVOLVE-TARGET also closes):
  ```yaml
  - scope: own
    kind: before_pay_cost_trigger       # NEW form — triggered effect at BeforePayCost
    when_this_digivolves_into:
      target_color_is: green            # NEW predicate (needs G-BEFORE-PAY-COST-DIGIVOLVE-TARGET)
      dna_only: true
    active_when: { your_turn: true }
    process:
      - gain_memory: 1
  ```
- Gap kind: hybrid (engine-side: BeforePayCost dispatch handles only cost_delta; DSL-side: no `before_pay_cost_trigger` kind with process body).
- Cards blocked: BT12-022 clause 0 (BLOCKED, omitted from YAML).
- First reported: 2026-05-04 (BT12-022 batch-implement-cards-rust-dsl)
- First reported: 2026-05-04 (BT3-002 DemiVeemon DSL implementation)

## EX1-014 — `aura` declarative target scoping  [G-DSL-AURA-TARGET-SOURCE-PERMANENT]

- Effect text: "[Your Turn] While this Digimon has [Imperialdramon] in its name or the [Free] trait, it gains ＜Jamming＞" — should grant Jamming ONLY to the carrier permanent (the Digimon containing this card in its digivolution stack), not all controller-side Digimon.
- Card first discovered in: EX1-014 ExVeemon (Digimon, Lv.4, Blue), also in BT12-022 (sister card).
- Missing DSL verb / step kind / predicate: `target_is_source: true` BoolPredicate (or equivalent) usable inside `kind: aura` `target:` filter, so the aura applies only to the carrier of the source permanent — not the entire `target: { owner: you, kind: digimon }` set. Currently `lower_aura.rs` applies to all matches of the target predicate.
- Lowers to engine API: `target` filter check `handle == ctx.source_permanent` (or `handle == carrier_of(source)` for inherited-source clauses).
- Suggested DSL syntax:
  ```yaml
  - kind: aura
    target: { owner: you, kind: digimon, is_carrier_of_source: true }
    grant_keyword: jamming
    active_when: { ... }
  ```
- Gap kind: dsl. Engine has the carrier handle resolution; only the predicate leaf is missing.
- Workaround: ship aura with broad target (over-fires to all your Digimon).
- First reported: 2026-05-04 (EX1-014 batch-implement-cards-rust-dsl)

---

## EX1-014 — `self_digivolution_contains_trait` predicate  [G-DSL-SELF-DIGIVOLUTION-CONTAINS-TRAIT]

- Effect text: "...has [Imperialdramon] in its name or the [Free] trait..." — needs a predicate that evaluates whether the carrier permanent's digivolution stack contains a card with a given trait.
- Card first discovered in: EX1-014 ExVeemon (Digimon, Lv.4, Blue).
- Missing DSL verb / step kind / predicate: `self_digivolution_contains_trait: <trait>` — boolean predicate over carrier permanent's digivolution stack. `source_permanent_trait_has` exists in `CompiledPredicate` spec but is not evaluated at runtime in `predicate.rs`.
- Lowers to engine API: `rctx.source_permanent()?.has_trait(name, rctx.card_data())` — engine has the data.
- Suggested DSL syntax:
  ```yaml
  active_when: { self_digivolution_contains_trait: "Free" }
  ```
- Gap kind: dsl.
- Workaround: omit the trait arm of the active_when (only name arm fires).
- First reported: 2026-05-04 (EX1-014 batch-implement-cards-rust-dsl)

---

## BT16-040 — effect-initiated digivolve from hand with permanent-target chain  [G-EFFECT-INITIATED-DIGIVOLVE-FROM-HAND-WITH-PERMANENT-TARGET]

- Effect text: "[Start of Your Main Phase] [On Play] If it's your turn, 1 of your Digimon may digivolve into a level 4 Digimon card with the [Insectoid] or [Free] trait in your trash with the digivolution cost reduced by 1." — process chain: select_own_permanent → select_trash_card → effect_initiated_digivolve.
- Card first discovered in: BT16-040 Wormmon (Digimon, Lv.3, Green/White). Same gap blocks BT17-015, BT17-027 clause 0.
- Missing DSL verb / step kind / predicate: process chain terminates after the permanent pick; the trash-pick prompt and `effect_initiated_digivolve` verb never execute when the source target is bound from a previous `select_own_permanent` step.
- Lowers to engine API: `EffectContext::effect_initiated_digivolve` exists; the chain orchestration in the lowering layer does not resume after the first pick when the resolved source binding feeds into a subsequent select prompt.
- Suggested DSL syntax: existing chain syntax should work; the gap is in the process-step continuation mechanism.
- Gap kind: dsl.
- Workaround: clause omitted from runtime; structural test passes, behavioral tests `#[ignore]`'d.
- First reported: 2026-05-04 (BT16-040 batch-implement-cards-rust-dsl)

## BT12-028 / BT16-025 / BT16-027 — `stack_size_lte_source` predicate  [G-PRED-STACK-SIZE-LTE-SOURCE]

- Effect text variants: "Return 1 of your opponent's Digimon with as many or fewer digivolution cards as this Digimon to the bottom of the deck." (BT16-027) / "Suspend all of your opponent's Digimon with as many or fewer digivolution cards as this Digimon" (BT16-025).
- Card first discovered in: BT16-027 Imperialdramon: Fighter Mode. Cross-listed in BT16-025 Paildramon (same gap).
- Missing DSL verb / step kind / predicate: `stack_size_lte_source: bool` BoolPredicate leaf evaluating `candidate.card_sources.len() <= source_permanent.card_sources.len()` at runtime. The existing `stack_size_lte: <u8>` takes a literal, not a dynamic source-stack reference.
- Lowers to engine API: `Game::permanent(handle).card_sources.len()` for both candidate and source — engine has the data; only the predicate dispatch is missing.
- Suggested DSL syntax: `filter: { stack_size_lte_source: true }` inside `select_opp_field` / `select_permanent`.
- Gap kind: dsl.
- Workaround: clauses omitted from runtime; structural tests pass; behavioral tests `#[ignore]`'d.
- First reported: 2026-05-04 (BT16-027 batch-implement-cards-rust-dsl).

---

## BT12-028 / BT16-027 — `self_digivolution_contains_name` predicate  [G-DSL-SELF-DIGIVOLUTION-CONTAINS-NAME]

- Effect text: "if [Imperialdramon: Dragon Mode] is in this Digimon's digivolution cards" (BT16-027). Sister of `G-DSL-SELF-DIGIVOLUTION-CONTAINS-TRAIT` (EX1-014).
- Card first discovered in: BT16-027 Imperialdramon: Fighter Mode. Cross-listed in BT12-028 (`source_name_contains` family).
- Missing DSL verb / step kind / predicate: `self_digivolution_contains_name: <name>` BoolPredicate leaf evaluating whether the source permanent's own `card_sources` stack contains a card matching the given name. `source_name_contains` is defined in `PredicateSpec` and validated, but has no runtime evaluation branch in `predicate.rs`.
- Lowers to engine API: `Permanent::contains_card_name` — engine has the primitive; only the predicate dispatch wiring is missing.
- Suggested DSL syntax: `condition: { self_digivolution_contains_name: "Imperialdramon: Dragon Mode" }`.
- Gap kind: dsl.
- Workaround: clause omitted from runtime; behavioral tests `#[ignore]`'d.
- First reported: 2026-05-04 (BT16-027 batch-implement-cards-rust-dsl).

---

## BT12-028 — `trash_top_n_digivolution_cards` step + engine primitive  [G-DSL-TRASH-TOP-N-DIGI-CARDS]

- Effect text: "Trash the top 3 digivolution cards of all of your opponent's Digimon." (BT12-028 clause 0a).
- Card first discovered in: BT12-028 Paildramon. Sibling to G-ASL-07 (BT17-077 all-source mass trash).
- Missing DSL verb / step kind / predicate: `trash_top_n_digivolution_cards: { target: <handle>, count: N }` — removes N source cards from BELOW the top card of a permanent (keeping the Digimon on the field), firing `OnDigivolutionCardTrashed` per source. Distinct from `de_digivolve` (removes top card → demotes Digimon) and `trash_top_source` (removes the active Digimon card itself).
- Lowers to engine API: `EffectContext::trash_top_n_digivolution_sources(target, count)` — NEW primitive needed; engine substrate currently lacks a "remove sources from below the top" operation.
- Suggested DSL syntax: `trash_top_n_digivolution_cards: { target: opp_digimon, count: 3 }`.
- Gap kind: hybrid (DSL verb + engine primitive both missing).
- Workaround: clause omitted from runtime; structural tests pass.
- First reported: 2026-05-04 (BT12-028 batch-implement-cards-rust-dsl).

---

## BT16-025 — `binding_is_none` / "if-no-target" predicate  [G-DSL-IF-NO-TARGET]

- Effect text: "Suspend 1 of your opponent's unsuspended Digimon. If this effect didn't suspend, unsuspend this Digimon." (BT16-025 clause 2).
- Card first discovered in: BT16-025 Paildramon.
- Missing DSL verb / step kind / predicate: `select_opponent_permanent` with `optional: true` skips silently when no targets exist, but does not bind a "skipped" flag. Need `binding_is_none: <name>` BoolPredicate for subsequent `if` conditions to test whether the previous selection was taken or skipped.
- Lowers to engine API: existing binding mechanism — only the BoolPredicate leaf is missing.
- Suggested DSL syntax:
  ```yaml
  - if:
      condition: { binding_is_none: tgt }
      then: [ unsuspend: { target: source } ]
  ```
- Gap kind: dsl.
- Workaround: conditional unsuspend-self omitted from runtime; behavioral test `#[ignore]`'d.
- First reported: 2026-05-04 (BT16-025 batch-implement-cards-rust-dsl).
- Also blocks: BT16-028 clause 0b — "[When Digivolving] by suspending 1 of their Digimon or Tamers, unsuspend 1 of your Digimon." Same structural gap: the optional suspend-cost step produces no binding result flag, so the own-unsuspend reward arm cannot be made conditional on the cost being paid. Cross-listed 2026-05-04.

---

## BT16-028 — `event_is_effect_initiated` predicate  [G-IS-EFFECT-INITIATED]

- Effect text: "[All Turns] When an effect plays or digivolves an opponent's Digimon, if you have a Tamer, this Digimon may digivolve into [Imperialdramon: Fighter Mode] in the hand without paying the cost."
- Card first discovered in: BT16-028 Imperialdramon: Dragon Mode (2026-05-04).
- Missing DSL verb / step kind / predicate: `event_is_effect_initiated: bool` — a BoolPredicate leaf that evaluates to `true` only when the entering/digivolving permanent was placed by a card effect (DCGO's `IsByEffect`) rather than by a player's normal main-phase play action. Without this gate, the `on_enter_field_anyone` / `on_digivolve` observer timings fire on ALL enters/digivolves regardless of cause, over-firing for normal player plays. This violates the printed "when an EFFECT plays or digivolves" condition.
- Companion engine gap: the engine must thread an `is_effect_initiated` flag through `TriggerContext` (or the event payload) when dispatching `OnEnterFieldAnyone` and `OnDigivolve`. Normal `Game::play_from_hand` and `Game::digivolve_from_hand` would emit `is_effect_initiated: false`; effect-driven entry/digivolve paths (e.g. `EffectContext::effect_play_card`, `EffectContext::effect_initiated_digivolve`) would emit `is_effect_initiated: true`. The DCGO `IsByEffect` check uses the `hashtable` context carried into each coroutine, which records the originating cause.
- Lowers to engine API: `TriggerContext` (or `EventPayload`) would need a new field `cause_is_effect: bool`. The DSL predicate would evaluate `rctx.game.current_trigger_context.map(|ctx| ctx.cause_is_effect).unwrap_or(false)`.
- Suggested DSL syntax:
  ```yaml
  - when: [on_enter_field_anyone, on_digivolve]
    optional: true
    active_when: { all_turns: true }
    condition:
      all_of:
        - event_target_owner: opponent
        - event_target_kind: digimon
        - event_is_effect_initiated: true    # ← new predicate leaf
        - any_permanent:
            of: you
            zone: [battle_area]
            kind: tamer
    process:
      - select_hand:
          of: you
          bind_as: fighter
          filter:
            all_of:
              - kind: digimon
              - name_contains: "Imperialdramon: Fighter Mode"
          prompt: "Digivolve into Imperialdramon: Fighter Mode (free, ignore reqs)"
      - effect_initiated_digivolve:
          target: self
          from_hand: fighter
          cost: 0
          ignore_requirements: true
  ```
- Gap kind: hybrid (engine must thread the cause flag through TriggerContext; DSL then needs the predicate leaf).
- Workaround: Clause omitted from BT16-028.yaml while this gap is open. Authoring it without the `event_is_effect_initiated` gate would fire on all plays/digivolves (violates printed text and no-approximations policy).
- Behavioral tests: 4 tests `#[ignore = "pending: G-IS-EFFECT-INITIATED from qa/dsl-vocab-gaps.md"]`.
- First reported: 2026-05-04 (BT16-028 batch-implement-cards-rust-dsl).

---

## BT12-031 — Alt-cost: return named source card from own digi-stack to hand  [G-DSL-COST-RETURN-SELF-DIGI-CARD-BY-NAME]
- Effect text (BT12-031 Clause 0, Step C): "By returning 1 [Imperialdramon: Dragon Mode] from this Digimon's digivolution cards to its owner's hand, return all of your opponent's suspended Digimon to the bottom of their owners' decks instead."
- Missing DSL verb / step kind / predicate: Two sub-gaps combine to block this step:
  1. **G-DSL-SELECT-OWN-SOURCES-FILTER** (see EX4-073 entry) — `select_own_sources` installs with `|_game, _source| true` hardcoded. There is no `filter:` key to restrict the selection to sources matching a specific card name.
  2. **G-DSL-BIND-PRESENT** (see EX9-066 entry) — After the optional selection, the alternative outcome must be conditioned on whether the player made a selection or passed. The `binding_present` predicate does not exist.
- Synthesizing gap ID: `G-DSL-COST-RETURN-SELF-DIGI-CARD-BY-NAME` — filing as a composite gap for the BT12-031 context.
- DCGO reference: `BT12_031.cs` — step C via optional `AddSelectCard` from own digi-cards filtered by `EqualsCardName("Imperialdramon: Dragon Mode")`, `canNoSelect: () => true`. If selected, card returns to hand and all suspended opp Digimon return to bottom of deck; if declined, only the single return-to-hand fires.
- Suggested DSL syntax:
  ```yaml
  - select_own_sources:
      bind_as: dragon_mode_src
      optional: true
      filter:
        name_is: "Imperialdramon: Dragon Mode"
      prompt: "Return [Imperialdramon: Dragon Mode] from your digivolution cards to hand to return ALL opponent suspended Digimon to bottom of decks instead"
  - if:
      condition:
        binding_present: dragon_mode_src
      then:
        - return_to_hand: { target: dragon_mode_src }
        - for_each:
            over:
              all_of:
                - of: opponent
                - zone: [battle_area]
                - kind: digimon
                - is_suspended: true
            bind_as: susp_opp
            body:
              - return_to_deck:
                  target: susp_opp
                  position: bottom
                  include_sources: false
      else:
        - select_opponent_permanent:
            bind_as: suspended_target
            filter:
              all_of:
                - kind: digimon
                - is_suspended: true
            prompt: "Return 1 of your opponent's suspended Digimon to its owner's hand"
        - return_to_hand: { target: suspended_target }
  ```
- Lowers to engine API: both sub-gaps are DSL-only additions; engine already stores the data.
  - `select_own_sources` filter: add `filter: Option<CompiledPredicate>` to `CompiledSelectOwnSources`, evaluate against each `card_source` entry.
  - `binding_present` predicate: add leaf that checks `ctx.bindings.get(name).is_some()`.
- Gap kind: DSL only.
- Workaround: Steps A (for_each suspend no-digi-card targets) and B (select 1 suspended opp → return to hand) are authored in BT12-031.yaml. Step C is commented out as BLOCKED.
- Behavioral tests: 2 tests `#[ignore = "pending: G-DSL-COST-RETURN-SELF-DIGI-CARD-BY-NAME from qa/dsl-vocab-gaps.md ..."]` in `code/digimon-engine/tests/cards_behavioral/bt12/bt12_031.rs`.
- First reported: 2026-05-04 (BT12-031 TDD implementation).

---

## BT17-077 — `return_all_trash_to_deck_bottom` step + player-choice target  [G-RETURN-ALL-TRASH-TO-DECK-BOTTOM]

- Effect text: "Then, return all cards from your or your opponent's trash to the bottom of the deck." (BT17-077 Clause 1b).
- Card first discovered in: BT17-077 Imperialdramon: Paladin Mode.
- Missing DSL verb / step kind / predicate: `return_all_trash_to_deck_bottom: { of: <player_ref> }` — moves every card currently in the specified player's trash zone to the bottom of their deck. Requires: (a) a zone-bulk-move primitive in the engine, (b) a DSL step that accepts `of: you` or `of: opponent`, and (c) owner routing (each card returns to its own deck bottom, not the controller's).
- Lowers to engine API: `EffectContext::return_all_trash_to_deck_bottom(player)` — NEW primitive; the engine currently has no "drain entire zone" bulk-move.
- Companion gap: the printed text says "your or your opponent's trash" — the choice of whose trash is returned is a player decision (DCGO: `BoolSelection`). This requires either `select_effect_choice` (choose 0 or 1) + `if` conditional wiring the correct `of:` player, or a single parametric verb `return_all_trash_to_deck_bottom: { of: chosen_player }` where `chosen_player` is a binding. Neither is currently in the DSL.
- Suggested DSL syntax:
  ```yaml
  - select_effect_choice:
      bind_as: whose_trash
      labels: ["Your Trash", "Opponent's Trash"]
      prompt: "Return all cards from your or your opponent's trash to the bottom of the deck"
  - if:
      condition: { equals: [whose_trash, 0] }
      then:
        - return_all_trash_to_deck_bottom: { of: you }    # ← new verb
      else:
        - return_all_trash_to_deck_bottom: { of: opponent }  # ← new verb
  ```
- Gap kind: hybrid (engine bulk-move primitive + DSL verb + owner-routing semantics all missing).
- Workaround: Clause 1b (and the dependent Clause 1c memory rider) are omitted from BT17-077.yaml pending G-ASL-07 closure. Behavioral tests #[ignore]'d.
- Cross-ref: G-ASL-07 (qa/archetype-qa/dsl/alter-s-ladder-cross-archetype-gaps-2026-05-03.md) tracks the broader mass-cleanup-and-trash-return family. G-DSL-TRASH-TOP-N-DIGI-CARDS (above) blocks Clause 1a (the mass-source-trash step that precedes this).
- First reported: 2026-05-04 (BT17-077 batch-implement-cards-rust-dsl).

---

## BT17-077 — `any_returned_card` result predicate  [G-ANY-RETURNED-CARD-PREDICATE]

- Effect text: "If this effect returned a white level 7 card, gain 3 memory." (BT17-077 Clause 1c).
- Card first discovered in: BT17-077 Imperialdramon: Paladin Mode. Clause 1c fires after the `return_all_trash_to_deck_bottom` step (Clause 1b) completes; the memory gain is conditional on at least one of the moved cards satisfying `color: white AND level: 7`.
- Missing DSL verb / step kind / predicate: `any_returned_card: { color_is: white, level_eq: 7 }` — a BoolPredicate that evaluates to true if the immediately preceding zone-move step returned at least one card matching the given filter. There is no "result-set predicate" that can inspect the set of cards moved by a prior step.
- Lowers to engine API: the step would need to bind a `Vec<CardData>` of moved cards as an effect-local result, which the subsequent `if` condition can test via `any_returned_card` iterating over that result set.
- Suggested DSL syntax:
  ```yaml
  - return_all_trash_to_deck_bottom:
      of: opponent
      bind_returned_as: returned_cards    # optional result binding
  - if:
      condition:
        any_returned_card:                # new BoolPredicate leaf
          binding: returned_cards
          color_is: white
          level_eq: 7
      then:
        - gain_memory: 3
  ```
- Gap kind: dsl (engine result-binding infrastructure would also need extending for the `bind_returned_as` step argument).
- Workaround: Clause 1c is omitted from BT17-077.yaml; behavioral test #[ignore]'d.
- Cross-ref: G-RETURN-ALL-TRASH-TO-DECK-BOTTOM (above) must close first (Clause 1b provides the moved-card set that Clause 1c predicates on).
- First reported: 2026-05-04 (BT17-077 batch-implement-cards-rust-dsl).
---

## Royal Knights — Delay/keyword leave-prevention replacements  [RK-G003]

- Effect text: `BT20-100` The Last Guardian: "[All Turns] When any of your Digimon with [Omnimon] in its name would leave the battle area, <Delay> ... 1 of those Digimon doesn't leave." `BT23-054` Magnamon: "<Armor Purge> (When this Digimon would be deleted, you may trash the top card of this Digimon to prevent that deletion.)"
- Missing DSL verb / step kind / predicate: reusable Delay replacement and Armor Purge keyword lowering that pay the printed cost, bind the replacement subject, and cancel the pending would-leave/deletion event without approximating the subject or cause.
- Companion engine state: generic replacement lowering exists for some cancel flows, but Royal Knights needs option-as-Delay source costs and keyword-provided top-card trash costs that are not yet represented as reusable declarative keyword/replacement emitters.
- Suggested DSL syntax:
  ```yaml
  - kind: replacement
    trigger: when_would_leave_battle_area
    source_is_delay_option: true
    active_when:
      all_of:
        - replacement_subject_is_mine: true
        - name_contains: "Omnimon"
    cost:
      trash_source_delay_option: {}
    process:
      - cancel_replacement: {}

  - kind: grant_keyword
    keyword: ArmorPurge
  ```
- Gap kind: hybrid. The engine replacement framework handles cancellation, but DSL/keyword lowering needs reusable costed replacement emitters and subject filters for these printed shapes.
- Workaround: None faithful. Auto-cancelling without the cost or omitting the replacement changes a player-visible prevention choice.
- First reported: 2026-05-05 (Royal Knights Batch 2: BT20-100, BT23-054).

---

## Royal Knights — would-leave observer that plays from hand without cancelling  [RK-G004]

- Effect text: `BT20-091` Cool Boy: "[Opponent's Turn] [Once Per Turn] When any of your Digimon with the [Royal Knight] trait would leave the battle area, you may play 1 [Omekamon] from your hand without paying the cost."
- Missing DSL verb / step kind / predicate: a would-leave observer that sees the replacement subject and can run an optional hand play while allowing the original leave event to proceed.
- Companion engine state: `kind: replacement` can observe would-leave events, but current authoring patterns are cancellation/prevention-centric. This card needs a non-cancelling reaction at the same timing with event subject filters, OPT accounting, and ordinary pending hand selection/play.
- Suggested DSL syntax:
  ```yaml
  - when: when_would_leave_battle_area
    active_when:
      all_of:
        - opponents_turn: true
        - replacement_subject_is_mine: true
        - trait_has: "Royal Knight"
    optional: true
    once_per_turn: true
    process:
      - select_hand:
          bind_as: omekamon
          filter: { name_is: "Omekamon" }
      - play_from_hand_free: { of: you, hand_index: omekamon }
  ```
- Gap kind: hybrid. Engine timing/context exists through replacement context, but DSL needs a first-class non-cancelling would-leave observer or a documented replacement form whose default outcome proceeds safely.
- Workaround: None faithful. A cancellation replacement would prevent the leaving Digimon incorrectly; omitting the clause hides a legal optional response.
- First reported: 2026-05-05 (Royal Knights Batch 3: BT20-091).

---

## Royal Knights — attack target retarget response  [G-ATTACK-RETARGET]

- Effect text: `BT19-072` LordKnightmon: "[Opponent's Turn] [Once Per Turn] When an opponent's Digimon attacks, you may switch the attack target to 1 of your Digimon with the [Royal Knight] trait."
- Missing DSL verb / step kind / predicate: attack-state pending selection that can replace the current defender/security target with a selected own permanent matching a filter.
- Companion engine state: attack declaration and blocker/Raid-like retargeting are action-state concerns; a normal triggered effect after attack declaration cannot faithfully mutate the target without a dedicated interrupt point.
- Suggested DSL syntax:
  ```yaml
  - when: opponent_attacks
    optional: true
    once_per_turn: true
    process:
      - select_own_permanent:
          bind_as: new_target
          filter: { kind: digimon, trait_has: "Royal Knight" }
      - switch_attack_target: { target: new_target }
  ```
- Gap kind: engine and DSL. The engine needs an attack-retarget pending state; DSL needs vocabulary to request it.
- Workaround: None faithful. Preselecting targets at attack declaration or auto-retargeting hides the printed optional timing.
- First reported: 2026-05-05 (Royal Knights Batch 3: BT19-072).
