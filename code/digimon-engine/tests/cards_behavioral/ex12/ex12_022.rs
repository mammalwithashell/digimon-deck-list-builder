use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

const CARD_ID: &str = "EX12-022";

fn blue_digimon(id: &str, traits: &[&str]) -> CardData {
    let mut card = make_test_card_with_level(id, id, 3);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Blue];
    card.play_cost = 3;
    card.dp = Some(3000);
    card.traits = traits
        .iter()
        .map(|trait_name| trait_name.to_string())
        .collect();
    card
}

fn option_with_trait(id: &str, trait_name: &str) -> CardData {
    let mut card = blue_digimon(id, &[trait_name]);
    card.card_kind = CardKind::Option;
    card.level = None;
    card.dp = None;
    card
}

fn hand_ids(runner: &DebugRunner, player: u8) -> Vec<String> {
    runner.game.players[player as usize]
        .hand
        .iter()
        .map(|card| card.card_id(&runner.game.card_data).to_string())
        .collect()
}

#[test]
fn ex12_022_on_play_reveals_three_adds_shambala_and_sw_cards_rest_to_bottom() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-022 YAML loads")
        .add_card(option_with_trait("SHAMBALA-OPTION", "Shambala"))
        .add_card(blue_digimon("SW-DIGIMON", &["SW"]))
        .add_card(blue_digimon("FILLER", &[]))
        .add_card(blue_digimon("PAD", &[]))
        .hand(0, &[CARD_ID])
        .deck(0, &["PAD", "SHAMBALA-OPTION", "SW-DIGIMON", "FILLER"])
        .memory(10)
        .start();
    let deck_before = runner.deck_size(0);

    assert!(runner.play(0, 0).is_some(), "EX12-022 should be playable");
    runner.auto_resolve().expect("resolve EX12-022 reveal");

    let hand = hand_ids(&runner, 0);
    assert!(
        hand.iter().any(|id| id == "SHAMBALA-OPTION"),
        "hand={hand:?}"
    );
    assert!(hand.iter().any(|id| id == "SW-DIGIMON"), "hand={hand:?}");
    assert!(
        !hand.iter().any(|id| id == "FILLER"),
        "non-matching revealed card must not be added: {hand:?}"
    );
    assert_eq!(runner.deck_size(0), deck_before - 2);
    assert_eq!(
        runner.game.players[0].deck[0].card_id(&runner.game.card_data),
        "FILLER",
        "unpicked reveal remainder returns to deck bottom"
    );
}

#[test]
fn ex12_022_inherited_when_attacking_draws_if_hand_has_7_or_fewer_cards() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-022 YAML loads")
        .add_card(blue_digimon("CARRIER", &[]))
        .add_card(blue_digimon("DRAW", &[]))
        .add_card(blue_digimon("SEC", &[]))
        .deck(0, &["DRAW"])
        .security(1, &["SEC"])
        .memory(5)
        .start();

    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    let hand_before = runner.hand_size(0);
    let deck_before = runner.deck_size(0);

    runner.attack_player(carrier, 1, false);
    runner
        .auto_resolve()
        .expect("resolve EX12-022 attack trigger");

    assert_eq!(runner.hand_size(0), hand_before + 1);
    assert_eq!(runner.deck_size(0), deck_before - 1);
}

#[test]
fn ex12_022_inherited_attack_hand_test_is_a_processing_condition_and_stays_orderable() {
    use digimon_engine::selection::SelectionKind;
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-022 YAML loads")
        .dsl_card("EX12-024")
        .expect("EX12-024 YAML loads")
        .add_card(blue_digimon("CARRIER", &[]))
        .add_card(blue_digimon("DRAW", &[]))
        .add_card(blue_digimon("SEC", &[]))
        .add_card(blue_digimon("FILL", &[]))
        .hand(0, &["FILL"; 8])
        .deck(0, &["DRAW"; 4])
        .security(1, &["SEC"])
        .memory(5)
        .start();

    let carrier = runner.place_stack(0, &[CARD_ID, "EX12-024", "CARRIER"]);
    runner.attack_player(carrier, 1, false);

    let pending = runner
        .pending_selection()
        .expect("both inherited [When Attacking] effects are orderable");
    assert_eq!(pending.kind, SelectionKind::TriggerOrder);
    let ex12_022_action = pending
        .effect_choices
        .as_ref()
        .and_then(|choices| choices.iter().find(|c| c.label.starts_with(CARD_ID)))
        .map(|c| c.action_id)
        .expect("EX12-022 entry must be in TriggerOrder at 8 hand cards");
    let hand_before = runner.hand_size(0);
    runner
        .execute_action(0, ex12_022_action)
        .expect("choose EX12-022");
    assert_eq!(
        runner.hand_size(0),
        hand_before + 1,
        "8 cards in hand: EX12-022 draws nothing (only sibling EX12-024 draws)"
    );
    assert_eq!(
        runner.game.opt_used(carrier),
        1,
        "EX12-022's OPT stays unspent after unmet hand count"
    );
}
