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

use digimon_dsl::compiled::{
    CompiledAggregateSelector, CompiledFormula, CompiledPerSelector, CompiledPlayerRef,
    CompiledZone,
};

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
    match f {
        CompiledFormula::Literal(n) => *n,
        CompiledFormula::BasePerDelta { base, per, delta } => {
            let count = evaluate_per(*per, ctx, target);
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
            let l = evaluate(&args[0], ctx, target);
            let r = evaluate(&args[1], ctx, target);
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
                .map(|a| evaluate(a, ctx, target))
                .max()
                .unwrap_or(0)
        }
        CompiledFormula::Min(args) => {
            if args.is_empty() {
                return 0;
            }
            args.iter()
                .map(|a| evaluate(a, ctx, target))
                .min()
                .unwrap_or(0)
        }
        CompiledFormula::Aggregate(sel) => evaluate_aggregate(*sel, CompiledPlayerRef::You, ctx),
        CompiledFormula::AggregateScoped { selector, scope } => {
            evaluate_aggregate(*selector, *scope, ctx)
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
    evaluate_read_with_raw(f, &read, target, raw)
}

pub fn evaluate_read(
    f: &CompiledFormula,
    ctx: &EffectReadContext<'_>,
    target: PermanentHandle,
) -> i32 {
    evaluate_read_with_raw(f, ctx, target, &EngineRawRustRegistry::new())
}

pub fn evaluate_read_with_raw(
    f: &CompiledFormula,
    ctx: &EffectReadContext<'_>,
    target: PermanentHandle,
    raw: &EngineRawRustRegistry,
) -> i32 {
    match f {
        CompiledFormula::Literal(n) => *n,
        CompiledFormula::BasePerDelta { base, per, delta } => {
            let count = evaluate_per_read(*per, ctx, target);
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
            let l = evaluate_read_with_raw(&args[0], ctx, target, raw);
            let r = evaluate_read_with_raw(&args[1], ctx, target, raw);
            if r == 0 {
                0
            } else {
                l.div_euclid(r)
            }
        }
        CompiledFormula::Max(args) => args
            .iter()
            .map(|a| evaluate_read_with_raw(a, ctx, target, raw))
            .max()
            .unwrap_or(0),
        CompiledFormula::Min(args) => args
            .iter()
            .map(|a| evaluate_read_with_raw(a, ctx, target, raw))
            .min()
            .unwrap_or(0),
        CompiledFormula::Aggregate(sel) => {
            evaluate_aggregate_read(*sel, CompiledPlayerRef::You, ctx)
        }
        CompiledFormula::AggregateScoped { selector, scope } => {
            evaluate_aggregate_read(*selector, *scope, ctx)
        }
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

fn evaluate_per(sel: CompiledPerSelector, ctx: &EffectContext<'_>, target: PermanentHandle) -> i32 {
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
        CompiledPerSelector::CardCountInZone => {
            // Legacy compiled packs had no zone/player payload. Keep the
            // defensive no-op so old malformed packs do not crash.
            0
        }
        CompiledPerSelector::CardCountInZoneScoped { zone, of } => players_for_ref(of, ctx)
            .into_iter()
            .map(|player| count_zone(zone, player, ctx))
            .sum(),
    }
}

fn evaluate_per_read(
    sel: CompiledPerSelector,
    ctx: &EffectReadContext<'_>,
    target: PermanentHandle,
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
        CompiledPerSelector::CardCountInZone => 0,
        CompiledPerSelector::CardCountInZoneScoped { zone, of } => players_for_ref_read(of, ctx)
            .into_iter()
            .map(|player| count_zone_read(zone, player, ctx))
            .sum(),
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

fn count_zone(zone: CompiledZone, player: PlayerId, ctx: &EffectContext<'_>) -> i32 {
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

fn count_zone_read(zone: CompiledZone, player: PlayerId, ctx: &EffectReadContext<'_>) -> i32 {
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
