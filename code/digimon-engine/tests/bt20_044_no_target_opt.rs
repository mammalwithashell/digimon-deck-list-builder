//! Official rules15-9-1-2 and15-14-1-5: mandatory no-target activation spends OPT.
//! DCGO BT20_044.cs adds a non-printed CanActivate target condition; this test
//! distinguishes that oracle behavior from the printed/rules implementation.
use digimon_engine::{
    action::{build_action_mask, space::ATTACK_START},
    debug_runner::{make_test_card, DebugRunner},
    permanent::PermanentHandle,
};
fn fixture() -> DebugRunner {
    let mut weak = make_test_card("OPT-WEAK", "Unrelated Digimon");
    weak.dp = Some(1000);
    weak.level = Some(3);
    let mut r = DebugRunner::builder()
        .dsl_card("BT20-044")
        .unwrap()
        .dsl_card("BT20-042")
        .unwrap()
        .dsl_card("EX3-074")
        .unwrap()
        .add_card(weak)
        .deck(0, &["OPT-WEAK"; 10])
        .deck(1, &["OPT-WEAK"; 10])
        .security(1, &["OPT-WEAK"; 5])
        .memory(10)
        .start();
    r.skip_mulligan();
    r.game.enter_main_phase();
    r
}
fn host(r: &mut DebugRunner, inherited: bool) -> PermanentHandle {
    if inherited {
        r.place_stack(0, &["BT20-044", "EX3-074"])
    } else {
        r.place_on_field(0, "BT20-044", Some(0));
        r.place_on_field(0, "EX3-074", Some(0))
    }
}
fn act(r: &mut DebugRunner, a: u16) {
    let p = r.game.pending_selection.as_ref().unwrap().selecting_player;
    assert_eq!(build_action_mask(&r.game, p)[a as usize], 1.0);
    r.game.resolve_selection(p, a).unwrap();
}
fn later_target_does_not_restore_opt(r: &mut DebugRunner, h: PermanentHandle) {
    let victim = r.place_on_field(1, "OPT-WEAK", Some(0));
    let spare = r.place_on_field(1, "OPT-WEAK", Some(0));
    r.game.players[1].battle_area[spare.index as usize].is_suspended = true;
    let pin = r.game.players[1].battle_area[spare.index as usize]
        .top_card()
        .handle();
    r.battle_digimon(h, victim);
    assert!(
        r.game.pending_selection.is_none(),
        "mandatory no-target activation already spent OPT"
    );
    assert!(r.game.players[1]
        .battle_area
        .iter()
        .any(|p| p.top_card().handle() == pin));
}
#[test]
fn mandatory_no_target_battle_activation_spends_face_and_inherited_opt() {
    for inherited in [false, true] {
        let mut r = fixture();
        let h = host(&mut r, inherited);
        let victim = r.place_on_field(1, "OPT-WEAK", Some(0));
        r.battle_digimon(h, victim);
        assert!(r.game.pending_selection.is_none());
        assert!(r.game.players[1].battle_area.is_empty());
        let saved = r.game.clone();
        for _ in 0..2 {
            r.game = saved.clone();
            later_target_does_not_restore_opt(&mut r, h);
        }
    }
}
#[test]
fn an_unsuspended_survivor_is_not_a_target_but_does_not_prevent_opt_use() {
    for inherited in [false, true] {
        let mut r = fixture();
        let h = host(&mut r, inherited);
        let victim = r.place_on_field(1, "OPT-WEAK", Some(0));
        r.place_on_field(1, "OPT-WEAK", Some(0));
        r.battle_digimon(h, victim);
        assert!(r.game.pending_selection.is_none());
        assert_eq!(r.battle_area_size(1), 1);
        later_target_does_not_restore_opt(&mut r, h);
    }
}
#[test]
fn valid_suspended_target_is_chosen_and_deleted_in_clones() {
    for inherited in [false, true] {
        let mut r = fixture();
        let h = host(&mut r, inherited);
        let victim = r.place_on_field(1, "OPT-WEAK", Some(0));
        let spare = r.place_on_field(1, "OPT-WEAK", Some(0));
        r.game.players[1].battle_area[spare.index as usize].is_suspended = true;
        r.battle_digimon(h, victim);
        let saved = r.game.clone();
        for _ in 0..2 {
            r.game = saved.clone();
            act(&mut r, ATTACK_START);
            assert!(r.game.pending_selection.is_none());
            assert!(r.game.players[1].battle_area.is_empty());
        }
    }
}
#[test]
fn no_target_mandatory_trigger_remains_in_both_simultaneous_orders() {
    let mut r = fixture();
    let h = r.place_stack(0, &["BT20-044", "BT20-042", "EX3-074"]);
    let victim = r.place_on_field(1, "OPT-WEAK", Some(0));
    r.battle_digimon(h, victim);
    let entries = r
        .game
        .pending_selection
        .as_ref()
        .expect("both real battle triggers")
        .effect_choices
        .as_ref()
        .unwrap()
        .clone();
    let picks: Vec<_> = entries
        .iter()
        .filter_map(|e| {
            e.source_card
                .and_then(|c| r.game.card_data_for_handle(c))
                .map(|d| (d.card_id.clone(), e.action_id))
        })
        .collect();
    assert_eq!(picks.len(), 2);
    assert!(picks.iter().any(|(id, _)| id == "BT20-044"));
    assert!(picks.iter().any(|(id, _)| id == "BT20-042"));
    let saved = r.game.clone();
    for (_, action) in picks {
        r.game = saved.clone();
        act(&mut r, action);
        assert!(r.game.pending_selection.is_none());
        assert_eq!(r.security_count(1), 4);
        later_target_does_not_restore_opt(&mut r, h);
    }
}
