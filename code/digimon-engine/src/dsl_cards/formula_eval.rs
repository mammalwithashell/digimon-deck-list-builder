//! Phase 2f2 Task 2 — runtime evaluator for `CompiledFormula`.
//!
//! Pure read-only against `EffectContext` + the target permanent handle.
//! Per-selectors that reference the target's stack (`stack_size`,
//! `material_count`, `digivolution_color_count`) resolve against `target`.
//! Aggregate selectors operate on the controller's battle area
//! (`ctx.player.battle_area`).
//!
//! Defensive convention: degenerate inputs (mis-arity FloorDiv, divide
//! by zero, missing target permanent, unregistered RawRust fn) return 0
//! rather than panicking. The validator (Phase 2 schema check) shouldn't
//! produce these but the engine must not crash on a malformed pack.
//!
//! The two card-author-bug branches (mis-arity FloorDiv, unknown
//! RawRust) emit `#[cfg(debug_assertions)] eprintln!` warnings so a
//! malformed pack surfaces in dev/test logs instead of producing
//! subtly wrong DP values silently. Logging is observability only —
//! the return value contract (always 0) is unchanged. The remaining
//! defensive branches (divide-by-zero, missing target permanent,
//! empty aggregate set) are validator/engine guarantees and stay
//! silent to keep hot loops quiet.
//!
//! ## Per-selector semantics
//!
//! - `StackSize` — total number of `CardSource`s in the target's
//!   digivolution stack (top + materials).
//! - `MaterialCount` — `stack_size - 1` (materials are the cards beneath
//!   the top).
//! - `AllyCount` — number of *other* permanents under the target's
//!   controller (excludes the target itself).
//! - `DigivolutionColorCount` — distinct colors across all `CardSource`s
//!   in the target's stack.
//! - `SameLevelPairsInSources` — counts source cards below the target's top
//!   card by level and sums `count / 2` for each level bucket.
//! - `CardCountInZoneScoped` — number of cards in the selected zone for
//!   the selected player scope. The legacy payload-less variant returns
//!   0 for compatibility with older malformed compiled packs.
//!
//! ## Aggregate selector semantics
//!
//! - `LowestDp` / `HighestDp` — min / max effective DP across the selected
//!   players' battle areas.
//! - `LowestLevel` / `HighestLevel` — min / max top-card level across the
//!   selected players' battle areas (permanents whose top card has no level
//!   — e.g. a Tamer or Option permanent — are skipped). The legacy
//!   payload-less aggregate variant scans the controller for compatibility.

use std::collections::BTreeMap;
use std::collections::HashSet;
use std::sync::OnceLock;

use digimon_dsl::compiled::{
    CompiledAggregateSelector, CompiledFormula, CompiledPerSelector, CompiledPlayerRef,
    CompiledPredicate, CompiledZone,
};

use crate::card_source::CardHandle;
use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::predicate::{eval_predicate_with_bindings, PredicateSubject};
use crate::dsl_cards::raw_rust::EngineRawRustRegistry;
use crate::effect_context::{EffectContext, EffectReadContext};
use crate::enums::{CardColor, PlayerId};
use crate::permanent::{Permanent, PermanentHandle};

// Compile-time guard for the `DigivolutionColorCount` u8 bitmask in
// `evaluate_per`. `1u8 << n` is undefined behavior for `n >= 8`. Today
// `CardColor` has 7 variants (Red=0..Purple=6); if a future color is
// added that pushes the discriminant >= 8, this build break forces the
// author to widen the bitmask (u16 / hash-set) instead of silently
// shifting into UB. `Purple` is the highest-disc variant in `enums.rs`.
const _: () = assert!(
    (CardColor::Purple as u8) < 8,
    "DigivolutionColorCount uses a u8 bitmask; CardColor must fit in u8"
);

/// Evaluate a `CompiledFormula` against the live game state in `ctx`,
/// using `target` as the resolution point for per-selectors that
/// reference a specific permanent (stack/material/color counts).
///
/// Scoped aggregate selectors ignore `target` and operate on the selected
/// players' battle areas.
pub fn evaluate(f: &CompiledFormula, ctx: &EffectContext<'_>, target: PermanentHandle) -> i32 {
    evaluate_with_bindings(f, ctx, target, None)
}

pub fn evaluate_with_bindings(
    f: &CompiledFormula,
    ctx: &EffectContext<'_>,
    target: PermanentHandle,
    bindings: Option<&Bindings>,
) -> i32 {
    match f {
        CompiledFormula::Literal(n) => *n,
        CompiledFormula::BasePerDelta { base, per, delta } => {
            let count = evaluate_per(per, ctx, target, bindings);
            base + count * delta
        }
        CompiledFormula::FloorDiv(args) => {
            // FloorDiv is defined as binary in the DSL surface (`floor_div(a, b)`),
            // and the compiled form ships the operands as a Vec for forward-
            // compatibility. Anything other than 2 operands is malformed; return 0.
            if args.len() != 2 {
                #[cfg(debug_assertions)]
                eprintln!(
                    "[debug] formula_eval: FloorDiv with {} operands (expected 2); \
                     returning 0. This indicates a malformed compiled pack — the \
                     DSL surface enforces binary `floor_div(a, b)`.",
                    args.len()
                );
                return 0;
            }
            let l = evaluate_with_bindings(&args[0], ctx, target, bindings);
            let r = evaluate_with_bindings(&args[1], ctx, target, bindings);
            if r == 0 {
                0
            } else {
                l.div_euclid(r)
            }
        }
        CompiledFormula::Max(args) => {
            if args.is_empty() {
                return 0;
            }
            args.iter()
                .map(|a| evaluate_with_bindings(a, ctx, target, bindings))
                .max()
                .unwrap_or(0)
        }
        CompiledFormula::Min(args) => {
            if args.is_empty() {
                return 0;
            }
            args.iter()
                .map(|a| evaluate_with_bindings(a, ctx, target, bindings))
                .min()
                .unwrap_or(0)
        }
        CompiledFormula::Aggregate(sel) => evaluate_aggregate(*sel, CompiledPlayerRef::You, ctx),
        CompiledFormula::AggregateScoped { selector, scope } => {
            evaluate_aggregate(*selector, *scope, ctx)
        }
        CompiledFormula::BindingDp(name) => bindings
            .and_then(|b| b.get_permanent(name))
            .and_then(|handle| ctx.game.effective_dp(handle))
            .unwrap_or(0),
        CompiledFormula::BindingPlayCost(name) => binding_play_cost(ctx, name, bindings),
        CompiledFormula::SourceStackDpSum {
            target: target_name,
            filter,
        } => {
            let read = ctx.as_read();
            source_stack_dp_sum(target_name, filter.as_deref(), &read, target, bindings)
        }
        CompiledFormula::RawRust(name) => {
            if let Some(value) = ctx.game.formula_extensions.evaluate(name, ctx, target) {
                return value;
            }
            #[cfg(debug_assertions)]
            eprintln!(
                "[debug] formula_eval: RawRust(\"{}\") has no engine callback; returning 0.",
                name
            );
            0
        }
    }
}

pub fn evaluate_with_raw(
    f: &CompiledFormula,
    ctx: &EffectContext<'_>,
    target: PermanentHandle,
    raw: &EngineRawRustRegistry,
) -> i32 {
    evaluate_with_raw_and_bindings(f, ctx, target, raw, None)
}

pub fn evaluate_with_raw_and_bindings(
    f: &CompiledFormula,
    ctx: &EffectContext<'_>,
    target: PermanentHandle,
    raw: &EngineRawRustRegistry,
    bindings: Option<&Bindings>,
) -> i32 {
    if let CompiledFormula::RawRust(name) = f {
        if let Some(f) = raw.formula_fn(name) {
            let read = ctx.as_read();
            return f(&read, target);
        }
        if let Some(value) = ctx.game.formula_extensions.evaluate(name, ctx, target) {
            return value;
        }
    }
    let read = ctx.as_read();
    evaluate_read_with_raw_and_bindings(f, &read, target, raw, bindings)
}

pub fn evaluate_read(
    f: &CompiledFormula,
    ctx: &EffectReadContext<'_>,
    target: PermanentHandle,
) -> i32 {
    evaluate_read_with_bindings(f, ctx, target, None)
}

pub fn evaluate_read_with_bindings(
    f: &CompiledFormula,
    ctx: &EffectReadContext<'_>,
    target: PermanentHandle,
    bindings: Option<&Bindings>,
) -> i32 {
    evaluate_read_with_raw_and_bindings(f, ctx, target, default_raw_registry(), bindings)
}

fn default_raw_registry() -> &'static EngineRawRustRegistry {
    static RAW: OnceLock<EngineRawRustRegistry> = OnceLock::new();
    RAW.get_or_init(crate::cards::raw_rust::build_registry)
}

pub fn evaluate_read_with_raw(
    f: &CompiledFormula,
    ctx: &EffectReadContext<'_>,
    target: PermanentHandle,
    raw: &EngineRawRustRegistry,
) -> i32 {
    evaluate_read_with_raw_and_bindings(f, ctx, target, raw, None)
}

fn evaluate_read_with_raw_and_bindings(
    f: &CompiledFormula,
    ctx: &EffectReadContext<'_>,
    target: PermanentHandle,
    raw: &EngineRawRustRegistry,
    bindings: Option<&Bindings>,
) -> i32 {
    match f {
        CompiledFormula::Literal(n) => *n,
        CompiledFormula::BasePerDelta { base, per, delta } => {
            let count = evaluate_per_read(per, ctx, target, bindings);
            base + count * delta
        }
        CompiledFormula::FloorDiv(args) => {
            if args.len() != 2 {
                #[cfg(debug_assertions)]
                eprintln!(
                    "[debug] formula_eval: FloorDiv with {} operands (expected 2); returning 0.",
                    args.len()
                );
                return 0;
            }
            let l = evaluate_read_with_raw_and_bindings(&args[0], ctx, target, raw, bindings);
            let r = evaluate_read_with_raw_and_bindings(&args[1], ctx, target, raw, bindings);
            if r == 0 {
                0
            } else {
                l.div_euclid(r)
            }
        }
        CompiledFormula::Max(args) => args
            .iter()
            .map(|a| evaluate_read_with_raw_and_bindings(a, ctx, target, raw, bindings))
            .max()
            .unwrap_or(0),
        CompiledFormula::Min(args) => args
            .iter()
            .map(|a| evaluate_read_with_raw_and_bindings(a, ctx, target, raw, bindings))
            .min()
            .unwrap_or(0),
        CompiledFormula::Aggregate(sel) => {
            evaluate_aggregate_read(*sel, CompiledPlayerRef::You, ctx)
        }
        CompiledFormula::AggregateScoped { selector, scope } => {
            evaluate_aggregate_read(*selector, *scope, ctx)
        }
        CompiledFormula::BindingDp(name) => bindings
            .and_then(|b| b.get_permanent(name))
            .and_then(|handle| ctx.game.effective_dp(handle))
            .unwrap_or(0),
        CompiledFormula::BindingPlayCost(name) => binding_play_cost_read(ctx, name, bindings),
        CompiledFormula::SourceStackDpSum {
            target: target_name,
            filter,
        } => source_stack_dp_sum(target_name, filter.as_deref(), ctx, target, bindings),
        CompiledFormula::RawRust(name) => {
            if let Some(f) = raw.formula_fn(name) {
                return f(ctx, target);
            }
            #[cfg(debug_assertions)]
            eprintln!(
                "[debug] formula_eval: RawRust(\"{}\") not registered; returning 0.",
                name
            );
            0
        }
    }
}

fn evaluate_per(
    sel: &CompiledPerSelector,
    ctx: &EffectContext<'_>,
    target: PermanentHandle,
    bindings: Option<&Bindings>,
) -> i32 {
    match sel {
        CompiledPerSelector::StackSize => {
            let Some(perm) = target_permanent(ctx, target) else {
                return 0;
            };
            perm.card_sources.len() as i32
        }
        CompiledPerSelector::MaterialCount => {
            let Some(perm) = target_permanent(ctx, target) else {
                return 0;
            };
            // Materials are the digivolution sources beneath the top card.
            // saturating_sub guards the (impossible) zero-source permanent.
            perm.card_sources.len().saturating_sub(1) as i32
        }
        CompiledPerSelector::AllyCount => {
            // Other permanents under the same controller (exclude self).
            ctx.game
                .player(target.player)
                .battle_area
                .len()
                .saturating_sub(1) as i32
        }
        CompiledPerSelector::SuspendedCount { of } => players_for_ref(*of, ctx)
            .into_iter()
            .map(|player| {
                ctx.game
                    .player(player)
                    .battle_area
                    .iter()
                    .filter(|perm| perm.is_suspended)
                    .count() as i32
            })
            .sum(),
        CompiledPerSelector::DigivolutionColorCount => {
            let Some(perm) = target_permanent(ctx, target) else {
                return 0;
            };
            // Distinct colors across every CardSource in the stack.
            let data = &ctx.game.card_data;
            let mut seen: u8 = 0;
            for src in &perm.card_sources {
                for c in src.colors(data) {
                    seen |= 1u8 << (*c as u8);
                }
            }
            seen.count_ones() as i32
        }
        CompiledPerSelector::SameLevelPairsInSources => target_permanent(ctx, target)
            .map(|perm| same_level_pairs_in_sources(perm, &ctx.game.card_data))
            .unwrap_or(0),
        CompiledPerSelector::SharedTrashCount { bucket } => {
            bucket_count(shared_trash_count(ctx), *bucket)
        }
        CompiledPerSelector::CardCountInZone => {
            // Legacy compiled packs had no zone/player payload. Keep the
            // defensive no-op so old malformed packs do not crash.
            0
        }
        CompiledPerSelector::CardCountInZoneScoped { zone, of } => players_for_ref(*of, ctx)
            .into_iter()
            .map(|player| count_zone(*zone, player, ctx, None, bindings))
            .sum(),
        CompiledPerSelector::FilteredCardCountInZoneScoped { zone, of, filter } => {
            players_for_ref(*of, ctx)
                .into_iter()
                .map(|player| count_zone(*zone, player, ctx, Some(filter), bindings))
                .sum()
        }
        CompiledPerSelector::DistinctColorsCountScoped { zone, of, filter } => {
            let read = ctx.as_read();
            players_for_ref(*of, ctx)
                .into_iter()
                .flat_map(|player| {
                    distinct_colors_in_zone(*zone, player, &read, filter.as_deref(), bindings)
                })
                .collect::<HashSet<_>>()
                .len() as i32
        }
    }
}

fn evaluate_per_read(
    sel: &CompiledPerSelector,
    ctx: &EffectReadContext<'_>,
    target: PermanentHandle,
    bindings: Option<&Bindings>,
) -> i32 {
    match sel {
        CompiledPerSelector::StackSize => target_permanent_read(ctx, target)
            .map(|perm| perm.card_sources.len() as i32)
            .unwrap_or(0),
        CompiledPerSelector::MaterialCount => target_permanent_read(ctx, target)
            .map(|perm| perm.card_sources.len().saturating_sub(1) as i32)
            .unwrap_or(0),
        CompiledPerSelector::AllyCount => ctx
            .game
            .player(target.player)
            .battle_area
            .len()
            .saturating_sub(1) as i32,
        CompiledPerSelector::SuspendedCount { of } => players_for_ref_read(*of, ctx)
            .into_iter()
            .map(|player| {
                ctx.game
                    .player(player)
                    .battle_area
                    .iter()
                    .filter(|perm| perm.is_suspended)
                    .count() as i32
            })
            .sum(),
        CompiledPerSelector::DigivolutionColorCount => {
            let Some(perm) = target_permanent_read(ctx, target) else {
                return 0;
            };
            let data = &ctx.game.card_data;
            let mut seen: u8 = 0;
            for src in &perm.card_sources {
                for c in src.colors(data) {
                    seen |= 1u8 << (*c as u8);
                }
            }
            seen.count_ones() as i32
        }
        CompiledPerSelector::SameLevelPairsInSources => target_permanent_read(ctx, target)
            .map(|perm| same_level_pairs_in_sources(perm, ctx.card_data()))
            .unwrap_or(0),
        CompiledPerSelector::SharedTrashCount { bucket } => {
            bucket_count(shared_trash_count_read(ctx), *bucket)
        }
        CompiledPerSelector::CardCountInZone => 0,
        CompiledPerSelector::CardCountInZoneScoped { zone, of } => players_for_ref_read(*of, ctx)
            .into_iter()
            .map(|player| count_zone_read(*zone, player, ctx, None, bindings))
            .sum(),
        CompiledPerSelector::FilteredCardCountInZoneScoped { zone, of, filter } => {
            players_for_ref_read(*of, ctx)
                .into_iter()
                .map(|player| count_zone_read(*zone, player, ctx, Some(filter), bindings))
                .sum()
        }
        CompiledPerSelector::DistinctColorsCountScoped { zone, of, filter } => {
            players_for_ref_read(*of, ctx)
                .into_iter()
                .flat_map(|player| {
                    distinct_colors_in_zone(*zone, player, ctx, filter.as_deref(), bindings)
                })
                .collect::<HashSet<_>>()
                .len() as i32
        }
    }
}

fn target_permanent<'a>(
    ctx: &'a EffectContext<'_>,
    target: PermanentHandle,
) -> Option<&'a crate::permanent::Permanent> {
    if target.index == crate::action::space::BREEDING_TARGET as u8 {
        return ctx.game.player(target.player).breeding_area.as_ref();
    }
    ctx.game
        .player(target.player)
        .battle_area
        .get(target.index as usize)
}

fn target_permanent_read<'a>(
    ctx: &'a EffectReadContext<'_>,
    target: PermanentHandle,
) -> Option<&'a Permanent> {
    if target.index == crate::action::space::BREEDING_TARGET as u8 {
        return ctx.game.player(target.player).breeding_area.as_ref();
    }
    ctx.game
        .player(target.player)
        .battle_area
        .get(target.index as usize)
}

fn same_level_pairs_in_sources(perm: &Permanent, data: &[crate::card_data::CardData]) -> i32 {
    let mut counts: BTreeMap<u8, i32> = BTreeMap::new();
    for source in perm.card_sources.iter().rev().skip(1) {
        if let Some(level) = source.level(data) {
            *counts.entry(level).or_default() += 1;
        }
    }
    counts.values().map(|count| count / 2).sum()
}

fn source_stack_dp_sum(
    target_name: &str,
    filter: Option<&CompiledPredicate>,
    ctx: &EffectReadContext<'_>,
    fallback_target: PermanentHandle,
    bindings: Option<&Bindings>,
) -> i32 {
    let Some(target) = resolve_formula_target(target_name, ctx, fallback_target, bindings) else {
        return 0;
    };
    let Some(perm) = target_permanent_read(ctx, target) else {
        return 0;
    };
    perm.card_sources
        .iter()
        .rev()
        .skip(1)
        .filter(|source| {
            filter.is_none_or(|filter| {
                eval_predicate_with_bindings(
                    filter,
                    ctx,
                    PredicateSubject::Card(source.handle()),
                    bindings,
                )
            })
        })
        .filter_map(|source| source.dp(ctx.card_data()))
        .sum()
}

fn binding_play_cost(ctx: &EffectContext<'_>, name: &str, bindings: Option<&Bindings>) -> i32 {
    let Some(bindings) = bindings else {
        return 0;
    };
    if let Some(card) = bindings.get_card(name) {
        return ctx
            .game
            .card_data_for_handle(card)
            .map(|data| i32::from(data.play_cost))
            .unwrap_or(0);
    }
    if let Some(handle) = bindings.get_permanent(name) {
        return target_permanent(ctx, handle)
            .and_then(|perm| ctx.game.card_data_for_handle(perm.top_card().handle()))
            .map(|data| i32::from(data.play_cost))
            .unwrap_or(0);
    }
    if let Some((player, index)) = bindings.get_hand_index_with_player(name) {
        return ctx
            .game
            .player(player)
            .hand
            .get(index as usize)
            .and_then(|card| ctx.game.card_data_for_handle(card.handle()))
            .map(|data| i32::from(data.play_cost))
            .unwrap_or(0);
    }
    if let Some((player, index)) = bindings.get_trash_index_with_player(name) {
        return ctx
            .game
            .player(player)
            .trash
            .get(index as usize)
            .and_then(|card| ctx.game.card_data_for_handle(card.handle()))
            .map(|data| i32::from(data.play_cost))
            .unwrap_or(0);
    }
    0
}

fn binding_play_cost_read(
    ctx: &EffectReadContext<'_>,
    name: &str,
    bindings: Option<&Bindings>,
) -> i32 {
    let Some(bindings) = bindings else {
        return 0;
    };
    if let Some(card) = bindings.get_card(name) {
        return ctx
            .game
            .card_data_for_handle(card)
            .map(|data| i32::from(data.play_cost))
            .unwrap_or(0);
    }
    if let Some(handle) = bindings.get_permanent(name) {
        return target_permanent_read(ctx, handle)
            .and_then(|perm| ctx.game.card_data_for_handle(perm.top_card().handle()))
            .map(|data| i32::from(data.play_cost))
            .unwrap_or(0);
    }
    if let Some((player, index)) = bindings.get_hand_index_with_player(name) {
        return ctx
            .game
            .player(player)
            .hand
            .get(index as usize)
            .and_then(|card| ctx.game.card_data_for_handle(card.handle()))
            .map(|data| i32::from(data.play_cost))
            .unwrap_or(0);
    }
    if let Some((player, index)) = bindings.get_trash_index_with_player(name) {
        return ctx
            .game
            .player(player)
            .trash
            .get(index as usize)
            .and_then(|card| ctx.game.card_data_for_handle(card.handle()))
            .map(|data| i32::from(data.play_cost))
            .unwrap_or(0);
    }
    0
}

fn resolve_formula_target(
    target_name: &str,
    ctx: &EffectReadContext<'_>,
    fallback_target: PermanentHandle,
    bindings: Option<&Bindings>,
) -> Option<PermanentHandle> {
    match target_name {
        "self" | "target" => Some(fallback_target),
        "source" => ctx.source_permanent,
        name => bindings.and_then(|b| b.get_permanent(name)),
    }
}

fn players_for_ref(of: CompiledPlayerRef, ctx: &EffectContext<'_>) -> Vec<PlayerId> {
    match of {
        CompiledPlayerRef::You => vec![ctx.player],
        CompiledPlayerRef::Opponent => vec![ctx.opponent_id()],
        CompiledPlayerRef::Active => vec![ctx.game.turn_player()],
        CompiledPlayerRef::Any => (0..ctx.game.players.len() as PlayerId).collect(),
    }
}

fn players_for_ref_read(of: CompiledPlayerRef, ctx: &EffectReadContext<'_>) -> Vec<PlayerId> {
    match of {
        CompiledPlayerRef::You => vec![ctx.player],
        CompiledPlayerRef::Opponent => vec![ctx.opponent_id()],
        CompiledPlayerRef::Active => vec![ctx.game.turn_player()],
        CompiledPlayerRef::Any => (0..ctx.game.players.len() as PlayerId).collect(),
    }
}

fn shared_trash_count(ctx: &EffectContext<'_>) -> i32 {
    ctx.game
        .players
        .iter()
        .map(|player| player.trash.len() as i32)
        .sum()
}

fn shared_trash_count_read(ctx: &EffectReadContext<'_>) -> i32 {
    ctx.game
        .players
        .iter()
        .map(|player| player.trash.len() as i32)
        .sum()
}

fn bucket_count(count: i32, bucket: Option<u32>) -> i32 {
    match bucket {
        Some(0) => 0,
        Some(bucket) => count.div_euclid(bucket as i32),
        None => count,
    }
}

fn count_zone(
    zone: CompiledZone,
    player: PlayerId,
    ctx: &EffectContext<'_>,
    filter: Option<&CompiledPredicate>,
    bindings: Option<&Bindings>,
) -> i32 {
    if filter.is_some() {
        let read = ctx.as_read();
        return count_zone_filtered_read(zone, player, &read, filter, bindings);
    }
    let player_state = ctx.game.player(player);
    let count = match zone {
        CompiledZone::Hand => player_state.hand.len(),
        CompiledZone::Deck => player_state.deck.len(),
        CompiledZone::Trash => player_state.trash.len(),
        CompiledZone::BattleArea => player_state.battle_area.len(),
        CompiledZone::Security => player_state.security.len(),
        CompiledZone::Breeding => usize::from(player_state.breeding_area.is_some()),
        CompiledZone::DigiEggDeck => player_state.digitama_deck.len(),
        CompiledZone::Reveal => ctx
            .game
            .revealed_cards
            .iter()
            .filter(|card| card.owner == player)
            .count(),
        CompiledZone::Material => {
            let battle_materials = player_state
                .battle_area
                .iter()
                .map(|perm| perm.card_sources.len().saturating_sub(1))
                .sum::<usize>();
            let breeding_materials = player_state
                .breeding_area
                .as_ref()
                .map(|perm| perm.card_sources.len().saturating_sub(1))
                .unwrap_or(0);
            battle_materials + breeding_materials
        }
    };
    count as i32
}

fn count_zone_read(
    zone: CompiledZone,
    player: PlayerId,
    ctx: &EffectReadContext<'_>,
    filter: Option<&CompiledPredicate>,
    bindings: Option<&Bindings>,
) -> i32 {
    if filter.is_some() {
        return count_zone_filtered_read(zone, player, ctx, filter, bindings);
    }
    let player_state = ctx.game.player(player);
    let count = match zone {
        CompiledZone::Hand => player_state.hand.len(),
        CompiledZone::Deck => player_state.deck.len(),
        CompiledZone::Trash => player_state.trash.len(),
        CompiledZone::BattleArea => player_state.battle_area.len(),
        CompiledZone::Security => player_state.security.len(),
        CompiledZone::Breeding => usize::from(player_state.breeding_area.is_some()),
        CompiledZone::DigiEggDeck => player_state.digitama_deck.len(),
        CompiledZone::Reveal => ctx
            .game
            .revealed_cards
            .iter()
            .filter(|card| card.owner == player)
            .count(),
        CompiledZone::Material => {
            let battle_materials = player_state
                .battle_area
                .iter()
                .map(|perm| perm.card_sources.len().saturating_sub(1))
                .sum::<usize>();
            let breeding_materials = player_state
                .breeding_area
                .as_ref()
                .map(|perm| perm.card_sources.len().saturating_sub(1))
                .unwrap_or(0);
            battle_materials + breeding_materials
        }
    };
    count as i32
}

fn distinct_colors_in_zone(
    zone: CompiledZone,
    player: PlayerId,
    ctx: &EffectReadContext<'_>,
    filter: Option<&CompiledPredicate>,
    bindings: Option<&Bindings>,
) -> Vec<CardColor> {
    let player_state = ctx.game.player(player);
    match zone {
        CompiledZone::BattleArea => player_state
            .battle_area
            .iter()
            .enumerate()
            .filter_map(|(index, perm)| {
                let handle = PermanentHandle {
                    player,
                    index: index as u8,
                };
                if filter.is_some_and(|filter| {
                    !eval_predicate_with_bindings(
                        filter,
                        ctx,
                        PredicateSubject::Permanent(handle),
                        bindings,
                    )
                }) {
                    return None;
                }
                Some(perm.top_card().colors(ctx.card_data()).to_vec())
            })
            .flatten()
            .collect(),
        CompiledZone::Breeding => player_state
            .breeding_area
            .as_ref()
            .filter(|_| {
                filter.is_none_or(|filter| {
                    eval_predicate_with_bindings(
                        filter,
                        ctx,
                        PredicateSubject::BreedingPermanent(player),
                        bindings,
                    )
                })
            })
            .map(|perm| perm.top_card().colors(ctx.card_data()).to_vec())
            .unwrap_or_default(),
        CompiledZone::Hand => distinct_colors_from_cards(&player_state.hand, filter, ctx, bindings),
        CompiledZone::Deck => distinct_colors_from_cards(&player_state.deck, filter, ctx, bindings),
        CompiledZone::Trash => {
            distinct_colors_from_cards(&player_state.trash, filter, ctx, bindings)
        }
        CompiledZone::Security => {
            distinct_colors_from_cards(&player_state.security, filter, ctx, bindings)
        }
        CompiledZone::DigiEggDeck => {
            distinct_colors_from_cards(&player_state.digitama_deck, filter, ctx, bindings)
        }
        CompiledZone::Reveal => ctx
            .game
            .revealed_cards
            .iter()
            .filter(|card| card.owner == player)
            .filter(|card| {
                filter.is_none_or(|filter| {
                    eval_predicate_with_bindings(
                        filter,
                        ctx,
                        PredicateSubject::RevealedCard(card.handle()),
                        bindings,
                    )
                })
            })
            .flat_map(|card| card.colors(ctx.card_data()).to_vec())
            .collect(),
        CompiledZone::Material => player_state
            .battle_area
            .iter()
            .chain(player_state.breeding_area.iter())
            .flat_map(|perm| perm.card_sources.iter().rev().skip(1))
            .filter(|source| {
                filter.is_none_or(|filter| {
                    eval_predicate_with_bindings(
                        filter,
                        ctx,
                        PredicateSubject::Card(source.handle()),
                        bindings,
                    )
                })
            })
            .flat_map(|source| source.colors(ctx.card_data()).to_vec())
            .collect(),
    }
}

fn distinct_colors_from_cards(
    cards: &[crate::card_source::CardSource],
    filter: Option<&CompiledPredicate>,
    ctx: &EffectReadContext<'_>,
    bindings: Option<&Bindings>,
) -> Vec<CardColor> {
    cards
        .iter()
        .filter(|source| {
            filter.is_none_or(|filter| {
                eval_predicate_with_bindings(
                    filter,
                    ctx,
                    PredicateSubject::Card(source.handle()),
                    bindings,
                )
            })
        })
        .flat_map(|source| source.colors(ctx.card_data()).to_vec())
        .collect()
}

fn count_zone_filtered_read(
    zone: CompiledZone,
    player: PlayerId,
    ctx: &EffectReadContext<'_>,
    filter: Option<&CompiledPredicate>,
    bindings: Option<&Bindings>,
) -> i32 {
    let Some(filter) = filter else {
        return 0;
    };
    let player_state = ctx.game.player(player);
    match zone {
        CompiledZone::BattleArea => player_state
            .battle_area
            .iter()
            .enumerate()
            .filter(|(index, _)| {
                let handle = PermanentHandle {
                    player,
                    index: *index as u8,
                };
                eval_predicate_with_bindings(
                    filter,
                    ctx,
                    PredicateSubject::Permanent(handle),
                    bindings,
                )
            })
            .count() as i32,
        CompiledZone::Breeding => player_state.breeding_area.as_ref().is_some_and(|_| {
            eval_predicate_with_bindings(
                filter,
                ctx,
                PredicateSubject::BreedingPermanent(player),
                bindings,
            )
        }) as i32,
        CompiledZone::Hand => {
            count_card_sources_filtered(&player_state.hand, filter, ctx, bindings)
        }
        CompiledZone::Deck => {
            count_card_sources_filtered(&player_state.deck, filter, ctx, bindings)
        }
        CompiledZone::Trash => {
            count_card_sources_filtered(&player_state.trash, filter, ctx, bindings)
        }
        CompiledZone::Security => {
            count_card_sources_filtered(&player_state.security, filter, ctx, bindings)
        }
        CompiledZone::DigiEggDeck => {
            count_card_sources_filtered(&player_state.digitama_deck, filter, ctx, bindings)
        }
        CompiledZone::Reveal => ctx
            .game
            .revealed_cards
            .iter()
            .filter(|card| card.owner == player)
            .filter(|card| {
                eval_predicate_with_bindings(
                    filter,
                    ctx,
                    PredicateSubject::RevealedCard(card.handle()),
                    bindings,
                )
            })
            .count() as i32,
        CompiledZone::Material => {
            let battle = player_state
                .battle_area
                .iter()
                .flat_map(|perm| perm.card_sources.iter().rev().skip(1))
                .filter(|source| card_source_matches(source.handle(), filter, ctx, bindings))
                .count();
            let breeding = player_state
                .breeding_area
                .as_ref()
                .map(|perm| {
                    perm.card_sources
                        .iter()
                        .rev()
                        .skip(1)
                        .filter(|source| {
                            card_source_matches(source.handle(), filter, ctx, bindings)
                        })
                        .count()
                })
                .unwrap_or(0);
            (battle + breeding) as i32
        }
    }
}

fn count_card_sources_filtered(
    cards: &[crate::card_source::CardSource],
    filter: &CompiledPredicate,
    ctx: &EffectReadContext<'_>,
    bindings: Option<&Bindings>,
) -> i32 {
    cards
        .iter()
        .filter(|source| card_source_matches(source.handle(), filter, ctx, bindings))
        .count() as i32
}

fn card_source_matches(
    card: CardHandle,
    filter: &CompiledPredicate,
    ctx: &EffectReadContext<'_>,
    bindings: Option<&Bindings>,
) -> bool {
    eval_card_zone_filter(filter, ctx, card, bindings)
}

fn eval_card_zone_filter(
    pred: &CompiledPredicate,
    ctx: &EffectReadContext<'_>,
    card: CardHandle,
    bindings: Option<&Bindings>,
) -> bool {
    if predicate_has_card_zone_unsupported_leaf(pred) {
        return false;
    }

    let mut leaf = pred.clone();
    leaf.all_of.clear();
    leaf.any_of.clear();
    leaf.none_of.clear();
    leaf.not = None;
    if !eval_predicate_with_bindings(&leaf, ctx, PredicateSubject::Card(card), bindings) {
        return false;
    }
    if !pred
        .all_of
        .iter()
        .all(|child| eval_card_zone_filter(child, ctx, card, bindings))
    {
        return false;
    }
    if !pred.any_of.is_empty()
        && !pred
            .any_of
            .iter()
            .any(|child| eval_card_zone_filter(child, ctx, card, bindings))
    {
        return false;
    }
    if pred
        .none_of
        .iter()
        .any(|child| eval_card_zone_filter(child, ctx, card, bindings))
    {
        return false;
    }
    if pred
        .not
        .as_deref()
        .is_some_and(|child| eval_card_zone_filter(child, ctx, card, bindings))
    {
        return false;
    }
    true
}

fn predicate_has_card_zone_unsupported_leaf(pred: &CompiledPredicate) -> bool {
    let has_permanent_only_leaf = pred.dp_eq.is_some()
        || pred.dp_lte.is_some()
        || pred.dp_gte.is_some()
        || pred.level_matches_aggregate.is_some()
        || pred.stack_size_lte.is_some()
        || pred.stack_size_gte.is_some()
        || pred.materials_count_lte.is_some()
        || pred.materials_count_gte.is_some()
        || pred.has_inherited.is_some()
        || pred.is_suspended.is_some()
        || pred.is_unsuspended.is_some()
        || pred.owner.is_some()
        || pred.other.is_some()
        || pred.of_permanent.is_some()
        || pred.source_is_tamer.is_some()
        || pred.source_is_unsuspended.is_some()
        || pred.source_name_contains.is_some()
        || pred.source_permanent_trait_has.is_some()
        || pred.is_face_down.is_some()
        || pred.is_bottom_source.is_some()
        || pred.host_kind_is.is_some()
        || pred.in_breeding.is_some()
        || pred.on_field.is_some()
        || pred.dna_origin.is_some()
        || pred.has_alt_path.is_some();
    has_permanent_only_leaf
}

fn evaluate_aggregate(
    sel: CompiledAggregateSelector,
    scope: CompiledPlayerRef,
    ctx: &EffectContext<'_>,
) -> i32 {
    use CompiledAggregateSelector as A;
    let values = players_for_ref(scope, ctx).into_iter().flat_map(|player| {
        let len = ctx.game.player(player).battle_area.len();
        (0..len).filter_map(move |index| aggregate_value(sel, player, index, ctx))
    });

    match sel {
        A::LowestDp | A::LowestLevel => values.min().unwrap_or(0),
        A::HighestDp | A::HighestLevel => values.max().unwrap_or(0),
    }
}

fn evaluate_aggregate_read(
    sel: CompiledAggregateSelector,
    scope: CompiledPlayerRef,
    ctx: &EffectReadContext<'_>,
) -> i32 {
    use CompiledAggregateSelector as A;
    let values = players_for_ref_read(scope, ctx)
        .into_iter()
        .flat_map(|player| {
            let len = ctx.game.player(player).battle_area.len();
            (0..len).filter_map(move |index| aggregate_value_read(sel, player, index, ctx))
        });

    match sel {
        A::LowestDp | A::LowestLevel => values.min().unwrap_or(0),
        A::HighestDp | A::HighestLevel => values.max().unwrap_or(0),
    }
}

fn aggregate_value(
    sel: CompiledAggregateSelector,
    player: PlayerId,
    index: usize,
    ctx: &EffectContext<'_>,
) -> Option<i32> {
    use CompiledAggregateSelector as A;
    let handle = PermanentHandle {
        player,
        index: index as u8,
    };
    match sel {
        A::LowestDp | A::HighestDp => ctx.game.effective_dp(handle),
        A::LowestLevel | A::HighestLevel => ctx
            .game
            .player(player)
            .battle_area
            .get(index)?
            .level(&ctx.game.card_data)
            .map(i32::from),
    }
}

fn aggregate_value_read(
    sel: CompiledAggregateSelector,
    player: PlayerId,
    index: usize,
    ctx: &EffectReadContext<'_>,
) -> Option<i32> {
    let handle = PermanentHandle {
        player,
        index: index as u8,
    };
    let perm = ctx.game.player(player).battle_area.get(index)?;
    match sel {
        CompiledAggregateSelector::LowestDp | CompiledAggregateSelector::HighestDp => {
            ctx.game.effective_dp(handle)
        }
        CompiledAggregateSelector::LowestLevel | CompiledAggregateSelector::HighestLevel => {
            perm.level(ctx.card_data()).map(i32::from)
        }
    }
}
