//! Official BT4-011 Q1157: temporary treated-as Digimon identity must participate in evolution prohibition.
//! Synthetic declarative gate isolates the reusable interaction; real BT4-011 and ST1-12 are unmodified.
const GATE: &str = r#"
card: TEST-Q1157-GATE
name: Digimon Evolution Gate
kind: digimon
level: 3
color: [white]
cost: 3
dp: 1000
effects:
  - kind: flood_gate
    modifier: CannotDigivolve
    target: { of: opponent, zone: [battle_area], kind: digimon }
"#;
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
        .from_dsl_yaml(GATE)
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
fn q1157_ordinary_digimon_gate_control() {
    let mut r = setup(false);
    r.place_on_field(0, "ST1-03", None);
    r.place_on_field(1, "TEST-Q1157-GATE", None);
    r.game.tick_declarative_effects();
    assert_eq!(
        build_action_mask(&r.game, 0)[encode_digivolve(0, 0) as usize],
        0.0
    );
}
#[test]
fn q1157_treated_tamer_is_subject_to_digimon_evolution_gate_in_mask_and_clone() {
    let mut r = setup(false);
    r.place_on_field(0, "ST1-12", None);
    r.place_on_field(1, "TEST-Q1157-GATE", None);
    r.game.tick_declarative_effects();
    let before = r.game.verification_digest();
    let clone = r.game.clone();
    for g in [&r.game, &clone] {
        assert_eq!(build_action_mask(g,0)[encode_digivolve(0,0) as usize],0.0,
            "Q1157: a Digimon-only evolution prohibition must include the declared treated-as Tamer route");
        assert_eq!(
            g.card_data[g.players[0].battle_area[0].top_card().data_index].card_kind,
            CardKind::Tamer,
            "legality query must not permanently mutate printed Tamer identity"
        );
    }
    assert_eq!(r.game.verification_digest(), before);
}
#[test]
fn q1157_unrestricted_tamer_route_control() {
    let mut r = setup(false);
    r.place_on_field(0, "ST1-12", None);
    r.game.tick_declarative_effects();
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
