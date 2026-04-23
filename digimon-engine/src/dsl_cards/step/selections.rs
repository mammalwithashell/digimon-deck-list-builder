//! Selection-step lowering: install a `PendingSelection` with the
//! remainder of the process-step slice as its callback.
//!
//! Phase 2b: concrete handlers for `SelectHand` / `SelectTrash` /
//! `SelectOwnPermanent` / `SelectOpponentPermanent` land in Task 4.

use digimon_dsl::compiled::CompiledStep;

use crate::dsl_cards::bindings::Bindings;
use crate::effect_context::EffectContext;

/// Returns `true` if `step` was a selection step and the remainder was
/// installed as its callback. Returns `false` for any non-selection
/// step, letting `run_steps` fall through to the synchronous path.
pub fn try_install(
    _step: &CompiledStep,
    _tail: &[CompiledStep],
    _ctx: &mut EffectContext<'_>,
    _bindings: Bindings,
) -> bool {
    false // Task 4 adds the first handler.
}
