//! BT13-102 Keenan Crier — Tamer, Purple, Cost 3.
//!
//! # Card text (card face / cards.json — agree)
//!
//! ```text
//! [On Play] Your opponent may trash 1 Tamer card or Option card in their
//!     hand. If they don't, gain 1 memory and <Draw 1> (Draw 1 card from your
//!     deck.)
//! [Opponent's Turn] When an effect plays a Digimon, by suspending this Tamer,
//!     gain 1 memory.
//! ```
//!
//! Security Effect: [Security] Play this card without paying the cost.
//!
//! # DCGO C# reference — DCGO/Assets/Scripts/CardEffect/BT13/Purple/BT13_102.cs
//!
//! Clause 1 (`OnEnterFieldAnyone` + `CanTriggerOnPlay`): the OPPONENT is the
//!   selecting player over THEIR OWN hand. `SelectHandEffect(selectPlayer:
//!   Enemy, canTargetCondition: Tamer || Option, maxCount: 1, canNoSelect:
//!   true, Mode.Discard)`; `discarded` set iff they trash. `if (!discarded) {
//!   Draw(1); AddMemory(1) }` for `card.Owner` — DCGO order is Draw THEN
//!   AddMemory. So declining (no trash) → controller draws 1 and gains 1
//!   memory; trashing → neither.
//!
//! Clause 2 (`OnEnterFieldAnyone`): `IsOpponentTurn(card)` +
//!   `CanTriggerOnPermanentPlay(Digimon)` + `IsByEffect(...)` → optional
//!   suspend-self cost (`CanActivateSuspendCostEffect`) → `AddMemory(1)`.
//!   I.e. on the OPPONENT's turn, when an EFFECT (not a hard hand-play) plays
//!   a Digimon, the controller may suspend this Tamer to gain 1 memory.
//!
//! Clause 3 (`SecuritySkill`): `PlaySelfTamerSecurityEffect` — already
//!   implemented and covered by `bt13_102_loads_security_play_slice`.
//!
//! # Precedent
//! - Clause 1 decline-branch shape: AD1-002 (`select_hand` + `if
//!   binding_*`), inverted to `binding_absent`; EX5-060 for the
//!   `as_selecting_player { of: opponent }` opponent-hidden-hand wrapper.
//! - Clause 2 suspend-self observer: EX11-057 clause 3 (`optional: true` +
//!   `activation_cost: { suspend_self: true }` + `source_is_unsuspended`),
//!   keyed on the effect-play event via `event_is_effect_initiated`.

#![allow(unused_imports, dead_code)]

use digimon_dsl::compiled::{CompiledClause, CompiledTiming};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "BT13-102";

fn tamer(id: &str, name: &str) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Tamer;
    card
}

fn option_card(id: &str, name: &str) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Option;
    card
}

fn digimon(id: &str, name: &str, level: u8) -> CardData {
    let mut card = make_test_card_with_level(id, name, level);
    card.card_kind = CardKind::Digimon;
    card.dp = Some(3000);
    card
}

// ════════════════════════════════════════════════════════════════════════════
// Section 0 — Structural / load
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn bt13_102_loads_security_play_slice() {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT13-102 must load from embedded DSL pack")
        .start();
}

/// The [On Play] opponent-discard clause (clause 1) must compile and be
/// present as an `on_play` trigger. (Clause 2, the [Opponent's Turn]
/// effect-play suspend observer, is stubbed in the YAML pending the engine gap
/// `G-SUSPEND-SELF-COST-ON-OPPONENTS-TURN`, so it is intentionally absent.)
#[test]
fn bt13_102_on_play_clause_present() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT13-102 loads")
        .start();
    let compiled = runner.compiled_card(CARD_ID).expect("BT13-102 in pack");

    let has_on_play = compiled.effects.iter().any(|c| {
        matches!(c, CompiledClause::Triggered(t)
            if t.when.contains(&CompiledTiming::OnPlay))
    });
    assert!(
        has_on_play,
        "BT13-102 must carry the [On Play] opponent-discard triggered clause"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Section 1 — Clause 1 [On Play] opponent-may-trash / if-they-don't gain+draw
// ════════════════════════════════════════════════════════════════════════════

/// Build a runner where the OPPONENT (player 1) holds 1 Tamer card in hand and
/// the controller (player 0) has a deck to draw from. BT13-102 sits on P0's
/// field; its [On Play] is fired manually.
fn on_play_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT13-102 loads")
        .add_card(tamer("OPP-TAMER", "OppTamer"))
        .add_card(option_card("OPP-OPTION", "OppOption"))
        .add_card(digimon("OPP-DIGI", "OppDigi", 3))
        .add_card(make_test_card("DRAW-A", "DrawA"))
        .hand(1, &["OPP-TAMER"])
        .deck(0, &["DRAW-A", "DRAW-A", "DRAW-A"])
        .memory(0)
        .start()
}

/// DECLINE branch: the opponent passes on trashing → the controller gains 1
/// memory AND draws 1 card. The decline must be exposed (opponent is the
/// selecting player and the prompt is optional / PASS-able).
#[test]
fn bt13_102_on_play_opponent_declines_controller_gains_memory_and_draws() {
    let mut runner = on_play_runner();
    let keenan = runner.place_on_field(0, CARD_ID, Some(0));

    let mem_before = runner.memory();
    let hand0_before = runner.hand_size(0);
    let hand1_before = runner.hand_size(1);

    runner.fire_on_play(0, keenan.index as usize);

    // The trash prompt must install for the OPPONENT and be declinable.
    let pending = runner
        .pending_selection()
        .expect("[On Play] opponent-discard prompt must install");
    assert_eq!(
        pending.selecting_player, 1,
        "the opponent (player 1) chooses over their own hidden hand"
    );
    assert!(
        pending.is_optional || pending.valid_action_ids.contains(&PASS),
        "'your opponent may trash' must surface a decline (PASS) to the RL action space"
    );

    // Opponent declines (PASS). Then resolve any follow-up.
    runner
        .execute_action(1, PASS)
        .expect("opponent declines the trash");
    let _ = runner.auto_resolve();

    // Controller gains 1 memory. Memory is stored from the turn player's
    // perspective; player 0 is the turn player, so +1 favours player 0.
    assert_eq!(
        runner.memory(),
        mem_before + 1,
        "declining → controller (P0) gains exactly 1 memory"
    );
    // Controller draws 1 (hand +1); the opponent's hand is unchanged.
    assert_eq!(
        runner.hand_size(0),
        hand0_before + 1,
        "declining → controller draws exactly 1 card"
    );
    assert_eq!(
        runner.hand_size(1),
        hand1_before,
        "declining → the opponent's hand is unchanged (nothing trashed)"
    );
}

/// ACCEPT branch: the opponent trashes a qualifying card → NO memory gain and
/// NO draw for the controller; the opponent's hand drops by 1.
#[test]
fn bt13_102_on_play_opponent_trashes_no_memory_no_draw() {
    let mut runner = on_play_runner();
    let keenan = runner.place_on_field(0, CARD_ID, Some(0));

    let mem_before = runner.memory();
    let hand0_before = runner.hand_size(0);
    let hand1_before = runner.hand_size(1);
    let opp_trash_before = runner.trash_size(1);

    runner.fire_on_play(0, keenan.index as usize);

    let pending = runner
        .pending_selection()
        .expect("[On Play] opponent-discard prompt must install");
    assert_eq!(pending.selecting_player, 1);
    // The Tamer card must be a selectable trash target (a non-PASS action).
    let pick = *pending
        .valid_action_ids
        .iter()
        .find(|&&id| id != PASS)
        .expect("the opponent's Tamer card must be a selectable trash target");

    runner
        .execute_action(1, pick)
        .expect("opponent trashes the Tamer card");
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.memory(),
        mem_before,
        "trashing → controller gains NO memory"
    );
    assert_eq!(
        runner.hand_size(0),
        hand0_before,
        "trashing → controller does NOT draw"
    );
    assert_eq!(
        runner.hand_size(1),
        hand1_before - 1,
        "trashing → the opponent's hand drops by 1"
    );
    assert_eq!(
        runner.trash_size(1),
        opp_trash_before + 1,
        "trashing → the trashed card goes to the opponent's trash"
    );
}

/// The opponent's selectable trash targets are restricted to Tamer / Option
/// cards. A Digimon in their hand must NOT be a valid pick.
#[test]
fn bt13_102_on_play_only_tamer_or_option_selectable() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT13-102 loads")
        .add_card(option_card("OPP-OPTION", "OppOption"))
        .add_card(digimon("OPP-DIGI", "OppDigi", 3))
        .add_card(make_test_card("DRAW-A", "DrawA"))
        // Opponent holds an Option (eligible) and a Digimon (NOT eligible).
        .hand(1, &["OPP-OPTION", "OPP-DIGI"])
        .deck(0, &["DRAW-A", "DRAW-A"])
        .memory(0)
        .start();
    let keenan = runner.place_on_field(0, CARD_ID, Some(0));

    runner.fire_on_play(0, keenan.index as usize);

    let pending = runner
        .pending_selection()
        .expect("opponent-discard prompt installs (opponent has an eligible card)");
    assert_eq!(pending.selecting_player, 1);
    // Exactly one non-PASS target (the Option); the Digimon is filtered out.
    let non_pass: Vec<u16> = pending
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&id| id != PASS)
        .collect();
    assert_eq!(
        non_pass.len(),
        1,
        "only the Tamer/Option card is selectable; the Digimon is filtered out"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Section 2 — Clause 2 [Opponent's Turn] effect-play suspend-self observer
// ════════════════════════════════════════════════════════════════════════════

/// Build a clause-2 runner: both players have a deck (so the staged board
/// passes turn-machine validation and no deckout fires mid-resolution),
/// BT13-102 on P0's field, and it is the OPPONENT's (P1's) turn.
fn observer_runner() -> (DebugRunner, PermanentHandle) {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT13-102 loads")
        .add_card(digimon("OPP-DIGI", "OppDigi", 4))
        .add_card(make_test_card("DECK-A", "DeckA"))
        .deck(0, &["DECK-A", "DECK-A", "DECK-A"])
        .deck(1, &["DECK-A", "DECK-A", "DECK-A"])
        .memory(0)
        .start();
    let keenan = runner.place_on_field(0, CARD_ID, Some(0));
    // Advance to the OPPONENT's (player 1's) turn by ending P0's turn.
    runner.end_turn();
    assert_eq!(runner.turn_player(), 1, "it must be the opponent's turn");
    runner.game.memory = 0;
    (runner, keenan)
}

/// On the OPPONENT's turn, when an EFFECT plays a Digimon, the controller may
/// suspend this Tamer to gain 1 memory. Accepting suspends + gains.
///
/// IGNORED — pending engine gap `G-SUSPEND-SELF-COST-ON-OPPONENTS-TURN`.
/// The observer clause fires correctly on the opponent's turn (the optional
/// pre-cost prompt installs and `gain_memory` resolves), but the suspend-self
/// activation cost silently no-ops whenever the effect's controller is NOT the
/// turn player. Verified empirically in this worktree:
///   - own turn (any `effect_initiated`, any playing player) → suspend WORKS;
///   - opponent's turn → suspend silently no-ops (no `OnSuspend` event), while
///     `Game::suspend_with_cause(keenan, true)` called directly DOES suspend
///     and both `progress_excludes(keenan, Some(0/1))` and
///     `permanent_is_unaffected_by_effect(keenan, 0/1, Tamer)` return false.
/// So `EffectContext::can_affect_permanent` would pass; the no-op lives deeper
/// in the live cost-resolution context's source-permanent / controller
/// threading for an `OnEnterFieldAnyone` observer fired on the opponent's turn.
/// Shipping the clause with the cost silently unpaid (free memory) would be an
/// approximation (CLAUDE.md §17/§18), so the clause is stubbed in the YAML.
#[ignore = "pending: G-SUSPEND-SELF-COST-ON-OPPONENTS-TURN — suspend-self activation cost silently no-ops when the effect's controller is not the turn player (opponent's turn); only the suspend half fails, gain_memory works"]
#[test]
fn bt13_102_observer_effect_play_opp_turn_accept_suspends_and_gains() {
    let (mut runner, keenan) = observer_runner();

    // An opponent Digimon enters via an EFFECT (effect_initiated: true).
    let opp = runner.place_on_field(1, "OPP-DIGI", Some(0));
    runner.fire_play_event_triggers(1, opp.index as usize, true, false);

    // The optional suspend-self prompt installs for the controller (P0).
    let pending = runner
        .pending_selection()
        .expect("the effect-play observer must install an optional prompt");
    assert_eq!(
        pending.selecting_player, 0,
        "P0 (the Tamer's controller) decides whether to pay the suspend cost"
    );
    assert!(
        pending.is_optional || pending.valid_action_ids.contains(&PASS),
        "'by suspending this Tamer' is a declinable activation cost"
    );
    assert!(
        !runner.game.players[0].battle_area[keenan.index as usize].is_suspended,
        "the suspend cost must not be paid before accepting"
    );

    runner.accept_optional_trigger().expect("accept the suspend");
    runner.game.drain_effect_queue();

    assert!(
        runner.game.players[0].battle_area[keenan.index as usize].is_suspended,
        "accepting pays the suspend cost — the Tamer is suspended"
    );
    // Memory is stored from the turn player's (P1's) perspective. P0 gains 1
    // memory on the opponent's turn, so the gauge moves toward P0: -1.
    assert_eq!(
        runner.memory(),
        -1,
        "accepting → the controller (P0) gains 1 memory on the opponent's turn"
    );
}

/// Declining the suspend prompt → the Tamer stays unsuspended and no memory.
///
/// IGNORED on the same engine gap as the accept test
/// (`G-SUSPEND-SELF-COST-ON-OPPONENTS-TURN`): clause 2 is stubbed in the YAML
/// until the gap closes, so no observer prompt installs. This test documents
/// the intended decline semantics for when the clause is re-enabled.
#[ignore = "pending: G-SUSPEND-SELF-COST-ON-OPPONENTS-TURN — clause 2 stubbed; observer prompt does not install"]
#[test]
fn bt13_102_observer_decline_no_suspend_no_memory() {
    let (mut runner, keenan) = observer_runner();

    let opp = runner.place_on_field(1, "OPP-DIGI", Some(0));
    runner.fire_play_event_triggers(1, opp.index as usize, true, false);

    assert!(
        runner.pending_selection().is_some(),
        "the optional prompt must install before declining"
    );
    runner.decline_optional_trigger().expect("decline");
    runner.game.drain_effect_queue();

    assert!(
        !runner.game.players[0].battle_area[keenan.index as usize].is_suspended,
        "declining → the Tamer is NOT suspended"
    );
    assert_eq!(runner.memory(), 0, "declining → no memory gain");
}

/// While clause 2 is stubbed (engine gap `G-SUSPEND-SELF-COST-ON-OPPONENTS-TURN`),
/// an effect-play of a Digimon on the opponent's turn installs NO prompt and
/// gains NO memory — the [Opponent's Turn] observer is intentionally absent.
/// (Keeps the gap honest: if the clause were re-added without the engine fix,
/// this asserts the current stubbed state, and a future un-stub flips it.)
#[test]
fn bt13_102_clause2_stubbed_no_observer_prompt() {
    let (mut runner, keenan) = observer_runner();

    let opp = runner.place_on_field(1, "OPP-DIGI", Some(0));
    runner.fire_play_event_triggers(1, opp.index as usize, true, false);

    assert!(
        runner.pending_selection().is_none(),
        "clause 2 is stubbed (gap) — no [Opponent's Turn] suspend prompt installs"
    );
    assert!(
        !runner.game.players[0].battle_area[keenan.index as usize].is_suspended,
        "the Tamer stays unsuspended (clause stubbed)"
    );
    assert_eq!(runner.memory(), 0, "no memory gain (clause stubbed)");
}
