//! EX6-074 Mirei Mikagura.

use digimon_engine::action::build_action_mask;
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

const YAML: &str = include_str!("../../../cards/ex6/EX6-074.yaml");

#[test]
fn ex6_074_on_ally_played_suspends_gains_memory_and_digivolves_from_trash() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("EX6-074 YAML loads")
        .add_card(lv4("BASE", "Base"))
        .add_card(lv5("ANGE", "Angewomon"))
        .add_card(holy_beast("HOLY", "Holy Beast"))
        .hand(0, &["HOLY"])
        .memory(5)
        .start();
    runner.place_on_field(0, "EX6-074", Some(0));
    let base = runner.place_on_field(0, "BASE", Some(0));
    push_to_trash(&mut runner, 0, "ANGE");

    runner.play(0, 0).expect("play Holy Beast");
    let accept = runner
        .pending_selection_view()
        .expect("Mirei optional trigger")
        .valid_action_ids[0];
    assert_mask_allows(&runner, 0, accept);
    runner.accept_optional_trigger().expect("accept Mirei");
    runner.auto_resolve().expect("finish digivolve");

    assert_eq!(
        runner.game.players[base.player as usize].battle_area[base.index as usize]
            .top_card()
            .card_id(&runner.game.card_data),
        "ANGE"
    );
    assert_eq!(runner.memory(), 4, "gain 1, then pay reduced cost 2");
}

#[test]
fn ex6_074_has_security_play_and_end_of_turn_dna_registration() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("EX6-074 YAML loads")
        .start();
    let compiled = runner
        .compiled_card("EX6-074")
        .expect("compiled card present");

    assert!(compiled.effects.iter().any(|clause| {
        matches!(
            clause,
            digimon_dsl::compiled::CompiledClause::Triggered(t)
                if t.when.contains(&digimon_dsl::compiled::CompiledTiming::OnSecurity)
        )
    }));
    assert!(compiled.effects.iter().any(|clause| {
        matches!(
            clause,
            digimon_dsl::compiled::CompiledClause::Declarative(
                digimon_dsl::compiled::CompiledDeclarativeClause::AltPathRegistration { .. }
            )
        )
    }));
}

fn holy_beast(id: &str, name: &str) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Digimon;
    card.traits = vec!["Holy Beast".to_string()];
    card.play_cost = 0;
    card
}

fn lv4(id: &str, name: &str) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.colors = vec![CardColor::Yellow];
    card
}

fn lv5(id: &str, name: &str) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Digimon;
    card.level = Some(5);
    card.colors = vec![CardColor::Yellow];
    card.evo_costs = vec![EvoCost {
        card_color: CardColor::Yellow as u8,
        level: 4,
        memory_cost: 3,
    }];
    card
}

fn push_to_trash(runner: &mut DebugRunner, player: usize, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("{card_id} not found in card_data"));
    let next = runner.game.next_card_index();
    runner.game.players[player]
        .trash
        .push(CardSource::new(data_idx, player as u8, next));
}

fn assert_mask_allows(runner: &DebugRunner, player: u8, action: u16) {
    assert_eq!(
        build_action_mask(&runner.game, player)[action as usize],
        1.0,
        "pending action {action} must be legal in the action mask"
    );
}
