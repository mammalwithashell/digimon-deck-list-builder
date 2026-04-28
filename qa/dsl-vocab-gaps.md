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
- Missing DSL verb / step kind / predicate: `[When Moving]` (DCGO `EffectTiming.OnMove`) has no DSL `when:` token. The closest existing token `on_hatch` fires for permanents already on field when an egg moves digitama→breeding — different timing.
- Lowers to engine API: needs new `EffectTiming::OnMove` variant in Rust (engine gap — see `qa/archetype-qa/engine-gaps.md`) AND a DSL token mapping to it.
- Suggested DSL syntax: `when: [on_move, on_play]` (new `on_move` token in the DSL `when` enum).
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
- Lowers to engine API: requires new `entering_permanent: Option<PermanentHandle>` field in `TriggerContext`, populated by `game_actions.rs::broadcast_on_enter_field_anyone` (and the digivolve equivalent) with the newly-entered permanent's handle. A DSL predicate would then read this field via `ctx.trigger_context.entering_permanent.map(|h| ctx.game.permanent_traits(h).contains(trait))`.
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
- Workaround: `trait_has: LIBERATOR` filter expressed in YAML (documents intent); cost-≤4 constraint silently not enforced at selection time. Tests for incorrect candidate filtering are `#[ignore = "pending: G-PLAY-COST-LTE"]`.
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
- Missing DSL verb / step kind / predicate: No DSL step verb installs a **player-level** modifier. `kind: flood_gate` lowers via `lower_flood_gate.rs` which iterates the battle area and calls `ctx.add_modifier(h, modifier, 0, Expiry::Permanent)` — permanent-level modifiers only. `EffectContext::add_modifier` takes a `PermanentHandle`, not a `PlayerId`. The engine's enforcement path for digivolve-cost reduction suppression uses `modifiers.player_has(acting_player, ModifierType::CannotReducePlayCost)` (player-level registry, separate from permanent modifiers). There is no DSL verb that calls `ctx.game.modifiers.add_player_modifier(player_id, ...)`.
- Additional engine gap: `scan_before_pay_cost_reduction` in `game_actions.rs` checks only `CannotReducePlayCost`, which covers ALL cost types (play + digivolve). A per-cost-type split (`CannotReduceDigivolveCost` vs `CannotReducePlayCost`) does not exist. The `CannotReduceCost` modifier type (enums.rs line 389) exists but is NEVER checked anywhere.
- Lowers to engine API: `modifiers.add_player_modifier(player_id, PlayerModifierEntry)` exists in `modifiers.rs` but is not exposed via `EffectContext`. Raw_rust functions can call `ctx.game.modifiers.add_player_modifier(...)` directly (field is public).
- Suggested DSL syntax:
  ```yaml
  - kind: flood_gate
    active_when: { opponents_turn: true }
    target: { player: opponent }     # NEW: player-level target
    modifier: CannotReduceDigivolveCost  # NEW: per-cost-type variant
  ```
  Requires: (1) add `player: Option<PlayerRef>` to `FloodGateBody` as alternative to the `target: PredicateSpec` field; (2) add `CannotReduceDigivolveCost` to `ModifierType` enum + validator allowlist; (3) install via `ctx.game.modifiers.add_player_modifier` in `lower_flood_gate.rs` when `player:` is set; (4) add enforcement branch in `scan_before_pay_cost_reduction` for the digivolve path.
- Gap kind: hybrid (DSL lacks player-targeted flood_gate; engine lacks `CannotReduceDigivolveCost` enforcement).
- Workaround: `kind: raw_rust` declarative no-op placeholder (`bt5_008_opp_cannot_reduce_digivolve_cost`). Test `#[ignore]`'d with `G-PLAYER-FLOOD-GATE-DSL` tag.
- First reported: 2026-04-27 (BT5-008 batch-implement-cards-rust-dsl, Medusamon archetype)

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
- Workaround: None — `dp_lte` predicate is also not evaluated (G-PRED-DP-LTE). Both gaps must close together for this clause to work.
- First reported: 2026-04-27 (ST22-08 batch-implement-cards-rust-dsl, Medusamon Batch 11)

---

## ST22-08 — Return Played Option Card to Hand Post-Security  [G-ADD-OPTION-SELF-TO-HAND]
- Effect text: "[Security] Delete 1 of your opponent's Digimon with the lowest DP. Then, add this card to the hand."
- Missing DSL verb / step kind / predicate: A step verb for "add this played Option card back to the controller's hand after security resolution." This is different from `add_top_security_to_hand` (which moves the TOP of the security stack to hand) and different from `return_to_hand` (which moves a battle-area permanent to hand). The played Option is in the "being resolved" state during security — it has not yet been trashed. The DCGO calls `CardEffectCommons.AddThisCardToHand(card, activateClass)`. In the Python engine, this was implemented as `CardEffectCommons.add_this_card_to_hand` after the security-effect fires. EX6-072 Mega Digimon Assembly uses `raw_rust: { fn: ex6_072_add_self_to_hand }` for the same pattern.
- Lowers to engine API: `ctx.add_security_option_to_hand()` or a method that retrieves the current card being resolved in the security context and places it in the controller's hand instead of trashing it. The exact engine mechanism depends on how `security_attack()` manages the Option card — checking `_security_played` flag vs. moving it to hand.
- Suggested DSL syntax:
  ```yaml
  - return_self_to_hand: {}   # Or: add_this_card_to_hand: {}
  ```
  This would lower to a method that finds the source card's handle and transfers it from the security-resolution staging to the controller's hand.
- Gap kind: dsl (the engine already has a pattern for this — ex6_072_add_self_to_hand raw_rust — but there is no DSL step verb).
- Workaround: `raw_rust: { fn: st22_08_add_self_to_hand }` (same pattern as `ex6_072_add_self_to_hand`).
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

- Effect text: "[On Play][When Digivolving] ... choose 1 of both players' Digimon and delete all other Digimon."
- Missing DSL verb / step kind / predicate: `not_in_binding` — a `CandidatePredicate` leaf in `for_each { over, filter, body }` that excludes permanents whose handle appears in a named binding (a prior selection). Without it, "delete all OTHER Digimon" (all except the two saved by selection) cannot be expressed purely in DSL.
- Engine API: the engine can iterate `battle_area` handles and compare against a collected `Vec<PermanentHandle>`. No new API needed — gap is purely in the DSL filter vocabulary.
- Suggested DSL syntax:
  ```yaml
  - for_each:
      over: { of: any, kind: digimon }
      bind_as: candidate
      filter:
        not_in_binding: saved   # CandidatePredicate: exclude if handle is in binding "saved"
      body:
        - delete_permanent: { target: candidate }
  ```
  Implementation: add `not_in_binding: Option<String>` to `CandidatePredicateSpec` in `digimon-dsl/src/predicate.rs`, compile, and evaluate by looking up the named binding in `Bindings` and comparing handle equality.
- Gap kind: dsl (engine can express this in a raw_rust loop; DSL has no filter for handle-set exclusion).
- Workaround: entire boardwipe clause routed through `raw_rust: { fn: bt20_102_boardwipe_and_return }` which collects `saved: Vec<PermanentHandle>` and filters deletions via `.contains()`.
- First reported: 2026-04-27 (BT20-102 batch-implement-cards-rust-dsl, Medusamon Batch 11)
