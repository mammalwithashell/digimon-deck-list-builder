use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind};
const ID: &str = "BT22-023";
fn base() -> DebugRunnerBuilder {
    let mut b = DebugRunner::builder().dsl_card(ID).expect("BT22-023 loads");
    for (id, name, level, color, kind) in [
        ("LOW", "Low", 4, CardColor::Blue, CardKind::Digimon),
        ("CS", "CS", 4, CardColor::Red, CardKind::Digimon),
        ("HIGH", "High", 5, CardColor::Red, CardKind::Digimon),
        (
            "VEED",
            "UlforceVeedramon",
            6,
            CardColor::Blue,
            CardKind::Digimon,
        ),
        ("TAMER", "Blue Tamer", 0, CardColor::Blue, CardKind::Tamer),
        ("RED", "Red Tamer", 0, CardColor::Red, CardKind::Tamer),
    ] {
        let mut c = make_test_card(id, name);
        c.level = Some(level);
        c.colors = vec![color];
        c.card_kind = kind;
        c.dp = Some(12000);
        if id == "CS" {
            c.traits = vec!["CS".into()];
        }
        b = b.add_card(c);
    }
    b.deck(0, &["LOW"; 20]).deck(1, &["LOW"; 20]).memory(10)
}
fn accept(r: &mut DebugRunner) {
    let v = r.pending_selection_view().expect("choice");
    let mask = digimon_engine::action::mask::build_action_mask(&r.game, v.selecting_player);
    for a in &v.valid_action_ids {
        assert!(mask[*a as usize] > 0.0);
    }
    r.execute_action(v.selecting_player, v.valid_action_ids[0])
        .unwrap();
}
#[test]
fn bt22_023_on_play_bottom_decks_only_low_level_digimon() {
    let mut r = base().hand(0, &[ID]).start();
    r.place_on_field(1, "LOW", Some(0));
    r.place_on_field(1, "HIGH", Some(0));
    r.place_on_field(1, "TAMER", Some(0));
    r.play(0, 0).unwrap();
    assert_eq!(
        r.pending_selection_view().unwrap().valid_action_ids.len(),
        1
    );
    accept(&mut r);
    assert_eq!(r.battle_area_size(1), 2);
    assert_eq!(r.game.players[1].deck.len(), 21);
}
#[test]
fn bt22_023_inherited_optional_unsuspend_and_opt() {
    let mut r = base().start();
    r.place_on_field(0, "TAMER", Some(0));
    let h = r.place_stack(0, &[ID, "VEED"]);
    r.game.suspend(h);
    assert!(r.pending_is_optional());
    accept(&mut r);
    assert!(!r.game.players[0].battle_area[1].is_suspended);
    r.game.suspend(h);
    assert!(r.pending_selection().is_none());
    assert!(r.game.players[0].battle_area[1].is_suspended);
}
#[test]
fn bt22_023_inherited_requires_name_and_blue_tamer_and_self_event() {
    for (carrier, tamer) in [("VEED", "RED"), ("HIGH", "TAMER")] {
        let mut r = base().start();
        r.place_on_field(0, tamer, Some(0));
        let h = r.place_stack(0, &[ID, carrier]);
        r.game.suspend(h);
        assert!(r.pending_selection().is_none());
    }
    let mut r = base().start();
    let other = r.place_on_field(0, "TAMER", Some(0));
    r.place_stack(0, &[ID, "VEED"]);
    r.game.suspend(other);
    assert!(r.pending_selection().is_none());
}
#[test]
fn bt22_023_inherited_decline() {
    let mut r = base().start();
    r.place_on_field(0, "TAMER", Some(0));
    let h = r.place_stack(0, &[ID, "VEED"]);
    r.game.suspend(h);
    assert!(r.pending_is_optional());
    r.execute_action(0, digimon_engine::action::space::PASS)
        .unwrap();
    assert!(r.game.players[0].battle_area[1].is_suspended);
}

#[test]
fn bt22_023_end_turn_blue_digimon_or_tamer_choices_and_decline() {
    for pick in 0..2 {
        let mut r = base().start();
        r.place_on_field(0, ID, Some(0));
        r.place_on_field(0, "LOW", Some(0));
        r.place_on_field(0, "TAMER", Some(0));
        r.place_on_field(0, "RED", Some(0));
        for p in &mut r.game.players[0].battle_area {
            p.is_suspended = true;
        }
        r.pass_turn();
        if r.pending_selection_view().unwrap().valid_action_ids.len() == 1 {
            accept(&mut r);
        }
        let v = r.pending_selection_view().unwrap();
        assert!(r.pending_is_optional());
        // AeroVeedramon, the blue Digimon and the blue Tamer, never the red Tamer.
        assert_eq!(v.valid_action_ids.len(), 3);
        r.execute_action(v.selecting_player, v.valid_action_ids[pick + 1])
            .unwrap();
        assert!(!r.game.players[0].battle_area[pick + 1].is_suspended);
        assert!(r.game.players[0].battle_area[3].is_suspended);
    }
    let mut r = base().start();
    r.place_on_field(0, ID, Some(0));
    r.game.players[0].battle_area[0].is_suspended = true;
    r.pass_turn();
    assert!(r.pending_is_optional());
    r.execute_action(0, digimon_engine::action::space::PASS)
        .unwrap();
    assert!(r.game.players[0].battle_area[0].is_suspended);
}
#[test]
fn bt22_023_no_bottom_deck_target() {
    let mut r = base().hand(0, &[ID]).start();
    r.place_on_field(1, "HIGH", Some(0));
    r.play(0, 0).unwrap();
    assert!(r.pending_selection().is_none());
    assert_eq!(r.battle_area_size(1), 1);
}

#[test]
fn bt22_023_evolution_routes_and_digivolving_removal() {
    for source in ["LOW", "CS"] {
        let mut r = base().hand(0, &[ID]).start();
        r.place_on_field(0, source, Some(0));
        r.place_on_field(1, "LOW", Some(0));
        let action = digimon_engine::action::space::encode_digivolve(0, 0);
        assert!(digimon_engine::action::mask::build_action_mask(&r.game, 0)[action as usize] > 0.0);
        r.game.decode_action(action, 0);
        assert_eq!(r.memory(), 7);
        accept(&mut r);
        assert_eq!(r.battle_area_size(1), 0);
    }
}
#[test]
fn bt22_023_end_turn_allows_already_unsuspended_target() {
    let mut r = base().start();
    r.place_on_field(0, ID, Some(0));
    r.place_on_field(0, "TAMER", Some(0));
    r.pass_turn();
    if r.pending_selection_view().unwrap().valid_action_ids.len() == 1 {
        accept(&mut r);
    }
    assert_eq!(
        r.pending_selection_view().unwrap().valid_action_ids.len(),
        2
    );
}
