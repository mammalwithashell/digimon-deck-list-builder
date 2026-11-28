//! Behavioral tests for v2 bootstrap teachers:
//! 1. Empty-board Lv.5/Lv.6 memory-dump guard (`greedy_v2`, `development_v2`):
//!    refuses to hard-play an unreduced Lv.5/Lv.6 onto an empty opponent board
//!    when doing so hands the opponent `>= 6` memory, preferring cheap choke
//!    plays or `PASS` (`62`) instead.
//! 2. Hybrid pilot Tamer-playing teachers (`greedy_hybrid_v2`, `development_hybrid_v2`):
//!    actively plays Tamers from hand (and Hybrid support Options) instead of
//!    passing at 0 memory, and prioritizes Tamer digivolutions when on board.

use std::path::Path;

use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::{PASS, PLAY_HAND_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::game::Game;
use digimon_engine::policies::development::DevelopmentPolicy;
use digimon_engine::policies::tamer_first::{
    greedy_hybrid_v2_action, greedy_v2_action, is_empty_board_high_level_dump,
};
use digimon_engine::rules::Rules;
use serde_json::Value;

fn replay_exam_position(rel_path: &str) -> (Game, u8) {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let exam_path = repo_root.join(rel_path);
    let cards_path = repo_root.join("data/cards.json");
    let exam: Value = serde_json::from_slice(&std::fs::read(&exam_path).unwrap()).unwrap();
    let data = CardData::load_from_str(&std::fs::read_to_string(&cards_path).unwrap()).unwrap();
    let setup = &exam["setup"];
    let decks: Vec<Vec<String>> = serde_json::from_value(setup["decks"].clone()).unwrap();
    let mut game = Game::new(
        &decks,
        &data,
        Rules::standard(),
        Some(setup["game_seed"].as_u64().unwrap()),
    )
    .unwrap();
    game.turn_count = setup["turn_count_reset"].as_u64().unwrap_or(1) as _;
    let actions: Vec<u16> = serde_json::from_value(setup["actions"].clone()).unwrap();
    for a in actions {
        let pid = digimon_engine::policies::current_decision_player(&game);
        game.decode_action(a, pid);
    }
    let viewer = exam["search"]["viewer"].as_u64().unwrap() as u8;
    (game, viewer)
}

fn run_on_large_stack<F: FnOnce() + Send + 'static>(f: F) {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(f)
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn v2_teachers_pass_instead_of_lv5_memory_dump_on_empty_board_exam() {
    run_on_large_stack(|| {
        let (game, viewer) =
            replay_exam_position("qa/search-exams/ex13-empty-board-pass-over-lv5-dump.json");
        let mask = build_action_mask(&game, viewer);

        // All four Lv.5 Wingdramon/Groundramon plays must be flagged as empty-board dumps.
        for a in [0u16, 1, 3, 4] {
            assert!(
                is_empty_board_high_level_dump(&game, viewer, a),
                "action {a} should be flagged as an empty-board Lv.5 memory dump"
            );
        }

        assert_eq!(
            greedy_v2_action(&game, &mask),
            PASS,
            "greedy_v2 must PASS (choke to 3) instead of hard-playing Lv.5 for 7 memory into an empty board"
        );

        let mut dev = DevelopmentPolicy::default();
        let dec = dev.decide_v2(&game, &mask).unwrap();
        assert_eq!(
            dec.action, PASS,
            "development_v2 must PASS (choke to 3) instead of hard-playing Lv.5 for 7 memory into an empty board"
        );
    });
}

#[test]
fn v2_teachers_avoid_lv5_dump_when_cheap_choke_exists_on_empty_board_exam() {
    run_on_large_stack(|| {
        let (game, viewer) =
            replay_exam_position("qa/search-exams/ex13-empty-board-choke-over-lv5-dump.json");
        let mask = build_action_mask(&game, viewer);
        let dump_action = 4u16; // BT26-029 Aegiochusmon: Holy (Lv.5, cost 8 -> gives 8 memory)
        assert!(is_empty_board_high_level_dump(&game, viewer, dump_action));

        let g_act = greedy_v2_action(&game, &mask);
        assert_ne!(
            g_act, dump_action,
            "greedy_v2 must not hard-play Lv.5 Aegiochusmon for 8 mem"
        );

        let mut dev = DevelopmentPolicy::default();
        let d_act = dev.decide_v2(&game, &mask).unwrap().action;
        assert_ne!(
            d_act, dump_action,
            "development_v2 must not hard-play Lv.5 Aegiochusmon for 8 mem"
        );
    });
}

#[test]
fn empty_board_dump_guard_does_not_block_non_empty_board_or_affordable_memory() {
    let mut lv5 = make_test_card("T-LV5", "TestLv5");
    lv5.level = Some(5);
    lv5.play_cost = 7;
    lv5.dp = Some(7000);

    let mut opp_rookie = make_test_card("T-OPP3", "OppRookie");
    opp_rookie.level = Some(3);
    opp_rookie.play_cost = 3;
    opp_rookie.dp = Some(2000);

    // Case A: empty board at memory 0 -> blocked (gives 7 >= 6 memory).
    let mut r = DebugRunner::builder()
        .add_card(lv5.clone())
        .add_card(opp_rookie.clone())
        .hand(0, &["T-LV5"])
        .start();
    r.skip_mulligan();
    r.set_first_player(0);
    r.game.enter_main_phase();
    r.game.memory = 0;
    assert!(is_empty_board_high_level_dump(&r.game, 0, PLAY_HAND_START));

    // Case B: empty board at memory 2 -> cost 7 - 2 = 5 (< 6) -> NOT blocked.
    r.game.memory = 2;
    assert!(!is_empty_board_high_level_dump(&r.game, 0, PLAY_HAND_START));

    // Case C: opponent has a Digimon on board at memory 0 -> NOT blocked.
    r.game.memory = 0;
    r.place_on_field(1, "T-OPP3", Some(0));
    assert!(!is_empty_board_high_level_dump(&r.game, 0, PLAY_HAND_START));
}

#[test]
fn hybrid_v2_teachers_play_tamer_from_hand_instead_of_passing_at_zero_memory() {
    let mut tamer = make_test_card("T-HYB-TAMER", "DualSpiritTamer");
    tamer.card_kind = CardKind::Tamer;
    tamer.colors = vec![CardColor::Blue, CardColor::Yellow];
    tamer.play_cost = 4;
    tamer.level = None;
    tamer.dp = None;
    tamer.inherited_text = "[Your Turn] This Digimon gets +1000 DP.".to_string();

    let mut lv5 = make_test_card("T-LV5-DUMP", "ExpensiveLv5");
    lv5.level = Some(5);
    lv5.play_cost = 8;
    lv5.dp = Some(8000);

    let mut r = DebugRunner::builder()
        .add_card(tamer)
        .add_card(lv5)
        .hand(0, &["T-LV5-DUMP", "T-HYB-TAMER"])
        .start();
    r.skip_mulligan();
    r.set_first_player(0);
    r.game.enter_main_phase();
    r.game.memory = 0;
    let mask = build_action_mask(&r.game, 0);
    let tamer_action = PLAY_HAND_START + 1;
    assert!(mask[tamer_action as usize] > 0.5, "4-cost tamer must be legal");

    assert_eq!(
        greedy_hybrid_v2_action(&r.game, &mask),
        tamer_action,
        "greedy_hybrid_v2 must play the 4-cost Tamer from hand instead of passing or dumping Lv.5"
    );

    let mut dev = DevelopmentPolicy::default();
    let dec = dev.decide_hybrid_v2(&r.game, &mask).unwrap();
    assert_eq!(
        dec.action, tamer_action,
        "development_hybrid_v2 must play the 4-cost Tamer from hand (fixing development_tamer_first_v1 passing over Tamers)"
    );
}

#[test]
fn hybrid_v2_teachers_digivolve_onto_tamer_when_on_board() {
    const HYBRID_LV4_YAML: &str = r#"
card: T-HYB-EVO-LV4
name: LobomonTest
kind: digimon
color: [blue]
level: 4
cost: 5
dp: 6000
traits: [Hybrid]
alt_paths:
  - kind: digivolve
    from: { kind: tamer, color_is: blue }
    cost: 2
effects: []
"#;
    let mut tamer = make_test_card("T-BLUE-TAMER", "KojiTest");
    tamer.card_kind = CardKind::Tamer;
    tamer.colors = vec![CardColor::Blue];
    tamer.play_cost = 4;
    tamer.level = None;
    tamer.dp = None;
    tamer.inherited_text = "[When Attacking] Draw 1.".to_string();

    let filler = make_test_card("T-FILL", "Filler");

    let mut r = DebugRunner::builder()
        .from_dsl_yaml(HYBRID_LV4_YAML)
        .unwrap()
        .add_card(tamer)
        .add_card(filler)
        .hand(0, &["T-HYB-EVO-LV4", "T-BLUE-TAMER"])
        .deck(0, &["T-FILL"; 6])
        .start();
    r.skip_mulligan();
    r.set_first_player(0);
    r.game.enter_main_phase();
    r.game.memory = 2;
    r.place_on_field(0, "T-BLUE-TAMER", Some(0));
    let mask = build_action_mask(&r.game, 0);
    let routes = digimon_engine::policies::tamer_first::legal_tamer_digivolutions(&r.game, &mask);
    assert_eq!(routes.len(), 1, "expected 1 legal tamer digivolution route");
    let expected = routes[0];

    assert_eq!(greedy_hybrid_v2_action(&r.game, &mask), expected);
    let mut dev = DevelopmentPolicy::default();
    assert_eq!(dev.decide_hybrid_v2(&r.game, &mask).unwrap().action, expected);
}
