use digimon_engine::action::space::{encode_digivolve, PASS, REPLACEMENT_ACCEPT};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, Keyword};
const ID: &str = "EX3-018";
fn base() -> DebugRunnerBuilder {
    let mut b = DebugRunner::builder()
        .from_dsl_yaml(include_str!("../../../cards/ex3/EX3-018.yaml"))
        .expect("EX3-018 production YAML");
    for (id, name, level, color) in [
        ("B", "Blue", 3, CardColor::Blue),
        ("G", "Green", 3, CardColor::Green),
        ("R", "Red", 3, CardColor::Red),
        ("D", "Dracomon X", 5, CardColor::Red),
        ("N", "Dracomon", 3, CardColor::Red),
        ("DR", "Airdramon", 5, CardColor::Red),
        ("EX", "Examon X", 7, CardColor::White),
        ("P", "Plain", 5, CardColor::Blue),
    ] {
        let mut c = make_test_card(id, name);
        c.level = Some(level);
        c.colors = vec![color];
        b = b.add_card(c);
    }
    b.deck(0, &["P"; 10]).deck(1, &["P"; 10]).memory(10)
}
#[test]
fn metadata_and_evolution_routes() {
    for (id, cost) in [
        ("B", Some(3)),
        ("G", Some(3)),
        ("R", None),
        ("D", Some(2)),
        ("N", Some(2)),
        ("P", None),
    ] {
        let mut r = base().hand(0, &[ID]).start();
        r.skip_mulligan();
        let h = r.place_on_field(0, id, Some(0));
        let a = encode_digivolve(0, h.index as u16);
        assert_eq!(
            digimon_engine::action::build_action_mask(&r.game, 0)[a as usize] > 0.0,
            cost.is_some(),
            "{id}"
        );
        if let Some(c) = cost {
            r.game.decode_action(a, 0);
            assert_eq!(r.memory(), 10 - c);
            assert_eq!(r.effective_dp(h), Some(6000));
        }
    }
}
#[test]
fn face_evade_accept_decline_and_paid_suspension() {
    for action in [REPLACEMENT_ACCEPT, PASS] {
        let mut r = base().start();
        r.skip_mulligan();
        let h = r.place_on_field(0, ID, Some(0));
        r.game.delete_permanent_with_effects(h);
        for a in [REPLACEMENT_ACCEPT, PASS] {
            if a == REPLACEMENT_ACCEPT {
                assert!(r.pending_selection().unwrap().valid_action_ids.contains(&a));
            }
            assert_eq!(
                digimon_engine::action::build_action_mask(&r.game, 0)[a as usize],
                1.0
            );
        }
        r.game.resolve_selection(0, action).unwrap();
        if action == PASS {
            assert_eq!(r.battle_area_size(0), 0);
        } else {
            assert!(r.game.players[0].battle_area[0].is_suspended);
            assert_eq!(r.battle_area_size(0), 1);
            r.game.delete_permanent_with_effects(h);
            assert!(r.pending_selection().is_none());
            assert_eq!(r.battle_area_size(0), 0);
        }
    }
}
#[test]
fn inherited_carrier_names_and_both_turns() {
    for turn in [0, 1] {
        for (name, granted) in [("DR", true), ("EX", true), ("P", false)] {
            let mut r = base().start();
            r.skip_mulligan();
            r.game.turn_player_idx = turn;
            let h = r.place_stack(0, &[ID, name]);
            let other = r.place_on_field(0, "DR", None);
            let enemy = r.place_on_field(1, "DR", None);
            r.game.tick_declarative_effects();
            assert_eq!(
                r.game.has_keyword(h, Keyword::Evade),
                granted,
                "{name} turn {turn}"
            );
            assert!(!r.game.has_keyword(other, Keyword::Evade));
            assert!(!r.game.has_keyword(enemy, Keyword::Evade));
            r.game.delete_permanent_with_effects(h);
            if granted {
                assert_eq!(
                    digimon_engine::action::build_action_mask(&r.game, 0)
                        [REPLACEMENT_ACCEPT as usize],
                    1.0
                );
                r.game.resolve_selection(0, REPLACEMENT_ACCEPT).unwrap();
                assert!(r.game.players[0].battle_area[0].is_suspended);
            } else {
                assert!(r.pending_selection().is_none());
                assert_eq!(r.battle_area_size(0), 1);
            }
        }
    }
}
#[test]
fn inherited_grant_rechecks_carrier_after_stack_changes() {
    let mut r = base().start();
    r.skip_mulligan();
    let h = r.place_stack(0, &[ID, "DR", "P"]);
    r.game.tick_declarative_effects();
    assert!(!r.game.has_keyword(h, Keyword::Evade));
    r.game.players[0].battle_area[0].card_sources.pop();
    r.game.tick_declarative_effects();
    assert!(r.game.has_keyword(h, Keyword::Evade));
}
#[test]
fn cannot_pay_evade_suspend_cost() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::ModifierEntry;
    let mut r = base().start();
    r.skip_mulligan();
    let h = r.place_on_field(0, ID, None);
    r.game.modifiers.add(
        h,
        ModifierEntry::simple(ModifierType::CannotSuspend, 0, Expiry::Permanent, 1),
    );
    r.game.delete_permanent_with_effects(h);
    assert!(r.pending_selection().is_none());
    assert_eq!(r.battle_area_size(0), 0);
}

#[test]
fn inherited_evade_is_optional_and_cannot_save_an_already_suspended_carrier() {
    for already_suspended in [false, true] {
        let mut r = base().start();
        r.skip_mulligan();
        let h = r.place_stack(0, &[ID, "DR"]);
        r.game.tick_declarative_effects();
        r.game.players[0].battle_area[h.index as usize].is_suspended = already_suspended;
        r.game.delete_permanent_with_effects(h);
        if !already_suspended {
            let mask = digimon_engine::action::build_action_mask(&r.game, 0);
            assert_eq!(mask[REPLACEMENT_ACCEPT as usize], 1.0);
            assert_eq!(mask[PASS as usize], 1.0);
            r.game.resolve_selection(0, PASS).unwrap();
        } else {
            assert!(r.pending_selection().is_none());
        }
        assert_eq!(r.battle_area_size(0), 0);
        assert_eq!(r.game.players[0].trash.len(), 2);
    }
}

#[test]
fn actual_digivolution_transfers_evade_only_to_a_qualifying_carrier() {
    for (yaml, qualifies) in [
        (include_str!("../../../cards/st9/ST9-05.yaml"), true),
        (include_str!("../../../cards/st2/ST2-09.yaml"), false),
    ] {
        let id = if qualifies { "ST9-05" } else { "ST2-09" };
        let mut r = base().from_dsl_yaml(yaml).unwrap().hand(0, &[id]).start();
        r.skip_mulligan();
        let h = r.place_on_field(0, ID, None);
        let action = encode_digivolve(0, h.index as u16);
        assert_eq!(digimon_engine::action::build_action_mask(&r.game, 0)[action as usize], 1.0);
        r.game.decode_action(action, 0);
        assert_eq!(r.game.has_keyword(h, Keyword::Evade), qualifies);
        r.game.delete_permanent_with_effects(h);
        if qualifies {
            assert_eq!(digimon_engine::action::build_action_mask(&r.game, 0)[REPLACEMENT_ACCEPT as usize], 1.0);
            r.game.resolve_selection(0, REPLACEMENT_ACCEPT).unwrap();
            assert_eq!(r.battle_area_size(0), 1);
            assert!(r.game.players[0].battle_area[h.index as usize].is_suspended);
        } else {
            assert!(r.pending_selection().is_none());
            assert_eq!(r.battle_area_size(0), 0);
        }
    }
}
