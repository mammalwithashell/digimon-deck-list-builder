//! Unit tests for the tensor-building helpers added for §3.1 / §3.2:
//! `source_dp_contribution`, `opt_total`, `opt_used`, `source_opt_state`.
//!
//! These tests exercise the pure functions without involving the full
//! tensor — they pin the semantics (filter by inherited-vs-top, condition
//! gating, activation tracking) before we wire the helpers into
//! `write_slot`.

use std::sync::Arc;

use digimon_engine::card_registry::CardRegistry;
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::cards::CardEffectRegistry;
use digimon_engine::debug_runner::{make_test_card, make_test_egg, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::Expiry;
use digimon_engine::game::Game;
use digimon_engine::permanent::PermanentHandle;

/// "DP-BUFF" — non-inherited: +1000 DP always.
pub struct DpBuff;
impl CardEffect for DpBuff {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::declarative(card)
            .name("+1000 DP (non-inherited)")
            .dp_modifier(1000)
            .build()]
    }
}

/// "INH-BUFF" — inherited: −3000 DP always.
pub struct InhBuff;
impl CardEffect for InhBuff {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::inherited(card)
            .name("-3000 DP (inherited)")
            .dp_modifier(-3000)
            .build()]
    }
}

/// "COND-BUFF" — non-inherited: +2000 DP only when memory >= 0.
pub struct CondBuff;
impl CardEffect for CondBuff {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::declarative(card)
            .name("+2000 DP when memory >= 0")
            .dp_modifier(2000)
            .condition(|ctx| ctx.memory() >= 0)
            .build()]
    }
}

/// "OPT" — non-inherited once-per-turn effect (no DP change; just used to
/// exercise OPT tracking).
pub struct OptCard;
impl CardEffect for OptCard {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("Once-per-turn marker")
            .once_per_turn()
            .process(|_| {})
            .build()]
    }
}

/// Build a registry with our local test effects only.
fn test_registry() -> CardEffectRegistry {
    let mut r = CardEffectRegistry::default();
    r.insert("DP-BUFF", Arc::new(DpBuff));
    r.insert("INH-BUFF", Arc::new(InhBuff));
    r.insert("COND-BUFF", Arc::new(CondBuff));
    r.insert("OPT", Arc::new(OptCard));
    r
}

fn fresh_runner() -> DebugRunner {
    DebugRunner::builder()
        .add_card(make_test_card("DP-BUFF", "DpBuff"))
        .add_card(make_test_card("INH-BUFF", "InhBuff"))
        .add_card(make_test_card("COND-BUFF", "CondBuff"))
        .add_card(make_test_card("OPT", "OptCard"))
        .with_registry(test_registry())
        .start()
}

pub fn sample_game_with_known_cards() -> (Game, CardRegistry) {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_egg("ST1-01", "Koromon"))
        .add_card(make_test_card("ST1-02", "Biyomon"))
        .add_card(make_test_card("ST1-03", "Agumon"))
        .add_card(make_test_card("ST1-04", "Greymon"))
        .add_card(make_test_card("ST1-05", "Garurumon"))
        .add_card(make_test_card("ST1-06", "Tyrannomon"))
        .add_card(make_test_card("ST1-07", "Birdramon"))
        .hand(0, &["ST1-02"])
        .hand(1, &["ST1-03"])
        .security(0, &["ST1-04"])
        .security(1, &["ST1-05"])
        .start();

    runner.place_in_breeding(0, "ST1-01");
    runner.place_on_field(0, "ST1-06", Some(0));
    let own_trash = card_source(&runner, 0, "ST1-04");
    let opp_trash = card_source(&runner, 1, "ST1-05");
    let revealed = card_source(&runner, 0, "ST1-07");
    runner.game.players[0].trash.push(own_trash);
    runner.game.players[1].trash.push(opp_trash);
    runner.game.revealed_cards.push(revealed);

    let cards = runner
        .game
        .card_data
        .iter()
        .map(|card| (card.card_id.clone(), card.clone()))
        .collect();
    let registry = CardRegistry::from_cards(&cards);
    (runner.game, registry)
}

fn card_source(runner: &DebugRunner, owner: u8, card_id: &str) -> CardSource {
    let data_index = runner
        .game
        .card_data
        .iter()
        .position(|card| card.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card_id {card_id}"));
    CardSource::new(data_index, owner, 10_000 + data_index as u16)
}

// ─── source_dp_contribution ─────────────────────────────────────────

#[test]
fn single_top_card_non_inherited_dp_buff_is_counted() {
    let mut r = fresh_runner();
    let h = r.place_on_field(0, "DP-BUFF", Some(0));
    assert_eq!(r.game.source_dp_contribution(h, 0), 1000);
}

#[test]
fn single_top_card_inherited_buff_is_not_counted_when_on_top() {
    // When INH-BUFF is the top card (no digivolve), its inherited effect
    // should NOT contribute — inherited effects only activate when under.
    let mut r = fresh_runner();
    let h = r.place_on_field(0, "INH-BUFF", Some(0));
    assert_eq!(r.game.source_dp_contribution(h, 0), 0);
}

#[test]
fn inherited_buff_under_a_top_card_contributes() {
    // Place DP-BUFF, then digivolve INH-BUFF onto it: index 0 is INH-BUFF
    // (under), index 1 is DP-BUFF (top). Source 0 contributes -3000;
    // source 1 contributes +1000.
    let mut r = fresh_runner();
    let _ = r.place_on_field(0, "INH-BUFF", Some(0));
    // Digivolve DP-BUFF on top.
    let data_idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "DP-BUFF")
        .unwrap();
    let next = r.game.next_card_index();
    let top = CardSource::new(data_idx, 0, next);
    r.game.digivolve_onto(0, 0, top);

    let h = PermanentHandle {
        player: 0,
        index: 0,
    };
    assert_eq!(
        r.game.source_dp_contribution(h, 0),
        -3000,
        "under-source INH-BUFF contributes its inherited -3000"
    );
    assert_eq!(
        r.game.source_dp_contribution(h, 1),
        1000,
        "top-source DP-BUFF contributes its non-inherited +1000"
    );
}

#[test]
fn non_inherited_buff_under_a_top_card_does_not_contribute() {
    // Place DP-BUFF (non-inherited buff) then digivolve DP-BUFF over it.
    // The underneath DP-BUFF's non-inherited effect should NOT carry up.
    let mut r = fresh_runner();
    let _ = r.place_on_field(0, "DP-BUFF", Some(0));
    let data_idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "DP-BUFF")
        .unwrap();
    let next = r.game.next_card_index();
    let top = CardSource::new(data_idx, 0, next);
    r.game.digivolve_onto(0, 0, top);

    let h = PermanentHandle {
        player: 0,
        index: 0,
    };
    assert_eq!(
        r.game.source_dp_contribution(h, 0),
        0,
        "under-source's non-inherited buff is suppressed"
    );
    assert_eq!(r.game.source_dp_contribution(h, 1), 1000);
}

#[test]
fn conditional_dp_buff_suppressed_when_condition_is_false() {
    let mut r = fresh_runner();
    let h = r.place_on_field(0, "COND-BUFF", Some(0));
    // Starts at memory 0 → condition true → +2000 counted.
    r.game.set_memory(0);
    assert_eq!(r.game.source_dp_contribution(h, 0), 2000);
    // Negative memory → condition false → 0.
    r.game.set_memory(-1);
    assert_eq!(r.game.source_dp_contribution(h, 0), 0);
}

#[test]
fn unregistered_card_contributes_zero() {
    let r = DebugRunner::builder()
        .add_card(make_test_card("UNKNOWN", "Unknown"))
        .start();
    // No place_on_field needed — we just pass an out-of-range handle.
    let h = PermanentHandle {
        player: 0,
        index: 0,
    };
    assert_eq!(r.game.source_dp_contribution(h, 0), 0);
}

// ─── opt_total / opt_used / source_opt_state ─────────────────────────

#[test]
fn opt_total_counts_max_per_turn_effects() {
    let mut r = fresh_runner();
    let h = r.place_on_field(0, "OPT", Some(0));
    assert_eq!(r.game.opt_total(h), 1);
    assert_eq!(r.game.opt_used(h), 0);
    assert!((r.game.source_opt_state(h, 0) - 1.0).abs() < 1e-6);
}

#[test]
fn opt_used_reflects_activation_count() {
    let mut r = fresh_runner();
    let h = r.place_on_field(0, "OPT", Some(0));
    // Manually record an activation (slot 0, the OPT effect).
    let source_handle = r.game.players[0].battle_area[0].card_sources[0].handle();
    r.game.players[0].battle_area[0].record_activation(source_handle, 0);
    assert_eq!(r.game.opt_used(h), 1);
    assert_eq!(r.game.opt_total(h), 1);
    assert!((r.game.source_opt_state(h, 0) - 0.0).abs() < 1e-6);
}

#[test]
fn opt_counts_persist_until_new_turn() {
    // Activation count clears when `new_turn` runs (via Player::new_turn
    // during begin_turn). Simulate a turn reset and observe the counter.
    let mut r = fresh_runner();
    let h = r.place_on_field(0, "OPT", Some(0));
    let source_handle = r.game.players[0].battle_area[0].card_sources[0].handle();
    r.game.players[0].battle_area[0].record_activation(source_handle, 0);
    assert_eq!(r.game.opt_used(h), 1);

    r.game.players[0].battle_area[0].new_turn();
    assert_eq!(r.game.opt_used(h), 0);
}

#[test]
fn opt_filters_inherited_vs_top_for_stack() {
    // If an OPT effect lives on an inherited source, it counts when under
    // and not when on top. Our OPT test card's effect is non-inherited, so
    // under a top card it should NOT contribute to opt_total.
    let mut r = fresh_runner();
    let _ = r.place_on_field(0, "OPT", Some(0));
    let data_idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "DP-BUFF")
        .unwrap();
    let next = r.game.next_card_index();
    let top = CardSource::new(data_idx, 0, next);
    r.game.digivolve_onto(0, 0, top);

    let h = PermanentHandle {
        player: 0,
        index: 0,
    };
    // OPT under a top card: its non-inherited OPT effect is filtered out.
    // DP-BUFF's only effect has max_per_turn = 0, so nothing counts.
    assert_eq!(r.game.opt_total(h), 0);
}

#[test]
fn source_opt_state_zero_when_source_has_no_opt_effects() {
    let mut r = fresh_runner();
    let h = r.place_on_field(0, "DP-BUFF", Some(0));
    assert!((r.game.source_opt_state(h, 0) - 0.0).abs() < 1e-6);
}

// Silence unused-import warnings if a future refactor removes Expiry.
#[allow(dead_code)]
fn _unused() -> Expiry {
    Expiry::EndOfTurn
}
