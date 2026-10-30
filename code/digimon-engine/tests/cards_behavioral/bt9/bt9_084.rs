//! BT9-084: printed image, official bundle and DCGO BT9_084; CR 4.3 15-7-5 permits paying a cost without a benefit.
use digimon_engine::action::{build_action_mask, encode_attack, PASS};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, GamePhase};
use digimon_engine::permanent::PermanentHandle;
const ID: &str = "BT9-084";
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
    let mut blue = red.clone();
    blue.card_id = "B".into();
    blue.colors = vec![CardColor::Blue];
    let mut small = red.clone();
    small.card_id = "S".into();
    small.dp = Some(1000);
    let mut tamer = red.clone();
    tamer.card_id = "T".into();
    tamer.card_kind = CardKind::Tamer;
    tamer.dp = None;
    DebugRunner::builder()
        .dsl_card(ID)
        .expect("BT9-084 embedded production card")
        .add_card(yellow)
        .add_card(multi)
        .add_card(red)
        .add_card(blue)
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
fn independent_start_turn_security_conditions() {
    for (own, opp, gain) in [(3, 3, 2), (3, 4, 1), (4, 3, 1), (4, 4, 0), (0, 0, 2)] {
        let mut r = base()
            .security(0, &vec!["R"; own])
            .security(1, &vec!["R"; opp])
            .start();
        r.place_on_field(0, ID, Some(0));
        r.end_turn();
        r.game.memory = -1;
        r.end_turn();
        assert_eq!(r.game.turn_player_idx, 0);
        assert_eq!(r.memory(), 1 + gain, "own={own} opp={opp}");
    }
}
#[test]
fn attack_accept_red_yellow_and_multicolor_security_only_and_expiry() {
    for color in ["R", "Y", "M"] {
        let mut r = base().start();
        let tk = r.place_on_field(0, ID, Some(0));
        let a = r.place_on_field(0, color, Some(0));
        let x = r.place_on_field(1, "R", Some(0));
        attack(&mut r, a);
        accept(&mut r);
        assert!(r.game.players[0].battle_area[tk.index as usize].is_suspended);
        assert_eq!(r.game.defender_security_dp_adjustment(1), -2000);
        assert_eq!(r.game.defender_security_dp_adjustment(0), 0);
        assert_eq!(r.effective_dp(x), Some(6000));
        r.auto_resolve().unwrap();
        r.end_turn();
        assert_eq!(r.game.defender_security_dp_adjustment(1), 0);
    }
}
#[test]
fn decline_suspended_blue_opponent_do_not_apply() {
    let mut r = base().start();
    let tk = r.place_on_field(0, ID, Some(0));
    let a = r.place_on_field(0, "Y", Some(0));
    attack(&mut r, a);
    choose(&mut r, PASS);
    r.auto_resolve().unwrap();
    assert!(!r.game.players[0].battle_area[tk.index as usize].is_suspended);
    assert_eq!(r.game.defender_security_dp_adjustment(1), 0);
    r.game.unsuspend(a);
    r.game.players[0].battle_area[tk.index as usize].is_suspended = true;
    attack(&mut r, a);
    assert!(r.pending_selection().is_none());
    r.auto_resolve().unwrap();
    r.game.unsuspend(tk);
    let b = r.place_on_field(0, "B", Some(0));
    attack(&mut r, b);
    assert!(r.pending_selection().is_none());
    r.auto_resolve().unwrap();
    r.pass_turn();
    let e = r.place_on_field(1, "R", Some(0));
    attack(&mut r, e);
    assert!(r.pending_selection().is_none());
    assert_eq!(r.game.defender_security_dp_adjustment(1), 0);
}
#[test]
fn legal_unsuspend_allows_reuse_without_once_per_turn() {
    let mut r = base().start();
    let tk = r.place_on_field(0, ID, Some(0));
    let a = r.place_on_field(0, "R", Some(0));
    for dp in [-2000, -4000] {
        r.game.unsuspend(tk);
        r.game.unsuspend(a);
        attack(&mut r, a);
        accept(&mut r);
        r.auto_resolve().unwrap();
        assert_eq!(r.game.defender_security_dp_adjustment(1), dp);
    }
}
#[test]
fn no_opponent_security_still_allows_payment() {
    let mut r = base().security(1, &[]).start();
    let tk = r.place_on_field(0, ID, Some(0));
    let a = r.place_on_field(0, "R", Some(0));
    attack(&mut r, a);
    accept(&mut r);
    assert!(r.game.players[0].battle_area[tk.index as usize].is_suspended);
}
#[test]
fn security_battle_really_uses_reduced_dp() {
    let mut r = base().security(1, &["R"]).start();
    r.place_on_field(0, ID, Some(0));
    let a = r.place_on_field(0, "Y", Some(0));
    attack(&mut r, a);
    accept(&mut r);
    r.auto_resolve().unwrap();
    assert!(
        r.game.players[0]
            .battle_area
            .iter()
            .any(|p| r.game.card_data[p.top_card().data_index].card_id == "Y"),
        "6000 attacker survives security 6000 reduced to 4000"
    );
    assert_eq!(r.security_count(1), 0);
}
#[test]
fn memory_gain_lock_blocks_both_start_turn_gains() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::PlayerModifierEntry;
    let mut r = base().security(0, &["R"]).security(1, &["R"]).start();
    r.place_on_field(0, ID, Some(0));
    r.game.modifiers.add_player_modifier(
        0,
        PlayerModifierEntry::simple(
            ModifierType::CannotGainMemoryByEffect,
            0,
            Expiry::Permanent,
            None,
            1,
        ),
    );
    r.end_turn();
    r.game.memory = -1;
    r.end_turn();
    assert_eq!(r.memory(), 1);
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
    let e = r.place_on_field(1, "R", Some(0));
    r.pass_turn();
    let before = r.memory();
    attack(&mut r, e);
    r.auto_resolve().unwrap();
    assert_eq!(r.memory(), before);
    assert_eq!(r.security_count(0), 0);
    assert!(r.game.players[0]
        .battle_area
        .iter()
        .any(|p| r.game.card_data[p.top_card().data_index].card_id == ID));
}

#[test]
fn observer_rejects_non_digimon_and_other_owner_event_subjects() {
    use digimon_engine::enums::EffectTiming;
    use digimon_engine::selection::TriggerSource;
    for (owner, kind) in [(0, "T"), (1, "R")] {
        let mut r = base().start();
        r.place_on_field(0, ID, Some(0));
        let h = r.place_on_field(owner, kind, Some(0));
        let card = r.game.players[owner as usize].battle_area[h.index as usize]
            .top_card()
            .handle();
        r.game.enqueue_triggered(
            EffectTiming::OnAllyAttack,
            TriggerSource::PlayerBattleAreaAttack {
                player: 0,
                attacker: h,
                card,
            },
        );
        r.game.drain_effect_queue();
        assert!(r.pending_selection().is_none());
        assert_eq!(r.game.defender_security_dp_adjustment(1), 0);
    }
}
#[test]
fn start_turn_does_not_fire_at_opponent_turn_start() {
    let mut r = base().security(0, &["R"]).security(1, &["R"]).start();
    r.place_on_field(0, ID, Some(0));
    r.end_turn();
    assert_eq!(r.game.turn_player_idx, 1);
    assert_eq!(r.memory(), -8);
}
