//! EX6-022 Angewomon.

use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::HAND_EFFECT_START;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming, Keyword, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

const YAML: &str = include_str!("../../../cards/ex6/EX6-022.yaml");

#[test]
fn ex6_022_with_mirei_gives_opponent_security_attack_minus_two() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("EX6-022 YAML loads")
        .add_card(tamer("MIREI", "Mirei Mikagura"))
        .add_card(digimon("TARGET", "Target"))
        .start();
    runner.place_on_field(0, "MIREI", Some(0));
    let target = runner.place_on_field(1, "TARGET", Some(0));
    let angewomon = runner.place_on_field(0, "EX6-022", Some(0));

    fire(&mut runner, EffectTiming::OnPlay, angewomon);
    let target_pick = runner
        .pending_selection_view()
        .expect("opponent Digimon selection")
        .valid_action_ids[0];
    assert_mask_allows(&runner, 0, target_pick);
    runner
        .execute_action(0, target_pick)
        .expect("choose opponent Digimon");
    runner.auto_resolve().expect("modifier resolves");

    assert_eq!(
        runner
            .game
            .modifiers
            .sum(target, ModifierType::SecurityAttackChange),
        -2
    );
}

#[test]
fn ex6_022_without_mirei_may_play_mirei_from_hand() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("EX6-022 YAML loads")
        .add_card(tamer("MIREI-HAND", "Mirei Mikagura"))
        .hand(0, &["MIREI-HAND"])
        .memory(0)
        .start();
    let angewomon = runner.place_on_field(0, "EX6-022", Some(0));

    fire(&mut runner, EffectTiming::OnPlay, angewomon);
    assert_mask_allows(&runner, 0, HAND_EFFECT_START);
    runner
        .execute_action(0, HAND_EFFECT_START)
        .expect("play Mirei from hand");
    runner.auto_resolve().expect("play resolves");

    assert!(battle_area_contains(&runner, 0, "MIREI-HAND"));
    assert_eq!(runner.memory(), 0, "Mirei is played without paying cost");
}

#[test]
fn ex6_022_inherited_grants_alliance_to_angel_carrier() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("EX6-022 YAML loads")
        .add_card(trait_digimon("ANGEL", "Angel"))
        .add_card(trait_digimon("OTHER", "Beast"))
        .start();
    let angel = runner.place_stack(0, &["EX6-022", "ANGEL"]);
    let other = runner.place_stack(0, &["EX6-022", "OTHER"]);
    runner.game.tick_declarative_effects();

    assert!(runner.game.has_keyword(angel, Keyword::Alliance));
    assert!(!runner.game.has_keyword(other, Keyword::Alliance));
}

fn fire(runner: &mut DebugRunner, timing: EffectTiming, handle: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(timing, TriggerSource::Permanent(handle));
    runner.game.drain_effect_queue();
}

fn digimon(id: &str, name: &str) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Digimon;
    card
}

fn trait_digimon(id: &str, trait_name: &str) -> CardData {
    let mut card = digimon(id, id);
    card.traits = vec![trait_name.to_string()];
    card
}

fn tamer(id: &str, name: &str) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Tamer;
    card.play_cost = 5;
    card
}

fn battle_area_contains(runner: &DebugRunner, player: u8, id: &str) -> bool {
    runner.game.players[player as usize]
        .battle_area
        .iter()
        .any(|permanent| permanent.top_card().card_id(&runner.game.card_data) == id)
}

fn assert_mask_allows(runner: &DebugRunner, player: u8, action: u16) {
    assert_eq!(
        build_action_mask(&runner.game, player)[action as usize],
        1.0,
        "pending action {action} must be legal in the action mask"
    );
}
