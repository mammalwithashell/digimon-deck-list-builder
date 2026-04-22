//! Phase 8 Task 3 — Delay Option flow tests.
//!
//! A Delay Option plays like a Standard Option (cost + `OptionMain` body
//! drain), but instead of trashing on dispose it **parks on the owner's
//! battle_area** as a Permanent with `OptionState::Delayed`. At the end of
//! the scheduled turn the engine fires `DelayEffect` and trashes the
//! permanent via the Phase 7 replacement framework.

use std::sync::{Arc, Mutex};

use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind, DelayTrigger};
use digimon_engine::permanent::OptionState;
use digimon_engine::selection::OptionPlayResult;

// ─── Inline test-effect helpers ──────────────────────────────────────

/// Delay Option body that fires `EndOfYourNextTurn`; records a witness
/// counter when its `DelayEffect` body fires.
struct DelayNextTurnWitness(Arc<Mutex<u32>>);
impl CardEffect for DelayNextTurnWitness {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let slot = self.0.clone();
        vec![Effect::on_play(card)
            .name("Delay next turn")
            .delay(DelayTrigger::EndOfYourNextTurn)
            .process(move |_ctx| {
                *slot.lock().unwrap() += 1;
            })
            .build()]
    }
}

/// Delay Option body that fires `EndOfThisTurn`.
struct DelayThisTurnWitness(Arc<Mutex<u32>>);
impl CardEffect for DelayThisTurnWitness {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let slot = self.0.clone();
        vec![Effect::on_play(card)
            .name("Delay this turn")
            .delay(DelayTrigger::EndOfThisTurn)
            .process(move |_ctx| {
                *slot.lock().unwrap() += 1;
            })
            .build()]
    }
}

/// Delay Option body with no side-effects — used for field-park shape and
/// attackability tests. Uses `EndOfYourNextTurn`.
struct DelayNoopNextTurn;
impl CardEffect for DelayNoopNextTurn {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("Delay noop")
            .delay(DelayTrigger::EndOfYourNextTurn)
            .process(|_ctx| {})
            .build()]
    }
}

/// Composite Delay + `WhenWouldBeDeleted` cancel replacement. Lets the
/// end-of-turn trash be intercepted by Phase 7's replacement window.
struct DelayPlusCancel(Arc<Mutex<u32>>);
impl CardEffect for DelayPlusCancel {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let slot = self.0.clone();
        vec![
            Effect::on_play(card)
                .name("Delay + cancel")
                .delay(DelayTrigger::EndOfYourNextTurn)
                .process(move |_ctx| {
                    *slot.lock().unwrap() += 1;
                })
                .build(),
            Effect::when_would_be_deleted(card)
                .name("Cancel delayed trash")
                .replacement_process(|rctx| {
                    rctx.cancel();
                })
                .build(),
        ]
    }
}

// ─── Fixture helpers ──────────────────────────────────────────────────

fn option_card(card_id: &str, cost: u16, color: CardColor) -> digimon_engine::CardData {
    let mut cd = make_test_card(card_id, card_id);
    cd.card_kind = CardKind::Option;
    cd.level = None;
    cd.dp = None;
    cd.play_cost = cost;
    cd.colors = vec![color];
    cd
}

fn digimon_card(card_id: &str, color: CardColor) -> digimon_engine::CardData {
    let mut cd = make_test_card(card_id, card_id);
    cd.colors = vec![color];
    cd
}

fn advance_to_main(r: &mut DebugRunner) {
    r.game.enter_main_phase();
}

// ─── Tests ─────────────────────────────────────────────────────────────

/// Test 1: after playing a Delay, the pending_option is cleared, the card
/// sits on the owner's battle_area as a Permanent in `OptionState::Delayed`
/// with `trash_on_turn` set to `turn_count + 2` (EndOfYourNextTurn on the
/// owner's own turn).
#[test]
fn delay_parks_on_field_with_delayed_state() {
    let mut r = DebugRunner::builder()
        .add_card(option_card("DELAY-PARK", 0, CardColor::Red))
        .add_card(digimon_card("RED-MATCH", CardColor::Red))
        .hand(0, &["DELAY-PARK"])
        .memory(0)
        .start();
    r.register_effect("DELAY-PARK", Arc::new(DelayNoopNextTurn));
    r.place_on_field(0, "RED-MATCH", Some(0));
    advance_to_main(&mut r);

    let turn_before = r.game.turn_count;
    let battle_before = r.battle_area_size(0);

    let result = r.game.play_option_from_hand(0, 0);

    assert_eq!(result, OptionPlayResult::Trashed);
    assert!(r.game.pending_option.is_none(), "pending_option cleared");
    assert_eq!(r.hand_size(0), 0, "card left hand");
    assert_eq!(r.trash_size(0), 0, "delay does NOT trash — it parks on field");
    assert_eq!(
        r.battle_area_size(0),
        battle_before + 1,
        "delayed option parks as a permanent on the battle_area"
    );
    // The delayed permanent is the newest addition.
    let idx = r.battle_area_size(0) - 1;
    let perm = &r.game.player(0).battle_area[idx];
    match perm.option_state {
        OptionState::Delayed {
            owner,
            trash_on_turn,
        } => {
            assert_eq!(owner, 0, "delayed state records the controller");
            assert_eq!(
                trash_on_turn,
                turn_before + 2,
                "EndOfYourNextTurn on own turn = turn + 2 (skip opponent)"
            );
        }
        other => panic!("expected Delayed option_state, got {:?}", other),
    }
}

/// Test 2: a Delay played on P0's turn 1 fires at the end of P0's next
/// turn (turn 3), not at P1's turn 2, and trashes afterward.
#[test]
fn delay_end_of_your_next_turn_fires_correctly() {
    let witness = Arc::new(Mutex::new(0u32));

    let mut r = DebugRunner::builder()
        .add_card(option_card("DELAY-NXT", 0, CardColor::Red))
        .add_card(digimon_card("RED-MATCH", CardColor::Red))
        .add_card(digimon_card("FILLER", CardColor::Red))
        .hand(0, &["DELAY-NXT"])
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 5])
        .memory(0)
        .start();
    r.register_effect("DELAY-NXT", Arc::new(DelayNextTurnWitness(witness.clone())));
    r.place_on_field(0, "RED-MATCH", Some(0));
    advance_to_main(&mut r);

    let start_turn = r.game.turn_count;
    let result = r.game.play_option_from_hand(0, 0);
    assert_eq!(result, OptionPlayResult::Trashed);
    assert_eq!(*witness.lock().unwrap(), 0, "body not fired yet");
    assert_eq!(r.battle_area_size(0), 2, "RED-MATCH + delayed option");

    // End P0's current turn — delay should NOT fire (it's scheduled for
    // P0's NEXT turn, not this one).
    r.end_turn();
    assert_eq!(*witness.lock().unwrap(), 0, "not fired at end of this turn");
    let delayed_still_there = r
        .game
        .player(0)
        .battle_area
        .iter()
        .any(|p| matches!(p.option_state, OptionState::Delayed { .. }));
    assert!(delayed_still_there, "still parked after opponent-turn rollover");

    // Advance to Main on P1's turn, then end it. Still no fire.
    r.game.enter_main_phase();
    r.end_turn();
    assert_eq!(*witness.lock().unwrap(), 0, "not fired at end of P1's turn");

    // Now on P0's next turn (turn = start_turn + 2). End it — fires here.
    assert_eq!(r.game.turn_count, start_turn + 2);
    assert_eq!(r.game.turn_player(), 0);
    r.game.enter_main_phase();
    r.end_turn();
    assert_eq!(*witness.lock().unwrap(), 1, "DelayEffect fired at end of own next turn");

    // Card trashed.
    let delayed_gone = !r
        .game
        .player(0)
        .battle_area
        .iter()
        .any(|p| matches!(p.option_state, OptionState::Delayed { .. }));
    assert!(delayed_gone, "delayed permanent left the battle_area");
    assert_eq!(r.trash_size(0), 1, "delayed option is now in trash");
}

/// Test 3: a Delay with `EndOfThisTurn` fires and trashes at the end of
/// the same turn it was played.
#[test]
fn delay_end_of_this_turn_fires_same_turn() {
    let witness = Arc::new(Mutex::new(0u32));

    let mut r = DebugRunner::builder()
        .add_card(option_card("DELAY-THIS", 0, CardColor::Red))
        .add_card(digimon_card("RED-MATCH", CardColor::Red))
        .hand(0, &["DELAY-THIS"])
        .memory(0)
        .start();
    r.register_effect(
        "DELAY-THIS",
        Arc::new(DelayThisTurnWitness(witness.clone())),
    );
    r.place_on_field(0, "RED-MATCH", Some(0));
    advance_to_main(&mut r);

    let result = r.game.play_option_from_hand(0, 0);
    assert_eq!(result, OptionPlayResult::Trashed);
    assert_eq!(*witness.lock().unwrap(), 0, "body not fired yet");

    r.end_turn();

    assert_eq!(*witness.lock().unwrap(), 1, "DelayEffect fired at end of this turn");
    let delayed_gone = !r
        .game
        .player(0)
        .battle_area
        .iter()
        .any(|p| matches!(p.option_state, OptionState::Delayed { .. }));
    assert!(delayed_gone);
    assert_eq!(r.trash_size(0), 1);
}

/// Test 4: a Delayed permanent is not a legal attack target. An
/// opponent's attacker cannot target it, and the action mask does not emit
/// an attack bit for it.
#[test]
fn delayed_permanent_is_not_attackable() {
    use digimon_engine::action::mask::build_action_mask;
    use digimon_engine::action::space::encode_attack;

    let mut r = DebugRunner::builder()
        .add_card(option_card("DELAY-TGT", 0, CardColor::Red))
        .add_card(digimon_card("RED-MATCH", CardColor::Red))
        .add_card(digimon_card("P1-ATKR", CardColor::Red))
        .hand(0, &["DELAY-TGT"])
        .memory(0)
        .start();
    r.register_effect("DELAY-TGT", Arc::new(DelayNoopNextTurn));
    r.place_on_field(0, "RED-MATCH", Some(0));
    advance_to_main(&mut r);
    let result = r.game.play_option_from_hand(0, 0);
    assert_eq!(result, OptionPlayResult::Trashed);

    // Find the delayed field index (latest push).
    let delayed_idx = r.game.player(0).battle_area.len() - 1;
    assert!(matches!(
        r.game.player(0).battle_area[delayed_idx].option_state,
        OptionState::Delayed { .. }
    ));

    r.end_turn(); // → P1's turn.

    // Place P1 attacker that's eligible to attack (turn 0 so no sickness).
    let attacker = r.place_on_field(1, "P1-ATKR", Some(0));
    r.game.enter_main_phase();

    // Mask: from P1's perspective, no attack bit for attacker→delayed_idx.
    let mask = build_action_mask(&r.game, 1);
    let bit = encode_attack(attacker.index as u16, delayed_idx as u16) as usize;
    assert_eq!(
        mask[bit], 0.0,
        "attack bit for delayed target must NOT be set in the mask"
    );

    // Direct call: attack rejected as Invalid.
    let delayed_handle = r.perm_handle(0, delayed_idx);
    let attack = r.game.attack_digimon(attacker, delayed_handle, false);
    assert_eq!(attack, digimon_engine::combat::AttackResult::Invalid);
}

/// Test 5: delayed permanents consume field slots. With the field already
/// holding `field_slots - 1` digimon, playing a Delay fills the last
/// slot; after that, another Digimon cannot be played from hand (field
/// full).
#[test]
fn delayed_permanent_counts_against_field_slots() {
    let field_slots = 14;
    // Pre-fill P0 field with field_slots - 1 Digimon + 1 open slot.
    let mut builder = DebugRunner::builder()
        .add_card(option_card("DELAY-SLOT", 0, CardColor::Red))
        .add_card(digimon_card("RED-MATCH", CardColor::Red))
        .add_card(digimon_card("EXTRA", CardColor::Red))
        .hand(0, &["DELAY-SLOT", "EXTRA"])
        .memory(10);
    builder = builder.add_card(digimon_card("FILLER", CardColor::Red));
    let mut r = builder.start();
    r.register_effect("DELAY-SLOT", Arc::new(DelayNoopNextTurn));
    // Seed one digimon that we'll keep "around" (satisfies color match).
    r.place_on_field(0, "RED-MATCH", Some(0));
    // Now fill up to `field_slots - 1` total (we already have 1 — add 12 more).
    for _ in 0..(field_slots - 2) {
        r.place_on_field(0, "FILLER", Some(0));
    }
    assert_eq!(r.battle_area_size(0), field_slots - 1);

    advance_to_main(&mut r);

    let result = r.game.play_option_from_hand(0, 0); // DELAY-SLOT at hand[0]
    assert_eq!(result, OptionPlayResult::Trashed);
    assert_eq!(
        r.battle_area_size(0),
        field_slots,
        "delayed option fills the last field slot"
    );

    // Remaining hand: "EXTRA" — attempt to play a Digimon from hand. With
    // the field full it must reject.
    let before = r.battle_area_size(0);
    let outcome = r.game.play_from_hand(0, 0); // now hand[0] == "EXTRA"
    assert_eq!(outcome, None, "field is full — Digimon play rejected");
    assert_eq!(r.battle_area_size(0), before, "no field growth on reject");
}

/// Test 6: a `WhenWouldBeDeleted` cancel replacement blocks the end-of-turn
/// trash of a Delayed permanent. The `DelayEffect` body still fires (it
/// runs BEFORE the trash), but the permanent remains on the field after
/// the delete path is replaced.
#[test]
fn delay_fires_observer_wwbt_at_end_of_turn_trash() {
    let witness = Arc::new(Mutex::new(0u32));

    let mut r = DebugRunner::builder()
        .add_card(option_card("DELAY-WWBT", 0, CardColor::Red))
        .add_card(digimon_card("RED-MATCH", CardColor::Red))
        .add_card(digimon_card("FILLER", CardColor::Red))
        .hand(0, &["DELAY-WWBT"])
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 5])
        .memory(0)
        .start();
    r.register_effect("DELAY-WWBT", Arc::new(DelayPlusCancel(witness.clone())));
    r.place_on_field(0, "RED-MATCH", Some(0));
    advance_to_main(&mut r);

    let result = r.game.play_option_from_hand(0, 0);
    assert_eq!(result, OptionPlayResult::Trashed);

    // End this turn (EndOfYourNextTurn — not yet); opp turn; end opp turn;
    // then P0 next turn; end it — that's when we fire.
    r.end_turn();
    r.game.enter_main_phase();
    r.end_turn();
    r.game.enter_main_phase();
    assert_eq!(r.game.turn_player(), 0);
    r.end_turn();

    assert_eq!(*witness.lock().unwrap(), 1, "DelayEffect body fired");

    // Replacement cancelled the delete — permanent is still on the field.
    let still_delayed = r
        .game
        .player(0)
        .battle_area
        .iter()
        .any(|p| matches!(p.option_state, OptionState::Delayed { .. }));
    assert!(
        still_delayed,
        "WhenWouldBeDeleted cancel kept the delayed permanent on the field"
    );
    assert_eq!(r.trash_size(0), 0, "not trashed — delete replaced");
}
