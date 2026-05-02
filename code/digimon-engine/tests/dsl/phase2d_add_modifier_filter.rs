//! Phase 2d Task 8: AddModifier with CompiledModifierTarget::Filter
//! evaluates the predicate against every battle-area permanent and
//! applies the modifier to each match.

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledModifierTarget, CompiledModifierValue, CompiledPredicate,
    CompiledStep,
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::ModifierType;

#[test]
fn add_modifier_filter_targets_every_match() {
    // P0 has 2 digimon, P1 has 1. Filter: { kind: digimon }. Expect all 3
    // to receive the CannotBeAffected modifier with end_of_turn expiry.
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .add_card(make_test_card("D1", "D1"))
        .add_card(make_test_card("D2", "D2"))
        .add_card(make_test_card("D3", "D3"))
        .hand(0, &["SRC"])
        .build();

    runner.place_on_field(0, "D1", None);
    runner.place_on_field(0, "D2", None);
    runner.place_on_field(1, "D3", None);

    let src_card = runner.game.players[0].hand[0].handle();

    let steps = vec![CompiledStep::AddModifier {
        target: CompiledModifierTarget::Filter(CompiledPredicate {
            kind: Some(CompiledCardKind::Digimon),
            ..CompiledPredicate::default()
        }),
        modifier: "CannotBeAffected".to_string(),
        value: CompiledModifierValue::Literal(0),
        expiry: "end_of_turn".to_string(),
    }];

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        let mut bindings = Bindings::new();
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    // Each battle-area permanent should now carry CannotBeAffected.
    let mut total = 0;
    for player_idx in 0..runner.game.players.len() {
        for i in 0..runner.game.players[player_idx].battle_area.len() {
            let h = runner.perm_handle(player_idx as u8, i);
            if runner.game.modifiers.has(h, ModifierType::CannotBeAffected) {
                total += 1;
            }
        }
    }
    assert_eq!(total, 3, "all 3 digimon should carry CannotBeAffected");
}
