//! BaoHuckmon: official BT13-013 image, bundle, and same-event inherited Q&A.
use digimon_engine::action::{build_action_mask, space::PASS};
use digimon_engine::card_data::EvoCost;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::CardKind;

fn builder() -> DebugRunnerBuilder {
    let mut sister = make_test_card("SISTER", "Sistermon Test");
    sister.play_cost = 0;
    let mut tamer = sister.clone();
    tamer.card_id = "TAMER".into();
    tamer.card_kind = CardKind::Tamer;
    let mut savior = make_test_card("SAVIOR", "SaviorHuckmon");
    savior.level = Some(5);
    savior.evo_costs = vec![EvoCost {
        card_color: 0,
        level: 4,
        memory_cost: 3,
    }];
    let mut wrong = savior.clone();
    wrong.card_id = "WRONG".into();
    wrong.card_name = "SaviorHuckmon X".into();
    let mut illegal = savior.clone();
    illegal.card_id = "ILLEGAL".into();
    illegal.evo_costs[0].level = 6;
    DebugRunner::builder()
        .dsl_card("BT13-013")
        .expect("BT13-013 production YAML")
        .add_card(sister)
        .add_card(tamer)
        .add_card(savior)
        .add_card(wrong)
        .add_card(illegal)
        .add_card(make_test_card("OTHER", "Agumon"))
        .deck(0, &["OTHER"; 5])
        .memory(10)
}

#[test]
fn bt13_013_reduced_evolution_and_same_event_does_not_gain_memory() {
    let mut r = builder()
        .hand(
            0,
            &["SISTER", "SAVIOR", "WRONG", "ILLEGAL", "SISTER", "SISTER"],
        )
        .start();
    let _base = r.place_on_field(0, "BT13-013", None);
    r.play(0, 0).unwrap();
    let p = r.pending_selection().expect("optional evolution");
    assert_eq!(
        p.valid_action_ids.len(),
        1,
        "exact name and legal evolution only"
    );
    let a = p.valid_action_ids[0];
    let mask = build_action_mask(&r.game, 0);
    assert_eq!(mask[a as usize], 1.0);
    assert_eq!(mask[PASS as usize], 1.0);
    r.execute_action(0, a).unwrap();
    assert!(r.pending_selection().is_none());
    assert_eq!(
        r.memory(),
        9,
        "pay 3-2; newly inherited effect must not trigger for this play"
    );
    assert_eq!(
        r.game.players[0].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "SAVIOR"
    );
    r.play(0, 2).unwrap();
    assert_eq!(r.memory(), 10, "next Sistermon play gains memory");
    r.play(0, 2).unwrap();
    assert_eq!(r.memory(), 10, "inherited once per turn");
}

#[test]
fn bt13_013_decline_keeps_card_and_can_trigger_again() {
    let mut r = builder().hand(0, &["SISTER", "SAVIOR", "SISTER"]).start();
    r.place_on_field(0, "BT13-013", None);
    r.play(0, 0).unwrap();
    assert!(r.pending_is_optional());
    r.execute_action(0, PASS).unwrap();
    assert_eq!(r.memory(), 10);
    assert_eq!(r.hand_size(0), 2);
    r.play(0, 1).unwrap();
    assert!(r.pending_selection().is_some(), "main effect is not OPT");
}

#[test]
fn bt13_013_filters_event_kind_owner_turn_and_missing_targets() {
    for (played, player, turn, target) in [
        ("TAMER", 0, 0, "SAVIOR"),
        ("OTHER", 0, 0, "SAVIOR"),
        ("SISTER", 1, 0, "SAVIOR"),
        ("SISTER", 0, 1, "SAVIOR"),
        ("SISTER", 0, 0, "WRONG"),
        ("SISTER", 0, 0, "ILLEGAL"),
    ] {
        let mut r = builder()
            .hand(0, &[target])
            .hand(player, &[played, target])
            .start();
        r.place_on_field(0, "BT13-013", None);
        r.game.turn_player_idx = turn;
        r.play(player, 0).unwrap();
        assert!(
            r.pending_selection().is_none(),
            "{played} p{player} turn{turn} target{target}"
        );
    }
}

#[test]
fn bt13_013_inherited_requires_own_sistermon_on_own_turn() {
    for (played, player, turn) in [
        ("TAMER", 0, 0),
        ("OTHER", 0, 0),
        ("SISTER", 1, 0),
        ("SISTER", 0, 1),
    ] {
        let mut r = builder().hand(player, &[played]).start();
        r.place_stack(0, &["BT13-013", "SAVIOR"]);
        r.game.turn_player_idx = turn;
        let cost = r
            .game
            .card_data
            .iter()
            .find(|c| c.card_id == played)
            .unwrap()
            .play_cost;
        r.play(player, 0).unwrap();
        assert_eq!(
            r.memory(),
            10 - if player == 0 {
                cost as i16
            } else {
                -(cost as i16)
            }
        );
    }
}

#[test]
fn bt13_013_inherited_is_mandatory_and_once_per_turn_on_existing_stack() {
    let mut r = builder().hand(0, &["SISTER", "SISTER"]).memory(5).start();
    r.place_stack(0, &["BT13-013", "SAVIOR"]);
    r.play(0, 0).unwrap();
    assert_eq!(r.memory(), 6);
    assert!(
        r.pending_selection().is_none(),
        "mandatory memory gain has no decline"
    );
    r.play(0, 0).unwrap();
    assert_eq!(r.memory(), 6);
}

#[test]
fn bt13_013_all_eligible_hand_copies_are_selectable() {
    let mut r = builder().hand(0, &["SISTER", "SAVIOR", "SAVIOR"]).start();
    r.place_on_field(0, "BT13-013", None);
    r.play(0, 0).unwrap();
    let p = r.pending_selection().unwrap();
    assert_eq!(p.valid_action_ids.len(), 2);
    let mask = build_action_mask(&r.game, 0);
    for a in &p.valid_action_ids {
        assert_eq!(mask[*a as usize], 1.0);
    }
    assert_eq!(mask[PASS as usize], 1.0);
}

#[test]
fn bt13_013_printed_red_level3_route_masks_and_pays_two() {
    use digimon_engine::action::space::encode_digivolve;
    use digimon_engine::enums::CardColor;
    let mut red = make_test_card("RED3", "Red level 3");
    red.level = Some(3);
    red.colors = vec![CardColor::Red];
    let mut blue = red.clone();
    blue.card_id = "BLUE3".into();
    blue.colors = vec![CardColor::Blue];
    let mut red4 = red.clone();
    red4.card_id = "RED4".into();
    red4.level = Some(4);
    let mut r = builder()
        .add_card(red)
        .add_card(blue)
        .add_card(red4)
        .hand(0, &["BT13-013"])
        .memory(5)
        .start();
    r.place_on_field(0, "RED3", None);
    r.place_on_field(0, "BLUE3", None);
    r.place_on_field(0, "RED4", None);
    let mask = build_action_mask(&r.game, 0);
    assert_eq!(
        mask[encode_digivolve(0, 0) as usize],
        1.0,
        "printed Red Lv3 route"
    );
    assert_eq!(mask[encode_digivolve(0, 1) as usize], 0.0, "wrong color");
    assert_eq!(mask[encode_digivolve(0, 2) as usize], 0.0, "wrong level");
    r.game.decode_action(encode_digivolve(0, 0), 0);
    assert_eq!(r.memory(), 3);
    assert_eq!(
        r.game.players[0].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "BT13-013"
    );
    assert_eq!(r.hand_size(0), 1, "digivolution bonus draw");
}

#[test]
fn bt13_013_play_event_survives_sistermon_leaving_before_evolution_resolution() {
    use digimon_engine::selection::SelectionKind;
    let mut r = builder()
        .hand(0, &["SISTER", "SAVIOR", "SAVIOR"])
        .memory(5)
        .start();
    r.place_on_field(0, "BT13-013", None);
    r.place_on_field(0, "BT13-013", None);
    r.play(0, 0).unwrap();
    assert_eq!(r.pending_kind(), Some(SelectionKind::TriggerOrder));
    // Isolate the captured play event from the played permanent's lifetime.
    let sister = r.game.players[0].battle_area.remove(2);
    assert_eq!(sister.top_card().card_id(&r.game.card_data), "SISTER");
    r.game.players[0].trash.extend(sister.card_sources);
    for expected_candidates in [2, 1] {
        if r.pending_kind() == Some(SelectionKind::TriggerOrder) {
            let a = r.pending_selection().unwrap().valid_action_ids[0];
            r.execute_action(0, a).unwrap();
        }
        let p = r
            .pending_selection()
            .expect("captured Sistermon play still offers evolution");
        assert_ne!(p.kind, SelectionKind::TriggerOrder);
        assert_eq!(p.valid_action_ids.len(), expected_candidates);
        let a = p.valid_action_ids[0];
        assert_eq!(build_action_mask(&r.game, 0)[a as usize], 1.0);
        r.execute_action(0, a).unwrap();
    }
    assert_eq!(
        r.memory(),
        3,
        "both evolutions cost 3 minus 2, no newly inherited gain"
    );
    assert!(r.pending_selection().is_none());
}

#[test]
fn bt13_013_captured_inherited_play_gains_memory_after_sistermon_leaves() {
    use digimon_engine::selection::SelectionKind;
    let mut r = builder().hand(0, &["SISTER", "SISTER"]).memory(5).start();
    r.place_stack(0, &["BT13-013", "SAVIOR"]);
    r.place_stack(0, &["BT13-013", "SAVIOR"]);
    r.play(0, 0).unwrap();
    assert_eq!(r.pending_kind(), Some(SelectionKind::TriggerOrder));
    let sister = r.game.players[0].battle_area.remove(2);
    assert_eq!(sister.top_card().card_id(&r.game.card_data), "SISTER");
    r.game.players[0].trash.extend(sister.card_sources);
    while let Some(p) = r.pending_selection() {
        assert_eq!(p.kind, SelectionKind::TriggerOrder);
        let a = p.valid_action_ids[0];
        assert_eq!(build_action_mask(&r.game, 0)[a as usize], 1.0);
        r.execute_action(0, a).unwrap();
    }
    assert_eq!(
        r.memory(),
        7,
        "both captured inherited triggers gain memory"
    );
    r.play(0, 0).unwrap();
    assert_eq!(r.memory(), 7, "both inherited OPT uses were consumed");
    assert!(r.pending_selection().is_none());
}
