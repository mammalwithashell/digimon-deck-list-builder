//! BT21-064 Guilmon - Digimon, Lv.3, Purple/Red, DP 1000, Cost 3.
//! Traits: [Reptile] / [Hero]. Form: Rookie. Attribute: Virus.
//!
//! # Card text (data/card_bundles/BT21-064.md - verbatim)
//!
//! ```text
//! [Digivolve] [Gigimon]/Lv.2 w/[Hero] trait: Cost 0   (plus the standard
//!   Lv.2 purple / red cost-1 circles)
//! [On Play] By trashing 1 card with [Guilmon], [Growlmon], [Gallantmon] or
//!   [Megidramon] in its name or the [Hero] trait from your hand, <Draw 2>.
//! Inherited: [On Deletion] Gain 1 memory.
//! ```
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/BT21/Purple/BT21_064.cs`
//!   - On Play: `SelectHandEffect` Mode.Discard, `canNoSelect: true`,
//!     candidates = [Hero] trait OR name contains Guilmon / Growlmon /
//!     Gallantmon / Megidramon; Draw 2 only when a card was trashed. No
//!     outer yes/no - the pick itself is the "may".
//!   - Inherited On Deletion: mandatory +1 memory.
//!   - Alt digivolve: name == Gigimon, or Lv.2 [Hero]; cost 0.
//!
//! Regression origin: a recorded PvP game's `SelectHandEffect` row (trash a
//! [Hero] card from hand) met no prompt because the card had no spec. These
//! tests use synthetic boards.

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{CompiledAltPathKind, CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::{encode_digivolve, PASS, PLAY_HAND_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "BT21-064";

fn digimon(id: &str, name: &str, level: u8, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(2000);
    c.play_cost = 2;
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

/// A fixture that deletes another of the controller's Digimon (the deletion
/// that fires the inherited [On Deletion] of a source beneath it).
const DELETER_YAML: &str = r#"
card: DSL-DELETER
name: Deleter
kind: digimon
level: 4
color: [red]
cost: 0
dp: 3000
effects:
  - when: on_play
    summary: "[On Play] Delete 1 of your other Digimon"
    process:
      - select_own_permanent:
          bind_as: victim
          filter: { kind: digimon, other: true }
          prompt: "Delete 1 of your other Digimon"
      - delete_permanent: { target: victim }
"#;

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT21-064 YAML parses and compiles")
        .from_dsl_yaml(DELETER_YAML)
        .expect("deleter fixture compiles")
        .add_card(make_test_card("PAD", "Filler"))
        .add_card(digimon("HERO-CARD", "Some Hero", 4, &["Hero"]))
        .add_card(digimon("NAMED-GUILMON", "Guilmon", 3, &[]))
        .add_card(digimon("NAMED-GROWLMON", "Growlmon (X Antibody)", 4, &[]))
        .add_card(digimon("NAMED-GALLANTMON", "Gallantmon", 6, &[]))
        .add_card(digimon("NAMED-MEGIDRAMON", "Megidramon", 6, &[]))
        .add_card(digimon("OTHER-CARD", "Agumon", 3, &["Reptile"]))
        .add_card(digimon("GIGIMON", "Gigimon", 2, &[]))
        .add_card(digimon("HERO-LV2", "Hero Egg Baby", 2, &["Hero"]))
        .add_card(digimon("PLAIN-LV2", "Plain Lv2", 2, &[]))
        .deck(0, &["PAD"; 12])
        .deck(1, &["PAD"; 12])
}

/// Hand cards offered by the parked `Hand` pick.
fn offered_hand_ids(runner: &DebugRunner) -> Vec<String> {
    let v = runner.pending_selection_view().expect("a hand pick is parked");
    assert_eq!(v.kind, SelectionKind::Hand, "On Play parks a hand pick");
    let ids: Vec<String> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    v.valid_action_ids
        .iter()
        .copied()
        .filter(|&id| id != PASS)
        .map(|id| ids[(id - PLAY_HAND_START) as usize].clone())
        .collect()
}

// ─── Structure + digivolve boxes ──────────────────────────────────────────────

#[test]
fn bt21_064_has_printed_metadata() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.name, "Guilmon");
    assert_eq!(card.level, Some(3));
    assert_eq!(card.dp, Some(1000));
    assert!(card.traits.iter().any(|t| t == "Hero"));
}

#[test]
fn bt21_064_digivolves_from_gigimon_or_lv2_hero_for_cost_0() {
    let mut runner = base().hand(0, &[CARD_ID]).memory(0).start();
    let gigimon = runner.place_on_field(0, "GIGIMON", Some(0));
    let hero = runner.place_on_field(0, "HERO-LV2", Some(0));
    let plain = runner.place_on_field(0, "PLAIN-LV2", Some(0));
    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(mask[encode_digivolve(0, gigimon.index as u16) as usize], 1.0, "[Gigimon] by name");
    assert_eq!(mask[encode_digivolve(0, hero.index as u16) as usize], 1.0, "Lv.2 w/[Hero]");
    assert_eq!(
        mask[encode_digivolve(0, plain.index as u16) as usize],
        0.0,
        "a Lv.2 that is neither [Gigimon] nor [Hero] is not a legal base (no standard circle in the test card data)"
    );
}

// ─── [On Play] ────────────────────────────────────────────────────────────────

#[test]
fn bt21_064_on_play_offers_only_hero_or_named_cards() {
    let mut runner = base()
        .hand(
            0,
            &[
                CARD_ID,
                "OTHER-CARD",
                "HERO-CARD",
                "NAMED-GUILMON",
                "NAMED-GROWLMON",
                "NAMED-GALLANTMON",
                "NAMED-MEGIDRAMON",
            ],
        )
        .memory(10)
        .start();
    runner.play(0, 0).expect("play Guilmon");
    let mut offered = offered_hand_ids(&runner);
    offered.sort();
    assert_eq!(
        offered,
        vec![
            "HERO-CARD".to_string(),
            "NAMED-GALLANTMON".to_string(),
            "NAMED-GROWLMON".to_string(),
            "NAMED-GUILMON".to_string(),
            "NAMED-MEGIDRAMON".to_string(),
        ],
        "[Hero] trait OR Guilmon/Growlmon/Gallantmon/Megidramon in the name; a plain card is not offered"
    );
    assert!(runner.pending_is_optional(), "DCGO canNoSelect: true - the pick is declinable");
}

#[test]
fn bt21_064_trashing_a_hero_card_draws_two() {
    let mut runner = base()
        .hand(0, &[CARD_ID, "HERO-CARD", "OTHER-CARD"])
        .memory(10)
        .start();
    runner.play(0, 0).expect("play Guilmon");
    let trash_before = runner.trash_size(0);
    let deck_before = runner.deck_size(0);
    let hand_before = runner.hand_size(0); // HERO-CARD + OTHER-CARD

    let v = runner.pending_selection_view().expect("hand pick");
    let hero_idx = runner.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "HERO-CARD")
        .unwrap();
    runner
        .execute_action(0, PLAY_HAND_START + hero_idx as u16)
        .expect("trash the [Hero] card");
    runner.auto_resolve().ok();

    assert_eq!(runner.trash_size(0), trash_before + 1, "the card was trashed");
    assert_eq!(runner.deck_size(0), deck_before - 2, "Draw 2");
    assert_eq!(runner.hand_size(0), hand_before - 1 + 2);
}

#[test]
fn bt21_064_declining_the_pick_trashes_nothing_and_draws_nothing() {
    let mut runner = base()
        .hand(0, &[CARD_ID, "HERO-CARD"])
        .memory(10)
        .start();
    runner.play(0, 0).expect("play Guilmon");
    let trash_before = runner.trash_size(0);
    let deck_before = runner.deck_size(0);

    runner.execute_action(0, PASS).expect("decline the optional trash");
    runner.auto_resolve().ok();

    assert_eq!(runner.trash_size(0), trash_before);
    assert_eq!(runner.deck_size(0), deck_before, "no card was trashed -> no draw");
    assert_eq!(runner.hand_size(0), 1);
}

#[test]
fn bt21_064_no_matching_card_means_no_prompt_and_no_draw() {
    let mut runner = base()
        .hand(0, &[CARD_ID, "OTHER-CARD"])
        .memory(10)
        .start();
    let deck_before = runner.deck_size(0);
    runner.play(0, 0).expect("play Guilmon");
    runner.auto_resolve().ok();
    assert!(runner.pending_selection().is_none());
    assert_eq!(runner.deck_size(0), deck_before, "nothing to trash -> no draw");
    assert_eq!(runner.hand_size(0), 1);
}

// ─── Inherited [On Deletion] ──────────────────────────────────────────────────

#[test]
fn bt21_064_inherited_on_deletion_gains_one_memory() {
    let mut runner = base()
        .add_card(digimon("CARRIER", "Carrier", 4, &[]))
        .hand(0, &["DSL-DELETER"])
        .memory(5)
        .start();
    // Guilmon as a digivolution source of CARRIER; the deleter deletes CARRIER.
    let stack = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    let memory_before = runner.memory();
    runner.play(0, 0).expect("play the deleter (costs 0)");
    runner.auto_resolve().ok();

    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .all(|p| p.top_card().card_id(&runner.game.card_data) != "CARRIER"),
        "the carrier was deleted"
    );
    assert_eq!(
        runner.memory(),
        memory_before + 1,
        "the inherited [On Deletion] gains 1 memory"
    );
}
