//! Production raw_rust function registrations for DSL long-tail cards.
//!
//! Phase 4 keeps bespoke mechanics behind named functions here instead of
//! handwritten card modules under `src/cards/<set>/`.

use crate::dsl_cards::bindings::{BindingValue, Bindings};
use crate::dsl_cards::raw_rust::EngineRawRustRegistry;
use crate::effect::Effect;
use crate::effect_context::EffectContext;

/// EX11-012 Medusamon — [When Digivolving][End of Attack] trash-return step.
///
/// Reads the `TrashIndex` binding named `"r"` (installed by the preceding
/// `select_trash` step) and moves that card from the opponent's trash to the
/// bottom of the opponent's deck.  If the binding is absent or the index is
/// out-of-range the function is a no-op (defensive).
fn ex11_012_return_trash_to_deck_bottom(ctx: &mut EffectContext<'_>, bindings: &mut Bindings) {
    if let Some(BindingValue::TrashIndex(owner, idx)) = bindings.get("r") {
        let idx = idx as usize;
        let owner = owner as usize;
        if idx < ctx.game.players[owner].trash.len() {
            let card = ctx.game.players[owner].trash.remove(idx);
            // Bottom of deck = index 0 (deck is stored front=bottom, back=top).
            ctx.game.players[owner].deck.insert(0, card);
        }
    }
}

/// EX11-054 Owen Dreadnought — [All Turns] Reptile/Dragonkin observer no-op placeholder.
///
/// The printed effect: "When your Digimon with [Reptile] or [Dragonkin] trait is played
/// or digivolves, by suspending this Tamer, Draw 1. Then 1 Progress Digimon gets +3000 DP."
///
/// This function is a no-op placeholder pending resolution of the following hybrid gap:
///
/// **Engine gap**: `OnEnterFieldAnyone` and `OnDigivolve` observer `TriggerContext` does
/// not expose the entering/digivolving permanent to observer permanents. The context's
/// `target_permanent` points to the observer (Owen) itself, not the card that just
/// entered the field.  Additionally, `GameEvent::Digivolve` is not yet emitted, blocking
/// even event-log approaches for the digivolve half.
///
/// **DSL gap**: No `entering_permanent_trait_has` / `digivolving_permanent_trait_has`
/// BoolPredicate leaf exists. Once the engine threads the entering permanent through
/// `TriggerContext`, a matching predicate would enable native DSL expression.
///
/// Tracked in `qa/dsl-vocab-gaps.md` under `entering_permanent_trait_has`.
fn ex11_054_all_turns_noop(_handle: crate::card_source::CardHandle) -> Vec<Effect> {
    // No-op: returns an empty effect list.
    // The real logic is pending engine + DSL gap closure.
    vec![]
}

pub fn build_registry() -> EngineRawRustRegistry {
    let mut r = EngineRawRustRegistry::new();
    r.register_step("ex11_012_return_trash_to_deck_bottom", ex11_012_return_trash_to_deck_bottom);
    r.register_declarative("ex11_054_all_turns_noop", ex11_054_all_turns_noop);
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
