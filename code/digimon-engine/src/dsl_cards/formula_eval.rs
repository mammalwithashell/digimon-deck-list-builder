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
//! The two card-author-bug branches (mis-arity FloorDiv, unregistered
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
//! - `CardCountInZone` — card count in the requested player/zone payload.
//!   `battle_area` counts top-level permanents; `material` counts all
//!   digivolution cards beneath battle-area permanents for the player.
//!
//! ## Aggregate selector semantics
//!
//! - `LowestDp` / `HighestDp` — min / max effective DP across the
//!   controller's battle area.
//! - `LowestLevel` / `HighestLevel` — min / max top-card level across the
//!   controller's battle area (permanents whose top card has no level —
//!   e.g. a Tamer or Option permanent — are skipped).

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
/// Aggregate selectors ignore `target` and operate on the *controller's*
/// (`ctx.player`) battle area.
pub fn evaluate(f: &CompiledFormula, ctx: &EffectContext<'_>, target: PermanentHandle) -> i32 {
    evaluate_with_raw(f, ctx, target, &EngineRawRustRegistry::new())
}

pub fn evaluate_with_raw(
    f: &CompiledFormula,
    ctx: &EffectContext<'_>,
    target: PermanentHandle,
    raw: &EngineRawRustRegistry,
) -> i32 {
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
            let l = evaluate_read_with_raw(&args[0], ctx, target, raw);
            let r = evaluate_read_with_raw(&args[1], ctx, target, raw);
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
                .map(|a| evaluate_read_with_raw(a, ctx, target, raw))
                .max()
                .unwrap_or(0)
        }
        CompiledFormula::Min(args) => {
            if args.is_empty() {
                return 0;
            }
            args.iter()
                .map(|a| evaluate_read_with_raw(a, ctx, target, raw))
                .min()
                .unwrap_or(0)
        }
        CompiledFormula::Aggregate(sel) => evaluate_aggregate(*sel, ctx),
        CompiledFormula::RawRust(name) => {
            if let Some(f) = raw.formula_fn(name) {
                return f(ctx, target);
            }
            #[cfg(debug_assertions)]
            eprintln!(
                "[debug] formula_eval: RawRust(\"{}\") not registered; returning 0. \
                 A pack referencing a missing raw_rust formula is malformed \
                 for runtime evaluation.",
                name
            );
            0
        }
    }
}

fn evaluate_per(
    sel: CompiledPerSelector,
    ctx: &EffectReadContext<'_>,
    target: PermanentHandle,
) -> i32 {
    match sel {
        CompiledPerSelector::StackSize => target_permanent(ctx, target)
            .map(|perm| perm.card_sources.len() as i32)
            .unwrap_or(0),
        CompiledPerSelector::MaterialCount => {
            // Materials are the digivolution sources beneath the top card.
            // saturating_sub guards the (impossible) zero-source permanent.
            target_permanent(ctx, target)
                .map(|perm| perm.card_sources.len().saturating_sub(1) as i32)
                .unwrap_or(0)
        }
        CompiledPerSelector::AllyCount => {
            // Other permanents under the same controller (exclude self).
            if target_permanent(ctx, target).is_none() {
                return 0;
            }
            ctx.game
                .player(target.player)
                .battle_area
                .len()
                .saturating_sub(1) as i32
        }
        CompiledPerSelector::DigivolutionColorCount => {
            // Distinct colors across every CardSource in the stack.
            let Some(perm) = target_permanent(ctx, target) else {
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
        CompiledPerSelector::CardCountInZone { of, zone } => formula_players(ctx, of)
            .into_iter()
            .map(|player| count_cards_in_zone(ctx, player, zone))
            .sum(),
    }
}

fn target_permanent<'a>(
    ctx: &'a EffectReadContext<'_>,
    target: PermanentHandle,
) -> Option<&'a Permanent> {
    ctx.game
        .player(target.player)
        .battle_area
        .get(target.index as usize)
}

fn formula_players(ctx: &EffectReadContext<'_>, of: CompiledPlayerRef) -> Vec<PlayerId> {
    match of {
        CompiledPlayerRef::You => vec![ctx.player],
        CompiledPlayerRef::Opponent => vec![ctx.opponent_id()],
        CompiledPlayerRef::Active => vec![ctx.game.turn_player()],
        CompiledPlayerRef::Any => (0..ctx.game.players.len() as PlayerId).collect(),
    }
}

fn count_cards_in_zone(ctx: &EffectReadContext<'_>, player: PlayerId, zone: CompiledZone) -> i32 {
    let p = ctx.game.player(player);
    match zone {
        CompiledZone::Hand => p.hand.len() as i32,
        CompiledZone::Deck => p.deck.len() as i32,
        CompiledZone::Trash => p.trash.len() as i32,
        CompiledZone::BattleArea => p.battle_area.len() as i32,
        CompiledZone::Security => p.security.len() as i32,
        CompiledZone::Breeding => i32::from(p.breeding_area.is_some()),
        CompiledZone::Reveal => ctx
            .game
            .revealed_cards
            .iter()
            .filter(|card| card.owner == player)
            .count() as i32,
        CompiledZone::DigiEggDeck => p.digitama_deck.len() as i32,
        CompiledZone::Material => p
            .battle_area
            .iter()
            .map(|perm| perm.card_sources.len().saturating_sub(1) as i32)
            .sum(),
    }
}

fn evaluate_aggregate(sel: CompiledAggregateSelector, ctx: &EffectReadContext<'_>) -> i32 {
    let perms = &ctx.game.player(ctx.player).battle_area;
    if perms.is_empty() {
        return 0;
    }
    use CompiledAggregateSelector as A;
    match sel {
        A::LowestDp => {
            let mut acc: Option<i32> = None;
            for (i, _) in perms.iter().enumerate() {
                let h = PermanentHandle {
                    player: ctx.player,
                    index: i as u8,
                };
                if let Some(dp) = ctx.game.effective_dp(h) {
                    acc = Some(match acc {
                        Some(x) if x < dp => x,
                        _ => dp,
                    });
                }
            }
            acc.unwrap_or(0)
        }
        A::HighestDp => {
            let mut acc: Option<i32> = None;
            for (i, _) in perms.iter().enumerate() {
                let h = PermanentHandle {
                    player: ctx.player,
                    index: i as u8,
                };
                if let Some(dp) = ctx.game.effective_dp(h) {
                    acc = Some(match acc {
                        Some(x) if x > dp => x,
                        _ => dp,
                    });
                }
            }
            acc.unwrap_or(0)
        }
        A::LowestLevel => perms
            .iter()
            .filter_map(|p| p.level(&ctx.game.card_data))
            .min()
            .map(|l| l as i32)
            .unwrap_or(0),
        A::HighestLevel => perms
            .iter()
            .filter_map(|p| p.level(&ctx.game.card_data))
            .max()
            .map(|l| l as i32)
            .unwrap_or(0),
    }
}
