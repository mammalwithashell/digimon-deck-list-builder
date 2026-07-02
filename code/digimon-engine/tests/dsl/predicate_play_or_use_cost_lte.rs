//! G-PLAY-OR-USE-COST-LTE — the `play_or_use_cost_lte: N` card-subject predicate
//! leaf. It admits a hand card iff `max(play_cost, option_use_cost) <= N`:
//!   - Digimon / Tamer → `play_cost` (option_use_cost is None).
//!   - pure Option → play and use cost coincide.
//!   - Dual → the max of the Digimon-face play cost and the Option-face use cost.
//!
//! Mirrors DCGO `CardSource.GetCostItself <= N` over a "play or use 1 ... card
//! with a play or use cost of N or less" hand filter (ST24-06 RizeGreymon).
//!
//! The test authors an inline card whose [On Play] body is an optional
//! `select_hand { filter: { play_or_use_cost_lte: 5 } }`, fires it, and asserts
//! the parked selection offers ONLY the eligible hand indices.

use digimon_engine::action::space::PLAY_HAND_START;
use digimon_engine::card_data::{CardData, DualCardData, DualDigimonFace, DualOptionFace};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::CardKind;

const SELECTOR_YAML: &str = r#"
card: TEST-POU
name: Play Or Use Selector
kind: digimon
level: 3
color: [yellow]
cost: 3
dp: 3000
effects:
  - when: on_play
    process:
      - select_hand:
          of: you
          bind_as: pick
          optional: true
          filter: { play_or_use_cost_lte: 5 }
          prompt: "Pick a cost<=5 play-or-use card"
"#;

fn digimon(id: &str, cost: u16) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = cost;
    c
}

fn option(id: &str, use_cost: u16) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Option;
    c.level = None;
    c.dp = None;
    c.play_cost = use_cost; // a pure Option's use cost lives in play_cost
    c
}

fn dual(id: &str, play_cost: u16, use_cost: u16) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Dual;
    c.play_cost = play_cost;
    c.dual = Some(DualCardData {
        digimon: DualDigimonFace {
            level: 4,
            dp: 4000,
            colors: vec![],
            traits: vec![],
            evo_costs: vec![],
            effect_text: String::new(),
            inherited_text: String::new(),
            keywords: vec![],
        },
        option: DualOptionFace {
            use_cost,
            colors: vec![],
            effect_text: String::new(),
            security_text: String::new(),
            keywords: vec![],
        },
    });
    c
}

/// Fire TEST-POU's [On Play] select_hand and return the offered hand indices
/// (the parked selection's legal `PLAY_HAND_START + i` actions).
fn offered_hand_indices(runner: &mut DebugRunner) -> Vec<usize> {
    let sel = runner.place_on_field(0, "TEST-POU", Some(0));
    runner.fire_on_play(0, sel.index as usize);
    let hand_len = runner.game.players[0].hand.len();
    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("the optional select_hand installs a pending selection");
    (0..hand_len)
        .filter(|&i| {
            pending
                .valid_action_ids
                .contains(&(PLAY_HAND_START + i as u16))
        })
        .collect()
}

#[test]
fn play_or_use_cost_lte_admits_digimon_at_or_below_ceiling() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(SELECTOR_YAML)
        .expect("selector YAML parses")
        .add_card(digimon("D-COST5", 5)) // eligible (== ceiling)
        .add_card(digimon("D-COST6", 6)) // excluded (> ceiling)
        .deck(0, &["D-COST5"; 4])
        .deck(1, &["D-COST5"; 4])
        .hand(0, &["D-COST5", "D-COST6"])
        .memory(10)
        .start();

    let offered = offered_hand_indices(&mut runner);
    assert_eq!(
        offered,
        vec![0],
        "only the cost-5 Digimon (index 0) is offered"
    );
}

#[test]
fn play_or_use_cost_lte_admits_option_by_use_cost() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(SELECTOR_YAML)
        .expect("selector YAML parses")
        .add_card(option("O-USE5", 5)) // eligible
        .add_card(option("O-USE7", 7)) // excluded
        .deck(0, &["O-USE5"; 4])
        .deck(1, &["O-USE5"; 4])
        .hand(0, &["O-USE5", "O-USE7"])
        .memory(10)
        .start();

    let offered = offered_hand_indices(&mut runner);
    assert_eq!(
        offered,
        vec![0],
        "only the use-cost-5 Option (index 0) is offered"
    );
}

#[test]
fn play_or_use_cost_lte_uses_max_of_both_dual_faces() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(SELECTOR_YAML)
        .expect("selector YAML parses")
        // Dual whose Digimon-face play cost is 5 but Option-face use cost is 7:
        // max(5, 7) = 7 > 5 → EXCLUDED.
        .add_card(dual("DUAL-5-7", 5, 7))
        // Dual whose both faces are <= 5: max(4, 5) = 5 → eligible.
        .add_card(dual("DUAL-4-5", 4, 5))
        .deck(0, &["DUAL-4-5"; 4])
        .deck(1, &["DUAL-4-5"; 4])
        .hand(0, &["DUAL-5-7", "DUAL-4-5"])
        .memory(10)
        .start();

    let offered = offered_hand_indices(&mut runner);
    assert_eq!(
        offered,
        vec![1],
        "the Dual whose costlier face exceeds 5 is excluded; only the max-5 Dual is offered"
    );
}
