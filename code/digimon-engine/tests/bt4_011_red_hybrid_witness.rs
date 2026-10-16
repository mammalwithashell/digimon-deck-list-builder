//! Full BT4-011 printed routes plus Q4035 discriminating negative/positive control.
//! Parent: BT4_011_TEST_YAML selects declaration baseline for genuine RED.
use digimon_engine::{
    action::{build_action_mask, encode_digivolve, space::PLAY_HAND_START},
    card_data::EvoCost,
    debug_runner::DebugRunner,
    enums::{CardKind, GamePhase},
};
fn setup(option: bool) -> DebugRunner {
    let path = std::env::var("BT4_011_TEST_YAML")
        .unwrap_or_else(|_| format!("{}/cards/bt4/BT4-011.yaml", env!("CARGO_MANIFEST_DIR")));
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(&std::fs::read_to_string(path).unwrap())
        .unwrap()
        .from_dsl_yaml(include_str!("../cards/st1/ST1-12.yaml"))
        .unwrap()
        .from_dsl_yaml(include_str!("../cards/st2/ST2-12.yaml"))
        .unwrap()
        .from_dsl_yaml(include_str!("../cards/st1/ST1-03.yaml"))
        .unwrap()
        .from_dsl_yaml(include_str!("../cards/LM/LM-027.yaml"))
        .unwrap()
        .hand(
            0,
            if option {
                &["LM-027", "BT4-011"][..]
            } else {
                &["BT4-011"][..]
            },
        )
        .deck(0, &["ST1-03"; 12])
        .deck(1, &["ST1-03"; 12])
        .memory(5)
        .start();
    // Exact printed circle; DebugRunner compiled metadata omits compatibility evo_costs.
    r.game
        .card_data
        .iter_mut()
        .find(|c| c.card_id == "BT4-011")
        .unwrap()
        .evo_costs = vec![EvoCost {
        card_color: 0,
        level: 3,
        memory_cost: 2,
    }];
    r.game.current_phase = GamePhase::Main;
    r
}
#[test]
fn red_tamer_native_route_cost_draw_sources_and_clone() {
    let mut r = setup(false);
    r.place_on_field(0, "ST1-12", None);
    assert_eq!(
        r.game.card_data[r.game.players[0].battle_area[0].top_card().data_index].card_kind,
        CardKind::Tamer
    );
    let a = encode_digivolve(0, 0);
    assert_eq!(build_action_mask(&r.game, 0)[a as usize], 1.0);
    let mut clone = r.game.clone();
    r.game.decode_action(a, 0);
    clone.decode_action(a, 0);
    assert_eq!(r.game.verification_digest(), clone.verification_digest());
    assert_eq!(r.memory(), 3);
    assert_eq!(r.game.players[0].deck.len(), 11);
    let ids: Vec<_> = r.game.players[0].battle_area[0]
        .card_sources
        .iter()
        .map(|c| c.card_id(&r.game.card_data))
        .collect();
    assert_eq!(ids, vec!["ST1-12", "BT4-011"]);
}
#[test]
fn blue_tamer_is_not_an_alternate_base() {
    let mut r = setup(false);
    r.place_on_field(0, "ST2-12", None);
    assert_eq!(
        build_action_mask(&r.game, 0)[encode_digivolve(0, 0) as usize],
        0.0
    );
}
#[test]
fn ordinary_red_rookie_route_retained() {
    let mut r = setup(false);
    r.place_on_field(0, "ST1-03", None);
    let a = encode_digivolve(0, 0);
    assert_eq!(build_action_mask(&r.game, 0)[a as usize], 1.0);
    r.game.decode_action(a, 0);
    assert_eq!(r.memory(), 3);
    assert_eq!(
        r.game.players[0].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "BT4-011"
    );
}
#[test]
fn q4035_red_tamer_excluded_from_scramble_but_native_route_remains_legal() {
    let mut r = setup(true);
    r.place_on_field(0, "ST1-12", None);
    assert_eq!(build_action_mask(&r.game, 0)[PLAY_HAND_START as usize], 1.0);
    r.game.decode_action(PLAY_HAND_START, 0);
    assert!(
        r.game.pending_selection.is_none(),
        "No red Digimon exists: no effect evolution choice may target the red Tamer"
    );
    assert_eq!(r.memory(), 3);
    assert_eq!(
        r.game.players[0].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "ST1-12"
    );
    assert_eq!(
        r.game.players[0].battle_area[1]
            .top_card()
            .card_id(&r.game.card_data),
        "LM-027"
    );
    let a = encode_digivolve(0, 0);
    assert_eq!(build_action_mask(&r.game, 0)[a as usize], 1.0);
    let mut clone = r.game.clone();
    r.game.decode_action(a, 0);
    clone.decode_action(a, 0);
    assert_eq!(r.game.verification_digest(), clone.verification_digest());
    assert_eq!(r.memory(), 1);
    assert_eq!(
        r.game.players[0].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "BT4-011"
    );
}

#[test]
fn printed_hybrid_form_and_wizard_type_reach_runtime_trait_queries() {
    let r = setup(false);
    let data = r
        .game
        .card_data
        .iter()
        .find(|c| c.card_id == "BT4-011")
        .unwrap();
    assert!(
        data.traits.iter().any(|t| t == "Hybrid"),
        "printed Hybrid form must be present in runtime traits for Hybrid selectors"
    );
    assert!(
        data.traits.iter().any(|t| t == "Wizard"),
        "printed Wizard type retained"
    );
    assert!(
        data.traits.iter().any(|t| t == "Variable"),
        "printed Variable attribute retained"
    );
}
