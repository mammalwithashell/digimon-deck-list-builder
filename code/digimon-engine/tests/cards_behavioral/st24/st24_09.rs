//! ST24-09 Sunflowmon — Digimon, Lv.4, Green, DP 5000, Cost 4.
//! Traits: Vegetation, DATA SQUAD.
//!
//! # Card text (card image / DCGO — authoritative; cards.json omits "or Tamers")
//! [On Play] [When Digivolving] You may suspend 1 of your opponent's Digimon or
//!   Tamers. Then, you may place the top card of your deck face down under any of
//!   your [DATA SQUAD] trait Tamers.
//! Inherited [All Turns]: This Digimon gets +1000 DP.
//!
//! Alt-digivolve (image/DCGO; cards.json xros_req string is stale):
//!   [Digivolve] Lv.3 w/[DATA SQUAD] trait: Cost 2.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/ST24/Green/ST24_09.cs
//!
//! # Patterns
//! - optional suspend of an opponent Digimon-OR-Tamer
//! - INDEPENDENT optional place deck-top under a [DATA SQUAD] Tamer (continue_on_decline)
//! - inherited [All Turns] +1000 DP

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

const CARD_ID: &str = "ST24-09";

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn ds_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![CardColor::Green];
    c.traits = vec!["DATA SQUAD".to_string()];
    c
}

fn plain_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![CardColor::Green];
    c
}

fn opp_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Red];
    c.level = Some(4);
    c.dp = Some(5000);
    c.play_cost = 5;
    c
}

fn opp_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![CardColor::Red];
    c
}

fn filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Green];
    c.level = Some(3);
    c.dp = Some(2000);
    c.play_cost = 3;
    c
}

fn base() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST24-09 in embedded DSL pack")
        .add_card(ds_tamer("DS-TAMER"))
        .add_card(plain_tamer("PLAIN-TAMER"))
        .add_card(opp_digimon("OPP-DIGI"))
        .add_card(opp_tamer("OPP-TAMER"))
        .add_card(filler("FILLER"))
        .deck(0, &["FILLER"; 12])
        .deck(1, &["FILLER"; 12])
        .memory(8)
        .start()
}

fn fire(r: &mut DebugRunner, timing: EffectTiming, handle: PermanentHandle) {
    r.game
        .enqueue_triggered(timing, TriggerSource::Permanent(handle));
    r.game.drain_effect_queue();
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn st24_09_metadata_and_alt_path() {
    let runner = base();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.name, "Sunflowmon");
    assert_eq!(card.level, Some(4));
    assert_eq!(card.cost, Some(4));
    let ds = card.alt_paths.iter().any(|p| {
        p.from
            .as_ref()
            .map(|f| f.level_eq == Some(3) && f.trait_has.as_deref() == Some("DATA SQUAD"))
            .unwrap_or(false)
    });
    assert!(ds, "Lv.3 [DATA SQUAD] alt-path present");

    let has_inherited_dp = card.effects.iter().any(|c| {
        matches!(c, CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
            scope, dp_modifier: Some(1000), ..
        }) if *scope == CompiledScope::Inherited)
    });
    assert!(has_inherited_dp, "inherited +1000 DP aura present");
}

// ─── Section 2 — Behavioral: suspend + place ─────────────────────────────────

/// POSITIVE: suspend the opponent's Digimon, then place the deck-top face-down
/// under the chosen DATA SQUAD Tamer.
#[test]
fn st24_09_suspends_opp_digimon_then_places_deck_top() {
    let mut r = base();
    r.set_first_player(0);
    let tamer = r.place_on_field(0, "DS-TAMER", Some(0));
    let opp = r.place_on_field(1, "OPP-DIGI", Some(0));
    let carrier = r.place_on_field(0, CARD_ID, Some(0));

    let deck_before = r.deck_size(0);
    let tamer_sources_before = r.game.players[0].battle_area[tamer.index as usize]
        .card_sources
        .len();

    fire(&mut r, EffectTiming::OnPlay, carrier);

    // Prompt 1: suspend the opponent Digimon (or Tamer).
    let v = r
        .pending_selection_view()
        .expect("suspend pick installs");
    assert!(v.is_optional, "'you may suspend' ⇒ optional");
    r.execute_action(0, v.valid_action_ids[0]).unwrap();

    // Prompt 2: choose the DATA SQUAD Tamer to place the deck-top under.
    let v2 = r
        .pending_selection_view()
        .expect("DATA SQUAD Tamer pick installs after the suspend");
    assert!(v2.is_optional, "'you may place' ⇒ optional");
    r.execute_action(0, v2.valid_action_ids[0]).unwrap();
    let _ = r.auto_resolve();

    assert!(
        r.game.players[1].battle_area[opp.index as usize].is_suspended,
        "the opponent's Digimon is suspended"
    );
    assert_eq!(
        r.game.players[0].battle_area[tamer.index as usize]
            .card_sources
            .len(),
        tamer_sources_before + 1,
        "the deck-top was placed under the DATA SQUAD Tamer"
    );
    assert!(
        r.game.players[0].battle_area[tamer.index as usize].card_sources[0].face_down,
        "the placed source is face-down"
    );
    assert_eq!(r.deck_size(0), deck_before - 1, "deck-top consumed by the place");
}

/// A Tamer is a legal suspend target too ("Digimon or Tamers").
#[test]
fn st24_09_can_suspend_opponent_tamer() {
    let mut r = base();
    r.set_first_player(0);
    r.place_on_field(0, "DS-TAMER", Some(0));
    let opp_t = r.place_on_field(1, "OPP-TAMER", Some(0));
    let carrier = r.place_on_field(0, CARD_ID, Some(0));

    fire(&mut r, EffectTiming::OnPlay, carrier);
    let v = r
        .pending_selection_view()
        .expect("suspend pick installs (an opponent Tamer is a legal target)");
    assert_eq!(v.valid_action_ids.len(), 1, "the opponent Tamer is a legal suspend target");
    r.execute_action(0, v.valid_action_ids[0]).unwrap();
    let _ = r.auto_resolve();

    assert!(
        r.game.players[1].battle_area[opp_t.index as usize].is_suspended,
        "the opponent's Tamer is suspended"
    );
}

/// DECLINE the suspend but the place leg still runs (independent).
#[test]
fn st24_09_declining_suspend_still_offers_place() {
    let mut r = base();
    r.set_first_player(0);
    let tamer = r.place_on_field(0, "DS-TAMER", Some(0));
    let opp = r.place_on_field(1, "OPP-DIGI", Some(0));
    let carrier = r.place_on_field(0, CARD_ID, Some(0));
    let tamer_sources_before = r.game.players[0].battle_area[tamer.index as usize]
        .card_sources
        .len();

    fire(&mut r, EffectTiming::OnPlay, carrier);
    let _v = r.pending_selection_view().expect("suspend pick installs");
    r.execute_action(0, digimon_engine::action::space::PASS)
        .expect("decline the suspend");
    let v2 = r
        .pending_selection_view()
        .expect("place pick installs even after declining the suspend");
    r.execute_action(0, v2.valid_action_ids[0]).unwrap();
    let _ = r.auto_resolve();

    assert!(
        !r.game.players[1].battle_area[opp.index as usize].is_suspended,
        "declined suspend ⇒ opponent Digimon stays unsuspended"
    );
    assert_eq!(
        r.game.players[0].battle_area[tamer.index as usize]
            .card_sources
            .len(),
        tamer_sources_before + 1,
        "the place still ran independently of the declined suspend"
    );
}

/// GATE: no DATA SQUAD Tamer ⇒ only the suspend is offered; the place is gated out.
#[test]
fn st24_09_no_ds_tamer_only_suspend() {
    let mut r = base();
    r.set_first_player(0);
    r.place_on_field(0, "PLAIN-TAMER", Some(0));
    r.place_on_field(1, "OPP-DIGI", Some(0));
    let carrier = r.place_on_field(0, CARD_ID, Some(0));
    let deck_before = r.deck_size(0);

    fire(&mut r, EffectTiming::OnPlay, carrier);
    let v = r.pending_selection_view().expect("suspend pick installs");
    r.execute_action(0, v.valid_action_ids[0]).unwrap();
    let _ = r.auto_resolve();

    assert!(
        r.game.pending_selection.is_none(),
        "no DATA SQUAD Tamer ⇒ no second (place) prompt"
    );
    assert_eq!(r.deck_size(0), deck_before, "no place ⇒ deck unchanged");
}
