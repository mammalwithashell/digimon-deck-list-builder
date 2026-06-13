//! BT25-098 Cyber Engage — Option, Yellow, Cost 3, [Appmon].
//!
//! # Card text (cards.json)
//! <Use Req. ([Appmon] trait)> (Specified cards let you ignore color
//!   requirements.)
//! [Main] Reveal the top 3 cards of your deck. Add 1 [Appmon] trait card among
//!   them to the hand. Trash the rest. Then, place this card in the battle area.
//! [Main] <Delay> (By trashing this card after the placing turn, activate the
//!   effect below.)
//! ・You may play 1 [Appmon] trait card from your hand with the cost reduced
//!   by 3.
//! Inherited: [Security] Place this card in the battle area.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Yellow/BT25_098.cs
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - D3 color ignore / bypass (Use Req. flood gate, IgnoreColorRequirement)
//! - Group E: reveal-N single pick (add 1 [Appmon] to hand, trash the rest)
//! - Option pipeline: [Main] places self as a standard <Delay> battle-area
//!   Option ("Then, place this card in the battle area")
//! - Standard <Delay> activation after the placing turn: self-trash cost +
//!   optional "you may play 1 [Appmon] ... cost reduced by 3"
//! - Inherited [Security] places self in battle area
//!
//! AUDIT note (batch-implement-cards-rust-dsl, drift FIXED 2026-06-12): the
//! reveal "Add 1 [Appmon] ... among them" pick is MANDATORY when an [Appmon] is
//! among the revealed three (printed text has no "may"; DCGO `maxCount: 1` with
//! no `canNoSelect`). The YAML's `optional: true` was corrected to
//! `optional: false`. When no [Appmon] is revealed the select finds no
//! candidates and auto-skips.

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledCostDelta, CompiledDeclarativeClause, CompiledScope,
    CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, DelayTrigger};
use digimon_engine::permanent::OptionState;
use digimon_engine::selection::OptionPlayResult;

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT25-098")
        .expect("BT25-098 YAML parses and compiles")
        .memory(10)
        .start()
}

/// Stack the given card IDs onto player 0's deck top. Last id in the slice ends
/// up on top of the deck (pushed last).
fn stack_deck_top(runner: &mut DebugRunner, ids: &[&str]) {
    for id in ids {
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == *id)
            .unwrap_or_else(|| panic!("card {id} registered"));
        let card_index = runner.game.next_card_index();
        runner.game.players[0]
            .deck
            .push(CardSource::new(data_idx, 0, card_index));
    }
}

fn appmon_digimon(id: &str, level: u8) -> digimon_engine::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Yellow];
    card.level = Some(level);
    card.dp = Some(i32::from(level) * 1000);
    card.play_cost = u16::from(level) + 2;
    card.traits = vec!["Appmon".to_string()];
    card
}

fn appmon_tamer(id: &str) -> digimon_engine::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Tamer;
    card.colors = vec![CardColor::Yellow];
    card.level = None;
    card.dp = None;
    card.play_cost = 3;
    card.traits = vec!["Appmon".to_string()];
    card
}

/// Recursively search a process for a `TrashFromReveal` step (the "trash the
/// rest" is authored inside a `per_selected` body).
fn process_contains_trash_from_reveal(steps: &[CompiledStep]) -> bool {
    steps.iter().any(|s| match s {
        CompiledStep::TrashFromReveal { .. } => true,
        CompiledStep::PerSelected { body, .. } | CompiledStep::ForEach { body, .. } => {
            process_contains_trash_from_reveal(body)
        }
        _ => false,
    })
}

fn plain_digimon(id: &str, level: u8) -> digimon_engine::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Yellow];
    card.level = Some(level);
    card.dp = Some(i32::from(level) * 1000);
    card.play_cost = u16::from(level) + 2;
    card
}

fn hand_ids(runner: &DebugRunner, player: u8) -> Vec<String> {
    runner.game.players[player as usize]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect()
}

fn trash_ids(runner: &DebugRunner, player: u8) -> Vec<String> {
    runner.game.players[player as usize]
        .trash
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect()
}

fn field_contains(runner: &DebugRunner, player: u8, card_id: &str) -> bool {
    runner.game.player(player).battle_area.iter().any(|perm| {
        perm.card_sources
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == card_id)
    })
}

// ── Section 1: Structural ──────────────────────────────────────────────────

#[test]
fn bt25_098_is_yellow_appmon_option_cost_3() {
    let runner = runner();
    let card = runner
        .compiled_card("BT25-098")
        .expect("BT25-098 compiled card present");

    assert_eq!(card.name, "Cyber Engage");
    assert_eq!(card.kind, CompiledCardKind::Option);
    assert_eq!(card.cost, Some(3));
    assert!(card.traits.iter().any(|t| t == "Appmon"));
    assert!(
        card.use_requirement.is_some(),
        "card has a <Use Req. ([Appmon] trait)>"
    );
}

#[test]
fn bt25_098_has_ignore_color_requirement_flood_gate() {
    let runner = runner();
    let card = runner.compiled_card("BT25-098").expect("compiled");
    assert!(
        card.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Declarative(CompiledDeclarativeClause::FloodGate { modifier, .. })
                if modifier == "IgnoreColorRequirement"
        )),
        "color-ignore flood gate present"
    );
}

#[test]
fn bt25_098_main_reveals_three_adds_appmon_trashes_rest_and_places_self() {
    let runner = runner();
    let card = runner.compiled_card("BT25-098").expect("compiled");
    let main = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::MainFromHand) => {
                Some(t)
            }
            _ => None,
        })
        .expect("[Main] reveal clause");

    assert!(main
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::RevealTopDeck { count: 3, .. })));
    assert!(main
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::SelectReveal { .. })));
    assert!(main
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::AddToHandFromReveal { .. })));
    // "Trash the rest" is authored as `per_selected { body: [trash_from_reveal] }`,
    // so the TrashFromReveal lives inside a PerSelected body, not at top level.
    assert!(
        process_contains_trash_from_reveal(&main.process),
        "the [Main] body trashes the non-added reveals (TrashFromReveal, possibly nested in per_selected)"
    );
    assert!(main
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::PlaceSelfAsDelayOption)));
}

#[test]
fn bt25_098_has_standard_delay_clause_playing_appmon_for_three_less() {
    let runner = runner();
    let card = runner.compiled_card("BT25-098").expect("compiled");
    let delay = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Declarative(CompiledDeclarativeClause::Delay {
                trigger,
                process,
                ..
            }) => Some((trigger, process)),
            _ => None,
        })
        .expect("standard <Delay> clause present");

    assert_eq!(
        *delay.0,
        CompiledTiming::Delayed,
        "the <Delay> is a standard delay (manual activation after the placing turn)"
    );
    // The body selects an [Appmon] hand card and plays it with cost reduced by 3.
    assert!(delay.1.iter().any(|s| matches!(
        s,
        CompiledStep::PlayFromHand {
            cost_delta: Some(CompiledCostDelta::Reduce(3)),
            ..
        }
    )));
}

#[test]
fn bt25_098_security_places_self_in_battle_area() {
    let runner = runner();
    let card = runner.compiled_card("BT25-098").expect("compiled");
    let security = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSecurity) => Some(t),
            _ => None,
        })
        .expect("[Security] clause");

    assert_eq!(security.scope, CompiledScope::Inherited);
    assert!(security
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::PlaceSelfAsDelayOption)));
}

// ── Section 2: Behavioral — [Main] reveal/add/trash/place ───────────────────

#[test]
fn bt25_098_main_adds_one_appmon_trashes_the_other_two_then_parks_as_delay() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT25-098")
        .expect("BT25-098 YAML loads")
        .add_card(appmon_digimon("APP-A", 4))
        .add_card(plain_digimon("PLAIN-B", 3))
        .add_card(plain_digimon("PLAIN-C", 3))
        .add_card(appmon_tamer("APP-TAMER"))
        .hand(0, &["BT25-098"])
        .memory(10)
        .start();
    // An Appmon Tamer on field satisfies <Use Req. ([Appmon] trait)>.
    runner.place_on_field(0, "APP-TAMER", Some(0));
    runner.game.enter_main_phase();

    // Top-3 (last pushed = top): PLAIN-C, PLAIN-B, APP-A.
    stack_deck_top(&mut runner, &["PLAIN-C", "PLAIN-B", "APP-A"]);
    let trash_before = runner.trash_size(0);

    // The [Main] reveal/select parks the Option pipeline at a reveal selection.
    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending,
        "the [Main] reveal selection parks the Option pipeline"
    );

    // Exactly one [Appmon] is among the revealed three — add it.
    let view = runner
        .pending_selection_view()
        .expect("reveal add-to-hand selection pending");
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only the [Appmon] card (APP-A) is eligible to add"
    );
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("add the lone Appmon to hand");
    runner
        .auto_resolve()
        .expect("resolve trash-the-rest + battle-area placement");

    // APP-A went to hand; the two non-Appmon were trashed.
    assert!(
        hand_ids(&runner, 0).contains(&"APP-A".to_string()),
        "the [Appmon] card is added to hand: {:?}",
        hand_ids(&runner, 0)
    );
    let trash = trash_ids(&runner, 0);
    assert!(
        trash.contains(&"PLAIN-B".to_string()) && trash.contains(&"PLAIN-C".to_string()),
        "the two non-added reveals are trashed: {trash:?}"
    );
    assert_eq!(
        runner.trash_size(0),
        trash_before + 2,
        "exactly the two non-added reveals leave to trash"
    );

    // "Then, place this card in the battle area" — parked as a standard
    // <Delay> Option (MainPhaseActivated trigger).
    let placed = runner
        .game
        .player(0)
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&runner.game.card_data) == "BT25-098")
        .expect("BT25-098 placed in the battle area after the [Main] body");
    assert!(matches!(
        placed.option_state,
        OptionState::Delayed {
            trigger: DelayTrigger::MainPhaseActivated,
            ..
        }
    ));
}

/// Negative for the reveal pick: when NO [Appmon] is among the top three, the
/// add-pick has no eligible candidate, so nothing is added and all three are
/// trashed; self still places.
#[test]
fn bt25_098_main_with_no_appmon_revealed_trashes_all_three_and_still_places() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT25-098")
        .expect("BT25-098 YAML loads")
        .add_card(plain_digimon("PLAIN-A", 3))
        .add_card(plain_digimon("PLAIN-B", 3))
        .add_card(plain_digimon("PLAIN-C", 3))
        .add_card(appmon_tamer("APP-TAMER"))
        .hand(0, &["BT25-098"])
        .memory(10)
        .start();
    runner.place_on_field(0, "APP-TAMER", Some(0));
    runner.game.enter_main_phase();
    stack_deck_top(&mut runner, &["PLAIN-C", "PLAIN-B", "PLAIN-A"]);
    let trash_before = runner.trash_size(0);

    let result = runner.game.play_option_from_hand(0, 0);
    // No eligible [Appmon] — the add-pick fizzles. The pipeline may still park
    // for the placement tail; drive whatever remains to completion.
    if matches!(result, OptionPlayResult::Pending) {
        let _ = runner.auto_resolve();
    }

    assert!(
        !hand_ids(&runner, 0).iter().any(|id| id.starts_with("PLAIN")),
        "no non-Appmon card is ever added to hand: {:?}",
        hand_ids(&runner, 0)
    );
    assert_eq!(
        runner.trash_size(0),
        trash_before + 3,
        "all three non-Appmon reveals are trashed"
    );
    assert!(
        field_contains(&runner, 0, "BT25-098"),
        "self is placed in the battle area even when nothing is added"
    );
}

// ── Section 3: Behavioral — <Delay> activation ──────────────────────────────

/// Seat BT25-098 as a standard <Delay> Option placed on the current turn, then
/// advance to the controller's next main phase so the placing-turn gate
/// (16-16-3) is satisfied and the <Delay> [Main] activation becomes legal.
fn seat_and_advance_to_activatable_delay(
    runner: &mut DebugRunner,
) -> digimon_engine::permanent::PermanentHandle {
    let handle = runner.place_on_field(0, "BT25-098", Some(0));
    let placing_turn = runner.game.turn_count;
    runner.game.player_mut(0).battle_area[handle.index as usize].option_state =
        OptionState::Delayed {
            owner: 0,
            trash_on_turn: u16::MAX,
            trigger: DelayTrigger::MainPhaseActivated,
            placed_on_turn: placing_turn,
        };
    // Advance past the placing turn to player 0's next main phase.
    runner.end_turn();
    runner.game.enter_main_phase();
    runner.end_turn();
    assert_eq!(runner.game.turn_player(), 0);
    runner.game.enter_main_phase();
    handle
}

/// The standard <Delay> can only be activated after the placing turn. Once
/// active, activating it trashes BT25-098 (the self-trash cost) and lets the
/// owner optionally play 1 [Appmon] from hand with the cost reduced by 3.
#[test]
fn bt25_098_delay_after_placing_turn_plays_appmon_for_three_less() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT25-098")
        .expect("BT25-098 YAML loads")
        .add_card(appmon_digimon("APP-PLAY", 4)) // play_cost 6 → 3 after reduce
        .add_card(plain_digimon("FILL", 3))
        .hand(0, &["APP-PLAY"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(10)
        .start();

    let handle = seat_and_advance_to_activatable_delay(&mut runner);
    runner.game.set_memory(10);
    let memory_before = runner.memory();

    // Activate the standard <Delay> [Main] on the delay option's field slot.
    assert!(
        runner
            .game
            .activate_field_main(0, handle.index as usize),
        "the standard <Delay> [Main] is activatable after the placing turn"
    );

    // The optional "you may play 1 [Appmon] ... reduced by 3" surfaces.
    let view = runner
        .pending_selection_view()
        .expect("delay body offers the optional Appmon play");
    assert!(
        runner.pending_is_optional(),
        "printed 'You may play' keeps PASS legal"
    );
    // Only APP-PLAY (the lone [Appmon] in hand) is a legal play target.
    assert!(view.valid_action_ids.iter().any(|&a| a != PASS));
    let play_action = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != PASS)
        .expect("a concrete play action");
    runner
        .execute_action(view.selecting_player, play_action)
        .expect("choose APP-PLAY to play for 3 less");
    runner.auto_resolve().expect("resolve the discounted play");

    // Self-trash is the activation cost (settles once the <Delay> resolves).
    assert!(
        trash_ids(&runner, 0).contains(&"BT25-098".to_string()),
        "the <Delay> cost trashes BT25-098: {:?}",
        trash_ids(&runner, 0)
    );
    assert!(
        !field_contains(&runner, 0, "BT25-098"),
        "BT25-098 leaves the battle area when its <Delay> activates"
    );
    assert!(
        field_contains(&runner, 0, "APP-PLAY"),
        "the chosen [Appmon] is played"
    );
    // play_cost 6 reduced by 3 = 3 memory spent.
    assert_eq!(
        runner.memory(),
        memory_before - 3,
        "APP-PLAY (cost 6) is played with the cost reduced by 3"
    );
}

/// The <Delay> play is optional ("You may"): the owner can decline it. The
/// self-trash cost is still paid when the delay activates.
#[test]
fn bt25_098_delay_play_can_be_declined() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT25-098")
        .expect("BT25-098 YAML loads")
        .add_card(appmon_digimon("APP-PLAY", 4))
        .add_card(plain_digimon("FILL", 3))
        .hand(0, &["APP-PLAY"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(10)
        .start();

    let handle = seat_and_advance_to_activatable_delay(&mut runner);
    runner.game.set_memory(10);
    let memory_before = runner.memory();
    assert!(runner.game.activate_field_main(0, handle.index as usize));

    let view = runner
        .pending_selection_view()
        .expect("delay body offers the optional Appmon play");
    assert!(runner.pending_is_optional());
    runner
        .execute_action(view.selecting_player, PASS)
        .expect("decline the optional Appmon play");
    runner.auto_resolve().expect("settle the declined delay");

    assert!(
        !field_contains(&runner, 0, "APP-PLAY"),
        "declining leaves APP-PLAY in hand"
    );
    assert!(
        hand_ids(&runner, 0).contains(&"APP-PLAY".to_string()),
        "declined target stays in hand"
    );
    assert_eq!(runner.memory(), memory_before, "declining spends no memory");
    assert!(
        trash_ids(&runner, 0).contains(&"BT25-098".to_string()),
        "the self-trash cost is paid even when the play is declined"
    );
}

// ── Section 4: Use Req. gating — IgnoreColorRequirement ─────────────────────

/// The flood gate that lets BT25-098 ignore color requirements is active only
/// while the controller has an [Appmon] Digimon or Tamer. (The Use Req. itself
/// is the same condition.) An Appmon Tamer satisfies it.
#[test]
fn bt25_098_use_requirement_is_present_and_targets_appmon() {
    let runner = runner();
    let card = runner.compiled_card("BT25-098").expect("compiled");
    // The flood gate's active_when references an Appmon Digimon or Tamer; we
    // assert the clause exists and is the color-ignore modifier (behavioral
    // color-bypass is covered by the engine's option-play color check, which is
    // exercised generically by sibling Use-Req tests, e.g. bt25_100).
    let has_gate = card.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Declarative(CompiledDeclarativeClause::FloodGate { modifier, active_when, .. })
            if modifier == "IgnoreColorRequirement" && active_when.is_some()
    ));
    assert!(
        has_gate,
        "IgnoreColorRequirement flood gate is conditioned on an Appmon Digimon/Tamer"
    );
}

// ── Section 5: Use Req. gating — negative and positive behavioral ────────────

/// Negative: without any Appmon Digimon or Tamer on the field, attempting to
/// play BT25-098 from hand must return OptionPlayResult::Invalid (use_requirement
/// not satisfied, color-bypass inactive, and the card is yellow requiring a
/// yellow source).
#[test]
fn bt25_098_play_blocked_without_appmon_on_field() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT25-098")
        .expect("BT25-098 YAML loads")
        .add_card(plain_digimon("PLAIN-A", 3))
        .add_card(plain_digimon("FILLER", 3))
        .hand(0, &["BT25-098"])
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 5])
        .memory(10)
        .start();
    runner.game.enter_main_phase();

    // No [Appmon] permanent on field → use_requirement fails and the non-yellow
    // color bypass is inactive → play must be rejected.
    let result = runner.game.play_option_from_hand(0, 0);
    assert_eq!(
        result,
        OptionPlayResult::Invalid,
        "playing BT25-098 without an Appmon on field must be Invalid (use_req not met, color lock active)"
    );
    // BT25-098 must still be in hand, not consumed.
    assert!(
        hand_ids(&runner, 0).contains(&"BT25-098".to_string()),
        "BT25-098 stays in hand after a failed play attempt"
    );
}

/// Positive: with an Appmon Digimon on field, the play is accepted.
#[test]
fn bt25_098_play_allowed_with_appmon_digimon_on_field() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT25-098")
        .expect("BT25-098 YAML loads")
        .add_card(appmon_digimon("APP-D", 4))
        .add_card(plain_digimon("FILLER", 3))
        .hand(0, &["BT25-098"])
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 5])
        .memory(10)
        .start();
    runner.place_on_field(0, "APP-D", Some(0));
    runner.game.enter_main_phase();
    // Stack 3 fillers so the reveal doesn't underflow.
    stack_deck_top(&mut runner, &["FILLER", "FILLER", "FILLER"]);

    let result = runner.game.play_option_from_hand(0, 0);
    assert!(
        result != OptionPlayResult::Invalid,
        "playing BT25-098 with an Appmon Digimon on field must NOT be Invalid; got {:?}",
        result
    );
}

// ── Section 6: <Delay> filter — non-Appmon in hand not selectable ────────────

/// When the <Delay> activates and the owner's hand contains only non-Appmon
/// cards, the filter must produce zero non-PASS actions (or go straight to
/// settlement without a selection prompt, depending on the engine's empty-
/// candidate behaviour for optional selects).
#[test]
fn bt25_098_delay_with_no_appmon_in_hand_offers_only_pass_or_skips() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT25-098")
        .expect("BT25-098 YAML loads")
        .add_card(plain_digimon("PLAIN-HAND", 3)) // non-Appmon
        .add_card(plain_digimon("FILL", 3))
        .hand(0, &["PLAIN-HAND"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(10)
        .start();

    let handle = seat_and_advance_to_activatable_delay(&mut runner);
    runner.game.set_memory(10);

    assert!(
        runner
            .game
            .activate_field_main(0, handle.index as usize),
        "the <Delay> must be activatable (placing-turn gate has passed)"
    );

    // Either: the engine skips the empty selection and auto-settles (no
    // pending_selection) OR the selection is optional with zero non-PASS
    // actions (only PASS available).
    let view_opt = runner.pending_selection_view();
    match view_opt {
        None => {
            // Engine auto-settled — the self-trash cost was paid.
            assert!(
                trash_ids(&runner, 0).contains(&"BT25-098".to_string()),
                "BT25-098 must be trashed as the <Delay> cost even when no play is made: {:?}",
                trash_ids(&runner, 0)
            );
        }
        Some(view) => {
            // Selection surfaced — must be optional and have no concrete play
            // action (only PASS is legal).
            assert!(
                runner.pending_is_optional(),
                "the selection must be optional when no [Appmon] is in hand"
            );
            assert!(
                view.valid_action_ids.iter().all(|&a| a == PASS),
                "only PASS should be legal when hand contains no [Appmon]: {:?}",
                view.valid_action_ids
            );
            // Pass the selection and confirm the cost was paid.
            runner
                .execute_action(view.selecting_player, PASS)
                .expect("pass the empty delay selection");
            runner.auto_resolve().expect("settle the delay after pass");
            assert!(
                trash_ids(&runner, 0).contains(&"BT25-098".to_string()),
                "BT25-098 must be trashed after <Delay> even when play is declined: {:?}",
                trash_ids(&runner, 0)
            );
        }
    }
    // In both paths PLAIN-HAND must NOT have been played.
    assert!(
        !field_contains(&runner, 0, "PLAIN-HAND"),
        "the non-Appmon PLAIN-HAND must never reach the field via the <Delay>"
    );
}

// ── Section 7: Security behavioral ───────────────────────────────────────────

/// When BT25-098 is revealed as a security card (P1's security), the
/// [Security] effect places it in P1's battle area as a <Delay> Option.
/// Drive by having P0 attack P1's player so the security check fires.
#[test]
fn bt25_098_security_places_self_in_battle_area_when_revealed() {
    let mut attacker_card = make_test_card("ATTACKER", "Attacker");
    attacker_card.card_kind = CardKind::Digimon;
    attacker_card.colors = vec![CardColor::Yellow];
    attacker_card.level = Some(5);
    attacker_card.dp = Some(9000);
    attacker_card.play_cost = 0;

    let mut runner = DebugRunner::builder()
        .dsl_card("BT25-098")
        .expect("BT25-098 YAML loads")
        .add_card(attacker_card)
        // Seed BT25-098 into P1's security so the security-check fires its
        // SecuritySkill ([Security] effect).
        .security(1, &["BT25-098"])
        .memory(10)
        .start();

    let atk = runner.place_on_field(0, "ATTACKER", Some(0));
    let p1_field_before = runner.battle_area_size(1);

    // P0 attacks P1's player — the security check fires BT25-098's
    // [Security] effect: "Place this card in the battle area."
    let _ = runner.attack_player(atk, 1, false);
    // auto_resolve handles the security effect chain.
    let _ = runner.auto_resolve();

    // BT25-098 should now be in P1's battle area as a Delayed Option.
    let placed = runner
        .game
        .player(1)
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&runner.game.card_data) == "BT25-098");
    assert!(
        placed.is_some(),
        "BT25-098 must be placed in P1's battle area after the [Security] effect fires; \
         field had {} permanents before, now has {}",
        p1_field_before,
        runner.battle_area_size(1)
    );
    if let Some(perm) = placed {
        assert!(
            matches!(perm.option_state, OptionState::Delayed { .. }),
            "BT25-098 placed via [Security] must be in a Delayed state (standard <Delay>)"
        );
    }
}

// ── Section 8: Event-log ──────────────────────────────────────────────────────

/// When the [Main] body runs, BT25-098 is placed in the battle area via
/// place_self_as_delay_option. Confirm via the event log that at least one
/// event fires during the pipeline (reveal + hand-add + trash steps all emit
/// events), and that BT25-098 ends up on the battle area (not in hand).
#[test]
fn bt25_098_main_pipeline_emits_events_and_places_self() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT25-098")
        .expect("BT25-098 YAML loads")
        .add_card(appmon_digimon("APP-A", 4))
        .add_card(plain_digimon("PLAIN-E", 3))
        .add_card(appmon_tamer("APP-TAMER"))
        .hand(0, &["BT25-098"])
        .deck(0, &["PLAIN-E", "PLAIN-E", "APP-A"])
        .deck(1, &["PLAIN-E"; 5])
        .memory(10)
        .start();
    runner.place_on_field(0, "APP-TAMER", Some(0));
    runner.game.enter_main_phase();

    // Capture the event log baseline before the Option fires.
    let cp = runner.event_checkpoint();

    // Play BT25-098 — use_req is satisfied by the APP-TAMER on field.
    // Stack the deck so APP-A is on top (last pushed = top).
    stack_deck_top(&mut runner, &["PLAIN-E", "PLAIN-E", "APP-A"]);
    let result = runner.game.play_option_from_hand(0, 0);
    assert!(
        result != OptionPlayResult::Invalid,
        "play must be accepted when an Appmon Tamer is on field; got {:?}",
        result
    );

    // Drive the reveal selection (pick APP-A to add) and settle.
    if let Some(view) = runner.pending_selection_view() {
        let action = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|&a| a != PASS)
            .unwrap_or(PASS);
        let _ = runner.execute_action(view.selecting_player, action);
    }
    let _ = runner.auto_resolve();

    // BT25-098 must be on the battle area (placed as a <Delay> Option).
    assert!(
        field_contains(&runner, 0, "BT25-098"),
        "BT25-098 must be placed in the battle area after [Main] resolves"
    );

    // BT25-098 must NOT be in hand any longer.
    assert!(
        !hand_ids(&runner, 0).contains(&"BT25-098".to_string()),
        "BT25-098 must leave the hand zone after play: {:?}",
        hand_ids(&runner, 0)
    );

    // At least one event must have been emitted during the pipeline (the
    // reveal / hand-add / trash-rest steps are observable via the event log).
    let events = runner.events_since(cp);
    assert!(
        !events.is_empty(),
        "playing BT25-098 must emit at least one engine event; got 0 events after checkpoint {cp}"
    );
}
