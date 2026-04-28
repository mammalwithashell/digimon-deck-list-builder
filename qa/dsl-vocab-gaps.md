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
