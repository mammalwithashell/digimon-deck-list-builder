//! Printed BT20-044: "Then, 1 of your Digimon may attack."
//! Select the Digimon first; the existing attack primitive still validates declaration.
use digimon_engine::{
    action::{
        build_action_mask,
        space::{encode_attack, encode_digivolve, PASS, SECURITY_TARGET},
    },
    debug_runner::{make_test_card, DebugRunner},
    enums::{CardColor, CardKind, Expiry, Keyword, ModifierType},
    modifiers::ModifierEntry,
    permanent::PermanentHandle,
    selection::SelectionKind,
};
fn fixture() -> DebugRunner {
    let y =
        std::fs::read_to_string(std::env::var("BT20_ATTACKER_YAML").unwrap_or_else(|_| {
            format!("{}/cards/bt20/BT20-044.yaml", env!("CARGO_MANIFEST_DIR"))
        }))
        .unwrap();
    let mut d = make_test_card("PICK-DIGIMON", "Unrelated Digimon");
    d.dp = Some(6000);
    d.level = Some(3);
    let mut filler = make_test_card("PICK-FILL", "Weak security");
    filler.dp = Some(1000);
    filler.level = Some(3);
    let mut t = make_test_card("PICK-TAMER", "Unrelated Tamer");
    t.card_kind = CardKind::Tamer;
    t.dp = None;
    t.level = None;
    let mut evo_host = make_test_card("PICK-EVOHOST", "Wingdramon");
    evo_host.colors = vec![CardColor::Blue];
    evo_host.level = Some(5);
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(&y)
        .unwrap()
        .add_card(d)
        .add_card(t)
        .add_card(filler)
        .add_card(evo_host)
        .deck(0, &["PICK-FILL"; 10])
        .deck(1, &["PICK-FILL"; 10])
        .security(1, &["PICK-FILL"; 5])
        .memory(10)
        .start();
    r.skip_mulligan();
    r.game.enter_main_phase();
    r
}
fn fresh(r: &mut DebugRunner) -> PermanentHandle {
    let turn = r.game.turn_count;
    r.place_on_field(0, "BT20-044", Some(turn))
}
fn act(r: &mut DebugRunner, a: u16) {
    let p = r.game.pending_selection.as_ref().unwrap().selecting_player;
    assert_eq!(
        build_action_mask(&r.game, p)[a as usize],
        1.0,
        "legal choice {a}"
    );
    r.game.resolve_selection(p, a).unwrap();
}
fn own_pick(r: &DebugRunner, id: &str) -> u16 {
    let p = r
        .game
        .pending_selection
        .as_ref()
        .expect("own Digimon chooser");
    assert_eq!(p.kind, SelectionKind::OwnField);
    let matches: Vec<_> = r.game.players[p.selecting_player as usize]
        .battle_area
        .iter()
        .enumerate()
        .filter(|(_, perm)| r.game.card_data[perm.top_card().data_index].card_id == id)
        .map(|(i, _)| encode_attack(0, i as u16))
        .filter(|a| p.valid_action_ids.contains(a))
        .collect();
    assert_eq!(
        matches.len(),
        1,
        "exact selectable physical card {id}: {:?}",
        p.valid_action_ids
    );
    matches[0]
}
fn pick(r: &mut DebugRunner, id: &str) {
    let a = own_pick(r, id);
    act(r, a);
}
fn start(r: &mut DebugRunner, h: PermanentHandle) {
    r.game.fire_on_play(0, h.index as usize);
    assert_eq!(
        r.game
            .pending_selection
            .as_ref()
            .expect("choose any own Digimon before validating attack")
            .kind,
        SelectionKind::OwnField
    );
}
fn unchanged(r: &DebugRunner) {
    assert!(r.game.pending_selection.is_none());
    assert!(r.game.pending_attack.is_none());
    assert_eq!(r.security_count(1), 5);
    assert_eq!(r.memory(), 10);
    assert!(!r.game.players[0].battle_area[0].is_suspended);
}
#[test]
fn fresh_only_attacker_is_selectable_but_cannot_declare_in_clones() {
    let mut r = fixture();
    let h = fresh(&mut r);
    start(&mut r, h);
    assert_eq!(
        build_action_mask(&r.game, 0)[own_pick(&r, "BT20-044") as usize],
        1.0
    );
    let saved = r.game.clone();
    for _ in 0..2 {
        r.game = saved.clone();
        pick(&mut r, "BT20-044");
        unchanged(&r);
    }
}

#[test]
fn real_digivolution_keeps_fresh_host_sickness_but_rush_and_ready_hosts_attack() {
    for (played_now, rush, may_attack) in [(true, false, false), (true, true, true), (false, false, true)] {
        let mut r = fixture();
        let played_turn = if played_now { r.game.turn_count } else { 0 };
        let host = r.place_on_field(0, "PICK-EVOHOST", Some(played_turn));
        if rush {
            r.game.modifiers.grant_keyword(host, Keyword::Rush, Expiry::EndOfTurn, 0);
        }
        r.add_to_hand(0, "BT20-044");
        let evolve = encode_digivolve(0, 0);
        assert_eq!(build_action_mask(&r.game, 0)[evolve as usize], 1.0);
        r.game.decode_action(evolve, 0);
        assert_eq!(r.game.players[0].battle_area[0].top_card().card_id(&r.game.card_data), "BT20-044");
        assert_eq!(r.game.players[0].battle_area[0].turn_played, played_turn);
        pick(&mut r, "BT20-044");
        if may_attack {
            let saved = r.game.clone();
            for _ in 0..2 {
                r.game = saved.clone();
                let attack = encode_attack(0, SECURITY_TARGET);
                assert_eq!(build_action_mask(&r.game, 0)[attack as usize], 1.0);
                act(&mut r, attack);
                assert_eq!(r.security_count(1), 4);
                assert!(r.game.pending_attack.is_none());
            }
        } else {
            assert!(r.game.pending_selection.is_none(), "digivolution cannot remove summoning sickness");
            assert!(r.game.pending_attack.is_none());
            assert!(!r.game.can_attack(host, false));
            assert_eq!(build_action_mask(&r.game, 0)[encode_attack(0, SECURITY_TARGET) as usize], 0.0);
            assert_eq!(r.security_count(1), 5);
        }
    }
}
#[test]
fn mixed_attacker_picker_exposes_fresh_and_ready_but_not_tamer() {
    let mut r = fixture();
    let h = fresh(&mut r);
    r.place_on_field(0, "PICK-DIGIMON", Some(0));
    r.place_on_field(0, "PICK-TAMER", None);
    start(&mut r, h);
    let mask = build_action_mask(&r.game, 0);
    assert_eq!(mask[own_pick(&r, "BT20-044") as usize], 1.0);
    assert_eq!(mask[own_pick(&r, "PICK-DIGIMON") as usize], 1.0);
    assert_eq!(mask[encode_attack(0, 2) as usize], 0.0);
    assert_eq!(mask[PASS as usize], 1.0);
    let saved = r.game.clone();
    r.game = saved.clone();
    pick(&mut r, "BT20-044");
    unchanged(&r);
    r.game = saved;
    pick(&mut r, "PICK-DIGIMON");
    let attack = encode_attack(1, SECURITY_TARGET);
    assert_eq!(build_action_mask(&r.game, 0)[attack as usize], 1.0);
    assert_eq!(
        build_action_mask(&r.game, 0)[encode_attack(0, SECURITY_TARGET) as usize],
        0.0
    );
    act(&mut r, attack);
    assert!(r.game.pending_selection.is_none());
    assert!(r.game.pending_attack.is_none());
    assert_eq!(r.security_count(1), 4);
    assert!(r.game.players[0].battle_area[1].is_suspended);
}
#[test]
fn restricted_attacker_can_be_picked_without_bypassing_declaration_rules() {
    for suspended in [false, true] {
        let mut r = fixture();
        let h = fresh(&mut r);
        let ally = r.place_on_field(0, "PICK-DIGIMON", Some(0));
        if suspended {
            r.game.players[0].battle_area[ally.index as usize].is_suspended = true;
        } else {
            r.game.modifiers.add(
                ally,
                ModifierEntry::simple(ModifierType::CannotAttack, 0, Expiry::Permanent, 0),
            );
        }
        start(&mut r, h);
        assert_eq!(
            build_action_mask(&r.game, 0)[own_pick(&r, "PICK-DIGIMON") as usize],
            1.0
        );
        let saved = r.game.clone();
        for _ in 0..2 {
            r.game = saved.clone();
            pick(&mut r, "PICK-DIGIMON");
            unchanged(&r);
            assert_eq!(
                r.game.players[0].battle_area[ally.index as usize].is_suspended,
                suspended
            );
        }
    }
}
#[test]
fn declining_attacker_selection_has_no_attack_or_suspension_in_clones() {
    let mut r = fixture();
    let h = fresh(&mut r);
    r.place_on_field(0, "PICK-DIGIMON", Some(0));
    start(&mut r, h);
    let saved = r.game.clone();
    for _ in 0..2 {
        r.game = saved.clone();
        act(&mut r, PASS);
        unchanged(&r);
        assert!(!r.game.players[0].battle_area[1].is_suspended);
    }
}
#[test]
fn declining_legal_attack_target_after_choosing_ready_attacker_is_independent() {
    let mut r = fixture();
    let h = fresh(&mut r);
    r.place_on_field(0, "PICK-DIGIMON", Some(0));
    start(&mut r, h);
    pick(&mut r, "PICK-DIGIMON");
    assert_eq!(build_action_mask(&r.game, 0)[PASS as usize], 1.0);
    let saved = r.game.clone();
    for _ in 0..2 {
        r.game = saved.clone();
        act(&mut r, PASS);
        unchanged(&r);
        assert!(!r.game.players[0].battle_area[1].is_suspended);
    }
}

#[test]
fn chosen_attacker_target_mask_preserves_player_and_unsuspended_target_restrictions() {
    let mut r = fixture();
    let h = fresh(&mut r);
    let attacker = r.place_on_field(0, "PICK-DIGIMON", Some(0));
    r.place_in_breeding(0, "PICK-EVOHOST");
    for _ in 0..3 { r.place_on_field(1, "PICK-FILL", Some(0)); }
    r.game.modifiers.add(attacker, ModifierEntry::simple(ModifierType::CannotAttackPlayer, 0, Expiry::Permanent, 0));
    r.game.fire_on_play(0, h.index as usize);
    // The printed preceding clause suspends two physical opponents.
    for _ in 0..2 {
        let p = r.pending_selection_view().expect("suspend opponent");
        let action = *p.valid_action_ids.iter().find(|a| **a != PASS).unwrap();
        act(&mut r, action);
    }
    assert_eq!(r.game.players[1].battle_area.iter().filter(|p| p.is_suspended).count(), 2);
    assert_eq!(r.pending_selection_view().unwrap().kind, SelectionKind::OwnField);
    assert_eq!(r.pending_selection_view().unwrap().valid_action_ids.len(), 2, "only two physical battle-area candidates; breeding is excluded");
    assert_eq!(build_action_mask(&r.game, 0)[PASS as usize], 1.0, "optional decline is separately mask-visible");
    pick(&mut r, "PICK-DIGIMON");
    let mask = build_action_mask(&r.game, 0);
    assert_eq!(mask[encode_attack(1, SECURITY_TARGET) as usize], 0.0);
    for index in 0..3 {
        assert_eq!(mask[encode_attack(1, index) as usize], if r.game.players[1].battle_area[index as usize].is_suspended { 1.0 } else { 0.0 });
    }
    assert_eq!(mask[PASS as usize], 1.0);
    let saved = r.game.clone();
    for _ in 0..2 {
        r.game = saved.clone();
        act(&mut r, PASS);
        assert!(!r.game.players[0].battle_area[1].is_suspended);
        assert_eq!(r.security_count(1), 5);
    }
}

fn player_only_mask_case(effect: bool, end_of_turn: bool, restricted_player: u8) {
    use digimon_engine::{enums::GamePhase, modifiers::PlayerModifierEntry};
    let mut r = fixture();
    let h = fresh(&mut r);
    let attacker = r.place_on_field(0, "PICK-DIGIMON", Some(0));
    let target = r.place_on_field(1, "PICK-FILL", Some(0));
    r.game.players[1].battle_area[0].is_suspended = true;
    r.game.modifiers.add_player_modifier(restricted_player, PlayerModifierEntry::simple(ModifierType::MayAttackPlayerOnly, 0, Expiry::Permanent, None, restricted_player));
    // Target-specific permission cannot override a restriction on the attacker.
    r.game.modifiers.add(target, ModifierEntry::simple(ModifierType::CanAttackTargetDefendingPermanent, 0, Expiry::Permanent, 1));
    if effect {
        r.game.fire_on_play(0, h.index as usize);
        let action = r.pending_selection_view().unwrap().valid_action_ids[0];
        act(&mut r, action);
        pick(&mut r, "PICK-DIGIMON");
    } else if end_of_turn {
        r.game.modifiers.add(attacker, ModifierEntry::simple(ModifierType::MayAttack, 0, Expiry::Permanent, 0));
        r.game.current_phase = GamePhase::EndOfTurnAction;
    }
    let saved = r.game.clone();
    for _ in 0..2 {
        r.game = saved.clone();
        let mask = build_action_mask(&r.game, 0);
        assert_eq!(mask[encode_attack(1, 0) as usize], if restricted_player == 0 { 0.0 } else { 1.0 });
        assert_eq!(mask[encode_attack(1, SECURITY_TARGET) as usize], 1.0);
    }
}

#[test]
fn player_only_modifier_filters_main_mask_without_affecting_other_player() {
    for player in [0, 1] { player_only_mask_case(false, false, player); }
}

#[test]
fn player_only_modifier_filters_end_of_turn_mask_without_affecting_other_player() {
    for player in [0, 1] { player_only_mask_case(false, true, player); }
}

#[test]
fn player_only_modifier_filters_real_bt20_effect_attack_mask_without_affecting_other_player() {
    for player in [0, 1] { player_only_mask_case(true, false, player); }
}

#[test]
fn suspension_clause_handles_one_two_and_mixed_tamer_targets_before_attacker_choice() {
    for opponents in [vec!["PICK-TAMER"], vec!["PICK-TAMER", "PICK-TAMER"], vec!["PICK-TAMER", "PICK-FILL"]] {
        let mut r = fixture();
        let h = fresh(&mut r);
        for card in &opponents { r.place_on_field(1, card, Some(0)); }
        r.game.fire_on_play(0, h.index as usize);
        for _ in 0..opponents.len() {
            let p = r.pending_selection_view().expect("suspend physical target");
            assert_ne!(p.kind, SelectionKind::OwnField);
            act(&mut r, p.valid_action_ids[0]);
        }
        assert_eq!(r.pending_selection_view().unwrap().kind, SelectionKind::OwnField);
        assert!(r.game.players[1].battle_area.iter().all(|p| p.is_suspended));
        act(&mut r, PASS);
        assert_eq!(r.security_count(1), 5);
    }
}

#[test]
fn effect_attack_respects_target_ban_and_explicit_target_permission() {
    for permission in [false, true] {
        let mut r = fixture();
        let h = fresh(&mut r);
        r.place_on_field(0, "PICK-DIGIMON", Some(0));
        let target = r.place_on_field(1, "PICK-FILL", Some(0));
        r.game.modifiers.add(target, ModifierEntry::simple(ModifierType::CannotAttackTarget, 0, Expiry::Permanent, 1));
        if permission { r.game.modifiers.add(target, ModifierEntry::simple(ModifierType::CanAttackTargetDefendingPermanent, 0, Expiry::Permanent, 1)); }
        r.game.fire_on_play(0, h.index as usize);
        let action = r.pending_selection_view().unwrap().valid_action_ids[0];
        act(&mut r, action);
        pick(&mut r, "PICK-DIGIMON");
        let mask = build_action_mask(&r.game, 0);
        assert_eq!(mask[encode_attack(1, 0) as usize], if permission { 1.0 } else { 0.0 });
        assert_eq!(mask[encode_attack(1, SECURITY_TARGET) as usize], 1.0);
        act(&mut r, PASS);
    }
}

// §11-2-7-1: an attack declaration targets the opponent or a suspended
// Digimon; only a "can attack unsuspended" grant (mode 2; §16-38-1 <Execute>)
// widens it. <Raid> (mode 1) switches the target AFTER the declaration
// (§16-23-1 / §16-23-2), so it adds no declaration target: inverted from the
// declaration-time <Raid> arm (G-ENGINE-RAID-DECLARATION-TARGET-UNSUSPENDED).
#[test]
fn effect_attack_widens_unsuspended_targets_only_with_permission() {
    for mode in 0..3 {
        let mut r = fixture();
        let h = fresh(&mut r);
        let attacker = r.place_on_field(0, "PICK-DIGIMON", Some(0));
        for _ in 0..3 { r.place_on_field(1, "PICK-FILL", Some(0)); }
        if mode == 1 { r.game.modifiers.grant_keyword(attacker, Keyword::Raid, Expiry::Permanent, 0); }
        if mode == 2 { r.game.modifiers.add(attacker, ModifierEntry::simple(ModifierType::CanAttackUnsuspended, 0, Expiry::Permanent, 0)); }
        r.game.fire_on_play(0, h.index as usize);
        for _ in 0..2 { let action = r.pending_selection_view().unwrap().valid_action_ids[0]; act(&mut r, action); }
        assert_eq!(r.game.players[1].battle_area.iter().filter(|p| p.is_suspended).count(), 2);
        pick(&mut r, "PICK-DIGIMON");
        let mask = build_action_mask(&r.game, 0);
        for index in 0..3 { assert_eq!(mask[encode_attack(1, index) as usize], if mode == 2 || r.game.players[1].battle_area[index as usize].is_suspended { 1.0 } else { 0.0 }); }
        act(&mut r, PASS);
    }
}

#[test]
fn effect_attacker_selection_does_not_bypass_player_ban_or_suspension_cost() {
    use digimon_engine::modifiers::PlayerModifierEntry;
    for player_ban in [false, true] {
        let mut r = fixture();
        let h = fresh(&mut r);
        let attacker = r.place_on_field(0, "PICK-DIGIMON", Some(0));
        if player_ban {
            r.game.modifiers.add_player_modifier(0, PlayerModifierEntry::simple(ModifierType::CannotAttack, 0, Expiry::Permanent, None, 0));
        } else {
            r.game.modifiers.add(attacker, ModifierEntry::simple(ModifierType::CannotSuspend, 0, Expiry::Permanent, 0));
        }
        start(&mut r, h);
        pick(&mut r, "PICK-DIGIMON");
        assert!(r.game.pending_selection.is_none());
        assert!(r.game.pending_attack.is_none());
        assert!(!r.game.players[0].battle_area[1].is_suspended);
        assert_eq!(r.security_count(1), 5);
    }
}
