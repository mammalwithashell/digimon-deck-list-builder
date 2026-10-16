//! Full BT7-059 printed OnPlay and inherited behavior; AXE_TEST_YAML selects
//! the root-controlled metadata-only baseline for real behavioral RED.
use digimon_engine::{
    action::{
        build_action_mask,
        space::{PASS, PLAY_HAND_START, SEL_REVEAL_START},
    },
    debug_runner::{make_test_card, DebugRunner},
    permanent::PermanentHandle,
};
use std::collections::BTreeSet;
fn yaml() -> String {
    std::fs::read_to_string(
        std::env::var("AXE_TEST_YAML")
            .unwrap_or_else(|_| format!("{}/cards/bt7/BT7-059.yaml", env!("CARGO_MANIFEST_DIR"))),
    )
    .unwrap()
}
fn setup(deck: &[&str]) -> DebugRunner {
    let mut alias = make_test_card("ALIAS", "Unrelated printed name");
    alias.also_treated_as.push("Knightmon".into());
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(&yaml())
        .unwrap()
        .add_card(alias)
        .add_card(make_test_card("K1", "SkullKnightmon"))
        .add_card(make_test_card("K2", "DarkKnightmon"))
        .add_card(make_test_card("K3", "LordKnightmon"))
        .add_card(make_test_card("BAG", "Bagramon"))
        .add_card(make_test_card("F1", "Filler one"))
        .add_card(make_test_card("F2", "Filler two"))
        .add_card(make_test_card("TAIL", "Unrevealed tail"))
        .add_card(make_test_card("F3", "Filler three"))
        .add_card(make_test_card("F4", "Filler four"))
        .add_card(make_test_card("F5", "Filler five"))
        .hand(0, &["BT7-059"])
        .deck(0, deck)
        .deck(1, &["TAIL"; 12])
        .memory(10)
        .start();
    r.skip_mulligan();
    r.game.enter_main_phase();
    r
}
fn play(r: &mut DebugRunner) {
    assert_eq!(build_action_mask(&r.game, 0)[PLAY_HAND_START as usize], 1.0);
    r.game.decode_action(PLAY_HAND_START, 0);
}
fn act(r: &mut DebugRunner, a: u16) {
    assert_eq!(
        build_action_mask(&r.game, 0)[a as usize],
        1.0,
        "actual offered action {a}"
    );
    r.execute_action(0, a).unwrap();
}
fn pick(r: &mut DebugRunner, id: &str) {
    let i = r
        .game
        .revealed_cards
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == id)
        .expect("named reveal");
    act(r, SEL_REVEAL_START + i as u16);
}
fn finish(r: &mut DebugRunner) {
    for _ in 0..10 {
        let Some(p) = r.game.pending_selection.as_ref() else {
            return;
        };
        let a = *p.valid_action_ids.iter().find(|a| **a != PASS).unwrap();
        act(r, a);
    }
    assert!(r.game.pending_selection.is_none());
}
fn hand(r: &DebugRunner) -> Vec<String> {
    let mut v: Vec<_> = r.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect();
    v.sort();
    v
}
#[test]
fn axe_mandatory_first_choice_can_stop_at_one_or_take_two_in_clones() {
    let mut r = setup(&["TAIL", "F2", "F1", "K3", "K2", "K1"]);
    play(&mut r);
    assert_eq!(r.game.revealed_cards.len(), 5);
    assert_eq!(
        build_action_mask(&r.game, 0)[PASS as usize],
        0.0,
        "rules 1-3-6 requires at least one available match"
    );
    let saved = r.game.clone();
    for two in [false, true] {
        r.game = saved.clone();
        pick(&mut r, "K1");
        assert_eq!(
            build_action_mask(&r.game, 0)[PASS as usize],
            1.0,
            "up to two permits stopping at one"
        );
        if two {
            pick(&mut r, "K2");
        } else {
            act(&mut r, PASS);
        }
        finish(&mut r);
        assert_eq!(
            hand(&r),
            if two {
                vec!["K1".to_string(), "K2".to_string()]
            } else {
                vec!["K1".to_string()]
            }
        );
        assert!(r.game.revealed_cards.is_empty());
        assert_eq!(
            r.game.players[0]
                .deck
                .last()
                .unwrap()
                .card_id(&r.game.card_data),
            "TAIL",
            "unrevealed remainder stays above returned cards"
        );
    }
}
#[test]
fn axe_every_matching_card_is_legal_but_bagramon_alone_is_not_knightmon() {
    let mut r = setup(&["TAIL", "F1", "BAG", "ALIAS", "K2", "K1"]);
    play(&mut r);
    let mask = build_action_mask(&r.game, 0);
    for c in ["K1", "K2", "ALIAS"] {
        let i = r
            .game
            .revealed_cards
            .iter()
            .position(|x| x.card_id(&r.game.card_data) == c)
            .unwrap();
        assert_eq!(mask[SEL_REVEAL_START as usize + i], 1.0);
    }
    for c in ["F1", "BAG"] {
        let i = r
            .game
            .revealed_cards
            .iter()
            .position(|x| x.card_id(&r.game.card_data) == c)
            .unwrap();
        assert_eq!(mask[SEL_REVEAL_START as usize + i], 0.0);
    }
    pick(&mut r, "ALIAS");
    act(&mut r, PASS);
    finish(&mut r);
    assert_eq!(hand(&r), vec!["ALIAS"]);
}
#[test]
fn axe_all_six_three_card_remainder_orders_are_selectable_and_clone_safe() {
    let mut r = setup(&["TAIL", "F2", "F1", "K3", "K2", "K1"]);
    play(&mut r);
    pick(&mut r, "K1");
    pick(&mut r, "K2");
    let saved = r.game.clone();
    let mut orders = BTreeSet::new();
    for first in 0..3 {
        for second in 0..2 {
            r.game = saved.clone();
            let a = r
                .game
                .pending_selection
                .as_ref()
                .expect("three-card permutation")
                .valid_action_ids[first];
            act(&mut r, a);
            if let Some(p) = r.game.pending_selection.as_ref() {
                let a = p.valid_action_ids[second];
                act(&mut r, a);
            }
            finish(&mut r);
            let ids: Vec<_> = r.game.players[0]
                .deck
                .iter()
                .map(|c| c.card_id(&r.game.card_data).to_string())
                .collect();
            assert_eq!(ids.len(), 4);
            assert_eq!(ids.last().unwrap(), "TAIL");
            orders.insert(ids);
        }
    }
    assert_eq!(orders.len(), 6);
}
#[test]
fn axe_empty_short_and_no_match_decks_finish_without_invented_cards() {
    for deck in [vec![], vec!["F1"], vec!["F1", "F2"], vec!["F1", "K1"]] {
        let matches = deck.contains(&"K1");
        let count = deck.len();
        let mut r = setup(&deck);
        play(&mut r);
        if matches {
            pick(&mut r, "K1");
        }
        finish(&mut r);
        assert_eq!(
            hand(&r),
            if matches {
                vec!["K1".to_string()]
            } else {
                vec![]
            }
        );
        assert_eq!(r.game.players[0].deck.len(), count - usize::from(matches));
        assert!(r.game.revealed_cards.is_empty());
    }
}
#[test]
fn axe_inherited_dp_is_own_turn_only_and_tracks_effective_names() {
    for (id, bonus) in [
        ("K1", 2000),
        ("K2", 2000),
        ("BAG", 2000),
        ("ALIAS", 2000),
        ("F1", 0),
    ] {
        let mut r = setup(&["TAIL"; 12]);
        r.game.players[0].hand.clear();
        r.place_stack(0, &["BT7-059", id]);
        let h = PermanentHandle {
            player: 0,
            index: 0,
        };
        let base = r.game.players[0].battle_area[0]
            .top_card()
            .dp(&r.game.card_data)
            .unwrap();
        r.game.tick_declarative_effects();
        assert_eq!(r.effective_dp(h), Some(base + bonus));
        r.game.turn_player_idx = 1;
        r.game.tick_declarative_effects();
        assert_eq!(r.effective_dp(h), Some(base));
    }
}

#[test]
fn axe_inherited_aura_only_changes_its_carrier_not_own_or_opponent_neighbors() {
    let mut r = setup(&["TAIL"; 12]);
    r.game.players[0].hand.clear();
    r.place_stack(0, &["BT7-059", "K1"]);
    r.place_on_field(0, "K2", None);
    r.place_on_field(0, "F1", None);
    r.place_on_field(1, "K2", None);
    let handles = [
        PermanentHandle {
            player: 0,
            index: 0,
        },
        PermanentHandle {
            player: 0,
            index: 1,
        },
        PermanentHandle {
            player: 0,
            index: 2,
        },
        PermanentHandle {
            player: 1,
            index: 0,
        },
    ];
    let bases: Vec<_> = handles
        .iter()
        .map(|h| {
            r.game.players[h.player as usize].battle_area[h.index as usize]
                .top_card()
                .dp(&r.game.card_data)
                .unwrap()
        })
        .collect();
    for turn in [0, 1, 0] {
        r.game.turn_player_idx = turn;
        r.game.tick_declarative_effects();
        for (i, h) in handles.iter().enumerate() {
            assert_eq!(
                r.effective_dp(*h),
                Some(bases[i] + if turn == 0 && i == 0 { 2000 } else { 0 }),
                "turn {turn} carrier/neighbor {i}"
            );
        }
    }
}
fn collect_remainder_orders(r: &mut DebugRunner, orders: &mut BTreeSet<Vec<String>>) {
    let Some(p) = r.game.pending_selection.as_ref() else {
        assert!(r.game.revealed_cards.is_empty());
        orders.insert(
            r.game.players[0]
                .deck
                .iter()
                .map(|c| c.card_id(&r.game.card_data).to_string())
                .collect(),
        );
        return;
    };
    let actions = p.valid_action_ids.clone();
    assert!(!actions.contains(&PASS));
    let saved = r.game.clone();
    for a in actions {
        r.game = saved.clone();
        act(r, a);
        collect_remainder_orders(r, orders);
    }
}
#[test]
fn axe_four_and_five_remainder_orders_are_all_reachable_with_no_unrevealed_tail() {
    for (deck, expected, has_match) in [
        (vec!["F4", "F3", "F2", "F1", "K1"], 24, true),
        (vec!["F5", "F4", "F3", "F2", "F1"], 120, false),
    ] {
        let mut r = setup(&deck);
        play(&mut r);
        if has_match {
            assert_eq!(
                build_action_mask(&r.game, 0)[PASS as usize],
                0.0,
                "one eligible match must be selected"
            );
            pick(&mut r, "K1");
        }
        let mut orders = BTreeSet::new();
        collect_remainder_orders(&mut r, &mut orders);
        assert_eq!(orders.len(), expected);
        assert!(orders
            .iter()
            .all(|order| order.len() == if has_match { 4 } else { 5 }));
        assert_eq!(
            hand(&r),
            if has_match {
                vec!["K1".to_string()]
            } else {
                vec![]
            }
        );
    }
}
