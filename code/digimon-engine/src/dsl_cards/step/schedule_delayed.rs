//! Phase 2f4 Task 3 — `ScheduleDelayed` step lowering.
//!
//! Translates `CompiledStep::ScheduleDelayed { when, body }` into a call
//! to `EffectContext::schedule_delayed` (Task 1's primitive), which appends
//! a `ScheduledEffect` to the engine queue. The body fires when the engine
//! reaches the matching timing boundary (Task 2 wired the drain at
//! `EndOfYourTurn`, `EndOfOpponentsTurn`, `EndOfBattle`, and `EndOfAttack`).
//!
//! Bindings are cloned at schedule time — subsequent mutations to the
//! caller's `bindings` after this step do NOT affect the captured copy.
//! The body slice is also cloned (the queue entry takes ownership).
//!
//! Unknown / virtual `CompiledTiming` variants (e.g. `Delayed`,
//! `OnAllyPlayed`) silently no-op — `compiled_timing_to_engine` returns
//! `None` for them, matching the engine convention for degenerate inputs.
//! The validator should reject unmappable timings upstream.

use digimon_dsl::compiled::CompiledStep;

use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::step::StepRuntime;
use crate::dsl_cards::timing_map::compiled_timing_to_engine;
use crate::effect_context::EffectContext;

/// Returns `true` if `step` is the `ScheduleDelayed` variant — including
/// the no-op case where the timing doesn't map to an engine `EffectTiming`,
/// since the variant *was* matched (consistent with `play_digivolve`'s
/// match-vs-effect convention).
pub fn try_run(
    step: &CompiledStep,
    ctx: &mut EffectContext<'_>,
    bindings: &Bindings,
    runtime: &StepRuntime,
) -> bool {
    match step {
        CompiledStep::ScheduleDelayed { when, body } => {
            if let Some(t) = compiled_timing_to_engine(*when) {
                ctx.schedule_delayed_with_runtime(
                    t,
                    body.clone(),
                    bindings.clone(),
                    runtime.clone(),
                );
            }
            true
        }
        _ => false,
    }
}
