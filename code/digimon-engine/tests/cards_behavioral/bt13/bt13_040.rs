//! BT13-040 Magnamon

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledStep, CompiledZone,
};
use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::replacement::ReplacementCause;

#[test]
fn bt13_040_has_blocker_keyword() {
    let runner = DebugRunner::builder()
        .dsl_card("BT13-040")
        .expect("BT13-040 must load from embedded DSL pack")
        .start();
    let card = runner.compiled_card("BT13-040").expect("compiled card");

    assert!(card.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { .. })
    )));
}

#[test]
fn bt13_040_when_leaving_uses_native_union_zone_for_hand_or_sources() {
    let runner = DebugRunner::builder()
        .dsl_card("BT13-040")
        .expect("BT13-040 must load from embedded DSL pack")
        .start();
    let card = runner.compiled_card("BT13-040").expect("compiled card");

    let replacement = card.effects.iter().find_map(|clause| match clause {
        CompiledClause::Declarative(CompiledDeclarativeClause::Replacement { process, .. }) => {
            Some(process)
        }
        _ => None,
    });
    let process = replacement.expect("BT13-040 replacement process");

    assert!(
        !process
            .iter()
            .any(|step| matches!(step, CompiledStep::RawRust { .. })),
        "BT13-040 must use native DSL for the optional Veemon play, not raw_rust"
    );
    assert!(
        process.iter().any(|step| matches!(
            step,
            CompiledStep::SelectUnionZone { zones, .. }
                if zones.contains(&CompiledZone::Hand) && zones.contains(&CompiledZone::Material)
        )),
        "BT13-040 must select Veemon from hand or source materials with select_union_zone"
    );
    assert!(
        process
            .iter()
            .any(|step| matches!(step, CompiledStep::PlayUnionBoundFree { .. })),
        "BT13-040 must play the bound hand/material card with play_union_bound_free"
    );
}

#[test]
fn bt13_040_when_leaving_draws_and_may_play_veemon_from_hand_or_source() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT13-040")
        .expect("BT13-040 must load from embedded DSL pack")
        .dsl_card("BT12-021")
        .expect("BT12-021 Veemon must load from embedded DSL pack")
        .deck(0, &["BT12-021"])
        .hand(0, &["BT12-021"])
        .start();

    let magnamon = runner.place_stack(0, &["BT12-021", "BT13-040"]);

    runner
        .game
        .delete_permanent_with_cause(magnamon, ReplacementCause::OpponentEffect);

    let pending = runner
        .pending_selection()
        .expect("Magnamon should offer a play-from-hand/source choice after drawing");
    assert!(pending.is_optional, "the Veemon play is optional");
    assert!(
        pending.valid_action_ids.contains(&physical_menu::card_action(&runner.game, "BT12-021", digimon_engine::owned_card_choice::CardChoiceZone::Hand)),
        "Veemon already in hand should be a legal pick"
    );
    let source_action = physical_menu::source_action(&runner.game, magnamon, 0);
    assert!(
        pending.valid_action_ids.contains(&source_action),
        "Veemon in Magnamon's sources should be a legal pick"
    );

    let selecting_player = pending.selecting_player;
    runner
        .execute_action(selecting_player, source_action)
        .expect("play source Veemon");

    assert!(
        runner.pending_selection().is_none(),
        "source choice should resolve the full would-leave observer"
    );
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "BT12-021"),
        "selected source Veemon should be played"
    );
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "BT13-040"),
        "the original leave event should still proceed"
    );
    assert_eq!(
        runner.game.players[0].hand.len(),
        2,
        "draw 1 should happen before the optional play, leaving original hand Veemon plus drawn card"
    );
}

#[test]
fn bt13_040_when_leaving_can_decline_veemon_play_and_still_leaves() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT13-040")
        .expect("BT13-040 must load from embedded DSL pack")
        .dsl_card("BT12-021")
        .expect("BT12-021 Veemon must load from embedded DSL pack")
        .deck(0, &["BT12-021"])
        .hand(0, &["BT12-021"])
        .start();

    let magnamon = runner.place_stack(0, &["BT12-021", "BT13-040"]);

    runner
        .game
        .delete_permanent_with_cause(magnamon, ReplacementCause::OpponentEffect);

    let selecting_player = runner
        .pending_selection()
        .expect("optional Veemon play prompt")
        .selecting_player;
    runner
        .execute_action(selecting_player, PASS)
        .expect("decline optional play");

    assert!(runner.pending_selection().is_none());
    assert!(
        runner.game.players[0].battle_area.is_empty(),
        "declining the play should not cancel the original leave"
    );
    assert_eq!(
        runner.game.players[0].hand.len(),
        2,
        "the draw still resolves even when the optional play is declined"
    );
}

// ─── Candidate legality: only a [Veemon] the free play would accept ─────────
// DCGO BT13_040.cs:41-51 `CanSelectCardCondition`: `CardNames.Contains("Veemon")`
// (the exact name — CR 4.3 2-3-1-2: a name given only in brackets references
// only cards with that name, unlike "with [Veemon] in its name", 2-3-1-3) &&
// `CanPlayAsNewPermanent(cardSource, payCost:false, activateClass)`; the same
// filter gates both the hand and the digivolution-card pick (:94-96). So
// ExVeemon is never offered, and under "can't play Digimon by effects" nothing
// is. A Digi-Egg (no play cost, CR 4.3 7-1-1) can't be offered either, but in
// this YAML that is the NAME filter's doing, not the playability predicate's:
// `can_play_from_effect_without_cost` falls through to true for a Digi-Egg
// (`Game::can_play_card_from_effect_without_cost_for_controller`, `_ => true`).
// No Digi-Egg is named exactly [Veemon] (DemiVeemon is excluded by `name_is`,
// and by the Language Standardization Rules for "in its name"), so the
// predicate's egg gap cannot reach this card.

/// The committable (card id, from-hand?) rows of the pending Veemon pick;
/// empty when no pick installs.
fn offered(runner: &DebugRunner) -> Vec<(String, bool)> {
    if runner.game.pending_selection.is_none() {
        return Vec::new();
    }
    let mut v: Vec<(String, bool)> = physical_menu::commits(&runner.game)
        .into_iter()
        .map(|row| {
            (
                row.card_id.clone().unwrap_or_default(),
                matches!(row.zone, Some(digimon_engine::owned_card_choice::CardChoiceZone::Hand)),
            )
        })
        .collect();
    v.sort();
    v
}

fn veemon_runner(hand: &[&str], stack: &[&str]) -> (DebugRunner, digimon_engine::permanent::PermanentHandle) {
    let mut exveemon = digimon_engine::debug_runner::make_test_card("EXVEEMON", "ExVeemon");
    exveemon.card_kind = digimon_engine::enums::CardKind::Digimon;
    exveemon.level = Some(4);
    exveemon.dp = Some(5000);
    exveemon.play_cost = 5;
    let mut demi = digimon_engine::debug_runner::make_test_card("DEMIVEEMON", "DemiVeemon");
    demi.card_kind = digimon_engine::enums::CardKind::DigiEgg;
    demi.level = Some(2);
    demi.dp = None;
    demi.play_cost = 0;
    let mut runner = DebugRunner::builder()
        .dsl_card("BT13-040")
        .expect("BT13-040 must load from embedded DSL pack")
        .dsl_card("BT12-021")
        .expect("BT12-021 Veemon must load from embedded DSL pack")
        .add_card(exveemon)
        .add_card(demi)
        .add_card(digimon_engine::debug_runner::make_test_card("FILLER", "Filler"))
        .deck(0, &["FILLER"])
        .hand(0, hand)
        .start();
    let magnamon = runner.place_stack(0, stack);
    (runner, magnamon)
}

/// ExVeemon has [Veemon] in its name but is not named [Veemon]: not offered,
/// while the real Veemon in hand is.
#[test]
fn bt13_040_exveemon_in_hand_not_offered() {
    let (mut runner, magnamon) = veemon_runner(&["EXVEEMON", "BT12-021"], &["BT13-040"]);
    runner
        .game
        .delete_permanent_with_cause(magnamon, ReplacementCause::OpponentEffect);
    assert_eq!(offered(&runner), vec![("BT12-021".to_string(), true)]);
}

/// Regression guard: the DemiVeemon Digi-Egg under Magnamon is not offered —
/// it is not named [Veemon], so `name_is` drops it (the playability predicate
/// alone would NOT drop a Digi-Egg); the Veemon source still is offered.
#[test]
fn bt13_040_digi_egg_source_not_offered() {
    let (mut runner, magnamon) =
        veemon_runner(&[], &["DEMIVEEMON", "BT12-021", "BT13-040"]);
    runner
        .game
        .delete_permanent_with_cause(magnamon, ReplacementCause::OpponentEffect);
    assert_eq!(offered(&runner), vec![("BT12-021".to_string(), false)]);
}

/// "You can't play Digimon by effects": neither the hand nor the source
/// Veemon is offered, no pick installs, and Magnamon still leaves.
#[test]
fn bt13_040_veemon_not_offered_under_digimon_play_prohibition() {
    let (mut runner, magnamon) = veemon_runner(&["BT12-021"], &["BT12-021", "BT13-040"]);
    runner.game.modifiers.add_player_modifier(
        0,
        digimon_engine::modifiers::PlayerModifierEntry::simple(
            digimon_engine::enums::ModifierType::CannotPlayDigimonByEffect,
            0,
            digimon_engine::enums::Expiry::Permanent,
            None,
            1,
        ),
    );
    runner
        .game
        .delete_permanent_with_cause(magnamon, ReplacementCause::OpponentEffect);
    assert!(offered(&runner).is_empty(), "got {:?}", offered(&runner));
    let _ = runner.auto_resolve();
    assert!(runner.game.players[0].battle_area.is_empty(), "nothing played; Magnamon left");
    assert_eq!(runner.game.players[0].hand.len(), 2, "the draw still happened");
}

// ─── Digivolve routes: Yellow Lv.3 / 4 circle + "Digivolve: 3 from [Veemon]" ──
// Printed (card image + official bundle `data/card_bundles/BT13-040.md`):
// "Digivolve: 3 from [Veemon]" beside the Yellow Lv.3 / 4 circle. DCGO
// BT13_040.cs:15-20 `AddSelfDigivolutionRequirementStaticEffect(permanentCondition:
// TopCard.CardNames.Contains("Veemon"), digivolutionCost: 3)` — exact name, no
// level or colour gate (CR 4.3 2-3-1-2).

fn digivolve_runner(base: digimon_engine::card_data::CardData) -> (DebugRunner, usize) {
    let base_id = base.card_id.clone();
    let mut runner = DebugRunner::builder()
        .dsl_card("BT13-040")
        .expect("BT13-040 must load from embedded DSL pack")
        .add_card(base)
        .add_card(digimon_engine::debug_runner::make_test_card("FILLER", "Filler"))
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 5])
        .hand(0, &["BT13-040"])
        .memory(10)
        .start();
    runner.set_first_player(0);
    runner.game.turn_count = 1;
    runner.game.current_phase = digimon_engine::enums::GamePhase::Main;
    let slot = runner.place_on_field(0, &base_id, Some(0)).index as usize;
    (runner, slot)
}

fn lv3(id: &str, name: &str, color: digimon_engine::enums::CardColor) -> digimon_engine::card_data::CardData {
    let mut c = digimon_engine::debug_runner::make_test_card(id, name);
    c.card_kind = digimon_engine::enums::CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(3000);
    c.play_cost = 3;
    c.colors = vec![color];
    c
}

fn digivolve_offered(runner: &DebugRunner, slot: usize) -> bool {
    digimon_engine::action::build_action_mask(&runner.game, 0)
        [digimon_engine::action::space::encode_digivolve(0, slot as u16) as usize]
        > 0.5
}

/// "Digivolve: 3 from [Veemon]": a BLUE Lv.3 [Veemon] (no Yellow circle
/// applies) is a legal base, and the digivolution costs 3.
#[test]
fn bt13_040_digivolves_from_veemon_for_3() {
    let (mut runner, slot) =
        digivolve_runner(lv3("BLUE-VEEMON", "Veemon", digimon_engine::enums::CardColor::Blue));
    assert!(digivolve_offered(&runner, slot), "[Veemon] base must offer the digivolve");
    let mem = runner.memory();
    assert!(runner.game.digivolve_from_hand(
        0,
        0,
        slot,
        digimon_engine::enums::PlaySource::ByHand
    ));
    assert_eq!(mem - runner.memory(), 3, "the [Veemon] route costs 3");
    assert_eq!(
        runner.game.players[0].battle_area[slot]
            .top_card()
            .card_id(&runner.game.card_data),
        "BT13-040"
    );
}

/// Regression guard: the printed Yellow Lv.3 circle still costs 4.
#[test]
fn bt13_040_standard_circle_yellow_lv3_costs_4() {
    let (mut runner, slot) =
        digivolve_runner(lv3("YELLOW3", "Yellow Three", digimon_engine::enums::CardColor::Yellow));
    assert!(digivolve_offered(&runner, slot));
    let mem = runner.memory();
    assert!(runner.game.digivolve_from_hand(
        0,
        0,
        slot,
        digimon_engine::enums::PlaySource::ByHand
    ));
    assert_eq!(mem - runner.memory(), 4);
}

/// Regression guard: [Veemon] is the exact name — a blue Lv.3 merely with
/// "Veemon" in its name (no Yellow circle either) is not a legal base.
#[test]
fn bt13_040_veemon_route_needs_the_exact_name() {
    let (runner, slot) =
        digivolve_runner(lv3("BLUE-XVEE", "Veemon Plus", digimon_engine::enums::CardColor::Blue));
    assert!(!digivolve_offered(&runner, slot));
}

#[path = "../../support/owned_card_choice.rs"]
mod physical_menu;
