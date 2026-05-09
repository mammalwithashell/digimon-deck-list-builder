//! EX10-003 Tumblemon
//!
//! Inherited Effect [Opponent's Turn] [Once Per Turn] When one of your
//! opponent's Digimon attacks, by trashing 3 [Mineral] or [Rock] trait cards
//! from this Digimon's digivolution cards, end that attack.

use digimon_engine::action::space::encode_source_select;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::CardData;

fn card_with_traits(id: &str, name: &str, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, name);
    card.traits = traits
        .iter()
        .map(|trait_name| trait_name.to_string())
        .collect();
    card
}

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX10-003")
        .expect("EX10-003 YAML parses and compiles")
        .add_card(card_with_traits("SRC-ROCK-1", "Rock Source 1", &["Rock"]))
        .add_card(card_with_traits(
            "SRC-MINERAL",
            "Mineral Source",
            &["Mineral"],
        ))
        .add_card(card_with_traits("SRC-ROCK-2", "Rock Source 2", &["Rock"]))
        .add_card(make_test_card("SRC-BAD", "Nonmatching Source"))
        .add_card(make_test_card("HOST", "Host Digimon"))
        .add_card(make_test_card("OTHER-HOST", "Other Host"))
        .add_card(make_test_card("ATTACKER", "Attacker"))
        .add_card(make_test_card("SECURITY", "Security"))
        .security(0, &["SECURITY"])
        .build()
}

#[test]
fn ex10_003_trashes_three_matching_carrier_sources_to_cancel_attack() {
    let mut runner = runner();
    let carrier = runner.place_stack(
        0,
        &[
            "EX10-003",
            "SRC-ROCK-1",
            "SRC-MINERAL",
            "SRC-ROCK-2",
            "SRC-BAD",
            "HOST",
        ],
    );
    let other = runner.place_stack(0, &["SRC-ROCK-1", "OTHER-HOST"]);
    let attacker = runner.place_on_field(1, "ATTACKER", Some(0));
    runner.end_turn();
    assert_eq!(runner.turn_player(), 1, "precondition: opponent's turn");

    runner.attack_player(attacker, 0, false);

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("Tumblemon source-cost prompt should be exposed");
    let self_source = encode_source_select(carrier.index as u16, 0).expect("self source action");
    let rock = encode_source_select(carrier.index as u16, 1).expect("rock source action");
    let mineral = encode_source_select(carrier.index as u16, 2).expect("mineral source action");
    let second_rock =
        encode_source_select(carrier.index as u16, 3).expect("second rock source action");
    let bad = encode_source_select(carrier.index as u16, 4).expect("bad source action");
    let other_rock = encode_source_select(other.index as u16, 0).expect("other stack source");
    assert!(pending.valid_action_ids.contains(&self_source));
    assert!(pending.valid_action_ids.contains(&rock));
    assert!(pending.valid_action_ids.contains(&mineral));
    assert!(pending.valid_action_ids.contains(&second_rock));
    assert!(!pending.valid_action_ids.contains(&bad));
    assert!(!pending.valid_action_ids.contains(&other_rock));

    runner
        .game
        .resolve_selection(0, self_source)
        .expect("pick EX10-003");
    runner
        .game
        .resolve_selection(0, rock)
        .expect("pick Rock source");
    runner
        .game
        .resolve_selection(0, mineral)
        .expect("pick Mineral source");

    assert!(runner.game.pending_attack.is_none(), "attack was cancelled");
    assert_eq!(runner.security_count(0), 1, "security was not checked");
    assert_eq!(runner.trash_size(0), 3, "three sources were paid");
    assert_eq!(
        runner.game.players[0].battle_area[other.index as usize]
            .card_sources
            .len(),
        2,
        "sources from other stacks were not touched"
    );
}
