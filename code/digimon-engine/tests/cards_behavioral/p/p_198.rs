//! P-198 DemiDevimon — Digimon, Lv.3, Purple, Cost 3, DP 1000.
//!
//! # Card text (cards/p/P-198.json)
//!
//! [Start of Your Main Phase] If you have 4 or less memory, this Digimon may
//! digivolve into a Digimon card with the [Fallen Angel] or [TS] trait in the
//! hand without paying the cost.
//!
//! Inherited Effect [When Attacking] [Once Per Turn] <Draw 1> (Draw 1 card
//! from your deck.) and trash 1 card in your hand.
//!
//! # Patterns
//! - Start-of-main optional effect digivolve from hand
//! - Trait-filtered hand selection
//! - Inherited When Attacking OPT draw-then-trash

use std::path::Path;

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledCardKind, CompiledClause, CompiledColor, CompiledCost,
    CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::{PASS, PLAY_HAND_START};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::selection::SelectionKind;
use digimon_engine::{EffectTiming, TriggerSource};

const CARD_ID: &str = "P-198";

fn p_198_yaml() -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("cards/p/P-198.yaml"))
        .expect("P-198 YAML must exist at cards/p/P-198.yaml")
}

fn p_198_runner() -> DebugRunner {
    let yaml = p_198_yaml();
    DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-198 YAML must parse and compile")
        .start()
}

fn digimon_card(id: &str, name: &str, traits: &[&str], level: u8) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(4000);
    card.play_cost = 4;
    card.colors = vec![CardColor::Purple];
    card.traits = traits.iter().map(|trait_name| trait_name.to_string()).collect();
    if level > 2 {
        card.evo_costs = vec![EvoCost {
            card_color: CardColor::Purple as u8,
            level: level - 1,
            memory_cost: 3,
        }];
    }
    card
}

fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

fn hand_ids(runner: &DebugRunner, player: usize) -> Vec<String> {
    zone_ids(&runner.game.players[player].hand, &runner.game.card_data)
}

fn trash_ids(runner: &DebugRunner, player: usize) -> Vec<String> {
    zone_ids(&runner.game.players[player].trash, &runner.game.card_data)
}

fn zone_ids(cards: &[CardSource], data: &[CardData]) -> Vec<String> {
    cards
        .iter()
        .map(|card| card.card_id(data).to_string())
        .collect()
}

#[test]
fn p_198_has_printed_metadata_and_lv2_purple_and_ts_alt_paths() {
    let runner = p_198_runner();
    let compiled = runner.compiled_card(CARD_ID).expect("P-198 compiled");

    assert_eq!(compiled.name, "DemiDevimon");
    assert_eq!(compiled.kind, CompiledCardKind::Digimon);
    assert_eq!(compiled.level, Some(3));
    assert_eq!(compiled.color, vec![CompiledColor::Purple]);
    assert_eq!(compiled.cost, Some(3));
    assert_eq!(compiled.dp, Some(1000));
    assert_eq!(compiled.traits, vec!["Evil".to_string(), "TS".to_string()]);

    let has_purple_lv2_cost_0 = compiled.alt_paths.iter().any(|path| {
        path.kind == CompiledAltPathKind::Digivolve
            && path.cost == Some(CompiledCost::Literal(0))
            && path.from.as_ref().is_some_and(|predicate| {
                predicate.level_eq == Some(2)
                    && predicate.color_is == Some(CompiledColor::Purple)
            })
    });
    assert!(
        has_purple_lv2_cost_0,
        "P-198 must digivolve from purple Lv.2 for cost 0"
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
        "P-198 must digivolve from Lv.2 with [TS] trait for cost 0"
    );
}

#[test]
fn p_198_declares_optional_start_main_and_inherited_when_attacking_opt() {
    let runner = p_198_runner();
    let compiled = runner.compiled_card(CARD_ID).expect("P-198 compiled");

    let start_main = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(triggered)
                if triggered.when == vec![CompiledTiming::StartOfYourMainPhase] =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("start-of-main clause exists");
    assert_eq!(start_main.scope, CompiledScope::FaceUp);
    assert!(
        start_main.optional,
        "printed 'may digivolve' must expose a decline"
    );
    assert!(
        !start_main.once_per_turn,
        "printed start-of-main clause has no OPT marker"
    );
    assert_eq!(
        start_main
            .condition
            .as_ref()
            .and_then(|condition| condition.memory_lte),
        Some(4),
        "start-of-main clause must be gated by memory <= 4"
    );

    let inherited = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(triggered)
                if triggered.scope == CompiledScope::Inherited
                    && triggered.when == vec![CompiledTiming::WhenAttacking] =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("inherited when-attacking clause exists");
    assert!(
        inherited.once_per_turn,
        "inherited text has [Once Per Turn]"
    );
    assert!(
        !inherited.optional,
        "Draw 1 then trash 1 card is mandatory once the inherited trigger fires"
    );
}

#[test]
fn p_198_start_main_prompts_for_fallen_angel_or_ts_hand_digimon_when_memory_is_four_or_less() {
    let yaml = p_198_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-198 YAML parses")
        .add_card(digimon_card("TS-TARGET", "TS Target", &["TS"], 4))
        .add_card(digimon_card(
            "FALLEN-TARGET",
            "Fallen Target",
            &["Fallen Angel"],
            4,
        ))
        .add_card(digimon_card("INVALID-TS", "Invalid TS", &["TS"], 5))
        .add_card(digimon_card("NON-TARGET", "Non Target", &["Evil"], 4))
        .hand(0, &["TS-TARGET", "FALLEN-TARGET", "INVALID-TS", "NON-TARGET"])
        .memory(4)
        .start();
    runner.place_on_field(0, CARD_ID, Some(0));

    runner.game.enter_main_phase();

    let view = runner
        .pending_selection_view()
        .expect("start-of-main should install a hand selection");
    assert_eq!(view.kind, SelectionKind::Hand);
    assert!(
        runner.pending_is_optional(),
        "printed 'may digivolve' must make PASS legal"
    );
    assert!(
        view.valid_action_ids.contains(&PLAY_HAND_START),
        "[TS] target should be legal"
    );
    assert!(
        view.valid_action_ids.contains(&(PLAY_HAND_START + 1)),
        "[Fallen Angel] target should be legal"
    );
    assert!(
        !view.valid_action_ids.contains(&(PLAY_HAND_START + 2)),
        "trait-matching hand Digimon without a valid evo path must not be legal"
    );
    assert!(
        !view.valid_action_ids.contains(&(PLAY_HAND_START + 3)),
        "non-Fallen Angel/non-TS hand Digimon must not be legal"
    );
}

#[test]
fn p_198_start_main_free_digivolves_this_digimon_without_spending_memory() {
    let yaml = p_198_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-198 YAML parses")
        .add_card(digimon_card("TS-TARGET", "TS Target", &["TS"], 4))
        .add_card(filler("DRAW-FILLER"))
        .hand(0, &["TS-TARGET"])
        .deck(0, &["DRAW-FILLER"])
        .deck(1, &["DRAW-FILLER"])
        .memory(4)
        .start();
    let base = runner.place_on_field(0, CARD_ID, Some(0));

    runner.game.enter_main_phase();
    runner
        .execute_action(0, PLAY_HAND_START)
        .expect("choose TS target");
    runner.auto_resolve().expect("finish free digivolve");

    let stack = &runner.game.players[0].battle_area[base.index as usize];
    assert_eq!(
        stack.top_card().card_id(&runner.game.card_data),
        "TS-TARGET",
        "selected hand card should become the top card of this Digimon"
    );
    assert_eq!(
        runner.memory(),
        4,
        "without paying cost must not spend the target's digivolution cost"
    );
    assert!(
        !hand_ids(&runner, 0).contains(&"TS-TARGET".to_string()),
        "selected target leaves hand"
    );
}

#[test]
fn p_198_start_main_can_be_declined() {
    let yaml = p_198_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-198 YAML parses")
        .add_card(digimon_card("TS-TARGET", "TS Target", &["TS"], 4))
        .hand(0, &["TS-TARGET"])
        .memory(4)
        .start();
    let base = runner.place_on_field(0, CARD_ID, Some(0));

    runner.game.enter_main_phase();
    runner
        .execute_action(0, PASS)
        .expect("decline optional digivolve");
    runner.auto_resolve().expect("finish decline");

    let stack = &runner.game.players[0].battle_area[base.index as usize];
    assert_eq!(
        stack.top_card().card_id(&runner.game.card_data),
        CARD_ID,
        "declining leaves DemiDevimon as the top card"
    );
    assert!(
        hand_ids(&runner, 0).contains(&"TS-TARGET".to_string()),
        "declining leaves the target in hand"
    );
}

#[test]
fn p_198_start_main_does_not_prompt_above_four_memory_or_without_matching_target() {
    let yaml = p_198_yaml();
    let mut too_much_memory = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-198 YAML parses")
        .add_card(digimon_card("TS-TARGET", "TS Target", &["TS"], 4))
        .hand(0, &["TS-TARGET"])
        .memory(5)
        .start();
    too_much_memory.place_on_field(0, CARD_ID, Some(0));
    too_much_memory.game.enter_main_phase();
    assert!(
        too_much_memory.pending_selection().is_none(),
        "memory > 4 must gate off the optional digivolve"
    );

    let mut no_target = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-198 YAML parses")
        .add_card(digimon_card("NON-TARGET", "Non Target", &["Evil"], 4))
        .hand(0, &["NON-TARGET"])
        .memory(4)
        .start();
    no_target.place_on_field(0, CARD_ID, Some(0));
    no_target.game.enter_main_phase();
    assert!(
        no_target.pending_selection().is_none(),
        "no Fallen Angel/TS hand Digimon means no player-visible choice"
    );
}

#[test]
fn p_198_inherited_when_attacking_draws_one_then_trashes_one_card_from_hand() {
    let yaml = p_198_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-198 YAML parses")
        .add_card(digimon_card("CARRIER", "Carrier", &["Evil"], 4))
        .add_card(filler("STARTING-HAND"))
        .add_card(filler("DRAWN-CARD"))
        .hand(0, &["STARTING-HAND"])
        .deck(0, &["DRAWN-CARD"])
        .start();
    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);

    runner
        .game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(carrier));
    runner.game.drain_effect_queue();

    let view = runner
        .pending_selection_view()
        .expect("draw then trash should install a hand selection");
    assert_eq!(view.kind, SelectionKind::Hand);
    assert!(
        !runner.pending_is_optional(),
        "trashing 1 card in hand is mandatory after Draw 1"
    );
    assert_eq!(
        runner.deck_size(0),
        0,
        "Draw 1 should happen before the trash prompt"
    );
    assert_eq!(
        runner.hand_size(0),
        2,
        "hand contains starting card plus drawn card before trashing"
    );

    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("trash a hand card");
    runner.auto_resolve().expect("finish inherited effect");

    assert_eq!(runner.hand_size(0), 1, "net hand size is unchanged");
    assert_eq!(runner.trash_size(0), 1, "one hand card is trashed");
}

#[test]
fn p_198_inherited_when_attacking_opt_blocks_second_activation_same_turn() {
    let yaml = p_198_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-198 YAML parses")
        .add_card(digimon_card("CARRIER", "Carrier", &["Evil"], 4))
        .add_card(filler("STARTING-HAND-1"))
        .add_card(filler("STARTING-HAND-2"))
        .add_card(filler("DRAWN-1"))
        .add_card(filler("DRAWN-2"))
        .hand(0, &["STARTING-HAND-1", "STARTING-HAND-2"])
        .deck(0, &["DRAWN-2", "DRAWN-1"])
        .start();
    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);

    runner
        .game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(carrier));
    runner.game.drain_effect_queue();
    let first_trash = runner
        .pending_selection_view()
        .expect("first activation trashes a hand card")
        .valid_action_ids[0];
    runner
        .execute_action(0, first_trash)
        .expect("trash for first activation");
    runner.auto_resolve().expect("finish first activation");

    let hand_after_first = hand_ids(&runner, 0);
    let trash_after_first = trash_ids(&runner, 0);
    let deck_after_first = runner.deck_size(0);

    runner
        .game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(carrier));
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "OPT must suppress the second activation prompt"
    );
    assert_eq!(
        runner.deck_size(0),
        deck_after_first,
        "second activation must not draw"
    );
    assert_eq!(
        hand_ids(&runner, 0),
        hand_after_first,
        "second activation must not change hand"
    );
    assert_eq!(
        trash_ids(&runner, 0),
        trash_after_first,
        "second activation must not trash another card"
    );
}
