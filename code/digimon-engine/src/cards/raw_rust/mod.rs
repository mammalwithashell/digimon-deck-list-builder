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

// BT13-040 Magnamon — REMOVED 2026-06-23 (make-engine-cloneable task 3.3).
// `bt13_040_may_play_veemon_from_hand_or_source` was a raw_rust step that
// installed a CLOSURE-based `pending_selection` (UnionZone over hand+material →
// play a Veemon free) — the only clone-UNSAFE raw_rust code. It was already DEAD:
// BT13-040.yaml uses the pure-DSL `select_union_zone` + `play_union_bound_free`
// path (now on the resumable VM via the flipped `install_select_union_zone`), and
// no YAML/test referenced this fn. Deleted to remove the clone-unsafe pattern.

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
    // EX8-070 (security delete lowest-play-cost) and BT24-062 (inherited
    // attack-target lock) were migrated off raw_rust to pure DSL —
    // `select_opponent_permanent { selector: lowest_play_cost }` and
    // `flood_gate { target: { is_source_permanent: true }, scope: both }`
    // respectively (fix-dsl-substrate-rot-and-bugs).
    // bt13_040_may_play_veemon_from_hand_or_source removed 2026-06-23 (dead +
    // clone-unsafe; BT13-040 uses pure-DSL select_union_zone — see above).
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
