//! Frozen 13-archetype pool: declaration spends Main OPT even when its body
//! is declined. Search clones must preserve the same counter and legal choices.
use digimon_engine::{
    action::{
        build_action_mask,
        space::{EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_MAIN, FIELD_EFFECT_START, PASS},
    },
    debug_runner::{make_test_card, DebugRunner},
    enums::{CardColor, CardKind, GamePhase},
    serialization::to_ui_json,
};

fn main_action(index: u16) -> u16 {
    FIELD_EFFECT_START + index * EFFECTS_PER_PERMANENT + FIELD_EFFECT_SLOT_FOR_MAIN
}

fn assert_decline_spends_use(card: &str, from_trash: bool) {
    let mut target = make_test_card("TARGET", "Eligible Tamer");
    target.card_kind = CardKind::Tamer;
    target.level = None;
    target.dp = None;
    target.colors = vec![CardColor::White];
    target.traits = ["VB", "SW", "TB", "TS"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let mut r = DebugRunner::builder()
        .dsl_card(card)
        .unwrap()
        .add_card(target)
        .hand(0, &["TARGET"])
        .memory(10)
        .start();
    r.set_first_player(0);
    r.place_on_field(0, card, Some(0));
    r.place_on_field(0, card, Some(0));
    if from_trash {
        let target = r.game.players[0].hand.pop().unwrap();
        r.game.players[0].trash.push(target);
    }
    r.game.enter_main_phase();
    let before = r.game.clone();
    assert!(
        build_action_mask(&r.game, 0)[main_action(0) as usize] > 0.0,
        "{card}"
    );
    assert!(build_action_mask(&r.game, 0)[main_action(0) as usize] > 0.0);
    r.game.decode_action(main_action(0), 0);
    assert!(r.pending_selection_view().unwrap().is_optional, "{card}");
    let mut cloned = r.game.clone();
    for game in [&mut r.game, &mut cloned] {
        assert!(build_action_mask(game, 0)[PASS as usize] > 0.0);
        game.resolve_selection(0, PASS).unwrap();
        assert!(game.pending_selection.is_none());
        assert_eq!(game.current_phase, GamePhase::Main);
        assert_eq!(game.memory, before.memory);
        assert_eq!(game.players[0].hand.len(), before.players[0].hand.len());
        assert_eq!(game.players[0].trash.len(), before.players[0].trash.len());
        assert_eq!(game.players[0].battle_area.len(), 2);
        let mask = build_action_mask(game, 0);
        assert_eq!(
            mask[main_action(0) as usize],
            0.0,
            "{card}: spent activation must not reopen"
        );
        assert_eq!(
            mask[main_action(1) as usize],
            1.0,
            "{card}: other copy remains usable"
        );
        assert_eq!(mask[PASS as usize], 1.0);
    }
    assert_eq!(to_ui_json(&r.game), to_ui_json(&cloned));
    // Resolving the cloned selection must not spend a use in the pre-activation clone.
    assert_eq!(build_action_mask(&before, 0)[main_action(0) as usize], 1.0);
}

#[test]
fn hakubamon_decline_cannot_reopen_main_in_search_clone() {
    assert_decline_spends_use("EX12-043", false);
}

#[test]
fn betelgammamon_decline_cannot_reopen_main_in_search_clone() {
    assert_decline_spends_use("EX12-013", false);
}

#[test]
fn manekimon_decline_cannot_reopen_main_in_search_clone() {
    assert_decline_spends_use("BT26-012", false);
}

#[test]
fn gekomon_decline_cannot_reopen_main_in_search_clone() {
    assert_decline_spends_use("BT26-021", true);
}

#[test]
fn nightchiropmon_decline_spends_use_with_another_cost_still_available() {
    let mut tamer = make_test_card("TAMER", "Tamer");
    tamer.card_kind = CardKind::Tamer;
    tamer.level = None;
    tamer.dp = None;
    let mut option = make_test_card("OPTION", "Option");
    option.card_kind = CardKind::Option;
    option.traits = vec!["Glowing Dawn".to_string()];
    let mut r = DebugRunner::builder()
        .dsl_card("BT26-070")
        .unwrap()
        .add_card(tamer)
        .add_card(option)
        .hand(0, &["OPTION"])
        .memory(10)
        .start();
    r.set_first_player(0);
    let opt = r.game.players[0].hand.pop().unwrap();
    r.game.players[0].trash.push(opt);
    r.place_on_field(0, "BT26-070", Some(0));
    r.place_on_field(0, "BT26-070", Some(0));
    let host = r.place_stack(0, &["OPTION", "OPTION", "OPTION", "OPTION", "TAMER"]);
    for source in &mut r.game.players[0].battle_area[host.index as usize].card_sources[..4] {
        source.face_down = true;
    }
    r.game.enter_main_phase();
    assert!(build_action_mask(&r.game, 0)[main_action(0) as usize] > 0.0);
    r.game.decode_action(main_action(0), 0);
    while r.game.players[0].battle_area[host.index as usize]
        .card_sources
        .len()
        > 3
    {
        let view = r.pending_selection_view().expect("pay two face-down cards");
        let action = *view.valid_action_ids.iter().find(|&&a| a != PASS).unwrap();
        r.execute_action(0, action).unwrap();
    }
    assert!(r.pending_selection_view().unwrap().is_optional);
    let mut cloned = r.game.clone();
    for game in [&mut r.game, &mut cloned] {
        game.resolve_selection(0, PASS).unwrap();
        assert!(game.pending_selection.is_none());
        assert_eq!(
            game.players[0].battle_area[host.index as usize]
                .card_sources
                .len(),
            3
        );
        assert_eq!(build_action_mask(game, 0)[main_action(0) as usize], 0.0);
        assert_eq!(
            build_action_mask(game, 0)[main_action(1) as usize],
            1.0,
            "remaining two cost cards still permit the other copy's Main"
        );
    }
    assert_eq!(to_ui_json(&r.game), to_ui_json(&cloned));
}
