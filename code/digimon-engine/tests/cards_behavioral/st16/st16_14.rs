//! ST16-14 Matt Ishida — Tamer, Cost 4. Colors: Purple.
//!
//! # Card text (data/card_bundles/ST16-14.md / card image — verbatim)
//! [Start of Your Turn] If you have 2 or fewer memory, set it to 3.
//! [All Turns] When one of your effects trashes a card in your hand, by
//!   suspending this Tamer, gain 1 memory.
//! Inherited: [Security] Play this card without paying the cost.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/ST16/Purple/ST16_14.cs
//!
//! # Patterns
//! - B (Tamer start-of-turn memory setter) — sister to ST18-14/ST19-14's
//!   opening clause.
//! - D (conditional gate: memory_lte)
//! - E2 OPT-adjacent: optional activatable ability with an activation_cost
//!   (suspend_self), gated on `on_discard_hand` + `event_discard_player: you`
//!   + `event_caused_by_own_effect: true` (G-ENGINE-ON-DISCARD-HAND).
//! - F (activation cost firing via suspend_self_as_cost)
//! - G3 inherited [Security] play-self

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{CompiledClause, CompiledPredicate, CompiledTiming};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "ST16-14";

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Red];
    c.level = Some(3);
    c.dp = Some(2000);
    c.play_cost = 3;
    c
}

fn hand_card(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Red];
    c.level = Some(3);
    c.dp = Some(2000);
    c.play_cost = 3;
    c
}

/// Inline DSL fixture: an `on_play` trigger that unconditionally trashes 1
/// card from `OWNER`'s hand via `select_hand` (mandatory) +
/// `trash_from_hand_by_index`. This is the "one of YOUR effects trashes a
/// card in your hand" causing effect DCGO gates on
/// (`CardEffectCommons.CanTriggerOnTrashHand`'s `cardEffect.EffectSourceCard.
/// Owner` + the discarded card's `Owner`). Category 2 (hypothetical-spec
/// scaffold to drive ST16-14's observer) per RUST_DSL_TEST_API.md §10 — no
/// shipping card of this exact shape exists to reuse.
///
/// `trash_own_hand: true` -> trashes the CONTROLLER's own hand (own-hand
/// case). `trash_own_hand: false` -> trashes the OPPONENT's hand (used for
/// the "your own effect trashes the OPPONENT's hand" negative case).
fn hand_trash_trigger_yaml(trash_own_hand: bool) -> String {
    let of = if trash_own_hand { "you" } else { "opponent" };
    format!(
        r#"
card: DSL-HANDTRASH-001
name: Hand Trash Trigger
kind: digimon
level: 3
color: [red]
cost: 3
dp: 2000
effects:
  - when: on_play
    process:
      - select_hand:
          of: {of}
          bind_as: victim
          filter: {{}}
          prompt: "Trash 1 hand card"
      - trash_from_hand_by_index: {{ of: {of}, hand_index: victim }}
"#
    )
}

/// Resolves the causing trigger's own mandatory `select_hand` cost prompt
/// (`SelectionKind::Hand`, installed by `hand_trash_trigger_yaml`'s
/// `select_hand` step) by picking its first valid action. This must run
/// BEFORE Matt's `on_discard_hand` observer can install its own
/// `TriggerOrder` activation prompt, since the hand-trash (the event Matt
/// observes) hasn't happened yet while the causing effect's own selection
/// is still pending. No-op if no selection is pending (defensive; keeps
/// call sites simple across the own-hand/opponent-hand fixture variants).
fn resolve_own_hand_trash_cost(runner: &mut DebugRunner) {
    if let Some(pending) = runner.pending_selection_view() {
        if pending.kind == SelectionKind::Hand {
            let action_id = pending
                .valid_action_ids
                .first()
                .copied()
                .expect("hand selection must offer at least one action");
            runner
                .execute_action(pending.selecting_player, action_id)
                .expect("resolve the causing effect's own hand-trash selection");
        }
    }
}

fn matt_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST16-14 YAML loads")
        .add_card(filler("FILLER"))
        .deck(0, &["FILLER"; 10])
        .deck(1, &["FILLER"; 10])
        .memory(10)
        .start()
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn st16_14_compiles_as_purple_tamer_cost_4() {
    let runner = matt_runner();
    let card = runner.compiled_card(CARD_ID).expect("ST16-14 compiled");
    assert_eq!(card.cost, Some(4));
    assert_eq!(card.color.len(), 1, "single color (purple)");
}

#[test]
fn st16_14_has_start_of_turn_trash_trigger_and_security_clauses() {
    let runner = matt_runner();
    let card = runner.compiled_card(CARD_ID).expect("ST16-14 compiled");
    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    assert!(
        triggered
            .iter()
            .any(|t| t.when == vec![CompiledTiming::StartOfYourTurn]),
        "[Start of Your Turn] memory-setter clause present"
    );
    assert!(
        triggered
            .iter()
            .any(|t| t.when == vec![CompiledTiming::OnDiscardHand]),
        "on_discard_hand clause present"
    );
    assert!(
        triggered
            .iter()
            .any(|t| t.when == vec![CompiledTiming::OnSecurity]),
        "[Security] play-self clause present"
    );
}

/// The on_discard_hand clause must be optional (an activatable ability, per
/// DCGO's `ActivateClass`, not a forced trigger).
#[test]
fn st16_14_discard_hand_clause_is_optional() {
    let runner = matt_runner();
    let card = runner.compiled_card(CARD_ID).expect("ST16-14 compiled");
    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when == vec![CompiledTiming::OnDiscardHand] => {
                Some(t)
            }
            _ => None,
        })
        .expect("on_discard_hand clause");
    assert!(clause.optional, "the gain-1-memory ability is declinable");
}

/// The on_discard_hand clause must gate on BOTH "your own hand" and "your
/// own effect" (DCGO `CanTriggerOnTrashHand`'s two predicates).
#[test]
fn st16_14_discard_hand_clause_gates_own_hand_and_own_effect() {
    let runner = matt_runner();
    let card = runner.compiled_card(CARD_ID).expect("ST16-14 compiled");
    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when == vec![CompiledTiming::OnDiscardHand] => {
                Some(t)
            }
            _ => None,
        })
        .expect("on_discard_hand clause");

    fn has_discard_player_you(p: &CompiledPredicate) -> bool {
        p.event_discard_player == Some(digimon_dsl::compiled::CompiledPlayerRef::You)
            || p.all_of.iter().any(has_discard_player_you)
            || p.any_of.iter().any(has_discard_player_you)
    }
    fn has_own_effect_true(p: &CompiledPredicate) -> bool {
        p.event_caused_by_own_effect == Some(true)
            || p.all_of.iter().any(has_own_effect_true)
            || p.any_of.iter().any(has_own_effect_true)
    }

    let aw = clause
        .active_when
        .as_ref()
        .expect("active_when must gate this ability");
    assert!(has_discard_player_you(aw), "must gate on your own hand");
    assert!(has_own_effect_true(aw), "must gate on your own effect");
}

// ─── Section 2 — [Start of Your Turn] memory setter ──────────────────────────

#[test]
fn st16_14_start_of_turn_sets_memory_to_3_when_lte_2() {
    let mut runner = matt_runner();
    runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.memory = 2;

    runner.end_turn();
    runner.end_turn();

    assert_eq!(
        runner.memory(),
        3,
        "Matt sets memory to 3 at start of your turn when memory is 2 or less"
    );
}

#[test]
fn st16_14_start_of_turn_does_not_change_memory_when_gt_2() {
    let mut runner = matt_runner();
    runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.memory = 5;

    runner.end_turn();
    runner.end_turn();

    assert_eq!(
        runner.memory(),
        5,
        "memory above 2 must be left untouched at start of turn"
    );
}

// ─── Section 3 — [All Turns] own-effect hand-trash -> suspend self -> +1 mem ─

/// POSITIVE: your own effect trashes a card from YOUR OWN hand -> the
/// activation installs; accepting suspends Matt and gains 1 memory.
#[test]
fn st16_14_own_effect_trashes_own_hand_offers_activation_and_gains_memory() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST16-14 YAML loads")
        .from_dsl_yaml(&hand_trash_trigger_yaml(true))
        .expect("trigger fixture loads")
        .add_card(hand_card("HANDFILL"))
        .add_card(filler("FILLER"))
        .hand(0, &["HANDFILL"])
        .deck(0, &["FILLER"; 10])
        .deck(1, &["FILLER"; 10])
        // NOT the (-10, 10) ceiling: memory is a shared seesaw value
        // (game/memory.rs::gain_memory_for_player_sourced clamps to
        // rules.memory_range, default (-10, 10)) — starting at the max
        // would silently clamp the +1 gain this test asserts on. 0 leaves
        // headroom in both directions.
        .memory(0)
        .start();
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 0;

    let matt = runner.place_on_field(0, CARD_ID, Some(0));
    let trig = runner.place_on_field(0, "DSL-HANDTRASH-001", Some(0));

    let mem_before = runner.memory();
    runner.fire_on_play(0, trig.index as usize);
    resolve_own_hand_trash_cost(&mut runner);

    let pending = runner
        .pending_selection_view()
        .expect("own-hand + own-effect trash must offer Matt's activation");
    assert_eq!(pending.kind, SelectionKind::TriggerOrder);
    assert!(pending.is_optional, "the activation is declinable");

    runner
        .accept_optional_trigger()
        .expect("accept the suspend-self activation");
    let _ = runner.auto_resolve();

    assert!(
        runner.game.players[0].battle_area[matt.index as usize].is_suspended,
        "Matt pays the printed suspend-self activation cost"
    );
    assert_eq!(
        runner.memory(),
        mem_before + 1,
        "accepting gains 1 memory"
    );
}

/// The activation can be declined — no suspend, no memory gain.
#[test]
fn st16_14_own_effect_trash_activation_can_be_declined() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST16-14 YAML loads")
        .from_dsl_yaml(&hand_trash_trigger_yaml(true))
        .expect("trigger fixture loads")
        .add_card(hand_card("HANDFILL"))
        .add_card(filler("FILLER"))
        .hand(0, &["HANDFILL"])
        .deck(0, &["FILLER"; 10])
        .deck(1, &["FILLER"; 10])
        .memory(10)
        .start();
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 0;

    let matt = runner.place_on_field(0, CARD_ID, Some(0));
    let trig = runner.place_on_field(0, "DSL-HANDTRASH-001", Some(0));

    let mem_before = runner.memory();
    runner.fire_on_play(0, trig.index as usize);
    resolve_own_hand_trash_cost(&mut runner);

    let pending = runner
        .pending_selection_view()
        .expect("activation prompt installs");
    assert_eq!(pending.kind, SelectionKind::TriggerOrder);
    runner
        .decline_optional_trigger()
        .expect("decline the activation");
    let _ = runner.auto_resolve();

    assert!(
        !runner.game.players[0].battle_area[matt.index as usize].is_suspended,
        "declining must not pay the suspend cost"
    );
    assert_eq!(
        runner.memory(),
        mem_before,
        "declining must not gain memory"
    );
}

/// NEGATIVE (own-effect gate): the OPPONENT's effect trashes YOUR hand ->
/// Matt's ability must NOT fire (not "one of YOUR effects").
#[test]
fn st16_14_does_not_fire_when_opponent_effect_trashes_your_hand() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST16-14 YAML loads")
        .from_dsl_yaml(&hand_trash_trigger_yaml(false))
        .expect("trigger fixture loads")
        .add_card(hand_card("HANDFILL"))
        .add_card(filler("FILLER"))
        .hand(0, &["HANDFILL"])
        .deck(0, &["FILLER"; 10])
        .deck(1, &["FILLER"; 10])
        .memory(10)
        .start();
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 1;

    // Matt sits on player 0's field; player 1 plays the trigger, which
    // trashes player 0's hand (of: opponent, from player 1's perspective).
    let matt = runner.place_on_field(0, CARD_ID, Some(0));
    let trig = runner.place_on_field(1, "DSL-HANDTRASH-001", Some(0));

    let mem_before = runner.memory();
    runner.fire_on_play(1, trig.index as usize);
    let _ = runner.auto_resolve();

    assert!(
        runner.pending_selection_view().is_none(),
        "own-effect gate must NOT fire when the OPPONENT's effect trashed your hand"
    );
    assert!(
        !runner.game.players[0].battle_area[matt.index as usize].is_suspended,
        "Matt must not suspend"
    );
    assert_eq!(runner.memory(), mem_before, "no memory gained");
}

/// NEGATIVE (own-hand gate): YOUR OWN effect trashes the OPPONENT's hand ->
/// Matt's ability must NOT fire (not "a card in YOUR hand").
#[test]
fn st16_14_does_not_fire_when_own_effect_trashes_opponents_hand() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST16-14 YAML loads")
        .from_dsl_yaml(&hand_trash_trigger_yaml(false))
        .expect("trigger fixture loads")
        .add_card(hand_card("HANDFILL"))
        .add_card(filler("FILLER"))
        .hand(1, &["HANDFILL"])
        .deck(0, &["FILLER"; 10])
        .deck(1, &["FILLER"; 10])
        .memory(10)
        .start();
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 0;

    // Matt sits on player 0's field; player 0 plays the trigger, which
    // trashes player 1 (opponent)'s hand.
    let matt = runner.place_on_field(0, CARD_ID, Some(0));
    let trig = runner.place_on_field(0, "DSL-HANDTRASH-001", Some(0));

    let mem_before = runner.memory();
    runner.fire_on_play(0, trig.index as usize);
    let _ = runner.auto_resolve();

    assert!(
        runner.pending_selection_view().is_none(),
        "own-hand gate must NOT fire when your own effect trashed the OPPONENT's hand"
    );
    assert!(
        !runner.game.players[0].battle_area[matt.index as usize].is_suspended,
        "Matt must not suspend"
    );
    assert_eq!(runner.memory(), mem_before, "no memory gained");
}

/// NEGATIVE (already-suspended source): Matt cannot pay the suspend-self
/// cost while already suspended, so no activation should be offered/paid.
#[test]
fn st16_14_cannot_activate_while_already_suspended() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST16-14 YAML loads")
        .from_dsl_yaml(&hand_trash_trigger_yaml(true))
        .expect("trigger fixture loads")
        .add_card(hand_card("HANDFILL"))
        .add_card(filler("FILLER"))
        .hand(0, &["HANDFILL"])
        .deck(0, &["FILLER"; 10])
        .deck(1, &["FILLER"; 10])
        .memory(10)
        .start();
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 0;

    let matt = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.players[0].battle_area[matt.index as usize].is_suspended = true;
    let trig = runner.place_on_field(0, "DSL-HANDTRASH-001", Some(0));

    let mem_before = runner.memory();
    runner.fire_on_play(0, trig.index as usize);
    resolve_own_hand_trash_cost(&mut runner);

    if let Some(pending) = runner.pending_selection_view() {
        // If a prompt still installs, accepting it must fail to pay the
        // cost (no-op) rather than silently suspending an already-suspended
        // permanent or granting memory anyway.
        assert!(pending.is_optional);
        runner.accept_optional_trigger().ok();
    }
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.memory(),
        mem_before,
        "an already-suspended Matt cannot pay the cost, so no memory is gained"
    );
}

/// Event-log check: the hand-trash step underlying the causing effect fires
/// a Trash event for the hand card (cost-firing sanity, per §5 Section 4).
#[test]
fn st16_14_own_effect_hand_trash_fires_trash_event() {
    use digimon_engine::events::GameEvent;

    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST16-14 YAML loads")
        .from_dsl_yaml(&hand_trash_trigger_yaml(true))
        .expect("trigger fixture loads")
        .add_card(hand_card("HANDFILL"))
        .add_card(filler("FILLER"))
        .hand(0, &["HANDFILL"])
        .deck(0, &["FILLER"; 10])
        .deck(1, &["FILLER"; 10])
        .memory(10)
        .start();
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 0;

    let _matt = runner.place_on_field(0, CARD_ID, Some(0));
    let trig = runner.place_on_field(0, "DSL-HANDTRASH-001", Some(0));

    let cp = runner.event_checkpoint();
    runner.fire_on_play(0, trig.index as usize);
    resolve_own_hand_trash_cost(&mut runner);
    // Drain the activation prompt regardless of branch to avoid leaving a
    // dangling selection before inspecting the event log.
    if let Some(pending) = runner.pending_selection_view() {
        assert!(pending.is_optional);
        runner.decline_optional_trigger().ok();
    }
    let _ = runner.auto_resolve();

    let events = runner.events_since(cp);
    let trash_count = events
        .iter()
        .filter(|e| matches!(e, GameEvent::Trash { player: 0, .. }))
        .count();
    assert!(
        trash_count >= 1,
        "the causing effect's hand-trash must emit a Trash event for player 0"
    );
}

// ─── Section 4 — Inherited [Security] play self free ─────────────────────────

#[test]
fn st16_14_security_plays_itself_without_paying_cost() {
    let mut attacker = make_test_card("ATTACKER-ST16-14", "Attacker");
    attacker.card_kind = CardKind::Digimon;
    attacker.colors = vec![CardColor::Red];
    attacker.level = Some(4);
    attacker.dp = Some(9000);

    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST16-14 YAML loads")
        .add_card(attacker)
        .security(1, &[CARD_ID])
        .memory(10)
        .start();
    let attacker = runner.place_on_field(0, "ATTACKER-ST16-14", Some(0));

    runner.attack_player(attacker, 1, false);
    runner.auto_resolve().expect("resolve security play");

    assert!(
        runner.game.players[1]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == CARD_ID),
        "Matt is played from the defender's security without paying the cost"
    );
}
