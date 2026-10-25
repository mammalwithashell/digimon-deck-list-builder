//! BT17-023: official observation b2425d7be87f04cc4e19aa0ae328bdee2c935e66f3db38830ea87845bcfdd755.
//! Image/Q2763-Q2765: hand-only yellow Tamer treatment, named routes and bonus draw.
use digimon_engine::action::{
    build_action_mask,
    space::{encode_digivolve, PASS},
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, CardSourceRef, CostDelta, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

fn base(id: &str, kind: CardKind, level: u8, color: CardColor) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = kind;
    c.colors = vec![color];
    c.level = if kind == CardKind::Tamer {
        None
    } else {
        Some(level)
    };
    c.dp = if kind == CardKind::Tamer {
        None
    } else {
        Some(10000)
    };
    c
}
fn named_base(id: &str, name: &str, kind: CardKind, level: u8, color: CardColor) -> CardData {
    let mut card = base(id, kind, level, color);
    card.card_name = name.into();
    card
}
fn builder() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    let mut hybrid = base("TEST-HYBRID", CardKind::Digimon, 5, CardColor::Blue);
    hybrid.traits = vec!["Hybrid".into()];
    hybrid.evo_costs = vec![EvoCost {
        card_color: 1,
        level: 4,
        memory_cost: 3,
    }];
    let mut wrong = hybrid.clone();
    wrong.card_id = "TEST-WRONG-ROUTE".into();
    wrong.card_name = "Wrong route".into();
    wrong.evo_costs[0].level = 5;
    let mut expensive = hybrid.clone();
    expensive.card_id = "TEST-EXPENSIVE".into();
    expensive.card_name = "Expensive".into();
    expensive.evo_costs[0].memory_cost = 25;
    let mut plain = hybrid.clone();
    plain.card_id = "TEST-NON-HYBRID".into();
    plain.card_name = "Plain".into();
    plain.traits = vec!["Ultimate".into()];
    DebugRunner::builder()
        .dsl_card("BT17-023")
        .expect("BT17-023 production YAML")
        .add_card(hybrid)
        .add_card(wrong)
        .add_card(expensive)
        .add_card(plain)
        .add_card(base("TEST-CARRIER", CardKind::Digimon, 5, CardColor::Blue))
        .add_card(base("TEST-FILL", CardKind::Tamer, 0, CardColor::White))
        .deck(0, &["TEST-FILL"; 30])
        .deck(1, &["TEST-FILL"; 30])
        .security(1, &["TEST-FILL"; 5])
        .memory(10)
}
fn choose(r: &mut DebugRunner, a: u16) {
    let p = r.pending_selection_view().expect("legal player choice");
    assert!(p.valid_action_ids.contains(&a) || (a == PASS && p.is_optional));
    assert!(build_action_mask(&r.game, p.selecting_player)[a as usize] > 0.5);
    r.execute_action(p.selecting_player, a).unwrap();
}
fn attack(r: &mut DebugRunner) {
    r.game.players[0].battle_area[0].is_suspended = false;
    r.attack_player(
        PermanentHandle {
            player: 0,
            index: 0,
        },
        1,
        false,
    );
}
fn until_hand(r: &mut DebugRunner) {
    for _ in 0..12 {
        let p = r.pending_selection_view().expect("pending attack effects");
        if p.kind == SelectionKind::Hand {
            return;
        }
        let a = p
            .effect_choices
            .as_ref()
            .and_then(|xs| xs.iter().find(|x| !x.is_optional).or_else(|| xs.first()))
            .map(|x| x.action_id)
            .or_else(|| p.valid_action_ids.iter().copied().find(|a| *a != PASS))
            .unwrap();
        choose(r, a);
    }
    panic!("attack did not reach Hybrid hand choice");
}
#[test]
fn bt17_023_all_printed_routes_and_negative_colors() {
    let mut cases = vec![
        (
            base("TEST-BLUE", CardKind::Digimon, 3, CardColor::Blue),
            Some(3),
        ),
        (
            base("TEST-YELLOW", CardKind::Digimon, 3, CardColor::Yellow),
            Some(3),
        ),
        (
            base("TEST-YELLOW-TAMER", CardKind::Tamer, 0, CardColor::Yellow),
            Some(3),
        ),
        (
            base("TEST-BLUE-TAMER", CardKind::Tamer, 0, CardColor::Blue),
            None,
        ),
        (
            named_base(
                "TEST-KOJI-WHITE",
                "Koji Minamoto",
                CardKind::Tamer,
                0,
                CardColor::White,
            ),
            Some(2),
        ),
        (
            named_base(
                "TEST-LOBOMON",
                "Lobomon",
                CardKind::Digimon,
                4,
                CardColor::Green,
            ),
            Some(1),
        ),
        (base("TEST-RED", CardKind::Digimon, 3, CardColor::Red), None),
    ];
    let mut alias = base("TEST-KOJI-ALIAS", CardKind::Tamer, 0, CardColor::White);
    alias.also_treated_as = vec!["Koji Minamoto".into()];
    cases.push((alias, Some(2)));
    for (card, cost) in cases {
        let id = card.card_id.clone();
        let mut r = builder().add_card(card).hand(0, &["BT17-023"]).start();
        let h = r.place_on_field(0, &id, Some(0));
        let a = encode_digivolve(0, h.index as u16);
        assert_eq!(
            build_action_mask(&r.game, 0)[a as usize] > 0.5,
            cost.is_some(),
            "{id}"
        );
        if let Some(cost) = cost {
            r.game.decode_action(a, 0);
            r.auto_resolve().unwrap();
            assert_eq!(r.memory(), 10 - cost, "{id}");
            assert_eq!(
                r.game.players[0].battle_area[0]
                    .top_card()
                    .card_id(&r.game.card_data),
                "BT17-023"
            );
        }
    }
}
#[test]
fn bt17_023_yellow_koji_exposes_native_and_treated_named_cost() {
    let mut koji = base("TEST-KOJI", CardKind::Tamer, 0, CardColor::Yellow);
    koji.card_name = "Koji Minamoto".into();
    let mut r = builder().add_card(koji).hand(0, &["BT17-023"]).start();
    r.place_on_field(0, "TEST-KOJI", Some(0));
    r.game.decode_action(encode_digivolve(0, 0), 0);
    let p = r.pending_selection_view().expect("route identity choices");
    let choices = p.effect_choices.expect("route choices");
    assert!(choices.iter().any(|c| c.label == "Digivolve for cost 2"));
    let treated = choices
        .iter()
        .find(|c| c.label.starts_with("Digivolve for cost 2 (treat base"))
        .expect("Q2764 treated named route");
    choose(&mut r, treated.action_id);
    r.auto_resolve().unwrap();
    assert_eq!(r.memory(), 8);
}
#[test]
fn bt17_023_generic_tamer_permission_is_hand_only() {
    for (kind, color, expected) in [
        (CardKind::Tamer, CardColor::Yellow, false),
        (CardKind::Digimon, CardColor::Blue, true),
    ] {
        let mut r = builder()
            .add_card(base("TEST-BASE", kind, 3, color))
            .hand(0, &["BT17-023"])
            .start();
        let incoming = r.game.players[0].hand.remove(0);
        r.game.players[0].trash.push(incoming);
        let h = r.place_on_field(0, "TEST-BASE", Some(0));
        let success = r.game.effect_initiated_digivolve_from_source(
            0,
            CardSourceRef::Trash(0, 0),
            h,
            CostDelta::Reduce(0),
            false,
            PlaySource::ByEffect,
        );
        assert_eq!(
            success, expected,
            "only the generic treated Tamer route needs hand origin"
        );
        if !expected {
            assert_eq!(r.memory(), 10);
            assert_eq!(r.game.players[0].trash.len(), 1);
        }
    }
}
#[test]
fn bt17_023_top_attack_draws_one() {
    let mut r = builder().start();
    r.place_on_field(0, "BT17-023", Some(0));
    let before = r.hand_size(0);
    attack(&mut r);
    r.auto_resolve().unwrap();
    assert_eq!(r.hand_size(0), before + 1);
}
#[test]
fn bt17_023_inherited_draw_checks_seven_card_boundary_and_is_not_opt() {
    for (hand, expected) in [(7, 8), (8, 8)] {
        let mut r = builder().hand(0, &vec!["TEST-FILL"; hand]).start();
        r.place_stack(0, &["BT17-023", "TEST-CARRIER"]);
        attack(&mut r);
        r.auto_resolve().unwrap();
        assert_eq!(r.hand_size(0), expected);
    }
    let mut r = builder().hand(0, &["TEST-FILL"; 6]).start();
    r.place_stack(0, &["BT17-023", "TEST-CARRIER"]);
    for expected in [7, 8, 8] {
        attack(&mut r);
        r.auto_resolve().unwrap();
        assert_eq!(r.hand_size(0), expected);
    }
}
#[test]
fn bt17_023_attack_evolution_masks_filters_and_pays_reduced_cost() {
    let mut r = builder()
        .hand(
            0,
            &[
                "TEST-HYBRID",
                "TEST-NON-HYBRID",
                "TEST-WRONG-ROUTE",
                "TEST-EXPENSIVE",
            ],
        )
        .start();
    r.place_on_field(0, "BT17-023", Some(0));
    attack(&mut r);
    until_hand(&mut r);
    let p = r.pending_selection_view().unwrap();
    assert!(p.is_optional);
    assert!(build_action_mask(&r.game, 0)[PASS as usize] > 0.5);
    assert!(p.valid_action_ids.contains(&0));
    for invalid in [1, 2, 3] {
        assert!(!p.valid_action_ids.contains(&invalid));
        assert_eq!(build_action_mask(&r.game, 0)[invalid as usize], 0.0);
    }
    choose(&mut r, 0);
    r.auto_resolve().unwrap();
    assert_eq!(r.memory(), 8);
    assert_eq!(
        r.game.players[0].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "TEST-HYBRID"
    );
    assert_eq!(r.hand_size(0), 5);
}
#[test]
fn bt17_023_optional_attack_evolution_can_be_declined() {
    let mut r = builder().hand(0, &["TEST-HYBRID"]).start();
    r.place_on_field(0, "BT17-023", Some(0));
    attack(&mut r);
    until_hand(&mut r);
    choose(&mut r, PASS);
    r.auto_resolve().unwrap();
    assert_eq!(r.memory(), 10);
    assert_eq!(
        r.game.players[0].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "BT17-023"
    );
    assert_eq!(r.hand_size(0), 2);
}

#[test]
fn bt17_023_production_metadata_preserves_the_borrowed_yellow_circle() {
    let mut runtime = CardData::load_from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/cards.json"
    )))
    .unwrap();
    let meta = runtime.remove("BT17-023").unwrap();
    assert!(meta
        .evo_costs
        .iter()
        .any(|c| c.card_color == 2 && c.level == 3 && c.memory_cost == 3));
    let mut r = DebugRunner::builder()
        .add_card(meta)
        .dsl_card("BT17-023")
        .unwrap()
        .add_card(base(
            "TEST-RUNTIME-YELLOW-TAMER",
            CardKind::Tamer,
            0,
            CardColor::Yellow,
        ))
        .add_card(base(
            "TEST-RUNTIME-FILL",
            CardKind::Tamer,
            0,
            CardColor::White,
        ))
        .deck(0, &["TEST-RUNTIME-FILL"; 10])
        .hand(0, &["BT17-023"])
        .memory(10)
        .start();
    r.place_on_field(0, "TEST-RUNTIME-YELLOW-TAMER", Some(0));
    let a = encode_digivolve(0, 0);
    assert!(build_action_mask(&r.game, 0)[a as usize] > 0.5);
    r.game.decode_action(a, 0);
    r.auto_resolve().unwrap();
    assert_eq!(r.memory(), 7);
    assert_eq!(
        r.game.players[0].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "BT17-023"
    );
}

#[test]
fn bt17_023_attack_can_draw_the_hybrid_then_choose_it_for_evolution() {
    let mut r = builder().deck(0, &["TEST-FILL", "TEST-HYBRID"]).start();
    r.place_on_field(0, "BT17-023", Some(0));
    attack(&mut r);
    until_hand(&mut r);
    let p = r.pending_selection_view().unwrap();
    assert!(p.valid_action_ids.contains(&0));
    assert_eq!(
        r.game.players[0].hand[0].card_id(&r.game.card_data),
        "TEST-HYBRID"
    );
    choose(&mut r, 0);
    r.auto_resolve().unwrap();
    assert_eq!(r.memory(), 8);
    assert_eq!(
        r.game.players[0].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "TEST-HYBRID"
    );
}

#[test]
fn bt17_023_q2769_evolution_first_invalidates_the_old_top_draw() {
    let mut r = builder().hand(0, &["TEST-HYBRID"]).start();
    r.place_on_field(0, "BT17-023", Some(0));
    let deck_before = r.game.players[0].deck.len();
    attack(&mut r);
    let order = r
        .pending_selection_view()
        .expect("the player orders simultaneous attack effects");
    let choices = order
        .effect_choices
        .as_ref()
        .expect("actual trigger-order actions");
    assert_eq!(order.kind, SelectionKind::TriggerOrder);
    assert!(
        choices.iter().any(|c| !c.is_optional),
        "mandatory Draw-first action remains available"
    );
    let evolution = choices
        .iter()
        .find(|c| c.is_optional)
        .expect("the optional evolution is an actual ordering choice")
        .action_id;
    choose(&mut r, evolution);
    let hand = r
        .pending_selection_view()
        .expect("chosen evolution effect asks for a card");
    assert_eq!(hand.kind, SelectionKind::Hand);
    choose(&mut r, 0);
    r.auto_resolve()
        .expect("finish evolution and drop the inactive old top effect");
    assert_eq!(
        r.game.players[0].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "TEST-HYBRID"
    );
    assert_eq!(r.memory(), 8);
    assert_eq!(
        r.hand_size(0),
        1,
        "Q2769: only the digivolution bonus draw happens, not the old top Draw 1"
    );
    assert_eq!(r.game.players[0].deck.len(), deck_before - 1);
}
