//! BT9-047 Pomumon — Digimon, Lv.3, Green, DP 2000, Cost 3.
//! Traits: Vegetation. Form: Rookie. Attribute: Data.
//! Digivolve: green Lv.2, cost 0.
//!
//! # Card text (official Bandai DB — data/card_bundles/BT9-047.md; card image agrees)
//!
//! ```text
//! [All Turns] Players can't play Digimon by effects.
//! ```
//!
//! No inherited effect (blank box on the card image), no security effect.
//!
//! # DCGO C# reference
//! None: DCGO ships no `BT9_047.cs` (no file under `CardEffect/BT9/Green/`).
//! The card is the BT9-033 Pillomon / BT14-009 Gotsumon text reprint; those
//! shipping YAMLs + tests are the exemplars.
//!
//! # Official Q&A (data/card_qa.json)
//! - Q1844: moving a Digimon out of the breeding area is NOT playing it, so it
//!   stays legal under Pomumon.
//! - Q1845: effects that only reduce play costs still work (no play happens).
//! - Q1846: Option-card effects are effects — they can't play Digimon either.
//! - Q5206: players can't play Digimon INTO THE BREEDING AREA by effects.
//!
//! # Rules
//! - §3-4-7-4: effects on cards in the breeding area can't activate, so a
//!   Pomumon sitting in the breeding area does not lock anything.
//!
//! # Patterns this test covers
//! - D6 player-scoped flood gate `CannotPlayDigimonByEffect`, `target_player: any`
//! - integrated install through the real hand play (`runner.play`)
//! - source discrimination: effect plays blocked, hand plays / breeding moves legal
//! - lifetime: gate absent before Pomumon is in play, gone after it leaves

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledPlayerRef,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, CostDelta, ModifierType, PlaySource};

const CARD_ID: &str = "BT9-047";

fn digimon(id: &str, level: u8) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Green];
    card.level = Some(level);
    card.dp = Some(3000);
    card.play_cost = 3;
    card
}

/// P0 holds Pomumon + an effect-play target; P1 holds an effect-play target.
fn runner() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT9-047 in embedded DSL pack")
        .add_card(digimon("DIG-A", 3))
        .add_card(digimon("DIG-B", 3))
        .add_card(digimon("EGG-HATCH", 3))
        .add_card(digimon("FILLER", 3))
        .hand(0, &[CARD_ID, "DIG-A"])
        .hand(1, &["DIG-B"])
        .deck(0, &["FILLER"; 10])
        .deck(1, &["FILLER"; 10])
        .memory(10)
        .start();
    r.set_first_player(0);
    r
}

fn hand_idx(r: &DebugRunner, player: u8, id: &str) -> usize {
    r.game.players[player as usize]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == id)
        .unwrap_or_else(|| panic!("{id} not in P{player} hand"))
}

/// Play Pomumon from P0's hand through the real play action.
fn play_pomumon(r: &mut DebugRunner) -> usize {
    let idx = hand_idx(r, 0, CARD_ID);
    let field = r.play(0, idx).expect("Pomumon hand play resolves");
    let _ = r.auto_resolve();
    r.game.tick_declarative_effects();
    field
}

fn gate(r: &DebugRunner, player: u8) -> bool {
    r.game
        .modifiers
        .player_has(player, ModifierType::CannotPlayDigimonByEffect)
}

// ─── SECTION 1 — Structural ─────────────────────────────────────────────────

#[test]
fn bt9_047_metadata_and_digivolve_path_match_printed_card() {
    let r = runner();
    let card = r.compiled_card(CARD_ID).expect("BT9-047 compiled");
    assert_eq!(card.name, "Pomumon");
    assert_eq!(card.kind, CompiledCardKind::Digimon);
    assert_eq!(card.level, Some(3));
    assert_eq!(card.cost, Some(3));
    assert_eq!(card.dp, Some(2000));
    assert_eq!(card.traits, vec!["Vegetation"]);
    assert_eq!(card.attribute, ["Data"]);
    assert_eq!(card.alt_paths.len(), 1, "green Lv.2 digivolve circle");
    assert_eq!(card.alt_paths[0].cost, Some(CompiledCost::Literal(0)));
}

#[test]
fn bt9_047_compiles_one_any_player_cannot_play_by_effect_flood_gate_only() {
    let r = runner();
    let card = r.compiled_card(CARD_ID).expect("BT9-047 compiled");
    assert_eq!(card.effects.len(), 1, "one printed clause, no inherited/security");
    let (modifier, target) = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Declarative(CompiledDeclarativeClause::FloodGate {
                modifier,
                target_player,
                ..
            }) => Some((modifier.clone(), *target_player)),
            _ => None,
        })
        .expect("BT9-047 must compile a flood_gate");
    assert_eq!(modifier, "CannotPlayDigimonByEffect");
    assert_eq!(target, Some(CompiledPlayerRef::Any), "printed 'Players' = both");
}

// ─── SECTION 2 — Gate lifetime (positive / negative) ───────────────────────

#[test]
fn bt9_047_hand_play_installs_gate_on_both_players() {
    let mut r = runner();
    play_pomumon(&mut r);
    assert!(gate(&r, 0), "Pomumon restricts its controller");
    assert!(gate(&r, 1), "Pomumon restricts the opponent");
}

#[test]
fn bt9_047_no_gate_before_pomumon_is_in_play() {
    let mut r = runner();
    r.game.tick_declarative_effects();
    assert!(!gate(&r, 0));
    assert!(!gate(&r, 1));
    let played = r.game.play_from_hand_with_cost(
        1,
        hand_idx(&r, 1, "DIG-B"),
        CostDelta::Free,
        PlaySource::ByEffect,
    );
    assert!(played.is_some(), "effect plays are legal without Pomumon");
}

#[test]
fn bt9_047_gate_lifts_once_pomumon_leaves_the_battle_area() {
    let mut r = runner();
    let field = play_pomumon(&mut r);
    assert!(gate(&r, 1));
    let handle = r.perm_handle(0, field);
    r.game.delete_permanent_with_effects(handle);
    let _ = r.auto_resolve();
    r.game.tick_declarative_effects();
    assert!(!gate(&r, 0), "gate must not outlive Pomumon");
    assert!(!gate(&r, 1), "gate must not outlive Pomumon");
    let played = r.game.play_from_hand_with_cost(
        1,
        hand_idx(&r, 1, "DIG-B"),
        CostDelta::Free,
        PlaySource::ByEffect,
    );
    assert!(played.is_some(), "effect plays are legal again");
}

/// §3-4-7-4: Pomumon in the breeding area has no active effect.
#[test]
fn bt9_047_in_breeding_area_does_not_lock_effect_plays() {
    let mut r = runner();
    r.place_in_breeding(0, CARD_ID);
    r.game.tick_declarative_effects();
    assert!(!gate(&r, 0));
    assert!(!gate(&r, 1));
}

// ─── SECTION 3 — Behavioral ────────────────────────────────────────────────

#[test]
fn bt9_047_blocks_both_players_effect_plays() {
    let mut r = runner();
    play_pomumon(&mut r);

    let p0 = r.game.play_from_hand_with_cost(
        0,
        hand_idx(&r, 0, "DIG-A"),
        CostDelta::Free,
        PlaySource::ByEffect,
    );
    assert!(p0.is_none(), "P0 effect-initiated Digimon play must be blocked");
    assert_eq!(r.hand_size(0), 1, "DIG-A stays in hand");

    let p1 = r.game.play_from_hand_with_cost(
        1,
        hand_idx(&r, 1, "DIG-B"),
        CostDelta::Free,
        PlaySource::ByEffect,
    );
    assert!(p1.is_none(), "P1 effect-initiated Digimon play must be blocked");
    assert_eq!(r.hand_size(1), 1, "DIG-B stays in hand");
}

#[test]
fn bt9_047_does_not_block_normal_hand_play() {
    let mut r = runner();
    play_pomumon(&mut r);
    let idx = hand_idx(&r, 0, "DIG-A");
    let field = r.play(0, idx);
    assert!(field.is_some(), "a hand play is not 'by effects'");
    assert_eq!(r.hand_size(0), 0);
}

/// Q1844: moving out of the breeding area is not playing.
#[test]
fn bt9_047_breeding_move_still_legal() {
    let mut r = runner();
    play_pomumon(&mut r);
    r.place_in_breeding(0, "EGG-HATCH");
    let before = r.battle_area_size(0);
    assert!(r.move_from_breeding(0), "breeding move is not a play");
    assert_eq!(r.battle_area_size(0), before + 1);
}

/// Q5206: effects can't play Digimon into the breeding area either.
#[test]
fn bt9_047_blocks_effect_play_into_breeding_area() {
    let mut r = runner();
    assert!(
        r.game
            .can_play_to_breeding_from_hand(1, hand_idx(&r, 1, "DIG-B"), 1),
        "precondition: legal without Pomumon"
    );
    play_pomumon(&mut r);
    assert!(
        !r.game
            .can_play_to_breeding_from_hand(1, hand_idx(&r, 1, "DIG-B"), 1),
        "Pomumon blocks effect plays into the breeding area"
    );
}
