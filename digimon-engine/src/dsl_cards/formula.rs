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
    _sel: CompiledPerSelector,
    _rctx: &EffectReadContext<'_>,
    _subject: FormulaSubject,
) -> u32 {
    // Task 2 implements per-selectors. Return 0 so BasePerDelta degrades to
    // its base until Task 2 fills in the real count source.
    0
}

fn resolve_aggregate(
    _sel: CompiledAggregateSelector,
    _rctx: &EffectReadContext<'_>,
) -> i32 {
    // Task 3 implements aggregates.
    0
}
