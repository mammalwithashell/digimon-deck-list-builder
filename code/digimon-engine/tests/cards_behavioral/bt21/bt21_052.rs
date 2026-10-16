//! BT21-052: official observation 0fda0130 and card image; DCGO cross-check.
use digimon_engine::action::space::{encode_digivolve, PASS};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Keyword};
use digimon_engine::selection::TriggerSource;
const ID: &str = "BT21-052";
fn base() -> DebugRunnerBuilder {
    let yaml = include_str!("../../../cards/bt21/BT21-052.yaml");
    let mut b = DebugRunner::builder().from_dsl_yaml(yaml).unwrap();
    for (id, name, kind, level, color) in [
        ("EX", "Examon", CardKind::Digimon, 7, CardColor::White),
        ("XA", "X Antibody", CardKind::Option, 3, CardColor::White),
        (
            "NEAR",
            "Examon Variant",
            CardKind::Digimon,
            7,
            CardColor::White,
        ),
        ("D", "Other", CardKind::Digimon, 6, CardColor::Green),
        ("R", "Red", CardKind::Digimon, 6, CardColor::Red),
        ("B", "Blue", CardKind::Digimon, 6, CardColor::Blue),
        ("T", "Tamer", CardKind::Tamer, 3, CardColor::White),
        ("O", "Option", CardKind::Option, 3, CardColor::White),
    ] {
        let mut c = make_test_card(id, name);
        c.card_kind = kind;
        c.level = Some(level);
        c.colors = vec![color];
        b = b.add_card(c);
    }
    b.deck(0, &["D"; 10])
        .deck(1, &["D"; 10])
        .security(1, &["D"; 4])
        .memory(10)
}
#[test]
fn keywords_are_top_only() {
    let mut r = base().start();
    let h = r.place_on_field(0, ID, Some(0));
    let carrier = r.place_stack(0, &[ID, "D"]);
    for k in [Keyword::Piercing, Keyword::Blocker, Keyword::Evade] {
        assert!(r.game.has_keyword(h, k));
        assert!(!r.game.has_keyword(carrier, k));
    }
}
#[test]
fn suspend_all_then_mandatory_delete_digimon_or_tamer() {
    for chosen in [0, 1] {
        let mut r = base().start();
        r.skip_mulligan();
        let h = r.place_on_field(0, ID, Some(0));
        r.place_on_field(1, "D", Some(0));
        r.place_on_field(1, "T", Some(1));
        r.place_on_field(1, "O", Some(2));
        r.game
            .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(h));
        r.game.drain_effect_queue();
        assert!(r.game.players[1].battle_area[0].is_suspended);
        assert!(r.game.players[1].battle_area[1].is_suspended);
        assert!(!r.game.players[1].battle_area[2].is_suspended);
        assert!(!r.game.players[0].battle_area[0].is_suspended);
        let a = r.pending_selection().unwrap().valid_action_ids.clone();
        assert_eq!(a.len(), 2);
        let m = digimon_engine::action::build_action_mask(&r.game, 0);
        assert_eq!(m[PASS as usize], 0.0);
        for x in &a {
            assert_eq!(m[*x as usize], 1.0);
        }
        r.execute_action(0, a[chosen]).unwrap();
        assert_eq!(r.trash_size(1), 1);
        assert_eq!(r.game.players[1].battle_area.len(), 2);
        assert!(r.pending_selection().is_none());
    }
}
#[test]
fn no_opponent_targets_finishes() {
    let mut r = base().start();
    let h = r.place_on_field(0, ID, Some(0));
    r.game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();
    assert!(r.pending_selection().is_none());
}
#[test]
fn self_suspend_is_mandatory_opt_and_exact_source_names() {
    for src in ["EX", "XA", "NEAR", "D"] {
        for owner_turn in [true, false] {
            let mut r = base().start();
            r.skip_mulligan();
            if !owner_turn {
                r.end_turn();
                r.auto_resolve().unwrap();
            }
            let h = r.place_stack(0, &[src, ID]);
            let other = r.place_on_field(0, "D", Some(1));
            r.game.suspend(other);
            assert_eq!(r.security_count(1), 4);
            r.game.suspend(h);
            assert!(!r.game.players[0].battle_area[h.index as usize].is_suspended);
            assert_eq!(
                r.security_count(1),
                if src == "EX" || src == "XA" { 3 } else { 4 }
            );
            assert!(r.pending_selection().is_none());
            r.game.suspend(h);
            assert!(r.game.players[0].battle_area[h.index as usize].is_suspended);
            assert_eq!(
                r.security_count(1),
                if src == "EX" || src == "XA" { 3 } else { 4 }
            );
        }
    }
}
#[test]
fn native_and_exact_examon_routes() {
    for (src, cost) in [
        ("EX", Some(2)),
        ("D", Some(6)),
        ("R", Some(6)),
        ("B", Some(6)),
        ("NEAR", None),
    ] {
        let mut r = base().hand(0, &[ID]).start();
        r.skip_mulligan();
        let h = r.place_on_field(0, src, Some(0));
        let a = encode_digivolve(0, h.index as u16);
        let m = digimon_engine::action::build_action_mask(&r.game, 0);
        assert_eq!(m[a as usize] > 0.0, cost.is_some(), "{src}");
        if let Some(cost) = cost {
            r.game.decode_action(a, 0);
            assert_eq!(r.memory(), 10 - cost);
        }
    }
}

#[test]
fn source_identity_is_local_and_opt_resets_next_turn() {
    let mut r = base().start();
    r.skip_mulligan();
    let h = r.place_on_field(0, ID, Some(0));
    r.place_stack(0, &["EX", "D"]);
    r.game.suspend(h);
    assert!(!r.game.players[0].battle_area[h.index as usize].is_suspended);
    assert_eq!(r.security_count(1), 4);
    r.game.suspend(h);
    assert!(r.game.players[0].battle_area[h.index as usize].is_suspended);
    r.end_turn();
    r.auto_resolve().unwrap();
    r.game.unsuspend(h);
    r.game.suspend(h);
    assert!(!r.game.players[0].battle_area[h.index as usize].is_suspended);
    assert_eq!(r.security_count(1), 4);
}
