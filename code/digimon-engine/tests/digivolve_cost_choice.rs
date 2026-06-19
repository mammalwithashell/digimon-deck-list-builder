//! Player-chosen digivolution cost (no-approximations, rule 17).
//!
//! When a base Digimon satisfies MORE THAN ONE of a hand card's digivolution
//! requirements at DIFFERENT costs, the player must be able to CHOOSE which cost
//! to pay. The engine previously auto-selected the cheapest route
//! (`normal_digivolve_route_for_card` did `min_by_key(memory_cost)`), hiding the
//! choice from the RL action space. DCGO registers each requirement with its own
//! `digivolutionCost` (`AddSelfDigivolutionRequirementStaticEffect`) and lets the
//! player pick — this suite pins the faithful behavior.
//!
//! These tests use SYNTHETIC card data (no DSL card / alt-path dependency) so
//! they exercise the engine mechanism in isolation and don't break when a real
//! card's printed data is corrected. Real-card faithfulness (e.g. BT23-037
//! Tentomon's green-Lv.2-cost-1 vs CS-Lv.2-cost-0, BT24-101 Jupitermon's
//! TS-cost-3 vs Aegiochusmon-cost-4) is covered in the per-card behavioral
//! suites.
//!
//! Setup: a Lv.3 evolver whose printed evo-costs are "Blue Lv.2 / cost 1" AND
//! "Green Lv.2 / cost 0". A two-colour (Blue+Green) Lv.2 base satisfies BOTH, so
//! the player chooses 1 vs 0. A single-colour base satisfies only one → no
//! prompt.

use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, GamePhase, PlaySource};
use digimon_engine::selection::SelectionKind;

const EVO_BLUE: u8 = 1;
const EVO_GREEN: u8 = 3;

/// The Lv.3 evolver: digivolves from a Blue Lv.2 (cost 1) OR a Green Lv.2
/// (cost 0).
fn evolver() -> CardData {
    let mut c = make_test_card("EVOLVER", "Evolver");
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.colors = vec![CardColor::Blue];
    c.dp = Some(3000);
    c.evo_costs = vec![
        EvoCost { card_color: EVO_BLUE, level: 2, memory_cost: 1 },
        EvoCost { card_color: EVO_GREEN, level: 2, memory_cost: 0 },
    ];
    c
}

/// A Lv.2 base with the given colours.
fn base(id: &str, colors: Vec<CardColor>) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(2);
    c.colors = colors;
    c.dp = Some(1000);
    c
}

fn runner_with_base(base_card: CardData) -> DebugRunner {
    let base_id = base_card.card_id.clone();
    let mut runner = DebugRunner::builder()
        .add_card(evolver())
        .add_card(base_card)
        .hand(0, &["EVOLVER"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;
    runner.game.current_phase = GamePhase::Main;
    runner.place_on_field(0, &base_id, Some(0));
    runner
}

/// Two-colour base → the engine PROMPTS for which cost to pay (EffectChoice with
/// one option per distinct cost) instead of auto-paying the cheaper.
#[test]
fn two_colour_base_prompts_for_cost_choice() {
    let mut runner = runner_with_base(base("BG-EGG", vec![CardColor::Blue, CardColor::Green]));

    let proceeded = runner.game.digivolve_from_hand(0, 0, 0, PlaySource::ByHand);
    assert!(
        !proceeded,
        "digivolve must PAUSE for a cost choice (return false) instead of auto-paying the min cost"
    );

    let view = runner
        .pending_selection_view()
        .expect("a cost-choice selection must be pending");
    assert_eq!(
        view.kind,
        SelectionKind::EffectChoice,
        "the cost choice is presented as an EffectChoice selection"
    );
    let choices = view
        .effect_choices
        .as_ref()
        .expect("the cost-choice selection must carry effect_choices");
    assert_eq!(
        choices.len(),
        2,
        "exactly two distinct costs (0 and 1) must be offered, got: {:?}",
        choices.iter().map(|c| c.label.clone()).collect::<Vec<_>>()
    );
    let labels: Vec<String> = choices.iter().map(|c| c.label.to_lowercase()).collect();
    assert!(labels.iter().any(|l| l.contains('0')), "an option for cost 0 (labels: {labels:?})");
    assert!(labels.iter().any(|l| l.contains('1')), "an option for cost 1 (labels: {labels:?})");
}

/// Choosing the COSTLIER option (cost 1) actually pays 1 memory — proving the
/// choice is honored, not overridden by the min-cost auto-pick.
#[test]
fn choosing_cost_1_pays_one_memory() {
    let mut runner = runner_with_base(base("BG-EGG", vec![CardColor::Blue, CardColor::Green]));
    let mem_before = runner.game.memory;

    assert!(!runner.game.digivolve_from_hand(0, 0, 0, PlaySource::ByHand));
    let view = runner.pending_selection_view().expect("cost choice pending");
    let choices = view.effect_choices.clone().expect("effect_choices");
    let cost1 = choices.iter().find(|c| c.label.contains('1')).expect("a cost-1 option");
    runner
        .execute_action(0, cost1.action_id)
        .expect("resolve the cost choice with the cost-1 option");

    assert_eq!(
        runner.game.memory,
        mem_before - 1,
        "choosing the cost-1 path must pay exactly 1 memory"
    );
    assert!(runner.pending_selection().is_none());
    let top = runner.game.players[0].battle_area[0]
        .top_card()
        .card_id(&runner.game.card_data)
        .to_string();
    assert_eq!(top, "EVOLVER", "the evolver must be stacked on top after digivolve");
}

/// Choosing the cheaper option (cost 0) pays 0.
#[test]
fn choosing_cost_0_pays_zero_memory() {
    let mut runner = runner_with_base(base("BG-EGG", vec![CardColor::Blue, CardColor::Green]));
    let mem_before = runner.game.memory;

    assert!(!runner.game.digivolve_from_hand(0, 0, 0, PlaySource::ByHand));
    let view = runner.pending_selection_view().expect("cost choice pending");
    let choices = view.effect_choices.clone().expect("effect_choices");
    let cost0 = choices.iter().find(|c| c.label.contains('0')).expect("a cost-0 option");
    runner
        .execute_action(0, cost0.action_id)
        .expect("resolve the cost choice with the cost-0 option");

    assert_eq!(runner.game.memory, mem_before, "the cost-0 path must pay 0 memory");
    assert!(runner.pending_selection().is_none());
}

/// Single-colour base → only one cost applies, so NO choice is prompted and the
/// digivolve completes immediately at that cost.
#[test]
fn single_colour_base_does_not_prompt() {
    let mut runner = runner_with_base(base("BLUE-EGG", vec![CardColor::Blue]));
    let mem_before = runner.game.memory;

    let proceeded = runner.game.digivolve_from_hand(0, 0, 0, PlaySource::ByHand);
    assert!(
        proceeded,
        "a single-cost digivolve must complete immediately (no spurious cost-choice prompt)"
    );
    assert!(runner.pending_selection().is_none());
    assert_eq!(
        runner.game.memory,
        mem_before - 1,
        "the lone applicable path (Blue Lv.2) costs 1 memory"
    );
}
