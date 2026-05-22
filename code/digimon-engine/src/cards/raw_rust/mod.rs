//! Production raw_rust function registrations for DSL long-tail cards.
//!
//! Phase 4 keeps bespoke mechanics behind named functions here instead of
//! handwritten card modules under `src/cards/<set>/`.

use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::raw_rust::EngineRawRustRegistry;
use crate::effect::Effect;
use crate::effect_context::EffectContext;

/// BT24-012 Dimetromon — [All Turns] "protect other Reptile/Dragonkin ally by bouncing self"
/// replacement — no-op placeholder.
///
/// Printed clause (b):
/// "[All Turns] When any of your OTHER Digimon with the [Reptile] or [Dragonkin] trait
/// would leave the battle area by your opponent's effects, by returning this Digimon to
/// your hand, they don't leave."
///
/// This is a cross-permanent replacement effect: the carrier (Dimetromon) intercepts a
/// *different* permanent leaving and cancels that departure by paying a cost (return self
/// to hand). The standard `kind: replacement` + `cancel_replacement` DSL path is blocked
/// by the `subject_matches` guard in `lower_replacement.rs` (line 83–91), which only fires
/// when the carrier IS the leaving subject.
///
/// The full implementation requires:
///
/// **Gap G-EVENT-TARGET-OWNER** — no predicate in `ReplacementContext` gates on whether
/// the leaving permanent is controlled by the same player as the carrier. Additionally,
/// removal-cause attribution ("by your opponent's effects") is not threaded into the
/// replacement context — the engine would need `ReplacementContext::caused_by_opponent`
/// populated from game-action callsites. Until this is wired, any implementation would
/// over-fire (fires for own-effect removal too), violating the no-approximations policy.
///
/// **subject_matches architecture gap** — `lower_replacement.rs` enforces that replacement
/// effects only fire when `rctx.effect.source_permanent == Some(subject_h)`. Lifting this
/// restriction to allow "protect others" patterns requires a targeted change to
/// `lower_replacement.rs`.
///
/// Until both gaps are closed this function returns an empty `Vec<Effect>`, preserving
/// no-op behavior while the YAML clause documents the intent.
///
/// When implemented, the fn must:
///   1. Build a `WhenWouldLeaveBattleArea` replacement effect scoped to the carrier.
///   2. In the replacement predicate: check subject != carrier, subject.controller == carrier.controller,
///      subject has Reptile or Dragonkin trait, and carrier is on the battle area.
///   3. Present optional prompt ("Accept/Decline"); on accept: return carrier to hand via
///      `ctx.return_to_hand(carrier)` and set outcome to Cancelled.
///
/// Tracked under G-EVENT-TARGET-OWNER in `qa/archetype-qa/engine-gaps.md`.
fn bt24_012_would_leave_replacement(_handle: crate::card_source::CardHandle) -> Vec<Effect> {
    // No-op: returns an empty effect list.
    // Full implementation blocked by G-EVENT-TARGET-OWNER (removal cause attribution
    // + cross-permanent replacement) and the subject_matches gate in lower_replacement.rs.
    vec![]
}

/// LM-027 Red Scramble — legacy shim for "add this card to hand".
///
/// The printed Security effect ends with "Then, add this card to the hand."
/// DCGO implements this via `CardEffectCommons.AddThisCardToHand(card, activateClass)`
/// which moves the currently-resolving option card from security-resolution
/// staging back to the controller's hand.
///
/// Prefer the native DSL `add_this_option_to_hand: {}` step for new scripts.
fn lm_027_add_self_to_hand(ctx: &mut EffectContext<'_>, _bindings: &mut Bindings) {
    ctx.add_pending_security_to_hand();
}

/// BT21-093 Raging Serpentine — Main + Security clause: delete 1 of opponent's
/// highest-DP Digimon (mandatory if any eligible target).
///
/// No-op stub: the highest-DP-aggregate selection requires either a new DSL
/// `aggregate: highest_dp` evaluator (G-PRED-DP-LTE family) or a manually
/// installed `select_opponent_permanent` selection from raw Rust. Behavioral
/// tests for this card are structural-only pending that work.
fn bt21_093_delete_highest_dp_opponent(_ctx: &mut EffectContext<'_>, _bindings: &mut Bindings) {
    // No-op: pending G-PRED-DP-LTE (highest-DP aggregate predicate) +
    // a raw_rust-driven selection installer.
}

/// EX8-070 Zofr Kabus — [Security] Delete 1 of opponent's Digimon with the
/// lowest play cost.
///
/// Printed text: "[Security] Delete 1 of your opponent's Digimon with the
/// lowest play cost."
///
/// GAP G-PLAY-COST-AGGREGATE: No DSL predicate for selecting permanents by
/// aggregate (minimum) play cost. This step computes the minimum play cost
/// among opponent Digimon and deletes the first one at that cost. When
/// multiple Digimon share the lowest cost, the printed "1 of" implies a
/// player choice (tie-breaking selection), but that requires a two-pass
/// raw_rust pending G-PLAY-COST-AGGREGATE closure. For now the lowest-index
/// tied target is auto-deleted as a simplification.
///
/// When G-PLAY-COST-AGGREGATE closes, replace with a native DSL expression
/// that uses `select_opponent_permanent` filtered by `play_cost_lte: { agg: min }`.
///
/// Tracked in qa/dsl-vocab-gaps.md under G-PLAY-COST-AGGREGATE.
fn ex8_070_delete_lowest_cost_digimon(ctx: &mut EffectContext<'_>, _bindings: &mut Bindings) {
    use crate::enums::CardKind;
    use crate::permanent::PermanentHandle;

    let opponent = ctx.opponent_id();

    // Collect (handle, play_cost) for every opponent Digimon.
    let digimon_costs: Vec<(PermanentHandle, u16)> = ctx
        .game
        .player(opponent)
        .battle_area
        .iter()
        .enumerate()
        .filter_map(|(idx, perm)| {
            let top = perm.top_card();
            let data = &ctx.game.card_data[top.data_index];
            if data.card_kind == CardKind::Digimon {
                let handle = PermanentHandle {
                    player: opponent,
                    index: idx as u8,
                };
                Some((handle, data.play_cost))
            } else {
                None
            }
        })
        .collect();

    // Nothing to do if opponent has no Digimon.
    if digimon_costs.is_empty() {
        return;
    }

    // Find the minimum play cost.
    let min_cost = digimon_costs
        .iter()
        .map(|(_, cost)| *cost)
        .min()
        .unwrap_or(0);

    // Delete the first Digimon at the minimum cost (lowest battle-area index).
    // Tie-breaking selection pending G-PLAY-COST-AGGREGATE.
    if let Some((handle, _)) = digimon_costs.into_iter().find(|(_, cost)| *cost == min_cost) {
        ctx.delete_permanent(handle);
    }
}

/// BT24-062 MasterBlimpmon — inherited [Your Turn] target-lock declarative.
///
/// Printed inherited text: "[Your Turn] This Digimon's attack target can't change."
///
/// Refreshes `ModifierType::CannotSwitchAttackTarget` on the host permanent
/// (the active top card of the digivolution stack containing this card source)
/// every declarative tick when the host's controller is the turn player. The
/// modifier is auto-cleared between ticks because `add_declarative_modifier`
/// sets `materialized_declarative: true`, and Track D's combat consult sites
/// (Block window early-return, Raid retarget early-return, and the unified
/// `apply_attack_target_substitution` no-op) read the modifier directly.
///
/// Two declaratives are emitted to cover both scopes the Rust engine's tick
/// model exposes — `face_up` (BT24-062 is the active top card) and
/// `inherited` (BT24-062 is a digivolution source under another Digimon).
/// In both cases `source_permanent` resolves to the host, so the body is
/// identical; only the `.inherited()` flag differs. Without the face-up
/// emission the modifier would not install when BT24-062 IS the host —
/// the tick walks the top card with `inherited_source = false` and skips
/// effects whose `inherited` flag is set.
fn bt24_062_attack_target_lock(card: crate::card_source::CardHandle) -> Vec<Effect> {
    use crate::enums::{Expiry, ModifierType};

    fn install(ctx: &mut EffectContext<'_>) {
        let Some(host) = ctx.source_permanent else {
            return;
        };
        if ctx.game.turn_player() != host.player {
            return;
        }
        ctx.add_declarative_modifier(
            host,
            ModifierType::CannotSwitchAttackTarget,
            0,
            Expiry::Permanent,
        );
    }

    vec![
        Effect::declarative(card)
            .name("[Your Turn] target lock — face up")
            .materializes_declarative_state()
            .process(install)
            .build(),
        Effect::declarative(card)
            .name("[Your Turn] target lock — inherited source")
            .inherited()
            .materializes_declarative_state()
            .process(install)
            .build(),
    ]
}

/// BT13-040 Magnamon — would-leave observer tail.
///
/// Printed text: "When this Digimon would leave the battle area, <Draw 1>.
/// Then, you may play 1 [Veemon] from your hand or this Digimon's digivolution
/// cards without paying the cost."
///
/// This is intentionally a non-cancelling replacement-process step: the
/// surrounding DSL replacement observer performs the draw, then this helper
/// installs one optional pending selection containing both hand and material
/// action IDs. No action-space expansion is needed; hand picks reuse
/// `PLAY_HAND_START + index`, source picks reuse `SOURCE_SELECT_START +
/// field * SOURCES_PER_FIELD + source_index`.
fn bt13_040_may_play_veemon_from_hand_or_source(
    ctx: &mut EffectContext<'_>,
    _bindings: &mut Bindings,
) {
    use crate::action::space::{
        encode_source_select, PLAY_HAND_END, PLAY_HAND_START, SOURCE_SELECT_END,
        SOURCE_SELECT_START,
    };
    use crate::enums::{CostDelta, GamePhase};
    use crate::selection::{PendingSelection, SelectionKind, UnionZoneSet};

    let Some(source_permanent) = ctx.source_permanent else {
        return;
    };

    let player = ctx.player;
    let mut valid_action_ids = Vec::new();

    for (idx, card) in ctx.game.player(player).hand.iter().enumerate() {
        if idx >= crate::action::space::HAND_MAIN_LIMIT {
            break;
        }
        if card
            .card_names(&ctx.game.card_data)
            .iter()
            .any(|name| name.contains("Veemon"))
        {
            valid_action_ids.push(PLAY_HAND_START + idx as u16);
        }
    }

    if let Some(perm) = ctx
        .game
        .player(source_permanent.player)
        .battle_area
        .get(source_permanent.index as usize)
    {
        let source_count = perm.card_sources.len().saturating_sub(1);
        for source_index in 0..source_count.min(crate::action::space::SOURCES_PER_FIELD as usize) {
            let source = &perm.card_sources[source_index];
            if source
                .card_names(&ctx.game.card_data)
                .iter()
                .any(|name| name.contains("Veemon"))
            {
                if let Some(action_id) =
                    encode_source_select(source_permanent.index as u16, source_index as u16)
                {
                    valid_action_ids.push(action_id);
                }
            }
        }
    }

    if valid_action_ids.is_empty() {
        return;
    }

    let previous_phase = ctx.game.current_phase;
    let selecting_player = ctx.override_selecting_player.unwrap_or(player);
    let source_card = ctx.source_card;
    let source_kind = ctx.source_kind;
    let override_pin = ctx.override_selecting_player;
    ctx.game.current_phase = GamePhase::SelectUnion;
    ctx.game.pending_selection = Some(PendingSelection {
        kind: SelectionKind::UnionZone {
            zones: UnionZoneSet::HAND | UnionZoneSet::MATERIAL,
        },
        selecting_player,
        previous_phase,
        valid_action_ids,
        is_optional: true,
        prompt: "You may play 1 [Veemon] from hand or this Digimon's sources".to_string(),
        effect_choices: None,
        source_card,
        source_permanent: Some(source_permanent),
        source_kind,
        callback: Box::new(move |game, action_id| {
            let mut cb_ctx = EffectContext::new_with_source_kind_and_override(
                game,
                source_card,
                Some(source_permanent),
                source_kind,
                player,
                override_pin,
            );
            if (PLAY_HAND_START..PLAY_HAND_END).contains(&action_id) {
                let hand_index = (action_id - PLAY_HAND_START) as usize;
                let _ = cb_ctx.play_from_hand_free(player, hand_index);
            } else if (SOURCE_SELECT_START..SOURCE_SELECT_END).contains(&action_id) {
                let (_, source_index) = crate::action::space::decode_source_select(action_id);
                let _ = cb_ctx.play_from_materials(
                    source_permanent,
                    source_index as usize,
                    CostDelta::Free,
                );
            }
        }),
        on_decline: None,
    });
}

pub fn build_registry() -> EngineRawRustRegistry {
    let mut r = EngineRawRustRegistry::new();
    // EX11-012, P-137, EX8-074, BT23-014, BT9-112, BT17-018 and LM-021 were
    // migrated off raw_rust to pure DSL — their bespoke functions were removed.
    r.register_declarative(
        "bt24_012_would_leave_replacement",
        bt24_012_would_leave_replacement,
    );
    r.register_step("lm_027_add_self_to_hand", lm_027_add_self_to_hand);
    r.register_step(
        "bt21_093_delete_highest_dp_opponent",
        bt21_093_delete_highest_dp_opponent,
    );
    r.register_step(
        "ex8_070_delete_lowest_cost_digimon",
        ex8_070_delete_lowest_cost_digimon,
    );
    r.register_declarative("bt24_062_attack_target_lock", bt24_062_attack_target_lock);
    r.register_step(
        "bt13_040_may_play_veemon_from_hand_or_source",
        bt13_040_may_play_veemon_from_hand_or_source,
    );
    r
}

pub fn raw_rust_budget_status(raw_fn_count: usize, dsl_card_count: usize) -> Result<(), String> {
    if dsl_card_count == 0 {
        return Ok(());
    }

    let pct = (raw_fn_count as f64 / dsl_card_count as f64) * 100.0;
    if pct > 3.0 {
        Err(format!(
            "raw_rust budget exceeded: {raw_fn_count} raw_rust fns for \
             {dsl_card_count} DSL cards ({pct:.1}%)"
        ))
    } else {
        Ok(())
    }
}
