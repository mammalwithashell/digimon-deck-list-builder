use digimon_engine::action::{
    mask::build_action_mask,
    space::{encode_digivolve, PASS},
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind};
const ID: &str = "BT12-029";
fn base() -> DebugRunnerBuilder {
    let mut b = DebugRunner::builder().dsl_card(ID).expect("BT12-029 loads");
    for (id, name, level, color, kind) in [
        (
            "ULT",
            "UlforceVeedramon",
            6,
            CardColor::Blue,
            CardKind::Digimon,
        ),
        (
            "SIMILAR",
            "UlforceVeedramon Future Mode",
            6,
            CardColor::Blue,
            CardKind::Digimon,
        ),
        ("LV5", "Level Five", 5, CardColor::Blue, CardKind::Digimon),
        ("LOW", "Low", 3, CardColor::Red, CardKind::Digimon),
        ("HIGH", "High", 5, CardColor::Red, CardKind::Digimon),
        ("TAMER", "Blue Tamer", 0, CardColor::Blue, CardKind::Tamer),
        ("RED", "Red Tamer", 0, CardColor::Red, CardKind::Tamer),
    ] {
        let mut c = make_test_card(id, name);
        c.level = (kind == CardKind::Digimon).then_some(level);
        c.colors = vec![color];
        c.card_kind = kind;
        c.dp = (kind == CardKind::Digimon).then_some(12000);
        b = b.add_card(c);
    }
    b.deck(0, &["LOW"; 20]).deck(1, &["LOW"; 20]).memory(10)
}
fn pick(r: &mut DebugRunner, index: usize, count: usize) {
    let v = r.pending_selection_view().expect("mandatory selection");
    assert_eq!(v.valid_action_ids.len(), count);
    assert!(!r.pending_is_optional());
    let mask = build_action_mask(&r.game, v.selecting_player);
    assert_eq!(mask[PASS as usize], 0.0);
    for a in &v.valid_action_ids {
        assert!(mask[*a as usize] > 0.0);
    }
    r.execute_action(v.selecting_player, v.valid_action_ids[index])
        .unwrap();
}
#[test]
fn bt12_029_routes_and_unsuspend_choices() {
    for (source, cost) in [("ULT", 1), ("LV5", 4)] {
        for choice in 0..2 {
            let mut r = base().hand(0, &[ID]).start();
            r.place_on_field(0, source, Some(0));
            r.place_on_field(0, "TAMER", Some(0));
            r.place_on_field(0, "RED", Some(0));
            r.place_on_field(0, "LOW", Some(0));
            for p in &mut r.game.players[0].battle_area {
                p.is_suspended = true;
            }
            let a = encode_digivolve(0, 0);
            assert!(build_action_mask(&r.game, 0)[a as usize] > 0.0);
            r.game.decode_action(a, 0);
            assert_eq!(r.memory(), 10 - cost);
            pick(&mut r, choice, 2);
            assert!(!r.game.players[0].battle_area[choice].is_suspended);
            assert!(r.game.players[0].battle_area[1 - choice].is_suspended);
            assert!(r.game.players[0].battle_area[2].is_suspended);
            assert!(r.game.players[0].battle_area[3].is_suspended);
        }
    }
    let mut r = base().hand(0, &[ID]).start();
    r.place_on_field(0, "SIMILAR", Some(0));
    assert_eq!(
        build_action_mask(&r.game, 0)[encode_digivolve(0, 0) as usize],
        0.0
    );
}
#[test]
fn bt12_029_unsuspend_allows_already_unsuspended_targets() {
    let mut r = base().hand(0, &[ID]).start();
    r.place_on_field(0, "LV5", Some(0));
    r.place_on_field(0, "TAMER", Some(0));
    let action = encode_digivolve(0, 0);
    assert!(build_action_mask(&r.game, 0)[action as usize] > 0.0);
    r.game.decode_action(action, 0);
    pick(&mut r, 0, 2);
    assert!(r.pending_selection().is_none());
}
#[test]
fn bt12_029_bounce_lowest_ties_and_once_per_turn() {
    for choice in 0..2 {
        let mut r = base().start();
        r.place_on_field(0, "TAMER", Some(0));
        let h = r.place_on_field(0, ID, Some(0));
        r.place_on_field(1, "HIGH", Some(0));
        r.place_on_field(1, "LOW", Some(0));
        r.place_on_field(1, "LOW", Some(0));
        r.place_on_field(1, "TAMER", Some(0));
        r.game.players[0].battle_area[1].is_suspended = true;
        r.game.unsuspend(h);
        pick(&mut r, choice, 2);
        assert_eq!(r.battle_area_size(1), 3);
        assert_eq!(r.hand_size(1), 1);
        r.game.players[0].battle_area[1].is_suspended = true;
        r.game.unsuspend(h);
        assert!(r.pending_selection().is_none());
        assert_eq!(r.battle_area_size(1), 3);
    }
}
#[test]
fn bt12_029_exact_named_source_alternative_and_negative_gates() {
    for (source, tamer, expected) in [
        ("ULT", "RED", true),
        ("SIMILAR", "RED", false),
        ("LOW", "RED", false),
        ("LOW", "TAMER", true),
    ] {
        let mut r = base().start();
        r.place_on_field(0, tamer, Some(0));
        let h = r.place_stack(0, &[source, ID]);
        r.place_on_field(1, "LOW", Some(0));
        r.game.players[0].battle_area[1].is_suspended = true;
        r.game.unsuspend(h);
        assert_eq!(
            r.pending_selection().is_some(),
            expected,
            "source={source} tamer={tamer}"
        );
        if expected {
            pick(&mut r, 0, 1);
            assert_eq!(r.battle_area_size(1), 0);
        }
    }
}
#[test]
fn bt12_029_other_unsuspend_does_not_trigger() {
    let mut r = base().start();
    let other = r.place_on_field(0, "TAMER", Some(0));
    r.place_on_field(0, ID, Some(0));
    r.place_on_field(1, "LOW", Some(0));
    r.game.players[0].battle_area[0].is_suspended = true;
    r.game.unsuspend(other);
    assert!(r.pending_selection().is_none());
    assert_eq!(r.battle_area_size(1), 1);
}
#[test]
fn bt12_029_triggers_on_opponents_turn() {
    let mut r = base().start();
    r.place_on_field(0, "TAMER", Some(0));
    let h = r.place_on_field(0, ID, Some(0));
    r.place_on_field(1, "LOW", Some(0));
    r.game.turn_player_idx = 1;
    r.game.players[0].battle_area[1].is_suspended = true;
    r.game.unsuspend(h);
    pick(&mut r, 0, 1);
    assert_eq!(r.battle_area_size(1), 0);
}
