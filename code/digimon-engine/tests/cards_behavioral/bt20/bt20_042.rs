use digimon_engine::action::space::{encode_digivolve, ATTACK_START, DNA_DIGIVOLVE_START};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, GamePhase, ModifierType};
use digimon_engine::permanent::PermanentHandle;
const ID: &str = "BT20-042";
fn setup(hand: &[&str]) -> DebugRunner {
    let yaml = include_str!("../../../cards/bt20/BT20-042.yaml");
    let mut b = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .unwrap()
        .dsl_card("EX13-045")
        .unwrap();
    for (id, name, color, level, dp) in [
        ("CORE", "Coredramon", CardColor::Blue, 4, 5000),
        ("GREEN", "Green", CardColor::Green, 4, 5000),
        ("RED", "Red", CardColor::Red, 4, 5000),
        ("BAD", "Coredramon X", CardColor::Blue, 4, 5000),
        ("BLUE", "Blue", CardColor::Blue, 6, 11000),
        ("TOP", "Top", CardColor::Red, 6, 11000),
        ("WEAK", "Weak", CardColor::Red, 3, 3000),
        ("TIE", "Tie", CardColor::Red, 6, 11000),
        ("FILL", "Fill", CardColor::Red, 3, 1000),
    ] {
        let mut c = make_test_card(id, name);
        c.colors = vec![color];
        c.level = Some(level);
        c.dp = Some(dp);
        b = b.add_card(c);
    }
    let mut t = make_test_card("TAMER", "Tamer");
    t.card_kind = CardKind::Tamer;
    t.level = None;
    t.dp = None;
    b = b.add_card(t);
    let mut r = b
        .hand(0, hand)
        .deck(0, &["FILL"; 10])
        .deck(1, &["FILL"; 10])
        .security(1, &["FILL"; 5])
        .memory(10)
        .start();
    r.skip_mulligan();
    r.game.current_phase = GamePhase::Main;
    r
}
fn pick(r: &mut DebugRunner, h: PermanentHandle) {
    let v = r.pending_selection_view().expect("mandatory selection");
    assert!(!v.is_optional);
    let a = ATTACK_START + h.index as u16;
    assert!(v.valid_action_ids.contains(&a));
    assert!(digimon_engine::action::mask::build_action_mask(&r.game, 0)[a as usize] > 0.5);
    r.execute_action(0, a).unwrap();
}
#[test]
fn bt20_042_independent_digimon_tamer_choices_and_lock_expiry() {
    for reverse in [false, true] {
        let mut r = setup(&[ID]);
        let d = r.place_on_field(1, "WEAK", Some(0));
        let t = r.place_on_field(1, "TAMER", Some(0));
        r.play(0, 0).unwrap();
        assert_eq!(
            r.pending_selection_view().unwrap().valid_action_ids,
            vec![ATTACK_START, ATTACK_START + 1]
        );
        let (a, b) = if reverse { (t, d) } else { (d, t) };
        pick(&mut r, a);
        assert_eq!(
            r.pending_selection_view().unwrap().valid_action_ids,
            vec![ATTACK_START, ATTACK_START + 1]
        );
        pick(&mut r, b);
        assert!(r.game.players[1].battle_area[a.index as usize].is_suspended);
        assert!(r.game.modifiers.has(b, ModifierType::CannotUnsuspend));
        assert!(!r.game.modifiers.has(a, ModifierType::CannotUnsuspend));
        r.game.players[1].battle_area[b.index as usize].is_suspended = true;
        r.game.unsuspend_with_cause(b, true);
        assert!(r.game.players[1].battle_area[b.index as usize].is_suspended);
        r.end_turn();
        r.auto_resolve().unwrap();
        assert!(r.game.modifiers.has(b, ModifierType::CannotUnsuspend));
        r.end_turn();
        r.auto_resolve().unwrap();
        assert!(!r.game.modifiers.has(b, ModifierType::CannotUnsuspend));
        r.game.unsuspend_with_cause(b, true);
        assert!(!r.game.players[1].battle_area[b.index as usize].is_suspended);
    }
}
#[test]
fn bt20_042_native_named_routes_and_when_digivolving() {
    for (source, cost) in [("CORE", 3), ("GREEN", 4), ("RED", 4), ("BAD", 0)] {
        let mut r = setup(&[ID]);
        let h = r.place_on_field(0, source, Some(0));
        let a = encode_digivolve(0, h.index as u16);
        let mask = digimon_engine::action::mask::build_action_mask(&r.game, 0);
        assert_eq!(mask[a as usize] > 0.5, cost > 0);
        if cost == 0 {
            continue;
        }
        let opp = r.place_on_field(1, "TAMER", Some(0));
        r.game.decode_action(a, 0);
        assert_eq!(r.memory(), 10 - cost);
        pick(&mut r, opp);
        pick(&mut r, opp);
        assert!(r.game.modifiers.has(opp, ModifierType::CannotUnsuspend));
    }
    let mut r = setup(&[ID]);
    r.play(0, 0).unwrap();
    assert!(r.pending_selection().is_none());
    assert_eq!(r.memory(), 3);
}
#[test]
fn bt20_042_identity_only_on_field_for_examon_and_not_inherited() {
    let mut r = setup(&["EX13-045", ID, "FILL"]);
    let h = r.place_on_field(0, ID, Some(0));
    r.place_on_field(0, "BLUE", Some(0));
    r.game.tick_declarative_effects();
    let ex = r.game.players[0].hand[0].clone();
    let other = r.game.players[0].hand[2].clone();
    let e = r.game.dna_material_extras(h, &ex);
    assert_eq!(e.levels, vec![6]);
    assert_eq!(e.names, vec!["Breakdramon"]);
    assert!(r.game.dna_material_extras(h, &other).levels.is_empty());
    assert!(
        digimon_engine::action::mask::build_action_mask(&r.game, 0)[DNA_DIGIVOLVE_START as usize]
            > 0.5
    );
    let meta = &r.game.card_data[ex.handle().0 as usize];
    let mat = &r.game.players[0].hand[1];
    let ground = &r.game.card_data[mat.handle().0 as usize];
    assert!(
        digimon_engine::dna_digivolve::matching_dna_cost_perm_and_card(
            meta,
            &r.game.players[0].battle_area[1],
            ground,
            &r.game.card_data
        )
        .is_none()
    );
    let inherited = r.place_stack(0, &[ID, "TOP"]);
    r.game.tick_declarative_effects();
    assert!(r.game.dna_material_extras(inherited, &ex).names.is_empty());
}
#[test]
fn bt20_042_inherited_self_only_opt_both_turns_and_reset() {
    let mut r = setup(&[]);
    let me = r.place_stack(0, &[ID, "TOP"]);
    let ally = r.place_on_field(0, "TOP", Some(0));
    let opp = r.place_on_field(1, "WEAK", Some(0));
    r.battle_digimon(ally, opp);
    r.auto_resolve().unwrap();
    assert_eq!(r.security_count(1), 5);
    for expected in [4, 4] {
        let opp = r.place_on_field(1, "WEAK", Some(0));
        r.battle_digimon(me, opp);
        r.auto_resolve().unwrap();
        assert_eq!(r.security_count(1), expected);
    }
    r.end_turn();
    r.auto_resolve().unwrap();
    let opp = r.place_on_field(1, "WEAK", Some(0));
    r.battle_digimon(me, opp);
    r.auto_resolve().unwrap();
    assert_eq!(r.security_count(1), 3);
}
#[test]
fn bt20_042_mutual_deletion_and_face_have_no_inherited_trigger() {
    for (top, target) in [(false, "WEAK"), (true, "TIE")] {
        let mut r = setup(&[]);
        let me = if top {
            r.place_stack(0, &[ID, "TOP"])
        } else {
            r.place_on_field(0, ID, Some(0))
        };
        let opp = r.place_on_field(1, target, Some(0));
        r.battle_digimon(me, opp);
        r.auto_resolve().unwrap();
        assert_eq!(r.security_count(1), 5);
    }
}

#[test]
fn bt20_042_prevented_battle_deletion_and_opponent_wins_do_not_trigger() {
    use digimon_engine::enums::Expiry;
    use digimon_engine::modifiers::ModifierEntry;
    let mut r = setup(&[]);
    let me = r.place_stack(0, &[ID, "TOP"]);
    let opp = r.place_on_field(1, "WEAK", Some(0));
    r.game.modifiers.add(
        opp,
        ModifierEntry::simple(
            ModifierType::CannotBeDestroyedByBattle,
            0,
            Expiry::Permanent,
            1,
        ),
    );
    r.battle_digimon(me, opp);
    r.auto_resolve().unwrap();
    assert_eq!(r.battle_area_size(1), 1);
    assert_eq!(r.security_count(1), 5);
    let opponent_winner = r.place_on_field(1, "TOP", Some(0));
    let own_loser = r.place_on_field(0, "WEAK", Some(0));
    r.battle_digimon(opponent_winner, own_loser);
    r.auto_resolve().unwrap();
    assert_eq!(r.security_count(1), 5);
}
#[test]
fn bt20_042_lock_installed_on_opponent_turn_expires_that_turn() {
    let mut r = setup(&[]);
    let opp = r.place_on_field(1, "TAMER", Some(0));
    r.end_turn();
    r.auto_resolve().unwrap();
    assert_eq!(r.turn_player(), 1);
    // Direct end_turn does not perform the pass action's memory payment.
    r.game.memory = 3;
    r.game.current_phase = GamePhase::Main;
    let me = r.place_on_field(0, ID, Some(0));
    r.fire_play_event_triggers(0, me.index as usize, true, false);
    pick(&mut r, opp);
    pick(&mut r, opp);
    assert!(r.game.modifiers.has(opp, ModifierType::CannotUnsuspend));
    r.end_turn();
    r.auto_resolve().unwrap();
    assert!(!r.game.modifiers.has(opp, ModifierType::CannotUnsuspend));
}

