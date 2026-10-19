//! EX3-037: official observation 5c58900c, image, Q3413/Q3414 and DCGO verified.
use digimon_engine::action::space::{encode_digivolve, PASS, SEL_REVEAL_START};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{
    make_test_card, make_test_egg, DebugRunner, DebugRunnerBuilder,
};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::selection::{SelectionKind, TriggerSource};
const ID: &str = "EX3-037";
fn base() -> DebugRunnerBuilder {
    let mut b = DebugRunner::builder()
        .from_dsl_yaml(include_str!("../../../cards/ex3/EX3-037.yaml"))
        .expect("EX3-037 production YAML required");
    for (id, name, color, kind) in [
        ("D", "Airdramon", CardColor::Green, CardKind::Digimon),
        ("O", "Dramon Option", CardColor::Blue, CardKind::Option),
        ("E", "Examon", CardColor::Red, CardKind::Digimon),
        ("B", "Dramon Examon", CardColor::Green, CardKind::Digimon),
        ("R", "Airdramon Red", CardColor::Red, CardKind::Digimon),
        ("T", "Dramon Tamer", CardColor::Green, CardKind::Tamer),
        ("P", "Plain", CardColor::Blue, CardKind::Digimon),
    ] {
        let mut c = make_test_card(id, name);
        c.colors = vec![color];
        c.card_kind = kind;
        b = b.add_card(c);
    }
    for (id, name, color) in [
        ("G2", "Green Egg", CardColor::Green),
        ("B2", "Blue Egg", CardColor::Blue),
        ("R2", "Red Egg", CardColor::Red),
        ("BE", "Bebydomon", CardColor::Red),
        ("FAKE", "Bebydomon X", CardColor::Red),
    ] {
        let mut c = make_test_egg(id, name);
        c.colors = vec![color];
        b = b.add_card(c);
    }
    b.deck(0, &["P"; 12]).deck(1, &["P"; 12]).memory(5)
}
fn reveal(ids: &[&str]) -> DebugRunner {
    let mut r = base().start();
    r.skip_mulligan();
    r.game.players[0].deck.clear();
    for id in ids.iter().rev() {
        let idx = r
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == *id)
            .unwrap();
        let serial = r.game.next_card_index();
        r.game.players[0].deck.push(CardSource::new(idx, 0, serial));
    }
    let h = r.place_on_field(0, ID, Some(0));
    r.game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();
    r
}
fn pick(r: &mut DebugRunner, id: &str) {
    let p = r
        .game
        .revealed_cards
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == id)
        .unwrap();
    let a = SEL_REVEAL_START + p as u16;
    assert_eq!(
        digimon_engine::action::build_action_mask(&r.game, 0)[a as usize],
        1.0
    );
    r.execute_action(0, a).unwrap();
}
fn finish(r: &mut DebugRunner) {
    for _ in 0..8 {
        let Some(v) = r.pending_selection_view() else {
            return;
        };
        assert!(matches!(v.kind, SelectionKind::OrderedPermutation { .. }));
        r.execute_action(v.selecting_player, *v.valid_action_ids.last().unwrap())
            .unwrap();
    }
    panic!("remainder failed to finish")
}
#[test]
fn reveal_two_mandatory_buckets_accept_any_card_kind_and_filter_color() {
    let mut r = reveal(&["O", "E", "R", "P"]);
    let m = digimon_engine::action::build_action_mask(&r.game, 0);
    assert_eq!(m[PASS as usize], 0.0);
    assert_eq!(m[SEL_REVEAL_START as usize + 2], 0.0);
    pick(&mut r, "O");
    assert_eq!(
        digimon_engine::action::build_action_mask(&r.game, 0)[PASS as usize],
        0.0
    );
    pick(&mut r, "E");
    finish(&mut r);
    assert_eq!(r.hand_size(0), 2);
    assert_eq!(r.game.players[0].deck.len(), 2);
    let ids: Vec<_> = r.game.players[0]
        .deck
        .iter()
        .map(|c| c.card_id(&r.game.card_data))
        .collect();
    assert_eq!(ids, vec!["R", "P"]);
}
#[test]
fn reveal_each_bucket_alone_and_no_matches() {
    for (ids, picked) in [
        (["D", "P", "P", "P"], Some("D")),
        (["E", "P", "P", "P"], Some("E")),
        (["R", "P", "P", "P"], None),
    ] {
        let mut r = reveal(&ids);
        if let Some(id) = picked {
            pick(&mut r, id)
        }
        finish(&mut r);
        assert_eq!(r.hand_size(0), usize::from(picked.is_some()));
    }
}
#[test]
fn overlapping_buckets_cannot_consume_one_card_twice() {
    let mut r = reveal(&["B", "P", "P", "P"]);
    pick(&mut r, "B");
    finish(&mut r);
    assert_eq!(r.hand_size(0), 1);
    let mut r = reveal(&["D", "B", "P", "P"]);
    pick(&mut r, "D");
    pick(&mut r, "B");
    finish(&mut r);
    assert_eq!(r.hand_size(0), 2);
}
#[test]
fn inherited_observes_correct_owner_kind_name_and_buffs_carrier_once() {
    let mut r = base().start();
    r.skip_mulligan();
    let h = r.place_stack(0, &[ID, "P"]);
    let top = r.place_on_field(0, ID, Some(1));
    for (owner, id) in [(1, "D"), (0, "T"), (0, "P")] {
        let x = r.place_on_field(owner, id, None);
        r.game.suspend(x);
        assert_eq!(r.effective_dp(h), Some(2000));
    }
    let d = r.place_on_field(0, "D", None);
    r.game.suspend(d);
    assert_eq!(r.effective_dp(h), Some(3000));
    assert_eq!(r.effective_dp(d), Some(2000));
    assert_eq!(r.effective_dp(top), Some(2000));
    let e = r.place_on_field(0, "E", None);
    r.game.suspend(e);
    assert_eq!(r.effective_dp(h), Some(3000));
    r.end_turn();
    r.auto_resolve().unwrap();
    assert_eq!(r.effective_dp(h), Some(2000));
    r.game.players[0].battle_area[e.index as usize].is_suspended = false;
    r.game.suspend(e);
    assert_eq!(r.effective_dp(h), Some(3000));
}
#[test]
fn carrier_can_be_the_named_suspended_digimon() {
    let mut r = base().start();
    r.skip_mulligan();
    let h = r.place_stack(0, &[ID, "D"]);
    r.game.suspend(h);
    assert_eq!(r.effective_dp(h), Some(3000));
}
#[test]
fn evolution_masks_and_costs_match_two_circles_and_exact_bebydomon() {
    for (egg, cost) in [
        ("G2", Some(1)),
        ("B2", Some(1)),
        ("BE", Some(0)),
        ("R2", None),
        ("FAKE", None),
    ] {
        let mut r = base().hand(0, &[ID]).start();
        r.skip_mulligan();
        let h = r.place_on_field(0, egg, Some(0));
        let a = encode_digivolve(0, h.index as u16);
        assert_eq!(
            digimon_engine::action::build_action_mask(&r.game, 0)[a as usize] > 0.0,
            cost.is_some(),
            "{egg}"
        );
        if let Some(c) = cost {
            r.game.decode_action(a, 0);
            assert_eq!(r.memory(), 5 - c);
        }
    }
}

#[test]
fn reveal_remainder_keeps_symbolic_identity_after_each_pick_and_clone() {
    use digimon_engine::dcgo_recording::SelectionRow;
    use digimon_engine::runners::selection_resolve::resolve_next;
    let mut r = reveal(&["O", "E", "R", "P"]);
    pick(&mut r, "O");
    pick(&mut r, "E");
    let row = |id: &str| -> SelectionRow {
        serde_json::from_value(serde_json::json!({
            "step": 0, "actor": 0, "prompt": "SelectCardEffect", "phase": "Main",
            "card_ids": [id]
        })).unwrap()
    };
    let first = resolve_next(&r.game, &row("R"), 0).expect("first remainder identity").unwrap();
    r.execute_action(0, first).unwrap();
    let mut cloned = r.game.clone();
    for game in [&mut r.game, &mut cloned] {
        let last = resolve_next(game, &row("P"), 0).expect("remaining identity survives repark").unwrap();
        game.decode_action(last, 0);
        assert!(game.pending_selection.is_none());
        let deck: Vec<_> = game.players[0].deck.iter().map(|c| c.card_id(&game.card_data)).collect();
        assert_eq!(deck, vec!["P", "R"]);
    }
}
