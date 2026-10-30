//! LM-045: printed face and official bundle verified; real Option use and Delay.
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{
    EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_MAIN, FIELD_EFFECT_START, PASS,
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::OptionState;
use digimon_engine::selection::OptionPlayResult;
fn card(id: &str, color: CardColor, kind: CardKind) -> digimon_engine::CardData {
    let mut c = make_test_card(id, id);
    c.colors = vec![color];
    c.card_kind = kind;
    c
}
fn fixture(deck: &[&str]) -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("LM-045")
        .expect("registered LM-045")
        .add_card(card("R", CardColor::Red, CardKind::Digimon))
        .add_card(card("Y", CardColor::Yellow, CardKind::Digimon))
        .add_card(card("B", CardColor::Blue, CardKind::Digimon))
        .add_card(card("T", CardColor::Yellow, CardKind::Tamer))
        .add_card(card("RT", CardColor::Red, CardKind::Tamer))
        .add_card(card("O", CardColor::Yellow, CardKind::Option))
        .hand(0, &["LM-045"])
        .deck(0, deck)
        .deck(1, &["B"; 8])
        .memory(10)
        .start()
}
fn seated(r: &DebugRunner) -> usize {
    let places: Vec<_> = r.game.players[0]
        .battle_area
        .iter()
        .enumerate()
        .filter(|(_, p)| p.top_card().card_id(&r.game.card_data) == "LM-045")
        .collect();
    assert_eq!(places.len(), 1);
    assert!(matches!(
        places[0].1.option_state,
        OptionState::Delayed { .. }
    ));
    assert!(!r.game.players[0]
        .trash
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == "LM-045"));
    places[0].0
}
#[test]
fn lm_045_normal_use_color_permission() {
    for (id, owner, breeding, legal) in [
        ("R", 0, false, true),
        ("Y", 0, false, true),
        ("T", 0, false, true),
        ("Y", 0, true, true),
        ("RT", 0, false, true),
        ("R", 0, true, true),
        ("B", 0, false, false),
        ("Y", 1, false, false),
        ("O", 0, false, false),
    ] {
        let mut r = fixture(&[]);
        if breeding {
            r.place_in_breeding(owner, id);
        } else {
            r.place_on_field(owner, id, Some(0));
        }
        r.game.enter_main_phase();
        assert_eq!(
            build_action_mask(&r.game, 0)[0],
            if legal { 1.0 } else { 0.0 }
        );
        let result = r.game.play_option_from_hand(0, 0);
        assert_eq!(
            result != OptionPlayResult::Invalid,
            legal,
            "{id} owner {owner} breeding {breeding}: {result:?}"
        );
    }
}
#[test]
fn lm_045_search_all_red_yellow_choices_and_both_bottom_orders() {
    for selected in 0..2 {
        for order in 0..2 {
            let mut r = fixture(&["B", "T", "Y", "R"]);
            r.place_on_field(0, "R", Some(0));
            r.game.enter_main_phase();
            assert_eq!(
                r.game.play_option_from_hand(0, 0),
                OptionPlayResult::Pending
            );
            let v = r.pending_selection_view().unwrap();
            assert_eq!(v.valid_action_ids.len(), 2);
            assert_eq!(build_action_mask(&r.game, 0)[PASS as usize], 0.0);
            for a in &v.valid_action_ids {
                assert_eq!(build_action_mask(&r.game, 0)[*a as usize], 1.0);
            }
            r.execute_action(0, v.valid_action_ids[selected]).unwrap();
            let v = r
                .pending_selection_view()
                .expect("player orders two remaining cards");
            assert_eq!(v.valid_action_ids.len(), 2);
            r.execute_action(0, v.valid_action_ids[order]).unwrap();
            r.auto_resolve().unwrap();
            let deck_ids: Vec<_> = r.game.players[0]
                .deck
                .iter()
                .map(|c| c.card_id(&r.game.card_data))
                .collect();
            let other = if selected == 0 { "Y" } else { "R" };
            assert_eq!(
                deck_ids,
                if order == 0 {
                    vec!["T", other, "B"]
                } else {
                    vec![other, "T", "B"]
                }
            );
            assert_eq!(r.hand_size(0), 1);
            let picked = r.game.players[0].hand[0].card_id(&r.game.card_data);
            assert_eq!(picked, if selected == 0 { "R" } else { "Y" });
            seated(&r);
            assert_eq!(r.memory(), 7);
            assert_eq!(r.deck_size(0), 3);
        }
    }
}
#[test]
fn lm_045_no_eligible_empty_and_short_deck_still_seat() {
    for deck in [vec![], vec!["B"], vec!["T", "O", "B"], vec!["Y"]] {
        let mut r = fixture(&deck);
        r.place_on_field(0, "R", Some(0));
        r.game.enter_main_phase();
        r.game.play_option_from_hand(0, 0);
        r.auto_resolve().unwrap();
        seated(&r);
        assert_eq!(r.memory(), 7);
        assert_eq!(r.hand_size(0), usize::from(deck.contains(&"Y")));
    }
}
#[test]
fn lm_045_delay_later_main_explicit_consumed_once() {
    let mut r = fixture(&["B"; 8]);
    r.place_on_field(0, "R", Some(0));
    r.game.enter_main_phase();
    r.game.play_option_from_hand(0, 0);
    r.auto_resolve().unwrap();
    let idx = seated(&r);
    let bit = FIELD_EFFECT_START + idx as u16 * EFFECTS_PER_PERMANENT + FIELD_EFFECT_SLOT_FOR_MAIN;
    assert_eq!(build_action_mask(&r.game, 0)[bit as usize], 0.0);
    r.end_turn();
    r.game.enter_main_phase();
    r.end_turn();
    r.game.enter_main_phase();
    r.game.set_memory(0);
    let mask = build_action_mask(&r.game, 0);
    assert_eq!(mask[bit as usize], 1.0);
    assert_eq!(mask[PASS as usize], 1.0);
    r.game.decode_action(bit, 0);
    assert_eq!(r.memory(), 2);
    assert_eq!(r.trash_size(0), 1);
    assert_eq!(build_action_mask(&r.game, 0)[bit as usize], 0.0);
    r.game.decode_action(bit, 0);
    assert_eq!(r.memory(), 2);
}
#[test]
fn lm_045_security_seats_without_search_or_memory() {
    let mut r = DebugRunner::builder()
        .dsl_card("LM-045")
        .unwrap()
        .add_card(card("R", CardColor::Red, CardKind::Digimon))
        .security(1, &["LM-045"])
        .deck(1, &["R"; 5])
        .memory(0)
        .start();
    let a = r.place_on_field(0, "R", Some(0));
    r.attack_player(a, 1, false);
    assert_eq!(r.deck_size(1), 5);
    assert_eq!(r.hand_size(1), 0);
    assert_eq!(r.memory(), 0);
    assert_eq!(r.trash_size(1), 0);
    assert_eq!(r.game.players[1].battle_area.len(), 1);
    assert!(matches!(
        r.game.players[1].battle_area[0].option_state,
        OptionState::Delayed { .. }
    ));
}

#[test]
fn lm_045_every_physical_duplicate_is_selectable() {
    for pick in 0..3 {
        let mut r = fixture(&["Y", "R", "R"]);
        r.place_on_field(0, "R", Some(0));
        r.game.enter_main_phase();
        r.game.play_option_from_hand(0, 0);
        let v = r.pending_selection_view().unwrap();
        assert_eq!(v.valid_action_ids.len(), 3);
        assert_eq!(build_action_mask(&r.game, 0)[PASS as usize], 0.0);
        r.execute_action(0, v.valid_action_ids[pick]).unwrap();
        r.auto_resolve().unwrap();
        assert_eq!(r.hand_size(0), 1);
        assert_eq!(r.deck_size(0), 2);
        seated(&r);
    }
}
#[test]
fn lm_045_all_six_no_match_remainder_orders() {
    let mut outcomes = std::collections::BTreeSet::new();
    for first in 0..3 {
        for second in 0..2 {
            let mut r = fixture(&["T", "O", "B"]);
            r.place_on_field(0, "R", Some(0));
            r.game.enter_main_phase();
            r.game.play_option_from_hand(0, 0);
            let v = r.pending_selection_view().unwrap();
            assert_eq!(v.valid_action_ids.len(), 3);
            r.execute_action(0, v.valid_action_ids[first]).unwrap();
            let v = r.pending_selection_view().unwrap();
            assert_eq!(v.valid_action_ids.len(), 2);
            r.execute_action(0, v.valid_action_ids[second]).unwrap();
            r.auto_resolve().unwrap();
            seated(&r);
            outcomes.insert(
                r.game.players[0]
                    .deck
                    .iter()
                    .map(|c| c.card_id(&r.game.card_data).to_string())
                    .collect::<Vec<_>>(),
            );
        }
    }
    assert_eq!(outcomes.len(), 6, "every bottom order is reachable");
}
#[test]
fn lm_045_yellow_waiver_does_not_authorize_other_red_option() {
    let mut r = DebugRunner::builder()
        .dsl_card("LM-045")
        .unwrap()
        .add_card(card("Y", CardColor::Yellow, CardKind::Digimon))
        .add_card(card("OTHER", CardColor::Red, CardKind::Option))
        .hand(0, &["LM-045", "OTHER"])
        .memory(10)
        .start();
    r.place_on_field(0, "Y", Some(0));
    r.game.enter_main_phase();
    assert_eq!(
        r.game.play_option_from_hand(0, 1),
        OptionPlayResult::Invalid
    );
    assert_ne!(
        r.game.play_option_from_hand(0, 0),
        OptionPlayResult::Invalid
    );
    r.auto_resolve().unwrap();
    seated(&r);
}
