//! Chapter 18 "Other Information" (Comprehensive Rules Ver. 4.3): pending
//! processing (§18-1), overwrite processing (§18-2) and infinite loops
//! (§18-3), with the two places the manual applies pending processing: the
//! used Option's trash (§9-1-4 / §9-1-5) and `<Piercing>` (§16-7-4..6).
//!
//! Every test asserts the RULE, never the engine's current output. A test the
//! engine fails is `#[ignore]`d with the gap id that explains it (discover then
//! pin); see `docs/RUST_ENGINE_GAPS.md` and `qa/rules/cr_obligations_status.yaml`.
//!
//! Fixture cards: `fixtures/common/*.yaml` + `fixtures/ch18/*.yaml` (plus the
//! `ch15_08` cards `EOT_MEM` and `PLAIN_TAMER`), one synthetic
//! card per file, authored to printed-style text (each clause's `summary:` is
//! that text).
//!
//! The overwrite fixtures express "instead" with the DSL's own branching (`if`
//! / `else` for a mandatory overwrite, `select_effect_choice` gated by
//! `legal_when` for an optional one). The tests show the engine performs ONLY
//! the overwriting processing and surfaces the §18-2-2 choice; they do not
//! show the engine recognises "instead" unaided -- the card author picks the
//! branch shape.

use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::Duration;

use crate::fixtures;
use crate::support::*;
use digimon_engine::action::space::ATTACK_START;
use digimon_engine::count_choice::CountChoiceOperation;
use digimon_engine::debug_runner::{DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{GamePhase, PlayerId};
use digimon_engine::events::GameEvent;
use digimon_engine::game::TerminalOutcomeReason;
use digimon_engine::selection::SelectionKind;

fn ch18() -> DebugRunnerBuilder {
    fixtures::builder(&[fixtures::ch15_08_cards(), fixtures::ch18_cards()])
}

/// `(player, source card id, delta)` of every effect-sourced memory change
/// since `cp`, in event order.
fn memory_deltas(r: &DebugRunner, cp: usize) -> Vec<(PlayerId, String, i16)> {
    r.events_since(cp)
        .iter()
        .filter_map(|e| match e {
            GameEvent::MemoryChange {
                player,
                source_card_id: Some(id),
                delta,
                ..
            } => Some((*player, id.clone(), *delta)),
            _ => None,
        })
        .collect()
}

/// Index in the event log (since `cp`) of the first effect-sourced memory
/// change whose source is `card_id`.
fn memory_event_index(r: &DebugRunner, cp: usize, card_id: &str) -> Option<usize> {
    r.events_since(cp).iter().position(
        |e| matches!(e, GameEvent::MemoryChange { source_card_id: Some(id), .. } if id == card_id),
    )
}

/// Indices in the event log (since `cp`) of every security card revealed from
/// `defender`'s security stack.
fn security_reveal_indices(r: &DebugRunner, cp: usize, defender: PlayerId) -> Vec<usize> {
    r.events_since(cp)
        .iter()
        .enumerate()
        .filter(
            |(_, e)| matches!(e, GameEvent::SecurityReveal { defender: d, .. } if *d == defender),
        )
        .map(|(i, _)| i)
        .collect()
}

/// Index in the event log (since `cp`) of the first `TurnStart` of `player`.
fn turn_start_index(r: &DebugRunner, cp: usize, player: PlayerId) -> Option<usize> {
    r.events_since(cp)
        .iter()
        .position(|e| matches!(e, GameEvent::TurnStart { player: p, .. } if *p == player))
}

/// The opponent-field slots the pending `OppField` pick offers, sorted.
fn offered_opp_slots(r: &DebugRunner) -> Vec<u16> {
    let sel = r.pending_selection().expect("an opponent-field pick");
    assert_eq!(sel.kind, SelectionKind::OppField, "{sel:?}");
    let mut slots: Vec<u16> = sel
        .valid_action_ids
        .iter()
        .map(|&a| a - ATTACK_START)
        .collect();
    slots.sort();
    slots
}

/// Resolve the pending `EffectChoice` by picking the branch labelled `label`.
fn pick_branch(r: &mut DebugRunner, label: &str) {
    let (player, action) = {
        let sel = r.pending_selection().expect("an effect choice");
        assert_eq!(sel.kind, SelectionKind::EffectChoice, "{sel:?}");
        let choice = sel
            .effect_choices
            .as_ref()
            .expect("EffectChoice branches")
            .iter()
            .find(|c| c.label == label)
            .unwrap_or_else(|| panic!("no branch {label:?}: {sel:?}"));
        (sel.selecting_player, choice.action_id)
    };
    r.execute_action(player, action)
        .unwrap_or_else(|e| panic!("branch {label:?} rejected: {e:?}"));
}

/// Run `scenario` on its own thread and give up after `limit`: an engine that
/// loops forever inside one call fails the test instead of hanging the suite.
fn within<T: Send + 'static>(
    what: &str,
    limit: Duration,
    scenario: impl FnOnce() -> T + Send + 'static,
) -> T {
    let (tx, rx) = mpsc::channel();
    let handle = std::thread::Builder::new()
        .name(what.to_string())
        .stack_size(64 << 20)
        .spawn(move || {
            let _ = tx.send(scenario());
        })
        .expect("spawn the scenario thread");
    match rx.recv_timeout(limit) {
        Ok(v) => v,
        Err(RecvTimeoutError::Timeout) => {
            panic!("{what}: the engine did not return within {limit:?} (an unbounded loop)")
        }
        // The scenario panicked: re-raise its own message on the test thread.
        Err(RecvTimeoutError::Disconnected) => match handle.join() {
            Err(payload) => std::panic::resume_unwind(payload),
            Ok(()) => panic!("{what}: the scenario thread ended without a result"),
        },
    }
}

// ─── §18-1-1 / §18-1-2 ──────────────────────────────────────────────────────

/// P0's `WA_GAIN3_LOSE3_EOT` (the §18-1-1 example text) attacks and deletes
/// P1's suspended `FILL`, starting from memory 0; `with_eot_mem` adds P0's
/// Tamer `EOT_MEM` ("[End of Your Turn] Gain 2 memory").
fn pending_loss_board(with_eot_mem: bool) -> (DebugRunner, usize) {
    pending_loss_board_with(with_eot_mem.then_some("EOT_MEM"))
}

/// [`pending_loss_board`] with P0's Tamer `tamer` (if any) on the field.
fn pending_loss_board_with(tamer: Option<&str>) -> (DebugRunner, usize) {
    let mut r = ch18()
        .deck(0, &[FILL; 4])
        .deck(1, &[FILL; 4])
        .memory(0)
        .start();
    if let Some(tamer) = tamer {
        r.place_on_field(0, tamer, Some(0));
    }
    let attacker = r.place_on_field(0, "WA_GAIN3_LOSE3_EOT", Some(0));
    let defender = r.place_on_field(1, FILL, Some(0));
    set_suspended(&mut r, defender, true);
    let cp = r.event_checkpoint();
    r.attack_digimon(attacker, defender, false);
    resolve_prompts(&mut r);
    assert!(field_top_ids(&r, 1).is_empty(), "the defender was deleted");
    (r, cp)
}

/// §18-1-1 "Pending processing refers to processing for rules or effects that
/// is pending. Pending processing will be processed at the predetermined
/// timing for the corresponding rule or effect, similar to triggered effects.
/// (Example: If an effect reads "[When Attacking] Gain 3 memory. Lose 3 memory
/// at the end of the turn," "lose 3 memory at the end of the turn" is pending
/// processing that is processed similar to how it would be processed if it
/// triggered at the end of the turn.)"
///
/// The rule's own example. P0's `WA_GAIN3_LOSE3_EOT` attacks: it gains 3
/// memory and nothing else happens while the turn goes on (the loss is
/// pending). When P0 passes, the 3 memory is lost at the end of the turn,
/// before P1's turn starts.
#[test]
fn pending_processing_fires_at_its_timing_like_a_trigger() {
    cite!(
        "18-1-1",
        "45d8bc5f213f07ad66c7abf88f2fdcaaf8e06a3da9c0c4409ee83907cd44af10"
    );
    let (mut r, cp) = pending_loss_board(false);
    assert_eq!(
        memory_deltas(&r, cp),
        vec![(0, "WA_GAIN3_LOSE3_EOT".to_string(), 3)],
        "[When Attacking] gained 3 memory; the loss is pending, not processed"
    );
    assert_eq!(r.turn_player(), 0);
    assert_eq!(r.current_phase(), GamePhase::Main, "P0's turn goes on");

    r.pass_turn();
    resolve_prompts(&mut r);
    assert_eq!(
        memory_deltas(&r, cp),
        vec![
            (0, "WA_GAIN3_LOSE3_EOT".to_string(), 3),
            (0, "WA_GAIN3_LOSE3_EOT".to_string(), -3),
        ],
        "the pending loss was processed at the end of the turn"
    );
    let loss = r
        .events_since(cp)
        .iter()
        .rposition(|e| {
            matches!(e, GameEvent::MemoryChange { source_card_id: Some(id), delta: -3, .. }
                if id == "WA_GAIN3_LOSE3_EOT")
        })
        .expect("the loss event");
    let p1_turn = turn_start_index(&r, cp, 1).expect("P1's turn started");
    assert!(
        loss < p1_turn,
        "processed at the end of P0's turn, before P1's turn starts (loss #{loss}, P1 turn #{p1_turn})"
    );
}

/// §18-1-2 "If other processing will be performed at the same time as pending
/// processing, it is performed similar to effects that trigger simultaneously.
/// For details, refer to 15-4-3 "Simultaneous Triggering""
///
/// The §18-1-1 board plus P0's Tamer `EOT_MEM` ("[End of Your Turn] Gain 2
/// memory"). At the end of P0's turn the pending "lose 3 memory" and
/// `EOT_MEM` are processed at the same time, so P0 (the turn player,
/// §15-4-3-5-1) chooses their order in ONE `TriggerOrder` prompt, and either
/// order is honoured.
#[test]
fn pending_processing_orders_with_simultaneous_triggers() {
    cite!(
        "18-1-2",
        "1b5f8339dafb6d36f24a37380b4ccbcf4131788de779f4d1800da5a65bbd3372"
    );
    for first in ["WA_GAIN3_LOSE3_EOT", "EOT_MEM"] {
        let (mut r, cp) = pending_loss_board(true);
        r.pass_turn();
        let choices = order_choices(&r, 0);
        assert_eq!(
            choices.len(),
            2,
            "the pending loss and [End of Your Turn] share one ordering choice: {choices:?}"
        );
        assert!(
            choices
                .iter()
                .any(|l| branch_card(l) == "WA_GAIN3_LOSE3_EOT")
                && choices.iter().any(|l| branch_card(l) == "EOT_MEM"),
            "{choices:?}"
        );
        assert_eq!(
            memory_deltas(&r, cp).len(),
            1,
            "nothing at the end of the turn was processed before the choice"
        );
        choose(&mut r, first);
        resolve_prompts(&mut r);
        let second = if first == "EOT_MEM" {
            "WA_GAIN3_LOSE3_EOT"
        } else {
            "EOT_MEM"
        };
        let end_of_turn: Vec<String> = memory_deltas(&r, cp)
            .into_iter()
            .skip(1)
            .map(|(_, id, _)| id)
            .collect();
        assert_eq!(
            end_of_turn,
            owned(&[first, second]),
            "P0 chose {first} first"
        );
    }
}

/// Index in the event log (since `cp`) of the first trash of `card_id`.
fn trash_event_index(r: &DebugRunner, cp: usize, card_id: &str) -> Option<usize> {
    r.events_since(cp)
        .iter()
        .position(|e| matches!(e, GameEvent::Trash { card_id: id, .. } if id == card_id))
}

/// The §18-1-1 board plus P0's Tamer `EOT_MAY_MEM` ("[End of Your Turn] You
/// may gain 1 memory"). At the end of P0's turn it pools with the pending
/// "lose 3 memory" (§18-1-2). Ordering it first decides only the ORDER: its
/// "you may" decision still follows, and both answers are honoured.
#[test]
fn optional_effect_ordered_first_in_the_pool_keeps_its_choice() {
    cite!(
        "18-1-2",
        "1b5f8339dafb6d36f24a37380b4ccbcf4131788de779f4d1800da5a65bbd3372"
    );
    for take in [true, false] {
        let (mut r, cp) = pending_loss_board_with(Some("EOT_MAY_MEM"));
        r.pass_turn();
        let choices = order_choices(&r, 0);
        assert_eq!(choices.len(), 2, "{choices:?}");
        choose(&mut r, "EOT_MAY_MEM");
        {
            let sel = r.pending_selection().expect("EOT_MAY_MEM's \"you may\"");
            assert_ne!(sel.kind, SelectionKind::TriggerOrder, "{sel:?}");
            assert!(sel.is_optional, "an accept / decline choice: {sel:?}");
            assert_eq!(sel.selecting_player, 0, "{sel:?}");
        }
        assert_eq!(
            memory_deltas(&r, cp).len(),
            1,
            "nothing at the end of the turn was processed before the choice"
        );
        if take {
            accept(&mut r, 0);
        } else {
            r.decline_optional_trigger()
                .unwrap_or_else(|e| panic!("decline rejected: {e:?}"));
        }
        resolve_prompts(&mut r);
        let end_of_turn: Vec<(String, i16)> = memory_deltas(&r, cp)
            .into_iter()
            .skip(1)
            .map(|(_, id, delta)| (id, delta))
            .collect();
        let mut expected = Vec::new();
        if take {
            expected.push(("EOT_MAY_MEM".to_string(), 1));
        }
        expected.push(("WA_GAIN3_LOSE3_EOT".to_string(), -3));
        assert_eq!(end_of_turn, expected, "accepted = {take}");
    }
}

/// "At the end of the turn, delete the Digimon this effect played" is
/// pending processing too (§18-1-1). P0 uses `MAIN_PLAY_LV3_DEL_EOT` to play
/// `VANILLA_LV3` and has the Tamer `EOT_MEM` ("[End of Your Turn] Gain 2
/// memory"). At the end of P0's turn the deletion and `EOT_MEM` are one
/// simultaneous pool that P0 orders (§18-1-2), and either order is honoured.
#[test]
fn pending_deletion_orders_with_simultaneous_triggers() {
    cite!(
        "18-1-2",
        "1b5f8339dafb6d36f24a37380b4ccbcf4131788de779f4d1800da5a65bbd3372"
    );
    for first in ["MAIN_PLAY_LV3_DEL_EOT", "EOT_MEM"] {
        let mut r = ch18()
            .hand(0, &["MAIN_PLAY_LV3_DEL_EOT", FILL])
            .deck(0, &[FILL; 4])
            .deck(1, &[FILL; 4])
            .memory(3)
            .start();
        r.place_on_field(0, "EOT_MEM", Some(0));
        use_option(&mut r, 0, "MAIN_PLAY_LV3_DEL_EOT");
        pick_hand_card(&mut r, 0, FILL);
        resolve_prompts(&mut r);
        assert_eq!(field_top_ids(&r, 0), owned(&["EOT_MEM", FILL]));
        let cp = r.event_checkpoint();
        r.pass_turn();
        let choices = order_choices(&r, 0);
        assert_eq!(
            choices.len(),
            2,
            "the pending deletion and [End of Your Turn] share one ordering choice: {choices:?}"
        );
        assert!(
            choices
                .iter()
                .any(|l| branch_card(l) == "MAIN_PLAY_LV3_DEL_EOT")
                && choices.iter().any(|l| branch_card(l) == "EOT_MEM"),
            "{choices:?}"
        );
        assert_eq!(
            field_top_ids(&r, 0),
            owned(&["EOT_MEM", FILL]),
            "nothing was deleted before the choice"
        );
        assert!(memory_deltas(&r, cp).is_empty(), "nor any memory gained");
        choose(&mut r, first);
        resolve_prompts(&mut r);
        assert_eq!(field_top_ids(&r, 0), owned(&["EOT_MEM"]), "deleted");
        let deleted = trash_event_index(&r, cp, FILL).expect("the deletion's trash event");
        let gained = memory_event_index(&r, cp, "EOT_MEM").expect("EOT_MEM's gain");
        if first == "EOT_MEM" {
            assert!(gained < deleted, "P0 chose EOT_MEM first");
        } else {
            assert!(deleted < gained, "P0 chose the deletion first");
        }
    }
}

/// A pending "delete the Digimon this effect played" whose Digimon already
/// left the battle area has nothing left to process, so it is not an item of
/// the end-of-turn pool: P0 plays `VANILLA_LV3` with `MAIN_PLAY_LV3_DEL_EOT`,
/// the Digimon is deleted before the turn ends, and at the end of the turn
/// `EOT_MEM` resolves on its own (no ordering choice with a dead branch).
#[test]
fn departed_pending_deletion_is_not_an_ordering_item() {
    cite!(
        "18-1-2",
        "1b5f8339dafb6d36f24a37380b4ccbcf4131788de779f4d1800da5a65bbd3372"
    );
    let mut r = ch18()
        .hand(0, &["MAIN_PLAY_LV3_DEL_EOT", FILL])
        .deck(0, &[FILL; 4])
        .deck(1, &[FILL; 4])
        .memory(3)
        .start();
    r.place_on_field(0, "EOT_MEM", Some(0));
    use_option(&mut r, 0, "MAIN_PLAY_LV3_DEL_EOT");
    pick_hand_card(&mut r, 0, FILL);
    resolve_prompts(&mut r);
    assert_eq!(field_top_ids(&r, 0), owned(&["EOT_MEM", FILL]));
    let played = r.perm_handle(0, 1);
    r.game.delete_permanent_with_cause(
        played,
        digimon_engine::replacement::ReplacementCause::OwnEffect,
    );
    resolve_prompts(&mut r);
    assert_eq!(field_top_ids(&r, 0), owned(&["EOT_MEM"]), "it left first");
    let cp = r.event_checkpoint();
    r.pass_turn();
    assert!(
        r.pending_selection().is_none(),
        "no ordering choice with a dead pending deletion: {:?}",
        r.pending_selection()
    );
    assert_eq!(
        memory_deltas(&r, cp)
            .into_iter()
            .filter(|(_, id, _)| id == "EOT_MEM")
            .count(),
        1,
        "EOT_MEM resolved on its own"
    );
    assert_eq!(r.turn_player(), 1, "the turn passed");
}

/// The pool at the end of the opponent's turn (§18-1-2) resolves before the
/// turn passes. P0 used `MAIN_MEM_AT_EOONT` ("At the end of your opponent's
/// next turn, gain 2 memory") and has the Tamer `EOOT_MEM` ("[End of
/// Opponent's Turn] Gain 1 memory"). When P1 passes, both are pending for P0
/// in one ordering choice; while it waits, P1's turn has not ended (no turn
/// advance, no `TurnStart` for P0); once it resolves, both are processed and
/// only then does P0's turn start.
#[test]
fn end_of_opponents_turn_pool_resolves_before_the_turn_passes() {
    cite!(
        "18-1-2",
        "1b5f8339dafb6d36f24a37380b4ccbcf4131788de779f4d1800da5a65bbd3372"
    );
    let mut r = ch18()
        .hand(0, &["MAIN_MEM_AT_EOONT"])
        .deck(0, &[FILL; 4])
        .deck(1, &[FILL; 4])
        .memory(3)
        .start();
    r.place_on_field(0, "EOOT_MEM", Some(0));
    use_option(&mut r, 0, "MAIN_MEM_AT_EOONT");
    resolve_prompts(&mut r);
    r.pass_turn();
    resolve_prompts(&mut r);
    assert_eq!(r.turn_player(), 1);
    let turn = r.game.turn_count;
    let cp = r.event_checkpoint();
    r.pass_turn();

    let choices = order_choices(&r, 0);
    assert_eq!(
        choices.len(),
        2,
        "the pending gain and [End of Opponent's Turn] share one ordering choice: {choices:?}"
    );
    assert_eq!(r.turn_player(), 1, "P1's turn has not ended");
    assert_eq!(r.game.turn_count, turn, "no turn advance while the pool waits");
    assert_eq!(turn_start_index(&r, cp, 0), None, "P0's turn has not started");
    assert!(memory_deltas(&r, cp).is_empty(), "nothing processed before the choice");

    choose(&mut r, "MAIN_MEM_AT_EOONT");
    resolve_prompts(&mut r);
    let gains: Vec<(String, i16)> = memory_deltas(&r, cp)
        .into_iter()
        .map(|(_, id, delta)| (id, delta))
        .collect();
    // The gauge is read from the turn player's (P1's) side, so P0's gains
    // are negative deltas.
    assert_eq!(
        gains,
        vec![
            ("MAIN_MEM_AT_EOONT".to_string(), -2),
            ("EOOT_MEM".to_string(), -1)
        ]
    );
    let p0_turn = turn_start_index(&r, cp, 0).expect("P0's turn started");
    let last_gain = memory_event_index(&r, cp, "EOOT_MEM").expect("EOOT_MEM's gain");
    assert!(
        last_gain < p0_turn,
        "the pool finished before P0's turn began (gain #{last_gain}, turn #{p0_turn})"
    );
    assert_eq!(r.turn_player(), 0);
    assert_eq!(r.game.turn_count, turn + 1);
}

// ─── §18-2-1 / §18-2-3 ──────────────────────────────────────────────────────

/// P0 holds `MAIN_DEL_6000_OR_LOWEST_IF_GREYMON` (the §18-2-1 example text)
/// and has a red Digimon (`FILL`, or `GREYMON_LV4` when `greymon`); P1 has
/// the Digimon `opponent` (slot order).
fn instead_board(greymon: bool, opponent: &[&str]) -> DebugRunner {
    let mut r = ch18()
        .hand(0, &["MAIN_DEL_6000_OR_LOWEST_IF_GREYMON"])
        .deck(0, &[FILL; 4])
        .deck(1, &[FILL; 4])
        .memory(3)
        .start();
    r.place_on_field(0, if greymon { "GREYMON_LV4" } else { FILL }, Some(0));
    for id in opponent {
        r.place_on_field(1, id, Some(0));
    }
    use_option(&mut r, 0, "MAIN_DEL_6000_OR_LOWEST_IF_GREYMON");
    r
}

/// §18-2-1 "Overwrite processing refers to when the processing that is
/// normally performed for a rule or effect is overwritten with different
/// processing using "instead" in text. (Example: In an effect that reads
/// "[Main] Delete 1 of your opponent's Digimon with 6000 DP or less. If you
/// have a Digimon with [Greymon] in its name, instead delete 1 of your
/// opponent's Digimon with the lowest DP," the overwrite processing is shown
/// right before "instead.")"
/// §18-2-3 "If overwrite processing is mandatory, the processing must be
/// overwritten with the different processing shown for the "instead" in text,
/// and it will be considered mandatory processing."
///
/// The rule's own example, against P1's 1000 / 3000 / 7000 DP Digimon:
/// - Without a [Greymon]: the normal processing -- the deletion is offered on
///   the two 6000-or-less Digimon only.
/// - With a [Greymon]: the overwrite is mandatory -- no choice of processing is
///   offered, and the deletion is offered ONLY on the lowest DP Digimon (the
///   3000 DP one, legal under the normal processing, is not).
/// - Against a lone 7000 DP Digimon: the normal processing deletes nothing;
///   the overwriting processing deletes it (it is not "normal + extra").
#[test]
fn instead_overwrites_the_normal_processing() {
    cite!(
        "18-2-1",
        "4030c08eb61185203dee86d2da87a427592d920084b95a977fe70cdf8d4b3bd5"
    );
    cite!(
        "18-2-3",
        "957ff084c19f3c860c546fdcde4c4372496b08fd1bf114c5d4bba86b18e2f49d"
    );
    let opponent = [FILL, "OP_MEM", "LV5_PLAIN"]; // 1000, 3000, 7000 DP

    // Normal processing.
    let mut r = instead_board(false, &opponent);
    assert_eq!(
        offered_opp_slots(&r),
        vec![0, 1],
        "without a [Greymon]: the 6000-or-less Digimon"
    );
    pick_field(&mut r, 1);
    resolve_prompts(&mut r);
    assert_eq!(field_top_ids(&r, 1), owned(&[FILL, "LV5_PLAIN"]));

    // Mandatory overwrite.
    let mut r = instead_board(true, &opponent);
    assert_eq!(
        offered_opp_slots(&r),
        vec![0],
        "with a [Greymon] the processing is overwritten: only the lowest DP Digimon, and no \
         choice to keep the normal processing"
    );
    pick_field(&mut r, 0);
    resolve_prompts(&mut r);
    assert_eq!(field_top_ids(&r, 1), owned(&["OP_MEM", "LV5_PLAIN"]));

    // A lone 7000 DP Digimon.
    let mut r = instead_board(false, &["LV5_PLAIN"]);
    resolve_prompts(&mut r);
    assert_eq!(
        field_top_ids(&r, 1),
        owned(&["LV5_PLAIN"]),
        "the normal processing has no target"
    );
    let mut r = instead_board(true, &["LV5_PLAIN"]);
    if r.pending_selection().is_some() {
        assert_eq!(offered_opp_slots(&r), vec![0]);
        pick_field(&mut r, 0);
    }
    resolve_prompts(&mut r);
    assert!(
        field_top_ids(&r, 1).is_empty(),
        "the overwriting processing deletes the lowest DP Digimon, 7000 DP"
    );
}

// ─── §18-2-2 ────────────────────────────────────────────────────────────────

/// P0 plays `OP_MEM_OR_DRAW_INSTEAD` ("[On Play] Gain 1 memory. If you have a
/// Tamer, you may <Draw 1> instead.") with or without the Tamer `PLAIN_TAMER`.
fn optional_instead_board(tamer: bool) -> (DebugRunner, usize, usize) {
    let mut r = ch18()
        .hand(0, &["OP_MEM_OR_DRAW_INSTEAD"])
        .deck(0, &[FILL; 4])
        .memory(5)
        .start();
    if tamer {
        r.place_on_field(0, "PLAIN_TAMER", Some(0));
    }
    let cp = r.event_checkpoint();
    r.play(0, 0);
    let hand = r.hand_size(0);
    (r, cp, hand)
}

/// §18-2-2 "If overwrite processing is optional, it will be possible to
/// choose to overwrite with the different processing shown for the "instead"
/// in text, and it will be considered optional processing. (Example: … while
/// the player has [Kiriha Aonuma], they can choose whether or not the
/// processing is replaced with the processing shown for "instead," …)"
///
/// `OP_MEM_OR_DRAW_INSTEAD` ("[On Play] Gain 1 memory. If you have a Tamer,
/// you may <Draw 1> instead."):
/// - Without a Tamer: nothing to choose; the normal processing (gain 1).
/// - With a Tamer: the player chooses. Keeping the normal processing gains 1
///   memory and draws nothing; overwriting draws 1 and gains nothing.
#[test]
fn optional_instead_lets_the_player_choose() {
    cite!(
        "18-2-2",
        "c098689f45e4cfdd32f6360c772021ccc4f4e1c89919fbd77cc67b7497231827"
    );
    // No Tamer: the overwrite isn't available.
    let (r, cp, hand) = optional_instead_board(false);
    assert!(
        r.pending_selection().is_none(),
        "without a Tamer there is nothing to choose: {:?}",
        r.pending_selection()
    );
    assert_eq!(
        memory_deltas(&r, cp),
        vec![(0, "OP_MEM_OR_DRAW_INSTEAD".to_string(), 1)]
    );
    assert_eq!(r.hand_size(0), hand);

    // Tamer: keep the normal processing.
    let (mut r, cp, hand) = optional_instead_board(true);
    pick_branch(&mut r, "Gain 1 memory");
    resolve_prompts(&mut r);
    assert_eq!(
        memory_deltas(&r, cp),
        vec![(0, "OP_MEM_OR_DRAW_INSTEAD".to_string(), 1)]
    );
    assert_eq!(r.hand_size(0), hand, "not overwritten: no draw");

    // Tamer: overwrite.
    let (mut r, cp, hand) = optional_instead_board(true);
    pick_branch(&mut r, "<Draw 1> instead");
    resolve_prompts(&mut r);
    assert!(
        memory_deltas(&r, cp).is_empty(),
        "overwritten: the memory gain is not performed: {:?}",
        memory_deltas(&r, cp)
    );
    assert_eq!(r.hand_size(0), hand + 1, "the overwriting <Draw 1>");
}

// ─── §9-1-4 / §9-1-5 ────────────────────────────────────────────────────────

/// §9-1-4 "A used Option card is treated as not being in any area during the
/// period from activation of its 1st [Main] effect until it's been resolved."
///
/// P0 uses `MAIN_PLAY_LV3`. While its `[Main]` waits on the hand pick, the
/// Option is in no area: not in the hand, the trash, or the battle area.
#[test]
fn used_option_is_in_no_area_during_its_main() {
    cite!(
        "9-1-4",
        "712223999228b4a06260393d74a1fa77930b3edabe46a05fac85dbe3e6c322ab"
    );
    let mut r = ch18()
        .hand(0, &["MAIN_PLAY_LV3", "OP_MEM"])
        .deck(0, &[FILL; 4])
        .memory(3)
        .start();
    r.place_on_field(0, FILL, Some(0));
    use_option(&mut r, 0, "MAIN_PLAY_LV3");
    assert_eq!(
        r.pending_selection().map(|s| s.kind),
        Some(SelectionKind::Hand),
        "the [Main] waits on the hand pick"
    );
    assert_eq!(hand_ids(&r, 0), owned(&["OP_MEM"]), "not in the hand");
    assert!(trash_ids(&r, 0).is_empty(), "not in the trash");
    assert_eq!(
        field_top_ids(&r, 0),
        owned(&[FILL]),
        "not in the battle area"
    );
}

/// §9-1-5 "A used Option card is immediately trashed as pending processing as
/// soon as the timing when its 1st [Main] effect has been resolved, unless
/// that Option card is considered to be placed in an area. Other effects and
/// rule checks don't interrupt the period between [Main] effect activation
/// and trashing."
///
/// Both halves read the board at a decision that parks while the
/// interrupting processing is under way, and the event log once it has
/// finished (the Option's trash is a `GameEvent::Trash`,
/// G-ENGINE-OPTION-TRASH-NO-EVENT):
/// - Rule check: P0's `MAIN_MINUS3000` leaves P1's 3000 DP
///   `WOULD_DEL_MAY_MEM` ("[All Turns] When this Digimon would be deleted,
///   you may gain 1 memory") at 0 DP. When the rule check is about to delete
///   it, its immediate-type decision parks; the Option must already be in the
///   trash.
/// - Triggered effect: P0's `MAIN_PLAY_LV3` plays `OP_MAY_MEM`, whose "[On
///   Play] You may gain 1 memory" triggers. The Option is trashed at once:
///   the next decision is that [On Play]'s own, never a choice that offers
///   the trash as one of P0's pending items, and the Option is already in the
///   trash when it is asked.
#[test]
fn used_option_trashed_immediately_after_first_main_uninterrupted() {
    cite!(
        "9-1-5",
        "2886ba315f2847fdd735e7c839b0b9c80cd0bf7391a1596460851d82cc7eb003"
    );
    // A rule check caused by the [Main].
    let mut r = fixtures::builder(&[
        fixtures::ch15_08_cards(),
        fixtures::ch17_cards(),
        fixtures::ch18_cards(),
    ])
    .hand(0, &["MAIN_MINUS3000"])
    .deck(0, &[FILL; 4])
    .deck(1, &[FILL; 4])
    .memory(3)
    .start();
    r.place_on_field(0, FILL, Some(0));
    r.place_on_field(1, "WOULD_DEL_MAY_MEM", Some(0));
    let cp1 = r.event_checkpoint();
    use_option(&mut r, 0, "MAIN_MINUS3000");
    pick_field(&mut r, 0);
    let sel = r
        .pending_selection()
        .expect("the 0 DP Digimon's would-be-deleted decision");
    assert!(
        sel.is_optional && sel.selecting_player == 1,
        "the rule check is about to delete WOULD_DEL_MAY_MEM: {sel:?}"
    );
    assert_eq!(
        field_top_ids(&r, 1),
        owned(&["WOULD_DEL_MAY_MEM"]),
        "not deleted yet"
    );
    assert_eq!(
        trash_ids(&r, 0),
        owned(&["MAIN_MINUS3000"]),
        "the rule check did not interrupt: the Option was trashed before it"
    );
    assert!(
        trash_event_index(&r, cp1, "MAIN_MINUS3000").is_some(),
        "the Option's trash is logged before the rule check's decision"
    );
    accept(&mut r, 1);
    resolve_prompts(&mut r);
    assert!(
        field_top_ids(&r, 1).is_empty(),
        "the 0 DP Digimon was deleted"
    );
    let option_trashed = trash_event_index(&r, cp1, "MAIN_MINUS3000")
        .expect("GameEvent::Trash for the used Option");
    let would_del = memory_event_index(&r, cp1, "WOULD_DEL_MAY_MEM")
        .expect("the would-be-deleted effect gained 1 memory");
    let deleted = trash_event_index(&r, cp1, "WOULD_DEL_MAY_MEM")
        .expect("GameEvent::Trash for the deleted Digimon");
    assert!(
        option_trashed < would_del && option_trashed < deleted,
        "event order: the Option's trash ({option_trashed}) precedes the rule check's \
         would-be-deleted effect ({would_del}) and deletion ({deleted})"
    );

    // A triggered effect caused by the [Main].
    let mut r = ch18()
        .hand(0, &["MAIN_PLAY_LV3", "OP_MAY_MEM"])
        .deck(0, &[FILL; 4])
        .memory(3)
        .start();
    r.place_on_field(0, FILL, Some(0));
    let cp = r.event_checkpoint();
    use_option(&mut r, 0, "MAIN_PLAY_LV3");
    pick_hand_card(&mut r, 0, "OP_MAY_MEM");
    let sel = r.pending_selection().expect("the [On Play]'s decision");
    assert!(
        sel.is_optional
            && sel
                .effect_choices
                .iter()
                .flatten()
                .all(|c| !c.label.starts_with("Trash the used Option")),
        "nothing interrupts between the [Main] and the trash, and the trash is not a \
         choice: {sel:?}"
    );
    assert_eq!(
        trash_ids(&r, 0),
        owned(&["MAIN_PLAY_LV3"]),
        "the Option was trashed before the triggered [On Play] activated"
    );
    assert!(
        trash_event_index(&r, cp, "MAIN_PLAY_LV3").is_some(),
        "the Option's trash is logged before the [On Play]'s decision"
    );
    accept(&mut r, 0);
    resolve_prompts(&mut r);
    assert_eq!(
        memory_deltas(&r, cp),
        vec![(0, "OP_MAY_MEM".to_string(), 1)]
    );
    let option_trashed = trash_event_index(&r, cp, "MAIN_PLAY_LV3")
        .expect("GameEvent::Trash for the used Option");
    let on_play = memory_event_index(&r, cp, "OP_MAY_MEM").expect("the [On Play] resolved");
    assert!(
        option_trashed < on_play,
        "event order: the Option's trash ({option_trashed}) precedes the triggered [On Play] \
         ({on_play})"
    );
}

// ─── §18-3 ──────────────────────────────────────────────────────────────────

/// What the §18-3 scenarios report back from their thread.
#[derive(Debug)]
struct LoopOutcome {
    game_over: bool,
    winner: Option<PlayerId>,
    draw_event: bool,
    pending: Option<String>,
    observer_resolved: bool,
}

/// P0 plays `loop_card` (its "[On Play] Suspend this Digimon" starts the
/// loop) while P1's `OBS_OPP_PLAY_MEM` ("[All Turns] When your opponent plays
/// a Digimon, gain 1 memory") triggers on the same play. Returns the runner,
/// the event checkpoint and P0's deck size, both taken just before the play.
fn loop_board(loop_card: &str) -> (DebugRunner, usize, usize) {
    let mut r = ch18()
        .hand(0, &[loop_card])
        .deck(0, &[FILL; 64])
        .deck(1, &[FILL; 4])
        .memory(5)
        .start();
    r.place_on_field(1, "OBS_OPP_PLAY_MEM", Some(0));
    let cp = r.event_checkpoint();
    let deck = r.deck_size(0);
    r.play(0, 0);
    (r, cp, deck)
}

/// §18-3-1 "An infinite loop refers to a state where a set of processing will
/// continue infinitely or must be continued infinitely as a result of
/// processing for a rule or effect, and game progress is no longer possible."
/// §18-3-2 "If an infinite loop occurs and neither player has the ability to
/// stop it, that game ends in a draw."
///
/// P0's `LOOP_FLIP` suspends itself on play; "[All Turns] When this Digimon
/// suspends, unsuspend it" and "[All Turns] When this Digimon unsuspends,
/// suspend it" then re-trigger each other forever, all mandatory: neither
/// player can stop the loop, so the game ends in a draw (`GameOver` with no
/// winner). It must not be silently abandoned with the game going on -- that
/// would also discard P1's `OBS_OPP_PLAY_MEM`, triggered by the same play.
///
/// The scenario runs on its own thread with a time limit, so an engine that
/// loops forever fails the test instead of hanging the suite.
#[test]
fn infinite_loop_is_stopped_not_silently_cleared() {
    cite!(
        "18-3-1",
        "ed4b8654d01a7037421af96f1ccf80ac81741aaff69b040659d881003f0e2d79"
    );
    cite!(
        "18-3-2",
        "989de42a09e690aa1c0e80632ee1aeb9f94eb27d3fdfeff9b09fa784f38f29ca"
    );
    let outcome = within("mandatory loop", Duration::from_secs(30), || {
        let (r, cp, _) = loop_board("LOOP_FLIP");
        LoopOutcome {
            game_over: r.game_over(),
            winner: r.winner(),
            draw_event: r
                .events_since(cp)
                .iter()
                .any(|e| matches!(e, GameEvent::GameOver { winner: None, .. })),
            pending: r.pending_selection().map(|s| format!("{s:?}")),
            observer_resolved: memory_event_index(&r, cp, "OBS_OPP_PLAY_MEM").is_some(),
        }
    });
    assert!(
        outcome.game_over && outcome.winner.is_none() && outcome.draw_event,
        "neither player can stop the loop: the game ends in a draw ({outcome:?})"
    );
}

/// §18-3-3 "If an infinite loop occurs and one of the players has the ability
/// to stop it, use the following procedure. After performing the following
/// processing, it will not be possible to perform the actions for the same
/// infinite loop again."
/// §18-3-3-3 "The processing is executed at least the number of times that was
/// declared it would be repeated, then the player stops the processing when
/// possible."
///
/// `LOOP_FLIP_MAY` is the same loop with "you may suspend it": P0 can stop it.
/// Each suspension trashes the top card of P0's deck, so the deck count
/// witnesses how many times the loop ran. P0 repeats it 40 times -- well past
/// any fixed iteration cap -- then stops. Either prompt shape satisfies
/// §18-3-3-3 as long as the count is honoured:
/// - a repeat-count declaration (§18-3-3-1; a count choice), answered with the
///   smallest offered count of at least 40; or
/// - one optional prompt per repetition.
///
/// In both shapes any "you may suspend it" is accepted until the witness shows
/// the target count of repetitions, then declined (the player stops when
/// possible). After the stop: the loop ran at least the target count, the
/// Digimon is unsuspended (it stopped on "you may suspend it"), the game is not
/// over, and P1's `OBS_OPP_PLAY_MEM`, pending since the play, activates exactly
/// once.
#[test]
fn stoppable_loop_repeats_until_its_owner_stops_it() {
    cite!(
        "18-3-3",
        "a169b8994a199a4ab6a8c59166edc61777c3d6e87c8fe2315c770341d74cd9c1"
    );
    cite!(
        "18-3-3-3",
        "ca1e36ce6f3f4e7299e9b32d4fe477ced92d86d03f8f391235592a23b66a6ca0"
    );
    within("stoppable loop", Duration::from_secs(30), || {
        const REPEATS: u64 = 40;
        let (mut r, cp, deck_before) = loop_board("LOOP_FLIP_MAY");
        // Suspensions since the play, less the [On Play] one: repetitions.
        let repetitions =
            |r: &DebugRunner| ((deck_before - r.deck_size(0)) as u64).saturating_sub(1);
        let mut declared: Option<u64> = None;
        // Hard step bound: the repetitions, a declaration and its pages, the stop.
        for step in 0..200 {
            let Some(sel) = r.pending_selection() else {
                break;
            };
            let player = sel.selecting_player;
            let counts: Vec<(u16, CountChoiceOperation)> = sel
                .effect_choices
                .iter()
                .flatten()
                .filter_map(|c| c.count_choice.map(|m| (c.action_id, m.operation)))
                .collect();
            if !counts.is_empty() {
                // A repeat-count declaration: commit the smallest count >= 40,
                // or page forward to find one.
                let commit = counts
                    .iter()
                    .filter_map(|(a, op)| match op {
                        CountChoiceOperation::Commit(v) if v.declared >= REPEATS => {
                            Some((v.declared, *a))
                        }
                        _ => None,
                    })
                    .min();
                let action = match commit {
                    Some((n, a)) => {
                        declared = Some(n);
                        a
                    }
                    None => counts
                        .iter()
                        .find(|(_, op)| *op == CountChoiceOperation::NextPage)
                        .map(|(a, _)| *a)
                        .unwrap_or_else(|| panic!("no count of at least {REPEATS}: {sel:?}")),
                };
                r.execute_action(player, action)
                    .unwrap_or_else(|e| panic!("declaration rejected: {e:?}"));
            } else if sel.is_optional && player == 0 {
                if repetitions(&r) < declared.unwrap_or(REPEATS) {
                    accept(&mut r, 0);
                } else {
                    r.decline_optional_trigger().expect("P0 stops the loop");
                }
            } else {
                panic!("step {step}: unexpected decision: {sel:?}");
            }
        }
        assert!(
            r.pending_selection().is_none(),
            "{:?}",
            r.pending_selection()
        );
        let target = declared.unwrap_or(REPEATS);
        let ran = repetitions(&r);
        assert!(
            ran >= target,
            "the loop ran {ran} times before it stopped; {target} were {} (P1's observer \
             resolved: {})",
            if declared.is_some() {
                "declared"
            } else {
                "chosen, with no count declaration offered"
            },
            memory_event_index(&r, cp, "OBS_OPP_PLAY_MEM").is_some()
        );
        assert!(!r.game_over(), "a stoppable loop does not end the game");
        let loop_perm = r.perm_handle(0, 0);
        assert!(
            !is_suspended(&r, loop_perm),
            "stopped on 'you may suspend it': it stays unsuspended"
        );
        // The gauge moves 1 toward P1 during P0's turn, so the turn-player-signed delta is -1 (controller-authorised correction of a mis-authored expectation; the test had never run).
        assert_eq!(
            memory_deltas(&r, cp),
            vec![(1, "OBS_OPP_PLAY_MEM".to_string(), -1)],
            "the trigger pending since the play activates once the loop stops"
        );
    });
}

/// §18-3-2 "If an infinite loop occurs and neither player has the ability to
/// stop it, that game ends in a draw."
///
/// A forced decision is not the ability to stop a loop: §18-3-3-3's stop is an
/// optional decision. P0's `LOOP_FLIP_PICK` suspends itself on play; "[All
/// Turns] When this Digimon suspends, unsuspend 1 of your Digimon" (with this
/// Digimon P0's only Digimon, a mandatory pick with one option) and "[All
/// Turns] When this Digimon unsuspends, suspend it" re-trigger each other
/// forever. P0 answers every pick (the only legal answer); the board returns
/// to the same state each time, so the game still ends in a draw
/// (`InfiniteLoop`, then `GameOver` with no winner) instead of asking the same
/// forced question forever.
#[test]
fn forced_prompt_loop_still_ends_in_a_draw() {
    cite!(
        "18-3-2",
        "989de42a09e690aa1c0e80632ee1aeb9f94eb27d3fdfeff9b09fa784f38f29ca"
    );
    within("forced-prompt loop", Duration::from_secs(30), || {
        let (mut r, cp, _) = loop_board("LOOP_FLIP_PICK");
        let mut picks = 0;
        // Hard step bound: a correct engine draws after a few repetitions.
        for _ in 0..100 {
            if r.game_over() {
                break;
            }
            let Some(sel) = r.pending_selection() else {
                break;
            };
            assert!(!sel.is_optional, "the pick is mandatory: {sel:?}");
            assert_eq!(
                sel.valid_action_ids.len(),
                1,
                "the pick has a single option: {sel:?}"
            );
            let (player, action) = (sel.selecting_player, sel.valid_action_ids[0]);
            r.execute_action(player, action)
                .unwrap_or_else(|e| panic!("forced pick rejected: {e:?}"));
            picks += 1;
        }
        assert!(picks >= 1, "the forced pick is offered");
        let events = r.events_since(cp);
        let loop_at = events
            .iter()
            .position(|e| matches!(e, GameEvent::InfiniteLoop { .. }));
        let over_at = events
            .iter()
            .position(|e| matches!(e, GameEvent::GameOver { winner: None, .. }));
        assert!(
            r.game_over() && r.winner().is_none(),
            "a forced-prompt loop nobody can stop ends in a draw ({picks} picks; pending: {:?})",
            r.pending_selection()
        );
        assert!(
            matches!((loop_at, over_at), (Some(l), Some(o)) if l < o),
            "InfiniteLoop then GameOver {{ winner: None }}: {loop_at:?} / {over_at:?}"
        );
    });
}

/// §18-3-2 when the state never repeats and every repetition asks a forced
/// question. `LOOP_PICK_GROW` gains +1000 DP on each suspension (so no loop
/// signature ever recurs) and asks a single-option mandatory "unsuspend 1 of
/// your Digimon" pick each time (so the chain-depth count restarts every
/// repetition). Neither player can stop it; the far backstop on the loop
/// watch's age (`MAX_CHAIN_DEPTH` steps with no optional decision) still ends
/// the game in exactly one draw, reported as a backstop `InfiniteLoop`.
#[test]
fn forced_prompt_loop_with_growing_state_still_ends_in_a_draw() {
    cite!(
        "18-3-2",
        "989de42a09e690aa1c0e80632ee1aeb9f94eb27d3fdfeff9b09fa784f38f29ca"
    );
    // Load-sensitive: ~60 s idle; 300 s keeps parallel-build runs green.
    within("growing forced loop", Duration::from_secs(300), || {
        let (mut r, cp, _) = loop_board("LOOP_PICK_GROW");
        let mut picks = 0usize;
        for _ in 0..20_000 {
            if r.game_over() {
                break;
            }
            let Some(sel) = r.pending_selection() else {
                break;
            };
            assert!(!sel.is_optional, "the pick is mandatory: {sel:?}");
            assert_eq!(sel.valid_action_ids.len(), 1, "one option: {sel:?}");
            let (player, action) = (sel.selecting_player, sel.valid_action_ids[0]);
            r.execute_action(player, action)
                .unwrap_or_else(|e| panic!("forced pick rejected: {e:?}"));
            picks += 1;
        }
        let events = r.events_since(cp);
        let overs: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                GameEvent::GameOver { winner, reason, .. } => Some((*winner, *reason)),
                _ => None,
            })
            .collect();
        assert_eq!(
            overs,
            vec![(None, TerminalOutcomeReason::InfiniteLoopDraw)],
            "one GameOver, the draw (after {picks} forced picks)"
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e, GameEvent::InfiniteLoop { backstop: true, .. })),
            "reported by the backstop"
        );
    });
}

/// The two-Digimon variant of `forced_prompt_loop_still_ends_in_a_draw`: with a
/// second Digimon on P0's field the mandatory "unsuspend 1 of your Digimon"
/// pick has two options. A mandatory pick is still not the optional decision
/// §18-3-3-3 stops a loop with, so it keeps the loop watch: P0 always picks the
/// looping Digimon, the same state recurs, and the game ends in the §18-3-2
/// draw.
#[test]
fn forced_multi_option_pick_loop_still_ends_in_a_draw() {
    cite!(
        "18-3-2",
        "989de42a09e690aa1c0e80632ee1aeb9f94eb27d3fdfeff9b09fa784f38f29ca"
    );
    within("forced multi-option loop", Duration::from_secs(30), || {
        let mut r = ch18()
            .hand(0, &["LOOP_FLIP_PICK"])
            .deck(0, &[FILL; 4])
            .deck(1, &[FILL; 4])
            .memory(5)
            .start();
        r.place_on_field(0, FILL, Some(0));
        let cp = r.event_checkpoint();
        r.play(0, 0);
        let loop_perm = r.perm_handle(0, 1);
        let mut picks = 0;
        for _ in 0..200 {
            if r.game_over() {
                break;
            }
            let Some(sel) = r.pending_selection() else {
                break;
            };
            assert!(!sel.is_optional, "the pick is mandatory: {sel:?}");
            assert_eq!(sel.valid_action_ids.len(), 2, "two options: {sel:?}");
            let player = sel.selecting_player;
            let action = offered_own_slot_action(&r, loop_perm.index);
            r.execute_action(player, action)
                .unwrap_or_else(|e| panic!("pick rejected: {e:?}"));
            picks += 1;
        }
        assert!(picks >= 1, "the pick is offered");
        assert!(
            r.game_over() && r.winner().is_none(),
            "a loop of mandatory picks ends in a draw ({picks} picks; pending: {:?})",
            r.pending_selection()
        );
        assert!(
            r.events_since(cp)
                .iter()
                .any(|e| matches!(e, GameEvent::InfiniteLoop { .. })),
            "the loop is reported"
        );
    });
}

/// The action that picks P0's own Digimon in battle-area slot `slot` from the
/// pending own-field selection.
fn offered_own_slot_action(r: &DebugRunner, slot: u8) -> u16 {
    let sel = r.pending_selection().expect("an own-field pick");
    let base = *sel.valid_action_ids.iter().min().expect("options");
    let action = base + u16::from(slot);
    assert!(
        sel.valid_action_ids.contains(&action),
        "slot {slot} is offered: {sel:?}"
    );
    action
}

/// §18-3-2, for a forced ORDERING decision. `LOOP_TWIN_GUARD`'s every
/// repetition raises two mandatory immediate-type effects on the same "would
/// be deleted" (§15-8-5-3: its player chooses their order), then re-suspends,
/// unsuspends and deletes itself again. Choosing an order cannot stop the loop,
/// so it is not the ability to stop it: P0 answers every prompt with its first
/// option and the game ends in a draw.
#[test]
fn forced_replacement_order_loop_still_ends_in_a_draw() {
    cite!(
        "18-3-2",
        "989de42a09e690aa1c0e80632ee1aeb9f94eb27d3fdfeff9b09fa784f38f29ca"
    );
    within("forced order loop", Duration::from_secs(30), || {
        let mut r = ch18()
            .hand(0, &["LOOP_TWIN_GUARD"])
            .deck(0, &[FILL; 4])
            .deck(1, &[FILL; 4])
            .memory(5)
            .start();
        let cp = r.event_checkpoint();
        r.play(0, 0);
        let mut orders = 0;
        for _ in 0..300 {
            if r.game_over() {
                break;
            }
            let Some(sel) = r.pending_selection() else {
                break;
            };
            assert!(!sel.is_optional, "every prompt is forced: {sel:?}");
            if sel.valid_action_ids.len() >= 2 {
                orders += 1;
            }
            let (player, action) = (sel.selecting_player, sel.valid_action_ids[0]);
            r.execute_action(player, action)
                .unwrap_or_else(|e| panic!("prompt rejected: {e:?}"));
        }
        assert!(orders >= 1, "the order of the two effects is asked");
        assert!(
            r.game_over() && r.winner().is_none(),
            "a loop whose only decisions are forced orderings ends in a draw ({orders} orders; pending: {:?})",
            r.pending_selection()
        );
        assert!(
            r.events_since(cp)
                .iter()
                .any(|e| matches!(e, GameEvent::InfiniteLoop { .. })),
            "the loop is reported"
        );
    });
}

/// §18-3-3-3, with the stop authored as the DSL's effect-text choose-one ("you
/// may X instead" is written that way) instead of an optional trigger.
/// `LOOP_FLIP_CHOOSE`: "When this Digimon unsuspends, choose one: suspend it;
/// or do nothing." The choice is the ability to stop the loop, so the same
/// state recurring is not a draw: P0 repeats it 30 times, then chooses "Do
/// nothing", and the loop stops with the game going on.
#[test]
fn choose_one_loop_repeats_until_its_owner_stops_it() {
    cite!(
        "18-3-3-3",
        "ca1e36ce6f3f4e7299e9b32d4fe477ced92d86d03f8f391235592a23b66a6ca0"
    );
    within("choose-one loop", Duration::from_secs(30), || {
        const REPEATS: usize = 30;
        let mut r = ch18()
            .hand(0, &["LOOP_FLIP_CHOOSE"])
            .deck(0, &[FILL; 4])
            .deck(1, &[FILL; 4])
            .memory(5)
            .start();
        let cp = r.event_checkpoint();
        r.play(0, 0);
        let mut chosen = 0;
        for _ in 0..200 {
            assert!(!r.game_over(), "a stoppable loop does not end the game");
            if r.pending_selection().is_none() {
                break;
            }
            let label = if chosen < REPEATS { "Suspend it" } else { "Do nothing" };
            pick_branch(&mut r, label);
            chosen += 1;
        }
        assert_eq!(chosen, REPEATS + 1, "the choice is offered every repetition");
        assert!(r.pending_selection().is_none());
        assert!(!r.game_over(), "a stoppable loop does not end the game");
        assert!(
            !r.events_since(cp)
                .iter()
                .any(|e| matches!(e, GameEvent::InfiniteLoop { .. })),
            "no loop was declared"
        );
        let loop_perm = r.perm_handle(0, 0);
        assert!(
            !is_suspended(&r, loop_perm),
            "stopped on 'do nothing': it stays unsuspended"
        );
    });
}

/// §18-3-2 is final: a draw declared while an attack is resolving ends the
/// attack too. P0's `WA_LOOP` attacks P1; the attack's suspension and its
/// "[When Attacking] Unsuspend this Digimon" start a mandatory loop (P0 only
/// orders the two simultaneous triggers, a forced decision), and the game ends
/// in a draw right there. No later attack step may overwrite that
/// result: against no security cards there is exactly one `GameOver` (no
/// winner, `InfiniteLoopDraw`), and against one there is also no security
/// check after it.
#[test]
fn infinite_loop_draw_during_an_attack_is_final() {
    cite!(
        "18-3-2",
        "989de42a09e690aa1c0e80632ee1aeb9f94eb27d3fdfeff9b09fa784f38f29ca"
    );
    for security in [0usize, 1] {
        within("loop during an attack", Duration::from_secs(30), move || {
            let mut r = ch18()
                .deck(0, &[FILL; 4])
                .deck(1, &[FILL; 4])
                .security(1, &vec![FILL; security])
                .memory(5)
                .start();
            let attacker = r.place_on_field(0, "WA_LOOP", Some(0));
            let cp = r.event_checkpoint();
            r.attack_player(attacker, 1, false);
            // The attack's suspension and its [When Attacking] effect trigger
            // together: P0 orders them (a forced decision), then the loop runs.
            for _ in 0..100 {
                if r.game_over() {
                    break;
                }
                let Some(sel) = r.pending_selection() else {
                    break;
                };
                assert!(!sel.is_optional, "only forced decisions: {sel:?}");
                let (player, action) = (sel.selecting_player, sel.valid_action_ids[0]);
                r.execute_action(player, action)
                    .unwrap_or_else(|e| panic!("prompt rejected: {e:?}"));
            }
            let events = r.events_since(cp);
            let overs: Vec<_> = events
                .iter()
                .filter_map(|e| match e {
                    GameEvent::GameOver { winner, reason, .. } => Some((*winner, *reason)),
                    _ => None,
                })
                .collect();
            assert_eq!(
                overs,
                vec![(None, TerminalOutcomeReason::InfiniteLoopDraw)],
                "{security} security: one GameOver, the draw"
            );
            assert!(r.game_over() && r.winner().is_none(), "{security} security");
            let loop_at = events
                .iter()
                .position(|e| matches!(e, GameEvent::InfiniteLoop { .. }))
                .expect("the loop is reported");
            assert!(
                !events[loop_at..]
                    .iter()
                    .any(|e| matches!(e, GameEvent::SecurityReveal { .. })),
                "{security} security: no security check after the draw"
            );
        });
    }
}

/// §18-3-1: a chain whose only progress is DP is still finite. `DP_CHAIN`'s
/// "When this Digimon suspends, it gets +1000 DP for the turn. Then, if it has
/// 29000 DP or less, unsuspend it." and "When this Digimon unsuspends, suspend
/// it." re-trigger each other 27 times from 3000 DP: the zones and suspension
/// repeat every cycle, but the DP does not, and the chain stops at 30000 DP.
/// It runs to its end with no infinite-loop draw.
#[test]
fn dp_progress_chain_is_not_an_infinite_loop() {
    cite!(
        "18-3-1",
        "ed4b8654d01a7037421af96f1ccf80ac81741aaff69b040659d881003f0e2d79"
    );
    within("DP chain", Duration::from_secs(30), || {
        let mut r = ch18()
            .hand(0, &["DP_CHAIN"])
            .deck(0, &[FILL; 4])
            .deck(1, &[FILL; 4])
            .memory(5)
            .start();
        let cp = r.event_checkpoint();
        r.play(0, 0);
        assert!(r.pending_selection().is_none(), "{:?}", r.pending_selection());
        assert!(!r.game_over(), "a finite chain does not end the game");
        assert!(
            !r.events_since(cp)
                .iter()
                .any(|e| matches!(e, GameEvent::InfiniteLoop { .. })),
            "no loop was declared"
        );
        let h = r.perm_handle(0, 0);
        assert_eq!(r.effective_dp(h), Some(30000), "every link ran");
        assert!(is_suspended(&r, h), "the chain ends on its last suspension");
    });
}

/// §18-3-1 "An infinite loop refers to a state where a set of processing will
/// continue infinitely or must be continued infinitely as a result of
/// processing for a rule or effect, and game progress is no longer possible."
///
/// A long chain that ends by itself is not an infinite loop, however many
/// times its effects re-trigger each other. P0's `LOOP_FLIP_FINITE` suspends
/// itself on play; "[All Turns] When this Digimon suspends, trash the top card
/// of your deck. Then, if you have a card in your deck, unsuspend it." and
/// "[All Turns] When this Digimon unsuspends, suspend it." (all mandatory)
/// re-trigger each other once per card of P0's 60-card deck: some 120 chained
/// resolutions, far past any small iteration cap, after which the deck is
/// empty and the chain stops. Every link runs (the deck ends empty and the
/// Digimon ends suspended, on its last suspension), the game goes on with no
/// infinite-loop event and no `GameOver`, and P1's `OBS_OPP_PLAY_MEM`,
/// triggered by the same play, activates once the chain has ended.
#[test]
fn long_finite_chain_is_not_an_infinite_loop() {
    cite!(
        "18-3-1",
        "ed4b8654d01a7037421af96f1ccf80ac81741aaff69b040659d881003f0e2d79"
    );
    within("finite chain", Duration::from_secs(30), || {
        let mut r = ch18()
            .hand(0, &["LOOP_FLIP_FINITE"])
            .deck(0, &[FILL; 60])
            .deck(1, &[FILL; 4])
            .memory(5)
            .start();
        r.place_on_field(1, "OBS_OPP_PLAY_MEM", Some(0));
        let cp = r.event_checkpoint();
        r.play(0, 0);
        assert!(
            r.pending_selection().is_none(),
            "a mandatory chain asks nothing: {:?}",
            r.pending_selection()
        );
        assert_eq!(r.deck_size(0), 0, "every link of the chain ran");
        let loop_perm = r.perm_handle(0, 0);
        assert!(
            is_suspended(&r, loop_perm),
            "the chain ends on its last suspension"
        );
        assert!(!r.game_over(), "a finite chain does not end the game");
        let terminal: Vec<&str> = r
            .events_since(cp)
            .iter()
            .map(|e| e.type_str())
            .filter(|t| matches!(*t, "InfiniteLoop" | "GameOver"))
            .collect();
        assert!(terminal.is_empty(), "no loop was detected: {terminal:?}");
        // Who and how often, not the sign: `MemoryChange.delta` is the gauge's
        // delta from the turn player's side (events.rs), so P1's gain during
        // P0's turn reads -1.
        let deltas = memory_deltas(&r, cp);
        assert_eq!(
            deltas
                .iter()
                .map(|(p, id, _)| (*p, id.as_str()))
                .collect::<Vec<_>>(),
            vec![(1, "OBS_OPP_PLAY_MEM")],
            "the trigger pending since the play activates once the chain ends: {deltas:?}"
        );
    });
}

// ─── §16-7-4 / §16-7-5 / §16-7-6 ────────────────────────────────────────────

/// P0's `attacker` stack (top last) attacks and deletes P1's suspended
/// `defender` (3000 DP); P1 has 3 security cards.
fn piercing_board(attacker: &[&str], defender: &str) -> (DebugRunner, usize) {
    let mut r = ch18()
        .deck(0, &[FILL; 4])
        .deck(1, &[FILL; 4])
        .security(1, &[FILL; 3])
        .memory(5)
        .start();
    let attacker = r.place_stack(0, attacker);
    let defender = r.place_on_field(1, defender, Some(0));
    set_suspended(&mut r, defender, true);
    let cp = r.event_checkpoint();
    r.attack_digimon(attacker, defender, false);
    (r, cp)
}

/// §16-7-4 "<Piercing> is pending processing that is processed immediately
/// before the transition to the end of attack timing. Therefore, it will
/// trigger simultaneously with effects triggered by the battle, but it is
/// processed before the end of the attack. (Example: If a Digimon with
/// <Piercing> attacks and deletes a Digimon with an [On Deletion] effect in
/// the battle, first the [On Deletion] effect is resolved, then <Piercing> is
/// processed.)"
///
/// The rule's own example. P0's `PIERCE_EOA` (<Piercing>, "[End of Attack]
/// Gain 1 memory") deletes P1's `OD_MEM` ("[On Deletion] Gain 1 memory") in
/// battle. Order: the [On Deletion] resolves, then the <Piercing> security
/// check, then the [End of Attack] effect.
#[test]
fn piercing_is_pending_processing_before_end_of_attack() {
    cite!(
        "16-7-4",
        "b5591590a21a8204243c527c38e7b651c10bb88a277084b7153a0d3babc80d91"
    );
    let (mut r, cp) = piercing_board(&["PIERCE_EOA"], "OD_MEM");
    resolve_prompts(&mut r);
    assert!(
        field_top_ids(&r, 1).is_empty(),
        "OD_MEM was deleted in battle"
    );
    let on_deletion = memory_event_index(&r, cp, "OD_MEM").expect("the [On Deletion] resolved");
    let checks = security_reveal_indices(&r, cp, 1);
    assert_eq!(checks.len(), 1, "<Piercing> checked 1 security card");
    let end_of_attack =
        memory_event_index(&r, cp, "PIERCE_EOA").expect("the [End of Attack] resolved");
    assert!(
        on_deletion < checks[0] && checks[0] < end_of_attack,
        "[On Deletion] (#{on_deletion}) -> <Piercing> check (#{}) -> [End of Attack] \
         (#{end_of_attack})",
        checks[0]
    );
    assert_eq!(r.security_count(1), 2);
}

/// §16-7-5 "Only 1 security check can be performed during a single attack,
/// even if <Piercing> activates multiple times. However, if the number of
/// cards that can be checked by the attacking Digimon is modified, that number
/// of cards will be checked in a single security check."
///
/// P0's attacker has two <Piercing> instances: its own (`PIERCE_EOA`) and an
/// inherited one (`INH_PIERCE`, its digivolution card). Deleting P1's
/// `FILL` in battle yields ONE security check of 1 card.
///
/// A regression pin only, so 16-7-5 stays a gap in the sidecar: the engine
/// gates the <Piercing> check on a boolean `has_keyword`, so this test cannot
/// tell one instance from two, and the second sentence (a modified check
/// count, e.g. <Security A. +1> on the Piercing attacker) is not staged.
#[test]
fn piercing_checks_security_once_however_many_instances() {
    cite!(
        "16-7-5",
        "17fb451c882263bc48e6451c841935dbb5263e4d5aa17ee704b94c8f4c47ca82"
    );
    let (mut r, cp) = piercing_board(&["INH_PIERCE", "PIERCE_EOA"], FILL);
    resolve_prompts(&mut r);
    assert!(field_top_ids(&r, 1).is_empty(), "the defender was deleted");
    assert_eq!(
        security_reveal_indices(&r, cp, 1).len(),
        1,
        "one security check for the attack"
    );
    assert_eq!(r.security_count(1), 2);
}

/// §16-7-6 "If there are no Digimon to perform a security check at the point
/// when <Piercing> is processed, a security check won't be possible."
///
/// P0's `PIERCE_EOA` deletes P1's `OD_DEL_OPP` ("[On Deletion] Delete 1 of
/// your opponent's Digimon") in battle; its [On Deletion] deletes the
/// attacker. When <Piercing> is processed there is no Digimon to check
/// security: nothing is checked.
#[test]
fn piercing_without_the_attacker_checks_nothing() {
    cite!(
        "16-7-6",
        "4c3da67a57b8a12572c04b9bcb88c0cf16f0524b4705d9a2228e6bb2c008f839"
    );
    let (mut r, cp) = piercing_board(&["PIERCE_EOA"], "OD_DEL_OPP");
    resolve_prompts(&mut r); // the [On Deletion] target: P0's only Digimon
    assert!(field_top_ids(&r, 0).is_empty(), "the attacker was deleted");
    assert!(
        security_reveal_indices(&r, cp, 1).is_empty(),
        "no Digimon to check security with"
    );
    assert_eq!(r.security_count(1), 3);
}

// ─── §9-1-5: an Option an EFFECT uses; an Option with two [Main] effects ────

/// §9-1-5 (as above). The Option is used by an EFFECT, from inside that
/// effect's own selection callback (the EX7-073 / P-180 shape): P0's
/// `OP_USE_OPT_FREE` ("[On Play] You may use 1 Option card from your hand
/// without paying the cost") uses `MAIN_ALL_MINUS3000` ("[Main] All of your
/// opponent's Digimon get -3000 DP for the turn" -- it parks no decision, so
/// its whole body runs inside the hand-pick callback), which leaves P1's
/// 3000 DP `WOULD_DEL_MAY_MEM` at 0 DP. The rule check that follows must find
/// the Option already in the trash: when the 0 DP Digimon's would-be-deleted
/// decision parks, `MAIN_ALL_MINUS3000` is in P0's trash, and in the event
/// log its trash precedes the would-be-deleted effect and the deletion.
///
/// Red on the base engine (83aa6bf02, Task 7 merged): the body ran with
/// `draining_deferred > 0`, the OptionMain arm's disposal was skipped, and
/// the rule check between queued effects deleted the Digimon first -- the
/// trash was empty at the would-be-deleted decision. (An Option whose [Main]
/// parks a pick does not show it: its body then finishes inside the pick's
/// own resolution, at depth 0, where the old gate already disposed it.)
#[test]
fn option_used_by_an_effect_is_trashed_before_the_rule_check_its_main_caused() {
    cite!(
        "9-1-5",
        "2886ba315f2847fdd735e7c839b0b9c80cd0bf7391a1596460851d82cc7eb003"
    );
    let mut r = fixtures::builder(&[
        fixtures::ch15_08_cards(),
        fixtures::ch17_cards(),
        fixtures::ch18_cards(),
    ])
    .hand(0, &["MAIN_ALL_MINUS3000"])
    .deck(0, &[FILL; 4])
    .deck(1, &[FILL; 4])
    .memory(3)
    .start();
    r.place_on_field(1, "WOULD_DEL_MAY_MEM", Some(0));
    let user = r.place_on_field(0, "OP_USE_OPT_FREE", Some(0));
    let cp = r.event_checkpoint();
    r.fire_on_play(0, user.index as usize);
    pick_hand_card(&mut r, 0, "MAIN_ALL_MINUS3000");
    let sel = r
        .pending_selection()
        .expect("the 0 DP Digimon's would-be-deleted decision");
    assert!(
        sel.is_optional && sel.selecting_player == 1,
        "the rule check is about to delete WOULD_DEL_MAY_MEM: {sel:?}"
    );
    assert_eq!(
        field_top_ids(&r, 1),
        owned(&["WOULD_DEL_MAY_MEM"]),
        "not deleted yet"
    );
    assert_eq!(
        trash_ids(&r, 0),
        owned(&["MAIN_ALL_MINUS3000"]),
        "used from inside an effect's selection callback: the Option was still trashed before \
         the rule check"
    );
    accept(&mut r, 1);
    resolve_prompts(&mut r);
    assert!(field_top_ids(&r, 1).is_empty(), "the 0 DP Digimon was deleted");
    let option_trashed = trash_event_index(&r, cp, "MAIN_ALL_MINUS3000")
        .expect("GameEvent::Trash for the used Option");
    let would_del = memory_event_index(&r, cp, "WOULD_DEL_MAY_MEM")
        .expect("the would-be-deleted effect gained 1 memory");
    let deleted = trash_event_index(&r, cp, "WOULD_DEL_MAY_MEM")
        .expect("GameEvent::Trash for the deleted Digimon");
    assert!(
        option_trashed < would_del && option_trashed < deleted,
        "event order: the Option's trash ({option_trashed}) precedes the rule check's \
         would-be-deleted effect ({would_del}) and deletion ({deleted})"
    );
}

/// §9-1-5 "... as soon as the timing when its 1st [Main] effect has been
/// resolved". P0's `MAIN_TWO` has two [Main] effects: "1 of your opponent's
/// Digimon gets -3000 DP for the turn" and "Gain 1 memory". After the FIRST
/// has been resolved the card is trashed; the rule check then deletes the
/// 0 DP `WOULD_DEL_MAY_MEM` (its would-be-deleted decision parks with the
/// Option already in the trash and the second [Main] not yet resolved), and
/// the second [Main] resolves afterwards, as the pending effect of a card in
/// the trash: the event log shows the Option's trash, then the would-be-
/// deleted effect's memory, then the second [Main]'s memory.
#[test]
fn option_with_two_mains_is_trashed_after_the_first() {
    cite!(
        "9-1-5",
        "2886ba315f2847fdd735e7c839b0b9c80cd0bf7391a1596460851d82cc7eb003"
    );
    let mut r = fixtures::builder(&[
        fixtures::ch15_08_cards(),
        fixtures::ch17_cards(),
        fixtures::ch18_cards(),
    ])
    .hand(0, &["MAIN_TWO"])
    .deck(0, &[FILL; 4])
    .deck(1, &[FILL; 4])
    .memory(3)
    .start();
    r.place_on_field(0, FILL, Some(0));
    r.place_on_field(1, "WOULD_DEL_MAY_MEM", Some(0));
    let cp = r.event_checkpoint();
    use_option(&mut r, 0, "MAIN_TWO");
    pick_field(&mut r, 0);
    let sel = r
        .pending_selection()
        .expect("the 0 DP Digimon's would-be-deleted decision");
    assert!(
        sel.is_optional && sel.selecting_player == 1,
        "the rule check is about to delete WOULD_DEL_MAY_MEM: {sel:?}"
    );
    assert_eq!(
        trash_ids(&r, 0),
        owned(&["MAIN_TWO"]),
        "trashed as soon as its 1st [Main] was resolved"
    );
    assert!(
        memory_event_index(&r, cp, "MAIN_TWO").is_none(),
        "the 2nd [Main] has not resolved yet"
    );
    accept(&mut r, 1);
    resolve_prompts(&mut r);
    assert!(field_top_ids(&r, 1).is_empty(), "the 0 DP Digimon was deleted");
    let option_trashed = trash_event_index(&r, cp, "MAIN_TWO")
        .expect("GameEvent::Trash for the used Option");
    let would_del = memory_event_index(&r, cp, "WOULD_DEL_MAY_MEM")
        .expect("the would-be-deleted effect gained 1 memory");
    let second_main = memory_event_index(&r, cp, "MAIN_TWO")
        .expect("the 2nd [Main] gained 1 memory");
    assert!(
        option_trashed < would_del && would_del < second_main,
        "event order: trash ({option_trashed}) < would-be-deleted ({would_del}) < 2nd [Main] \
         ({second_main})"
    );
}
