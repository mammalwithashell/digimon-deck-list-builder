//! BT11-023: official observation 21ecbd8c..., Q2066/Q2067 and card image.
use digimon_engine::action::space::{PASS, SEL_REVEAL_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

fn card(id: &str, name: &str, kind: CardKind, colors: Vec<CardColor>) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = kind;
    c.colors = colors;
    c.play_cost = 2;
    c
}
fn runner(deck: &[&str], hand: &[&str]) -> DebugRunner {
    // Callers specify top first; the builder stores the top at the vector end.
    let deck: Vec<_> = deck.iter().rev().copied().collect();
    DebugRunner::builder()
        .dsl_card("BT11-023")
        .expect("BT11-023 loads")
        .add_card(card(
            "V",
            "AeroVeedramon",
            CardKind::Digimon,
            vec![CardColor::Blue],
        ))
        .add_card(card(
            "T",
            "Blue Tamer",
            CardKind::Tamer,
            vec![CardColor::Blue, CardColor::Green],
        ))
        .add_card(card(
            "R",
            "Red Tamer",
            CardKind::Tamer,
            vec![CardColor::Red],
        ))
        .add_card(card(
            "N",
            "Veedramon Option",
            CardKind::Option,
            vec![CardColor::Blue],
        ))
        .add_card(card(
            "F",
            "Other Digimon",
            CardKind::Digimon,
            vec![CardColor::Blue],
        ))
        .deck(0, &deck)
        .hand(0, hand)
        .memory(10)
        .start()
}
fn pick(r: &mut DebugRunner, id: &str) {
    let index = r
        .game
        .revealed_cards
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == id)
        .unwrap();
    let action = SEL_REVEAL_START + index as u16;
    assert!(r
        .pending_selection_view()
        .unwrap()
        .valid_action_ids
        .contains(&action));
    r.execute_action(0, action).expect("legal reveal choice");
}
fn ids(r: &DebugRunner, hand: bool) -> Vec<String> {
    let zone = if hand {
        &r.game.players[0].hand
    } else {
        &r.game.players[0].deck
    };
    zone.iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}
#[test]
fn bt11_023_search_requires_both_buckets_and_bottoms_rest() {
    let mut r = runner(&["V", "T", "R", "F"], &["BT11-023"]);
    r.play(0, 0).unwrap();
    let view = r
        .pending_selection_view()
        .expect("mandatory Digimon bucket");
    assert_eq!(view.valid_action_ids, vec![SEL_REVEAL_START]);
    assert!(!view.valid_action_ids.contains(&PASS));
    pick(&mut r, "V");
    let view = r
        .pending_selection_view()
        .expect("mandatory blue Tamer bucket");
    assert_eq!(view.valid_action_ids.len(), 1);
    assert!(!view.valid_action_ids.contains(&PASS));
    pick(&mut r, "T");
    r.auto_resolve().unwrap();
    assert_eq!(ids(&r, true), vec!["V", "T"]);
    assert_eq!(ids(&r, false), vec!["R", "F"]);
}
#[test]
fn bt11_023_search_rejects_named_option_and_red_tamer_orders_remainder() {
    let mut r = runner(&["N", "R", "T", "F"], &["BT11-023"]);
    r.play(0, 0).unwrap();
    let view = r.pending_selection_view().expect("only blue Tamer legal");
    assert_eq!(view.valid_action_ids, vec![SEL_REVEAL_START + 2]);
    pick(&mut r, "T");
    let view = r
        .pending_selection_view()
        .expect("bottom order is player choice");
    assert_eq!(view.valid_action_ids.len(), 2);
    pick(&mut r, "R");
    r.auto_resolve().unwrap();
    assert_eq!(ids(&r, true), vec!["T"]);
    assert_eq!(ids(&r, false), vec!["N", "R", "F"]);
}
#[test]
fn bt11_023_inherited_blue_tamer_gain_is_opt_and_filters_kind_color() {
    let mut r = runner(&["F"], &["R", "F", "T", "T"]);
    r.place_stack(0, &["BT11-023", "V"]);
    r.play(0, 0).unwrap();
    r.auto_resolve().unwrap();
    assert_eq!(r.memory(), 8, "red Tamer does not consume OPT");
    r.play(0, 0).unwrap();
    r.auto_resolve().unwrap();
    assert_eq!(r.memory(), 6, "blue Digimon does not consume OPT");
    r.play(0, 0).unwrap();
    r.auto_resolve().unwrap();
    assert_eq!(r.memory(), 5, "multicolor blue Tamer gains 1");
    r.play(0, 0).unwrap();
    r.auto_resolve().unwrap();
    assert_eq!(r.memory(), 3, "second Tamer does not gain again");
}
#[test]
fn bt11_023_top_card_does_not_gain_inherited_memory() {
    let mut r = runner(&["F"], &["T"]);
    r.place_on_field(0, "BT11-023", None);
    r.play(0, 0).unwrap();
    r.auto_resolve().unwrap();
    assert_eq!(r.memory(), 8);
}

#[test]
fn bt11_023_opponent_tamer_does_not_trigger_inherited_effect() {
    let mut r = runner(&["F"], &["T"]);
    r.place_stack(1, &["BT11-023", "V"]);
    r.play(0, 0).unwrap();
    r.auto_resolve().unwrap();
    assert_eq!(r.memory(), 8, "opponent blue Tamer is not your play");
}
#[test]
fn bt11_023_search_single_bucket_short_deck_and_no_matches() {
    let mut r = runner(&["V"], &["BT11-023"]);
    r.play(0, 0).unwrap();
    pick(&mut r, "V");
    r.auto_resolve().unwrap();
    assert_eq!(ids(&r, true), vec!["V"]);
    assert!(ids(&r, false).is_empty());

    let mut r = runner(&["N", "R", "F"], &["BT11-023"]);
    r.play(0, 0).unwrap();
    assert_eq!(
        r.pending_selection_view().unwrap().valid_action_ids.len(),
        3,
        "with neither category matching, all three cards need ordering"
    );
    r.auto_resolve().unwrap();
    assert!(ids(&r, true).is_empty());
    assert_eq!(r.deck_size(0), 3);
}

#[test]
fn bt11_023_own_tamer_during_opponent_turn_does_not_gain_memory() {
    let mut r = runner(&["F"], &[]);
    r.place_stack(0, &["BT11-023", "V"]);
    r.set_first_player(1);
    assert_eq!(r.turn_player(), 1);
    let tamer = r.place_on_field(0, "T", None);
    let memory = r.memory();
    r.fire_play_event_triggers(0, tamer.index as usize, true, false);
    r.auto_resolve().unwrap();
    assert_eq!(
        r.memory(),
        memory,
        "own blue Tamer alone cannot satisfy Your Turn"
    );
}
#[test]
fn bt11_023_printed_blue_level2_digivolution_costs_zero() {
    use digimon_engine::enums::{GamePhase, PlaySource};
    let mut egg = card("EGG", "Blue Egg", CardKind::DigiEgg, vec![CardColor::Blue]);
    egg.level = Some(2);
    let mut r = DebugRunner::builder()
        .dsl_card("BT11-023")
        .unwrap()
        .add_card(egg)
        .add_card(make_test_card("DRAW", "Draw"))
        .hand(0, &["BT11-023"])
        .deck(0, &["DRAW"])
        .memory(5)
        .start();
    r.place_in_breeding(0, "EGG");
    r.set_phase(GamePhase::Main);
    assert!(
        r.game
            .digivolve_from_hand_onto_breeding(0, 0, PlaySource::ByDigivolve),
        "printed blue Lv2 route must be legal"
    );
    assert_eq!(r.memory(), 5, "printed evolution is free");
    assert_eq!(r.hand_size(0), 1, "evolution bonus draws exactly one");
}
