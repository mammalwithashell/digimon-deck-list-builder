//! LM-043 Darkdramon.

use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming};
use digimon_engine::selection::TriggerSource;

#[test]
fn lm_043_on_play_de_digivolves_then_deletes_lowest_play_cost_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card("LM-043")
        .expect("LM-043 loads")
        .add_card(digimon("BASE", 3, 3))
        .add_card(digimon("TOP", 5, 6))
        .add_card(digimon("LOW", 4, 3))
        .add_card(digimon("HIGH", 5, 8))
        .memory(5)
        .start();
    let darkdramon = runner.place_on_field(0, "LM-043", Some(0));
    let stack = runner.place_stack(1, &["BASE", "TOP"]);
    let _low = runner.place_on_field(1, "LOW", Some(0));
    let _high = runner.place_on_field(1, "HIGH", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(darkdramon));
    runner.game.drain_effect_queue();
    let view = runner.pending_selection_view().expect("de-digivolve target");
    runner
        .execute_action(view.selecting_player, encode_permanent(stack))
        .expect("select stack");
    runner.auto_resolve().expect("finish Darkdramon");

    let remaining: Vec<_> = runner.game.players[1]
        .battle_area
        .iter()
        .map(|perm| perm.top_card().card_id(&runner.game.card_data).to_string())
        .collect();
    assert_eq!(remaining, vec!["HIGH".to_string()]);
}

#[test]
fn lm_043_has_ace_overflow_blast_and_scapegoat_replacement() {
    let runner = DebugRunner::builder()
        .dsl_card("LM-043")
        .expect("LM-043 loads")
        .start();
    let card = runner.compiled_card("LM-043").expect("compiled card");

    assert_eq!(card.ace_overflow, Some(-4));
    assert!(card.effects.iter().any(|clause| matches!(
        clause,
        digimon_dsl::compiled::CompiledClause::Declarative(
            digimon_dsl::compiled::CompiledDeclarativeClause::GrantKeyword { keyword, .. }
        ) if keyword == "BlastDigivolve"
    )));
    assert!(card.effects.iter().any(|clause| matches!(
        clause,
        digimon_dsl::compiled::CompiledClause::Declarative(
            digimon_dsl::compiled::CompiledDeclarativeClause::Replacement { .. }
        )
    )));
}

fn encode_permanent(handle: digimon_engine::permanent::PermanentHandle) -> u16 {
    digimon_engine::action::space::encode_attack(0, handle.index as u16)
}

fn digimon(id: &str, level: u8, play_cost: u16) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(5000);
    card.play_cost = play_cost;
    card
}
