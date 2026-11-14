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
            CompiledClause::Declarative(CompiledDeclarativeClause::Delay {
                trigger,
                process,
                ..
            }) => Some((trigger, process)),
            _ => None,
        })
        .expect("<Delay> clause");
    assert_eq!(
        *delay.0,
        CompiledTiming::Delayed,
        "standard player-activated [Main] <Delay>"
    );
    assert!(
        process_contains(delay.1, &|s| matches!(
            s,
            CompiledStep::EffectInitiatedDigivolve { .. }
        )),
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

    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending
    );
    let view = runner
        .pending_selection_view()
        .expect("reveal bucket prompt");
    assert!(matches!(view.kind, SelectionKind::RevealBucket { .. }));
    assert_eq!(
        view.valid_action_ids.iter().filter(|&&a| a != PASS).count(),
        1,
        "only PURPLE-A is a legal pick"
    );
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("add PURPLE-A");
    let _ = runner.auto_resolve();

    assert_eq!(
        hand_ids(&runner, 0),
        vec!["PURPLE-A".to_string()],
        "the purple card is in hand"
    );
    assert_eq!(
        runner.deck_size(0),
        deck_before - 2 + 1,
        "2 revealed, 1 returned to the bottom"
    );
    assert_eq!(
        deck_bottom_ids(&runner, 0, 1),
        vec!["RED-A".to_string()],
        "RED-A went to the bottom"
    );
    assert!(
        find_delayed(&runner, 0).is_some(),
        "P-108 is placed in the battle area as a Delay option"
    );
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

    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending
    );
    let view = runner
        .pending_selection_view()
        .expect("reveal bucket prompt");
    let picks: Vec<u16> = view
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != PASS)
        .collect();
    assert_eq!(picks.len(), 2, "both purple cards are offered");
    runner
        .execute_action(view.selecting_player, picks[1])
        .expect("pick the second purple");
    let _ = runner.auto_resolve();

    let hand = hand_ids(&runner, 0);
    assert_eq!(hand.len(), 1, "exactly one card added");
    assert!(hand[0].starts_with("PURPLE-"));
    assert_eq!(deck_bottom_ids(&runner, 0, 1).len(), 1);
    assert!(
        deck_bottom_ids(&runner, 0, 1)[0].starts_with("PURPLE-"),
        "the other purple went to the bottom"
    );
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
    assert_eq!(
        runner.deck_size(0),
        deck_before,
        "both revealed cards returned to the deck"
    );
    let bottom = deck_bottom_ids(&runner, 0, 2);
    assert!(bottom.contains(&"RED-A".to_string()) && bottom.contains(&"RED-B".to_string()));
    assert!(
        find_delayed(&runner, 0).is_some(),
        "still placed in the battle area"
    );
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

    let delay_handle = PermanentHandle {
        player: 0,
        index: delay_idx as u8,
    };
    assert!(
        runner.game.activate_delayed_option_main(delay_handle),
        "the <Delay> is activatable after the placing turn"
    );

    let view = runner
        .pending_selection_view()
        .expect("choose the Digimon that digivolves");
    assert_eq!(view.kind, SelectionKind::OwnField);
    assert!(
        view.is_optional,
        "'1 of your Digimon MAY digivolve' — PASS is legal"
    );
    let pick = *view
        .valid_action_ids
        .iter()
        .find(|&&a| a != PASS)
        .expect("PURPLE-A is offered");
    runner
        .execute_action(view.selecting_player, pick)
        .expect("choose PURPLE-A");

    let hand_view = runner
        .pending_selection_view()
        .expect("choose the purple hand card");
    assert_eq!(hand_view.kind, SelectionKind::Hand);
    assert_eq!(
        hand_view
            .valid_action_ids
            .iter()
            .filter(|&&a| a != PASS)
            .count(),
        1,
        "only the purple Digimon card is offered (RED-EVO excluded)"
    );
    runner
        .execute_action(hand_view.selecting_player, hand_view.valid_action_ids[0])
        .expect("digivolve into PURPLE-EVO");
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.memory(),
        8,
        "digivolution cost 4 reduced by 2 → 2 memory paid"
    );
    let evolved = &runner.game.players[0].battle_area[base_digimon.index as usize];
    assert_eq!(
        evolved.top_card().card_id(&runner.game.card_data),
        "PURPLE-EVO"
    );
    assert!(
        find_delayed(&runner, 0).is_none(),
        "P-108 was trashed as the <Delay> cost"
    );
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

    let delay_handle = PermanentHandle {
        player: 0,
        index: delay_idx as u8,
    };
    assert!(runner.game.activate_delayed_option_main(delay_handle));
    let view = runner
        .pending_selection_view()
        .expect("optional Digimon pick");
    assert!(view.is_optional);
    runner
        .execute_action(view.selecting_player, PASS)
        .expect("decline");
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
    let delay_handle = PermanentHandle {
        player: 0,
        index: delay_idx as u8,
    };
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
        OptionState::Delayed {
            trigger: DelayTrigger::MainPhaseActivated,
            ..
        }
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
        OptionState::Delayed {
            owner: 1,
            trigger: DelayTrigger::MainPhaseActivated,
            ..
        }
    ));
}

// ─── <Delay> candidate legality (select-hand audit) ────────────────────────────
//
// DCGO P/Purple/P_108.cs:72-80 `CanSelectPermanentCondition` offers only an own
// battle-area Digimon for which SOME hand card passing `CanSelectCardCondition`
// (`IsDigimon && HasCardColor(Purple)`, :93) satisfies
// `CanPlayCardTargetFrame(permanent.PermanentFrame, false, …)` (:80; the
// `false` is `PayCost`, so DCGO's target check is route-only); the chosen Digimon then
// goes to `CardEffectCommons.DigivolveIntoHandOrTrashCard` (payCost true,
// reduceCost 2, :145), whose hand pick (CardEffectCommons.cs:859-880)
// offers only cards with `!CanNotEvolve(target) && CanPlayCardTargetFrame(frame,
// PayCost: true, …)` under the −2 reducer. CR 4.3 8-1-3-1: the revealed card
// needs a digivolution requirement the chosen Digimon meets; CR 4.3 1-3-11-1: a
// digivolution can't be declared when its cost can't be paid. So the hand pick
// offers only cards that can digivolve onto the target for the reduced cost
// (matches DCGO). The target pick offers only Digimon that have such a card:
// deliberately STRICTER than DCGO's route-only target check (CR 4.3 1-3-11-1 —
// a Digimon whose only candidates are unpayable can't digivolve). DCGO's extra
// choice there leads to an empty hand pick, which resolves identically to PASS.
mod delay_candidate_legality {
    use std::path::Path;

    use digimon_engine::action::space::{PASS, PLAY_HAND_START};
    use digimon_engine::card_data::{CardData, EvoCost};
    use digimon_engine::debug_runner::{make_test_card, DebugRunner};
    use digimon_engine::enums::{CardColor, CardKind, DelayTrigger, EffectTiming};
    use digimon_engine::permanent::{OptionState, PermanentHandle};
    use digimon_engine::selection::{SelectionKind, TriggerSource};

    const CARD: &str = "P-108";
    const COLOR: CardColor = CardColor::Purple;
    const OFF: CardColor = CardColor::Red;

    fn yaml() -> String {
        std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("cards/p/P-108.yaml"),
        )
        .expect("P-108 YAML")
    }

    fn digimon(id: &str, color: CardColor, level: u8) -> CardData {
        let mut c = make_test_card(id, id);
        c.card_kind = CardKind::Digimon;
        c.colors = vec![color];
        c.level = Some(level);
        c.play_cost = 3;
        c
    }

    /// A Purple Lv.`level` Digimon that digivolves from a Purple Lv.`from` for `cost`.
    fn evo(id: &str, level: u8, from: u8, cost: u16) -> CardData {
        let mut c = digimon(id, COLOR, level);
        c.evo_costs = vec![EvoCost {
            card_color: COLOR as u8,
            level: from,
            memory_cost: cost,
        }];
        c
    }

    fn runner(hand: &[&str], memory: i16) -> DebugRunner {
        DebugRunner::builder()
            .from_dsl_yaml(&yaml())
            .expect("P-108 YAML compiles")
            .add_card(digimon("BASE", COLOR, 3))
            .add_card(digimon("OFF-BASE", OFF, 5))
            // Lv.4 from a Lv.3 for 4 (2 after the reduction): legal and payable.
            .add_card(evo("EVO", 4, 3, 4))
            // Lv.6 from a Lv.5: BASE (Lv.3) meets no requirement of it.
            .add_card(evo("NO-ROUTE", 6, 5, 3))
            // Lv.4 from a Lv.3 for 13: 11 after the reduction, more than the
            // 10 memory payable from 0 (CR 4.3 1-4-2-2).
            .add_card(evo("PRICEY", 4, 3, 13))
            .add_card(make_test_card("FILL", "FILL"))
            .hand(0, hand)
            .deck(0, &["FILL"; 4])
            .deck(1, &["FILL"; 4])
            .memory(memory)
            .start()
    }

    /// Seat the Option as a Delay permanent and activate its <Delay> body.
    fn fire_delay(runner: &mut DebugRunner) {
        let handle = runner.place_on_field(0, CARD, Some(0));
        let placed_on_turn = runner.game.turn_count;
        runner.game.player_mut(0).battle_area[handle.index as usize].option_state =
            OptionState::Delayed {
                owner: 0,
                trash_on_turn: u16::MAX,
                trigger: DelayTrigger::MainPhaseActivated,
                placed_on_turn,
            };
        runner
            .game
            .enqueue_triggered(EffectTiming::DelayEffect, TriggerSource::Permanent(handle));
        runner.game.drain_effect_queue();
    }

    fn non_pass(runner: &DebugRunner) -> Vec<u16> {
        runner
            .pending_selection_view()
            .map(|v| v.valid_action_ids.into_iter().filter(|&a| a != PASS).collect())
            .unwrap_or_default()
    }

    fn hand_ids(runner: &DebugRunner) -> Vec<String> {
        runner.game.players[0]
            .hand
            .iter()
            .map(|c| c.card_id(&runner.game.card_data).to_string())
            .collect()
    }

    fn top_id(runner: &DebugRunner, handle: PermanentHandle) -> String {
        runner.game.players[0].battle_area[handle.index as usize]
            .top_card()
            .card_id(&runner.game.card_data)
            .to_string()
    }

    /// Choose the single offered Digimon, then return the offered hand cards.
    fn offered_hand_after_target(runner: &mut DebugRunner) -> Vec<String> {
        assert_eq!(runner.pending_kind(), Some(SelectionKind::OwnField));
        let targets = non_pass(runner);
        assert_eq!(targets.len(), 1, "only BASE is offered as the Digimon to digivolve");
        runner.execute_action(0, targets[0]).expect("choose BASE");
        assert_eq!(runner.pending_kind(), Some(SelectionKind::Hand));
        let hand = hand_ids(runner);
        non_pass(runner)
            .into_iter()
            .map(|a| hand[(a - PLAY_HAND_START) as usize].clone())
            .collect()
    }

    /// Regression guard: a Purple Digimon card whose digivolution requirement the
    /// chosen Digimon doesn't meet (CR 4.3 8-1-3-1) is not offered.
    #[test]
    fn p_108_delay_hand_pick_excludes_a_card_the_target_cannot_digivolve_into() {
        let mut runner = runner(&["NO-ROUTE", "EVO"], 10);
        let base = runner.place_on_field(0, "BASE", Some(0));
        fire_delay(&mut runner);
        assert_eq!(
            offered_hand_after_target(&mut runner),
            vec!["EVO".to_string()],
            "NO-ROUTE (Lv.6 from Lv.5) can't digivolve onto a Lv.3"
        );
        let pick = non_pass(&runner)[0];
        runner.execute_action(0, pick).expect("digivolve into EVO");
        let _ = runner.auto_resolve();
        assert_eq!(top_id(&runner, base), "EVO");
        assert_eq!(runner.memory(), 8, "4 − 2 = 2 memory paid");
    }

    /// A Purple Digimon card whose reduced digivolution cost can't be paid
    /// (CR 4.3 1-3-11-1) is not offered; DCGO's hand pick checks `PayCost: true`.
    #[test]
    fn p_108_delay_hand_pick_excludes_a_card_whose_reduced_cost_cannot_be_paid() {
        let mut runner = runner(&["PRICEY", "EVO"], 0);
        let base = runner.place_on_field(0, "BASE", Some(0));
        fire_delay(&mut runner);
        assert_eq!(
            offered_hand_after_target(&mut runner),
            vec!["EVO".to_string()],
            "PRICEY costs 11 after the reduction; only 10 memory is payable"
        );
        let pick = non_pass(&runner)[0];
        runner.execute_action(0, pick).expect("digivolve into EVO");
        let _ = runner.auto_resolve();
        assert_eq!(top_id(&runner, base), "EVO");
    }

    /// DCGO's target pick offers only a Digimon that some Purple hand card can
    /// digivolve onto: OFF-BASE (a Lv.5) has none, so only BASE is offered.
    #[test]
    fn p_108_delay_target_pick_excludes_a_digimon_with_no_legal_hand_card() {
        let mut runner = runner(&["EVO"], 10);
        let base = runner.place_on_field(0, "BASE", Some(0));
        let off = runner.place_on_field(0, "OFF-BASE", Some(0));
        fire_delay(&mut runner);
        assert!(runner.pending_is_optional(), "the printed 'may' keeps PASS");
        assert_eq!(offered_hand_after_target(&mut runner), vec!["EVO".to_string()]);
        let pick = non_pass(&runner)[0];
        runner.execute_action(0, pick).expect("digivolve into EVO");
        let _ = runner.auto_resolve();
        assert_eq!(top_id(&runner, base), "EVO", "the offered Digimon was BASE");
        assert_eq!(top_id(&runner, off), "OFF-BASE");
    }

    /// With only an unpayable Purple card in hand no Digimon can digivolve, so no
    /// Digimon is offered. Stricter than DCGO, whose target check is route-only
    /// (`CanPlayCardTargetFrame(…, false, …)` = `PayCost: false`, P/Purple/P_108.cs:80)
    /// and would offer BASE here: CR 4.3 1-3-11-1 — a Digimon with only unpayable
    /// candidates can't digivolve; DCGO's extra choice resolves identically to PASS.
    #[test]
    fn p_108_delay_with_only_an_unpayable_hand_card_offers_no_target() {
        let mut runner = runner(&["PRICEY"], 0);
        let base = runner.place_on_field(0, "BASE", Some(0));
        fire_delay(&mut runner);
        assert!(
            runner.pending_selection_view().is_none(),
            "no Digimon can digivolve, so nothing is asked"
        );
        assert_eq!(top_id(&runner, base), "BASE");
        assert_eq!(hand_ids(&runner), vec!["PRICEY".to_string()]);
    }
}
