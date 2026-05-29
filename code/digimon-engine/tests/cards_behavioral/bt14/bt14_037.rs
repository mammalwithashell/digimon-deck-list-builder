//! BT14-037 MagnaAngemon.

use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming, ModifierType};
use digimon_engine::selection::TriggerSource;

#[test]
fn bt14_037_on_play_recovers_then_debuffs_by_security_count() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT14-037")
        .expect("BT14-037 loads")
        .add_card(make_test_card("RECOVER", "Recover"))
        .add_card(digimon("TARGET", 4, 7000))
        .deck(0, &["RECOVER"])
        .security(0, &["RECOVER", "RECOVER", "RECOVER", "RECOVER", "RECOVER"])
        .memory(5)
        .start();
    let ange = runner.place_on_field(0, "BT14-037", Some(0));
    let target = runner.place_on_field(1, "TARGET", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(ange));
    runner.game.drain_effect_queue();
    let view = runner.pending_selection_view().expect("DP target");
    runner
        .execute_action(view.selecting_player, first_non_pass(&view.valid_action_ids))
        .expect("select target");
    runner.auto_resolve().expect("finish MagnaAngemon");

    assert_eq!(runner.game.players[0].security.len(), 6);
    assert_eq!(
        runner.game.modifiers.sum(target, ModifierType::ChangeDp),
        -6000,
        "DP reduction should count security after recovery"
    );
}

#[test]
fn bt14_037_has_ace_overflow_and_blast_digivolve() {
    let runner = DebugRunner::builder()
        .dsl_card("BT14-037")
        .expect("BT14-037 loads")
        .start();
    let card = runner.compiled_card("BT14-037").expect("compiled card");

    assert_eq!(card.ace_overflow, Some(-3));
    assert!(
        card.effects.iter().any(|clause| matches!(
            clause,
            digimon_dsl::compiled::CompiledClause::Declarative(
                digimon_dsl::compiled::CompiledDeclarativeClause::GrantKeyword { keyword, .. }
            ) if keyword == "BlastDigivolve"
        )),
        "MagnaAngemon should grant Blast Digivolve"
    );
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
