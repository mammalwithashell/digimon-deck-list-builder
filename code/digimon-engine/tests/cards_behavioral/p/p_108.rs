//! P-108 Wisdom Training — Option, Purple, Cost 2. Traits: (none).
//!
//! # Card text (card image / official DB — authoritative)
//!
//! [Main] Reveal the top 2 cards of your deck. Add 1 purple card among them
//! to the hand. Return the rest to the bottom of the deck in any order. Then,
//! place this card in the battle area.
//! [Main] ＜Delay＞ (After this card is placed, by trashing it the next turn or
//! later, activate the effect below.)
//! ・1 of your Digimon may digivolve into a purple Digimon card in your hand
//! for its digivolution cost. When it would digivolve by this effect, reduce
//! the cost by 2.
//!
//! Inherited (Security):
//! Security Effect [Security] Place this card in the battle area.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/P/Purple/P_108.cs
//!   - OptionSkill: `SimplifiedRevealDeckTopCardsAndSelect(revealCount 2,
//!     bucket HasCardColor(Purple) AddHand max 1, remaining DeckBottom)` then
//!     `PlaceDelayOptionCards(card)`.
//!   - OnDeclaration (`CanDeclareOptionDelayEffect`): trash self →
//!     SelectPermanentEffect over own Digimon that have a purple hand card
//!     able to digivolve onto them (canNoSelect TRUE — the "may") →
//!     `DigivolveIntoHandOrTrashCard(payCost true, reduceCost 2, isHand true)`.
//!   - SecuritySkill: `PlaceSelfDelayOptionSecurityEffect`.
//!
//! # Patterns this test covers
//! - A2 two-pass reveal (`reveal_top_deck` + `select_reveal_buckets` + bottom
//!   the rest) with the Delay-option auto-seat
//! - Standard `<Delay>` (`kind: delay`, `trigger: delayed`) driving an
//!   effect-initiated digivolve from hand with `cost: { reduce: 2 }`
//! - Inherited [Security] `place_self_as_delay_option`

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep,
    CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, DelayTrigger};
use digimon_engine::permanent::{OptionState, PermanentHandle};
use digimon_engine::selection::{OptionPlayResult, SelectionKind};

const CARD_ID: &str = "P-108";

fn digimon(id: &str, color: CardColor, level: u8) -> CardData {
    let mut c = make_test_card(id, id);
    c.colors = vec![color];
    c.level = Some(level);
    c
}

/// A purple Lv.4 evo target: digivolves from a Lv.3 purple for 4 memory.
fn purple_evo(id: &str) -> CardData {
    let mut c = digimon(id, CardColor::Purple, 4);
    c.dp = Some(5000);
    c.play_cost = 5;
    c.evo_costs = vec![EvoCost {
        card_color: CardColor::Purple as u8,
        level: 3,
        memory_cost: 4,
    }];
    c
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("P-108 must load from the embedded DSL pack")
        .add_card(digimon("PURPLE-A", CardColor::Purple, 3))
        .add_card(digimon("PURPLE-B", CardColor::Purple, 3))
        .add_card(digimon("RED-A", CardColor::Red, 3))
        .add_card(digimon("RED-B", CardColor::Red, 3))
        .add_card(purple_evo("PURPLE-EVO"))
        .add_card({
            let mut c = digimon("RED-EVO", CardColor::Red, 4);
            c.evo_costs = vec![EvoCost {
                card_color: CardColor::Purple as u8,
                level: 3,
                memory_cost: 4,
            }];
            c
        })
        .deck(1, &["RED-A"; 6])
}

fn find_delayed(runner: &DebugRunner, player: usize) -> Option<usize> {
    runner.game.players[player]
        .battle_area
        .iter()
        .position(|p| {
            p.top_card().card_id(&runner.game.card_data) == CARD_ID
                && matches!(p.option_state, OptionState::Delayed { .. })
        })
}

fn hand_ids(runner: &DebugRunner, player: usize) -> Vec<String> {
    runner.game.players[player]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect()
}

/// The bottom `n` cards of the deck, bottom-most first. `Player::draw` pops
/// the LAST element (top), so the bottom of the deck is the front of the Vec
/// — and `.deck(...)` lists are bottom → top.
fn deck_bottom_ids(runner: &DebugRunner, player: usize, n: usize) -> Vec<String> {
    let deck = &runner.game.players[player].deck;
    deck[..n]
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect()
}

/// Recursive search for a step kind inside a compiled process (steps nested
/// under `if:` / `else:` branches included).
fn process_contains(steps: &[CompiledStep], pred: &dyn Fn(&CompiledStep) -> bool) -> bool {
    steps.iter().any(|s| {
        pred(s)
            || match s {
                CompiledStep::If {
                    then, else_branch, ..
                } => process_contains(then, pred) || process_contains(else_branch, pred),
                _ => false,
            }
    })
}

/// Play P-108 from hand index 0 and resolve the reveal by picking the first
/// bucket candidate whenever one is offered (a single purple card = one legal
/// pick). Returns once the Option has been seated as a Delay permanent.
fn play_main_and_seat(runner: &mut DebugRunner) -> usize {
    let result = runner.game.play_option_from_hand(0, 0);
    assert_ne!(result, OptionPlayResult::Invalid, "P-108 must be usable");
    let _ = runner.auto_resolve();
    find_delayed(runner, 0).expect("P-108 is seated in P0's battle area as a Delay option")
}

/// Advance until the Delay may legally be activated (turn_count > placed_on_turn).
fn advance_past_placing_turn(runner: &mut DebugRunner, delay_idx: usize) {
    let placed_on_turn = match runner.game.players[0].battle_area[delay_idx].option_state {
        OptionState::Delayed { placed_on_turn, .. } => placed_on_turn,
        _ => panic!("expected a Delay option at index {delay_idx}"),
    };
    for _ in 0..4 {
        if runner.game.turn_count > placed_on_turn && runner.game.turn_player() == 0 {
            break;
        }
        runner.game.set_memory(3);
        runner.end_turn();
        runner.game.enter_main_phase();
    }
    assert_eq!(runner.game.turn_player(), 0);
}

// ─── Section 1: structural ───────────────────────────────────────────────────

#[test]
fn p_108_metadata_and_clause_shapes() {
    let runner = base().start();
    let compiled = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(compiled.kind, CompiledCardKind::Option);
    assert_eq!(compiled.cost, Some(2));

    let main = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when == vec![CompiledTiming::MainFromHand] => Some(t),
            _ => None,
        })
        .expect("[Main] reveal clause");
    assert!(!main.optional, "the reveal is mandatory once used");

    let delay = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::Delay { trigger, process, .. }) => {
                Some((trigger, process))
            }
            _ => None,
        })
        .expect("<Delay> clause");
    assert_eq!(*delay.0, CompiledTiming::Delayed, "standard player-activated [Main] <Delay>");
    assert!(
        process_contains(delay.1, &|s| matches!(s, CompiledStep::EffectInitiatedDigivolve { .. })),
        "the Delay body digivolves from hand (nested under the 'may' pick's `if binding_exists`)"
    );

    let sec = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when == vec![CompiledTiming::OnSecurity] => Some(t),
            _ => None,
        })
        .expect("[Security] clause");
    assert_eq!(sec.scope, CompiledScope::Inherited);
    assert!(sec
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::PlaceSelfAsDelayOption)));
}

// ─── Section 2: [Main] reveal 2 / add 1 purple / bottom the rest / seat ──────

#[test]
fn p_108_main_adds_the_purple_card_bottoms_the_other_and_seats_itself() {
    let mut runner = base()
        .hand(0, &[CARD_ID])
        // bottom → top: the revealed top 2 are RED-A (top) and PURPLE-A.
        .deck(0, &["RED-B", "RED-B", "RED-B", "PURPLE-A", "RED-A"])
        .memory(5)
        .start();
    runner.place_on_field(0, "PURPLE-B", Some(0)); // colour requirement
    let deck_before = runner.deck_size(0);

    assert_eq!(runner.game.play_option_from_hand(0, 0), OptionPlayResult::Pending);
    let view = runner.pending_selection_view().expect("reveal bucket prompt");
    assert!(matches!(view.kind, SelectionKind::RevealBucket { .. }));
    assert_eq!(view.valid_action_ids.iter().filter(|&&a| a != PASS).count(), 1, "only PURPLE-A is a legal pick");
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("add PURPLE-A");
    let _ = runner.auto_resolve();

    assert_eq!(hand_ids(&runner, 0), vec!["PURPLE-A".to_string()], "the purple card is in hand");
    assert_eq!(runner.deck_size(0), deck_before - 2 + 1, "2 revealed, 1 returned to the bottom");
    assert_eq!(deck_bottom_ids(&runner, 0, 1), vec!["RED-A".to_string()], "RED-A went to the bottom");
    assert!(find_delayed(&runner, 0).is_some(), "P-108 is placed in the battle area as a Delay option");
    assert_eq!(runner.memory(), 3, "cost 2 paid");
}

#[test]
fn p_108_main_with_two_purple_cards_lets_the_player_choose_one() {
    let mut runner = base()
        .hand(0, &[CARD_ID])
        // bottom → top: both revealed cards are purple.
        .deck(0, &["RED-B", "RED-B", "PURPLE-B", "PURPLE-A"])
        .memory(5)
        .start();
    runner.place_on_field(0, "RED-A", Some(0));
    runner.place_on_field(0, "PURPLE-B", Some(0));

    assert_eq!(runner.game.play_option_from_hand(0, 0), OptionPlayResult::Pending);
    let view = runner.pending_selection_view().expect("reveal bucket prompt");
    let picks: Vec<u16> = view.valid_action_ids.iter().copied().filter(|&a| a != PASS).collect();
    assert_eq!(picks.len(), 2, "both purple cards are offered");
    runner.execute_action(view.selecting_player, picks[1]).expect("pick the second purple");
    let _ = runner.auto_resolve();

    let hand = hand_ids(&runner, 0);
    assert_eq!(hand.len(), 1, "exactly one card added");
    assert!(hand[0].starts_with("PURPLE-"));
    assert_eq!(deck_bottom_ids(&runner, 0, 1).len(), 1);
    assert!(deck_bottom_ids(&runner, 0, 1)[0].starts_with("PURPLE-"), "the other purple went to the bottom");
}

#[test]
fn p_108_main_with_no_purple_card_bottoms_both_and_still_seats_itself() {
    let mut runner = base()
        .hand(0, &[CARD_ID])
        // bottom → top: the revealed top 2 are RED-A and RED-B; the purple
        // cards sit beneath and are never revealed.
        .deck(0, &["PURPLE-A", "PURPLE-A", "RED-B", "RED-A"])
        .memory(5)
        .start();
    runner.place_on_field(0, "PURPLE-B", Some(0));
    let deck_before = runner.deck_size(0);

    let _ = runner.game.play_option_from_hand(0, 0);
    let _ = runner.auto_resolve();

    assert!(hand_ids(&runner, 0).is_empty(), "nothing added to hand");
    assert_eq!(runner.deck_size(0), deck_before, "both revealed cards returned to the deck");
    let bottom = deck_bottom_ids(&runner, 0, 2);
    assert!(bottom.contains(&"RED-A".to_string()) && bottom.contains(&"RED-B".to_string()));
    assert!(find_delayed(&runner, 0).is_some(), "still placed in the battle area");
}

// ─── Section 3: <Delay> — digivolve a Digimon into a purple hand card, −2 ────

#[test]
fn p_108_delay_digivolves_into_a_purple_hand_card_for_cost_minus_2() {
    let mut runner = base()
        .hand(0, &[CARD_ID, "PURPLE-EVO", "RED-EVO"])
        .deck(0, &["RED-A"; 6])
        .memory(10)
        .start();
    let base_digimon = runner.place_on_field(0, "PURPLE-A", Some(0));
    let delay_idx = play_main_and_seat(&mut runner);
    advance_past_placing_turn(&mut runner, delay_idx);
    runner.game.set_memory(10);

    let delay_handle = PermanentHandle { player: 0, index: delay_idx as u8 };
    assert!(
        runner.game.activate_delayed_option_main(delay_handle),
        "the <Delay> is activatable after the placing turn"
    );

    let view = runner.pending_selection_view().expect("choose the Digimon that digivolves");
    assert_eq!(view.kind, SelectionKind::OwnField);
    assert!(view.is_optional, "'1 of your Digimon MAY digivolve' — PASS is legal");
    let pick = *view.valid_action_ids.iter().find(|&&a| a != PASS).expect("PURPLE-A is offered");
    runner.execute_action(view.selecting_player, pick).expect("choose PURPLE-A");

    let hand_view = runner.pending_selection_view().expect("choose the purple hand card");
    assert_eq!(hand_view.kind, SelectionKind::Hand);
    assert_eq!(
        hand_view.valid_action_ids.iter().filter(|&&a| a != PASS).count(),
        1,
        "only the purple Digimon card is offered (RED-EVO excluded)"
    );
    runner
        .execute_action(hand_view.selecting_player, hand_view.valid_action_ids[0])
        .expect("digivolve into PURPLE-EVO");
    let _ = runner.auto_resolve();

    assert_eq!(runner.memory(), 8, "digivolution cost 4 reduced by 2 → 2 memory paid");
    let evolved = &runner.game.players[0].battle_area[base_digimon.index as usize];
    assert_eq!(evolved.top_card().card_id(&runner.game.card_data), "PURPLE-EVO");
    assert!(find_delayed(&runner, 0).is_none(), "P-108 was trashed as the <Delay> cost");
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == CARD_ID),
        "P-108 is in the trash"
    );
}

#[test]
fn p_108_delay_declining_the_digivolve_changes_no_digimon() {
    let mut runner = base()
        .hand(0, &[CARD_ID, "PURPLE-EVO"])
        .deck(0, &["RED-A"; 6])
        .memory(10)
        .start();
    let base_digimon = runner.place_on_field(0, "PURPLE-A", Some(0));
    let delay_idx = play_main_and_seat(&mut runner);
    advance_past_placing_turn(&mut runner, delay_idx);
    runner.game.set_memory(10);

    let delay_handle = PermanentHandle { player: 0, index: delay_idx as u8 };
    assert!(runner.game.activate_delayed_option_main(delay_handle));
    let view = runner.pending_selection_view().expect("optional Digimon pick");
    assert!(view.is_optional);
    runner.execute_action(view.selecting_player, PASS).expect("decline");
    let _ = runner.auto_resolve();

    assert_eq!(runner.memory(), 10, "no digivolution cost paid");
    let still = &runner.game.players[0].battle_area[base_digimon.index as usize];
    assert_eq!(still.top_card().card_id(&runner.game.card_data), "PURPLE-A");
    assert!(hand_ids(&runner, 0).contains(&"PURPLE-EVO".to_string()));
}

#[test]
fn p_108_delay_cannot_be_activated_on_the_placing_turn() {
    let mut runner = base()
        .hand(0, &[CARD_ID, "PURPLE-EVO"])
        .deck(0, &["RED-A"; 6])
        .memory(10)
        .start();
    runner.place_on_field(0, "PURPLE-A", Some(0));
    let delay_idx = play_main_and_seat(&mut runner);
    let delay_handle = PermanentHandle { player: 0, index: delay_idx as u8 };
    assert!(
        !runner.game.activate_delayed_option_main(delay_handle),
        "<Delay> may only be activated the next turn or later"
    );
    assert!(find_delayed(&runner, 0).is_some());
}

#[test]
fn p_108_seated_delay_carries_the_main_phase_activated_trigger() {
    let mut runner = base()
        .hand(0, &[CARD_ID])
        .deck(0, &["RED-A"; 6])
        .memory(10)
        .start();
    runner.place_on_field(0, "PURPLE-A", Some(0));
    let delay_idx = play_main_and_seat(&mut runner);
    assert!(matches!(
        runner.game.players[0].battle_area[delay_idx].option_state,
        OptionState::Delayed { trigger: DelayTrigger::MainPhaseActivated, .. }
    ));
}

// ─── Section 4: [Security] place this card in the battle area ────────────────

#[test]
fn p_108_security_places_itself_in_the_owners_battle_area() {
    let mut attacker = make_test_card("ATK", "ATK");
    attacker.dp = Some(5000);
    let mut runner = base()
        .add_card(attacker)
        .deck(0, &["RED-A"; 6])
        .security(1, &[CARD_ID])
        .memory(3)
        .start();
    let atk = runner.place_on_field(0, "ATK", Some(0));

    let _ = runner.attack_player(atk, 1, false);
    let _ = runner.auto_resolve();

    assert_eq!(runner.security_count(1), 0);
    assert_eq!(runner.trash_size(1), 0, "not disposed to trash");
    let placed = runner.game.players[1]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID)
        .expect("P-108 placed in P1's battle area");
    assert!(matches!(
        placed.option_state,
        OptionState::Delayed { owner: 1, trigger: DelayTrigger::MainPhaseActivated, .. }
    ));
}
