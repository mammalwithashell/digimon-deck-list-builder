use digimon_engine::{
    action::{build_action_mask, encode_digivolve, space::PASS},
    card_data::EvoCost,
    debug_runner::DebugRunner,
    enums::GamePhase,
};
const CARD: &str = r#"
card: TEST-TREATED-ROUTES
name: Treated Route Fixture
kind: digimon
level: 4
color: [red]
cost: 5
dp: 5000
alt_paths:
  - kind: digivolve
    from: {kind: tamer, color_is: red}
    cost: 2
  - kind: digivolve
    from: {kind: tamer, color_is: red}
    source_treated_as: level_3_red_digimon
    cost: 2
"#;
const OBS: &str = r#"
card: TEST-ROUTE-OBSERVER
name: Observer
kind: tamer
color: [white]
cost: 2
effects:
  - kind: replacement
    trigger: when_would_digivolve
    active_when: {kind: digimon}
    optional: true
    process:
      - draw: {of: you, count: 1}
"#;
#[test]
fn same_cost_routes_keep_distinct_identity_and_visible_labels() {
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(CARD)
        .unwrap()
        .from_dsl_yaml(OBS)
        .unwrap()
        .from_dsl_yaml(include_str!("../cards/st1/ST1-12.yaml"))
        .unwrap()
        .from_dsl_yaml(include_str!("../cards/st1/ST1-03.yaml"))
        .unwrap()
        .hand(0, &["TEST-TREATED-ROUTES"])
        .deck(0, &["ST1-03"; 12])
        .deck(1, &["ST1-03"; 12])
        .memory(5)
        .start();
    r.game
        .card_data
        .iter_mut()
        .find(|c| c.card_id == "TEST-TREATED-ROUTES")
        .unwrap()
        .evo_costs = vec![EvoCost {
        card_color: 0,
        level: 3,
        memory_cost: 2,
    }];
    r.place_on_field(0, "ST1-12", None);
    r.place_on_field(0, "TEST-ROUTE-OBSERVER", None);
    r.game.current_phase = GamePhase::Main;
    r.game.tick_declarative_effects();
    let a = encode_digivolve(0, 0);
    assert_eq!(build_action_mask(&r.game, 0)[a as usize], 1.0);
    r.game.decode_action(a, 0);
    let rows = r
        .game
        .pending_selection
        .as_ref()
        .expect("same price does not erase distinct source identity routes")
        .effect_choices
        .as_ref()
        .unwrap()
        .clone();
    assert_eq!(rows.len(), 2);
    assert_ne!(rows[0].label, rows[1].label);
    assert_eq!(build_action_mask(&r.game, 0)[PASS as usize], 0.0);
    for row in rows {
        let mut g = r.game.clone();
        let mut twin = g.clone();
        assert_eq!(build_action_mask(&g, 0)[row.action_id as usize], 1.0);
        g.resolve_selection(0, row.action_id).unwrap();
        twin.resolve_selection(0, row.action_id).unwrap();
        assert_eq!(g.verification_digest(), twin.verification_digest());
        if row.label.contains("treat base") {
            assert!(
                g.pending_selection.is_some(),
                "Digimon observer belongs to treated route only"
            );
        } else {
            assert!(g.pending_selection.is_none());
            assert_eq!(g.memory, 3);
        }
    }
}
