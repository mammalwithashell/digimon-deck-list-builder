//! BT24-091 Tidal Stream.

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledDeclarativeClause, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::{encode_attack, PASS};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::selection::{OptionPlayResult, SelectionKind};

const YAML: &str = include_str!("../../../cards/bt24/BT24-091.yaml");

#[test]
fn bt24_091_has_use_requirement_main_security_and_link_requirement() {
    let runner = tidal_runner().start();
    let compiled = runner
        .compiled_card("BT24-091")
        .expect("BT24-091 compiled card present");

    assert_eq!(compiled.card, "BT24-091");
    assert_eq!(compiled.name, "Tidal Stream");
    assert_eq!(compiled.kind, CompiledCardKind::Option);
    assert_eq!(compiled.cost, Some(5));
    assert!(compiled.traits.iter().any(|t| t == "TS"));
    assert!(
        compiled.use_requirement.is_some(),
        "TS Digimon/Tamer color bypass must compile as a use_requirement"
    );
    assert!(compiled.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Declarative(CompiledDeclarativeClause::FloodGate { modifier, .. })
            if modifier == "IgnoreColorRequirement"
    )));

    let main = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(t) if t.when == vec![CompiledTiming::MainFromHand] => Some(t),
            _ => None,
        })
        .expect("MainFromHand clause");
    assert!(main.process.iter().any(|step| matches!(
        step,
        CompiledStep::ForEach { over, .. }
            if over.level_matches_aggregate.is_some()
    )));
    assert!(main.process.iter().any(|step| matches!(
        step,
        CompiledStep::If { condition, .. } if condition.effect_returned_any_card == Some(true)
    )));
    assert!(main.process.iter().any(|step| matches!(
        step,
        CompiledStep::LinkToOwnDigimon {
            optional: true,
            free: true,
            ..
        }
    )));

    assert!(compiled.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Triggered(t) if t.scope == digimon_dsl::compiled::CompiledScope::Inherited
            && t.when == vec![CompiledTiming::OnSecurity]
    )));
    assert!(compiled.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Declarative(CompiledDeclarativeClause::LinkRequirement { cost: 3, filter, .. })
            if filter.trait_has.as_deref() == Some("TS")
    )));
}

#[test]
fn bt24_091_main_returns_all_lowest_level_then_unsuspends_ts_and_can_decline_link() {
    let mut runner = tidal_runner().hand(0, &["BT24-091"]).memory(20).start();
    runner.place_on_field(0, "TS-TAMER", Some(0));
    let own_ts = runner.place_on_field(0, "TS-DIGIMON", Some(0));
    runner.game.players[0].battle_area[own_ts.index as usize].is_suspended = true;
    runner.place_on_field(1, "OPP-LV3-A", Some(0));
    runner.place_on_field(1, "OPP-LV3-B", Some(0));
    runner.place_on_field(1, "OPP-LV5", Some(0));
    runner.game.enter_main_phase();

    assert_eq!(play_tidal_standard(&mut runner), OptionPlayResult::Pending);

    assert!(
        !battle_area_contains(&runner, 1, "OPP-LV3-A")
            && !battle_area_contains(&runner, 1, "OPP-LV3-B"),
        "all tied lowest-level opponent Digimon should return to hand"
    );
    assert!(
        battle_area_contains(&runner, 1, "OPP-LV5"),
        "higher-level opponent Digimon must remain on field"
    );
    assert!(
        hand_contains(&runner, 1, "OPP-LV3-A") && hand_contains(&runner, 1, "OPP-LV3-B"),
        "returned lowest-level Digimon should be in the opponent's hand"
    );

    let view = runner
        .pending_selection_view()
        .expect("returned-card result should install TS unsuspend selection");
    assert_eq!(view.kind, SelectionKind::OwnField);
    let unsuspend_action = encode_attack(0, own_ts.index as u16);
    assert!(
        view.valid_action_ids.contains(&unsuspend_action),
        "unsuspend prompt should offer the TS Digimon, got {:?}",
        view.valid_action_ids
    );
    runner
        .execute_action(view.selecting_player, unsuspend_action)
        .expect("choose TS Digimon to unsuspend");
    assert!(
        !runner.game.players[0].battle_area[own_ts.index as usize].is_suspended,
        "the chosen TS Digimon should unsuspend"
    );

    let link_view = runner
        .pending_selection_view()
        .expect("optional link prompt follows the unsuspend");
    assert_eq!(link_view.kind, SelectionKind::OwnField);
    assert!(runner.pending_is_optional());
    runner
        .execute_action(link_view.selecting_player, PASS)
        .expect("decline optional free link");
}

#[test]
fn bt24_091_main_skips_unsuspend_when_nothing_returned_but_still_offers_link() {
    let mut runner = tidal_runner().hand(0, &["BT24-091"]).memory(20).start();
    runner.place_on_field(0, "TS-TAMER", Some(0));
    let own_ts = runner.place_on_field(0, "TS-DIGIMON", Some(0));
    runner.game.players[0].battle_area[own_ts.index as usize].is_suspended = true;
    runner.game.enter_main_phase();

    assert_eq!(play_tidal_standard(&mut runner), OptionPlayResult::Pending);

    let view = runner
        .pending_selection_view()
        .expect("link prompt should still be offered even without returned cards");
    assert_eq!(view.kind, SelectionKind::OwnField);
    assert!(
        runner.game.players[0].battle_area[own_ts.index as usize].is_suspended,
        "without a returned card, the conditional unsuspend step must not run"
    );
    runner
        .execute_action(view.selecting_player, PASS)
        .expect("decline optional free link");
}

#[test]
fn bt24_091_main_accepting_link_attaches_the_option_to_a_ts_digimon() {
    let mut runner = tidal_runner().hand(0, &["BT24-091"]).memory(20).start();
    runner.place_on_field(0, "TS-TAMER", Some(0));
    let host = runner.place_on_field(0, "TS-DIGIMON", Some(0));
    runner.game.enter_main_phase();

    assert_eq!(play_tidal_standard(&mut runner), OptionPlayResult::Pending);
    let view = runner
        .pending_selection_view()
        .expect("optional link prompt");
    let link_action = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&aid| aid != PASS)
        .expect("host action");
    runner
        .execute_action(view.selecting_player, link_action)
        .expect("link Tidal Stream to the TS Digimon");

    let linked = &runner.game.player(0).battle_area[host.index as usize].linked_cards;
    assert_eq!(linked.len(), 1);
    assert_eq!(linked[0].card_id(&runner.game.card_data), "BT24-091");
}

fn tidal_runner() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT24-091 YAML loads")
        .add_card(make_ts_digimon("TS-DIGIMON", 4))
        .add_card(make_tamer("TS-TAMER", &["TS"], CardColor::Red))
        .add_card(make_digimon("OPP-LV3-A", 3))
        .add_card(make_digimon("OPP-LV3-B", 3))
        .add_card(make_digimon("OPP-LV5", 5))
}

fn play_tidal_standard(runner: &mut DebugRunner) -> OptionPlayResult {
    let result = runner.game.play_option_from_hand(0, 0);
    if matches!(
        runner.game.pending_selection.as_ref().map(|s| &s.kind),
        Some(SelectionKind::EffectChoice)
    ) {
        let first = runner
            .game
            .pending_selection
            .as_ref()
            .unwrap()
            .valid_action_ids[0];
        runner
            .game
            .resolve_selection(0, first)
            .expect("choose Standard [Main] mode");
    }
    result
}

fn make_ts_digimon(id: &str, level: u8) -> digimon_engine::CardData {
    let mut card = make_digimon(id, level);
    card.traits = vec!["TS".to_string()];
    card
}

fn make_digimon(id: &str, level: u8) -> digimon_engine::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Red];
    card.level = Some(level);
    card.dp = Some(i32::from(level) * 1000);
    card.play_cost = level as u16;
    card
}

fn make_tamer(id: &str, traits: &[&str], color: CardColor) -> digimon_engine::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Tamer;
    card.colors = vec![color];
    card.level = None;
    card.dp = None;
    card.play_cost = 3;
    card.traits = traits.iter().map(|s| s.to_string()).collect();
    card
}

fn battle_area_contains(runner: &DebugRunner, player: usize, card_id: &str) -> bool {
    runner.game.players[player]
        .battle_area
        .iter()
        .any(|perm| perm.top_card().card_id(&runner.game.card_data) == card_id)
}

fn hand_contains(runner: &DebugRunner, player: usize, card_id: &str) -> bool {
    runner.game.players[player]
        .hand
        .iter()
        .any(|card| card.card_id(&runner.game.card_data) == card_id)
}
