//! EX11-060 Arisa Kinosaki — Tamer, Yellow.
//! Traits: LIBERATOR.
//!
//! # Card text (cards.json — authoritative)
//!
//! - [Start of Your Turn] If you have 2 or less memory, set it to 3.
//! - [All Turns] When any of your Tokens or [Puppet] trait Digimon are
//!   deleted, by suspending this Tamer, <Draw 1> (Draw 1 card from your
//!   deck.). If deleted by <Overclock>, you may play 1 level 4 or lower
//!   [Puppet] trait Digimon card from your hand without paying the cost.
//! - Inherited (Security Effect) [Security] Play this card without paying
//!   the cost.
//!
//! # DCGO reference
//!
//! DCGO/Assets/Scripts/CardEffect/EX11/Yellow/EX11_060.cs
//! - "Start of Main" OnStartTurn: SetMemoryTo3TamerEffect (memory <= 2 -> 3).
//! - "Your Turn" OnDestroyedAnyone: optional ActivateClass. CanUse = this
//!   Tamer on battle area && a YOUR Token / [Puppet]-trait Digimon was just
//!   deleted. CanActivate additionally checks CanActivateSuspendCostEffect —
//!   false when this Tamer is already suspended. Activate suspends this Tamer
//!   (cost) -> Draw 1, then if the triggering effect's name == "Overclock"
//!   and hand has a level<=4 [Puppet] Digimon, optional SelectHandEffect
//!   maxCount 1 canNoSelect true -> play it free from hand.
//! - "Security Effect" SecuritySkill: PlaySelfTamerSecurityEffect.
//!
//! # Patterns this test covers
//! - Start-of-your-turn memory floor (positive + negative threshold).
//! - [Security] play-self-free from the security stack.
//! - [All Turns] on_any_deletion observer with deleted-object event context
//!   (event_target_owner / event_target_kind / event_target_trait_has).
//! - Track B activation_cost { suspend_self } visible cost gating the body.
//! - event_cause: overclock predicate gating the optional Puppet hand-play.
//! - No-approximations: the suspend-to-draw "may" and the Overclock optional
//!   Puppet play both surface through pending_selection.

use digimon_dsl::compiled::{
    CompiledClause, CompiledScope, CompiledTiming, CompiledTriggeredClause,
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, GamePhase, Keyword};
use digimon_engine::events::GameEvent;
use digimon_engine::{
    action::space::{
        encode_attack, EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_OVERCLOCK, FIELD_EFFECT_START,
    },
    card_data::CardData,
    permanent::PermanentHandle,
    replacement::ReplacementCause,
    selection::SelectionKind,
};

// ─── Start of Your Turn: memory floor ──────────────────────────────────

#[test]
fn ex11_060_start_of_turn_sets_memory_to_3_when_lte_2() {
    let filler = make_test_card("FILLER-EX11-060", "Filler");
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-060")
        .expect("EX11-060 YAML loads")
        .add_card(filler)
        .deck(0, &["FILLER-EX11-060"])
        .deck(1, &["FILLER-EX11-060"])
        .memory(2)
        .start();

    runner.place_on_field(0, "EX11-060", Some(0));
    runner.game.memory = 2;

    runner.end_turn();
    runner.end_turn();

    assert_eq!(
        runner.memory(),
        3,
        "Arisa sets memory to 3 at start of your turn when memory is 2 or less"
    );
}

#[test]
fn ex11_060_start_of_turn_does_not_lower_memory_above_2() {
    let filler = make_test_card("FILLER-EX11-060", "Filler");
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-060")
        .expect("EX11-060 YAML loads")
        .add_card(filler)
        .deck(0, &["FILLER-EX11-060"])
        .deck(1, &["FILLER-EX11-060"])
        .memory(5)
        .start();

    runner.place_on_field(0, "EX11-060", Some(0));
    runner.game.memory = 5;

    runner.end_turn();
    runner.end_turn();

    assert_eq!(
        runner.memory(),
        5,
        "Arisa must not set memory to 3 when memory is above the printed threshold"
    );
}

// ─── Security: play-self-free ──────────────────────────────────────────

#[test]
fn ex11_060_security_plays_itself_without_paying_cost() {
    let mut attacker = make_test_card("ATTACKER-EX11-060", "Attacker");
    attacker.card_kind = CardKind::Digimon;
    attacker.level = Some(4);
    attacker.dp = Some(9000);
    attacker.play_cost = 0;

    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-060")
        .expect("EX11-060 YAML loads")
        .add_card(attacker)
        .security(1, &["EX11-060"])
        .memory(10)
        .start();
    let attacker = runner.place_on_field(0, "ATTACKER-EX11-060", Some(0));

    runner.attack_player(attacker, 1, false);
    runner.auto_resolve().expect("resolve security play");

    assert!(runner.game.players[1]
        .battle_area
        .iter()
        .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "EX11-060"));
}

// ─── [All Turns] deletion observer: structural ─────────────────────────

#[test]
fn ex11_060_deletion_observer_is_optional_all_turns_on_any_deletion_clause() {
    let runner = DebugRunner::builder()
        .dsl_card("EX11-060")
        .expect("EX11-060 YAML loads")
        .memory(10)
        .start();
    let card = runner
        .compiled_card("EX11-060")
        .expect("EX11-060 compiled card present");

    let observer = find_deletion_observer(card)
        .expect("EX11-060 must ship an [All Turns] OnAnyDeletion observer");

    assert!(
        observer.when.contains(&CompiledTiming::OnAnyDeletion),
        "deletion observer must trigger on on_any_deletion"
    );
    assert_eq!(
        observer.scope,
        CompiledScope::FaceUp,
        "printed [All Turns] clause is a face-up effect of this Tamer"
    );
    assert!(
        observer.active_when.is_some(),
        "[All Turns] window must be encoded via active_when"
    );
    // The printed text carries no [Once Per Turn]; the suspend-this-Tamer
    // cost is the natural limiter. DCGO uses no OPT counter.
    assert!(
        !observer.once_per_turn,
        "printed text has no [Once Per Turn]; the suspend cost limits reuse"
    );
    assert!(
        observer.condition.is_some(),
        "observer must gate on the deleted-object event context"
    );
}

// ─── [All Turns] deletion observer: condition gating ───────────────────

#[test]
fn ex11_060_all_turns_draws_when_own_token_is_deleted() {
    let mut runner = arisa_deletion_runner();
    let arisa = runner.place_on_field(0, "EX11-060", Some(0));
    let token = place_own_token(&mut runner, "PUPPET-TOKEN");

    let hand_before = runner.hand_size(0);
    runner
        .game
        .delete_permanent_with_cause(token, ReplacementCause::OwnEffect);

    choose_arisa_activation(&mut runner, "own Token deletion offers Arisa activation");
    runner.auto_resolve().expect("finish Arisa Token branch");

    assert!(
        runner.game.player(0).battle_area[arisa.index as usize].is_suspended,
        "paying the visible cost suspends this Tamer"
    );
    assert_eq!(
        runner.hand_size(0),
        hand_before + 1,
        "own Token deletion should Draw 1 after the suspend cost"
    );
}

#[test]
fn ex11_060_all_turns_draws_when_own_puppet_is_deleted_by_non_overclock() {
    let mut runner = arisa_deletion_runner();
    let arisa = runner.place_on_field(0, "EX11-060", Some(0));
    let puppet = runner.place_on_field(0, "PUPPET-SAC", Some(0));

    runner
        .game
        .delete_permanent_with_cause(puppet, ReplacementCause::OwnEffect);

    choose_arisa_activation(&mut runner, "ordinary deletion offers Arisa activation");
    runner
        .auto_resolve()
        .expect("finish non-Overclock Arisa branch");

    let arisa_perm = &runner.game.player(0).battle_area[arisa.index as usize];
    assert!(
        arisa_perm.is_suspended,
        "Arisa's visible cost should suspend this Tamer"
    );
    assert!(
        runner
            .game
            .player(0)
            .hand
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "DRAW-FILLER-EX11-060"),
        "Arisa should Draw 1 after the suspend cost is paid"
    );
    assert!(
        !runner
            .game
            .player(0)
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "PUPPET-HAND"),
        "ordinary deletion must not expose the Overclock-only hand-play branch"
    );
    assert!(
        runner.pending_selection().is_none(),
        "ordinary deletion should resolve after suspend and draw"
    );
}

#[test]
fn ex11_060_all_turns_does_not_fire_for_own_non_puppet_digimon_deletion() {
    let mut runner = arisa_deletion_runner();
    runner.place_on_field(0, "EX11-060", Some(0));
    let non_puppet = runner.place_on_field(0, "NON-PUPPET-SAC", Some(0));

    let hand_before = runner.hand_size(0);
    runner
        .game
        .delete_permanent_with_cause(non_puppet, ReplacementCause::OwnEffect);

    assert!(
        runner.pending_selection().is_none(),
        "deleting your own non-Puppet Digimon must not offer Arisa activation"
    );
    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "non-Puppet Digimon deletion must not Draw"
    );
}

#[test]
fn ex11_060_all_turns_does_not_fire_for_opponent_puppet_deletion() {
    let mut runner = arisa_deletion_runner();
    runner.place_on_field(0, "EX11-060", Some(0));
    let opponent_puppet = runner.place_on_field(1, "PUPPET-SAC", Some(1));

    let hand_before = runner.hand_size(0);
    runner
        .game
        .delete_permanent_with_cause(opponent_puppet, ReplacementCause::OwnEffect);

    assert!(
        runner.pending_selection().is_none(),
        "an opponent's Puppet Digimon deletion must not offer your Arisa activation"
    );
    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "printed 'your Tokens or [Puppet] trait Digimon' excludes the opponent's board"
    );
}

// ─── [All Turns] deletion observer: suspend cost gating ────────────────

#[test]
fn ex11_060_all_turns_cannot_activate_when_arisa_already_suspended() {
    let mut runner = arisa_deletion_runner();
    let arisa = runner.place_on_field(0, "EX11-060", Some(0));
    let puppet = runner.place_on_field(0, "PUPPET-SAC", Some(0));

    // Pre-suspend Arisa: the printed cost "by suspending this Tamer" cannot
    // be paid, so the observer must not offer an activation (DCGO's
    // CanActivateSuspendCostEffect returns false).
    runner.game.players[0].battle_area[arisa.index as usize].is_suspended = true;

    let hand_before = runner.hand_size(0);
    runner
        .game
        .delete_permanent_with_cause(puppet, ReplacementCause::OwnEffect);

    assert!(
        runner.pending_selection().is_none(),
        "a suspended Arisa cannot pay the suspend cost, so no activation prompt appears"
    );
    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "no Draw when the suspend cost cannot be paid"
    );
}

#[test]
fn ex11_060_all_turns_suspend_to_draw_is_optional_and_may_be_declined() {
    let mut runner = arisa_deletion_runner();
    let arisa = runner.place_on_field(0, "EX11-060", Some(0));
    let puppet = runner.place_on_field(0, "PUPPET-SAC", Some(0));

    let hand_before = runner.hand_size(0);
    runner
        .game
        .delete_permanent_with_cause(puppet, ReplacementCause::OwnEffect);

    // No-approximations: the "by suspending this Tamer" choice must surface
    // as a real prompt; declining leaves Arisa unsuspended with no Draw.
    let view = runner
        .pending_selection_view()
        .expect("deletion must offer the suspend-to-draw choice");
    assert_eq!(view.kind, SelectionKind::EffectChoice);
    let decline_action = *view
        .valid_action_ids
        .last()
        .expect("decline action available");
    runner
        .execute_action(view.selecting_player, decline_action)
        .expect("decline Arisa activation");
    runner.auto_resolve().expect("resolve after decline");

    assert!(
        !runner.game.player(0).battle_area[arisa.index as usize].is_suspended,
        "declining the optional clause must leave Arisa unsuspended"
    );
    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "declining the optional clause must not Draw"
    );
}

// ─── [All Turns] deletion observer: Overclock rider ────────────────────

#[test]
fn ex11_060_all_turns_overclock_deletion_may_play_level_4_or_lower_puppet() {
    let mut runner = arisa_deletion_runner();
    let _arisa = runner.place_on_field(0, "EX11-060", Some(0));
    let overclock = runner.place_on_field(0, "PUPPET-OVERCLOCK", Some(0));
    let sacrifice = runner.place_on_field(0, "PUPPET-SAC", Some(0));
    runner.game.current_phase = GamePhase::EndOfTurnAction;

    let cp = runner.event_checkpoint();

    let overclock_action = encode_field_effect(overclock, FIELD_EFFECT_SLOT_FOR_OVERCLOCK);
    runner.game.decode_action(overclock_action, 0);
    runner
        .game
        .resolve_selection(0, encode_attack(0, sacrifice.index as u16))
        .expect("choose Puppet sacrifice for Overclock");

    choose_arisa_activation(&mut runner, "Overclock deletion offers Arisa activation");

    let hand_pick = runner
        .pending_selection_view()
        .expect("Overclock branch should offer optional Puppet hand-play");
    assert_eq!(hand_pick.kind, SelectionKind::Hand);
    assert!(
        hand_pick
            .valid_action_ids
            .iter()
            .any(|&action| action != digimon_engine::action::space::PASS),
        "hand-play branch should expose the level 4 Puppet candidate"
    );
    let play_action = hand_pick
        .valid_action_ids
        .iter()
        .copied()
        .find(|&action| action != digimon_engine::action::space::PASS)
        .expect("Puppet hand card action");
    runner
        .execute_action(hand_pick.selecting_player, play_action)
        .expect("choose level 4 Puppet from hand");
    runner
        .auto_resolve()
        .expect("finish Overclock Arisa branch and resumed attack");

    let arisa_perm = runner
        .game
        .player(0)
        .battle_area
        .iter()
        .find(|perm| perm.top_card().card_id(&runner.game.card_data) == "EX11-060")
        .expect("Arisa remains on field");
    assert!(
        arisa_perm.is_suspended,
        "Arisa's visible cost should suspend this Tamer"
    );
    assert!(
        runner
            .game
            .player(0)
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "PUPPET-HAND"),
        "Overclock branch should play the selected level 4 Puppet from hand"
    );

    // Event-log assertion for the cost-firing clause: the Overclock-only
    // hand-play emits a Play event for the level 4 Puppet.
    let plays = runner.events_of_kind(cp, |event| {
        matches!(event, GameEvent::Play { card_id, player, .. } if card_id == "PUPPET-HAND" && *player == 0)
    });
    assert_eq!(
        plays.len(),
        1,
        "the Overclock rider must emit exactly one free Puppet Play event"
    );
}

#[test]
fn ex11_060_all_turns_overclock_puppet_play_is_optional_and_may_be_declined() {
    let mut runner = arisa_deletion_runner();
    let _arisa = runner.place_on_field(0, "EX11-060", Some(0));
    let overclock = runner.place_on_field(0, "PUPPET-OVERCLOCK", Some(0));
    let sacrifice = runner.place_on_field(0, "PUPPET-SAC", Some(0));
    runner.game.current_phase = GamePhase::EndOfTurnAction;

    let overclock_action = encode_field_effect(overclock, FIELD_EFFECT_SLOT_FOR_OVERCLOCK);
    runner.game.decode_action(overclock_action, 0);
    runner
        .game
        .resolve_selection(0, encode_attack(0, sacrifice.index as u16))
        .expect("choose Puppet sacrifice for Overclock");

    choose_arisa_activation(&mut runner, "Overclock deletion offers Arisa activation");

    // No-approximations: the Overclock "you may play" Puppet pick is optional;
    // declining must leave the candidate in hand.
    let hand_pick = runner
        .pending_selection_view()
        .expect("Overclock branch should offer optional Puppet hand-play");
    assert_eq!(hand_pick.kind, SelectionKind::Hand);
    runner
        .execute_action(
            hand_pick.selecting_player,
            digimon_engine::action::space::PASS,
        )
        .expect("decline optional Puppet hand-play");
    runner
        .auto_resolve()
        .expect("finish Overclock branch after declining the Puppet play");

    assert!(
        runner
            .game
            .player(0)
            .hand
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "PUPPET-HAND"),
        "declining the optional Overclock Puppet play must leave it in hand"
    );
    assert!(
        !runner
            .game
            .player(0)
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "PUPPET-HAND"),
        "no Puppet enters the battle area when the optional play is declined"
    );
}

// ─── Helpers ───────────────────────────────────────────────────────────

fn arisa_deletion_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX11-060")
        .expect("EX11-060 YAML loads")
        .add_card(puppet_digimon("PUPPET-SAC", 4, 4000))
        .add_card(puppet_digimon("PUPPET-HAND", 4, 4000))
        .add_card(non_puppet_digimon("NON-PUPPET-SAC", 4, 4000))
        .add_card(overclock_puppet("PUPPET-OVERCLOCK"))
        .add_card(puppet_token("PUPPET-TOKEN"))
        .add_card(make_test_card("DRAW-FILLER-EX11-060", "Draw Filler"))
        .hand(0, &["PUPPET-HAND"])
        .deck(0, &["DRAW-FILLER-EX11-060"])
        .security(1, &["DRAW-FILLER-EX11-060"])
        .start()
}

fn puppet_digimon(id: &str, level: u8, dp: i32) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Yellow];
    card.level = Some(level);
    card.dp = Some(dp);
    card.play_cost = 4;
    card.traits = vec!["Puppet".to_string()];
    card
}

fn non_puppet_digimon(id: &str, level: u8, dp: i32) -> CardData {
    let mut card = puppet_digimon(id, level, dp);
    card.traits = vec!["Beast".to_string()];
    card
}

fn puppet_token(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Token;
    card.colors = vec![CardColor::Yellow];
    card.level = None;
    card.dp = Some(3000);
    card.play_cost = 0;
    card.traits = Vec::new();
    card
}

fn overclock_puppet(id: &str) -> CardData {
    let mut card = puppet_digimon(id, 4, 7000);
    card.keywords = vec![Keyword::Overclock];
    card.effect_text =
        "＜Overclock ([Puppet] Trait)＞ (At the end of your turn, by deleting 1 of your Tokens or other [Puppet] trait Digimon, this Digimon attacks a player without suspending.)"
            .to_string();
    card
}

/// Push a Token directly onto player 0's battle area. Tokens have no card
/// data path through `place_on_field`, so the test wires the permanent and
/// then drives a real deletion through the engine to fire the observer.
fn place_own_token(runner: &mut DebugRunner, card_id: &str) -> PermanentHandle {
    use digimon_engine::card_source::CardSource;
    use digimon_engine::permanent::Permanent;

    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|card| card.card_id == card_id)
        .expect("token card data registered");
    let card_index = runner.game.next_card_index();
    let mut source = CardSource::new_token(data_idx, 0, card_index);
    source.card_index = card_index;
    let turn = runner.game.turn_count;
    runner.game.players[0]
        .battle_area
        .push(Permanent::new(source, turn));
    PermanentHandle {
        player: 0,
        index: (runner.game.players[0].battle_area.len() - 1) as u8,
    }
}

fn encode_field_effect(handle: PermanentHandle, effect_slot: u16) -> u16 {
    FIELD_EFFECT_START + handle.index as u16 * EFFECTS_PER_PERMANENT + effect_slot
}

fn choose_arisa_activation(runner: &mut DebugRunner, label: &str) {
    let view = runner.pending_selection_view().expect(label);
    assert_eq!(view.kind, SelectionKind::EffectChoice);
    let action = view.valid_action_ids[0];
    runner
        .execute_action(view.selecting_player, action)
        .expect("accept Arisa activation");
}

fn find_deletion_observer(
    card: &digimon_dsl::compiled::CompiledCard,
) -> Option<&CompiledTriggeredClause> {
    card.effects.iter().find_map(|clause| match clause {
        CompiledClause::Triggered(triggered)
            if triggered.when.contains(&CompiledTiming::OnAnyDeletion) =>
        {
            Some(triggered)
        }
        _ => None,
    })
}
