//! Official observation 44e13d06..., card image, Q2142. Q2143 carrier rebinding
//! remains unverified in this test set; normal refiring is covered here.
use digimon_engine::action::{build_action_mask, space::PASS};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, Expiry, ModifierType};
use digimon_engine::modifiers::ModifierEntry;
use digimon_engine::Keyword;
const ID: &str = "BT11-112";
fn base() -> DebugRunnerBuilder {
    let mut b = DebugRunner::builder().dsl_card(ID).expect("BT11-112 loads")
        .from_dsl_yaml("card: WD\nname: AeroVeedramon\nkind: digimon\ncolor: [blue]\ncost: 6\nlevel: 5\ndp: 7000\neffects:\n  - when: when_digivolving\n    process:\n      - gain_memory: 2\n  - when: when_digivolving\n    process:\n      - draw: {of: you, count: 1}\n").unwrap();
    for (id, name, kind, color) in [
        (
            "VEED",
            "UlforceVeedramon",
            CardKind::Digimon,
            CardColor::Blue,
        ),
        ("VEE", "Veemon", CardKind::Digimon, CardColor::Blue),
        ("OTHER", "Other", CardKind::Digimon, CardColor::Blue),
        ("RED", "Red", CardKind::Digimon, CardColor::Red),
        ("TAMER", "Veedramon Tamer", CardKind::Tamer, CardColor::Blue),
    ] {
        let mut c = make_test_card(id, name);
        c.card_kind = kind;
        c.colors = vec![color];
        b = b.add_card(c);
    }
    b.deck(0, &["OTHER"; 10]).deck(1, &["OTHER"; 10]).memory(5)
}
fn pick(r: &mut DebugRunner, index: usize) {
    let v = r.pending_selection_view().expect("player choice");
    let mask = build_action_mask(&r.game, v.selecting_player);
    for a in &v.valid_action_ids {
        assert_eq!(mask[*a as usize], 1.0);
    }
    assert_eq!(mask[PASS as usize], if v.is_optional { 1.0 } else { 0.0 });
    r.execute_action(v.selecting_player, v.valid_action_ids[index])
        .unwrap();
}
#[test]
fn bt11_112_on_play_filters_digimon_name_and_grants_both_keywords() {
    let mut r = base().hand(0, &[ID]).start();
    let vee = r.place_on_field(0, "VEE", Some(0));
    let veed = r.place_on_field(0, "VEED", Some(0));
    r.place_on_field(0, "OTHER", Some(0));
    r.place_on_field(0, "TAMER", Some(0));
    r.place_on_field(1, "VEED", Some(0));
    r.play(0, 0).unwrap();
    assert_eq!(
        r.pending_selection_view().unwrap().valid_action_ids.len(),
        2
    );
    pick(&mut r, 1);
    assert!(r.game.has_keyword(veed, Keyword::Blocker));
    assert!(r.game.has_keyword(veed, Keyword::Evade));
    assert!(!r.game.has_keyword(vee, Keyword::Blocker));
}
#[test]
fn bt11_112_paid_refire_selects_exactly_one_effect_on_either_turn() {
    for turn in [0, 1] {
        let mut r = base().start();
        r.skip_mulligan();
        r.game.turn_player_idx = turn;
        let rina = r.place_on_field(0, ID, Some(0));
        let veed = r.place_on_field(0, "WD", Some(0));
        r.game.suspend(veed);
        assert!(r.pending_is_optional());
        pick(&mut r, 0);
        assert!(r.game.players[0].battle_area[rina.index as usize].is_suspended);
        let v = r.pending_selection_view().expect("two WD choices");
        assert!(!v.is_optional);
        assert_eq!(v.valid_action_ids.len(), 2);
        let mut clone = DebugRunner::wrap(r.game.clone());
        pick(&mut r, 0);
        pick(&mut clone, 1);
        assert_eq!(r.memory(), if turn == 0 { 7 } else { 3 });
        assert_eq!(r.hand_size(0), 0);
        assert_eq!(clone.memory(), 5);
        assert_eq!(clone.hand_size(0), 1);
    }
}
#[test]
fn bt11_112_q2142_cost_without_wd_and_decline() {
    for decline in [false, true] {
        let mut r = base().start();
        let rina = r.place_on_field(0, ID, Some(0));
        let veed = r.place_on_field(0, "VEED", Some(0));
        r.game.suspend(veed);
        assert!(r.pending_is_optional());
        if decline {
            r.execute_action(0, PASS).unwrap();
        } else {
            pick(&mut r, 0);
        }
        assert_eq!(
            r.game.players[0].battle_area[rina.index as usize].is_suspended,
            !decline
        );
        assert!(r.pending_selection().is_none());
        assert_eq!(r.memory(), 5);
    }
}
#[test]
fn bt11_112_suspend_trigger_filters_name_kind_owner_and_cost() {
    for case in 0..5 {
        let mut r = base().start();
        let rina = r.place_on_field(0, ID, Some(0));
        if case == 3 {
            r.game.players[0].battle_area[0].is_suspended = true;
        }
        if case == 4 {
            r.game.modifiers.add(
                rina,
                ModifierEntry::simple(ModifierType::CannotSuspend, 0, Expiry::Permanent, 1),
            );
        }
        let id = match case {
            0 => "VEE",
            1 => "TAMER",
            _ => "WD",
        };
        let target = r.place_on_field(if case == 2 { 1 } else { 0 }, id, Some(0));
        r.game.suspend(target);
        assert!(r.pending_selection().is_none(), "case {case}");
        assert_eq!(r.memory(), 5);
    }
}
#[test]
fn bt11_112_unsuspend_memory_filters_and_opt() {
    let mut r = base().start();
    r.place_on_field(0, ID, Some(0));
    for (owner, id, expected) in [
        (0, "RED", 5),
        (0, "TAMER", 5),
        (1, "OTHER", 5),
        (0, "OTHER", 6),
        (0, "OTHER", 6),
    ] {
        let h = r.place_on_field(owner, id, Some(0));
        r.game.players[owner as usize].battle_area[h.index as usize].is_suspended = true;
        r.game.unsuspend(h);
        assert_eq!(r.memory(), expected);
    }
    let mut r = base().start();
    r.place_on_field(0, ID, Some(0));
    r.game.turn_player_idx = 1;
    let h = r.place_on_field(0, "OTHER", Some(0));
    r.game.players[0].battle_area[h.index as usize].is_suspended = true;
    r.game.unsuspend(h);
    assert_eq!(r.memory(), 5);
}
#[test]
fn bt11_112_security_free_play_and_no_on_play_target() {
    let mut r = base().security(1, &[ID]).start();
    r.skip_mulligan();
    let h = r.place_on_field(0, "OTHER", Some(0));
    r.attack_player(h, 1, false);
    r.auto_resolve().unwrap();
    assert_eq!(r.battle_area_size(1), 1);
    assert_eq!(r.memory(), 5);
    assert_eq!(
        r.game.players[1].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        ID
    );
}
