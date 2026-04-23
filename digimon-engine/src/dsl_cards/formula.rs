//! Pure formula evaluator — `CompiledFormula → i32` against a read-only
//! context. No engine-state mutation.

use digimon_dsl::compiled::{
    CompiledAggregateSelector, CompiledFormula, CompiledPerSelector,
};

use crate::effect_context::EffectReadContext;
use crate::permanent::PermanentHandle;

/// Subject for per-selector/aggregate variants. `None` means no specific
/// permanent is in scope (caller is at the effect-source level).
#[derive(Debug, Clone, Copy)]
pub enum FormulaSubject {
    Permanent(PermanentHandle),
    None,
}

pub fn eval_formula(
    f: &CompiledFormula,
    rctx: &EffectReadContext<'_>,
    subject: FormulaSubject,
) -> i32 {
    match f {
        CompiledFormula::Literal(n) => *n,
        CompiledFormula::BasePerDelta { base, per, delta } => {
            let count = resolve_per(*per, rctx, subject);
            base + (count as i32) * delta
        }
        CompiledFormula::FloorDiv(args) => {
            if args.len() != 2 {
                return 0;
            }
            let a = eval_formula(&args[0], rctx, subject);
            let b = eval_formula(&args[1], rctx, subject);
            if b == 0 { 0 } else { a / b }
        }
        CompiledFormula::Max(args) => args
            .iter()
            .map(|a| eval_formula(a, rctx, subject))
            .max()
            .unwrap_or(0),
        CompiledFormula::Min(args) => args
            .iter()
            .map(|a| eval_formula(a, rctx, subject))
            .min()
            .unwrap_or(0),
        CompiledFormula::Aggregate(sel) => resolve_aggregate(*sel, rctx),
        CompiledFormula::RawRust(_) => 0, // Phase 4 wires raw_rust dispatch.
    }
}

fn resolve_per(
    sel: CompiledPerSelector,
    rctx: &EffectReadContext<'_>,
    subject: FormulaSubject,
) -> u32 {
    match sel {
        CompiledPerSelector::MaterialCount => subject_stack(rctx, subject)
            .map(|n| n.saturating_sub(1)) // top card excluded
            .unwrap_or(0),
        CompiledPerSelector::StackSize => subject_stack(rctx, subject).unwrap_or(0),
        CompiledPerSelector::AllyCount => {
            // Count Digimon on rctx.player's battle area excluding the subject.
            let subject_idx = match subject {
                FormulaSubject::Permanent(h) if h.player == rctx.player => Some(h.index),
                _ => None,
            };
            let n = rctx.game.player(rctx.player).battle_area.len();
            let mut count = 0u32;
            for i in 0..n {
                if subject_idx == Some(i as u8) {
                    continue;
                }
                count += 1;
            }
            count
        }
        CompiledPerSelector::DigivolutionColorCount => {
            let h = match subject {
                FormulaSubject::Permanent(h) => h,
                _ => return 0,
            };
            let Some(perm) = rctx.game.player(h.player).battle_area.get(h.index as usize) else {
                return 0;
            };
            let mut colors: std::collections::HashSet<crate::enums::CardColor> =
                Default::default();
            for cs in &perm.card_sources {
                for c in cs.colors(&rctx.game.card_data) {
                    colors.insert(*c);
                }
            }
            colors.len() as u32
        }
        CompiledPerSelector::CardCountInZone => {
            // Defaults to the acting player's hand when no zone annotation
            // is available at the formula level. Phase 2+ refines this when
            // the formula carries a zone tag.
            rctx.game.player(rctx.player).hand.len() as u32
        }
    }
}

fn subject_stack(rctx: &EffectReadContext<'_>, subject: FormulaSubject) -> Option<u32> {
    let FormulaSubject::Permanent(h) = subject else {
        return None;
    };
    let perm = rctx.game.player(h.player).battle_area.get(h.index as usize)?;
    Some(perm.card_sources.len() as u32)
}

fn resolve_aggregate(
    _sel: CompiledAggregateSelector,
    _rctx: &EffectReadContext<'_>,
) -> i32 {
    // Task 3 implements aggregates.
    0
}
