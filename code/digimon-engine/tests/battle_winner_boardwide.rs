//! Gap 1 — board-wide battle-winner trigger [G-DSL-BATTLE-WINNER-BOARDWIDE].
//!
//! Driver: BT25-020 Marsmon — "[All Turns][OPT] When any of your [TS] trait
//! Digimon win a battle, trash your opponent's top security card."
//!
//! The existing `source_deleted_battle_opponent` predicate is CARRIER-scoped
//! (fires only when the observer itself is the winner, via the OnAnyDeletion
//! path). This adds a BOARD-WIDE observer keyed on `EndOfBattle` that carries
//! the winner permanent so a DIFFERENT Digimon can react to an ally winning.
//!
//! DCGO semantics (BT25_020.cs + CardEffectCommons.CanTriggerWhenWinBattle):
//! - "wins a battle" = a Digimon-vs-Digimon battle where the opposing Digimon
//!   is deleted (there is a LoserCard) AND it was NOT a tie (WasTie == false).
//! - the winner permanent (owner + trait) is exposed to the predicate layer.
//! - a TIE (mutual destruction) is NOT a win — no winner.
//! - a direct player attack (security check, no Digimon battle) is NOT a win.
//!
//! These tests drive the ENGINE dispatch with hand-written recording effects.
//! They do NOT author the driver card (BT25-020).

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use std::sync::{Arc, Mutex};

fn digimon(id: &str, name: &str, dp: i32, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(dp);
    c.play_cost = 5;
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

/// A board-wide observer that records each time it fires on a battle-win event
/// where the winner is owned by the observer's controller and has the given
/// trait. This is the `on_ally_won_battle` board-wide substrate under test.
struct AllyWonBattleObserver {
    /// Winner-trait gate; the observer fires only when the battle winner has it.
    trait_gate: String,
    /// Records the observer's controller each time the body fires.
    fired: Arc<Mutex<Vec<PlayerId>>>,
}

impl CardEffect for AllyWonBattleObserver {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let trait_gate = self.trait_gate.clone();
        let fired = self.fired.clone();
        vec![Effect::end_of_battle(card)
            .name("board-wide: when an ally with trait wins a battle")
            .condition(move |ctx| {
                // Board-wide winner gate: a winner permanent must exist
                // (non-tie battle-win), be owned by the observer, and carry
                // the trait.
                let Some(winner) = ctx.battle_winner() else {
                    return false;
                };
                if winner.player != ctx.player() {
                    return false;
                }
                ctx.battle_winner_has_trait(&trait_gate)
            })
            .process(move |ctx| {
                fired.lock().unwrap().push(ctx.player);
            })
            .build()]
    }
}

fn base(fired: Arc<Mutex<Vec<PlayerId>>>, trait_gate: &str) -> DebugRunner {
    // OBS is the board-wide observer (a separate Digimon). ATK-TS / ATK-PLAIN
    // are the attackers; DEF-* the defenders.
    let mut r = DebugRunner::builder()
        .add_card(digimon("OBS", "Observer", 3000, &[]))
        .add_card(digimon("ATK-TS", "TsAttacker", 9000, &["TS"]))
        .add_card(digimon("ATK-PLAIN", "PlainAttacker", 9000, &[]))
        .add_card(digimon("DEF-WEAK", "WeakDef", 3000, &[]))
        .add_card(digimon("DEF-EQUAL", "EqualDef", 9000, &[]))
        .add_card(digimon("F", "F", 1000, &[]))
        .deck(0, &["F"; 10])
        .deck(1, &["F"; 10])
        .memory(10)
        .start();
    r.register_effect(
        "OBS",
        Arc::new(AllyWonBattleObserver {
            trait_gate: trait_gate.to_string(),
            fired,
        }),
    );
    r.game.turn_count = 1;
    r.game.turn_player_idx = 0;
    r
}

// ─── Positive: a DIFFERENT ally Digimon with the trait wins a battle ─────────

#[test]
fn fires_when_a_separate_ally_ts_digimon_wins_a_battle() {
    let fired = Arc::new(Mutex::new(Vec::new()));
    let mut r = base(fired.clone(), "TS");

    // Player 0 fields the observer (no TS) AND a separate TS attacker.
    let _obs = r.place_on_field(0, "OBS", Some(0));
    let atk = r.place_on_field(0, "ATK-TS", Some(0));
    // Player 1 fields a weak defender that will be deleted (a battle win).
    let def = r.place_on_field(1, "DEF-WEAK", Some(0));

    r.attack_digimon(atk, def, false);
    r.auto_resolve().expect("resolve battle");

    let f = fired.lock().unwrap();
    assert_eq!(
        f.as_slice(),
        &[0],
        "board-wide observer fires exactly once for player 0 when its separate \
         [TS] ally wins a battle (winner != observer)"
    );
}

// ─── Negative: winner lacks the trait ────────────────────────────────────────

#[test]
fn does_not_fire_when_winner_lacks_trait() {
    let fired = Arc::new(Mutex::new(Vec::new()));
    let mut r = base(fired.clone(), "TS");

    let _obs = r.place_on_field(0, "OBS", Some(0));
    let atk = r.place_on_field(0, "ATK-PLAIN", Some(0)); // no TS trait
    let def = r.place_on_field(1, "DEF-WEAK", Some(0));

    r.attack_digimon(atk, def, false);
    r.auto_resolve().expect("resolve battle");

    assert!(
        fired.lock().unwrap().is_empty(),
        "board-wide [TS] observer must NOT fire when the battle winner has no [TS] trait"
    );
}

// ─── Negative: winner owned by the OPPONENT ──────────────────────────────────

#[test]
fn does_not_fire_when_opponent_ts_digimon_wins() {
    let fired = Arc::new(Mutex::new(Vec::new()));
    let mut r = base(fired.clone(), "TS");

    // Observer on player 0. A TS attacker on player 1 wins against player 0's
    // weak defender — the winner is the OPPONENT's, so player 0's observer
    // must not fire.
    let _obs = r.place_on_field(0, "OBS", Some(0));
    let def0 = r.place_on_field(0, "DEF-WEAK", Some(0));
    let atk1 = r.place_on_field(1, "ATK-TS", Some(0));

    r.game.turn_player_idx = 1;
    r.attack_digimon(atk1, def0, false);
    r.auto_resolve().expect("resolve battle");

    assert!(
        fired.lock().unwrap().is_empty(),
        "board-wide observer on player 0 must NOT fire when the OPPONENT's [TS] \
         Digimon wins (winner.owner != observer.owner)"
    );
}

// ─── Negative: a TIE (mutual destruction) is not a win ───────────────────────

#[test]
fn does_not_fire_on_a_tie_mutual_destruction() {
    let fired = Arc::new(Mutex::new(Vec::new()));
    let mut r = base(fired.clone(), "TS");

    let _obs = r.place_on_field(0, "OBS", Some(0));
    let atk = r.place_on_field(0, "ATK-TS", Some(0)); // 9000
    let def = r.place_on_field(1, "DEF-EQUAL", Some(0)); // 9000 → tie

    r.attack_digimon(atk, def, false);
    r.auto_resolve().ok();

    assert!(
        fired.lock().unwrap().is_empty(),
        "a tie (mutual destruction) has no winner → the board-wide observer must not fire"
    );
}

// ─── Negative: a direct player attack (security check) is not a battle win ───

#[test]
fn does_not_fire_on_direct_player_attack() {
    let fired = Arc::new(Mutex::new(Vec::new()));
    let mut r = base(fired.clone(), "TS");

    let _obs = r.place_on_field(0, "OBS", Some(0));
    let atk = r.place_on_field(0, "ATK-TS", Some(0));
    // Player 1 has NO Digimon; attacking the player is a security check, not a
    // Digimon-vs-Digimon battle → EndOfBattle never fires.

    r.attack_player(atk, 1, false);
    r.auto_resolve().ok();

    assert!(
        fired.lock().unwrap().is_empty(),
        "a direct player attack / security check is not a battle win"
    );
}

// ─── OPT accounting: two battle wins in one turn only fire the OPT body once ─

/// An OPT board-wide observer: fires at most once per turn even across two
/// separate battle wins. Mirrors BT25-020's [Once Per Turn].
struct AllyWonBattleObserverOpt {
    fired: Arc<Mutex<u32>>,
}
impl CardEffect for AllyWonBattleObserverOpt {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let fired = self.fired.clone();
        vec![Effect::end_of_battle(card)
            .name("board-wide OPT ally-won-battle")
            .once_per_turn()
            .condition(|ctx| {
                let Some(winner) = ctx.battle_winner() else {
                    return false;
                };
                winner.player == ctx.player() && ctx.battle_winner_has_trait("TS")
            })
            .process(move |ctx| {
                *fired.lock().unwrap() += 1;
            })
            .build()]
    }
}

#[test]
fn opt_body_fires_at_most_once_per_turn_across_two_wins() {
    let count = Arc::new(Mutex::new(0u32));
    let mut r = DebugRunner::builder()
        .add_card(digimon("OBS", "Observer", 3000, &[]))
        .add_card(digimon("ATK-TS", "TsAttacker", 9000, &["TS"]))
        .add_card(digimon("DEF-WEAK", "WeakDef", 3000, &[]))
        .add_card(digimon("DEF-WEAK2", "WeakDef2", 3000, &[]))
        .add_card(digimon("F", "F", 1000, &[]))
        .deck(0, &["F"; 10])
        .deck(1, &["F"; 10])
        .memory(10)
        .start();
    r.register_effect("OBS", Arc::new(AllyWonBattleObserverOpt { fired: count.clone() }));
    r.game.turn_count = 1;
    r.game.turn_player_idx = 0;

    let _obs = r.place_on_field(0, "OBS", Some(0));
    let atk = r.place_on_field(0, "ATK-TS", Some(0));
    let def1 = r.place_on_field(1, "DEF-WEAK", Some(0));
    let def2 = r.place_on_field(1, "DEF-WEAK2", Some(1));

    // First win.
    r.attack_digimon(atk, def1, false);
    r.auto_resolve().expect("resolve battle 1");
    // Second win (same attacker, same turn) — attacker unsuspended for the test.
    if let Some(perm) = r.game.players[0].battle_area.get_mut(atk.index as usize) {
        perm.is_suspended = false;
    }
    let def2_now = r.perm_handle(1, 0);
    r.attack_digimon(atk, def2_now, false);
    r.auto_resolve().expect("resolve battle 2");

    assert_eq!(
        *count.lock().unwrap(),
        1,
        "[Once Per Turn] board-wide ally-won-battle body fires only once across two wins in a turn"
    );
}
