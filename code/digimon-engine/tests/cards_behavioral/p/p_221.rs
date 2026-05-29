//! P-221 Chaosmon.

use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming, ModifierType};
use digimon_engine::selection::TriggerSource;

#[test]
fn p_221_when_attacking_applies_minus_ten_thousand_dp() {
    let mut runner = DebugRunner::builder()
        .dsl_card("P-221")
        .expect("P-221 loads")
        .add_card(digimon("TARGET", 6, 12000))
        .memory(5)
        .start();
    let chaos = runner.place_on_field(0, "P-221", Some(0));
    let target = runner.place_on_field(1, "TARGET", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(chaos));
    runner.game.drain_effect_queue();
    let view = runner.pending_selection_view().expect("DP target");
    runner
        .execute_action(view.selecting_player, first_non_pass(&view.valid_action_ids))
        .expect("select target");
    runner.auto_resolve().expect("finish Chaosmon");

    assert_eq!(
        runner.game.modifiers.sum(target, ModifierType::ChangeDp),
        -10000
    );
}

#[test]
fn p_221_has_security_attack_plus_and_partition() {
    let runner = DebugRunner::builder()
        .dsl_card("P-221")
        .expect("P-221 loads")
        .start();
    let card = runner.compiled_card("P-221").expect("compiled card");

    assert!(card.effects.iter().any(|clause| matches!(
        clause,
        digimon_dsl::compiled::CompiledClause::Declarative(
            digimon_dsl::compiled::CompiledDeclarativeClause::GrantKeyword { keyword, value, .. }
        ) if keyword == "SecurityAttackPlus" && *value == Some(1)
    )));
    assert!(card.effects.iter().any(|clause| matches!(
        clause,
        digimon_dsl::compiled::CompiledClause::Declarative(
            digimon_dsl::compiled::CompiledDeclarativeClause::Partition { .. }
        )
    )));
}

fn first_non_pass(ids: &[u16]) -> u16 {
    ids.iter()
        .copied()
        .find(|id| *id != digimon_engine::action::space::PASS)
        .expect("non-pass action")
}

fn digimon(id: &str, level: u8, dp: i32) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card
}
