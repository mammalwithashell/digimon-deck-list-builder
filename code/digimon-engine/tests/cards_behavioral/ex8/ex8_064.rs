//! EX8-064 Boltboutamon — Digimon, Purple/Black/Yellow, Lv.7, cost 15, 15000
//! DP. Traits: Wizard, NSo. Attribute: Virus.
//!
//! # Card text (official Bandai DB — data/card_bundles/EX8-064.md)
//! [Digivolve] Purple/Black/Yellow Lv.6: Cost 5
//! [DNA Digivolve] Purple/Black Lv.6 + Yellow/Green Lv.6: Cost 0
//! [DNA Digivolve] [Piedmon] + [Myotismon]: Cost 0
//! [When Digivolving] <De-Digivolve 3> 1 of your opponent's Digimon and, for the
//! turn, all of their Digimon get -6000 DP. Then, if DNA digivolving, you may
//! play up to 10 play cost's total worth of [NSo] trait Digimon cards from your
//! trash without paying the cost.
//! [All Turns] [Once Per Turn] When other Digimon are deleted, trash your
//! opponent's top security card.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX8/Purple/EX8_064.cs — see the YAML header.

use digimon_dsl::compiled::{CompiledAltPathKind, CompiledClause, CompiledColor, CompiledTiming};
use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::encode_digivolve;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, EffectTiming, GamePhase, PlaySource};
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::TriggerSource;

use super::nso_support::*;

const CARD_ID: &str = "EX8-064";

fn builder() -> DebugRunnerBuilder {
    let mut piedmon = nso("PIEDMON", 6, 12000, 7, CardColor::Purple);
    piedmon.card_name = "Piedmon".to_string();
    let mut myotismon = nso("MYOTISMON", 5, 7000, 7, CardColor::Purple);
    myotismon.card_name = "Myotismon".to_string();
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-064 YAML loads")
        .add_card(make_test_card("FILL", "Filler"))
        .add_card(plain("X6-PURPLE", 6, 6000, CardColor::Purple))
        .add_card(plain("X6-BLACK", 6, 6000, CardColor::Black))
        .add_card(plain("X6-YELLOW", 6, 6000, CardColor::Yellow))
        .add_card(plain("X6-BLUE", 6, 6000, CardColor::Blue))
        .add_card(piedmon)
        .add_card(myotismon)
        .add_card(nso("N7-CHEAP", 4, 4000, 4, CardColor::Purple))
        .add_card(nso("N6-CHEAP", 3, 3000, 6, CardColor::Purple))
        .add_card(nso("N10-COST", 5, 5000, 10, CardColor::Purple))
        .add_card(nso("N5-COST", 3, 3000, 5, CardColor::Purple))
        .add_card(plain("X-TRASH", 3, 3000, CardColor::Purple))
        .add_card(plain("OPP-A", 4, 9000, CardColor::Blue))
        .add_card(plain("OPP-B", 4, 8000, CardColor::Blue))
        .add_card(plain("OPP-STACK-1", 3, 8000, CardColor::Blue))
        // <De-Digivolve N> never trashes a level 3 top card (CR 4.3 16-12-4), so the
        // cards stacked above the bottom one are level 4 to be removable.
        .add_card(plain("OPP-STACK-2", 4, 3000, CardColor::Blue))
        .add_card(plain("OPP-STACK-3", 4, 3000, CardColor::Blue))
        .add_card(plain("MINE-OTHER", 3, 3000, CardColor::Purple))
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .security(1, &["FILL", "FILL", "FILL"])
        .memory(20)
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

/// Enqueue [When Digivolving] for the Boltboutamon on P0's field, as a DNA
/// digivolution or a standard one.
fn fire_when_digivolving(r: &mut DebugRunner, me: digimon_engine::permanent::PermanentHandle, dna: bool) {
    let card = r.game.players[0].battle_area[me.index as usize].top_card().handle();
    r.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Digivolved {
            player: 0,
            permanent: me,
            card,
            effect_initiated: false,
            dna_origin: dna,
        },
    );
    r.game.drain_effect_queue();
}

// ─── Digivolution routes ─────────────────────────────────────────────────────

#[test]
fn ex8_064_standard_circles_purple_black_yellow_level_6_cost_5() {
    for base in ["X6-PURPLE", "X6-BLACK", "X6-YELLOW"] {
        let (mut r, slot) = digivolve_over(base);
        assert!(offered(&r, slot), "{base}");
        let mem = r.memory();
        assert!(r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand));
        assert_eq!(mem - r.memory(), 5, "{base}");
    }
}

#[test]
fn ex8_064_rejects_a_blue_level_6() {
    let (r, slot) = digivolve_over("X6-BLUE");
    assert!(!offered(&r, slot));
}

#[test]
fn ex8_064_has_the_two_printed_dna_recipes() {
    let r = builder().start();
    let card = r.compiled_card(CARD_ID).unwrap();
    let dnas: Vec<_> = card
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::DnaDigivolve)
        .collect();
    assert_eq!(dnas.len(), 2, "colour recipe + [Piedmon]/[Myotismon] recipe");
    assert!(dnas.iter().all(|d| d.stacks_unsuspended));
    let colours = dnas
        .iter()
        .find(|d| d.materials[0].filter.color_only.is_some())
        .expect("colour recipe");
    assert_eq!(colours.materials[0].filter.level_eq, Some(6));
    assert_eq!(colours.materials[1].filter.level_eq, Some(6));
    let first = colours.materials[0].filter.color_only.clone().unwrap();
    let second = colours.materials[1].filter.color_only.clone().unwrap();
    assert!(first.contains(&CompiledColor::Purple) && first.contains(&CompiledColor::Black));
    assert!(second.contains(&CompiledColor::Yellow) && second.contains(&CompiledColor::Green));
    let names = dnas
        .iter()
        .find(|d| d.materials[0].filter.name_is.is_some())
        .expect("name recipe");
    assert_eq!(names.materials[0].filter.name_is.as_deref(), Some("Piedmon"));
    assert_eq!(names.materials[1].filter.name_is.as_deref(), Some("Myotismon"));
}

#[test]
fn ex8_064_compiles_when_digivolving_and_all_turns_clauses() {
    let r = builder().start();
    let card = r.compiled_card(CARD_ID).unwrap();
    assert!(card.effects.iter().any(|c| matches!(c,
        CompiledClause::Triggered(t) if t.when == vec![CompiledTiming::WhenDigivolving])));
    assert!(card.effects.iter().any(|c| matches!(c,
        CompiledClause::Triggered(t) if t.once_per_turn && t.when == vec![CompiledTiming::OnAnyDeletion])));
}

// ─── [When Digivolving] ──────────────────────────────────────────────────────

fn de_digivolve_board() -> (DebugRunner, digimon_engine::permanent::PermanentHandle) {
    let mut r = builder().start();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    r.place_stack(1, &["OPP-STACK-1", "OPP-STACK-2", "OPP-STACK-3", "OPP-A"]);
    r.place_on_field(1, "OPP-B", Some(0));
    (r, me)
}

#[test]
fn ex8_064_when_digivolving_de_digivolves_3_and_shrinks_every_opponent_digimon() {
    let (mut r, me) = de_digivolve_board();
    fire_when_digivolving(&mut r, me, false);
    let view = r.pending_selection_view().expect("de-digivolve pick");
    assert!(!view.is_optional, "the De-Digivolve target is mandatory");
    pick_field(&mut r, 1, "OPP-A");
    declare_max(&mut r);
    drain_declining(&mut r);
    // OPP-A lost 3 digivolution cards: only its (former) bottom card remains.
    let opp_a_slot = field_ids(&r, 1)
        .iter()
        .position(|s| s == "OPP-STACK-1")
        .unwrap_or_else(|| panic!("de-digivolved to the bottom card: field={:?}", field_ids(&r, 1)));
    let perm = &r.game.players[1].battle_area[opp_a_slot];
    assert_eq!(perm.card_sources.len(), 1, "De-Digivolve 3 removed the three top cards");
    // -6000 for the turn on ALL opponent Digimon.
    let b = handle_of(&r, 1, "OPP-B");
    assert_eq!(r.effective_dp(b), Some(8000 - 6000));
    let a = handle_of(&r, 1, "OPP-STACK-1");
    assert_eq!(r.effective_dp(a), Some(8000 - 6000), "the de-digivolved Digimon is shrunk too");
}

#[test]
fn ex8_064_the_free_nso_plays_require_dna_digivolving() {
    let (mut r, me) = de_digivolve_board();
    push_trash(&mut r, 0, "N7-CHEAP");
    fire_when_digivolving(&mut r, me, false);
    pick_field(&mut r, 1, "OPP-A");
    declare_max(&mut r);
    drain_declining(&mut r);
    assert!(!field_ids(&r, 0).contains(&"N7-CHEAP".to_string()), "no DNA, no plays");
}

#[test]
fn ex8_064_dna_digivolving_plays_up_to_10_total_play_cost_of_nso_from_the_trash() {
    let (mut r, me) = de_digivolve_board();
    push_trash(&mut r, 0, "N7-CHEAP"); // cost 4
    push_trash(&mut r, 0, "N6-CHEAP"); // cost 6
    push_trash(&mut r, 0, "N10-COST"); // cost 10
    push_trash(&mut r, 0, "N5-COST"); // cost 5
    push_trash(&mut r, 0, "X-TRASH"); // not [NSo]
    fire_when_digivolving(&mut r, me, true);
    pick_field(&mut r, 1, "OPP-A");
    declare_max(&mut r);
    // The multi-pick: 10 total play cost, [NSo] Digimon only.
    let offered = offered_ids(&r);
    assert!(!offered.contains(&"X-TRASH".to_string()), "{offered:?}");
    assert!(offered.contains(&"N10-COST".to_string()), "{offered:?}");
    let mem = r.memory();
    pick_trash(&mut r, "N7-CHEAP"); // 4
    let offered = offered_ids(&r);
    assert!(offered.contains(&"N6-CHEAP".to_string()), "4 + 6 = 10 still fits: {offered:?}");
    assert!(offered.contains(&"N5-COST".to_string()), "4 + 5 = 9 still fits: {offered:?}");
    assert!(!offered.contains(&"N10-COST".to_string()), "4 + 10 > 10: {offered:?}");
    pick_trash(&mut r, "N6-CHEAP"); // 4 + 6 = 10: the budget is spent
    drain_declining(&mut r);
    let field = field_ids(&r, 0);
    assert!(field.contains(&"N7-CHEAP".to_string()) && field.contains(&"N6-CHEAP".to_string()));
    assert!(!field.contains(&"N5-COST".to_string()), "the budget was exhausted");
    assert!(trash_ids(&r, 0).contains(&"N5-COST".to_string()));
    assert_eq!(r.memory(), mem, "played without paying the cost");
}

#[test]
fn ex8_064_the_free_plays_are_optional() {
    let (mut r, me) = de_digivolve_board();
    push_trash(&mut r, 0, "N7-CHEAP");
    fire_when_digivolving(&mut r, me, true);
    pick_field(&mut r, 1, "OPP-A");
    declare_max(&mut r);
    pass(&mut r);
    drain_declining(&mut r);
    assert!(!field_ids(&r, 0).contains(&"N7-CHEAP".to_string()));
}

// ─── [All Turns][OPT] trash the opponent's top security card ──────────────────

fn security_board() -> DebugRunner {
    let mut r = builder().start();
    r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(0, "MINE-OTHER", Some(0));
    r.place_on_field(1, "OPP-A", Some(0));
    r
}

#[test]
fn ex8_064_when_another_digimon_is_deleted_trash_the_opponents_top_security_card() {
    let mut r = security_board();
    let before = r.game.players[1].security.len();
    let top_ci = r.game.players[1].security.last().unwrap().card_index;
    let other = handle_of(&r, 0, "MINE-OTHER");
    r.game.delete_permanent_with_cause(other, ReplacementCause::OpponentEffect);
    drain_declining(&mut r);
    assert_eq!(r.game.players[1].security.len(), before - 1);
    assert!(
        r.game.players[1].trash.iter().any(|c| c.card_index == top_ci),
        "the TOP security card was trashed"
    );
}

#[test]
fn ex8_064_triggers_on_an_opponents_digimon_deletion_too_but_only_once_per_turn() {
    let mut r = security_board();
    r.place_on_field(1, "OPP-B", Some(0));
    let before = r.game.players[1].security.len();
    let a = handle_of(&r, 1, "OPP-A");
    r.game.delete_permanent_with_cause(a, ReplacementCause::OpponentEffect);
    drain_declining(&mut r);
    assert_eq!(r.game.players[1].security.len(), before - 1);
    let b = handle_of(&r, 1, "OPP-B");
    r.game.delete_permanent_with_cause(b, ReplacementCause::OpponentEffect);
    drain_declining(&mut r);
    assert_eq!(r.game.players[1].security.len(), before - 1, "[Once Per Turn]");
}

#[test]
fn ex8_064_does_not_trigger_on_its_own_deletion() {
    let mut r = security_board();
    let before = r.game.players[1].security.len();
    let me = handle_of(&r, 0, CARD_ID);
    r.game.delete_permanent_with_cause(me, ReplacementCause::OpponentEffect);
    drain_declining(&mut r);
    assert_eq!(r.game.players[1].security.len(), before, "'other Digimon' only");
}
