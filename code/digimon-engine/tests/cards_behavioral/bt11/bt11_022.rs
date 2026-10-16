//! BT11-022: official observation ef6be7e5949581e869ad22a85ba4e39c4101b82ad22c0033b3ba4328b0b0d0a0,
//! printed image, and base DCGO BT11/Blue/BT11_022.cs.
use digimon_engine::action::{build_action_mask, space::encode_digivolve};
use digimon_engine::debug_runner::{
    make_test_card, make_test_egg, DebugRunner, DebugRunnerBuilder,
};
use digimon_engine::enums::{CardColor, CardKind};
const ID: &str = "BT11-022";
fn base() -> DebugRunnerBuilder {
    let mut flare = make_test_card("FLARE", "Other Name");
    flare.traits = vec!["Blue Flare".into()];
    let mut tamer = make_test_card("TAMER", "Dramon Tamer");
    tamer.card_kind = CardKind::Tamer;
    tamer.traits = vec!["Blue Flare".into()];
    let mut blue = make_test_egg("BLUE", "Blue Egg");
    blue.colors = vec![CardColor::Blue];
    let mut champion = make_test_card("CHAMPION", "Plain Champion");
    champion.level = Some(4);
    champion.evo_costs = vec![digimon_engine::card_data::EvoCost {
        card_color: CardColor::Blue as u8,
        level: 3,
        memory_cost: 0,
    }];
    DebugRunner::builder()
        .from_dsl_yaml(include_str!("../../../cards/bt11/BT11-022.yaml"))
        .expect("BT11-022 must be implemented")
        .add_card(make_test_card("DRAMON", "Coredramon"))
        .add_card(make_test_card("PLAIN", "Plain"))
        .add_card(flare)
        .add_card(tamer)
        .add_card(blue)
        .add_card(champion)
        .add_card(make_test_egg("BEBY", "Bebydomon"))
        .add_card(make_test_egg("WRONG", "Bebydomon Variant"))
        .deck(0, &["PLAIN"; 20])
        .deck(1, &["PLAIN"; 20])
        .memory(5)
}
fn setup(inherited: bool) -> DebugRunner {
    let mut r = base().start();
    r.skip_mulligan();
    if inherited {
        r.place_stack(0, &[ID, "DRAMON"]);
    } else {
        r.place_on_field(0, ID, Some(0));
    }
    r
}
fn play_event(r: &mut DebugRunner, player: u8, id: &str) {
    let h = r.place_on_field(player, id, Some(0));
    r.fire_play_event_triggers(player, h.index as usize, true, false);
    r.game.drain_effect_queue();
    r.auto_resolve().unwrap();
}
#[test]
fn both_name_and_trait_trigger_face_draw_or_inherited_memory() {
    for inherited in [false, true] {
        for card in ["DRAMON", "FLARE"] {
            let mut r = setup(inherited);
            let h = r.place_on_field(0, card, Some(0));
            r.fire_play_event_triggers(0, h.index as usize, true, false);
            r.game.drain_effect_queue();
            assert!(
                r.pending_selection().is_none(),
                "mandatory reward cannot offer a decline"
            );
            assert_eq!(
                r.hand_size(0),
                if inherited { 0 } else { 1 },
                "{card}/{inherited}"
            );
            assert_eq!(r.memory(), if inherited { 6 } else { 5 });
            assert!(
                r.pending_selection().is_none(),
                "mandatory effect has no optional prompt"
            );
        }
    }
}
#[test]
fn unrelated_opponent_tamer_and_self_do_not_trigger_or_spend_opt() {
    for inherited in [false, true] {
        let mut r = setup(inherited);
        r.fire_play_event_triggers(0, 0, true, false);
        r.game.drain_effect_queue();
        r.auto_resolve().unwrap();
        play_event(&mut r, 0, "PLAIN");
        play_event(&mut r, 1, "DRAMON");
        play_event(&mut r, 0, "TAMER");
        assert_eq!(r.hand_size(0), 0);
        assert_eq!(r.memory(), 5);
        play_event(&mut r, 0, "FLARE");
        assert_eq!(r.hand_size(0), if inherited { 0 } else { 1 });
        assert_eq!(r.memory(), if inherited { 6 } else { 5 });
    }
}
#[test]
fn once_per_turn_resets_and_opponents_turn_does_not_trigger() {
    for inherited in [false, true] {
        let mut r = setup(inherited);
        play_event(&mut r, 0, "DRAMON");
        play_event(&mut r, 0, "FLARE");
        assert_eq!(r.hand_size(0), if inherited { 0 } else { 1 });
        assert_eq!(r.memory(), if inherited { 6 } else { 5 });
        r.end_turn();
        r.auto_resolve().unwrap();
        let before = (r.hand_size(0), r.memory());
        play_event(&mut r, 0, "DRAMON");
        assert_eq!((r.hand_size(0), r.memory()), before);
        r.end_turn();
        r.auto_resolve().unwrap();
        let before = (r.hand_size(0), r.memory());
        play_event(&mut r, 0, "FLARE");
        assert_eq!(r.hand_size(0), before.0 + if inherited { 0 } else { 1 });
        assert_eq!(r.memory(), before.1 + if inherited { 1 } else { 0 });
    }
}
#[test]
fn simultaneous_face_and_inherited_observers_both_resolve_after_clone() {
    let mut r = setup(false);
    r.place_stack(0, &[ID, "PLAIN"]);
    let mut clone = DebugRunner::wrap(r.game.clone());
    for game in [&mut r, &mut clone] {
        play_event(game, 0, "DRAMON");
        assert_eq!(game.hand_size(0), 1);
        assert_eq!(game.memory(), 6);
        play_event(game, 0, "FLARE");
        assert_eq!(game.hand_size(0), 1);
        assert_eq!(game.memory(), 6);
    }
}
#[test]
fn own_play_is_not_another_even_though_dracomon_has_blue_flare() {
    let mut r = base().hand(0, &[ID]).start();
    r.skip_mulligan();
    r.play(0, 0).unwrap();
    r.auto_resolve().unwrap();
    assert_eq!(r.hand_size(0), 0);
    assert_eq!(r.memory(), 2);
    play_event(&mut r, 0, "FLARE");
    assert_eq!(r.hand_size(0), 1);
}
#[test]
fn same_copy_face_and_inherited_have_independent_once_per_turn_uses() {
    let mut r = setup(false);
    play_event(&mut r, 0, "DRAMON");
    let hand = r.add_to_hand(0, "CHAMPION");
    let action = encode_digivolve(hand as u16, 0);
    assert_eq!(build_action_mask(&r.game, 0)[action as usize], 1.0);
    r.game.decode_action(action, 0);
    let cards = r.hand_size(0);
    play_event(&mut r, 0, "FLARE");
    assert_eq!(
        r.memory(),
        6,
        "inherited OPT remains available after face OPT"
    );
    assert_eq!(
        r.hand_size(0),
        cards,
        "face draw no longer active under a top card"
    );
    play_event(&mut r, 0, "DRAMON");
    assert_eq!(r.memory(), 6);
}
#[test]
fn normal_blue_and_exact_bebydomon_routes_are_zero_cost() {
    for (egg, legal) in [("BLUE", true), ("BEBY", true), ("WRONG", false)] {
        let mut r = base().hand(0, &[ID]).start();
        r.skip_mulligan();
        let h = r.place_on_field(0, egg, Some(0));
        let action = encode_digivolve(0, h.index as u16);
        assert_eq!(
            build_action_mask(&r.game, 0)[action as usize] > 0.0,
            legal,
            "{egg}"
        );
        if legal {
            r.game.decode_action(action, 0);
            assert_eq!(r.memory(), 5);
            assert_eq!(r.hand_size(0), 1, "evolution bonus only");
            assert_eq!(
                r.game.players[0].battle_area[0]
                    .top_card()
                    .card_id(&r.game.card_data),
                ID
            );
        }
    }
}
