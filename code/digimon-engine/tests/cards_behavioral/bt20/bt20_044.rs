use digimon_engine::action::{
    mask::build_action_mask,
    space::{encode_digivolve, ATTACK_START, PASS},
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, GamePhase};
const ID: &str = "BT20-044";
fn setup(hand: &[&str]) -> DebugRunner {
    let mut b = DebugRunner::builder()
        .from_dsl_yaml(include_str!("../../../cards/bt20/BT20-044.yaml"))
        .unwrap();
    for (id, name, color, level, dp, text) in [
        ("ALLY", "Ally", CardColor::Blue, 6, 15000, "Examon"),
        ("DRACO", "Ally", CardColor::Blue, 6, 15000, "Dracomon"),
        ("BAD", "Unrelated", CardColor::Blue, 6, 15000, ""),
        ("WEAK", "Weak", CardColor::Blue, 3, 1000, ""),
        ("TIE", "Tie", CardColor::Blue, 6, 15000, ""),
        ("FACE_TIE", "Tie", CardColor::Blue, 6, 12000, ""),
        ("GREEN", "Green", CardColor::Green, 5, 7000, ""),
        ("RED", "Red", CardColor::Red, 5, 7000, ""),
        ("GROUND", "Groundramon", CardColor::Blue, 5, 7000, ""),
        ("WING", "Wingdramon", CardColor::Blue, 5, 7000, ""),
        ("NEAR", "Wingdramon X", CardColor::Blue, 5, 7000, ""),
    ] {
        let mut c = make_test_card(id, name);
        c.colors = vec![color];
        c.level = Some(level);
        c.dp = Some(dp);
        c.effect_text = text.into();
        b = b.add_card(c);
    }
    let mut t = make_test_card("TAMER", "Tamer");
    t.card_kind = CardKind::Tamer;
    t.dp = None;
    t.level = None;
    b = b.add_card(t);
    let mut r = b
        .hand(0, hand)
        .deck(0, &["WEAK"; 15])
        .deck(1, &["WEAK"; 15])
        .security(1, &["WEAK"; 5])
        .memory(10)
        .start();
    r.skip_mulligan();
    r.game.current_phase = GamePhase::Main;
    r
}
fn choose(r: &mut DebugRunner, a: u16) {
    let v = r.pending_selection_view().expect("selection");
    assert!(
        (a == PASS && v.is_optional) || v.valid_action_ids.contains(&a),
        "{v:?} lacks {a}"
    );
    assert!(build_action_mask(&r.game, 0)[a as usize] > 0.5);
    r.execute_action(0, a).unwrap();
}
fn exact_choices(r: &DebugRunner, expected: &[u16], optional: bool) {
    let v = r.pending_selection_view().expect("selection");
    assert_eq!(v.valid_action_ids, expected);
    assert_eq!(v.is_optional, optional);
    let mask = build_action_mask(&r.game, 0);
    let mut legal: Vec<_> = expected.iter().copied().collect();
    if optional && !legal.contains(&PASS) {
        legal.push(PASS);
    }
    legal.sort_unstable();
    let actual: Vec<_> = mask
        .iter()
        .enumerate()
        .filter_map(|(a, &m)| (m > 0.5).then_some(a as u16))
        .collect();
    assert_eq!(actual, legal);
}
#[test]
fn bt20_044_suspend_two_distinct_then_decline_clone() {
    let mut r = setup(&[ID]);
    r.place_on_field(0, "ALLY", Some(0));
    r.place_on_field(1, "WEAK", Some(0));
    r.place_on_field(1, "TAMER", Some(0));
    r.play(0, 0).unwrap();
    exact_choices(&r, &[ATTACK_START, ATTACK_START + 1], false);
    choose(&mut r, ATTACK_START);
    exact_choices(&r, &[ATTACK_START + 1], false);
    assert!(!r
        .pending_selection_view()
        .unwrap()
        .valid_action_ids
        .contains(&ATTACK_START));
    let mut c = DebugRunner::wrap(r.game.clone());
    for r in [&mut r, &mut c] {
        choose(r, ATTACK_START + 1);
        assert!(r.game.players[1].battle_area.iter().all(|p| p.is_suspended));
        choose(r, PASS);
        assert_eq!(r.security_count(1), 5);
    }
}
#[test]
fn bt20_044_attack_tail_zero_one_targets_and_sickness() {
    for count in 0..=1 {
        let mut r = setup(&[ID]);
        r.place_on_field(0, "ALLY", Some(0));
        if count == 1 {
            r.place_on_field(1, "TAMER", Some(0));
        }
        r.play(0, 0).unwrap();
        if count == 1 {
            choose(&mut r, ATTACK_START);
        }
        // Choosing a fresh Digimon is legal; declaring its attack is not.
        exact_choices(&r, &[ATTACK_START, ATTACK_START + 1], true);
        let mut fresh = DebugRunner::wrap(r.game.clone());
        assert_eq!(fresh.security_count(1), 5);
        choose(&mut fresh, ATTACK_START + 1);
        assert!(fresh.game.pending_selection.is_none());
        assert!(fresh.game.pending_attack.is_none());
        assert_eq!(fresh.security_count(1), 5);
        assert!(fresh.game.players[0]
            .battle_area
            .iter()
            .all(|permanent| !permanent.is_suspended));
        choose(&mut r, ATTACK_START);
        let v = r.pending_selection_view().unwrap();
        let a = *v.valid_action_ids.iter().find(|&&a| a != PASS).unwrap();
        choose(&mut r, a);
        r.auto_resolve().unwrap();
        assert_eq!(r.security_count(1), 4);
    }
}
#[test]
fn bt20_044_routes() {
    for (id, cost) in [
        ("GREEN", 4),
        ("RED", 4),
        ("GROUND", 3),
        ("WING", 3),
        ("NEAR", 0),
    ] {
        let mut r = setup(&[ID]);
        let h = r.place_on_field(0, id, Some(0));
        let a = encode_digivolve(0, h.index as u16);
        assert_eq!(build_action_mask(&r.game, 0)[a as usize] > 0.5, cost > 0);
        if cost == 0 {
            continue;
        }
        r.place_on_field(1, "TAMER", Some(0));
        r.game.decode_action(a, 0);
        assert_eq!(r.memory(), 10 - cost);
        choose(&mut r, ATTACK_START);
        choose(&mut r, PASS);
        assert!(r.game.players[1].battle_area[0].is_suspended);
    }
}
#[test]
fn bt20_044_face_and_inherited_other_ally_battle_opt_reset() {
    for inherited in [false, true] {
        let mut r = setup(&[]);
        if inherited {
            r.place_stack(0, &[ID, "BAD"]);
        } else {
            r.place_on_field(0, ID, Some(0));
        }
        let ally = r.place_on_field(0, "ALLY", Some(0));
        for n in 0..3 {
            if n == 2 {
                r.end_turn();
                r.auto_resolve().unwrap();
            }
            let target = r.place_on_field(1, "TAMER", Some(0));
            r.game.players[1].battle_area[target.index as usize].is_suspended = true;
            let weak = r.place_on_field(1, "WEAK", Some(0));
            r.battle_digimon(ally, weak);
            if n == 1 {
                assert!(r.pending_selection().is_none());
            } else {
                choose(&mut r, ATTACK_START + target.index as u16);
            }
            r.auto_resolve().unwrap();
            assert_eq!(
                r.game.players[1].battle_area.len(),
                if n == 0 { 0 } else { 1 }
            );
        }
    }
}
#[test]
fn bt20_044_negative_text_and_security() {
    for inherited in [false, true] {
        for kind in [0, 2] {
            let mut r = setup(&[]);
            if inherited {
                r.place_stack(0, &[ID, "BAD"]);
            } else {
                r.place_on_field(0, ID, Some(0));
            }
            let ally = r.place_on_field(0, if kind == 0 { "BAD" } else { "DRACO" }, Some(0));
            let t = r.place_on_field(1, "TAMER", Some(0));
            r.game.players[1].battle_area[t.index as usize].is_suspended = true;
            if kind == 2 {
                r.attack_player(ally, 1, false);
            } else {
                let w = r.place_on_field(1, "WEAK", Some(0));
                r.battle_digimon(ally, w);
            }
            r.auto_resolve().unwrap();
            assert_eq!(r.battle_area_size(1), 1);
        }
    }
}
#[test]
fn bt20_044_delete_filter_and_dracomon_text() {
    let mut r = setup(&[]);
    r.place_on_field(0, ID, Some(0));
    let a = r.place_on_field(0, "DRACO", Some(0));
    r.place_on_field(1, "TAMER", Some(0));
    let t = r.place_on_field(1, "TAMER", Some(0));
    r.game.players[1].battle_area[t.index as usize].is_suspended = true;
    let w = r.place_on_field(1, "WEAK", Some(0));
    r.battle_digimon(a, w);
    assert_eq!(
        r.pending_selection_view().unwrap().valid_action_ids,
        vec![ATTACK_START + 1]
    );
    choose(&mut r, ATTACK_START + 1);
    assert_eq!(r.game.players[1].battle_area.len(), 1);
}

#[test]
fn bt20_044_blocker_face_only_and_prevented_deletion() {
    use digimon_engine::enums::{Expiry, Keyword, ModifierType};
    use digimon_engine::modifiers::ModifierEntry;
    let mut r = setup(&[]);
    let face = r.place_on_field(0, ID, Some(0));
    let inherited = r.place_stack(0, &[ID, "BAD"]);
    r.game.tick_declarative_effects();
    assert!(r.game.has_keyword(face, Keyword::Blocker));
    assert!(!r.game.has_keyword(inherited, Keyword::Blocker));
    // Use one observer for the battle negative so simultaneous triggers cannot obscure it.
    let mut r = setup(&[]);
    r.place_on_field(0, ID, Some(0));
    let ally = r.place_on_field(0, "ALLY", Some(0));
    let target = r.place_on_field(1, "TAMER", Some(0));
    r.game.players[1].battle_area[target.index as usize].is_suspended = true;
    let weak = r.place_on_field(1, "WEAK", Some(0));
    r.game.modifiers.add(
        weak,
        ModifierEntry::simple(
            ModifierType::CannotBeDestroyedByBattle,
            0,
            Expiry::Permanent,
            1,
        ),
    );
    r.battle_digimon(ally, weak);
    assert!(r.pending_selection().is_none());
    assert_eq!(r.battle_area_size(1), 2);
}

// A different ally may mutually delete its battle opponent while this observer survives.
// Q4364/Q4367 exclude a departed observer, not this case. DCGO BT20_044 uses
// isOnlyWinnerSurvive:false; CardController includes both tied combatants in snapshots.
#[test]
fn bt20_044_other_ally_mutual_battle_still_triggers() {
    for inherited in [false, true] {
        let mut r = setup(&[]);
        if inherited {
            r.place_stack(0, &[ID, "BAD"]);
        } else {
            r.place_on_field(0, ID, Some(0));
        }
        let ally = r.place_on_field(0, "DRACO", Some(0));
        let t = r.place_on_field(1, "TAMER", Some(0));
        r.game.players[1].battle_area[t.index as usize].is_suspended = true;
        let opponent = r.place_on_field(1, "TIE", Some(0));
        r.battle_digimon(ally, opponent);
        assert_eq!(r.battle_area_size(0), 1);
        assert_eq!(r.battle_area_size(1), 1);
        assert!(r.pending_selection().is_some(),"Surviving observer must trigger when another qualifying ally mutually deletes an opponent");
        exact_choices(&r, &[ATTACK_START], false);
        let mut cloned = DebugRunner::wrap(r.game.clone());
        for branch in [&mut r, &mut cloned] {
            choose(branch, ATTACK_START);
            assert_eq!(branch.battle_area_size(1), 0);
        }
    }
}

// Official Q4364/Q4367: the observer cannot activate after its own deletion.
#[test]
fn bt20_044_deleted_observer_does_not_trigger_after_mutual_battle() {
    for inherited in [false, true] {
        let mut r = setup(&[]);
        let observer = if inherited {
            r.place_stack(0, &[ID, "DRACO"])
        } else {
            r.place_on_field(0, ID, Some(0))
        };
        let t = r.place_on_field(1, "TAMER", Some(0));
        r.game.players[1].battle_area[t.index as usize].is_suspended = true;
        let opponent = r.place_on_field(1, if inherited { "TIE" } else { "FACE_TIE" }, Some(0));
        r.battle_digimon(observer, opponent);
        assert_eq!(r.battle_area_size(0), 0);
        assert_eq!(r.battle_area_size(1), 1);
        assert!(r.pending_selection().is_none());
    }
}

#[test]
fn bt20_044_no_suspended_deletion_targets_and_opponent_winner() {
    for inherited in [false, true] {
        for opponent_wins in [false, true] {
            let mut r = setup(&[]);
            if inherited {
                r.place_stack(0, &[ID, "BAD"]);
            } else {
                r.place_on_field(0, ID, Some(0));
            }
            let t = r.place_on_field(1, "TAMER", Some(0));
            r.game.players[1].battle_area[t.index as usize].is_suspended = opponent_wins;
            if opponent_wins {
                let weak = r.place_on_field(0, "WEAK", Some(0));
                let enemy = r.place_on_field(1, "DRACO", Some(0));
                r.battle_digimon(enemy, weak);
            } else {
                let ally = r.place_on_field(0, "DRACO", Some(0));
                let weak = r.place_on_field(1, "WEAK", Some(0));
                r.battle_digimon(ally, weak);
            }
            assert!(r.pending_selection().is_none());
            assert_eq!(r.battle_area_size(1), if opponent_wins { 2 } else { 1 });
        }
    }
}
