//! P-101 Raremon.
//! Printed text covered here:
//! [On Play] [When Digivolving] By trashing 1 card from your hand, Draw 2.
//! Inherited: [When Attacking] By trashing 1 card from your hand, delete 1 of
//! your opponent's level 3 Digimon.

use digimon_engine::action::space::{PASS, PLAY_HAND_START};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming};
use digimon_engine::selection::{SelectionKind, TriggerSource};

#[test]
fn p_101_on_play_trashes_hand_card_and_draws_two() {
    let mut runner = DebugRunner::builder()
        .dsl_card("P-101")
        .expect("P-101 YAML loads")
        .add_card(make_digimon("DISCARD", 3, 1000))
        .add_card(make_digimon("DRAW-1", 3, 1000))
        .add_card(make_digimon("DRAW-2", 3, 1000))
        .deck(0, &["DRAW-2", "DRAW-1"])
        .hand(0, &["P-101", "DISCARD"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Raremon");

    let view = runner.pending_selection_view().expect("discard prompt");
    assert_eq!(view.kind, SelectionKind::Hand);
    assert!(runner.pending_is_optional(), "cost payment can be declined");
    assert_eq!(
        view.valid_action_ids,
        vec![PLAY_HAND_START],
        "only the remaining hand card is discardable after P-101 leaves hand"
    );

    runner
        .execute_action(0, PLAY_HAND_START)
        .expect("trash hand card");
    runner.auto_resolve().expect("draw two");

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(hand_ids.contains(&"DRAW-1".to_string()));
    assert!(hand_ids.contains(&"DRAW-2".to_string()));
    assert!(!hand_ids.contains(&"DISCARD".to_string()));

    let trash_ids = zone_ids(&runner.game.players[0].trash, &runner.game.card_data);
    assert!(trash_ids.contains(&"DISCARD".to_string()));
}

#[test]
fn p_101_on_play_declining_discard_does_not_draw() {
    let mut runner = DebugRunner::builder()
        .dsl_card("P-101")
        .expect("P-101 YAML loads")
        .add_card(make_digimon("DISCARD", 3, 1000))
        .add_card(make_digimon("DRAW-1", 3, 1000))
        .deck(0, &["DRAW-1"])
        .hand(0, &["P-101", "DISCARD"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Raremon");
    runner.execute_action(0, PASS).expect("decline discard");
    runner.auto_resolve().expect("finish decline");

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert_eq!(hand_ids, vec!["DISCARD".to_string()]);
    assert!(runner.game.players[0].trash.is_empty());
}

#[test]
fn p_101_when_digivolving_trashes_hand_card_and_draws_two() {
    let mut runner = DebugRunner::builder()
        .dsl_card("P-101")
        .expect("P-101 YAML loads")
        .add_card(make_digimon("BASE", 3, 1000))
        .add_card(make_digimon("DISCARD", 3, 1000))
        .add_card(make_digimon("DRAW-1", 3, 1000))
        .add_card(make_digimon("DRAW-2", 3, 1000))
        .deck(0, &["DRAW-2", "DRAW-1"])
        .hand(0, &["DISCARD"])
        .memory(10)
        .start();

    let raremon = runner.place_stack(0, &["BASE", "P-101"]);
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(raremon),
    );
    runner.game.drain_effect_queue();

    let view = runner.pending_selection_view().expect("discard prompt");
    assert_eq!(view.kind, SelectionKind::Hand);
    assert!(runner.pending_is_optional(), "cost payment can be declined");
    runner
        .execute_action(0, PLAY_HAND_START)
        .expect("trash hand card");
    runner.auto_resolve().expect("draw two");

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(hand_ids.contains(&"DRAW-1".to_string()));
    assert!(hand_ids.contains(&"DRAW-2".to_string()));

    let trash_ids = zone_ids(&runner.game.players[0].trash, &runner.game.card_data);
    assert!(trash_ids.contains(&"DISCARD".to_string()));
}

#[test]
fn p_101_inherited_when_attacking_trashes_hand_card_and_deletes_opponent_level_3() {
    let mut runner = DebugRunner::builder()
        .dsl_card("P-101")
        .expect("P-101 YAML loads")
        .add_card(make_digimon("CARRIER", 4, 5000))
        .add_card(make_digimon("DISCARD", 3, 1000))
        .add_card(make_digimon("OPP-LV3", 3, 2000))
        .add_card(make_digimon("OPP-LV4", 4, 4000))
        .hand(0, &["DISCARD"])
        .memory(10)
        .start();
    runner.game.players[1].security.clear();

    let carrier = runner.place_stack(0, &["P-101", "CARRIER"]);
    runner.place_on_field(1, "OPP-LV3", Some(0));
    runner.place_on_field(1, "OPP-LV4", Some(0));

    runner.attack_player(carrier, 1, false);

    let discard = runner.pending_selection_view().expect("discard prompt");
    assert_eq!(discard.kind, SelectionKind::Hand);
    assert!(runner.pending_is_optional(), "cost payment can be declined");
    runner
        .execute_action(0, PLAY_HAND_START)
        .expect("trash hand card");

    let target = runner
        .pending_selection_view()
        .expect("opponent level 3 target prompt");
    assert_eq!(target.kind, SelectionKind::OppField);
    assert_eq!(
        runner.pending_action_count(),
        1,
        "only the opponent's level 3 Digimon is a legal delete target"
    );
    runner
        .execute_action(0, target.valid_action_ids[0])
        .expect("delete level 3");
    runner.auto_resolve().expect("finish inherited effect");

    let opp_field = field_ids(&runner, 1);
    assert!(!opp_field.contains(&"OPP-LV3".to_string()));
    assert!(opp_field.contains(&"OPP-LV4".to_string()));

    let own_trash = zone_ids(&runner.game.players[0].trash, &runner.game.card_data);
    assert!(own_trash.contains(&"DISCARD".to_string()));
    let opp_trash = zone_ids(&runner.game.players[1].trash, &runner.game.card_data);
    assert!(opp_trash.contains(&"OPP-LV3".to_string()));
}

#[test]
fn p_101_inherited_when_attacking_does_not_prompt_without_level_3_target() {
    let mut runner = DebugRunner::builder()
        .dsl_card("P-101")
        .expect("P-101 YAML loads")
        .add_card(make_digimon("CARRIER", 4, 5000))
        .add_card(make_digimon("DISCARD", 3, 1000))
        .add_card(make_digimon("OPP-LV4", 4, 4000))
        .hand(0, &["DISCARD"])
        .memory(10)
        .start();
    runner.game.players[1].security.clear();

    let carrier = runner.place_stack(0, &["P-101", "CARRIER"]);
    runner.place_on_field(1, "OPP-LV4", Some(0));

    runner.attack_player(carrier, 1, false);
    runner.auto_resolve().expect("finish attack");

    assert!(
        runner.pending_selection().is_none(),
        "no opponent level 3 means no discard-cost prompt should be installed"
    );
    let own_trash = zone_ids(&runner.game.players[0].trash, &runner.game.card_data);
    assert!(!own_trash.contains(&"DISCARD".to_string()));
    assert!(field_ids(&runner, 1).contains(&"OPP-LV4".to_string()));
}

fn make_digimon(id: &str, level: u8, dp: i32) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card
}

fn zone_ids(cards: &[CardSource], data: &[CardData]) -> Vec<String> {
    cards
        .iter()
        .map(|card| card.card_id(data).to_string())
        .collect()
}

fn field_ids(runner: &DebugRunner, player: usize) -> Vec<String> {
    runner.game.players[player]
        .battle_area
        .iter()
        .map(|perm| perm.top_card().card_id(&runner.game.card_data).to_string())
        .collect()
}
