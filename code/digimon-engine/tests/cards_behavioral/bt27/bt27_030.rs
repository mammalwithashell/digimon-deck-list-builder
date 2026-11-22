//! BT27-030 official English image/category522040, Q7661-63, checked2026-11-22.
//! Synthetic targets isolate the real DSL; this is not an admission fixture.
use digimon_engine::{
    action::{
        mask::build_action_mask,
        space::{encode_digivolve, HAND_EFFECT_START, PASS},
    },
    card_data::CardData,
    debug_runner::{make_test_card_with_level, DebugRunner, DebugRunnerBuilder},
    enums::{CardColor, CardKind},
    permanent::PermanentHandle,
    replacement::ReplacementCause,
};
const ID: &str = "BT27-030";
fn card(id: &str, level: u8, kind: CardKind, cost: u16, squad: bool) -> CardData {
    let mut c = make_test_card_with_level(id, id, level);
    c.card_kind = kind;
    c.play_cost = cost;
    c.colors = vec![CardColor::Blue];
    c.dp = Some(1000);
    if squad {
        c.traits = vec!["DATA SQUAD".into()];
    }
    c
}
fn base() -> DebugRunnerBuilder {
    let mut named = card("NAME", 6, CardKind::Digimon, 10, false);
    named.card_name = "MirageGaogamon".into();
    let mut red = card("RED", 4, CardKind::Digimon, 4, false);
    red.colors = vec![CardColor::Red];
    let mut ds = red.clone();
    ds.card_id = "DS4".into();
    ds.traits = vec!["DATA SQUAD".into()];
    DebugRunner::builder()
        .dsl_card(ID)
        .expect("BT27-030 production YAML")
        .add_card(card("F", 3, CardKind::Digimon, 3, false))
        .add_card(card("L4", 4, CardKind::Digimon, 4, false))
        .add_card(card("L5", 5, CardKind::Digimon, 5, false))
        .add_card(card("T", 0, CardKind::Tamer, 3, false))
        .add_card(card("CHEAP", 3, CardKind::Digimon, 5, true))
        .add_card(card("EXP", 3, CardKind::Digimon, 6, true))
        .add_card(card("OPT", 0, CardKind::Option, 5, true))
        .add_card(card("DST", 0, CardKind::Tamer, 5, true))
        .add_card(card("TOP", 6, CardKind::Digimon, 10, true))
        .add_card(named)
        .add_card(red)
        .add_card(ds)
        .deck(0, &["F"; 30])
        .deck(1, &["F"; 30])
        .security(1, &["F"; 5])
        .memory(10)
}
fn act(r: &mut DebugRunner, a: u16) {
    let p = r.pending_selection_view().unwrap().selecting_player;
    assert_eq!(build_action_mask(&r.game, p)[a as usize], 1.0);
    r.execute_action(p, a).unwrap();
}
fn pick(r: &mut DebugRunner, n: usize) {
    let a: Vec<_> = r
        .pending_selection_view()
        .unwrap()
        .valid_action_ids
        .into_iter()
        .filter(|a| *a != PASS)
        .collect();
    act(r, a[n]);
}
fn drive(r: &mut DebugRunner) {
    for _ in 0..20 {
        if r.pending_selection().is_none() {
            return;
        }
        pick(r, 0);
    }
    panic!("selection did not terminate");
}
fn stash(r: &mut DebugRunner, p: u8, n: usize) -> PermanentHandle {
    let mut ids = vec!["F"; n];
    ids.push("T");
    let h = r.place_stack(p, &ids);
    for c in &mut r.game.players[p as usize].battle_area[h.index as usize].card_sources[..n] {
        c.face_down = true;
    }
    h
}
fn count(r: &DebugRunner, p: u8, id: &str) -> usize {
    r.game.players[p as usize]
        .battle_area
        .iter()
        .filter(|x| x.top_card().card_id(&r.game.card_data) == id)
        .count()
}
fn start(hand: &[&str]) -> DebugRunner {
    let mut r = base().hand(0, hand).start();
    r.skip_mulligan();
    r.set_first_player(0);
    r
}
fn pay(r: &mut DebugRunner, yes: bool) {
    act(r, HAND_EFFECT_START + if yes { 0 } else { 1 });
}
#[test]
fn bt27_030_play_bounce_filter_and_optional_decline() {
    let mut r = start(&[ID]);
    r.place_on_field(0, "L4", Some(0));
    r.place_on_field(1, "L4", Some(0));
    r.place_on_field(1, "L5", Some(0));
    r.place_on_field(1, "T", Some(0));
    r.play(0, 0);
    assert!(r.pending_is_optional());
    assert_eq!(
        r.pending_selection_view()
            .unwrap()
            .valid_action_ids
            .iter()
            .filter(|a| **a != PASS)
            .count(),
        1
    );
    act(&mut r, PASS);
    assert_eq!(count(&r, 1, "L4"), 1);
    assert!(r.pending_selection().is_none());
}
#[test]
fn bt27_030_pay_two_same_tamer_then_free_digimon() {
    let mut r = start(&[ID, "CHEAP", "EXP", "F"]);
    let t = stash(&mut r, 0, 3);
    r.place_on_field(1, "L4", Some(0));
    r.play(0, 0);
    pick(&mut r, 0);
    assert_eq!(r.hand_size(1), 1);
    pay(&mut r, true);
    pick(&mut r, 0);
    pick(&mut r, 0);
    assert_eq!(r.game.players[0].trash.len(), 2);
    assert!(r.game.players[0].trash.iter().all(|c| !c.face_down));
    assert_eq!(
        r.game.players[0].battle_area[t.index as usize]
            .card_sources
            .len(),
        2
    );
    assert_eq!(
        r.pending_selection_view()
            .unwrap()
            .valid_action_ids
            .iter()
            .filter(|a| **a != PASS)
            .count(),
        1
    );
    pick(&mut r, 0);
    assert_eq!(count(&r, 0, "CHEAP"), 1);
    assert_eq!(r.memory(), 3);
}
#[test]
fn bt27_030_split_payment_and_free_tamer() {
    let mut r = start(&[ID, "DST"]);
    stash(&mut r, 0, 1);
    stash(&mut r, 0, 1);
    stash(&mut r, 1, 2);
    r.play(0, 0);
    pay(&mut r, true);
    pick(&mut r, 1);
    pick(&mut r, 0);
    pick(&mut r, 0);
    assert_eq!(count(&r, 0, "DST"), 1);
    assert_eq!(r.game.players[0].trash.len(), 2);
    assert_eq!(r.game.players[1].trash.len(), 0);
    assert_eq!(r.memory(), 3);
}
#[test]
fn bt27_030_decline_payment_or_paid_hand_pick() {
    for decline_payment in [true, false] {
        let mut r = start(&[ID, "CHEAP"]);
        stash(&mut r, 0, 2);
        r.play(0, 0);
        pay(&mut r, !decline_payment);
        if !decline_payment {
            pick(&mut r, 0);
            pick(&mut r, 0);
            act(&mut r, PASS);
        }
        assert_eq!(
            r.game.players[0].trash.len(),
            if decline_payment { 0 } else { 2 }
        );
        assert_eq!(count(&r, 0, "CHEAP"), 0);
    }
}
#[test]
fn bt27_030_insufficient_or_wrong_host_payment_not_partial() {
    for case in 0..3 {
        let mut r = start(&[ID, "CHEAP"]);
        let t = stash(
            &mut r,
            if case == 1 { 1 } else { 0 },
            if case == 0 { 1 } else { 2 },
        );
        if case == 2 {
            let h = r.place_stack(0, &["F", "F", "L4"]);
            for c in &mut r.game.players[0].battle_area[h.index as usize].card_sources[..2] {
                c.face_down = true;
            }
            for c in &mut r.game.players[0].battle_area[t.index as usize].card_sources[..2] {
                c.face_down = false;
            }
        }
        r.play(0, 0);
        assert!(r.pending_selection().is_none());
        assert_eq!(r.game.players[0].trash.len(), 0);
    }
}
#[test]
fn bt27_030_can_pay_with_no_eligible_hand_card() {
    let mut r = start(&[ID, "EXP", "F"]);
    stash(&mut r, 0, 2);
    r.play(0, 0);
    pay(&mut r, true);
    pick(&mut r, 0);
    pick(&mut r, 0);
    assert!(r.pending_selection().is_none());
    assert_eq!(r.game.players[0].trash.len(), 2);
}
#[test]
fn bt27_030_free_option_uses_and_trashes() {
    let mut r = start(&[ID, "OPT"]);
    stash(&mut r, 0, 2);
    r.play(0, 0);
    pay(&mut r, true);
    pick(&mut r, 0);
    pick(&mut r, 0);
    pick(&mut r, 0);
    assert_eq!(r.memory(), 3);
    assert_eq!(
        r.game.players[0]
            .trash
            .iter()
            .filter(|c| c.card_id(&r.game.card_data) == "OPT")
            .count(),
        1
    );
    assert_eq!(count(&r, 0, "OPT"), 0);
}
#[test]
fn bt27_030_actual_evolution_routes_and_attack_shared_opt() {
    for host in ["L4", "DS4", "RED"] {
        let mut r = start(&[ID]);
        let h = r.place_on_field(0, host, Some(0));
        r.place_on_field(1, "L4", Some(0));
        let a = encode_digivolve(0, h.index as u16);
        assert_eq!(
            build_action_mask(&r.game, 0)[a as usize],
            if host == "RED" { 0.0 } else { 1.0 }
        );
        if host == "RED" {
            continue;
        }
        r.game.decode_action(a, 0);
        pick(&mut r, 0);
        assert_eq!(r.memory(), 7);
        assert_eq!(r.hand_size(1), 1);
        r.place_on_field(1, "L4", Some(0));
        r.attack_player(h, 1, false);
        assert_eq!(count(&r, 1, "L4"), 1);
        assert!(r.pending_selection().is_none());
    }
}
#[test]
fn bt27_030_actual_attack_can_bounce_without_payment() {
    let mut r = start(&[]);
    let h = r.place_on_field(0, ID, Some(0));
    r.place_on_field(1, "L4", Some(0));
    r.attack_player(h, 1, false);
    pick(&mut r, 0);
    assert_eq!(count(&r, 1, "L4"), 0);
    assert_eq!(r.hand_size(1), 1);
}
#[test]
fn bt27_030_inherited_carrier_name_or_trait_save_and_opt() {
    for top in ["NAME", "TOP"] {
        let mut r = start(&[]);
        let h = r.place_stack(0, &[ID, top]);
        stash(&mut r, 0, 2);
        r.game
            .delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
        drive(&mut r);
        assert_eq!(count(&r, 0, top), 1);
        assert_eq!(r.game.players[0].trash.len(), 1);
        r.game
            .delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
        drive(&mut r);
        assert_eq!(count(&r, 0, top), 0);
    }
}
#[test]
fn bt27_030_inherited_wrong_carrier_top_face_and_other_subject_no_save() {
    for case in 0..3 {
        let mut r = start(&[]);
        let h = if case == 0 {
            r.place_stack(0, &[ID, "L5"])
        } else if case == 1 {
            r.place_on_field(0, ID, Some(0))
        } else {
            r.place_stack(0, &[ID, "TOP"]);
            r.place_on_field(0, "TOP", Some(0))
        };
        stash(&mut r, 0, 1);
        r.game
            .delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
        assert!(r.pending_selection().is_none());
        assert_eq!(r.game.players[0].trash.len(), if case == 0 { 2 } else { 1 });
    }
}
#[test]
fn bt27_030_inherited_decline_or_no_own_payment_leaves() {
    for no_own in [false, true] {
        let mut r = start(&[]);
        let h = r.place_stack(0, &[ID, "TOP"]);
        stash(&mut r, if no_own { 1 } else { 0 }, 1);
        r.game
            .delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
        if r.pending_selection().is_some() {
            act(&mut r, PASS);
        }
        assert_eq!(count(&r, 0, "TOP"), 0);
    }
}
#[test]
fn bt27_030_bottom_face_down_identity_and_face_up_skipping() {
    let mut r = start(&[ID]);
    let t = r.place_stack(0, &["L4", "F", "L5", "EXP", "T"]);
    for i in [1usize, 2, 3] {
        r.game.players[0].battle_area[t.index as usize].card_sources[i].face_down = true;
    }
    r.play(0, 0);
    pay(&mut r, true);
    pick(&mut r, 0);
    pick(&mut r, 0);
    let ids: Vec<_> = r.game.players[0]
        .trash
        .iter()
        .map(|c| c.card_id(&r.game.card_data))
        .collect();
    assert_eq!(ids, vec!["F", "L5"]);
    let ids: Vec<_> = r.game.players[0].battle_area[t.index as usize]
        .card_sources
        .iter()
        .map(|c| c.card_id(&r.game.card_data))
        .collect();
    assert_eq!(ids, vec!["L4", "EXP", "T"]);
    assert!(r.game.players[0].trash.iter().all(|c| !c.face_down));
}
#[test]
fn bt27_030_q7661_continues_after_activator_leaves_during_bounce() {
    // A minimal immediate-effect opponent fixture removes the activating card.
    let fixture = r#"
card: REACTOR
name: Reactor
kind: digimon
level: 4
color: [red]
cost: 4
dp: 4000
effects:
  - kind: replacement
    trigger: when_would_leave_battle_area
    once_per_turn: true
    active_when: { is_source: true }
    process:
      - select_opponent_permanent:
          bind_as: attacker
          filter: { name_is: MachGaogamon }
          prompt: "Remove the activating MachGaogamon"
      - delete_permanent: { target: attacker }
"#;
    let mut r = base()
        .from_dsl_yaml(fixture)
        .unwrap()
        .hand(0, &[ID, "CHEAP"])
        .start();
    r.skip_mulligan();
    r.set_first_player(0);
    stash(&mut r, 0, 2);
    r.place_on_field(1, "REACTOR", Some(0));
    r.play(0, 0);
    drive(&mut r);
    assert_eq!(count(&r, 0, ID), 0);
    assert_eq!(count(&r, 0, "CHEAP"), 1);
    assert_eq!(r.game.players[0].trash.len(), 3);
    assert_eq!(r.memory(), 3);
}
