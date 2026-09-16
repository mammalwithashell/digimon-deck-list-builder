//! BT3-096 Mimi Tachikawa — Tamer, Purple, Cost 2. Traits: (none).
//!
//! # Card text (card image / official DB — authoritative)
//!
//! [All Turns] When a player uses an Option card, you may suspend this Tamer
//! to gain 1 memory.
//!
//! Inherited (Security):
//! Security Effect [Security] Play this card without paying its memory cost.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT3/Purple/BT3_096.cs
//!   - `EffectTiming.OnUseOption` ActivateClass, isOptional = true;
//!     CanUseCondition = IsExistOnBattleArea + `CanTriggerWhenUseOption(…, null,
//!     null, card)` (ANY player's Option use — no owner filter);
//!     CanActivateCondition = `CanActivateSuspendCostEffect(card)` (Mimi must
//!     be unsuspended); ActivateCoroutine = SuspendPermanentsClass(self).Tap()
//!     then `card.Owner.AddMemory(1)`.
//!   - SecuritySkill → `PlaySelfTamerSecurityEffect`.
//!   - ORDER: `UseOptionClass.UseOption` STACKS the OnUseOption skill infos
//!     and runs the `OptionSkill` inline → Mimi resolves AFTER the used
//!     Option's [Main] (official Q&A: "It can be activated after activating
//!     the used Option card's [Main] effect."). The engine's
//!     `Game::on_use_option_armed` / `OptionResolutionPhase::OnUseOptionDrain`
//!     deferral was added for this card.
//!
//! # Patterns this test covers
//! - B3 trigger-on-event tamer (`when: on_use_option`, [All Turns]) —
//!   G-DSL-ON-USE-OPTION-TIMING (the timing widened for this card)
//! - E2 optional suspend-as-cost (`activation_cost: { suspend_self: true }`)
//!   with explicit accept AND decline
//! - Negative gate: an already-suspended Mimi is never offered
//! - Tamer [Security] play-self (`play_from_security`)

use digimon_dsl::compiled::{CompiledCardKind, CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::CardColor;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::OptionPlayResult;

const CARD_ID: &str = "BT3-096";
/// ST6-15 Death Claw — Purple, cost 1, "[Main] You may delete 1 of your
/// Digimon to delete 1 opponent level 4 or lower Digimon." A cheap purple
/// Option whose [Main] is fully declinable — the Option-use trigger driver.
const OPTION_ID: &str = "ST6-15";

fn purple_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.colors = vec![CardColor::Purple];
    c
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT3-096 must load from the embedded DSL pack")
        .dsl_card(OPTION_ID)
        .expect("ST6-15 must load from the embedded DSL pack")
        .add_card(purple_digimon("FILL"))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
}

fn mimi_handle(runner: &DebugRunner) -> PermanentHandle {
    let idx = runner.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID)
        .expect("Mimi on P0's field");
    runner.perm_handle(0, idx)
}

fn is_suspended(runner: &DebugRunner, h: PermanentHandle) -> bool {
    runner.game.players[h.player as usize].battle_area[h.index as usize].is_suspended
}

/// Accept Mimi's optional suspend-cost prompt (the non-PASS action).
fn accept_prompt(runner: &mut DebugRunner) {
    let view = runner
        .pending_selection_view()
        .expect("Mimi's optional suspend-cost prompt must be parked");
    assert!(view.is_optional, "'you may suspend' must expose PASS");
    let accept = *view
        .valid_action_ids
        .iter()
        .find(|&&a| a != PASS)
        .expect("an accept action exists");
    runner
        .execute_action(view.selecting_player, accept)
        .expect("accept the suspend cost");
}

// ─── Section 1: structural ───────────────────────────────────────────────────

#[test]
fn bt3_096_metadata_and_clauses() {
    let runner = base().start();
    let compiled = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(compiled.kind, CompiledCardKind::Tamer);
    assert_eq!(compiled.cost, Some(2));

    let triggered: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(triggered.len(), 2, "on_use_option + on_security");

    let use_opt = triggered
        .iter()
        .find(|t| t.when == vec![CompiledTiming::OnUseOption])
        .expect("on_use_option clause");
    assert!(use_opt.optional, "'you may suspend this Tamer' is optional");
    assert!(!use_opt.once_per_turn, "no [Once Per Turn] printed — the suspend cost self-limits");
    assert_eq!(use_opt.scope, CompiledScope::FaceUp);

    let sec = triggered
        .iter()
        .find(|t| t.when == vec![CompiledTiming::OnSecurity])
        .expect("on_security clause");
    assert!(!sec.optional);
}

// ─── Section 2: own Option use ───────────────────────────────────────────────

#[test]
fn bt3_096_own_option_use_offers_suspend_then_gains_one_memory() {
    let mut runner = base().hand(0, &[OPTION_ID]).memory(5).start();
    runner.place_on_field(0, CARD_ID, Some(0));
    let mimi = mimi_handle(&runner);
    assert!(!is_suspended(&runner, mimi));

    // Cost 1 is paid first (5 → 4); Death Claw's [Main] self-skips (P0 has no
    // Digimon to offer for the self-delete); Mimi's trigger then parks.
    assert_eq!(runner.game.play_option_from_hand(0, 0), OptionPlayResult::Pending);
    assert_eq!(runner.memory(), 4, "the Option's use cost was paid before the trigger");

    accept_prompt(&mut runner);
    assert!(is_suspended(&runner, mimi), "Mimi suspends as the cost");
    assert_eq!(runner.memory(), 5, "gained 1 memory");

    // Nothing else is parked — the Option was already trashed after Mimi.
    let _ = runner.auto_resolve();
    assert!(runner.pending_selection().is_none());
}

#[test]
fn bt3_096_declining_leaves_mimi_unsuspended_and_memory_unchanged() {
    let mut runner = base().hand(0, &[OPTION_ID]).memory(5).start();
    runner.place_on_field(0, CARD_ID, Some(0));
    let mimi = mimi_handle(&runner);

    assert_eq!(runner.game.play_option_from_hand(0, 0), OptionPlayResult::Pending);
    let view = runner.pending_selection_view().expect("prompt parked");
    assert!(view.is_optional);
    runner
        .execute_action(view.selecting_player, PASS)
        .expect("decline");
    let _ = runner.auto_resolve();

    assert!(!is_suspended(&runner, mimi), "declining skips the suspend");
    assert_eq!(runner.memory(), 4, "only the Option's cost was paid; no +1");
}

#[test]
fn bt3_096_suspended_mimi_is_not_offered() {
    let mut runner = base().hand(0, &[OPTION_ID]).memory(5).start();
    let idx = runner.place_on_field(0, CARD_ID, Some(0)).index as usize;
    runner.game.players[0].battle_area[idx].is_suspended = true;

    let result = runner.game.play_option_from_hand(0, 0);
    // Death Claw's own optional [Main] may park; Mimi's prompt must NOT be the
    // parked selection (it would be an accept/decline over Mimi herself).
    if result == OptionPlayResult::Pending {
        let view = runner.pending_selection_view().expect("parked");
        assert_ne!(
            view.kind,
            digimon_engine::selection::SelectionKind::TriggerOrder,
            "a suspended Mimi cannot pay the cost — no trigger prompt (DCGO CanActivateSuspendCostEffect)"
        );
        let _ = runner.auto_resolve();
    }
    assert_eq!(runner.memory(), 4, "no memory gained");
}

// ─── Section 3: [All Turns] — the OPPONENT's Option use also triggers ────────

#[test]
fn bt3_096_opponent_option_use_also_triggers_on_their_turn() {
    let mut runner = base().hand(1, &[OPTION_ID]).memory(5).start();
    runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "FILL", Some(0)); // purple permanent → P1 meets the colour requirement
    let mimi = mimi_handle(&runner);

    runner.end_turn();
    assert_eq!(runner.game.turn_player(), 1);
    runner.game.set_memory(5); // P1's perspective

    assert_eq!(runner.game.play_option_from_hand(1, 0), OptionPlayResult::Pending);
    let after_cost = runner.memory();
    assert_eq!(after_cost, 4, "P1 paid the Option's cost");

    // ORDER (DCGO `UseOptionClass.UseOption` stacks OnUseOption and runs the
    // OptionSkill inline; official BT3-096 Q&A: "It can be activated after
    // activating the used Option card's [Main] effect."): Death Claw's own
    // [Main] resolves FIRST — P1 (who has a FILL Digimon) is offered its
    // optional self-delete pick — and only then does Mimi's trigger park.
    let body = runner
        .pending_selection_view()
        .expect("Death Claw's optional [Main] pick parks first");
    assert_eq!(body.selecting_player, 1, "the Option's own body belongs to its user (P1)");
    assert!(body.is_optional, "'you may delete 1 of your Digimon' exposes PASS");
    runner.execute_action(1, PASS).expect("P1 declines the self-delete");
    assert!(!is_suspended(&runner, mimi), "Mimi has not been asked yet");

    let view = runner
        .pending_selection_view()
        .expect("Mimi's prompt parks on P1's turn, after the Option's [Main]");
    assert_eq!(view.selecting_player, 0, "Mimi's controller (P0) makes the choice");
    accept_prompt(&mut runner);

    assert!(is_suspended(&runner, mimi), "Mimi suspends on the opponent's turn");
    // Memory is stored from the turn player's (P1's) perspective: P0 gaining 1
    // moves the gauge by -1.
    assert_eq!(runner.memory(), after_cost - 1, "P0 (not the turn player) gained 1 memory");
    let _ = runner.auto_resolve();
}

// ─── Section 4: [Security] play without paying the cost ──────────────────────

#[test]
fn bt3_096_security_plays_itself_free() {
    let mut attacker = make_test_card("ATK", "ATK");
    attacker.dp = Some(5000);
    let mut runner = base()
        .add_card(attacker)
        .security(1, &[CARD_ID])
        .memory(3)
        .start();
    let atk = runner.place_on_field(0, "ATK", Some(0));
    let memory_before = runner.memory();

    let _ = runner.attack_player(atk, 1, false);
    let _ = runner.auto_resolve();

    assert_eq!(runner.security_count(1), 0);
    assert!(
        runner.game.players[1]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID),
        "Mimi was played into P1's battle area by her [Security] effect"
    );
    assert_eq!(runner.memory(), memory_before, "played without paying the cost");
}
