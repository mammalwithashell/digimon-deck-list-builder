//! BT9-103 Kongou
//!
//! Printed text:
//! - [Main] Until end of your opponent's next turn, your opponent's Digimon
//!   with play cost 7 or less can't attack players. Your opponent can't
//!   add cards to security by effects.
//! - [Security] Activate this card's Main effects.
//!
//! Phase 2 Track E (2026-05-17): BT9-103's YAML uses `add_player_modifier`
//! for the opponent security-add lock and `for_each + add_modifier` to
//! apply `CannotAttackPlayer` to every opponent Digimon matching the
//! play-cost ≤ 7 filter. Both modifiers expire at end of opponent's turn.
//!
//! Reuses the bilateral player-scoped modifier pattern demonstrated by
//! BT14-009 (player flood gate) and ST13-08 (player effect lock). This
//! test verifies the BT9-103 Main clause body installs both modifiers
//! when executed against an opponent board.

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledStep, CompiledTiming};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::ModifierType;

#[test]
fn bt9_103_has_main_and_security_mirror() {
    let runner = DebugRunner::builder()
        .dsl_card("BT9-103")
        .expect("BT9-103 must load from embedded DSL pack")
        .memory(5)
        .start();
    let card = runner.compiled_card("BT9-103").expect("compiled card");

    assert!(
        card.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::MainFromHand)
        )),
        "BT9-103 must have a Main clause"
    );
    assert!(
        card.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSecurity)
        )),
        "BT9-103 must have a Security mirror clause"
    );
}

/// Structural: BT9-103 Main clause body must contain both
/// `add_player_modifier` (for `CannotAddSecurityByEffect`) and a
/// `for_each + add_modifier` loop (for the per-Digimon
/// `CannotAttackPlayer`).
#[test]
fn bt9_103_main_clause_emits_both_player_and_permanent_modifiers() {
    let runner = DebugRunner::builder()
        .dsl_card("BT9-103")
        .expect("BT9-103 YAML parses and compiles")
        .build();
    let card = runner.compiled_card("BT9-103").expect("compiled card");

    let main = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.scope == CompiledScope::FaceUp
                    && t.when.contains(&CompiledTiming::MainFromHand) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("Main clause present");

    let has_player_modifier = main
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::AddPlayerModifier { .. }));
    let has_for_each_add_modifier = main.process.iter().any(|s| match s {
        CompiledStep::ForEach { body, .. } => body
            .iter()
            .any(|inner| matches!(inner, CompiledStep::AddModifier { .. })),
        _ => false,
    });

    assert!(
        has_player_modifier,
        "Main clause must include add_player_modifier (CannotAddSecurityByEffect on opponent)"
    );
    assert!(
        has_for_each_add_modifier,
        "Main clause must include for_each + add_modifier (CannotAttackPlayer on filtered opponent Digimon)"
    );
}

/// Behavioral: when BT9-103's Main clause body runs, the opponent gains a
/// `CannotAddSecurityByEffect` player modifier AND every opponent Digimon
/// with play cost ≤ 7 gains a `CannotAttackPlayer` permanent modifier.
/// Higher-cost opponent Digimon stay unaffected.
#[test]
fn bt9_103_main_installs_modifiers_on_opponent_and_low_cost_digimon() {
    // Two opponent Digimon: one at cost 5 (should gain CannotAttackPlayer),
    // one at cost 10 (should NOT be affected).
    let mut low = make_test_card("OPP-LOW", "OPP-LOW");
    low.play_cost = 5;
    let mut high = make_test_card("OPP-HIGH", "OPP-HIGH");
    high.play_cost = 10;
    let bt9_holder = make_test_card("BT9_103_HOLDER", "Kongou Holder");

    let mut runner = DebugRunner::builder()
        .dsl_card("BT9-103")
        .expect("BT9-103 YAML parses and compiles")
        .add_card(low.clone())
        .add_card(high.clone())
        .add_card(bt9_holder)
        .build();

    // Place opponent Digimon on the field.
    let low_perm = runner.place_on_field(1, "OPP-LOW", None);
    let high_perm = runner.place_on_field(1, "OPP-HIGH", None);

    // Place a stand-in source card for the Option effect — the engine
    // needs a CardHandle to anchor `source_card` on EffectContext.
    let bt9_holder_perm = runner.place_on_field(0, "BT9_103_HOLDER", None);
    let src_handle = runner.top_card(bt9_holder_perm);

    // Extract BT9-103's Main clause process and run it via run_steps.
    let compiled = runner.compiled_card("BT9-103").expect("compiled").clone();
    let main_process = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::MainFromHand) => {
                Some(t.process.clone())
            }
            _ => None,
        })
        .expect("Main process present");

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_handle, None, 0);
        run_steps(&main_process, &mut ctx, &mut Bindings::new());
    }

    // Opponent player has CannotAddSecurityByEffect.
    assert!(
        runner
            .game
            .modifiers
            .player_has(1, ModifierType::CannotAddSecurityByEffect),
        "Opponent must have CannotAddSecurityByEffect after BT9-103 Main resolves"
    );

    // OPP-LOW (cost 5) has CannotAttackPlayer; OPP-HIGH (cost 10) does not.
    assert!(
        runner
            .game
            .modifiers
            .has(low_perm, ModifierType::CannotAttackPlayer),
        "OPP-LOW (cost 5) must gain CannotAttackPlayer (play_cost_lte: 7)"
    );
    assert!(
        !runner
            .game
            .modifiers
            .has(high_perm, ModifierType::CannotAttackPlayer),
        "OPP-HIGH (cost 10) must NOT gain CannotAttackPlayer — outside the cost-≤7 filter"
    );
}
