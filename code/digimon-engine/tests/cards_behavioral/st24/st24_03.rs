//! ST24-03 Gaogamon — Digimon, Lv.4, Blue, DP 5000, Cost 4.
//! Traits: Beast, DATA SQUAD.
//!
//! # Card text (data/cards.json — verbatim)
//! [On Play] [When Digivolving] You may return 1 of your opponent's level 3
//!   Digimon to the hand. Then, you may place the top card of your deck face down
//!   under any of your [DATA SQUAD] trait Tamers.
//! Inherited [When Attacking] [Once Per Turn] If your hand has 7 or fewer cards,
//!   ＜Draw 1＞.
//!
//! Printed digivolve box (image/DCGO — OMITTED from cards.json effect text):
//!   [Digivolve] Lv.3 w/[DATA SQUAD] trait: Cost 2.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/ST24/Blue/ST24_03.cs
//!
//! # Patterns
//! - optional bounce of an opponent Lv.3 Digimon (level_eq 3 filter)
//! - INDEPENDENT optional place deck-top face-down under a selected DATA SQUAD Tamer
//! - inherited [When Attacking][OPT] hand-size-gated <Draw 1>

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

const CARD_ID: &str = "ST24-03";

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn ds_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![CardColor::Blue];
    c.traits = vec!["DATA SQUAD".to_string()];
    c
}

fn plain_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![CardColor::Blue];
    c
}

fn opp_lv3(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Red];
    c.level = Some(3);
    c.dp = Some(3000);
    c.play_cost = 3;
    c
}

fn opp_lv4(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Red];
    c.level = Some(4);
    c.dp = Some(5000);
    c.play_cost = 5;
    c
}

fn carrier_lv4(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Blue];
    c.level = Some(4);
    c.dp = Some(5000);
    c.play_cost = 4;
    c.traits = vec!["DATA SQUAD".to_string()];
    c
}

fn filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Blue];
    c.level = Some(3);
    c.dp = Some(2000);
    c.play_cost = 3;
    c
}

fn push_to_hand(runner: &mut DebugRunner, p: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card_id {card_id}"));
    let next_idx = runner.game.next_card_index();
    runner.game.players[p as usize]
        .hand
        .push(CardSource::new(data_idx, p, next_idx));
}

fn clear_hand(runner: &mut DebugRunner, p: u8) {
    runner.game.players[p as usize].hand.clear();
}

fn base() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST24-03 in embedded DSL pack")
        .add_card(ds_tamer("DS-TAMER"))
        .add_card(plain_tamer("PLAIN-TAMER"))
        .add_card(opp_lv3("OPP-LV3"))
        .add_card(opp_lv4("OPP-LV4"))
        .add_card(carrier_lv4("CARRIER"))
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
fn st24_03_metadata_and_alt_path() {
    let runner = base();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.name, "Gaogamon");
    assert_eq!(card.level, Some(4));
    assert_eq!(card.cost, Some(4));
    let ds = card.alt_paths.iter().any(|p| {
        p.from
            .as_ref()
            .and_then(|f| f.trait_has.as_deref())
            .map(|t| t == "DATA SQUAD")
            .unwrap_or(false)
    });
    assert!(ds, "Lv.3 [DATA SQUAD] alt-path present");
}

// ─── Section 2 — On Play/WD bounce + place ───────────────────────────────────

/// POSITIVE: bounce the opponent's only Lv.3 Digimon, then place the deck-top
/// face-down under the chosen DATA SQUAD Tamer.
#[test]
fn st24_03_bounces_lv3_then_places_deck_top_under_ds_tamer() {
    let mut r = base();
    r.set_first_player(0);
    let tamer = r.place_on_field(0, "DS-TAMER", Some(0));
    let victim = r.place_on_field(1, "OPP-LV3", Some(0));
    let carrier = r.place_on_field(0, CARD_ID, Some(0));

    let opp_hand_before = r.hand_size(1);
    let deck_before = r.deck_size(0);
    let tamer_sources_before = r.game.players[0].battle_area[tamer.index as usize]
        .card_sources
        .len();
    let opp_field_before = r.game.players[1].battle_area.len();

    fire(&mut r, EffectTiming::OnPlay, carrier);

    // Prompt 1: bounce the opponent Lv.3 Digimon.
    let v = r
        .pending_selection_view()
        .expect("bounce pick for the opponent's Lv.3 Digimon installs");
    assert!(v.is_optional, "'you may return' ⇒ optional");
    assert_eq!(v.valid_action_ids.len(), 1, "only the Lv.3 is a legal bounce");
    r.execute_action(0, v.valid_action_ids[0]).unwrap();

    // Prompt 2: choose the DATA SQUAD Tamer to place the deck-top under.
    let v2 = r
        .pending_selection_view()
        .expect("DATA SQUAD Tamer pick installs after the bounce");
    assert!(v2.is_optional, "'you may place' ⇒ optional");
    r.execute_action(0, v2.valid_action_ids[0]).unwrap();
    let _ = r.auto_resolve();

    let _ = victim;
    assert_eq!(
        r.game.players[1].battle_area.len(),
        opp_field_before - 1,
        "the opponent's Lv.3 Digimon left the field (bounced)"
    );
    assert_eq!(r.hand_size(1), opp_hand_before + 1, "the bounced card went to the opponent's hand");
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

/// FILTER: a Lv.4 opponent Digimon is not a legal bounce target; with NO Lv.3,
/// the bounce prompt does not surface but the place half still runs.
#[test]
fn st24_03_no_lv3_skips_bounce_but_place_still_offered() {
    let mut r = base();
    r.set_first_player(0);
    let tamer = r.place_on_field(0, "DS-TAMER", Some(0));
    r.place_on_field(1, "OPP-LV4", Some(0)); // not Lv.3
    let carrier = r.place_on_field(0, CARD_ID, Some(0));
    let tamer_sources_before = r.game.players[0].battle_area[tamer.index as usize]
        .card_sources
        .len();

    fire(&mut r, EffectTiming::OnPlay, carrier);
    // The place half runs independently — the Tamer pick surfaces.
    let v = r
        .pending_selection_view()
        .expect("place-under-tamer pick still offered (independent of the bounce)");
    r.execute_action(0, v.valid_action_ids[0]).unwrap();
    let _ = r.auto_resolve();

    assert_eq!(
        r.game.players[0].battle_area[tamer.index as usize]
            .card_sources
            .len(),
        tamer_sources_before + 1,
        "the deck-top was placed even though there was no Lv.3 to bounce"
    );
}

/// DECLINE both halves: PASS through the bounce and the place leaves all zones
/// untouched.
#[test]
fn st24_03_declining_both_halves_is_clean_noop() {
    let mut r = base();
    r.set_first_player(0);
    let tamer = r.place_on_field(0, "DS-TAMER", Some(0));
    r.place_on_field(1, "OPP-LV3", Some(0));
    let carrier = r.place_on_field(0, CARD_ID, Some(0));

    let opp_field_before = r.game.players[1].battle_area.len();
    let deck_before = r.deck_size(0);
    let tamer_sources_before = r.game.players[0].battle_area[tamer.index as usize]
        .card_sources
        .len();

    fire(&mut r, EffectTiming::OnPlay, carrier);
    let _v = r.pending_selection_view().expect("bounce pick installs");
    r.execute_action(0, digimon_engine::action::space::PASS)
        .expect("decline the bounce");
    let _v2 = r
        .pending_selection_view()
        .expect("place pick installs even after declining the bounce");
    r.execute_action(0, digimon_engine::action::space::PASS)
        .expect("decline the place");
    let _ = r.auto_resolve();

    assert_eq!(
        r.game.players[1].battle_area.len(),
        opp_field_before,
        "declined bounce ⇒ opponent field unchanged"
    );
    assert_eq!(
        r.game.players[0].battle_area[tamer.index as usize]
            .card_sources
            .len(),
        tamer_sources_before,
        "declined place ⇒ nothing under the Tamer"
    );
    assert_eq!(r.deck_size(0), deck_before, "declined place ⇒ deck unchanged");
}

/// GATE: no DATA SQUAD Tamer ⇒ only the bounce is offered; the place half is
/// gated out.
#[test]
fn st24_03_no_ds_tamer_only_bounce() {
    let mut r = base();
    r.set_first_player(0);
    r.place_on_field(0, "PLAIN-TAMER", Some(0));
    r.place_on_field(1, "OPP-LV3", Some(0));
    let carrier = r.place_on_field(0, CARD_ID, Some(0));
    let deck_before = r.deck_size(0);

    fire(&mut r, EffectTiming::OnPlay, carrier);
    let v = r.pending_selection_view().expect("bounce pick installs");
    r.execute_action(0, v.valid_action_ids[0]).unwrap();
    let _ = r.auto_resolve();

    assert!(
        r.game.pending_selection.is_none(),
        "no DATA SQUAD Tamer ⇒ no second prompt"
    );
    assert_eq!(r.deck_size(0), deck_before, "no place ⇒ deck unchanged");
}

// ─── Section 3 — Inherited conditional draw ──────────────────────────────────

#[test]
fn st24_03_inherited_draw_when_hand_seven_or_fewer() {
    let mut r = base();
    r.set_first_player(0);
    let carrier = r.place_stack(0, &[CARD_ID, "FILLER"]);
    clear_hand(&mut r, 0);
    for _ in 0..3 {
        push_to_hand(&mut r, 0, "FILLER");
    }
    let hand_before = r.hand_size(0);
    let deck_before = r.deck_size(0);

    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    let _ = r.auto_resolve();

    assert_eq!(r.hand_size(0), hand_before + 1, "hand ≤7 ⇒ <Draw 1>");
    assert_eq!(r.deck_size(0), deck_before - 1, "drew 1");
}

#[test]
fn st24_03_inherited_draw_not_when_hand_over_seven() {
    let mut r = base();
    r.set_first_player(0);
    let carrier = r.place_stack(0, &[CARD_ID, "FILLER"]);
    clear_hand(&mut r, 0);
    for _ in 0..8 {
        push_to_hand(&mut r, 0, "FILLER");
    }
    let hand_before = r.hand_size(0);
    let deck_before = r.deck_size(0);

    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    let _ = r.auto_resolve();

    assert_eq!(r.hand_size(0), hand_before, "hand >7 ⇒ no draw");
    assert_eq!(r.deck_size(0), deck_before, "hand >7 ⇒ no draw");
}
