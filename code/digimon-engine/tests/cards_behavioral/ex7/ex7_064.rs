use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming, Keyword};
use digimon_engine::selection::TriggerSource;

fn digimon(id: &str, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.traits = traits.iter().map(|trait_name| trait_name.to_string()).collect();
    card
}

#[test]
fn ex7_064_start_main_gains_memory_when_opponent_has_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX7-064")
        .expect("EX7-064 YAML loads")
        .add_card(digimon("OPP", &[]))
        .memory(0)
        .start();
    let shoto = runner.place_on_field(0, "EX7-064", Some(0));
    runner.place_on_field(1, "OPP", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::StartOfYourMainPhase,
        TriggerSource::Permanent(shoto),
    );
    runner.game.drain_effect_queue();

    assert_eq!(runner.memory(), 1);
}

#[test]
fn ex7_064_end_turn_suspends_tamer_grants_keywords_and_unsuspends_vortex_warrior() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX7-064")
        .expect("EX7-064 YAML loads")
        .add_card(digimon("VW", &["Vortex Warriors"]))
        .memory(10)
        .start();
    let shoto = runner.place_on_field(0, "EX7-064", Some(0));
    let target = runner.place_on_field(0, "VW", Some(0));
    runner.game.players[0].battle_area[target.index as usize].is_suspended = true;

    runner
        .game
        .enqueue_triggered(EffectTiming::EndOfYourTurn, TriggerSource::Permanent(shoto));
    runner.game.drain_effect_queue();
    let prompt = runner.pending_selection_view().expect("optional suspend prompt");
    runner
        .execute_action(prompt.selecting_player, prompt.valid_action_ids[0])
        .expect("accept Shoto trigger");
    let target_prompt = runner.pending_selection_view().expect("target pick");
    runner
        .execute_action(target_prompt.selecting_player, target_prompt.valid_action_ids[0])
        .expect("choose Vortex Warriors Digimon");

    assert!(runner.game.players[0].battle_area[shoto.index as usize].is_suspended);
    assert!(!runner.game.players[0].battle_area[target.index as usize].is_suspended);
    assert!(runner.game.has_keyword(target, Keyword::Piercing));
    assert!(runner.game.has_keyword(target, Keyword::Blocker));
}
