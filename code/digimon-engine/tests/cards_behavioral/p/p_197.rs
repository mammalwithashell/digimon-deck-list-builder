//! P-197 Patamon — Digimon, Lv.3, Yellow, Cost 3, DP 1000.
//!
//! Printed text from `code/digimon-engine/cards/p/P-197.json`:
//! [Start of Your Main Phase] If you have 4 or less memory, this Digimon may
//! digivolve into a Digimon card with the [Angel] or [TS] trait in the hand
//! without paying the cost.
//!
//! Inherited: [When Attacking] [Once Per Turn] 1 of your opponent's Digimon
//! gets -2000 DP for the turn.

use std::path::Path;

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledCardKind, CompiledClause, CompiledColor, CompiledCost,
    CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::{PASS, PLAY_HAND_START};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, PlaySource};
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "P-197";

fn p_197_yaml() -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("cards/p/P-197.yaml"))
        .expect("P-197 YAML must exist at cards/p/P-197.yaml")
}

fn p_197_runner() -> DebugRunner {
    let yaml = p_197_yaml();
    DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-197 YAML must parse and compile")
        .start()
}

fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

fn egg(id: &str, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::DigiEgg;
    card.level = Some(2);
    card.dp = None;
    card.play_cost = 0;
    card.colors = vec![CardColor::Yellow];
    card.traits = traits
        .iter()
        .map(|trait_name| (*trait_name).to_string())
        .collect();
    card
}

fn yellow_lv4(id: &str, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(4000);
    card.play_cost = 4;
    card.colors = vec![CardColor::Yellow];
    card.traits = traits
        .iter()
        .map(|trait_name| (*trait_name).to_string())
        .collect();
    card.evo_costs = vec![EvoCost {
        card_color: CardColor::Yellow as u8,
        level: 3,
        memory_cost: 3,
    }];
    card
}

fn invalid_yellow_lv4(id: &str, traits: &[&str]) -> CardData {
    let mut card = yellow_lv4(id, traits);
    card.evo_costs = Vec::new();
    card
}

fn accept_optional_trigger(runner: &mut DebugRunner) {
    let view = runner
        .pending_selection_view()
        .expect("optional trigger choice must be visible");
    if view.kind == SelectionKind::TriggerOrder {
        assert!(
            view.is_optional,
            "printed 'may digivolve' must expose a PASSable trigger choice"
        );
        let action = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|action| *action != PASS)
            .expect("optional trigger should include a non-PASS action");
        runner
            .execute_action(0, action)
            .expect("accept P-197 start-main trigger");
    }
}

fn hand_contains_card(runner: &DebugRunner, player: usize, card_id: &str) -> bool {
    runner.game.players[player]
        .hand
        .iter()
        .any(|card| card.card_id(&runner.game.card_data) == card_id)
}

#[test]
fn p_197_has_printed_metadata_alt_paths_and_clauses() {
    let runner = p_197_runner();
    let compiled = runner.compiled_card(CARD_ID).expect("P-197 compiled");

    assert_eq!(compiled.name, "Patamon");
    assert_eq!(compiled.kind, CompiledCardKind::Digimon);
    assert_eq!(compiled.level, Some(3));
    assert_eq!(compiled.color, vec![CompiledColor::Yellow]);
    assert_eq!(compiled.cost, Some(3));
    assert_eq!(compiled.dp, Some(1000));
    assert_eq!(
        compiled.traits,
        vec!["Mammal".to_string(), "TS".to_string()]
    );

    let has_yellow_lv2_cost_0 = compiled.alt_paths.iter().any(|path| {
        path.kind == CompiledAltPathKind::Digivolve
            && path.cost == Some(CompiledCost::Literal(0))
            && path.from.as_ref().is_some_and(|predicate| {
                predicate.level_eq == Some(2) && predicate.color_is == Some(CompiledColor::Yellow)
            })
    });
    assert!(
        has_yellow_lv2_cost_0,
        "P-197 must digivolve from yellow level 2 for cost 0"
    );

    let has_ts_lv2_cost_0 = compiled.alt_paths.iter().any(|path| {
        path.kind == CompiledAltPathKind::Digivolve
            && path.cost == Some(CompiledCost::Literal(0))
            && path.from.as_ref().is_some_and(|predicate| {
                predicate.all_of.iter().any(|leaf| leaf.level_eq == Some(2))
                    && predicate
                        .all_of
                        .iter()
                        .any(|leaf| leaf.trait_has.as_deref() == Some("TS"))
            })
    });
    assert!(
        has_ts_lv2_cost_0,
        "P-197 must also digivolve from level 2 with [TS] trait for cost 0"
    );

    let start_main = compiled
        .effects
        .iter()
        .filter_map(|clause| match clause {
            CompiledClause::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .find(|triggered| {
            triggered
                .when
                .contains(&CompiledTiming::StartOfYourMainPhase)
        })
        .expect("P-197 must have a Start of Your Main Phase clause");
    assert_eq!(start_main.scope, CompiledScope::FaceUp);
    assert!(start_main.optional, "printed 'may digivolve' is optional");

    let inherited = compiled
        .effects
        .iter()
        .filter_map(|clause| match clause {
            CompiledClause::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .find(|triggered| triggered.when.contains(&CompiledTiming::WhenAttacking))
        .expect("P-197 must have inherited When Attacking");
    assert_eq!(inherited.scope, CompiledScope::Inherited);
    assert!(inherited.once_per_turn, "inherited clause is Once Per Turn");
    assert!(
        !inherited.optional,
        "inherited DP reduction has no 'may' in printed text"
    );
}

#[test]
fn p_197_start_main_offers_visible_optional_hand_digivolve_at_memory_4() {
    let yaml = p_197_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-197 YAML parses")
        .add_card(yellow_lv4("ANGEL-EVO", &["Angel"]))
        .hand(0, &["ANGEL-EVO"])
        .memory(4)
        .start();
    let patamon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.memory = 4;

    runner.game.enqueue_triggered(
        EffectTiming::StartOfYourMainPhase,
        TriggerSource::Permanent(patamon),
    );
    runner.game.drain_effect_queue();

    let view = runner
        .pending_selection_view()
        .expect("start-main effect must install a visible selection");
    if view.kind == SelectionKind::TriggerOrder {
        accept_optional_trigger(&mut runner);
    }
    let view = runner
        .pending_selection_view()
        .expect("start-main effect must install a hand selection");
    assert_eq!(view.kind, SelectionKind::Hand);
    assert!(
        view.is_optional,
        "the hand selection itself must allow declining the 'may digivolve'"
    );
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only the Angel/TS Digimon in hand should be selectable"
    );
}

#[test]
fn p_197_start_main_digivolves_into_angel_or_ts_without_spending_memory() {
    let yaml = p_197_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-197 YAML parses")
        .add_card(yellow_lv4("ANGEL-EVO", &["Angel"]))
        .add_card(yellow_lv4("TS-EVO", &["TS"]))
        .add_card(invalid_yellow_lv4("INVALID-TS-EVO", &["TS"]))
        .hand(0, &["ANGEL-EVO", "TS-EVO", "INVALID-TS-EVO"])
        .memory(4)
        .start();
    let patamon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.memory = 4;
    let memory_before = runner.memory();

    runner.game.enqueue_triggered(
        EffectTiming::StartOfYourMainPhase,
        TriggerSource::Permanent(patamon),
    );
    runner.game.drain_effect_queue();
    accept_optional_trigger(&mut runner);

    let hand_view = runner
        .pending_selection_view()
        .expect("Angel/TS hand selection");
    assert!(
        hand_view.valid_action_ids.contains(&PLAY_HAND_START),
        "Angel target with a valid evo path must be legal"
    );
    assert!(
        hand_view.valid_action_ids.contains(&(PLAY_HAND_START + 1)),
        "TS target with a valid evo path must be legal"
    );
    assert!(
        !hand_view.valid_action_ids.contains(&(PLAY_HAND_START + 2)),
        "trait-matching hand card without a valid evo path must not be legal"
    );
    runner
        .execute_action(0, PLAY_HAND_START + 1)
        .expect("choose TS Digimon");
    runner.auto_resolve().expect("finish free digivolve");

    let stack = &runner.game.players[0].battle_area[patamon.index as usize];
    let top_id = stack.top_card().card_id(&runner.game.card_data);
    assert!(
        top_id == "TS-EVO",
        "selected TS Digimon should become the top card; got {top_id}"
    );
    assert_eq!(
        runner.memory(),
        memory_before,
        "without paying the cost must not spend digivolution memory"
    );
}

#[test]
fn p_197_start_main_can_be_declined() {
    let yaml = p_197_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-197 YAML parses")
        .add_card(yellow_lv4("ANGEL-EVO", &["Angel"]))
        .hand(0, &["ANGEL-EVO"])
        .memory(4)
        .start();
    let patamon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.memory = 4;
    let hand_before = runner.hand_size(0);

    runner.game.enqueue_triggered(
        EffectTiming::StartOfYourMainPhase,
        TriggerSource::Permanent(patamon),
    );
    runner.game.drain_effect_queue();

    let view = runner
        .pending_selection_view()
        .expect("optional trigger choice must be visible");
    assert!(
        matches!(view.kind, SelectionKind::TriggerOrder | SelectionKind::Hand),
        "printed optional digivolve must expose either a trigger choice or hand selection; got {:?}",
        view.kind
    );
    assert!(view.is_optional);
    runner
        .execute_action(0, PASS)
        .expect("decline P-197 start-main digivolve");
    runner.auto_resolve().expect("settle declined trigger");

    assert_eq!(runner.hand_size(0), hand_before);
    assert!(hand_contains_card(&runner, 0, "ANGEL-EVO"));
    assert_eq!(
        runner.game.players[0].battle_area[patamon.index as usize]
            .top_card()
            .card_id(&runner.game.card_data),
        CARD_ID,
        "declining must leave Patamon as the top card"
    );
}

#[test]
fn p_197_start_main_does_not_offer_when_memory_is_5_or_hand_has_no_angel_or_ts() {
    let yaml = p_197_yaml();
    let mut memory_runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-197 YAML parses")
        .add_card(yellow_lv4("ANGEL-EVO", &["Angel"]))
        .hand(0, &["ANGEL-EVO"])
        .memory(5)
        .start();
    let patamon = memory_runner.place_on_field(0, CARD_ID, Some(0));
    memory_runner.game.memory = 5;

    memory_runner.game.enqueue_triggered(
        EffectTiming::StartOfYourMainPhase,
        TriggerSource::Permanent(patamon),
    );
    memory_runner.game.drain_effect_queue();
    assert!(
        memory_runner.pending_selection_view().is_none(),
        "memory > 4 must fail the printed condition"
    );

    let mut trait_runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-197 YAML parses")
        .add_card(yellow_lv4("NON-MATCHING-EVO", &["Mammal"]))
        .hand(0, &["NON-MATCHING-EVO"])
        .memory(4)
        .start();
    let patamon = trait_runner.place_on_field(0, CARD_ID, Some(0));
    trait_runner.game.memory = 4;

    trait_runner.game.enqueue_triggered(
        EffectTiming::StartOfYourMainPhase,
        TriggerSource::Permanent(patamon),
    );
    trait_runner.game.drain_effect_queue();
    assert!(
        trait_runner.pending_selection_view().is_none(),
        "hand cards without [Angel] or [TS] must not satisfy the hand digivolve"
    );
}

#[test]
fn p_197_inherited_when_attacking_gives_minus_2000_dp_once_per_turn() {
    let yaml = p_197_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-197 YAML parses")
        .add_card(filler("CARRIER"))
        .add_card(filler("OPP"))
        .start();
    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    let opp = runner.place_on_field(1, "OPP", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(carrier),
    );
    runner.game.drain_effect_queue();

    let view = runner.pending_selection_view().expect("opponent target");
    assert_eq!(view.kind, SelectionKind::OppField);
    assert!(
        !view.is_optional,
        "inherited DP reduction has no printed 'may'"
    );
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("choose opponent Digimon");
    runner.auto_resolve().expect("finish DP modifier");

    assert_eq!(runner.game.effective_dp(opp), Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(carrier),
    );
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection_view().is_none(),
        "Once Per Turn inherited effect must not offer a second target in the same turn"
    );
    assert_eq!(
        runner.game.effective_dp(opp),
        Some(0),
        "second same-turn trigger must not apply another -2000 DP"
    );
}

#[test]
fn p_197_can_digivolve_from_yellow_or_ts_level_2_for_cost_0() {
    let yaml = p_197_yaml();
    let mut yellow_runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-197 YAML parses")
        .add_card(egg("YELLOW-LV2", &[]))
        .hand(0, &[CARD_ID])
        .memory(0)
        .start();
    let yellow_base = yellow_runner.place_on_field(0, "YELLOW-LV2", Some(0));
    assert!(
        yellow_runner.game.digivolve_from_hand(
            0,
            0,
            yellow_base.index as usize,
            PlaySource::ByHand,
        ),
        "P-197 should digivolve from yellow Lv.2 for cost 0"
    );
    assert_eq!(yellow_runner.memory(), 0);

    let mut ts_runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-197 YAML parses")
        .add_card(egg("TS-LV2", &["TS"]))
        .hand(0, &[CARD_ID])
        .memory(0)
        .start();
    let ts_base = ts_runner.place_on_field(0, "TS-LV2", Some(0));
    assert!(
        ts_runner
            .game
            .digivolve_from_hand(0, 0, ts_base.index as usize, PlaySource::ByHand),
        "P-197 should digivolve from any Lv.2 with [TS] for cost 0"
    );
    assert_eq!(ts_runner.memory(), 0);
}
