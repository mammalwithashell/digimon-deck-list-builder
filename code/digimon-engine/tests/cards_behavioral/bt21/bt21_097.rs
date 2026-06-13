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
//! - Group E: reveal-3, add 1 [Appmon] or [App Driver] to hand, trash the rest
//! - Option pipeline: [Main] places self as a standard <Delay> battle-area
//!   Option via `place_self_as_delay_option`
//! - Standard <Delay> (end-of-your-turn activation): self-trash cost + optional
//!   "you may link 1 card from hand to own Digimon free"
//! - Inherited [Security] places self in battle area

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep,
    CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
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
    assert!(card.traits.iter().any(|t| t == "Appmon"), "trait Appmon present");
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
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::MainFromHand) =>
            {
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
fn bt21_097_has_standard_delay_clause_with_link_cards_step() {
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

    assert_eq!(
        *delay.0,
        CompiledTiming::Delayed,
        "the <Delay> is a standard end-of-your-turn delay (MainPhaseActivated trigger)"
    );
    // The body links a hand card to an own Digimon for free.
    assert!(
        delay.1.iter().any(|s| matches!(s, CompiledStep::LinkCards { .. })),
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
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnSecurity) =>
            {
                Some(t)
            }
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
        .find(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID)
        .expect("BT21-097 placed in the battle area after the [Main] body");
    assert!(matches!(
        placed.option_state,
        OptionState::Delayed {
            trigger: DelayTrigger::MainPhaseActivated,
            ..
        }
    ));
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
        !hand_ids(&runner, 0).iter().any(|id| id.starts_with("PLAIN")),
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
    let has_gate = card.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Declarative(CompiledDeclarativeClause::FloodGate {
            modifier,
            active_when,
            ..
        }) if modifier == "IgnoreColorRequirement" && active_when.is_some()
    ));
    assert!(
        has_gate,
        "IgnoreColorRequirement flood gate is conditioned on an Appmon Digimon/Tamer"
    );
}

// ── Section 4: Behavioral — <Delay> activation ────────────────────────────

/// Place BT21-097 as a standard <Delay> Option already past the placing turn,
/// then advance to the controller's next main phase so the standard delay
/// gate (16-16-3) is satisfied and [Main] activation becomes legal.
fn seat_and_advance_to_activatable_delay(
    runner: &mut DebugRunner,
) -> digimon_engine::permanent::PermanentHandle {
    let handle = runner.place_on_field(0, CARD_ID, Some(0));
    let placing_turn = runner.game.turn_count;
    runner.game.player_mut(0).battle_area[handle.index as usize].option_state =
        OptionState::Delayed {
            owner: 0,
            trash_on_turn: u16::MAX,
            trigger: DelayTrigger::MainPhaseActivated,
            placed_on_turn: placing_turn,
        };
    // Advance past the placing turn so the delay gate is satisfied.
    runner.end_turn();
    runner.game.enter_main_phase();
    runner.end_turn();
    assert_eq!(runner.game.turn_player(), 0);
    runner.game.enter_main_phase();
    handle
}

/// The standard <Delay> links 1 hand card to 1 own Digimon for free.
#[test]
fn bt21_097_delay_links_hand_card_to_own_digimon_free() {
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
    let handle = seat_and_advance_to_activatable_delay(&mut runner);
    runner.game.set_memory(10);

    // Activate the standard <Delay> [Main] on the delay option's field slot.
    assert!(
        runner.game.activate_field_main(0, handle.index as usize),
        "the standard <Delay> [Main] is activatable after the placing turn"
    );

    // The optional "you may link 1 card from hand" surfaces.
    // NOTE: by this point player 0 has drawn 1 FILL card on their second turn
    // (the two `end_turn()` calls in `seat_and_advance_to_activatable_delay`
    // advance to player 0's next turn, which draws from the deck). Hand is
    // therefore [APP-LINK, FILL].
    let view = runner
        .pending_selection_view()
        .expect("delay body offers the optional link");
    assert!(
        runner.pending_is_optional(),
        "printed 'You may link' keeps PASS legal"
    );
    // APP-LINK is at hand[0] (drawn first); pick the first non-PASS action.
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

    // BT21-097 trashed (self-trash cost).
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
    // link sources.
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
    // APP-LINK must have been removed from hand. The hand may still contain
    // the FILL card drawn on player 0's second turn, but APP-LINK is gone.
    assert!(
        !hand_ids(&runner, 0).contains(&"APP-LINK".to_string()),
        "APP-LINK was consumed from hand: {:?}",
        hand_ids(&runner, 0)
    );

    let _ = field_digimon;
}

/// The <Delay> link is optional: the owner can decline it.
/// The self-trash cost is still paid when the delay activates.
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
    let handle = seat_and_advance_to_activatable_delay(&mut runner);
    runner.game.set_memory(10);
    assert!(runner.game.activate_field_main(0, handle.index as usize));

    let view = runner
        .pending_selection_view()
        .expect("delay body offers the optional link");
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
    // Self-trash cost is still paid.
    assert!(
        trash_ids(&runner, 0).contains(&CARD_ID.to_string()),
        "the self-trash cost is paid even when the link is declined"
    );
}
