//! EX8-062 Piedmon — Digimon, Purple/Yellow, Lv.6, cost 7, 12000 DP. Traits:
//! Wizard, NSo. Attribute: Virus. ACE.
//!
//! # Card text (official Bandai DB — data/card_bundles/EX8-062.md)
//! [Digivolve] Purple Lv.5 / Yellow Lv.5: Cost 4
//! [Digivolve] Lv.5 w/[NSo] trait: Cost 3
//! {Hand} [Counter] <Blast Digivolve>
//! [On Play] [When Digivolving] Activate the effect below 4 times:
//!   - 1 of your opponent's Digimon gets -2000 DP for the turn.
//! [All Turns] [Once Per Turn] When other Digimon are deleted, you may play 1
//! level 4 or lower [NSo] trait Digimon card from your trash without paying the
//! cost.
//! <Overflow (-4)>
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX8/Purple/EX8_062.cs — alt digivolve
//! (level 5 + EqualsTraits("NSo"), cost 3), BlastDigivolveEffect, the
//! `for (i < 4)` MANDATORY -2000 DP loop (the same Digimon may be picked
//! repeatedly; an iteration is skipped only when no opponent Digimon exists),
//! and the OPT `canNoSelect` trash play over `CanTriggerOnPermanentDeleted`
//! (a battle-area Digimon that is not this permanent).

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::encode_digivolve;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, GamePhase, PlaySource};
use digimon_engine::replacement::ReplacementCause;

use super::nso_support::*;

const CARD_ID: &str = "EX8-062";

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-062 YAML loads")
        .add_card(make_test_card("FILL", "Filler"))
        .add_card(nso("N5-RED", 5, 5000, 5, CardColor::Red))
        .add_card(plain("X5-PURPLE", 5, 5000, CardColor::Purple))
        .add_card(plain("X5-YELLOW", 5, 5000, CardColor::Yellow))
        .add_card(plain("X5-BLUE", 5, 5000, CardColor::Blue))
        .add_card(nso("N4-TRASH", 4, 4000, 4, CardColor::Purple))
        .add_card(nso("N5-TRASH", 5, 5000, 5, CardColor::Purple))
        .add_card(plain("X4-TRASH", 4, 4000, CardColor::Purple))
        .add_card(plain("OPP-A", 4, 12000, CardColor::Blue))
        .add_card(plain("OPP-B", 4, 12000, CardColor::Blue))
        .add_card(plain("MINE-OTHER", 3, 3000, CardColor::Purple))
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .security(1, &["FILL", "FILL", "FILL"])
        .memory(10)
}

fn digivolve_over(base: &str) -> (DebugRunner, usize) {
    let mut r = builder().hand(0, &[CARD_ID]).start();
    r.game.turn_count = 1;
    r.game.current_phase = GamePhase::Main;
    let slot = r.place_on_field(0, base, Some(0)).index as usize;
    (r, slot)
}

fn offered(r: &DebugRunner, slot: usize) -> bool {
    build_action_mask(&r.game, 0)[encode_digivolve(0, slot as u16) as usize] > 0.5
}

// ─── Structure ───────────────────────────────────────────────────────────────

#[test]
fn ex8_062_is_an_ace_blast_digivolve_card() {
    let r = builder().start();
    let card = r.compiled_card(CARD_ID).unwrap();
    assert_eq!(card.ace_overflow, Some(-4), "<Overflow (-4)>");
    assert!(card.effects.iter().any(|c| matches!(c,
        CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { keyword, .. })
            if keyword == "BlastDigivolve")));
    assert!(card.effects.iter().any(|c| matches!(c,
        CompiledClause::Triggered(t)
            if t.scope != CompiledScope::Inherited
                && t.when.contains(&CompiledTiming::OnPlay)
                && t.when.contains(&CompiledTiming::WhenDigivolving))));
}

// ─── Digivolution routes ─────────────────────────────────────────────────────

#[test]
fn ex8_062_standard_circles_purple_and_yellow_level_5_cost_4() {
    for base in ["X5-PURPLE", "X5-YELLOW"] {
        let (mut r, slot) = digivolve_over(base);
        assert!(offered(&r, slot), "{base}");
        let mem = r.memory();
        assert!(r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand));
        assert_eq!(mem - r.memory(), 4, "{base}");
    }
}

#[test]
fn ex8_062_alt_route_level_5_nso_of_any_colour_costs_3() {
    let (mut r, slot) = digivolve_over("N5-RED");
    assert!(offered(&r, slot));
    let mem = r.memory();
    assert!(r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand));
    assert_eq!(mem - r.memory(), 3);
}

#[test]
fn ex8_062_rejects_an_off_colour_level_5_without_the_nso_trait() {
    let (mut r, slot) = digivolve_over("X5-BLUE");
    assert!(!offered(&r, slot));
}

// ─── [On Play] / [When Digivolving] x4 ───────────────────────────────────────

#[test]
fn ex8_062_on_play_applies_minus_2000_four_times_to_chosen_digimon() {
    let mut r = builder().start();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(1, "OPP-A", Some(0));
    r.place_on_field(1, "OPP-B", Some(0));
    r.fire_on_play(0, me.index as usize);
    // Four mandatory picks; the same Digimon may be chosen repeatedly.
    for target in ["OPP-A", "OPP-A", "OPP-B", "OPP-A"] {
        let view = r.pending_selection_view().expect("a -2000 DP pick");
        assert!(!view.is_optional, "each activation is mandatory");
        pick_field(&mut r, 1, target);
    }
    assert!(r.pending_selection_view().is_none(), "exactly four activations");
    assert_eq!(r.effective_dp(handle_of(&r, 1, "OPP-A")), Some(12000 - 6000));
    assert_eq!(r.effective_dp(handle_of(&r, 1, "OPP-B")), Some(12000 - 2000));
}

#[test]
fn ex8_062_when_digivolving_also_applies_four_activations() {
    let (mut r, slot) = digivolve_over("N5-RED");
    r.place_on_field(1, "OPP-A", Some(0));
    assert!(r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand));
    for _ in 0..4 {
        let _ = r.pending_selection_view().expect("a -2000 DP pick");
        pick_field(&mut r, 1, "OPP-A");
    }
    assert_eq!(r.effective_dp(handle_of(&r, 1, "OPP-A")), Some(12000 - 8000));
}

#[test]
fn ex8_062_activations_stop_when_the_opponent_has_no_digimon() {
    let mut r = builder().start();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    r.fire_on_play(0, me.index as usize);
    assert!(r.pending_selection_view().is_none(), "no opponent Digimon, no picks");
}

// ─── [All Turns][OPT] play from the trash when other Digimon are deleted ──────

fn deletion_board() -> DebugRunner {
    let mut r = builder().start();
    r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(0, "MINE-OTHER", Some(0));
    r.place_on_field(1, "OPP-A", Some(0));
    push_trash(&mut r, 0, "N4-TRASH");
    push_trash(&mut r, 0, "N5-TRASH");
    push_trash(&mut r, 0, "X4-TRASH");
    r
}

#[test]
fn ex8_062_when_another_of_your_digimon_is_deleted_play_a_level_4_nso_from_trash() {
    let mut r = deletion_board();
    let other = handle_of(&r, 0, "MINE-OTHER");
    r.game.delete_permanent_with_cause(other, ReplacementCause::OpponentEffect);
    let view = r.pending_selection_view().expect("trash play prompt");
    assert!(view.is_optional, "'you may play'");
    let offered = offered_ids(&r);
    assert!(offered.contains(&"N4-TRASH".to_string()), "{offered:?}");
    assert!(!offered.contains(&"N5-TRASH".to_string()), "level 5 is too high: {offered:?}");
    assert!(!offered.contains(&"X4-TRASH".to_string()), "no [NSo] trait: {offered:?}");
    let mem = r.memory();
    pick_trash(&mut r, "N4-TRASH");
    drain_declining(&mut r);
    assert!(field_ids(&r, 0).contains(&"N4-TRASH".to_string()), "played from the trash");
    assert_eq!(r.memory(), mem, "without paying the cost");
}

#[test]
fn ex8_062_also_triggers_on_an_opponents_digimon_being_deleted() {
    let mut r = deletion_board();
    let opp = handle_of(&r, 1, "OPP-A");
    r.game.delete_permanent_with_cause(opp, ReplacementCause::OpponentEffect);
    let view = r.pending_selection_view().expect("[All Turns]: any other Digimon's deletion");
    assert!(view.is_optional);
}

#[test]
fn ex8_062_the_play_is_optional() {
    let mut r = deletion_board();
    let other = handle_of(&r, 0, "MINE-OTHER");
    r.game.delete_permanent_with_cause(other, ReplacementCause::OpponentEffect);
    pass(&mut r);
    drain_declining(&mut r);
    assert!(!field_ids(&r, 0).contains(&"N4-TRASH".to_string()));
    assert!(trash_ids(&r, 0).contains(&"N4-TRASH".to_string()));
}

#[test]
fn ex8_062_does_not_trigger_on_its_own_deletion() {
    let mut r = deletion_board();
    let me = handle_of(&r, 0, CARD_ID);
    r.game.delete_permanent_with_cause(me, ReplacementCause::OpponentEffect);
    drain_declining(&mut r);
    assert!(!field_ids(&r, 0).contains(&"N4-TRASH".to_string()), "'other Digimon' only");
}

#[test]
fn ex8_062_is_once_per_turn() {
    let mut r = deletion_board();
    r.place_on_field(0, "MINE-OTHER", Some(0));
    let first = handle_of(&r, 0, "MINE-OTHER");
    r.game.delete_permanent_with_cause(first, ReplacementCause::OpponentEffect);
    pick_trash(&mut r, "N4-TRASH");
    drain_declining(&mut r);
    // A second deletion in the same turn does not offer the play again.
    let opp = handle_of(&r, 1, "OPP-A");
    r.game.delete_permanent_with_cause(opp, ReplacementCause::OpponentEffect);
    assert!(
        r.pending_selection_view().is_none(),
        "[Once Per Turn]: {:?}",
        r.pending_selection_view()
    );
}
