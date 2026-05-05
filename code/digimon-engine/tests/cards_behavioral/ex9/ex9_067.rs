//! EX9-067 Mirai Kinosaki.
//! Printed text covered here: [On Play] Reveal the top 3 cards of your deck.
//! Add 1 card with the [Puppet] or [LIBERATOR] trait among them to the hand.
//! Return the rest to the bottom of the deck.
//!
//! Security: Play this card without paying the cost.
//!
//! The [Your Turn] digivolve observer is left out until effect-initiated
//! digivolve/play context and effect-play cleanup are proven for Puppet Tamers.

use digimon_engine::action::space::SEL_REVEAL_START;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};

#[test]
fn ex9_067_on_play_adds_one_puppet_or_liberator_and_bottoms_rest() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX9-067")
        .expect("EX9-067 YAML loads")
        .add_card(make_trait_card("PUPPET", &["Puppet"]))
        .add_card(make_trait_card("LIBERATOR", &["LIBERATOR"]))
        .add_card(make_test_card("BLANK", "Blank"))
        .deck(0, &["BLANK", "LIBERATOR", "PUPPET"])
        .hand(0, &["EX9-067"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Mirai");

    let view = runner.pending_selection_view().expect("reveal pick");
    assert!(
        !runner.pending_is_optional(),
        "printed add-1 search is mandatory when a match exists"
    );
    assert!(
        !view
            .valid_action_ids
            .contains(&digimon_engine::action::space::PASS),
        "PASS is not legal for the mandatory add-1 search"
    );
    assert!(view
        .valid_action_ids
        .contains(&revealed_action_for_id(&runner, "PUPPET")));
    assert!(view
        .valid_action_ids
        .contains(&revealed_action_for_id(&runner, "LIBERATOR")));
    runner
        .execute_action(0, revealed_action_for_id(&runner, "PUPPET"))
        .expect("pick Puppet");
    runner.auto_resolve().expect("bottom remainder");

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(hand_ids.contains(&"PUPPET".to_string()));
    assert!(!hand_ids.contains(&"LIBERATOR".to_string()));
    assert_eq!(
        runner.game.players[0]
            .deck
            .first()
            .unwrap()
            .card_id(&runner.game.card_data),
        "BLANK",
        "unchosen matching cards return to deck bottom with the rest"
    );
}

#[test]
fn ex9_067_security_plays_itself_without_paying_cost() {
    let mut attacker = make_test_card("ATTACKER", "Attacker");
    attacker.level = Some(4);
    attacker.dp = Some(9000);

    let mut runner = DebugRunner::builder()
        .dsl_card("EX9-067")
        .expect("EX9-067 YAML loads")
        .add_card(attacker)
        .security(1, &["EX9-067"])
        .memory(10)
        .start();
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));

    runner.attack_player(attacker, 1, false);
    runner.auto_resolve().expect("resolve security play");

    assert!(
        runner.game.players[1]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "EX9-067"),
        "Mirai is played from the defender's security"
    );
}

fn make_trait_card(id: &str, traits: &[&str]) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.traits = traits.iter().map(|s| s.to_string()).collect();
    card
}

fn revealed_action_for_id(runner: &DebugRunner, id: &str) -> u16 {
    runner
        .game
        .revealed_cards
        .iter()
        .enumerate()
        .find_map(|(idx, card)| {
            (card.card_id(&runner.game.card_data) == id).then_some(SEL_REVEAL_START + idx as u16)
        })
        .expect("revealed card exists")
}

fn zone_ids(
    cards: &[digimon_engine::card_source::CardSource],
    data: &[digimon_engine::card_data::CardData],
) -> Vec<String> {
    cards
        .iter()
        .map(|card| card.card_id(data).to_string())
        .collect()
}
