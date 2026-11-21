//! BT17-094 Ancient Guardian Deity — Option, Red + Blue, cost 2. Ten Warriors.
//!
//! Printed text (data/card_bundles/BT17-094.md, official Bandai DB):
//!
//! ```text
//! While you have a Tamer or a Digimon with the [Hybrid] trait, you may ignore
//! this card's color requirements.
//! [Main] You may return 1 Digimon card with the [Hybrid] or [Ten Warriors]
//!   trait from your trash to the hand. Then, you may play 1 Digimon card with
//!   the [Ten Warriors] trait or 1 Tamer card with inherited effects from your
//!   hand with the play cost reduced by 4.
//! [Security] You may play 1 Tamer card with inherited effects from your hand
//!   or trash without paying the cost. Then, add this card to the hand.
//! ```
//!
//! DCGO: DCGO/Assets/Scripts/CardEffect/BT17/Red/BT17_094.cs. Boards are
//! synthetic (no recorded game data).

#![allow(dead_code, unused_imports)]

use digimon_engine::action::space::{PASS, PLAY_HAND_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{OptionPlayResult, SelectionKind, UnionZoneSet};

fn digimon(id: &str, name: &str, traits: &[&str], cost: u16, color: CardColor) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = cost;
    c.colors = vec![color];
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn tamer(id: &str, name: &str, inherited: bool) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 4;
    c.colors = vec![CardColor::Green];
    if inherited {
        c.inherited_text = "[Security] Play this card without paying the cost.".to_string();
    }
    c
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card("BT17-094")
        .expect("BT17-094 compiles into the embedded pack")
        // A green Tamer: satisfies "you have a Tamer" but not red/blue.
        .add_card(tamer("T-TAMER-ANY", "Any Tamer", false))
        .add_card(tamer("T-TAMER-INH", "Inherited Tamer", true))
        .add_card(digimon("T-GREEN", "Green Digimon", &[], 3, CardColor::Green))
        .add_card(digimon("T-HYBRID", "Hybrid Digimon", &["Hybrid"], 3, CardColor::Green))
        .add_card(digimon("T-TW", "Ten Warriors Digimon", &["Ten Warriors"], 6, CardColor::Red))
        .add_card(digimon("T-HYB-TRASH", "Hybrid Spirit", &["Hybrid"], 5, CardColor::Red))
        .add_card(digimon("T-PLAIN-TRASH", "Plain Digimon", &[], 5, CardColor::Red))
        .add_card(digimon("T-ATK", "Attacker", &[], 4, CardColor::Red))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
}

fn playable(runner: &DebugRunner) -> bool {
    digimon_engine::action::build_action_mask(&runner.game, 0)[PLAY_HAND_START as usize] == 1.0
}

// ─── Structure ───────────────────────────────────────────────────────────────

#[test]
fn bt17_094_compiles_with_use_requirement_main_and_security() {
    let runner = base().memory(10).start();
    let compiled = runner.compiled_card("BT17-094").expect("compiled");
    assert_eq!(compiled.cost, Some(2));
    assert!(
        compiled.use_requirement.is_some(),
        "the colour bypass is the card-level use_requirement"
    );
}

// ─── Colour bypass ───────────────────────────────────────────────────────────

/// A Tamer (any colour) lets the red/blue Option be used.
#[test]
fn bt17_094_colour_requirement_ignored_while_you_have_a_tamer() {
    let mut runner = base().hand(0, &["BT17-094"]).memory(10).start();
    runner.place_on_field(0, "T-TAMER-ANY", Some(0));
    runner.game.enter_main_phase();
    assert!(playable(&runner), "an off-colour Option is usable with a Tamer");
}

/// A Hybrid-trait Digimon does too.
#[test]
fn bt17_094_colour_requirement_ignored_while_you_have_a_hybrid_digimon() {
    let mut runner = base().hand(0, &["BT17-094"]).memory(10).start();
    runner.place_on_field(0, "T-HYBRID", Some(0));
    runner.game.enter_main_phase();
    assert!(playable(&runner), "an off-colour Option is usable with a Hybrid Digimon");
}

/// A non-Hybrid off-colour Digimon is not enough.
#[test]
fn bt17_094_colour_requirement_still_applies_with_a_plain_off_colour_digimon() {
    let mut runner = base().hand(0, &["BT17-094"]).memory(10).start();
    runner.place_on_field(0, "T-GREEN", Some(0));
    runner.game.enter_main_phase();
    assert!(!playable(&runner), "neither a Tamer nor a Hybrid: colours still required");
}

// ─── [Main] ──────────────────────────────────────────────────────────────────

/// Return a Hybrid Digimon from the trash, then play a Ten Warriors Digimon
/// from the hand for its cost minus 4. Only Hybrid / Ten Warriors DIGIMON are
/// offered from the trash.
#[test]
fn bt17_094_main_returns_hybrid_then_plays_ten_warriors_digimon_for_4_less() {
    let mut runner = base()
        .hand(0, &["BT17-094", "T-TW"])
        .memory(10)
        .start();
    runner.place_on_field(0, "T-TAMER-ANY", Some(0));
    runner.inject_trash(0, "T-HYB-TRASH");
    runner.inject_trash(0, "T-PLAIN-TRASH");
    runner.game.enter_main_phase();
    let memory_before = runner.game.memory;
    let result = runner.game.play_option_from_hand(0, 0);
    assert_ne!(result, OptionPlayResult::Invalid, "the Option is playable");
    runner.game.drain_effect_queue();

    // 1) the optional trash pick offers ONLY the Hybrid Digimon.
    let view = runner.pending_selection_view().expect("trash pick parked");
    assert_eq!(view.kind, SelectionKind::Trash);
    let picks: Vec<u16> = view
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != PASS)
        .collect();
    assert_eq!(picks.len(), 1, "only the Hybrid Digimon qualifies: {picks:?}");
    runner.execute_action(view.selecting_player, picks[0]).expect("return it");
    runner.game.drain_effect_queue();
    assert!(
        runner.game.players[0]
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "T-HYB-TRASH"),
        "the Hybrid Digimon returned to the hand"
    );

    // 2) the optional hand pick: the Ten Warriors Digimon (the returned Hybrid
    // Spirit has no Ten Warriors trait and is not a Tamer -> not offered).
    let view = runner.pending_selection_view().expect("hand pick parked");
    assert_eq!(view.kind, SelectionKind::Hand);
    let picks: Vec<u16> = view
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != PASS)
        .collect();
    assert_eq!(picks.len(), 1, "only the Ten Warriors Digimon: {picks:?}");
    runner.execute_action(view.selecting_player, picks[0]).expect("play it");
    runner.auto_resolve().expect("finish");
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "T-TW"),
        "the Ten Warriors Digimon was played"
    );
    // Option cost 2 + (6 - 4) = 4 memory.
    assert_eq!(memory_before - runner.game.memory, 4, "Option 2 + reduced play cost 2");
}

/// The Tamer branch needs inherited effects; a Tamer without any is not offered.
#[test]
fn bt17_094_main_offers_only_tamers_with_inherited_effects() {
    let mut runner = base()
        .hand(0, &["BT17-094", "T-TAMER-ANY", "T-TAMER-INH"])
        .memory(10)
        .start();
    runner.place_on_field(0, "T-GREEN", Some(0));
    runner.place_on_field(0, "T-HYBRID", Some(0));
    runner.game.enter_main_phase();
    let result = runner.game.play_option_from_hand(0, 0);
    assert_ne!(result, OptionPlayResult::Invalid);
    runner.game.drain_effect_queue();
    let view = runner.pending_selection_view().expect("hand pick parked");
    let picks: Vec<u16> = view
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != PASS)
        .collect();
    assert_eq!(picks.len(), 1, "only the inherited-effect Tamer: {picks:?}");
}

// ─── [Security] ──────────────────────────────────────────────────────────────

fn security_runner(hand: &[&str], trash: &[&str]) -> (DebugRunner, PermanentHandle) {
    let mut runner = base()
        .hand(1, hand)
        .security(1, &["BT17-094"])
        .memory(10)
        .start();
    for id in trash {
        runner.inject_trash(1, id);
    }
    let attacker = runner.place_on_field(0, "T-ATK", Some(0));
    (runner, attacker)
}

/// Plays the inherited-effect Tamer from hand free, then ALWAYS adds the Option
/// to the hand.
#[test]
fn bt17_094_security_plays_tamer_then_adds_itself_to_hand() {
    let (mut runner, attacker) = security_runner(&["T-TAMER-ANY", "T-TAMER-INH"], &[]);
    let _ = runner.attack_player(attacker, 1, false);
    let sel = runner.game.pending_selection.as_ref().expect("union pick parked");
    assert!(matches!(sel.kind, SelectionKind::UnionZone { .. }));
    let picks: Vec<u16> = sel
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != PASS)
        .collect();
    assert_eq!(picks.len(), 1, "only the inherited-effect Tamer: {picks:?}");
    runner.execute_action(1, picks[0]).expect("play it");
    runner.auto_resolve().expect("finish");
    assert!(
        runner.game.players[1]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "T-TAMER-INH"),
        "the Tamer was played free"
    );
    assert_eq!(runner.security_count(1), 0, "the Option left security");
    assert!(
        runner.game.players[1]
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "BT17-094"),
        "Then, add this card to the hand"
    );
}

/// "You may": declining the play still adds the Option to the hand.
#[test]
fn bt17_094_security_decline_still_adds_itself_to_hand() {
    let (mut runner, attacker) = security_runner(&["T-TAMER-INH"], &[]);
    let _ = runner.attack_player(attacker, 1, false);
    assert!(matches!(
        runner.game.pending_selection.as_ref().map(|s| &s.kind),
        Some(SelectionKind::UnionZone { .. })
    ));
    runner.execute_action(1, PASS).expect("decline");
    runner.auto_resolve().expect("finish");
    assert!(
        !runner.game.players[1]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "T-TAMER-INH"),
        "declined: nothing played"
    );
    assert!(
        runner.game.players[1]
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "BT17-094"),
        "the Option is added to the hand regardless"
    );
}

/// With no eligible Tamer the Option still goes to the hand, with no prompt.
#[test]
fn bt17_094_security_with_no_eligible_tamer_just_adds_itself_to_hand() {
    let (mut runner, attacker) = security_runner(&["T-TAMER-ANY"], &[]);
    let _ = runner.attack_player(attacker, 1, false);
    runner.auto_resolve().expect("finish");
    assert!(
        runner.game.players[1]
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "BT17-094"),
        "the Option is added to the hand"
    );
}
