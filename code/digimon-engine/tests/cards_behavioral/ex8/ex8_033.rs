//! EX8-033 Pumpkinmon — Digimon, Yellow/Purple, Lv.5, cost 6, 6000 DP.
//! Traits: Puppet, NSo. Attribute: Data.
//!
//! # Card text (official Bandai DB — data/card_bundles/EX8-033.md)
//! [Digivolve] Yellow Lv.4 / Purple Lv.4: Cost 4
//! [Digivolve] Lv.4 w/[NSo] trait: Cost 3
//! [DNA Digivolve] Yellow Lv.4 + Purple/Red Lv.4: Cost 0 (stack the 2 specified
//! Digimon and digivolve unsuspended)
//! [On Play] [When Digivolving] Return 1 card with the [NSo] trait from your
//! trash to the hand.
//! [On Deletion] 1 of your opponent's Digimon gets -4000 DP for the turn.
//! Inherited Effect
//! [On Deletion] <Recovery +1 (Deck)>
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX8/Yellow/EX8_033.cs — alt digivolve (top
//! level 4 + EqualsTraits("NSo"), cost 3), jogress (Yellow L4 + Purple/Red L4,
//! cost 0), [On Play]/[When Digivolving] MANDATORY NSo trash pick -> hand,
//! [On Deletion] MANDATORY opponent pick -4000 until end of turn, inherited
//! [On Deletion] IRecovery(1).

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledScope, CompiledTiming,
};
use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::encode_digivolve;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, GamePhase, PlaySource};
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;

use super::nso_support::*;

const CARD_ID: &str = "EX8-033";

fn builder() -> DebugRunnerBuilder {
    let mut nso_option = nso("NSO-OPTION", 3, 0, 2, CardColor::Purple);
    nso_option.card_kind = CardKind::Option;
    nso_option.level = None;
    nso_option.dp = None;
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-033 YAML loads")
        .add_card(make_test_card("FILL", "Filler"))
        .add_card(nso("N4-YELLOW", 4, 4000, 4, CardColor::Yellow))
        .add_card(nso("N4-RED", 4, 4000, 4, CardColor::Red))
        .add_card(plain("X4-YELLOW", 4, 4000, CardColor::Yellow))
        .add_card(plain("X4-PURPLE", 4, 4000, CardColor::Purple))
        .add_card(plain("X4-BLUE", 4, 4000, CardColor::Blue))
        .add_card(nso("N3-TRASH", 3, 3000, 3, CardColor::Purple))
        .add_card(nso_option)
        .add_card(plain("X-TRASH", 3, 3000, CardColor::Purple))
        .add_card(plain("OPP-A", 4, 6000, CardColor::Blue))
        .add_card(plain("OPP-B", 4, 7000, CardColor::Blue))
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 6])
        .security(0, &["FILL", "FILL"])
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

// ─── Digivolution routes ─────────────────────────────────────────────────────

#[test]
fn ex8_033_standard_circles_yellow_and_purple_level_4_cost_4() {
    for base in ["X4-YELLOW", "X4-PURPLE"] {
        let (mut r, slot) = digivolve_over(base);
        assert!(offered(&r, slot), "{base}");
        let mem = r.memory();
        assert!(r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand));
        assert_eq!(mem - r.memory(), 4, "{base}");
    }
}

#[test]
fn ex8_033_alt_route_level_4_nso_of_any_colour_costs_3() {
    let (mut r, slot) = digivolve_over("N4-RED");
    assert!(offered(&r, slot));
    let mem = r.memory();
    assert!(r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand));
    assert_eq!(mem - r.memory(), 3);
}

#[test]
fn ex8_033_rejects_an_off_colour_level_4_without_the_nso_trait() {
    let (mut r, slot) = digivolve_over("X4-BLUE");
    assert!(!offered(&r, slot));
}

#[test]
fn ex8_033_dna_alt_path_is_yellow_l4_plus_purple_or_red_l4_cost_0_unsuspended() {
    let r = builder().start();
    let card = r.compiled_card(CARD_ID).unwrap();
    let dna = card
        .alt_paths
        .iter()
        .find(|p| p.kind == CompiledAltPathKind::DnaDigivolve)
        .expect("DNA alt-path");
    assert_eq!(dna.materials.len(), 2);
    assert!(dna.stacks_unsuspended, "'digivolve unsuspended'");
    assert_eq!(dna.materials[0].filter.level_eq, Some(4));
    assert_eq!(dna.materials[1].filter.level_eq, Some(4));
    let first = dna.materials[0].filter.color_only.clone().unwrap_or_default();
    let second = dna.materials[1].filter.color_only.clone().unwrap_or_default();
    assert_eq!(first, vec![CompiledColor::Yellow]);
    assert_eq!(second.len(), 2);
    assert!(second.contains(&CompiledColor::Purple) && second.contains(&CompiledColor::Red));
}

// ─── [On Play] / [When Digivolving] ──────────────────────────────────────────

#[test]
fn ex8_033_compiles_the_printed_clauses() {
    let r = builder().start();
    let card = r.compiled_card(CARD_ID).unwrap();
    assert!(card.effects.iter().any(|c| matches!(c,
        CompiledClause::Triggered(t)
            if t.when.contains(&CompiledTiming::OnPlay)
                && t.when.contains(&CompiledTiming::WhenDigivolving))));
    assert!(card.effects.iter().any(|c| matches!(c,
        CompiledClause::Triggered(t)
            if t.scope != CompiledScope::Inherited && t.when == vec![CompiledTiming::OnDeletion])));
    assert!(card.effects.iter().any(|c| matches!(c,
        CompiledClause::Triggered(t)
            if t.scope == CompiledScope::Inherited && t.when == vec![CompiledTiming::OnDeletion])));
}

#[test]
fn ex8_033_on_play_returns_an_nso_card_from_the_trash_to_the_hand() {
    let mut r = builder().start();
    push_trash(&mut r, 0, "N3-TRASH");
    push_trash(&mut r, 0, "NSO-OPTION");
    push_trash(&mut r, 0, "X-TRASH");
    let me = r.place_on_field(0, CARD_ID, Some(0));
    r.fire_on_play(0, me.index as usize);
    let view = r.pending_selection_view().expect("trash pick");
    assert!(!view.is_optional, "'Return 1 card' is mandatory");
    // Both the NSo Digimon and the NSo Option are legal (any card kind), the
    // non-[NSo] card is not.
    let offered: Vec<String> = offered_ids(&r);
    assert!(offered.contains(&"N3-TRASH".to_string()), "{offered:?}");
    assert!(offered.contains(&"NSO-OPTION".to_string()), "{offered:?}");
    assert!(!offered.contains(&"X-TRASH".to_string()), "{offered:?}");
    pick_trash(&mut r, "N3-TRASH");
    drain_declining(&mut r);
    assert_eq!(hand_ids(&r, 0).len(), 1, "exactly one card returned");
    assert!(
        trash_ids(&r, 0).len() == 2,
        "the other two stay in the trash: {:?}",
        trash_ids(&r, 0)
    );
}

#[test]
fn ex8_033_on_play_with_no_nso_card_in_the_trash_does_nothing() {
    let mut r = builder().start();
    push_trash(&mut r, 0, "X-TRASH");
    let me = r.place_on_field(0, CARD_ID, Some(0));
    r.fire_on_play(0, me.index as usize);
    assert!(r.pending_selection_view().is_none());
    assert!(hand_ids(&r, 0).is_empty());
}

#[test]
fn ex8_033_when_digivolving_returns_an_nso_card_from_the_trash() {
    // A red [NSo] base has exactly one route (cost 3): no cost-choice prompt.
    let (mut r, slot) = digivolve_over("N4-RED");
    push_trash(&mut r, 0, "N3-TRASH");
    assert!(r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand));
    let view = r.pending_selection_view().expect("trash pick on [When Digivolving]");
    let _ = view;
    pick_trash(&mut r, "N3-TRASH");
    drain_declining(&mut r);
    assert!(hand_ids(&r, 0).contains(&"N3-TRASH".to_string()));
}

// ─── [On Deletion] ───────────────────────────────────────────────────────────

#[test]
fn ex8_033_on_deletion_gives_an_opponent_digimon_minus_4000() {
    let mut r = builder().start();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(1, "OPP-A", Some(0));
    r.place_on_field(1, "OPP-B", Some(0));
    r.game.delete_permanent_with_cause(me, ReplacementCause::OpponentEffect);
    let view = r.pending_selection_view().expect("the -4000 DP pick");
    assert!(!view.is_optional);
    assert_eq!(view.kind, SelectionKind::OppField);
    pick_field(&mut r, 1, "OPP-B");
    drain_declining(&mut r);
    assert_eq!(r.effective_dp(handle_of(&r, 1, "OPP-B")), Some(3000), "7000 - 4000");
    assert_eq!(r.effective_dp(handle_of(&r, 1, "OPP-A")), Some(6000));
}

#[test]
fn ex8_033_inherited_on_deletion_recovers_one_from_the_deck() {
    let mut r = builder().start();
    let host = r.place_stack(0, &[CARD_ID, "N4-YELLOW"]);
    let security_before = r.game.players[0].security.len();
    let deck_before = r.game.players[0].deck.len();
    r.game.delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);
    drain_declining(&mut r);
    assert_eq!(r.game.players[0].security.len(), security_before + 1, "Recovery +1");
    assert_eq!(r.game.players[0].deck.len(), deck_before - 1);
}
