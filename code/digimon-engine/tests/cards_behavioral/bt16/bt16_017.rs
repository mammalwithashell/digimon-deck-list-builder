//! BT16-017 Veemon — Digimon, Lv.3, Blue/Red, DP 2000, Cost 3.
//! Traits: Mini Dragon. Attribute: Free.
//! Digivolve: green or purple Lv.2 / cost 1 (one split circle); [DemiVeemon]: cost 0.
//!
//! # Card text (official Bandai DB, data/card_bundles/BT16-017.md)
//!
//! [Your Turn][Once Per Turn] When one of your other Digimon is played or
//! digivolves, if it has the [Free] trait or is green, gain 1 memory.
//! Inherited: [Your Turn] This Digimon gets +2000 DP.
//! Official Q&A: the check reads the Digimon after it digivolves.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT16/Blue/BT16_017.cs — OnEnterFieldAnyone,
//! maxCount 1, IsOwnerTurn; the played/digivolved permanent is ours, not this
//! one, a Digimon, and its top card is green or has [Free] → AddMemory(1).
//!
//! # Patterns
//! - F-observer: on_ally_played + on_digivolve with event-target gates, OPT
//! - D4 inherited [Your Turn] DP aura
//! - alt digivolve by name ([DemiVeemon])

use digimon_dsl::compiled::{CompiledClause, CompiledDeclarativeClause, CompiledScope};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, PlaySource};

const CARD_ID: &str = "BT16-017";

fn digimon(id: &str, color: CardColor, level: u8, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![color];
    c.level = Some(level);
    c.dp = Some(3000);
    c.play_cost = 3;
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

/// A Lv.4 that digivolves from a red Lv.3 for 2.
fn lv4(id: &str, color: CardColor, traits: &[&str]) -> CardData {
    let mut c = digimon(id, color, 4, traits);
    c.evo_costs = vec![EvoCost {
        card_color: CardColor::Red as u8,
        level: 3,
        memory_cost: 2,
    }];
    c
}

fn egg(id: &str, color: CardColor) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::DigiEgg;
    c.colors = vec![color];
    c.level = Some(2);
    c
}

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT16-017 in embedded DSL pack")
        .add_card(digimon("GREEN-PLAIN", CardColor::Green, 3, &[]))
        .add_card(digimon("RED-FREE", CardColor::Red, 3, &["Free"]))
        .add_card(digimon("RED-PLAIN", CardColor::Red, 3, &[]))
        .add_card(lv4("GREEN-LV4", CardColor::Green, &[]))
        .add_card(lv4("BLACK-LV4", CardColor::Black, &[]))
        .add_card(egg("GREEN-EGG", CardColor::Green))
        .add_card(egg("BLUE-EGG", CardColor::Blue))
        .add_card(make_test_card("DECK-PAD", "DECK-PAD"))
        .deck(0, &["DECK-PAD"; 5])
        .deck(1, &["DECK-PAD"; 5])
        .memory(5)
        .start()
}

fn push_to_hand(runner: &mut DebugRunner, card_id: &str) -> usize {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("{card_id} in card_data"));
    let next = runner.game.next_card_index();
    runner.game.players[0]
        .hand
        .push(CardSource::new(data_idx, 0, next));
    runner.game.players[0].hand.len() - 1
}

/// Memory change from playing `card_id` (cost 3) with Veemon on the field.
fn memory_delta_playing(runner: &mut DebugRunner, card_id: &str) -> i16 {
    let idx = push_to_hand(runner, card_id);
    let before = runner.memory();
    runner.play(0, idx).expect("plays from hand");
    let _ = runner.auto_resolve();
    runner.memory() - before
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn bt16_017_metadata_and_clause_shape() {
    let runner = runner();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.name, "Veemon");
    assert_eq!(card.level, Some(3));
    assert_eq!(
        card.alt_paths.len(),
        3,
        "green Lv.2, purple Lv.2, [DemiVeemon]"
    );

    let observer = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .expect("played-or-digivolves observer");
    assert!(observer.once_per_turn, "[Once Per Turn]");
    assert!(!observer.optional, "'gain 1 memory' is mandatory");

    let aura = card.effects.iter().find_map(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
            scope, dp_modifier, ..
        }) => Some((*scope, *dp_modifier)),
        _ => None,
    });
    assert_eq!(aura, Some((CompiledScope::Inherited, Some(2000))));
}

// ─── Section 2/3 — Observer gating ───────────────────────────────────────────

#[test]
fn bt16_017_green_digimon_played_gains_1_memory() {
    let mut runner = runner();
    runner.place_on_field(0, CARD_ID, None);
    assert_eq!(memory_delta_playing(&mut runner, "GREEN-PLAIN"), -3 + 1);
}

#[test]
fn bt16_017_free_trait_digimon_played_gains_1_memory() {
    let mut runner = runner();
    runner.place_on_field(0, CARD_ID, None);
    assert_eq!(memory_delta_playing(&mut runner, "RED-FREE"), -3 + 1);
}

#[test]
fn bt16_017_non_green_non_free_digimon_gains_nothing() {
    let mut runner = runner();
    runner.place_on_field(0, CARD_ID, None);
    assert_eq!(memory_delta_playing(&mut runner, "RED-PLAIN"), -3);
}

#[test]
fn bt16_017_is_once_per_turn_and_resets() {
    let mut runner = runner();
    runner.game.set_memory(9);
    runner.place_on_field(0, CARD_ID, None);
    assert_eq!(memory_delta_playing(&mut runner, "GREEN-PLAIN"), -2);
    assert_eq!(
        memory_delta_playing(&mut runner, "GREEN-PLAIN"),
        -3,
        "second trigger in the same turn is locked out"
    );
    runner.end_turn();
    runner.end_turn();
    assert_eq!(runner.turn_player(), 0);
    runner.game.set_memory(5);
    assert_eq!(
        memory_delta_playing(&mut runner, "GREEN-PLAIN"),
        -2,
        "lockout clears on the next turn"
    );
}

#[test]
fn bt16_017_veemon_itself_being_played_does_not_trigger() {
    // Veemon is [Free]; "one of your OTHER Digimon" excludes itself.
    let mut runner = runner();
    assert_eq!(memory_delta_playing(&mut runner, CARD_ID), -3);
}

#[test]
fn bt16_017_does_not_trigger_on_opponents_turn() {
    let mut runner = runner();
    runner.place_on_field(0, CARD_ID, None);
    runner.end_turn();
    assert_eq!(runner.turn_player(), 1);
    let before = runner.memory();
    let field = runner.place_on_field(0, "GREEN-PLAIN", None);
    runner.fire_play_event_triggers(0, field.index as usize, true, false);
    let _ = runner.auto_resolve();
    assert_eq!(runner.memory(), before, "[Your Turn] only");
}

#[test]
fn bt16_017_green_digivolve_gains_1_memory() {
    let mut runner = runner();
    runner.place_on_field(0, CARD_ID, None);
    let base = runner.place_on_field(0, "RED-PLAIN", None);
    let idx = push_to_hand(&mut runner, "GREEN-LV4");
    let before = runner.memory();
    assert!(runner
        .game
        .digivolve_from_hand(0, idx, base.index as usize, PlaySource::ByHand));
    let _ = runner.auto_resolve();
    assert_eq!(
        runner.memory() - before,
        -2 + 1,
        "reads the Digimon after it digivolves"
    );
}

#[test]
fn bt16_017_non_green_digivolve_gains_nothing() {
    let mut runner = runner();
    runner.place_on_field(0, CARD_ID, None);
    let base = runner.place_on_field(0, "RED-PLAIN", None);
    let idx = push_to_hand(&mut runner, "BLACK-LV4");
    let before = runner.memory();
    assert!(runner
        .game
        .digivolve_from_hand(0, idx, base.index as usize, PlaySource::ByHand));
    let _ = runner.auto_resolve();
    assert_eq!(runner.memory() - before, -2);
}

// ─── Digivolve requirements ──────────────────────────────────────────────────

#[test]
fn bt16_017_digivolves_from_green_lv2_for_1() {
    let mut runner = runner();
    let base = runner.place_on_field(0, "GREEN-EGG", None);
    let idx = push_to_hand(&mut runner, CARD_ID);
    let before = runner.memory();
    assert!(runner
        .game
        .digivolve_from_hand(0, idx, base.index as usize, PlaySource::ByHand));
    assert_eq!(before - runner.memory(), 1);
}

#[test]
fn bt16_017_cannot_digivolve_from_blue_lv2() {
    let runner = runner();
    let mut runner = runner;
    let base = runner.place_on_field(0, "BLUE-EGG", None);
    let idx = push_to_hand(&mut runner, CARD_ID);
    let card = runner.game.players[0].hand[idx].clone();
    let perm = runner.game.players[0].battle_area[base.index as usize].clone();
    assert!(!runner.game.can_digivolve(&card, &perm), "no blue circle");
}

#[test]
fn bt16_017_digivolves_from_demiveemon_for_0() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT16-017")
        .dsl_card("BT16-002")
        .expect("BT16-002 DemiVeemon")
        .memory(5)
        .start();
    let base = runner.place_on_field(0, "BT16-002", None);
    let idx = push_to_hand(&mut runner, CARD_ID);
    let before = runner.memory();
    assert!(runner
        .game
        .digivolve_from_hand(0, idx, base.index as usize, PlaySource::ByHand));
    let _ = runner.auto_resolve();
    assert_eq!(before - runner.memory(), 0);
}

// ─── Inherited [Your Turn] +2000 ─────────────────────────────────────────────

#[test]
fn bt16_017_inherited_plus_2000_on_your_turn_only() {
    let mut runner = runner();
    let base = runner.place_on_field(0, CARD_ID, None);
    let idx = push_to_hand(&mut runner, "GREEN-LV4");
    let mut lv4 = runner.game.card_data[runner.game.players[0].hand[idx].data_index].clone();
    lv4.evo_costs.push(EvoCost {
        card_color: CardColor::Blue as u8,
        level: 3,
        memory_cost: 2,
    });
    let data_index = runner.game.players[0].hand[idx].data_index;
    runner.game.card_data[data_index] = lv4;
    assert!(runner
        .game
        .digivolve_from_hand(0, idx, base.index as usize, PlaySource::ByHand));
    let _ = runner.auto_resolve();
    runner.game.tick_declarative_effects();
    assert_eq!(
        runner.effective_dp(base),
        Some(5000),
        "3000 + 2000 on your turn"
    );
    runner.end_turn();
    runner.game.tick_declarative_effects();
    assert_eq!(
        runner.effective_dp(base),
        Some(3000),
        "inactive on the opponent's turn"
    );
}
