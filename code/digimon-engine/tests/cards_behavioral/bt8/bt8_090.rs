//! BT8-090 Kari Kamiya — Tamer, Yellow, Cost 4.
//!
//! # Card text (card image / cards.json — authoritative)
//!
//! - [Start of Your Turn] If you have 2 memory or less, set your memory to 3.
//! - [Your Turn] When a card is added to your security stack, you may suspend
//!   this Tamer to gain 1 memory.
//! - Security Effect [Security] Play this card without paying its memory cost.
//!
//! # Implemented slice (all three printed clauses)
//! - Memory floor, Security free-play, and the [Your Turn] own-security-added
//!   observer (suspend-self cost → gain 1 memory).
//!
//! # Patterns this test covers
//! - StartOfYourTurn memory floor (set_memory).
//! - OnSecurity play-self-free.
//! - on_added_to_security observer scoped to the controller's OWN security via
//!   TriggerSource::SecurityPlaced (fires only on adds to your stack), gated
//!   [Your Turn] via active_when, with an optional activation_cost suspend-self
//!   prerequisite (source_is_unsuspended) and gain_memory(1).
//! - DCGO crosscheck: BT8_090.cs OnAddSecurity →
//!   ActivateClass("Memory +1") with CanUseCondition = IsExistOnBattleArea +
//!   IsOwnerTurn + CanTriggerWhenAddSecurity(player == card.Owner), and
//!   CanActivateCondition = CanActivateSuspendCostEffect (unsuspended).
//!   ActivateCoroutine taps this Tamer then AddMemory(1).

use digimon_dsl::compiled::{CompiledClause, CompiledStep, CompiledTiming};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardSourceRef, StackPosition};

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT8-090")
        .expect("BT8-090 must load from embedded DSL pack")
        .memory(5)
        .start()
}

// ─── Structural: the three printed clauses ────────────────────────────────

#[test]
fn bt8_090_has_memory_floor_and_security_play_clauses() {
    let runner = runner();
    let card = runner
        .compiled_card("BT8-090")
        .expect("BT8-090 must be compiled");

    assert!(
        card.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::StartOfYourTurn)
        )),
        "BT8-090 must have the memory floor clause"
    );
    assert!(
        card.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSecurity)
        )),
        "BT8-090 must have the Security play clause"
    );
}

/// Structural: the [Your Turn] own-security-added observer must ship as an
/// optional on_added_to_security clause with an active_when gate and a
/// suspend-self activation cost.
#[test]
fn bt8_090_has_your_turn_security_added_observer_clause() {
    let runner = runner();
    let card = runner
        .compiled_card("BT8-090")
        .expect("BT8-090 must be compiled");

    let observer = card.effects.iter().find_map(|clause| match clause {
        CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnAddedToSecurity) => {
            Some(t)
        }
        _ => None,
    });
    let observer = observer.expect("BT8-090 must have an on_added_to_security observer clause");

    assert!(
        observer.optional,
        "the security-added clause is 'you may' — must be optional so PASS is exposed"
    );
    assert!(
        observer.active_when.is_some(),
        "[Your Turn] gate must be encoded via active_when"
    );
    assert!(
        observer.condition.is_some(),
        "must gate the suspend cost on source_is_unsuspended"
    );
    // The suspend activation cost is lifted out of the process body into the
    // EffectBuilder at lowering time, so it is not visible as a process step;
    // the behavioral tests below assert that accepting actually suspends Kari.
    // The remaining body step must gain exactly 1 memory.
    assert!(
        observer
            .process
            .iter()
            .any(|step| matches!(step, CompiledStep::GainMemory(1))),
        "the body must gain exactly 1 memory"
    );
}

// ─── [Start of Your Turn] memory floor ────────────────────────────────────

#[test]
fn bt8_090_start_of_turn_sets_memory_to_3_when_lte_2() {
    let filler = make_test_card("BT8-090-FILLER", "Filler");
    let mut runner = DebugRunner::builder()
        .dsl_card("BT8-090")
        .expect("BT8-090 must load")
        .add_card(filler)
        .deck(0, &["BT8-090-FILLER"])
        .deck(1, &["BT8-090-FILLER"])
        .memory(2)
        .start();

    runner.place_on_field(0, "BT8-090", None);
    runner.game.memory = 2;
    runner.end_turn();
    runner.end_turn();

    assert_eq!(runner.memory(), 3, "Kari should set memory to 3 at <=2");
}

// ─── [Your Turn] security-added observer ──────────────────────────────────
//
// A card added to the controller's OWN security stack on their turn must offer
// "you may suspend this Tamer to gain 1 memory". Accepting suspends Kari and
// gains exactly 1 memory.

/// Helper: place `count` deck-top cards on `player`'s own security through the
/// real game flow (`EffectContext::place_on_security` from `DeckTop`), which
/// fires `fire_on_place_security` exactly as an in-game recovery would.
fn add_own_deck_top_to_security(runner: &mut DebugRunner, player: u8, source: digimon_engine::card_source::CardHandle) {
    let mut ctx = EffectContext::new(&mut runner.game, source, None, 0);
    let placed = ctx.place_on_security(
        player,
        CardSourceRef::DeckTop(player),
        StackPosition::Top,
        false,
    );
    assert!(placed, "deck-top must place onto own security");
}

#[test]
fn bt8_090_security_added_offers_suspend_to_gain_memory_and_accepting_works() {
    let filler = make_test_card("BT8090-SA-A", "Sec Add A");
    let mut runner = DebugRunner::builder()
        .dsl_card("BT8-090")
        .expect("BT8-090 must load")
        .add_card(filler)
        .deck(0, &["BT8090-SA-A", "BT8090-SA-A"])
        .deck(1, &["BT8090-SA-A"])
        .memory(3)
        .start();

    // Kari on P0's field, unsuspended, on P0's turn ([Your Turn] satisfied).
    let kari = runner.place_on_field(0, "BT8-090", Some(0));
    assert!(
        !runner.game.players[0].battle_area[kari.index as usize].is_suspended,
        "precondition: Kari starts unsuspended"
    );
    runner.game.memory = 3;
    let memory_before = runner.memory();
    let kari_source = runner.game.players[0].battle_area[kari.index as usize]
        .top_card()
        .handle();

    add_own_deck_top_to_security(&mut runner, 0, kari_source);

    let view = runner
        .pending_selection_view()
        .expect("adding to own security on your turn must offer the suspend-to-gain choice");
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("accept the optional suspend cost");
    runner.auto_resolve().expect("resolve after accept");

    assert!(
        runner.game.players[0].battle_area[kari.index as usize].is_suspended,
        "accepting must suspend this Tamer (the activation cost)"
    );
    assert_eq!(
        runner.memory(),
        memory_before + 1,
        "accepting gains exactly 1 memory for the controller"
    );
}

#[test]
fn bt8_090_security_added_player_may_decline_leaving_unsuspended_and_memory_unchanged() {
    let filler = make_test_card("BT8090-SA-B", "Sec Add B");
    let mut runner = DebugRunner::builder()
        .dsl_card("BT8-090")
        .expect("BT8-090 must load")
        .add_card(filler)
        .deck(0, &["BT8090-SA-B", "BT8090-SA-B"])
        .deck(1, &["BT8090-SA-B"])
        .memory(3)
        .start();

    let kari = runner.place_on_field(0, "BT8-090", Some(0));
    runner.game.memory = 3;
    let memory_before = runner.memory();
    let kari_source = runner.game.players[0].battle_area[kari.index as usize]
        .top_card()
        .handle();

    add_own_deck_top_to_security(&mut runner, 0, kari_source);

    runner
        .pending_selection_view()
        .expect("must offer the suspend-to-gain choice");
    runner
        .decline_optional_trigger()
        .expect("decline the optional suspend cost");
    runner.auto_resolve().expect("resolve after decline");

    assert!(
        !runner.game.players[0].battle_area[kari.index as usize].is_suspended,
        "declining must leave this Tamer unsuspended"
    );
    assert_eq!(
        runner.memory(),
        memory_before,
        "declining must not gain memory"
    );
}

/// Negative: an add to the controller's own security on the OPPONENT's turn
/// must NOT fire ([Your Turn] gate).
#[test]
fn bt8_090_security_added_does_not_fire_on_opponents_turn() {
    let filler = make_test_card("BT8090-SA-C", "Sec Add C");
    let mut runner = DebugRunner::builder()
        .dsl_card("BT8-090")
        .expect("BT8-090 must load")
        .add_card(filler)
        .deck(0, &["BT8090-SA-C", "BT8090-SA-C"])
        .deck(1, &["BT8090-SA-C"])
        .memory(3)
        .start();

    let kari = runner.place_on_field(0, "BT8-090", Some(0));
    runner.end_turn(); // P0 ends → P1's turn (opponent's turn for Kari's controller)
    runner.game.memory = 3;
    let memory_before = runner.memory();
    let kari_source = runner.game.players[0].battle_area[kari.index as usize]
        .top_card()
        .handle();

    // Add a card to P0's own security on P1's turn.
    add_own_deck_top_to_security(&mut runner, 0, kari_source);

    assert!(
        runner.pending_selection().is_none(),
        "[Your Turn] gate must suppress the observer on the opponent's turn"
    );
    assert!(
        !runner.game.players[0].battle_area[kari.index as usize].is_suspended,
        "Kari must stay unsuspended when the observer does not fire"
    );
    assert_eq!(
        runner.memory(),
        memory_before,
        "no memory gain when the [Your Turn] gate blocks the observer"
    );
}

/// Negative: while Kari is already suspended, the suspend cost is unpayable, so
/// the observer must not offer the choice (source_is_unsuspended guard).
#[test]
fn bt8_090_security_added_does_not_offer_when_kari_already_suspended() {
    let filler = make_test_card("BT8090-SA-D", "Sec Add D");
    let mut runner = DebugRunner::builder()
        .dsl_card("BT8-090")
        .expect("BT8-090 must load")
        .add_card(filler)
        .deck(0, &["BT8090-SA-D", "BT8090-SA-D"])
        .deck(1, &["BT8090-SA-D"])
        .memory(3)
        .start();

    let kari = runner.place_on_field(0, "BT8-090", Some(0));
    runner.game.players[0].battle_area[kari.index as usize].is_suspended = true;
    runner.game.memory = 3;
    let memory_before = runner.memory();
    let kari_source = runner.game.players[0].battle_area[kari.index as usize]
        .top_card()
        .handle();

    add_own_deck_top_to_security(&mut runner, 0, kari_source);

    assert!(
        runner.pending_selection().is_none(),
        "already-suspended Kari cannot pay the suspend cost — no prompt"
    );
    assert_eq!(
        runner.memory(),
        memory_before,
        "no memory gain when the suspend cost is unpayable"
    );
}
