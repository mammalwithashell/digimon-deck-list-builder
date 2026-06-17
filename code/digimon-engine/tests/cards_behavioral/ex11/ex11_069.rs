//! EX11-069 Yuuki — Tamer, Purple/Red, Cost 4. Traits: Liberator.
//!
//! # Printed card text (card image EX11-069 — authoritative for printed text)
//!
//! [Start of Your Main Phase] [On Play] By trashing 1 card in your hand, gain 1
//!   memory.
//! [Your Turn] [Once Per Turn] When one of your Digimon attacks, if you have 4
//!   or fewer cards in your hand, it may digivolve into a [Dark Dragon] or
//!   [Evil Dragon] trait Digimon card in the trash with the digivolution cost
//!   reduced by 1.
//! [End of All Turns] If you have 4 or fewer cards in your hand, by suspending
//!   this Tamer, you may return 1 [Evil], [Dark Dragon] or [Evil Dragon] trait
//!   card from your trash to the hand.
//! [Security] Play this card without paying the cost.
//!
//! # DCGO C# reference (behavioral tiebreaker)
//! DCGO/Assets/Scripts/CardEffect/EX11/Purple/EX11_069.cs
//!   - OnAllyAttack ([Your Turn] [Once Per Turn], isOptional:true): when the
//!     ATTACKING own Digimon attacks, hand <= 4 and a matching trash card
//!     exists → DigivolveIntoHandOrTrashCard(targetPermanent: AttackingPermanent,
//!     payCost:true, reduceCost:1, isHand:false). CanSelectCardCondition =
//!     IsDigimon && (EqualsTraits("Dark Dragon") || EqualsTraits("Evil Dragon")).
//!   - OnEndTurn ([End of All Turns], isOptional:true): hand <= 4 &&
//!     CanActivateSuspendCostEffect → suspend self (Tap), then SelectCardEffect
//!     (canNoSelect:true, maxCount Min(1,..)) AddHand from Trash.
//!     CanSelectCardCondition = EqualsTraits("Evil") || EqualsTraits("Dark
//!     Dragon") || EqualsTraits("Evil Dragon") (NOT restricted to Digimon).
//!
//! # Patterns covered (RUST_DSL_TEST_API §4.3)
//! - shared trash-for-memory body across StartOfYourMainPhase + OnPlay
//! - [Security] free play
//! - [Your Turn][OPT][OPT] on_ally_attack → effect_initiated_digivolve of the
//!   ATTACKER (target: event_target) into a trash card (source: <trash binding>),
//!   cost reduced by 1; once-per-turn; hand<=4 gate
//! - [End of All Turns] optional + activation_cost suspend_self → trait-union
//!   trash return to hand; hand<=4 gate

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{CompiledClause, CompiledStep, CompiledTiming};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{
    make_test_card, make_test_card_with_level, DebugRunner, DebugRunnerBuilder,
};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "EX11-069";

// ─── Fixture helpers ─────────────────────────────────────────────────────────

/// A purple Lv.4 [Dark Dragon] Digimon used as the ATTACKER whose digivolution
/// the [Your Turn] clause upgrades. Purple (card_color 6) so the trash card's
/// evo cost can match.
fn dark_dragon_lv4(card_id: &str) -> CardData {
    let mut card = make_test_card_with_level(card_id, card_id, 4);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Purple];
    card.dp = Some(5000);
    card.traits = vec!["Dark Dragon".to_string()];
    card
}

/// A purple Lv.5 [Dark Dragon] Digimon waiting in the trash to be digivolved
/// into. Printed evo cost 4 from a Lv.4 Purple base, so a -1 reduction is
/// observable as 3 memory spent.
fn dark_dragon_lv5(card_id: &str) -> CardData {
    let mut card = make_test_card_with_level(card_id, card_id, 5);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Purple];
    card.dp = Some(7000);
    card.traits = vec!["Dark Dragon".to_string()];
    card.evo_costs = vec![EvoCost {
        card_color: 6, // Purple — matches the attacker
        level: 4,
        memory_cost: 4,
    }];
    card
}

/// A purple Lv.5 [Evil Dragon] Digimon (the OTHER admitted attack-clause trait).
fn evil_dragon_lv5(card_id: &str) -> CardData {
    let mut card = dark_dragon_lv5(card_id);
    card.traits = vec!["Evil Dragon".to_string()];
    card
}

/// A non-Dark/Evil-Dragon Lv.5 Digimon — must NOT be offered by the attack clause.
fn plain_lv5(card_id: &str) -> CardData {
    let mut card = dark_dragon_lv5(card_id);
    card.traits = vec!["Reptile".to_string()];
    card
}

/// An [Evil]-trait NON-Digimon card (a Tamer) — the EoT clause admits it
/// (DCGO's EoT CanSelectCardCondition is NOT restricted to Digimon), while the
/// attack clause must NOT admit it (attack requires IsDigimon).
fn evil_tamer(card_id: &str) -> CardData {
    let mut card = make_test_card(card_id, card_id);
    card.card_kind = CardKind::Tamer;
    card.colors = vec![CardColor::Purple];
    card.traits = vec!["Evil".to_string()];
    card
}

/// A defender for the opponent so the attacker can declare an attack on a Digimon.
fn defender(card_id: &str) -> CardData {
    let mut card = make_test_card_with_level(card_id, card_id, 4);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Red];
    card.dp = Some(1000);
    card
}

fn push_to_hand(runner: &mut DebugRunner, player: u8, card_id: &str) -> usize {
    let data_index = runner
        .game
        .card_data
        .iter()
        .position(|card| card.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card id {card_id}"));
    let instance_id = runner.game.next_card_index();
    runner.game.players[player as usize]
        .hand
        .push(CardSource::new(data_index, player, instance_id));
    runner.game.players[player as usize].hand.len() - 1
}

fn push_to_trash(runner: &mut DebugRunner, player: u8, card_id: &str) {
    let data_index = runner
        .game
        .card_data
        .iter()
        .position(|card| card.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card id {card_id}"));
    let instance_id = runner.game.next_card_index();
    runner.game.players[player as usize]
        .trash
        .push(CardSource::new(data_index, player, instance_id));
}

/// Set the controller's hand to exactly `n` filler cards.
fn set_hand_size(runner: &mut DebugRunner, player: u8, n: usize) {
    runner.game.players[player as usize].hand.clear();
    for _ in 0..n {
        push_to_hand(runner, player, "FILL");
    }
}

/// Resolve the current pending selection by choosing the first NON-PASS action
/// (i.e. actually make the pick, rather than declining an optional prompt).
fn resolve_pick_non_pass(runner: &mut DebugRunner) {
    let view = runner
        .pending_selection_view()
        .expect("a selection must be pending to resolve a pick");
    let action = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|a| *a != digimon_engine::action::space::PASS)
        .expect("an actual (non-PASS) pick must be available");
    runner
        .game
        .resolve_selection(view.selecting_player, action)
        .expect("resolve the pick");
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX11-069 YAML parses and compiles")
        .add_card(dark_dragon_lv4("ATK-DD4"))
        .add_card(dark_dragon_lv5("DD5"))
        .add_card(evil_dragon_lv5("ED5"))
        .add_card(plain_lv5("PLAIN5"))
        .add_card(evil_tamer("EVIL-T"))
        .add_card(defender("DEF"))
        .add_card(make_test_card_with_level("FILL", "Filler", 3))
        .deck(0, &["FILL"; 12])
        .deck(1, &["FILL"; 12])
        .memory(10)
}

// ─── Section 1 — Structural (shared trash-for-memory + security) ─────────────

#[test]
fn ex11_069_has_start_main_on_play_and_security_clauses() {
    let runner = base().start();
    let card = runner.compiled_card("EX11-069").expect("compiled card");

    assert!(
        card.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::StartOfYourMainPhase)
                    && t.when.contains(&CompiledTiming::OnPlay)
        )),
        "EX11-069 must share one trash-for-memory body across StartOfYourMainPhase and OnPlay"
    );
    assert!(
        card.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSecurity)
        )),
        "EX11-069 must have Security play"
    );
}

#[test]
fn ex11_069_has_attack_and_end_of_all_turns_clauses() {
    let runner = base().start();
    let card = runner.compiled_card("EX11-069").expect("compiled card");

    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    // [Your Turn][Once Per Turn] attack observer, optional, with an
    // effect_initiated_digivolve step.
    let has_attack = triggered.iter().any(|t| {
        t.when.contains(&CompiledTiming::OnAllyAttack)
            && t.optional
            && t.process
                .iter()
                .any(|s| matches!(s, CompiledStep::EffectInitiatedDigivolve { .. }))
    });
    assert!(
        has_attack,
        "EX11-069 must carry an optional on_ally_attack clause with an effect_initiated_digivolve"
    );

    // [End of All Turns] = end_of_your_turn + end_of_opponents_turn, optional.
    let has_eot = triggered.iter().any(|t| {
        t.when.contains(&CompiledTiming::EndOfYourTurn)
            && t.when.contains(&CompiledTiming::EndOfOpponentsTurn)
            && t.optional
    });
    assert!(
        has_eot,
        "EX11-069 must carry an [End of All Turns] (end of your + opponents turn) optional clause"
    );
}

// ─── Section 2 — existing shared trash-for-memory body ───────────────────────

#[test]
fn ex11_069_on_play_trashes_one_card_and_gains_memory() {
    let discard = make_test_card("EX11-069-DISCARD", "Discard");
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-069")
        .expect("EX11-069 must load")
        .add_card(discard)
        .hand(0, &["EX11-069", "EX11-069-DISCARD"])
        .memory(4)
        .start();

    runner.play(0, 0);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.memory(),
        1,
        "Yuuki should gain 1 memory after paying the 4 play cost"
    );
    assert_eq!(
        runner.game.players[0].trash.len(),
        1,
        "one hand card should be trashed"
    );
}

// ─── Section 3 — [Your Turn][OPT][OPT] attack-digivolve from trash, cost -1 ──

/// With hand <= 4 and a matching [Dark Dragon] card in trash, attacking offers
/// the optional digivolve of the ATTACKER into the trash card.
#[test]
fn ex11_069_attack_observer_offers_trash_digivolve() {
    let mut runner = base().start();
    runner.game.turn_count = 1; // your turn
    let _yuuki = runner.place_on_field(0, CARD_ID, Some(0));
    let attacker = runner.place_on_field(0, "ATK-DD4", Some(0));
    let _defender = runner.place_on_field(1, "DEF", Some(0));
    push_to_trash(&mut runner, 0, "DD5");
    set_hand_size(&mut runner, 0, 4); // 4 or fewer

    let defender = PermanentHandle {
        player: 1,
        index: 0,
    };
    let _ = runner.attack_digimon(attacker, defender, false);

    assert!(
        runner.pending_kind().is_some(),
        "attacking with hand <= 4 and a matching trash card must offer Yuuki's digivolve"
    );
    runner.auto_resolve().ok();
}

/// The attacker actually digivolves into the trash card and the reduced cost is
/// paid (printed evo cost 4, minus 1 = 3 memory).
#[test]
fn ex11_069_attack_digivolve_stacks_trash_card_and_pays_reduced_cost() {
    let mut runner = base().start();
    runner.game.turn_count = 1;
    let _yuuki = runner.place_on_field(0, CARD_ID, Some(0));
    let attacker = runner.place_on_field(0, "ATK-DD4", Some(0));
    let _defender = runner.place_on_field(1, "DEF", Some(0));
    push_to_trash(&mut runner, 0, "DD5");
    set_hand_size(&mut runner, 0, 4);
    runner.game.memory = 10;

    let defender = PermanentHandle {
        player: 1,
        index: 0,
    };
    let _ = runner.attack_digimon(attacker, defender, false);

    // Optional clause whose body begins with a select (not a leading mandatory
    // cost): accept the outer offer if one installs, then pick the trash card.
    if runner.pending_kind() == Some(SelectionKind::Replacement) {
        runner
            .accept_optional_trigger()
            .expect("accept the optional attack-digivolve");
    }
    runner.auto_resolve().expect("resolve the trash digivolve");

    // (1) The attacker permanent's top card is now the trash card (DD5).
    let top = runner.game.players[0].battle_area[attacker.index as usize]
        .top_card()
        .data_index;
    let dd5_index = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "DD5")
        .unwrap();
    assert_eq!(
        top, dd5_index,
        "the attacker must be digivolved into the [Dark Dragon] trash card"
    );

    // (2) The trash card left the trash.
    assert!(
        !runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.data_index == dd5_index),
        "the digivolved card must no longer be in the trash"
    );

    // (3) Reduced cost: printed evo cost 4, reduced by 1 = 3 memory spent.
    assert_eq!(
        runner.memory(),
        7,
        "digivolve cost (4) reduced by 1 must cost exactly 3 memory (10 -> 7)"
    );
}

/// Hand size 5 (> 4) gates the clause off — no offer.
#[test]
fn ex11_069_attack_observer_gated_off_when_hand_over_four() {
    let mut runner = base().start();
    runner.game.turn_count = 1;
    let _yuuki = runner.place_on_field(0, CARD_ID, Some(0));
    let attacker = runner.place_on_field(0, "ATK-DD4", Some(0));
    let _defender = runner.place_on_field(1, "DEF", Some(0));
    push_to_trash(&mut runner, 0, "DD5");
    set_hand_size(&mut runner, 0, 5); // more than 4

    let defender = PermanentHandle {
        player: 1,
        index: 0,
    };
    let _ = runner.attack_digimon(attacker, defender, false);

    assert!(
        runner.pending_selection().is_none(),
        "hand > 4 must gate the attack-digivolve off (no selection offered)"
    );
    runner.auto_resolve().ok();
}

/// Only [Dark Dragon]/[Evil Dragon] Digimon in trash are eligible — a plain
/// trait Digimon is not offered, and an [Evil] non-Digimon (the EoT-only trait)
/// is not offered either (attack clause requires IsDigimon + Dark/Evil Dragon).
#[test]
fn ex11_069_attack_observer_filters_trait_and_kind() {
    let mut runner = base().start();
    runner.game.turn_count = 1;
    let _yuuki = runner.place_on_field(0, CARD_ID, Some(0));
    let attacker = runner.place_on_field(0, "ATK-DD4", Some(0));
    let _defender = runner.place_on_field(1, "DEF", Some(0));
    // Only ineligible cards in trash: a plain (Reptile) Lv.5 and an [Evil] Tamer.
    push_to_trash(&mut runner, 0, "PLAIN5");
    push_to_trash(&mut runner, 0, "EVIL-T");
    set_hand_size(&mut runner, 0, 4);

    let defender = PermanentHandle {
        player: 1,
        index: 0,
    };
    let _ = runner.attack_digimon(attacker, defender, false);

    assert!(
        runner.pending_selection().is_none(),
        "no [Dark Dragon]/[Evil Dragon] Digimon in trash → no digivolve offered"
    );
    runner.auto_resolve().ok();
}

// ─── Section 4 — [End of All Turns] suspend → return trait card from trash ───

/// With hand <= 4 and an eligible trash card, the optional EoT clause fires;
/// accepting suspends Yuuki (the cost) and returns the trait card to hand.
///
/// The clause is fired in isolation via `enqueue_triggered` +
/// `drain_effect_queue` (the BT20-084 idiom) rather than through `end_turn`, so
/// the assertion observes the clause's own effect without the turn-rotation
/// begin-of-turn unsuspend confounding the suspend-cost check.
#[test]
fn ex11_069_end_of_all_turns_returns_evil_or_dragon_trait_card() {
    let mut runner = base().start();
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 0;
    let yuuki = runner.place_on_field(0, CARD_ID, Some(0));
    push_to_trash(&mut runner, 0, "EVIL-T"); // [Evil]-trait card in trash
    set_hand_size(&mut runner, 0, 4);

    let hand_before = runner.hand_size(0);
    runner
        .game
        .enqueue_triggered(EffectTiming::EndOfYourTurn, TriggerSource::PlayerBattleArea(0));
    runner.game.drain_effect_queue();

    // The optional [End of All Turns] clause (cost-bearing) surfaces a pre-cost
    // TriggerOrder accept/decline prompt. Accept it, then pick the trash card.
    let view = runner
        .pending_selection_view()
        .expect("EoT clause must offer a trigger to order/accept");
    assert_eq!(
        view.kind,
        SelectionKind::TriggerOrder,
        "the optional EoT clause must surface as a TriggerOrder entry"
    );
    runner
        .game
        .resolve_selection(view.selecting_player, view.valid_action_ids[0])
        .expect("accept the EoT suspend-and-return clause from the TriggerOrder");

    // The suspend cost is paid up front (before the optional return pick).
    assert!(
        runner.game.players[0].battle_area[yuuki.index as usize].is_suspended,
        "Yuuki must be suspended (the printed cost) once the clause is accepted"
    );

    // Make the actual return pick (don't decline), then drain the add-to-hand.
    resolve_pick_non_pass(&mut runner);
    runner.auto_resolve().expect("resolve the trash return");

    // The [Evil] card moved trash → hand.
    let evil_index = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "EVIL-T")
        .unwrap();
    assert!(
        runner.game.players[0]
            .hand
            .iter()
            .any(|c| c.data_index == evil_index),
        "the [Evil] trait card must be returned to hand"
    );
    assert!(
        !runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.data_index == evil_index),
        "the returned card must leave the trash"
    );
    assert_eq!(
        runner.hand_size(0),
        hand_before + 1,
        "hand grows by exactly the 1 returned card"
    );

    // Yuuki stays suspended after the return resolves (the cost is not refunded).
    assert!(
        runner.game.players[0].battle_area[yuuki.index as usize].is_suspended,
        "Yuuki remains suspended after the return resolves"
    );
}

/// Hand size 5 (> 4) gates the EoT clause off — Yuuki stays unsuspended and no
/// selection is offered.
#[test]
fn ex11_069_end_of_all_turns_gated_off_when_hand_over_four() {
    let mut runner = base().start();
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 0;
    let yuuki = runner.place_on_field(0, CARD_ID, Some(0));
    push_to_trash(&mut runner, 0, "EVIL-T");
    set_hand_size(&mut runner, 0, 5); // more than 4

    runner
        .game
        .enqueue_triggered(EffectTiming::EndOfYourTurn, TriggerSource::PlayerBattleArea(0));
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "hand > 4 must gate the EoT return off"
    );
    assert!(
        !runner.game.players[0].battle_area[yuuki.index as usize].is_suspended,
        "with the clause gated off, Yuuki must not be suspended"
    );
}

/// The EoT trait filter also admits [Dark Dragon] and [Evil Dragon] cards
/// (the union of three traits), not just [Evil].
#[test]
fn ex11_069_end_of_all_turns_admits_dragon_trait_cards() {
    let mut runner = base().start();
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 0;
    let yuuki = runner.place_on_field(0, CARD_ID, Some(0));
    push_to_trash(&mut runner, 0, "ED5"); // [Evil Dragon] Digimon in trash
    set_hand_size(&mut runner, 0, 3);

    runner
        .game
        .enqueue_triggered(EffectTiming::EndOfYourTurn, TriggerSource::PlayerBattleArea(0));
    runner.game.drain_effect_queue();
    let view = runner
        .pending_selection_view()
        .expect("EoT clause should offer with an [Evil Dragon] trait card present");
    assert_eq!(
        view.kind,
        SelectionKind::TriggerOrder,
        "the optional EoT clause must surface as a TriggerOrder entry"
    );
    runner
        .game
        .resolve_selection(view.selecting_player, view.valid_action_ids[0])
        .expect("accept the EoT return clause");
    resolve_pick_non_pass(&mut runner);
    runner.auto_resolve().expect("resolve");

    let ed5_index = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "ED5")
        .unwrap();
    assert!(
        runner.game.players[0]
            .hand
            .iter()
            .any(|c| c.data_index == ed5_index),
        "the [Evil Dragon] card must be returnable to hand"
    );
}
