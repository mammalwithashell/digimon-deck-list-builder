//! ST24-04 Agumon — Digimon, Lv.3, Yellow/Red, DP (face) , Cost 0.
//! Traits: Dinosaur, DATA SQUAD. Attribute: Vaccine.
//!
//! # cards.json TRAP
//! `data/cards.json` files ST24-04's `effect_description_eng` as the ST24-05
//! GeoGreymon text ("If you have 1 or fewer Tamers, play a Tamer…"). That is a
//! DUPLICATION bug. The REAL effect (card image + DCGO ST24_04.cs) is a
//! [On Play] [When Moving] reveal-3 effect. Authored from DCGO + image.
//!
//! # Real card text (card image / DCGO — authoritative)
//! [On Play] [When Moving] Reveal the top 3 cards of your deck. Among them, add
//!   1 [DATA SQUAD] trait card to the hand and place 1 such card face down under
//!   any of your [DATA SQUAD] trait Tamers. Return the rest to the bottom of the
//!   deck.
//! Inherited [Your Turn]: This Digimon gets +2000 DP.
//! Alt-digivolve: from Koromon (cost 0); from Lv.2 [DATA SQUAD] (cost 0).
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/ST24/Yellow/ST24_04.cs
//!
//! # Patterns — sister to ST23-06 (reveal-and-stash, trait-swapped)

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

use crate::dsl_card_data::{card_data_from_compiled, compiled};

const CARD_ID: &str = "ST24-04";

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn agumon() -> CardData {
    card_data_from_compiled(CARD_ID)
}

/// A [DATA SQUAD] Digimon card (legal for both reveal buckets).
fn ds_card(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 5;
    c.traits = vec!["DATA SQUAD".to_string()];
    c
}

/// A non-[DATA SQUAD] card (illegal for both buckets; goes to deck bottom).
fn plain_card(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 5;
    c.traits = vec!["Beast".to_string()];
    c
}

fn ds_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![CardColor::Yellow];
    c.traits = vec!["DATA SQUAD".to_string()];
    c
}

fn plain_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![CardColor::Yellow];
    c.traits = vec!["Beast".to_string()];
    c
}

fn filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c.level = Some(3);
    c.dp = Some(2000);
    c.play_cost = 3;
    c
}

fn fire_on_play(r: &mut DebugRunner, handle: PermanentHandle) {
    r.fire_on_play(0, handle.index as usize);
}

fn process_contains_place_under_tamer(steps: &[CompiledStep]) -> bool {
    steps.iter().any(|s| match s {
        CompiledStep::PlaceSelectedCardUnderTamer { .. } => true,
        CompiledStep::If {
            then, else_branch, ..
        } => {
            process_contains_place_under_tamer(then)
                || process_contains_place_under_tamer(else_branch)
        }
        CompiledStep::ForEach { body, .. } | CompiledStep::PerSelected { body, .. } => {
            process_contains_place_under_tamer(body)
        }
        _ => false,
    })
}

// ════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn st24_04_is_agumon_lv3_data_squad_with_alt_paths() {
    let card = compiled(CARD_ID);
    assert_eq!(card.name, "Agumon");
    assert_eq!(card.kind, CompiledCardKind::Digimon);
    assert_eq!(card.level, Some(3));
    assert!(card.traits.iter().any(|t| t == "DATA SQUAD"));
    // Two alt-paths: Koromon (name) and Lv.2 [DATA SQUAD].
    let has_koromon = card.alt_paths.iter().any(|p| {
        p.from
            .as_ref()
            .and_then(|f| f.name_contains.as_deref())
            .map(|n| n.contains("Koromon"))
            .unwrap_or(false)
    });
    let has_lv2_ds = card.alt_paths.iter().any(|p| {
        p.from
            .as_ref()
            .map(|f| {
                f.level_eq == Some(2)
                    && f.trait_has.as_deref() == Some("DATA SQUAD")
            })
            .unwrap_or(false)
    });
    assert!(has_koromon, "alt-path from Koromon present");
    assert!(has_lv2_ds, "alt-path from Lv.2 [DATA SQUAD] present");
}

#[test]
fn st24_04_has_onplay_onmove_reveal_and_inherited_dp() {
    let card = compiled(CARD_ID);
    let has_reveal_clause = card.effects.iter().any(|c| {
        matches!(c, CompiledClause::Triggered(t)
            if t.when.contains(&CompiledTiming::OnPlay)
            && t.when.contains(&CompiledTiming::OnMove)
            && process_contains_place_under_tamer(&t.process))
    });
    assert!(
        has_reveal_clause,
        "[On Play][When Moving] reveal clause that places under a Tamer"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Section 2 — Behavioral: reveal-3 → add 1 + place 1 under DATA SQUAD Tamer
// ════════════════════════════════════════════════════════════════════════════

fn setup(deck_top: &[&str], with_ds_tamer: bool, with_plain_tamer: bool) -> (DebugRunner, PermanentHandle) {
    let mut builder = DebugRunner::builder()
        .add_card(agumon())
        .add_card(ds_card("DS_A"))
        .add_card(ds_card("DS_B"))
        .add_card(plain_card("PLAIN"))
        .add_card(ds_tamer("DST"))
        .add_card(plain_tamer("PT"))
        .add_card(filler("FILLER"));

    let mut deck: Vec<&str> = Vec::new();
    deck.extend(std::iter::repeat("FILLER").take(4)); // deck bottom padding
    for &c in deck_top.iter().rev() {
        deck.push(c);
    }
    builder = builder.deck(0, &deck).deck(1, &["FILLER"; 6]).memory(5);
    let mut runner = builder.start();
    runner.set_first_player(0);

    if with_ds_tamer {
        runner.place_on_field(0, "DST", Some(0));
    }
    if with_plain_tamer {
        runner.place_on_field(0, "PT", Some(0));
    }

    let agu = runner.place_on_field(0, CARD_ID, Some(0));
    (runner, agu)
}

fn tamer_handle(runner: &DebugRunner, id: &str) -> PermanentHandle {
    runner.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&runner.game.card_data) == id)
        .map(|i| PermanentHandle { player: 0, index: i as u8 })
        .unwrap_or_else(|| panic!("{id} on field"))
}

/// POSITIVE: reveal DS_A, DS_B, PLAIN with a [DATA SQUAD] Tamer present: add 1
/// DS to hand, place 1 DS face-down under the Tamer, PLAIN to deck bottom.
#[test]
fn st24_04_adds_one_and_places_one_under_ds_tamer() {
    let (mut runner, agu) = setup(&["DS_A", "DS_B", "PLAIN"], true, false);
    let tamer = tamer_handle(&runner, "DST");
    let hand_before = runner.hand_size(0);
    let tamer_sources_before = runner.game.players[0].battle_area[tamer.index as usize]
        .card_sources
        .len();

    fire_on_play(&mut runner, agu);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.hand_size(0),
        hand_before + 1,
        "exactly 1 [DATA SQUAD] card added to hand"
    );
    assert_eq!(
        runner.game.players[0].battle_area[tamer.index as usize]
            .card_sources
            .len(),
        tamer_sources_before + 1,
        "1 card placed under the [DATA SQUAD] Tamer"
    );
    assert!(
        runner.game.players[0].battle_area[tamer.index as usize].card_sources[0].face_down,
        "the placed card is face-down"
    );
    let placed_id = runner.game.players[0].battle_area[tamer.index as usize].card_sources[0]
        .card_id(&runner.game.card_data);
    assert!(
        placed_id == "DS_A" || placed_id == "DS_B",
        "the placed card is a revealed [DATA SQUAD] card (got {placed_id})"
    );
    assert!(runner.game.pending_selection.is_none(), "no stuck selection");
}

/// NEGATIVE (no DS Tamer): only the add-to-hand bucket runs.
#[test]
fn st24_04_only_adds_to_hand_when_no_ds_tamer() {
    let (mut runner, agu) = setup(&["DS_A", "DS_B", "PLAIN"], false, true);
    let pt = tamer_handle(&runner, "PT");
    let hand_before = runner.hand_size(0);
    let pt_sources_before = runner.game.players[0].battle_area[pt.index as usize]
        .card_sources
        .len();

    fire_on_play(&mut runner, agu);
    let _ = runner.auto_resolve();

    assert_eq!(runner.hand_size(0), hand_before + 1, "1 [DATA SQUAD] added to hand");
    assert_eq!(
        runner.game.players[0].battle_area[pt.index as usize]
            .card_sources
            .len(),
        pt_sources_before,
        "no card placed under a non-[DATA SQUAD] Tamer"
    );
    assert!(runner.game.pending_selection.is_none(), "no stuck selection");
}

/// NEGATIVE (no [DATA SQUAD] revealed): nothing added or placed; all to bottom.
#[test]
fn st24_04_no_data_squad_revealed_all_to_deck_bottom() {
    let (mut runner, agu) = setup(&["PLAIN", "PLAIN", "PLAIN"], true, false);
    let tamer = tamer_handle(&runner, "DST");
    let hand_before = runner.hand_size(0);
    let tamer_sources_before = runner.game.players[0].battle_area[tamer.index as usize]
        .card_sources
        .len();

    fire_on_play(&mut runner, agu);
    let _ = runner.auto_resolve();

    assert_eq!(runner.hand_size(0), hand_before, "no [DATA SQUAD] to add");
    assert_eq!(
        runner.game.players[0].battle_area[tamer.index as usize]
            .card_sources
            .len(),
        tamer_sources_before,
        "no [DATA SQUAD] to place"
    );
    assert!(runner.game.pending_selection.is_none());
}

/// Single [DATA SQUAD] revealed: it goes to hand (add bucket priority); nothing
/// left for the place bucket (no_duplicate_cards).
#[test]
fn st24_04_single_data_squad_revealed_goes_to_hand_only() {
    let (mut runner, agu) = setup(&["DS_A", "PLAIN", "PLAIN"], true, false);
    let tamer = tamer_handle(&runner, "DST");
    let hand_before = runner.hand_size(0);
    let tamer_sources_before = runner.game.players[0].battle_area[tamer.index as usize]
        .card_sources
        .len();

    fire_on_play(&mut runner, agu);
    let _ = runner.auto_resolve();

    assert_eq!(runner.hand_size(0), hand_before + 1, "single [DATA SQUAD] added to hand");
    assert_eq!(
        runner.game.players[0].battle_area[tamer.index as usize]
            .card_sources
            .len(),
        tamer_sources_before,
        "no second [DATA SQUAD] to place"
    );
    assert!(runner.game.pending_selection.is_none());
}
