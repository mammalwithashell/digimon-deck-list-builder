use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

fn yellow_vaccine(id: &str, level: u8) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.colors = vec![CardColor::Yellow];
    card.traits = vec!["Vaccine".to_string()];
    card.evo_costs = vec![EvoCost {
        card_color: 2,
        level: 3,
        memory_cost: 3,
    }];
    card
}

fn tk_tamer() -> CardData {
    let mut card = make_test_card("BT1-087", "T.K. Takaishi");
    card.card_kind = CardKind::Tamer;
    card.colors = vec![CardColor::Yellow];
    card
}

#[test]
fn bt14_093_security_search_digivolves_and_recovers_with_tk_tamer() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT14-093")
        .expect("BT14-093 YAML loads")
        .add_card(yellow_vaccine("BASE", 3))
        .add_card(yellow_vaccine("EVO", 4))
        .add_card(tk_tamer())
        .add_card(make_test_card("RECOVERY", "Recovery"))
        .hand(0, &["BT14-093"])
        .security(0, &["EVO"])
        .deck(0, &["RECOVERY"])
        .memory(10)
        .start();
    let base = runner.place_on_field(0, "BASE", Some(0));
    runner.place_on_field(0, "BT1-087", Some(0));

    assert!(runner.game.activate_hand_main(0, 0));
    let security_pick = runner.pending_selection_view().expect("security evo pick");
    runner
        .execute_action(security_pick.selecting_player, security_pick.valid_action_ids[0])
        .expect("pick EVO from security");
    let target_pick = runner.pending_selection_view().expect("digivolve target pick");
    runner
        .execute_action(target_pick.selecting_player, target_pick.valid_action_ids[0])
        .expect("pick BASE");
    runner.auto_resolve().expect("finish option");

    assert_eq!(
        runner.game.players[0].battle_area[base.index as usize]
            .top_card()
            .card_id(&runner.game.card_data),
        "EVO"
    );
    assert_eq!(runner.game.players[0].security.len(), 1);
}

#[test]
fn bt14_093_security_effect_can_play_patamon_then_adds_self_to_hand() {
    let runner = DebugRunner::builder()
        .dsl_card("BT14-093")
        .expect("BT14-093 YAML loads")
        .build();
    let card = runner
        .game
        .card_data
        .iter()
        .find(|card| card.card_id == "BT14-093")
        .expect("card data");
    assert_eq!(card.card_kind, CardKind::Option);
}
