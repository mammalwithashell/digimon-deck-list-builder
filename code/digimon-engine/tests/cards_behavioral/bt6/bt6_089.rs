//! BT6-089: image and official bundle; CR 4.3 15-7-5 permits paying a cost without a benefit.
use digimon_engine::action::{build_action_mask, encode_attack, PASS};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, GamePhase};
use digimon_engine::permanent::PermanentHandle;
const ID: &str = "BT6-089";
fn base() -> DebugRunnerBuilder {
    let mut yellow = make_test_card("Y", "Yellow");
    yellow.colors = vec![CardColor::Yellow];
    yellow.dp = Some(6000);
    let mut multi = yellow.clone();
    multi.card_id = "M".into();
    multi.colors.push(CardColor::Red);
    let mut red = yellow.clone();
    red.card_id = "R".into();
    red.colors = vec![CardColor::Red];
    let mut small = red.clone();
    small.card_id = "S".into();
    small.dp = Some(1000);
    let mut tamer = red.clone();
    tamer.card_id = "T".into();
    tamer.card_kind = CardKind::Tamer;
    tamer.dp = None;
    DebugRunner::builder()
        .dsl_card(ID)
        .expect("BT6-089 embedded production card")
        .add_card(yellow)
        .add_card(multi)
        .add_card(red)
        .add_card(small)
        .add_card(tamer)
        .deck(0, &["R"; 20])
        .deck(1, &["R"; 20])
        .security(1, &["S"; 5])
        .memory(8)
}
fn choose(r: &mut DebugRunner, a: u16) {
    let p = r
        .pending_selection_view()
        .expect("visible choice")
        .selecting_player;
    assert_eq!(build_action_mask(&r.game, p)[a as usize], 1.0);
    r.execute_action(p, a).unwrap();
}
fn accept(r: &mut DebugRunner) {
    let v = r
        .pending_selection_view()
        .expect("optional activation prompt");
    assert_eq!(
        build_action_mask(&r.game, v.selecting_player)[PASS as usize],
        1.0
    );
    choose(
        r,
        v.valid_action_ids
            .iter()
            .copied()
            .find(|a| *a != PASS)
            .unwrap(),
    );
}
fn attack(r: &mut DebugRunner, h: PermanentHandle) {
    r.game.current_phase = GamePhase::Main;
    let a = encode_attack(h.index as u16, 14);
    assert_eq!(
        build_action_mask(&r.game, h.player)[a as usize],
        1.0,
        "legal attack player={} turn={} phase={:?} suspended={} turn_played={} turn_count={}",
        h.player,
        r.game.turn_player_idx,
        r.game.current_phase,
        r.game.players[h.player as usize].battle_area[h.index as usize].is_suspended,
        r.game.players[h.player as usize].battle_area[h.index as usize].turn_played,
        r.game.turn_count
    );
    r.game.decode_action(a, h.player);
}
#[test]
fn start_turn_strict_security_comparison() {
    for (own, opp, gain) in [(1, 3, 2), (3, 3, 0), (4, 3, 0), (0, 0, 0), (0, 1, 2)] {
        let mut r = base()
            .security(0, &vec!["R"; own])
            .security(1, &vec!["R"; opp])
            .start();
        r.place_on_field(0, ID, Some(0));
        r.end_turn();
        r.game.memory = -1;
        r.end_turn();
        assert_eq!(r.game.turn_player_idx, 0);
        assert_eq!(r.memory(), 1 + gain, "own={own}, opp={opp}");
    }
}
#[test]
fn attack_optional_accept_and_all_opponent_digimon_targets_only() {
    let mut r = base().start();
    let tk = r.place_on_field(0, ID, Some(0));
    let a = r.place_on_field(0, "M", Some(0));
    r.place_on_field(0, "Y", Some(0));
    let x = r.place_on_field(1, "R", Some(0));
    let y = r.place_on_field(1, "Y", Some(0));
    r.place_on_field(1, "T", Some(0));
    r.place_in_breeding(1, "Y");
    attack(&mut r, a);
    accept(&mut r);
    assert!(r.game.players[0].battle_area[tk.index as usize].is_suspended);
    let v = r.pending_selection_view().expect("choose opposing Digimon");
    assert_eq!(v.valid_action_ids.len(), 2);
    assert!(!v.is_optional);
    let mask = build_action_mask(&r.game, 0);
    for id in &v.valid_action_ids {
        assert_eq!(mask[*id as usize], 1.0);
    }
    choose(&mut r, v.valid_action_ids[1]);
    assert_eq!(r.effective_dp(x), Some(6000));
    assert_eq!(r.effective_dp(y), Some(5000));
    r.auto_resolve().unwrap();
    r.end_turn();
    assert_eq!(r.effective_dp(y), Some(6000));
}
#[test]
fn decline_and_already_suspended_do_not_reduce_dp() {
    let mut r = base().start();
    let tk = r.place_on_field(0, ID, Some(0));
    let a = r.place_on_field(0, "Y", Some(0));
    let x = r.place_on_field(1, "R", Some(0));
    attack(&mut r, a);
    choose(&mut r, PASS);
    r.auto_resolve().unwrap();
    assert!(!r.game.players[0].battle_area[tk.index as usize].is_suspended);
    assert_eq!(r.effective_dp(x), Some(6000));
    r.game.players[0].battle_area[a.index as usize].is_suspended = false;
    r.game.players[0].battle_area[tk.index as usize].is_suspended = true;
    attack(&mut r, a);
    assert!(r.pending_selection().is_none());
    assert_eq!(r.effective_dp(x), Some(6000));
}
#[test]
fn nonyellow_and_opponent_attacks_do_not_trigger() {
    let mut r = base().start();
    r.place_on_field(0, ID, Some(0));
    let red = r.place_on_field(0, "R", Some(0));
    attack(&mut r, red);
    assert!(r.pending_selection().is_none());
    r.auto_resolve().unwrap();
    r.pass_turn();
    let enemy = r.place_on_field(1, "Y", Some(0));
    attack(&mut r, enemy);
    assert!(r.pending_selection().is_none());
}
#[test]
fn no_target_still_allows_suspend_payment() {
    let mut r = base().start();
    let tk = r.place_on_field(0, ID, Some(0));
    let a = r.place_on_field(0, "Y", Some(0));
    attack(&mut r, a);
    accept(&mut r);
    r.auto_resolve().unwrap();
    assert!(r.game.players[0].battle_area[tk.index as usize].is_suspended);
}
#[test]
fn no_once_per_turn_after_unsuspend() {
    let mut r = base().start();
    let tk = r.place_on_field(0, ID, Some(0));
    let a = r.place_on_field(0, "Y", Some(0));
    let x = r.place_on_field(1, "R", Some(0));
    for expected in [5000, 4000] {
        r.game.players[0].battle_area[tk.index as usize].is_suspended = false;
        r.game.players[0].battle_area[a.index as usize].is_suspended = false;
        attack(&mut r, a);
        accept(&mut r);
        let v = r.pending_selection_view().unwrap();
        choose(&mut r, v.valid_action_ids[0]);
        r.auto_resolve().unwrap();
        assert_eq!(r.effective_dp(x), Some(expected));
    }
}
#[test]
fn zero_dp_is_deleted_by_rules() {
    let mut r = base().start();
    r.place_on_field(0, ID, Some(0));
    let a = r.place_on_field(0, "Y", Some(0));
    r.place_on_field(1, "S", Some(0));
    attack(&mut r, a);
    accept(&mut r);
    let v = r.pending_selection_view().unwrap();
    choose(&mut r, v.valid_action_ids[0]);
    r.auto_resolve().unwrap();
    assert!(r.game.players[1].battle_area.is_empty());
    assert!(r.game.players[1]
        .trash
        .iter()
        .any(|c| r.game.card_data[c.data_index].card_id == "S"));
}
#[test]
fn normal_play_costs_four() {
    let mut r = base().hand(0, &[ID]).start();
    r.game.current_phase = GamePhase::Main;
    assert_eq!(build_action_mask(&r.game, 0)[0], 1.0);
    r.game.decode_action(0, 0);
    assert_eq!(r.memory(), 4);
    assert_eq!(r.game.players[0].battle_area.len(), 1);
}
#[test]
fn security_plays_tamer_without_cost() {
    let mut r = base().security(0, &[ID]).start();
    let enemy = r.place_on_field(1, "R", Some(0));
    r.pass_turn();
    let before = r.memory();
    attack(&mut r, enemy);
    r.auto_resolve().unwrap();
    assert_eq!(r.memory(), before);
    assert_eq!(r.security_count(0), 0);
    assert!(r.game.players[0]
        .battle_area
        .iter()
        .any(|p| r.game.card_data[p.top_card().data_index].card_id == ID));
}
