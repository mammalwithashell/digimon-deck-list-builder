//! RB1-035 Hokuto Amanokawa
//!
//! Printed text (card image RB1-035.webp; cross-checked DCGO RB1_035.cs):
//! - [Start of Your Turn] If your opponent has 3 or more Tamers, gain 1 memory.
//! - [All Turns] When an opponent plays a Digimon, by suspending this Tamer,
//!   gain 1 memory if that Digimon is level 4 or higher, and <Draw 1> if it is
//!   level 3.
//! - [Security] Play this card without paying the cost.
//!
//! DCGO note: `SetUpActivateClass(..., true, ...)` (4th positional `true`) makes
//! the [All Turns] suspend effect optional/declinable; the suspend cost is paid
//! ONCE then both branches (Lv>=4 and Lv==3) are evaluated independently, so a
//! single played Digimon (one level) fires exactly one branch.

use digimon_dsl::compiled::{CompiledClause, CompiledTiming};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::selection::SelectionKind;

#[test]
fn rb1_035_has_start_turn_and_security_clauses() {
    let runner = DebugRunner::builder()
        .dsl_card("RB1-035")
        .expect("RB1-035 must load from embedded DSL pack")
        .memory(5)
        .start();
    let card = runner.compiled_card("RB1-035").expect("compiled card");

    assert!(
        card.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::StartOfYourTurn)
        )),
        "RB1-035 must have a start-of-turn clause"
    );
    assert!(
        card.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSecurity)
        )),
        "RB1-035 must have Security play"
    );
}

#[test]
fn rb1_035_exposes_opponent_played_digimon_level_branch_observer() {
    let runner = DebugRunner::builder()
        .dsl_card("RB1-035")
        .expect("RB1-035 must load from embedded DSL pack")
        .start();
    let card = runner.compiled_card("RB1-035").expect("compiled card");

    let observer = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnAnyDigimonPlayed) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("RB1-035 must have an opponent-played-Digimon observer");

    assert!(
        observer.optional,
        "[All Turns] suspend effect is declinable (DCGO SetUpActivateClass true)"
    );
    assert!(
        observer.condition.is_some(),
        "observer must gate on opponent-owned Digimon"
    );
}

/// Lv.4 opponent Digimon: by suspending the Tamer, gain 1 memory.
#[test]
fn rb1_035_opponent_plays_lv4_gains_one_memory() {
    let mut runner = observer_runner(&[], &["DRAW-RB1-035"], opp_digimon("OPP-LV4-RB1-035", 4));
    let tamer = runner.place_on_field(0, "RB1-035", Some(0));
    let memory_before = runner.memory();
    let hand_before = runner.hand_size(0);
    let deck_before = runner.deck_size(0);

    // Opponent (player 1) plays a Lv.4 Digimon from hand index 0.
    runner.play(1, 0).expect("opponent plays Lv.4 Digimon");
    accept_optional_trigger(&mut runner, 0);
    runner.auto_resolve().expect("settle level-4 branch");

    assert!(
        runner.game.player(0).battle_area[tamer.index as usize].is_suspended,
        "the Tamer must suspend as the activation cost"
    );
    assert_eq!(
        runner.memory(),
        memory_before + 1,
        "Lv>=4 branch must gain 1 memory"
    );
    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "Lv>=4 branch must NOT draw"
    );
    assert_eq!(
        runner.deck_size(0),
        deck_before,
        "Lv>=4 branch leaves the deck untouched"
    );
}

/// Lv.6 opponent Digimon: still in the "level 4 or higher" branch → gain 1 memory.
#[test]
fn rb1_035_opponent_plays_lv6_gains_one_memory_not_draw() {
    let mut runner = observer_runner(&[], &["DRAW-RB1-035"], opp_digimon("OPP-LV6-RB1-035", 6));
    let tamer = runner.place_on_field(0, "RB1-035", Some(0));
    let memory_before = runner.memory();
    let deck_before = runner.deck_size(0);

    runner.play(1, 0).expect("opponent plays Lv.6 Digimon");
    accept_optional_trigger(&mut runner, 0);
    runner.auto_resolve().expect("settle level-6 branch");

    assert!(
        runner.game.player(0).battle_area[tamer.index as usize].is_suspended,
        "the Tamer must suspend as the activation cost"
    );
    assert_eq!(
        runner.memory(),
        memory_before + 1,
        "Lv>=4 (incl. Lv.6) must gain 1 memory"
    );
    assert_eq!(
        runner.deck_size(0),
        deck_before,
        "Lv>=4 must not draw — deck untouched"
    );
}

/// Lv.3 opponent Digimon: by suspending the Tamer, Draw 1 (no memory gain).
#[test]
fn rb1_035_opponent_plays_lv3_draws_one() {
    let mut runner = observer_runner(&[], &["DRAW-RB1-035"], opp_digimon("OPP-LV3-RB1-035", 3));
    let tamer = runner.place_on_field(0, "RB1-035", Some(0));
    let memory_before = runner.memory();

    runner.play(1, 0).expect("opponent plays Lv.3 Digimon");
    accept_optional_trigger(&mut runner, 0);
    runner.auto_resolve().expect("settle level-3 branch");

    assert!(
        runner.game.player(0).battle_area[tamer.index as usize].is_suspended,
        "the Tamer must suspend as the activation cost"
    );
    assert_eq!(runner.hand_size(0), 1, "Lv==3 branch must Draw 1 to hand");
    assert_eq!(runner.deck_size(0), 0, "Draw 1 consumes a deck card");
    assert_eq!(
        runner.memory(),
        memory_before,
        "Lv==3 branch must NOT gain memory"
    );
}

/// Lv.2 opponent Digimon: neither branch fires (no memory, no draw). The card
/// may still suspend per the cost — but with no body effect there's no payoff.
/// DCGO still pays the cost; faithful behavior is: suspend, then neither branch.
#[test]
fn rb1_035_opponent_plays_lv2_no_memory_no_draw() {
    let mut runner = observer_runner(&[], &["DRAW-RB1-035"], opp_digimon("OPP-LV2-RB1-035", 2));
    runner.place_on_field(0, "RB1-035", Some(0));
    let memory_before = runner.memory();
    let hand_before = runner.hand_size(0);
    let deck_before = runner.deck_size(0);

    runner.play(1, 0).expect("opponent plays Lv.2 Digimon");
    // The trigger may still offer to suspend; accept it if offered.
    accept_optional_trigger(&mut runner, 0);
    runner.auto_resolve().expect("settle level-2 (no branch)");

    assert_eq!(
        runner.memory(),
        memory_before,
        "Lv.2 fires neither branch — no memory gain"
    );
    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "Lv.2 fires neither branch — no draw"
    );
    assert_eq!(
        runner.deck_size(0),
        deck_before,
        "Lv.2 leaves the deck untouched"
    );
}

/// Controller's OWN Digimon played: opponent-only filter → no trigger at all.
#[test]
fn rb1_035_own_digimon_played_does_not_trigger() {
    let mut runner = observer_runner(
        &["OWN-LV4-RB1-035"],
        &["DRAW-RB1-035"],
        opp_digimon("OPP-LV4-RB1-035", 4),
    );
    // Give player 0 a level-4 Digimon to play themselves.
    runner.place_on_field(0, "RB1-035", Some(0));
    let memory_before = runner.memory();
    let deck_before = runner.deck_size(0);

    runner.play(0, 0).expect("controller plays own Lv.4 Digimon");
    runner.auto_resolve().expect("settle own play");

    assert!(
        runner.pending_selection_view().is_none(),
        "own Digimon must NOT offer the opponent-played observer"
    );
    assert_eq!(
        runner.memory(),
        memory_before,
        "own Digimon must not gain memory"
    );
    assert_eq!(
        runner.deck_size(0),
        deck_before,
        "own Digimon must not draw"
    );
}

/// Player may decline the optional suspend: PASS leaves the Tamer unsuspended
/// and produces no memory/draw.
#[test]
fn rb1_035_opponent_played_observer_can_be_declined() {
    let mut runner = observer_runner(&[], &["DRAW-RB1-035"], opp_digimon("OPP-LV4-RB1-035", 4));
    let tamer = runner.place_on_field(0, "RB1-035", Some(0));
    let memory_before = runner.memory();
    let deck_before = runner.deck_size(0);

    runner.play(1, 0).expect("opponent plays Lv.4 Digimon");

    // An optional TriggerOrder selection must appear, exposing PASS.
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::TriggerOrder),
        "optional [All Turns] trigger must offer a TriggerOrder before the cost"
    );
    assert!(
        runner.pending_is_optional(),
        "optional trigger must expose PASS"
    );
    runner
        .execute_action(0, PASS)
        .expect("PASS declines the optional trigger");
    runner.auto_resolve().expect("settle after decline");

    assert!(
        !runner.game.player(0).battle_area[tamer.index as usize].is_suspended,
        "declining must NOT suspend the Tamer"
    );
    assert_eq!(
        runner.memory(),
        memory_before,
        "declining must not gain memory"
    );
    assert_eq!(
        runner.deck_size(0),
        deck_before,
        "declining must not draw"
    );
}

/// Already-suspended Tamer cannot pay the suspend cost: no memory/draw and the
/// trigger does not fire (the source-bound cost preflight fails).
#[test]
fn rb1_035_already_suspended_tamer_cannot_pay_cost() {
    let mut runner = observer_runner(&[], &["DRAW-RB1-035"], opp_digimon("OPP-LV4-RB1-035", 4));
    let tamer = runner.place_on_field(0, "RB1-035", Some(0));
    runner.game.player_mut(0).battle_area[tamer.index as usize].is_suspended = true;
    let memory_before = runner.memory();
    let deck_before = runner.deck_size(0);

    runner.play(1, 0).expect("opponent plays Lv.4 Digimon");
    runner.auto_resolve().expect("settle (cost unpayable)");

    assert!(
        runner.game.player(0).battle_area[tamer.index as usize].is_suspended,
        "pre-suspended Tamer stays suspended (cost cannot be re-paid)"
    );
    assert_eq!(
        runner.memory(),
        memory_before,
        "cost failure must not gain memory"
    );
    assert_eq!(
        runner.deck_size(0),
        deck_before,
        "cost failure must not draw"
    );
}

// ─── helpers ────────────────────────────────────────────────────────────────

/// Build a DebugRunner with RB1-035 implied via `.dsl_card`, the opponent's
/// Digimon `opp` registered (placed into player 1's hand index 0 for `play`),
/// plus a draw deck for player 0 and optional own-hand cards.
fn observer_runner(own_hand: &[&str], own_deck: &[&str], opp: CardData) -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("RB1-035")
        .expect("RB1-035 YAML loads")
        .add_card(opp.clone())
        .add_card(own_lv4_digimon("OWN-LV4-RB1-035"))
        .add_card(draw_card("DRAW-RB1-035"))
        .hand(0, own_hand)
        .hand(1, &[card_id(&opp)])
        .deck(0, own_deck)
        // Mid-range memory: high enough to hard-play, low enough that the +1
        // memory gain is not clamped at the `memory_range.1` (=10) ceiling.
        .memory(3)
        .start()
}

fn card_id(c: &CardData) -> &str {
    c.card_id.as_str()
}

/// An opponent's Digimon with `play_cost = 0` (so the opponent can hard-play it
/// for free regardless of memory) at the given level.
fn opp_digimon(id: &str, level: u8) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Red];
    card.level = Some(level);
    card.dp = Some(3000);
    card.play_cost = 0;
    card
}

fn own_lv4_digimon(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::White];
    card.level = Some(4);
    card.dp = Some(4000);
    card.play_cost = 0;
    card
}

fn draw_card(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::White];
    card.level = Some(3);
    card.dp = Some(1000);
    card.play_cost = 2;
    card
}

/// If an optional TriggerOrder selection is pending for `player`, accept the
/// trigger (pick the first non-PASS valid action). If no selection is pending,
/// this is a no-op (the trigger may have been skipped because the cost is
/// unpayable or no branch applies).
fn accept_optional_trigger(runner: &mut DebugRunner, player: u8) {
    if runner.pending_kind() == Some(SelectionKind::TriggerOrder) {
        let action = {
            let sel = runner.pending_selection().expect("pending selection");
            // Prefer a non-PASS action to accept the trigger.
            sel.valid_action_ids
                .iter()
                .copied()
                .find(|&a| a != PASS)
                .unwrap_or(PASS)
        };
        runner
            .execute_action(player, action)
            .expect("accept optional trigger");
    }
}
