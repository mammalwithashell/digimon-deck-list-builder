//! P-196 Gomamon — Digimon, Lv.3, Blue, Cost 3, DP 1000.
//!
//! # Card text (cards/p/P-196.json)
//!
//! [Start of Your Main Phase] If you have 4 or less memory, this Digimon may
//! digivolve into a Digimon card with the [Sea Beast] or [TS] trait in the hand
//! without paying the cost.
//!
//! Inherited Effect [When Attacking] [Once Per Turn] If you have 7 or fewer
//! cards in your hand, <Draw 1> (Draw 1 card from your deck.)
//!
//! # Patterns
//! - TS Olympos Lv.3 blue/TS cost-0 alternate digivolution paths
//! - Start-of-main optional effect-initiated digivolve from hand
//! - Inherited When Attacking OPT hand-count-gated Draw 1

use std::path::Path;

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledCardKind, CompiledClause, CompiledColor, CompiledCost,
    CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::{PASS, PLAY_HAND_START};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "P-196";

fn p_196_yaml() -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("cards/p/P-196.yaml"))
        .expect("P-196 YAML must exist at cards/p/P-196.yaml")
}

fn p_196_runner() -> DebugRunner {
    let yaml = p_196_yaml();
    DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-196 YAML must parse and compile")
        .memory(4)
        .start()
}

fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

fn blue_lv3_base(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(3);
    card.dp = Some(1000);
    card.play_cost = 3;
    card.colors = vec![CardColor::Blue];
    card.traits = vec!["Sea Beast".to_string(), "TS".to_string()];
    card
}

fn sea_beast_lv4(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(5000);
    card.play_cost = 5;
    card.colors = vec![CardColor::Blue];
    card.traits = vec!["Sea Beast".to_string()];
    card.evo_costs = vec![EvoCost {
        card_color: CardColor::Blue as u8,
        level: 3,
        memory_cost: 2,
    }];
    card
}

fn ts_lv4(id: &str) -> CardData {
    let mut card = sea_beast_lv4(id);
    card.traits = vec!["TS".to_string()];
    card
}

fn invalid_ts_lv4(id: &str) -> CardData {
    let mut card = ts_lv4(id);
    card.colors = vec![CardColor::Yellow];
    card.evo_costs = vec![EvoCost {
        card_color: CardColor::Yellow as u8,
        level: 3,
        memory_cost: 2,
    }];
    card
}

fn unrelated_lv4(id: &str) -> CardData {
    let mut card = sea_beast_lv4(id);
    card.traits = vec!["Mammal".to_string()];
    card
}

fn place_p_196_as_source(runner: &mut DebugRunner, carrier_id: &str) -> PermanentHandle {
    let handle = runner.place_on_field(0, carrier_id, Some(0));
    runner.push_source(handle, CARD_ID);
    handle
}

fn hand_contains(runner: &DebugRunner, player: usize, card_id: &str) -> bool {
    runner.game.players[player]
        .hand
        .iter()
        .any(|card| card.card_id(&runner.game.card_data) == card_id)
}

#[test]
fn p_196_has_printed_metadata_and_blue_lv2_ts_lv2_cost_zero_paths() {
    let runner = p_196_runner();
    let compiled = runner.compiled_card(CARD_ID).expect("P-196 compiled");

    assert_eq!(compiled.name, "Gomamon");
    assert_eq!(compiled.kind, CompiledCardKind::Digimon);
    assert_eq!(compiled.level, Some(3));
    assert_eq!(compiled.color, vec![CompiledColor::Blue]);
    assert_eq!(compiled.cost, Some(3));
    assert_eq!(compiled.dp, Some(1000));
    assert_eq!(
        compiled.traits,
        vec!["Sea Beast".to_string(), "TS".to_string()]
    );

    let has_blue_lv2_cost_0 = compiled.alt_paths.iter().any(|path| {
        path.kind == CompiledAltPathKind::Digivolve
            && path.cost == Some(CompiledCost::Literal(0))
            && path.from.as_ref().is_some_and(|predicate| {
                predicate.all_of.iter().any(|p| p.level_eq == Some(2))
                    && predicate
                        .all_of
                        .iter()
                        .any(|p| p.color_is == Some(CompiledColor::Blue))
            })
    });
    assert!(
        has_blue_lv2_cost_0,
        "P-196 must digivolve from blue level 2 for cost 0"
    );

    let has_ts_lv2_cost_0 = compiled.alt_paths.iter().any(|path| {
        path.kind == CompiledAltPathKind::Digivolve
            && path.cost == Some(CompiledCost::Literal(0))
            && path.from.as_ref().is_some_and(|predicate| {
                predicate.all_of.iter().any(|p| p.level_eq == Some(2))
                    && predicate
                        .all_of
                        .iter()
                        .any(|p| p.trait_has.as_deref() == Some("TS"))
            })
    });
    assert!(
        has_ts_lv2_cost_0,
        "P-196 must also digivolve from level 2 with [TS] trait for cost 0"
    );
}

#[test]
fn p_196_has_start_main_and_inherited_when_attacking_opt_clauses() {
    let runner = p_196_runner();
    let compiled = runner.compiled_card(CARD_ID).expect("P-196 compiled");

    let triggered: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|clause| match clause {
            CompiledClause::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .collect();

    let start_main = triggered
        .iter()
        .find(|clause| clause.when.contains(&CompiledTiming::StartOfYourMainPhase))
        .expect("start_of_your_main_phase clause must exist");
    assert_eq!(start_main.scope, CompiledScope::FaceUp);

    let inherited_attack = triggered
        .iter()
        .find(|clause| clause.when.contains(&CompiledTiming::WhenAttacking))
        .expect("inherited when_attacking clause must exist");
    assert_eq!(inherited_attack.scope, CompiledScope::Inherited);
    assert!(
        inherited_attack.once_per_turn,
        "printed inherited effect is [Once Per Turn]"
    );
}

#[test]
fn p_196_start_main_at_four_memory_offers_optional_sea_beast_hand_digivolve() {
    let yaml = p_196_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-196 YAML parses")
        .add_card(sea_beast_lv4("SEA-BEAST-EVO"))
        .add_card(invalid_ts_lv4("INVALID-TS-EVO"))
        .add_card(filler("FILL"))
        .hand(0, &["SEA-BEAST-EVO", "INVALID-TS-EVO"])
        .deck(0, &["FILL", "FILL"])
        .deck(1, &["FILL", "FILL"])
        .memory(4)
        .start();

    let gomamon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.enter_main_phase();

    assert_eq!(runner.pending_kind(), Some(SelectionKind::Hand));
    assert!(
        runner.pending_is_optional(),
        "printed 'may digivolve' must expose PASS"
    );
    assert_eq!(
        runner.pending_selection_view().unwrap().valid_action_ids,
        vec![PLAY_HAND_START],
        "only trait-matching cards that can digivolve from P-196 should be legal"
    );

    let memory_before = runner.memory();
    runner
        .execute_action(0, PLAY_HAND_START)
        .expect("select Sea Beast evolution");
    runner.auto_resolve().expect("finish effect digivolve");

    let stack = &runner.game.players[0].battle_area[gomamon.index as usize];
    assert_eq!(
        stack.top_card().card_id(&runner.game.card_data),
        "SEA-BEAST-EVO",
        "selected Sea Beast card must become the top card"
    );
    assert!(!hand_contains(&runner, 0, "SEA-BEAST-EVO"));
    assert_eq!(
        runner.memory(),
        memory_before,
        "without paying the cost must spend no memory"
    );
}

#[test]
fn p_196_start_main_accepts_ts_hand_digivolve() {
    let yaml = p_196_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-196 YAML parses")
        .add_card(ts_lv4("TS-EVO"))
        .add_card(filler("FILL"))
        .hand(0, &["TS-EVO"])
        .deck(0, &["FILL", "FILL"])
        .deck(1, &["FILL", "FILL"])
        .memory(4)
        .start();

    let gomamon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.enter_main_phase();

    assert_eq!(
        runner.pending_selection_view().unwrap().valid_action_ids,
        vec![PLAY_HAND_START],
        "TS Digimon in hand must be a legal evolution card"
    );
    runner
        .execute_action(0, PLAY_HAND_START)
        .expect("select TS evolution");
    runner.auto_resolve().expect("finish TS digivolve");

    let stack = &runner.game.players[0].battle_area[gomamon.index as usize];
    assert_eq!(stack.top_card().card_id(&runner.game.card_data), "TS-EVO");
}

#[test]
fn p_196_start_main_can_be_declined() {
    let yaml = p_196_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-196 YAML parses")
        .add_card(sea_beast_lv4("SEA-BEAST-EVO"))
        .hand(0, &["SEA-BEAST-EVO"])
        .memory(4)
        .start();

    let gomamon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.enter_main_phase();

    assert_eq!(runner.pending_kind(), Some(SelectionKind::Hand));
    runner
        .execute_action(0, PASS)
        .expect("decline optional digivolve");
    runner.auto_resolve().expect("finish decline");

    let stack = &runner.game.players[0].battle_area[gomamon.index as usize];
    assert_eq!(stack.top_card().card_id(&runner.game.card_data), CARD_ID);
    assert!(hand_contains(&runner, 0, "SEA-BEAST-EVO"));
}

#[test]
fn p_196_start_main_does_not_prompt_above_four_memory() {
    let yaml = p_196_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-196 YAML parses")
        .add_card(sea_beast_lv4("SEA-BEAST-EVO"))
        .hand(0, &["SEA-BEAST-EVO"])
        .memory(5)
        .start();

    runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.enter_main_phase();

    assert!(
        runner.pending_selection().is_none(),
        "memory > 4 must suppress the start-of-main digivolve prompt"
    );
}

#[test]
fn p_196_start_main_filters_out_non_sea_beast_non_ts_cards() {
    let yaml = p_196_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-196 YAML parses")
        .add_card(unrelated_lv4("UNRELATED-EVO"))
        .hand(0, &["UNRELATED-EVO"])
        .memory(4)
        .start();

    runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.enter_main_phase();

    assert!(
        runner.pending_selection().is_none(),
        "non-Sea Beast / non-TS cards must not be selectable"
    );
}

#[test]
fn p_196_start_main_filters_out_trait_match_without_valid_evo_path() {
    let yaml = p_196_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-196 YAML parses")
        .add_card(invalid_ts_lv4("INVALID-TS-EVO"))
        .hand(0, &["INVALID-TS-EVO"])
        .memory(4)
        .start();

    runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.enter_main_phase();

    assert!(
        runner.pending_selection().is_none(),
        "a trait-matching hand card without a valid evo path from P-196 must not be selectable"
    );
}

#[test]
fn p_196_inherited_when_attacking_draws_one_at_seven_or_fewer_cards() {
    let yaml = p_196_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-196 YAML parses")
        .add_card(blue_lv3_base("CARRIER"))
        .add_card(filler("FILL"))
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL"])
        .security(1, &["FILL", "FILL", "FILL"])
        .memory(5)
        .start();

    let carrier = place_p_196_as_source(&mut runner, "CARRIER");
    let hand_before = runner.hand_size(0);

    runner.attack_player(carrier, 1, false);
    runner.auto_resolve().expect("resolve inherited draw");

    assert_eq!(
        runner.hand_size(0),
        hand_before + 1,
        "inherited effect must draw exactly 1 when hand count is <= 7"
    );
}

#[test]
fn p_196_inherited_when_attacking_does_not_draw_above_seven_cards() {
    let yaml = p_196_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-196 YAML parses")
        .add_card(blue_lv3_base("CARRIER"))
        .add_card(filler("FILL"))
        .hand(
            0,
            &[
                "FILL", "FILL", "FILL", "FILL", "FILL", "FILL", "FILL", "FILL",
            ],
        )
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL"])
        .security(1, &["FILL", "FILL", "FILL"])
        .memory(5)
        .start();

    let carrier = place_p_196_as_source(&mut runner, "CARRIER");
    let hand_before = runner.hand_size(0);

    runner.attack_player(carrier, 1, false);
    runner.auto_resolve().expect("resolve attack");

    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "hand count above 7 must gate off the inherited draw"
    );
}

#[test]
fn p_196_inherited_when_attacking_is_once_per_turn() {
    let yaml = p_196_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-196 YAML parses")
        .add_card(blue_lv3_base("CARRIER"))
        .add_card(filler("FILL"))
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL"])
        .security(1, &["FILL", "FILL", "FILL"])
        .memory(5)
        .start();

    let carrier = place_p_196_as_source(&mut runner, "CARRIER");

    runner.attack_player(carrier, 1, false);
    runner.auto_resolve().expect("resolve first attack draw");
    let hand_after_first = runner.hand_size(0);

    runner.attack_player(carrier, 1, false);
    runner.auto_resolve().expect("resolve second attack");

    assert_eq!(
        runner.hand_size(0),
        hand_after_first,
        "OPT must block a second inherited draw in the same turn"
    );
}
