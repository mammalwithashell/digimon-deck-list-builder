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
//! `ch15_08` cards `EOT_MEM`, `LV4_PLAIN` and `PLAIN_TAMER`), one synthetic
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
use digimon_engine::debug_runner::{DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{GamePhase, PlayerId};
use digimon_engine::events::GameEvent;
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
    std::thread::Builder::new()
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
        Err(RecvTimeoutError::Disconnected) => panic!("{what}: the scenario thread panicked"),
    }
}

// ─── §18-1-1 / §18-1-2 ──────────────────────────────────────────────────────

/// P0's `WA_GAIN3_LOSE3_EOT` (the §18-1-1 example text) attacks and deletes
/// P1's suspended `FILL`, starting from memory 0; `with_eot_mem` adds P0's
/// Tamer `EOT_MEM` ("[End of Your Turn] Gain 2 memory").
fn pending_loss_board(with_eot_mem: bool) -> (DebugRunner, usize) {
    let mut r = ch18()
        .deck(0, &[FILL; 4])
        .deck(1, &[FILL; 4])
        .memory(0)
        .start();
    if with_eot_mem {
        r.place_on_field(0, "EOT_MEM", Some(0));
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
#[ignore = "G-ENGINE-PENDING-PROCESSING-OUTSIDE-TRIGGER-POOL (§18-1-2)"]
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
    settle_after_option(&mut r);
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
    settle_after_option(&mut r);
    assert_eq!(field_top_ids(&r, 1), owned(&["OP_MEM", "LV5_PLAIN"]));

    // A lone 7000 DP Digimon.
    let mut r = instead_board(false, &["LV5_PLAIN"]);
    settle_after_option(&mut r);
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
    settle_after_option(&mut r);
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
/// interrupting processing is under way (the engine logs no event for the
/// Option's trash):
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
#[ignore = "F-ENGINE-EFFECT-USED-OPTION-MAIN-IS-ORDERABLE (§9-1-5)"]
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
    accept(&mut r, 1);
    resolve_prompts(&mut r);
    assert!(
        field_top_ids(&r, 1).is_empty(),
        "the 0 DP Digimon was deleted"
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
    accept(&mut r, 0);
    resolve_prompts(&mut r);
    assert_eq!(
        memory_deltas(&r, cp),
        vec![(0, "OP_MAY_MEM".to_string(), 1)]
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
/// a Digimon, gain 1 memory") triggers on the same play.
fn loop_board(loop_card: &str) -> (DebugRunner, usize) {
    let mut r = ch18()
        .hand(0, &[loop_card])
        .deck(0, &[FILL; 4])
        .deck(1, &[FILL; 4])
        .memory(5)
        .start();
    r.place_on_field(1, "OBS_OPP_PLAY_MEM", Some(0));
    let cp = r.event_checkpoint();
    r.play(0, 0);
    (r, cp)
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
#[ignore = "G-ENGINE-INFINITE-LOOP-SILENTLY-CLEARED (§18-3-2)"]
fn infinite_loop_is_stopped_not_silently_cleared() {
    cite!(
        "18-3-1",
        "ed4b8654d01a7037421af96f1ccf80ac81741aaff69b040659d881003f0e2d79"
    );
    cite!(
        "18-3-2",
        "989de42a09e690aa1c0e80632ee1aeb9f94eb27d3fdfeff9b09fa784f38f29ca"
    );
    let outcome = within("mandatory loop", Duration::from_secs(120), || {
        let (r, cp) = loop_board("LOOP_FLIP");
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
/// `LOOP_FLIP_MAY` is the same loop with "you may suspend it": P0 can stop it,
/// and each acceptance repeats it once more. P0 repeats it 40 times -- well
/// past any fixed iteration cap -- then stops. Every repetition must still be
/// offered, and once the loop stops, P1's `OBS_OPP_PLAY_MEM`, pending since
/// the play, still activates.
///
/// (§18-3-3-1/-2's up-front declaration of a repeat count is not modelled:
/// a per-repetition choice lets the player realise any declared count.)
#[test]
#[ignore = "G-ENGINE-INFINITE-LOOP-SILENTLY-CLEARED (§18-3-3)"]
fn stoppable_loop_repeats_until_its_owner_stops_it() {
    cite!(
        "18-3-3",
        "a169b8994a199a4ab6a8c59166edc61777c3d6e87c8fe2315c770341d74cd9c1"
    );
    cite!(
        "18-3-3-3",
        "ca1e36ce6f3f4e7299e9b32d4fe477ced92d86d03f8f391235592a23b66a6ca0"
    );
    const REPEATS: usize = 40;
    let (mut r, cp) = loop_board("LOOP_FLIP_MAY");
    for i in 0..REPEATS {
        let sel = r.pending_selection().unwrap_or_else(|| {
            panic!(
                "repetition {i} of {REPEATS} was not offered: the loop was abandoned (P1's \
                 observer resolved: {})",
                memory_event_index(&r, cp, "OBS_OPP_PLAY_MEM").is_some()
            )
        });
        assert!(sel.is_optional, "repetition {i}: {sel:?}");
        assert_eq!(sel.selecting_player, 0, "repetition {i}: {sel:?}");
        accept(&mut r, 0);
    }
    assert!(
        memory_event_index(&r, cp, "OBS_OPP_PLAY_MEM").is_none(),
        "P1's observer waits while the loop goes on"
    );
    r.decline_optional_trigger().expect("P0 stops the loop");
    resolve_prompts(&mut r);
    assert!(!r.game_over(), "a stoppable loop does not end the game");
    let loop_perm = r.perm_handle(0, 0);
    assert!(
        !is_suspended(&r, loop_perm),
        "stopped after an unsuspend: it stays unsuspended"
    );
    assert_eq!(
        memory_deltas(&r, cp),
        vec![(1, "OBS_OPP_PLAY_MEM".to_string(), 1)],
        "the trigger pending since the play still activates"
    );
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
/// `FILL` in battle yields ONE security check of 1 card. (The second
/// sentence, a modified check count, is not staged here.)
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
