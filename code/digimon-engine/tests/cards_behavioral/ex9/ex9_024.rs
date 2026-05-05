//! EX9-024 Hanimon.
//! Printed text covered here: [On Play] by trashing 1 card in your hand,
//! you may return 1 Puppet Digimon card from trash to hand.
//!
//! Inherited: [Opponent's Turn] [Once Per Turn] when an opponent's Digimon
//! attacks, by deleting this Digimon, end the attack.

use digimon_engine::action::space::{PASS, PLAY_HAND_START};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::selection::SelectionKind;

#[test]
fn ex9_024_on_play_trashes_hand_card_and_returns_puppet_from_trash() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX9-024")
        .expect("EX9-024 YAML loads")
        .add_card(make_puppet("PUPPET-TRASH"))
        .add_card(make_filler("DISCARD"))
        .hand(0, &["EX9-024", "DISCARD"])
        .memory(10)
        .start();
    push_to_trash(&mut runner, "PUPPET-TRASH");

    runner.play(0, 0).expect("play Hanimon");

    let discard_view = runner.pending_selection_view().expect("discard prompt");
    assert_eq!(discard_view.kind, SelectionKind::Hand);
    assert!(runner.pending_is_optional(), "cost payment can be declined");
    assert_eq!(
        discard_view.valid_action_ids,
        vec![PLAY_HAND_START],
        "only the remaining hand card is discardable"
    );
    runner
        .execute_action(0, PLAY_HAND_START)
        .expect("trash hand card");

    let trash_view = runner
        .pending_selection_view()
        .expect("Puppet trash recursion prompt");
    assert_eq!(trash_view.kind, SelectionKind::Trash);
    assert!(
        runner.pending_is_optional(),
        "return-to-hand selection is optional"
    );
    runner
        .execute_action(0, trash_view.valid_action_ids[0])
        .expect("return Puppet");
    runner.auto_resolve().expect("finish effect");

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(hand_ids.contains(&"PUPPET-TRASH".to_string()));
    assert!(!hand_ids.contains(&"DISCARD".to_string()));

    let trash_ids = zone_ids(&runner.game.players[0].trash, &runner.game.card_data);
    assert!(trash_ids.contains(&"DISCARD".to_string()));
    assert!(!trash_ids.contains(&"PUPPET-TRASH".to_string()));
}

#[test]
fn ex9_024_decline_discard_does_not_return_trash_card() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX9-024")
        .expect("EX9-024 YAML loads")
        .add_card(make_puppet("PUPPET-TRASH"))
        .add_card(make_filler("DISCARD"))
        .hand(0, &["EX9-024", "DISCARD"])
        .memory(10)
        .start();
    push_to_trash(&mut runner, "PUPPET-TRASH");

    runner.play(0, 0).expect("play Hanimon");
    runner.execute_action(0, PASS).expect("decline discard");
    runner.auto_resolve().expect("finish decline");

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert_eq!(hand_ids, vec!["DISCARD".to_string()]);
    let trash_ids = zone_ids(&runner.game.players[0].trash, &runner.game.card_data);
    assert_eq!(trash_ids, vec!["PUPPET-TRASH".to_string()]);
}

#[test]
fn ex9_024_inherited_may_delete_carrier_to_end_opponent_attack() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX9-024")
        .expect("EX9-024 YAML loads")
        .add_card(make_filler("CARRIER"))
        .add_card(make_filler("ATTACKER"))
        .add_card(make_filler("SECURITY"))
        .security(0, &["SECURITY"])
        .start();
    runner.place_stack(0, &["EX9-024", "CARRIER"]);
    let attacker = runner.place_on_field(1, "ATTACKER", Some(0));
    runner.end_turn();

    runner.attack_player(attacker, 0, false);

    let view = runner
        .pending_selection_view()
        .expect("attack-cancel choice");
    assert_eq!(view.kind, SelectionKind::EffectChoice);
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("choose to delete carrier and end attack");
    runner.auto_resolve().expect("finish attack cancel");

    assert_eq!(runner.security_count(0), 1, "security was not checked");
    assert!(
        runner.game.players[0].battle_area.is_empty(),
        "carrier was deleted as the printed cost"
    );
    assert!(
        runner.game.pending_attack.is_none(),
        "attack state is fully cleared"
    );
}

fn make_puppet(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.traits = vec!["Puppet".to_string()];
    card
}

fn make_filler(id: &str) -> CardData {
    make_test_card(id, id)
}

fn push_to_trash(runner: &mut DebugRunner, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|card| card.card_id == card_id)
        .expect("card exists");
    let src = CardSource::new(data_idx, 0, runner.game.next_card_index());
    runner.game.players[0].trash.push(src);
}

fn zone_ids(cards: &[CardSource], data: &[digimon_engine::card_data::CardData]) -> Vec<String> {
    cards
        .iter()
        .map(|card| card.card_id(data).to_string())
        .collect()
}
