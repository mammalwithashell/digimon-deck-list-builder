//! ST5 option cards: Laser Eye and Dark Side Attack.

use digimon_engine::action::space::encode_attack;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::CardKind;
use digimon_engine::selection::SelectionKind;

fn digimon(id: &str, name: &str, level: u8, play_cost: u16) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.play_cost = play_cost;
    card.dp = Some(3000);
    card
}

/// A cost-10 / level-6 attacker for P0. The [Security] tests check the option
/// from P1's security stack with a REAL attack, so the [Security] body runs
/// through `SecurityRevealed` (the battle-area scan skips `security` clauses).
/// The option's owner is P1; its opponent is P0.
fn security_attacker() -> digimon_engine::card_data::CardData {
    let mut card = digimon("SEC-ATK", "Security Attacker", 6, 10);
    card.dp = Some(9000);
    card
}

fn stack_sources(runner: &DebugRunner, player: usize, field_index: usize) -> usize {
    runner.game.players[player].battle_area[field_index]
        .card_sources
        .len()
}

#[test]
fn st5_15_main_de_digivolves_up_to_two_opponent_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST5-15")
        .expect("ST5-15 YAML parses")
        .add_card(digimon("ST5-15-A3", "Laser Eye Target A3", 3, 3))
        .add_card(digimon("ST5-15-A4", "Laser Eye Target A4", 4, 5))
        .add_card(digimon("ST5-15-B3", "Laser Eye Target B3", 3, 3))
        .add_card(digimon("ST5-15-B4", "Laser Eye Target B4", 4, 5))
        .hand(0, &["ST5-15"])
        .build();
    runner.place_stack(1, &["ST5-15-A3", "ST5-15-A4"]);
    runner.place_stack(1, &["ST5-15-B3", "ST5-15-B4"]);

    assert!(runner.game.activate_hand_main(0, 0));
    assert_eq!(runner.pending_kind(), Some(SelectionKind::OppField));
    let first = runner.pending_selection_view().unwrap().valid_action_ids[0];
    runner
        .execute_action(0, first)
        .expect("select first Laser Eye target");
    assert_eq!(runner.pending_kind(), Some(SelectionKind::OppField));
    let second = runner.pending_selection_view().unwrap().valid_action_ids[0];
    runner
        .execute_action(0, second)
        .expect("select second Laser Eye target");
    runner.auto_resolve().expect("finish Laser Eye");

    assert_eq!(stack_sources(&runner, 1, 0), 1);
    assert_eq!(stack_sources(&runner, 1, 1), 1);
}

#[test]
fn st5_15_security_activates_laser_eye_main_effect() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST5-15")
        .expect("ST5-15 YAML parses")
        .add_card(digimon("ST5-15-SEC3", "Laser Eye Security Target 3", 3, 3))
        .add_card(digimon("ST5-15-SEC4", "Laser Eye Security Target 4", 4, 5))
        .add_card(security_attacker())
        .security(1, &["ST5-15"])
        .start();
    let stack = runner.place_stack(0, &["ST5-15-SEC3", "ST5-15-SEC4"]);
    let attacker = runner.place_on_field(0, "SEC-ATK", Some(0));

    runner.attack_player(attacker, 1, false);

    assert_eq!(runner.pending_kind(), Some(SelectionKind::OppField));
    let view = runner.pending_selection_view().unwrap();
    assert_eq!(view.selecting_player, 1, "the option's owner picks");
    let pick = encode_attack(0, stack.index as u16);
    assert!(view.valid_action_ids.contains(&pick));
    runner
        .execute_action(1, pick)
        .expect("select security Laser Eye target");
    runner.auto_resolve().expect("finish Laser Eye security");

    assert_eq!(runner.security_count(1), 0, "ST5-15 was checked");
    assert_eq!(stack_sources(&runner, 0, stack.index as usize), 1);
}

#[test]
fn st5_16_main_deletes_one_opponent_digimon_with_play_cost_seven_or_less() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST5-16")
        .expect("ST5-16 YAML parses")
        .add_card(digimon("ST5-16-LOW", "Dark Side Low", 4, 7))
        .add_card(digimon("ST5-16-HIGH", "Dark Side High", 5, 8))
        .hand(0, &["ST5-16"])
        .build();
    runner.place_on_field(1, "ST5-16-LOW", Some(0));
    runner.place_on_field(1, "ST5-16-HIGH", Some(0));

    assert!(runner.game.activate_hand_main(0, 0));
    assert_eq!(runner.pending_kind(), Some(SelectionKind::OppField));
    assert_eq!(
        runner
            .pending_selection_view()
            .unwrap()
            .valid_action_ids
            .len(),
        1,
        "play_cost_lte: 7 excludes the cost-8 Digimon"
    );
    let pick = runner.pending_selection_view().unwrap().valid_action_ids[0];
    runner
        .execute_action(0, pick)
        .expect("delete cost-7 Digimon");

    assert_eq!(runner.battle_area_size(1), 1);
}

#[test]
fn st5_16_security_activates_dark_side_attack_main_effect() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST5-16")
        .expect("ST5-16 YAML parses")
        .add_card(digimon("ST5-16-SEC", "Dark Side Security Target", 4, 7))
        .add_card(security_attacker())
        .security(1, &["ST5-16"])
        .start();
    runner.place_on_field(0, "ST5-16-SEC", Some(0));
    let attacker = runner.place_on_field(0, "SEC-ATK", Some(0));

    runner.attack_player(attacker, 1, false);

    assert_eq!(runner.pending_kind(), Some(SelectionKind::OppField));
    let view = runner.pending_selection_view().unwrap();
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "play_cost_lte: 7 excludes the cost-10 attacker"
    );
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("delete security target");
    runner
        .auto_resolve()
        .expect("finish Dark Side Attack security");

    let survivors: Vec<String> = runner.game.players[0]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&runner.game.card_data).to_string())
        .collect();
    assert_eq!(survivors, vec!["SEC-ATK".to_string()]);
}

#[test]
fn st5_15_main_de_digivolve_targets_up_to_two_and_allows_zero() {
    // ST5-15 Laser Eye prints "<De-Digivolve 1> UP TO 2 of your opponent's
    // Digimon" (the per-card JSON drops the "up to"; DCGO's description + its
    // canEndNotMax behaviour confirm it). Per the "up to N" rule (same as ST1-15
    // Giga Destroyer / ST5-12 / ST6-12) the player may pick 0, 1, or 2 — the
    // selection is optional-at-zero, and the De-Digivolve amount is 1 per target.
    let mut runner = DebugRunner::builder()
        .dsl_card("ST5-15")
        .expect("ST5-15 YAML parses")
        .add_card(digimon("ST5-15-A3", "Laser Eye Target A3", 3, 3))
        .add_card(digimon("ST5-15-A4", "Laser Eye Target A4", 4, 5))
        .hand(0, &["ST5-15"])
        .build();
    runner.place_stack(1, &["ST5-15-A3", "ST5-15-A4"]);
    runner.place_stack(1, &["ST5-15-A3", "ST5-15-A4"]);

    assert!(runner.game.activate_hand_main(0, 0));
    let view = runner
        .pending_selection_view()
        .expect("ST5-15 installs an up-to-2 De-Digivolve selection");
    assert_eq!(
        view.kind,
        SelectionKind::OppField,
        "caps the De-Digivolve at 2 targets"
    );
    assert!(
        view.is_optional,
        "\"up to 2\" lets the player pick zero targets"
    );
    assert_eq!(
        view.valid_action_ids.len(),
        2,
        "both opponent Digimon are eligible De-Digivolve targets"
    );
}
