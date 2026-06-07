//! BT25-093 Ignition Flare — Option, Red, Cost 5, [TS].
//!
//! # Card text (cards.json)
//! <Use Req. ([TS] trait)> (Specified cards let you ignore color requirements.)
//! [Security] Activate this card's [Main] effects.
//! [Main] Delete all of your opponent's Digimon with the lowest DP. If this
//!   effect didn't delete, trash 1 of your opponent's Option cards in the
//!   battle area. Then, you may link this card to 1 of your Digimon on the
//!   field without paying the cost.
//! Inherited: [When Attacking] [Once Per Turn] Delete 1 of your opponent's
//!   Digimon with as much DP as this Digimon or less.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Red/BT25_093.cs
//!
//! # Patterns this test covers
//! - D3 color ignore / bypass (Use Req. flood gate)
//! - F: delete lowest-DP (aggregate) + conditional fallback branch
//! - C1/C3-adjacent: link_to_own_digimon
//! - [Security] activates [Main]
//! - Inherited link-ESS (When Attacking delete) — blocked by G-LINK-INHERITED-ESS

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep,
    CompiledTiming,
};
use digimon_engine::action::space::encode_attack;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::selection::{OptionPlayResult, SelectionKind};

const YAML: &str = include_str!("../../../cards/bt25/BT25-093.yaml");

// ── Section 1: structural ────────────────────────────────────────────────

#[test]
fn bt25_093_structure_use_req_main_security_when_attacking_and_link_requirement() {
    let runner = flare_runner().start();
    let compiled = runner
        .compiled_card("BT25-093")
        .expect("BT25-093 compiled card present");

    assert_eq!(compiled.card, "BT25-093");
    assert_eq!(compiled.kind, CompiledCardKind::Option);
    assert_eq!(compiled.cost, Some(5));
    assert!(compiled.use_requirement.is_some());

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
    assert!(main.process.iter().any(|s| matches!(
        s,
        CompiledStep::LinkToOwnDigimon { optional: true, free: true, .. }
    )));

    // [Security] inherited activation.
    assert!(compiled.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Triggered(t)
            if t.scope == CompiledScope::Inherited && t.when == vec![CompiledTiming::OnSecurity]
    )));

    // Inherited [When Attacking] [OPT].
    assert!(compiled.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Triggered(t)
            if t.scope == CompiledScope::Inherited
                && t.when == vec![CompiledTiming::WhenAttacking]
                && t.once_per_turn
    )));

    assert!(compiled.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Declarative(CompiledDeclarativeClause::LinkRequirement { cost: 3, .. })
    )));
}

// ── Section 2: [Main] delete lowest-DP path ──────────────────────────────

#[test]
fn bt25_093_main_deletes_all_lowest_dp_opponent_digimon() {
    let mut runner = flare_runner()
        .hand(0, &["BT25-093"])
        .add_card(make_ts_tamer("TS-TAMER"))
        .memory(20)
        .start();
    runner.place_on_field(0, "TS-TAMER", Some(0));
    // Two Lv3 (3000 DP, lowest, two copies) + one Lv5 (5000 DP).
    let low_a = runner.place_on_field(1, "OPP-LV3", Some(1));
    let low_b = runner.place_on_field(1, "OPP-LV3", Some(1));
    let high = runner.place_on_field(1, "OPP-LV5", Some(1));
    runner.game.enter_main_phase();

    let field_before = runner.battle_area_size(1);
    assert_eq!(play_flare_standard(&mut runner), OptionPlayResult::Pending);
    let _ = runner.auto_resolve();

    // Both lowest-DP (3000) Digimon deleted; the 5000 survives.
    assert_eq!(
        runner.battle_area_size(1),
        field_before - 2,
        "both lowest-DP Digimon are deleted"
    );
    let _ = (low_a, low_b, high);
}

#[test]
fn bt25_093_main_did_not_delete_falls_back_to_trashing_opponent_option() {
    // No opponent Digimon at all → the delete clause deletes nothing, so the
    // fallback "trash 1 opponent Option in the battle area" branch installs.
    let mut runner = flare_runner()
        .hand(0, &["BT25-093"])
        .add_card(make_ts_tamer("TS-TAMER"))
        .add_card(make_option_permanent("OPP-OPT"))
        .memory(20)
        .start();
    runner.place_on_field(0, "TS-TAMER", Some(0));
    let opt = runner.place_on_field(1, "OPP-OPT", Some(1));
    runner.game.enter_main_phase();

    assert_eq!(play_flare_standard(&mut runner), OptionPlayResult::Pending);

    // Fallback branch: select 1 opponent battle-area Option to trash.
    let view = runner
        .pending_selection_view()
        .expect("fallback opponent-Option select installs");
    assert_eq!(view.kind, SelectionKind::OppField);
    let trash_before = runner.trash_size(1);
    runner
        .execute_action(view.selecting_player, encode_attack(0, opt.index as u16))
        .expect("trash opponent Option");
    let _ = runner.auto_resolve();
    assert_eq!(
        runner.trash_size(1),
        trash_before + 1,
        "opponent Option moved to trash"
    );
}

#[test]
fn bt25_093_no_fallback_when_deletion_happened() {
    // A lowest-DP Digimon AND an opponent Option are present. Because the
    // deletion fires, the fallback Option-trash branch must NOT install.
    let mut runner = flare_runner()
        .hand(0, &["BT25-093"])
        .add_card(make_ts_tamer("TS-TAMER"))
        .add_card(make_option_permanent("OPP-OPT"))
        .memory(20)
        .start();
    runner.place_on_field(0, "TS-TAMER", Some(0));
    runner.place_on_field(1, "OPP-LV3", Some(1));
    let opt = runner.place_on_field(1, "OPP-OPT", Some(1));
    runner.game.enter_main_phase();

    let opt_before = runner.game.player(1).battle_area[opt.index as usize]
        .top_card()
        .card_id(&runner.game.card_data)
        .to_string();
    assert_eq!(play_flare_standard(&mut runner), OptionPlayResult::Pending);
    let _ = runner.auto_resolve();

    // The opponent Option survives (no fallback trash); only the Lv3 was deleted.
    assert!(
        runner
            .game
            .player(1)
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == opt_before),
        "opponent Option must survive when a deletion happened"
    );
}

// ── fixtures ─────────────────────────────────────────────────────────────

fn flare_runner() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT25-093 YAML loads")
        .add_card(make_digimon("OPP-LV3", 3))
        .add_card(make_digimon("OPP-LV5", 5))
}

fn play_flare_standard(runner: &mut digimon_engine::debug_runner::DebugRunner) -> OptionPlayResult {
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

fn make_digimon(id: &str, level: u8) -> digimon_engine::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Red];
    card.level = Some(level);
    card.dp = Some(i32::from(level) * 1000);
    card.play_cost = level as u16;
    card
}

fn make_ts_tamer(id: &str) -> digimon_engine::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Tamer;
    card.colors = vec![CardColor::Red];
    card.level = None;
    card.dp = None;
    card.play_cost = 3;
    card.traits = vec!["TS".to_string()];
    card
}

fn make_option_permanent(id: &str) -> digimon_engine::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Option;
    card.colors = vec![CardColor::Red];
    card.level = None;
    card.dp = None;
    card.play_cost = 2;
    card
}
