//! BT21-097 App Link — Option, Green, Cost 3, [Appmon].
//!
//! # Card text (from card image — authoritative)
//!
//! (NOTE: cards.json mislabels this as card_kind 2 / Tamer; the card image
//! shows "OPTION" with "Use 3". It is an OPTION card.)
//!
//! <Use Req. ([Appmon] trait)>
//!   While you have a Digimon or Tamer with the [Appmon] trait on the field,
//!   you can ignore this card's color requirements.
//! [Main] Reveal the top 3 cards of your deck. Add 1 card with the [Appmon]
//!   or [App Driver] trait among them to the hand. Trash the rest. Then,
//!   place this card in the battle area.
//! [End of Your Turn] ＜Delay＞ (By trashing this card after the placing turn,
//!   activate the effect below.)
//!   ・You may link 1 card from your hand with 1 of your Digimon without
//!     paying the cost.
//! Inherited: [Security] Place this card in the battle area.
//!
//! # DCGO C# reference (READ-ONLY)
//! C:/Users/james/Documents/digimon-deck-list-builder-1/DCGO/Assets/Scripts/
//! CardEffect/BT21/Green/BT21_097.cs
//!
//! # DCGO Delay body (ActivateCoroutine for OnEndTurn):
//!   CanUseCondition: IsOwnerTurn && CanDeclareOptionDelayEffect(card).
//!   Body:
//!     1. DeletePermanentAndProcessAccordingToResult(card.PermanentOfThisCard())
//!        → trash-self cost (declinable success-process gate).
//!     2. If successful: select 1 hand card (CanLink check), then select 1 own
//!        Digimon (CanLinkToTargetPermanent), then AddLinkCard(cardForLinking).
//!
//! # Patterns this test covers
//! - D3 color-ignore / Use Req. flood gate (IgnoreColorRequirement, Appmon
//!   Digimon or Tamer gated)
//! - Group E: reveal-3, MANDATORY add of 1 [Appmon] or [App Driver] to hand
//!   (printed text has no "may"), trash the rest
//! - Option pipeline: [Main] places self as a scheduled <Delay> battle-area
//!   Option via `place_self_as_delay_option` (trigger EndOfYourNextTurn —
//!   printed timing header is [End of Your Turn], NOT [Main], so no
//!   main-phase activation action may be exposed)
//! - Scheduled <Delay>: fires at the end of the owner's NEXT turn (first
//!   legal window per rule 16-16-3), self-trash + optional "you may link 1
//!   card from hand to own Digimon free". The mandatory-scan residual
//!   (decline cannot keep the Option parked) is the OPEN engine gap
//!   G-ENGINE-SCHEDULED-DELAY-MANDATORY-SCAN — pinned by an `#[ignore]`d
//!   faithful test below.
//! - Inherited [Security] places self in battle area (behavioral, via a real
//!   security check)

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep,
    CompiledTiming,
};
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{
    EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_MAIN, FIELD_EFFECT_START, PASS,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, DelayTrigger};
use digimon_engine::permanent::OptionState;
use digimon_engine::selection::OptionPlayResult;

const CARD_ID: &str = "BT21-097";

fn appmon_digimon(id: &str, level: u8) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Green];
    card.level = Some(level);
    card.dp = Some(i32::from(level) * 1000);
    card.play_cost = u16::from(level) + 2;
    card.traits = vec!["Appmon".to_string()];
    card
}

fn app_driver_digimon(id: &str, level: u8) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Green];
    card.level = Some(level);
    card.dp = Some(i32::from(level) * 1000);
    card.play_cost = u16::from(level) + 2;
    card.traits = vec!["App Driver".to_string()];
    card
}

fn appmon_tamer(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Tamer;
    card.colors = vec![CardColor::Green];
    card.level = None;
    card.dp = None;
    card.play_cost = 3;
    card.traits = vec!["Appmon".to_string()];
    card
}

fn plain_digimon(id: &str, level: u8) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Green];
    card.level = Some(level);
    card.dp = Some(i32::from(level) * 1000);
    card.play_cost = u16::from(level) + 2;
    card
}

/// Push cards onto player 0's deck top. Last element ends up on top.
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

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT21-097 YAML parses and compiles")
        .memory(10)
        .start()
}

// ── Section 1: Structural ──────────────────────────────────────────────────

#[test]
fn bt21_097_is_green_appmon_option_cost_3() {
    let runner = runner();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("BT21-097 compiled card present");

    assert_eq!(card.name, "App Link");
    assert_eq!(card.kind, CompiledCardKind::Option);
    assert_eq!(card.cost, Some(3));
    assert!(
        card.traits.iter().any(|t| t == "Appmon"),
        "trait Appmon present"
    );
    assert!(
        card.use_requirement.is_some(),
        "card has a <Use Req. ([Appmon] trait)>"
    );
}

#[test]
fn bt21_097_has_ignore_color_requirement_flood_gate() {
    let runner = runner();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
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
fn bt21_097_main_reveals_three_adds_appmon_or_appdriver_trashes_rest_places_self() {
    let runner = runner();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
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
    // "Trash the rest" — TrashFromReveal inside PerSelected body.
    fn process_contains_trash_from_reveal(steps: &[CompiledStep]) -> bool {
        steps.iter().any(|s| match s {
            CompiledStep::TrashFromReveal { .. } => true,
            CompiledStep::PerSelected { body, .. } | CompiledStep::ForEach { body, .. } => {
                process_contains_trash_from_reveal(body)
            }
            _ => false,
        })
    }
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
fn bt21_097_has_scheduled_end_of_turn_delay_clause_with_link_cards_step() {
    let runner = runner();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
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

    // Printed timing header is [End of Your Turn] (DCGO EffectTiming.OnEndTurn
    // gated on IsOwnerTurn) — NOT [Main]. `CompiledTiming::Delayed`
    // (MainPhaseActivated) would expose a main-phase activation, letting the
    // link land before attacks; the faithful engine trigger is the scheduled
    // owner-turn-end scan, whose first legal window is the end of the owner's
    // next turn (rule 16-16-3).
    assert_eq!(
        *delay.0,
        CompiledTiming::EndOfYourNextTurn,
        "the <Delay> is a scheduled [End of Your Turn] delay (EndOfYourNextTurn trigger), not a [Main]-activated one"
    );
    // The body links a hand card to an own Digimon for free.
    assert!(
        delay
            .1
            .iter()
            .any(|s| matches!(s, CompiledStep::LinkCards { .. })),
        "Delay body must contain a LinkCards step"
    );
}

#[test]
fn bt21_097_security_places_self_in_battle_area() {
    let runner = runner();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
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

// ── Section 2: Behavioral — [Main] reveal/add/trash/place ─────────────────

#[test]
fn bt21_097_main_adds_appmon_trashes_rest_then_parks_as_delay() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT21-097 YAML loads")
        .add_card(appmon_digimon("APP-A", 4))
        .add_card(plain_digimon("PLAIN-B", 3))
        .add_card(plain_digimon("PLAIN-C", 3))
        .add_card(appmon_tamer("APP-TAMER"))
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();

    // An Appmon Tamer satisfies <Use Req. ([Appmon] trait)>.
    runner.place_on_field(0, "APP-TAMER", Some(0));
    runner.game.enter_main_phase();

    // Top-3 (last pushed = top): PLAIN-C, PLAIN-B, APP-A.
    stack_deck_top(&mut runner, &["PLAIN-C", "PLAIN-B", "APP-A"]);
    let trash_before = runner.trash_size(0);
    let placing_turn = runner.game.turn_count;

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
    assert!(
        !runner.pending_is_optional(),
        "printed 'Add 1 card ... to the hand' has no 'may' — the add is mandatory (no PASS)"
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

    // "Then, place this card in the battle area" — parked as a scheduled
    // [End of Your Turn] <Delay> Option. Placed on the owner's own turn, the
    // first legal activation window (16-16-3: not the placing turn) is the
    // end of the owner's NEXT turn = placing_turn + 2 in 2-player rotation.
    let placed = runner
        .game
        .player(0)
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID)
        .expect("BT21-097 placed in the battle area after the [Main] body");
    match placed.option_state {
        OptionState::Delayed {
            trigger,
            trash_on_turn,
            ..
        } => {
            assert_eq!(
                trigger,
                DelayTrigger::EndOfYourNextTurn,
                "printed [End of Your Turn] <Delay> parks with the scheduled owner-turn-end trigger"
            );
            assert_eq!(
                trash_on_turn,
                placing_turn + 2,
                "the <Delay> is scheduled for the end of the owner's next turn (16-16-3)"
            );
        }
        other => panic!("BT21-097 must park as OptionState::Delayed; got {other:?}"),
    }
}

#[test]
fn bt21_097_main_adds_app_driver_trashes_rest_then_parks_as_delay() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT21-097 YAML loads")
        .add_card(app_driver_digimon("DRIVER-A", 4))
        .add_card(plain_digimon("PLAIN-B", 3))
        .add_card(plain_digimon("PLAIN-C", 3))
        .add_card(appmon_tamer("APP-TAMER"))
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();

    runner.place_on_field(0, "APP-TAMER", Some(0));
    runner.game.enter_main_phase();

    // Top-3: PLAIN-C, PLAIN-B, DRIVER-A.
    stack_deck_top(&mut runner, &["PLAIN-C", "PLAIN-B", "DRIVER-A"]);
    let trash_before = runner.trash_size(0);

    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending,
        "the [Main] reveal selection parks the Option pipeline"
    );

    let view = runner
        .pending_selection_view()
        .expect("reveal add-to-hand selection pending");
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only the [App Driver] card (DRIVER-A) is eligible to add"
    );
    assert!(
        !runner.pending_is_optional(),
        "printed 'Add 1 card ... to the hand' has no 'may' — the add is mandatory (no PASS)"
    );
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("add the App Driver to hand");
    runner
        .auto_resolve()
        .expect("resolve trash-the-rest + placement");

    assert!(
        hand_ids(&runner, 0).contains(&"DRIVER-A".to_string()),
        "the [App Driver] card is added to hand"
    );
    assert_eq!(
        runner.trash_size(0),
        trash_before + 2,
        "exactly the two non-added reveals are trashed"
    );
    assert!(
        field_contains(&runner, 0, CARD_ID),
        "BT21-097 placed on field after [Main]"
    );
}

/// Negative: when NO [Appmon] or [App Driver] is among the top 3, the
/// add-pick has no eligible candidates, nothing is added and all 3 are
/// trashed; self still places.
#[test]
fn bt21_097_main_with_no_eligible_trashes_all_three_and_places_self() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT21-097 YAML loads")
        .add_card(plain_digimon("PLAIN-A", 3))
        .add_card(plain_digimon("PLAIN-B", 3))
        .add_card(plain_digimon("PLAIN-C", 3))
        .add_card(appmon_tamer("APP-TAMER"))
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();

    runner.place_on_field(0, "APP-TAMER", Some(0));
    runner.game.enter_main_phase();
    stack_deck_top(&mut runner, &["PLAIN-C", "PLAIN-B", "PLAIN-A"]);
    let trash_before = runner.trash_size(0);

    let result = runner.game.play_option_from_hand(0, 0);
    if matches!(result, OptionPlayResult::Pending) {
        let _ = runner.auto_resolve();
    }

    assert!(
        !hand_ids(&runner, 0)
            .iter()
            .any(|id| id.starts_with("PLAIN")),
        "no non-Appmon/App-Driver card is ever added to hand: {:?}",
        hand_ids(&runner, 0)
    );
    assert_eq!(
        runner.trash_size(0),
        trash_before + 3,
        "all three non-eligible reveals are trashed"
    );
    assert!(
        field_contains(&runner, 0, CARD_ID),
        "self is placed in the battle area even when nothing is added"
    );
}

// ── Section 3: Use Req. — color-ignore flood gate ──────────────────────────

#[test]
fn bt21_097_use_requirement_targets_appmon_digimon_or_tamer() {
    let runner = runner();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    let has_gate = card.effects.iter().any(|clause| {
        matches!(
            clause,
            CompiledClause::Declarative(CompiledDeclarativeClause::FloodGate {
                modifier,
                active_when,
                ..
            }) if modifier == "IgnoreColorRequirement" && active_when.is_some()
        )
    });
    assert!(
        has_gate,
        "IgnoreColorRequirement flood gate is conditioned on an Appmon Digimon/Tamer"
    );
}

// ── Section 4: Behavioral — <Delay> fires at the end of the owner's next turn ─

/// Seat BT21-097 on P0's field as a scheduled [End of Your Turn] <Delay>
/// Option placed THIS turn. In 2-player rotation the first legal activation
/// window (16-16-3: not the placing turn) is the end of P0's next turn =
/// `turn_count + 2` (skip P1's turn) — the same schedule
/// `place_self_as_delay_option` computes for a [Main] placement.
fn seat_as_scheduled_end_delay(runner: &mut DebugRunner) {
    let handle = runner.place_on_field(0, CARD_ID, Some(0));
    let placing_turn = runner.game.turn_count;
    runner.game.player_mut(0).battle_area[handle.index as usize].option_state =
        OptionState::Delayed {
            owner: 0,
            trash_on_turn: placing_turn + 2,
            trigger: DelayTrigger::EndOfYourNextTurn,
            placed_on_turn: placing_turn,
        };
}

/// The full printed timing arc: the <Delay> does NOT fire at the end of the
/// placing turn (16-16-3), exposes NO main-phase activation action (the
/// printed timing header is [End of Your Turn], not [Main]), then fires at
/// the end of the owner's next turn and links 1 hand card to 1 own Digimon
/// for free.
#[test]
fn bt21_097_delay_fires_at_end_of_owners_next_turn_and_links_hand_card_free() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT21-097 YAML loads")
        .add_card(appmon_digimon("APP-FIELD", 5))
        .add_card(appmon_digimon("APP-LINK", 4))
        .add_card(plain_digimon("FILL", 3))
        .hand(0, &["APP-LINK"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(10)
        .start();

    let field_digimon = runner.place_on_field(0, "APP-FIELD", Some(0));
    seat_as_scheduled_end_delay(&mut runner);

    // 16-16-3 gate: ending the PLACING turn must not fire the <Delay>.
    runner.end_turn();
    assert!(
        runner.game.pending_selection.is_none(),
        "the <Delay> must not fire at the end of the placing turn (16-16-3)"
    );
    assert!(
        field_contains(&runner, 0, CARD_ID),
        "BT21-097 stays parked through the placing turn's end"
    );

    // P1's turn passes; the delay only cares about the OWNER's turn end.
    runner.end_turn();
    assert_eq!(runner.game.turn_player(), 0);
    assert!(
        field_contains(&runner, 0, CARD_ID),
        "BT21-097 stays parked through the opponent's turn end"
    );

    // On the owner's next turn, NO main-phase activation is exposed — the
    // printed timing is [End of Your Turn], not a [Main] <Delay> action.
    runner.game.enter_main_phase();
    let delay_idx = runner
        .game
        .player(0)
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID)
        .expect("BT21-097 still parked on the owner's next turn");
    let main_bit = (FIELD_EFFECT_START
        + delay_idx as u16 * EFFECTS_PER_PERMANENT
        + FIELD_EFFECT_SLOT_FOR_MAIN) as usize;
    assert_eq!(
        build_action_mask(&runner.game, 0)[main_bit],
        0.0,
        "an [End of Your Turn] <Delay> must not offer a main-phase activation action"
    );

    // End of the owner's next turn — the <Delay> fires: optional link pick.
    // NOTE: P0 drew 1 FILL at the start of this turn, so the hand is
    // [APP-LINK, FILL].
    runner.end_turn();
    let view = runner
        .pending_selection_view()
        .expect("the <Delay> body offers the optional link at the owner's turn end");
    assert!(
        runner.pending_is_optional(),
        "printed 'You may link' keeps PASS legal"
    );
    // APP-LINK is at hand[0]; pick the first non-PASS action.
    let link_card_action = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != PASS)
        .expect("a concrete hand action (APP-LINK at hand[0])");
    runner
        .execute_action(view.selecting_player, link_card_action)
        .expect("choose APP-LINK to link");

    // After picking the hand card, select the own Digimon to link to.
    let view2 = runner
        .pending_selection_view()
        .expect("after hand pick, own Digimon pick must install");
    let digimon_action = view2
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != PASS)
        .expect("a concrete own-Digimon action");
    runner
        .execute_action(view2.selecting_player, digimon_action)
        .expect("choose APP-FIELD as link target");
    runner.auto_resolve().expect("settle the link");

    // BT21-097 trashed (self-trash cost of the <Delay> activation).
    assert!(
        trash_ids(&runner, 0).contains(&CARD_ID.to_string()),
        "the <Delay> cost trashes BT21-097: {:?}",
        trash_ids(&runner, 0)
    );
    assert!(
        !field_contains(&runner, 0, CARD_ID),
        "BT21-097 leaves the battle area when its <Delay> activates"
    );

    // APP-LINK was linked to APP-FIELD — confirm it's in the field Digimon's
    // link sources. APP-FIELD was placed before the option, so its index is
    // stable across the option's trash.
    let host = runner
        .game
        .player(0)
        .battle_area
        .get(field_digimon.index as usize)
        .expect("APP-FIELD still on field");
    let link_ids: Vec<String> = host
        .linked_cards
        .iter()
        .map(|c: &CardSource| c.card_id(&runner.game.card_data).to_string())
        .collect();
    assert!(
        link_ids.contains(&"APP-LINK".to_string()),
        "APP-LINK is linked to APP-FIELD: {link_ids:?}"
    );
    // APP-LINK must have been removed from hand (the drawn FILL may remain).
    assert!(
        !hand_ids(&runner, 0).contains(&"APP-LINK".to_string()),
        "APP-LINK was consumed from hand: {:?}",
        hand_ids(&runner, 0)
    );
}

/// The link itself is optional ("You may link"): the owner can PASS.
///
/// Under the current scheduled-scan machinery the Option is trashed even
/// when the link is declined — the mandatory-scan residual tracked as
/// G-ENGINE-SCHEDULED-DELAY-MANDATORY-SCAN (docs/RUST_ENGINE_GAPS.md).
/// The faithful decline-keeps-it-parked behavior is pinned by the
/// `#[ignore]`d test below; this test locks the link optionality plus the
/// current engine trash semantics so a silent behavior change is visible.
#[test]
fn bt21_097_delay_link_can_be_declined() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT21-097 YAML loads")
        .add_card(appmon_digimon("APP-FIELD", 5))
        .add_card(appmon_digimon("APP-LINK", 4))
        .add_card(plain_digimon("FILL", 3))
        .hand(0, &["APP-LINK"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(10)
        .start();

    runner.place_on_field(0, "APP-FIELD", Some(0));
    seat_as_scheduled_end_delay(&mut runner);

    // Advance to the end of the owner's next turn (placing turn → P1's turn
    // → owner's next turn end, where the scheduled <Delay> fires).
    runner.end_turn();
    runner.end_turn();
    assert_eq!(runner.game.turn_player(), 0);
    runner.end_turn();

    // The scheduled window now offers the §16-16-2 cost FIRST. Accept it here —
    // this test is about declining the body's own inner "may", not the cost.
    let outer = runner
        .pending_selection_view()
        .expect("the scheduled window offers the <Delay> cost");
    let accept = outer
        .valid_action_ids
        .iter()
        .copied()
        .find(|a| *a != PASS)
        .expect("the accept branch must be offered");
    runner
        .execute_action(outer.selecting_player, accept)
        .expect("accept the <Delay> cost");

    let view = runner
        .pending_selection_view()
        .expect("the <Delay> body offers the optional link");
    assert!(runner.pending_is_optional());
    runner
        .execute_action(view.selecting_player, PASS)
        .expect("decline the optional link");
    runner.auto_resolve().expect("settle the declined delay");

    // APP-LINK stays in hand when declined.
    assert!(
        hand_ids(&runner, 0).contains(&"APP-LINK".to_string()),
        "declined target stays in hand"
    );
    // The Option IS trashed — and this is now correct for a stated reason
    // rather than an engine limitation. The player ACCEPTED the §16-16-2 cost
    // above; §15-7-2 only bars the processing after a condition when the
    // condition itself wasn't executed. Declining the body's separate inner
    // "may" does not refund a cost that was already paid.
    //
    // This assertion used to read "under the mandatory scheduled scan the
    // Option is trashed even on a declined link", citing
    // G-ENGINE-SCHEDULED-DELAY-MANDATORY-SCAN. That gap is CLOSED: declining
    // the cost is covered by
    // `bt21_097_delay_cost_may_be_declined_leaving_the_option_on_the_field`.
    assert!(
        trash_ids(&runner, 0).contains(&CARD_ID.to_string()),
        "accepting the cost trashes the Option even when the inner link is declined"
    );
}

/// FAITHFUL (§16-16-2, DCGO OnEndTurn isOptional: true): declining the
/// <Delay> at the owner's turn end must keep the Option parked in the battle
/// area, re-offerable at a later owner turn end. Blocked on the OPEN engine
/// gap G-ENGINE-SCHEDULED-DELAY-MANDATORY-SCAN (the scheduled scan fires
/// mandatorily and trashes the Option); un-ignore when the scan routes
/// through the outer accept/decline prompt.
#[test]
#[ignore = "G-ENGINE-SCHEDULED-DELAY-MANDATORY-SCAN — scheduled <Delay> scan is mandatory; decline cannot keep the Option parked yet"]
fn bt21_097_declining_delay_keeps_option_parked_for_a_later_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT21-097 YAML loads")
        .add_card(appmon_digimon("APP-FIELD", 5))
        .add_card(appmon_digimon("APP-LINK", 4))
        .add_card(plain_digimon("FILL", 3))
        .hand(0, &["APP-LINK"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(10)
        .start();

    runner.place_on_field(0, "APP-FIELD", Some(0));
    seat_as_scheduled_end_delay(&mut runner);

    runner.end_turn();
    runner.end_turn();
    assert_eq!(runner.game.turn_player(), 0);
    runner.end_turn();

    // Decline everything the boundary offers (the accept/decline prompt once
    // the gap fix lands; today only the inner link pick surfaces).
    while runner.game.pending_selection.is_some() {
        let view = runner
            .pending_selection_view()
            .expect("pending selection view");
        runner
            .execute_action(view.selecting_player, PASS)
            .expect("decline the <Delay>");
    }
    runner.auto_resolve().expect("finish the turn boundary");

    // Rules 16-16-2: the processing from <Delay> is optional — declining
    // keeps the Option in the battle area for a later owner turn end.
    assert!(
        field_contains(&runner, 0, CARD_ID),
        "a declined [End of Your Turn] <Delay> stays parked in the battle area"
    );
    assert!(
        !trash_ids(&runner, 0).contains(&CARD_ID.to_string()),
        "a declined <Delay> is not trashed"
    );
}

// ── Section 5: Behavioral — [Security] places self in battle area ─────────

/// A real security check on BT21-097 places it into the owner's battle area
/// as a parked <Delay> Option (DCGO PlaceSelfDelayOptionSecurityEffect)
/// instead of trashing it. Placed during the OPPONENT's turn, its scheduled
/// window is the end of the owner's next turn (turn_count + 1).
#[test]
fn bt21_097_security_check_places_self_in_battle_area_as_delay_option() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT21-097 YAML loads")
        .add_card(plain_digimon("ATTACKER", 4))
        .add_card(plain_digimon("FILL", 3))
        .security(1, &[CARD_ID])
        .deck(0, &["FILL"; 4])
        .deck(1, &["FILL"; 4])
        .memory(10)
        .start();

    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));
    let check_turn = runner.game.turn_count;

    let result = runner.attack_player(attacker, 1, false);

    assert_eq!(result, AttackResult::SecurityCheckSurvived);
    assert_eq!(
        runner.security_count(1),
        0,
        "BT21-097 leaves the security stack"
    );
    assert!(
        !trash_ids(&runner, 1).contains(&CARD_ID.to_string()),
        "BT21-097 is placed in the battle area instead of trashed"
    );
    let placed = runner
        .game
        .player(1)
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID)
        .expect("BT21-097 placed as a battle-area <Delay> Option");
    match placed.option_state {
        OptionState::Delayed {
            trigger,
            trash_on_turn,
            ..
        } => {
            assert_eq!(
                trigger,
                DelayTrigger::EndOfYourNextTurn,
                "the security placement parks the printed [End of Your Turn] <Delay>"
            );
            // Placed during the opponent's (P0's) turn, the owner's next turn
            // end is turn_count + 1 — a legal window (not the placing turn).
            assert_eq!(
                trash_on_turn,
                check_turn + 1,
                "security placement schedules the end of the owner's next turn"
            );
        }
        other => panic!("BT21-097 must park as OptionState::Delayed; got {other:?}"),
    }
}

/// §16-16-2: "The processing from <Delay> is optional." The scheduled window
/// must OFFER the trash-this-card cost, and declining it must leave the Option
/// on the field with its window moved forward — §16-16-1 keeps a Delay
/// available "while a card with this effect is in the battle area", and
/// BT21-097's printed [End of Your Turn] window comes round every own turn.
///
/// Closes G-ENGINE-SCHEDULED-DELAY-MANDATORY-SCAN: the scheduled scan used to
/// auto-pay the cost and auto-run the body, with no decline anywhere in the
/// action space (rule 17).
#[test]
fn bt21_097_delay_cost_may_be_declined_leaving_the_option_on_the_field() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT21-097 YAML loads")
        .add_card(appmon_digimon("APP-FIELD", 5))
        .add_card(appmon_digimon("APP-LINK", 4))
        .add_card(plain_digimon("FILL", 3))
        .hand(0, &["APP-LINK"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(10)
        .start();

    runner.place_on_field(0, "APP-FIELD", Some(0));
    seat_as_scheduled_end_delay(&mut runner);

    // Advance to the end of the owner's next turn (placing turn → P1's turn
    // → owner's next turn end, where the scheduled <Delay> fires).
    runner.end_turn();
    runner.end_turn();
    assert_eq!(runner.game.turn_player(), 0);
    runner.end_turn();

    let outer = runner
        .pending_selection_view()
        .expect("the scheduled window must offer the <Delay> cost (rule 17)");
    assert!(
        outer.is_optional,
        "the <Delay> cost confirm must expose PASS (§16-16-2)"
    );
    runner
        .execute_action(outer.selecting_player, PASS)
        .expect("declining the <Delay> cost must be reachable");
    runner.auto_resolve().expect("settle the declined delay");

    // (a) the cost is unpaid — the Option is NOT in the trash.
    assert!(
        !trash_ids(&runner, 0).contains(&CARD_ID.to_string()),
        "declining must not trash the Option (the trash IS the unpaid cost)"
    );
    // (b) §15-7-2 — the linked effect did not resolve, so the target stays put.
    assert!(
        hand_ids(&runner, 0).contains(&"APP-LINK".to_string()),
        "§15-7-2: with the cost declined, the processing after it can't execute"
    );
    // (c) the Option is still a live Delay, not stranded inert on the field.
    assert!(
        runner.game.player(0).battle_area.iter().any(|permanent| {
            permanent.top_card().card_id(&runner.game.card_data) == CARD_ID
                && matches!(
                    permanent.option_state,
                    digimon_engine::permanent::OptionState::Delayed { .. }
                )
        }),
        "the declined Option must remain a Delayed Option for its next window"
    );
}
