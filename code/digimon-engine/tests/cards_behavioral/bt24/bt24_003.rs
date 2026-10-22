//! BT24-003 Tsunomon — Digi-Egg, Lv.2, Yellow, Cost 0.
//! Traits: [Lesser, Iliad, TS]. Form: In-Training.
//!
//! # Card text (per-card JSON `code/digimon-engine/cards/bt24/BT24-003.json` —
//! # verbatim)
//!
//! Inherited Effect [Your Turn] [Once Per Turn] When your security stack is
//! removed from, this Digimon may digivolve into a [Shaman] trait Digimon
//! card in the hand with the digivolution cost reduced by 1.
//!
//! (No own/security-face effect text — Tsunomon has exactly one clause.)
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT24/Yellow/BT24_003.cs
//!
//! ActivateClass (SetUpActivateClass(..., isInherited: true, canNoSelect:
//! true)) gated by `EffectTiming.OnLoseSecurity` +
//! `CanTriggerWhenLoseSecurity(hashtable, player => player == card.Owner)`
//! (own security only) + `IsOwnerTurn(card)` ([Your Turn]). Body:
//! `DigivolveIntoHandOrTrashCard(cardCondition: IsDigimon &&
//! EqualsTraits("Shaman") && CanPlayCardTargetFrame(...), payCost: true,
//! reduceCostTuple: (1, null), isHand: true)`. The digivolve target is
//! declinable (canNoSelect: true → "may digivolve").
//!
//! # Patterns this test covers (RUST_DSL_TEST_API.md §4.3)
//! - G4-adjacent: inherited trigger on a Digi-Egg (own-security-removed
//!   analogue of "inherited When Attacking on DigiEgg")
//! - E2 OPT + optional decline
//! - D2-adjacent: cost reduction via `effect_initiated_digivolve` (`cost: {
//!   reduce: 1 }`), BT12-016 idiom
//! - "[Your Turn]" turn gate via `active_when: { your_turn: true }`
//!   (AD1-021 / AD1-017 idiom)

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledPredicate, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::EffectTiming;
use digimon_engine::selection::{SelectionKind, TriggerSource};
use digimon_engine::trigger_context::EventCause;

const CARD_ID: &str = "BT24-003";

/// Recursively search a CompiledPredicate tree for a node satisfying `f`.
fn pred_any<F: Fn(&CompiledPredicate) -> bool + Copy>(p: &CompiledPredicate, f: F) -> bool {
    f(p) || p.all_of.iter().any(|q| pred_any(q, f))
        || p.any_of.iter().any(|q| pred_any(q, f))
        || p.none_of.iter().any(|q| pred_any(q, f))
        || p.not.as_ref().map(|q| pred_any(q, f)).unwrap_or(false)
}

fn base() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT24-003 YAML parses and compiles")
        // ST3-02 Salamon: Lv.3, yellow, standard alt-path `from: { level_eq:
        // 2, color_is: yellow }` — Tsunomon (Lv.2 yellow) is a legal
        // digivolution source under it, so a stack Tsunomon(source) →
        // ST3-02(top) gives the inherited clause a real host permanent.
        .dsl_card("ST3-02")
        .expect("ST3-02 YAML parses")
        // BT24-034 Aegiomon: Lv.4, yellow, Shaman trait, standard alt-path
        // `from: { level_eq: 3, color_is: yellow }` (also TS-trait branch) —
        // the exact "[Shaman] trait Digimon card" this effect can dig into,
        // legally reachable from ST3-02's Lv.3 top.
        .dsl_card("BT24-034")
        .expect("BT24-034 YAML parses")
        .add_card(make_test_card("FILLER-DECK", "Filler"))
        .add_card(make_test_card("ATTACKER", "Attacker"))
        .deck(0, &["FILLER-DECK"; 10])
        .deck(1, &["FILLER-DECK"; 10])
        .memory(10)
        .start()
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn bt24_003_yaml_has_printed_metadata() {
    let runner = base();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.name, "Tsunomon");
    assert_eq!(card.level, Some(2));
    assert_eq!(card.cost, Some(0));
}

#[test]
fn bt24_003_has_exactly_one_inherited_own_security_removed_clause() {
    let runner = base();
    let card = runner.compiled_card(CARD_ID).expect("compiled");

    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    assert_eq!(
        triggered.len(),
        1,
        "Tsunomon has exactly one triggered clause (no own/security-face text)"
    );
    let clause = triggered[0];
    assert_eq!(clause.scope, CompiledScope::Inherited, "Inherited Effect");
    assert!(
        clause.when.contains(&CompiledTiming::OnOwnSecurityRemoved),
        "fires when the controller's own security stack is removed from"
    );
    assert!(clause.optional, "'may digivolve' is optional");
    assert!(clause.once_per_turn, "[Once Per Turn]");
}

#[test]
fn bt24_003_clause_is_gated_to_your_turn() {
    let runner = base();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnOwnSecurityRemoved))
        .expect("own-security-removed clause");

    assert!(
        clause
            .active_when
            .as_ref()
            .map(|p| pred_any(p, |q| q.your_turn == Some(true)))
            .unwrap_or(false),
        "[Your Turn] must be modeled as active_when: {{ your_turn: true }}, not all_turns"
    );
    assert!(
        !clause
            .active_when
            .as_ref()
            .map(|p| pred_any(p, |q| q.all_turns == Some(true)))
            .unwrap_or(false),
        "must NOT be gated as [All Turns] — printed text says [Your Turn]"
    );
}

#[test]
fn bt24_003_process_uses_effect_initiated_digivolve_with_cost_reduction() {
    let runner = base();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnOwnSecurityRemoved))
        .expect("own-security-removed clause");

    use digimon_dsl::compiled::CompiledCostDelta;
    // The digivolve sits in the `if binding_present` branch; the `else`
    // branch refunds the [Once Per Turn] (Q1818).
    fn any_step(steps: &[CompiledStep], f: &dyn Fn(&CompiledStep) -> bool) -> bool {
        steps.iter().any(|s| {
            f(s) || match s {
                CompiledStep::If {
                    then, else_branch, ..
                } => any_step(then, f) || any_step(else_branch, f),
                _ => false,
            }
        })
    }
    let has_reduced_digivolve = any_step(&clause.process, &|s| {
        matches!(
            s,
            CompiledStep::EffectInitiatedDigivolve {
                cost: CompiledCostDelta::Reduce(1),
                ..
            }
        )
    });
    assert!(
        any_step(&clause.process, &|s| matches!(s, CompiledStep::RefundOpt)),
        "declining the pick refunds the [Once Per Turn] (Q1818)"
    );
    assert!(
        has_reduced_digivolve,
        "must digivolve via effect_initiated_digivolve with cost reduced by 1; got {:?}",
        clause.process
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 2 — Condition gating: own-turn vs opponent's turn
// ═══════════════════════════════════════════════════════════════════════════

/// Positive: on the controller's own turn, breaking their own security via
/// combat installs the optional digivolve-into-Shaman prompt.
#[test]
fn bt24_003_fires_on_controllers_own_turn() {
    let mut runner = base();
    // Tsunomon buried as a source under Salamon (Lv.3 yellow) on player 0.
    let host = runner.place_stack(0, &["BT24-003", "ST3-02"]);
    // Give player 0 a Shaman-trait hand card to digivolve into.
    let aegiomon_idx = {
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "BT24-034")
            .unwrap();
        let next_idx = runner.game.next_card_index();
        let card = digimon_engine::card_source::CardSource::new(data_idx, 0, next_idx);
        runner.game.players[0].hand.push(card);
        runner.game.players[0].hand.len() - 1
    };
    // A lone security card for player 0; player 1 attacks it (default turn
    // player is player 0 — this is still player 0's own security, and the
    // active_when gate reads "your turn" against the effect's controller
    // (player 0) vs the CURRENT turn player, so we need it to be player 0's
    // turn while player 0's OWN security is what breaks. Use the direct
    // enqueue idiom (AD1-017 precedent) to isolate the your-turn gate
    // itself; the full attack-driven path is covered by the opponent's-turn
    // negative test and the end-to-end integrated test below.
    runner.game.players[0].security.clear();
    let sec_data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "FILLER-DECK")
        .unwrap();
    let sec_idx = runner.game.next_card_index();
    runner.game.players[0]
        .security
        .push(digimon_engine::card_source::CardSource::new(
            sec_data_idx,
            0,
            sec_idx,
        ));

    assert_eq!(runner.game.turn_player(), 0, "default turn player is P0");

    runner.game.enqueue_triggered(
        EffectTiming::OnOwnSecurityRemoved,
        TriggerSource::SecurityRemoved {
            affected_player: 0,
            observer_player: 0,
            source_player: 1,
            card: CardHandle(0),
            cause: EventCause::SecurityRemoval,
        },
    );
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_some(),
        "on the controller's own turn, the optional digivolve prompt must install"
    );
    assert!(
        runner.pending_is_optional(),
        "'may digivolve' is declinable"
    );
    let _ = (host, aegiomon_idx);
}

/// Negative: when it is the OPPONENT's turn, the [Your Turn] gate must
/// suppress the trigger even though it is still the controller's OWN
/// security that was removed from.
#[test]
fn bt24_003_does_not_fire_on_opponents_turn() {
    let mut runner = base();
    let _host = runner.place_stack(0, &["BT24-003", "ST3-02"]);
    {
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "BT24-034")
            .unwrap();
        let next_idx = runner.game.next_card_index();
        let card = digimon_engine::card_source::CardSource::new(data_idx, 0, next_idx);
        runner.game.players[0].hand.push(card);
    }

    // Advance to player 1's turn.
    runner.end_turn();
    assert_eq!(
        runner.game.turn_player(),
        1,
        "after end_turn it is now P1's turn"
    );

    runner.game.enqueue_triggered(
        EffectTiming::OnOwnSecurityRemoved,
        TriggerSource::SecurityRemoved {
            affected_player: 0,
            observer_player: 0,
            source_player: 1,
            card: CardHandle(0),
            cause: EventCause::SecurityRemoval,
        },
    );
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "[Your Turn] must block the trigger when it is the opponent's turn, \
         even though it is still the controller's own security stack"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 3 — Behavioral outcome: accept / decline the digivolve
// ═══════════════════════════════════════════════════════════════════════════

/// Accepting the prompt and picking the Shaman hand card digivolves Salamon
/// into Aegiomon with the cost reduced by 1 (printed cost 2 → paid 1).
#[test]
fn bt24_003_accepting_digivolves_into_shaman_hand_card() {
    let mut runner = base();
    let host = runner.place_stack(0, &["BT24-003", "ST3-02"]);
    let aegiomon_hand_idx = {
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "BT24-034")
            .unwrap();
        let next_idx = runner.game.next_card_index();
        let card = digimon_engine::card_source::CardSource::new(data_idx, 0, next_idx);
        runner.game.players[0].hand.push(card);
        runner.game.players[0].hand.len() - 1
    };
    let memory_before = runner.memory();
    let hand_before = runner.game.players[0].hand.len();

    runner.game.enqueue_triggered(
        EffectTiming::OnOwnSecurityRemoved,
        TriggerSource::SecurityRemoved {
            affected_player: 0,
            observer_player: 0,
            source_player: 1,
            card: CardHandle(0),
            cause: EventCause::SecurityRemoval,
        },
    );
    runner.game.drain_effect_queue();

    let view = runner
        .pending_selection_view()
        .expect("the optional digivolve prompt installs");
    // Drive the hand-card pick: accept by selecting the only legal
    // (non-PASS) action.
    let pick = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != PASS)
        .expect("a legal Shaman hand-card pick exists");
    runner
        .execute_action(view.selecting_player, pick)
        .expect("select the Shaman hand card");
    runner.game.drain_effect_queue();

    // The permanent's top card is now Aegiomon (BT24-034), not Salamon.
    let top_id = runner.game.players[0].battle_area[host.index as usize]
        .top_card()
        .card_id(&runner.game.card_data)
        .to_string();
    assert_eq!(
        top_id, "BT24-034",
        "the stack's top card must now be the digivolved Aegiomon"
    );
    // §8-1-3-3: digivolving draws 1 card, so the hand SIZE is unchanged --
    // assert the Aegiomon card's departure by identity, not by arithmetic.
    assert!(
        !runner.game.players[0]
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "BT24-034"),
        "the Aegiomon hand card left the hand"
    );
    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before,
        "the digivolve draw replaced it (§8-1-3-3)"
    );
    let _ = aegiomon_hand_idx;
    let _ = memory_before;
}

/// Declining the optional digivolve leaves the stack and hand untouched.
#[test]
fn bt24_003_declining_leaves_stack_untouched() {
    let mut runner = base();
    let host = runner.place_stack(0, &["BT24-003", "ST3-02"]);
    {
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "BT24-034")
            .unwrap();
        let next_idx = runner.game.next_card_index();
        let card = digimon_engine::card_source::CardSource::new(data_idx, 0, next_idx);
        runner.game.players[0].hand.push(card);
    }
    let hand_before = runner.game.players[0].hand.len();
    let top_before = runner.game.players[0].battle_area[host.index as usize]
        .top_card()
        .card_id(&runner.game.card_data)
        .to_string();

    runner.game.enqueue_triggered(
        EffectTiming::OnOwnSecurityRemoved,
        TriggerSource::SecurityRemoved {
            affected_player: 0,
            observer_player: 0,
            source_player: 1,
            card: CardHandle(0),
            cause: EventCause::SecurityRemoval,
        },
    );
    runner.game.drain_effect_queue();

    let view = runner
        .pending_selection_view()
        .expect("the optional digivolve prompt installs");
    assert!(view.is_optional, "PASS must be a legal action");
    runner
        .execute_action(view.selecting_player, PASS)
        .expect("decline the optional digivolve");
    runner.game.drain_effect_queue();

    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before,
        "declining must not consume the hand card"
    );
    let top_after = runner.game.players[0].battle_area[host.index as usize]
        .top_card()
        .card_id(&runner.game.card_data)
        .to_string();
    assert_eq!(
        top_after, top_before,
        "declining must not change the stack's top card"
    );
}

/// No eligible [Shaman] hand card → the (still-optional) selection has no
/// legal non-PASS action, so it degenerates to a no-op / does not install a
/// selection offering a candidate that cannot legally exist.
#[test]
fn bt24_003_no_shaman_hand_card_leaves_no_legal_pick() {
    let mut runner = base();
    let host = runner.place_stack(0, &["BT24-003", "ST3-02"]);
    // No Shaman-trait card in hand at all.
    let hand_before = runner.game.players[0].hand.len();

    runner.game.enqueue_triggered(
        EffectTiming::OnOwnSecurityRemoved,
        TriggerSource::SecurityRemoved {
            affected_player: 0,
            observer_player: 0,
            source_player: 1,
            card: CardHandle(0),
            cause: EventCause::SecurityRemoval,
        },
    );
    runner.game.drain_effect_queue();

    // Either no selection installs (empty optional select auto-skips), or a
    // selection installs with PASS as the only legal action. Either shape is
    // faithful to "no eligible target exists"; assert the outcome (hand
    // untouched, no crash) rather than the specific selection-absence shape.
    if let Some(view) = runner.pending_selection_view() {
        assert!(
            view.valid_action_ids.iter().all(|&a| a == PASS),
            "with no eligible [Shaman] hand card, PASS must be the only legal action"
        );
        runner
            .execute_action(view.selecting_player, PASS)
            .expect("decline");
        runner.game.drain_effect_queue();
    }
    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before,
        "hand must be untouched when no Shaman target exists"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 4 — OPT enforcement
// ═══════════════════════════════════════════════════════════════════════════

fn push_hand(runner: &mut DebugRunner, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("{card_id} registered"));
    let next_idx = runner.game.next_card_index();
    let card = digimon_engine::card_source::CardSource::new(data_idx, 0, next_idx);
    runner.game.players[0].hand.push(card);
}

fn remove_own_security(runner: &mut DebugRunner) {
    runner.game.enqueue_triggered(
        EffectTiming::OnOwnSecurityRemoved,
        TriggerSource::SecurityRemoved {
            affected_player: 0,
            observer_player: 0,
            source_player: 1,
            card: CardHandle(0),
            cause: EventCause::SecurityRemoval,
        },
    );
    runner.game.drain_effect_queue();
}

fn pick_first_candidate(runner: &mut DebugRunner) {
    let view = runner
        .pending_selection_view()
        .expect("the optional digivolve prompt installs");
    let pick = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != PASS)
        .expect("a legal Shaman hand-card pick exists");
    runner
        .execute_action(view.selecting_player, pick)
        .expect("select the Shaman hand card");
    runner.game.drain_effect_queue();
}

fn top_id(runner: &DebugRunner, host_index: u8) -> String {
    runner.game.players[0].battle_area[host_index as usize]
        .top_card()
        .card_id(&runner.game.card_data)
        .to_string()
}

/// Official Q&A Q1818 (same "you may" + [Once Per Turn] shape, BT9-018):
/// "If you choose not to, the effect's [Once Per Turn] requirement won't be
/// used up". CR 4.3 §15-14-1-4 counts a use when the player CHOOSES to
/// activate. Declining "this Digimon may digivolve" is choosing not to
/// activate, so a second own-security removal the same turn offers the
/// digivolve again, and accepting it digivolves. DCGO agrees
/// (`BT24_003.cs` `canNoSelect: true`: declining the OptionalSkill gate
/// never uses the ActivateClass; exam
/// `probe:BT24-003#inherited#0:optional_decline`).
#[test]
fn bt24_003_declined_trigger_is_offered_again_same_turn_q1818() {
    let mut runner = base();
    let host = runner.place_stack(0, &["BT24-003", "ST3-02"]);
    push_hand(&mut runner, "BT24-034");

    // First removal: decline.
    remove_own_security(&mut runner);
    let view = runner
        .pending_selection_view()
        .expect("the optional digivolve prompt installs");
    assert!(view.is_optional, "'may digivolve' is declinable");
    runner
        .execute_action(view.selecting_player, PASS)
        .expect("decline the first activation");
    runner.game.drain_effect_queue();
    assert_eq!(top_id(&runner, host.index), "ST3-02", "declining digivolves nothing");

    // Second removal, same turn: the [Once Per Turn] was not used up.
    remove_own_security(&mut runner);
    assert!(
        runner.pending_selection().is_some(),
        "Q1818: a declined [Once Per Turn] 'may' effect is offered again the same turn"
    );
    pick_first_candidate(&mut runner);
    assert_eq!(
        top_id(&runner, host.index),
        "BT24-034",
        "accepting the second offer digivolves into the [Shaman] hand card"
    );
}

/// §15-14-1-4: once the player chooses to activate (digivolves), the use is
/// counted, so a second own-security removal the same turn does not offer the
/// effect again even with a legal [Shaman] target in hand (BT24-014
/// Aegiochusmon, Lv.5, digivolves from a yellow Lv.4); the lockout clears
/// when the turn changes (§15-14-1-5-1).
#[test]
fn bt24_003_accepted_trigger_locks_out_second_activation_same_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT24-003 YAML parses and compiles")
        .dsl_card("ST3-02")
        .expect("ST3-02 YAML parses")
        .dsl_card("BT24-034")
        .expect("BT24-034 YAML parses")
        .dsl_card("BT24-014")
        .expect("BT24-014 YAML parses")
        .add_card(make_test_card("FILLER-DECK", "Filler"))
        .deck(0, &["FILLER-DECK"; 10])
        .deck(1, &["FILLER-DECK"; 10])
        .memory(10)
        .start();
    let host = runner.place_stack(0, &["BT24-003", "ST3-02"]);
    push_hand(&mut runner, "BT24-034");

    // First removal: accept -> digivolve into Aegiomon (BT24-034).
    remove_own_security(&mut runner);
    let view = runner
        .pending_selection_view()
        .expect("the optional digivolve prompt installs");
    let aegiomon_pick = view
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != PASS)
        .collect::<Vec<_>>();
    assert_eq!(aegiomon_pick.len(), 1, "Aegiomon is the only [Shaman] hand card");
    runner
        .execute_action(view.selecting_player, aegiomon_pick[0])
        .expect("digivolve into Aegiomon");
    runner.game.drain_effect_queue();
    assert_eq!(top_id(&runner, host.index), "BT24-034");

    // Second removal, same turn: Aegiochusmon (BT24-014) is a legal target
    // onto the Lv.4 Aegiomon, but the use is spent.
    push_hand(&mut runner, "BT24-014");
    remove_own_security(&mut runner);
    assert!(
        runner.pending_selection().is_none(),
        "an activated [Once Per Turn] effect is not offered again the same turn"
    );
    assert_eq!(top_id(&runner, host.index), "BT24-034");

    // Cycle back to player 0's turn; the lockout clears.
    runner.end_turn(); // -> P1
    runner.end_turn(); // -> P0 again
    assert_eq!(runner.game.turn_player(), 0);
    remove_own_security(&mut runner);
    assert!(
        runner.pending_selection().is_some(),
        "after the turn changes the [Once Per Turn] use resets (§15-14-1-5-1)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 5 — Integrated end-to-end: real combat breaks security
// ═══════════════════════════════════════════════════════════════════════════

/// Full integration: player 1 attacks player 0's (undefended) security with
/// a real permanent; the security break fires the OnOwnSecurityRemoved
/// observer for player 0 through the real combat path (not the direct
/// enqueue helper), and — because it is player 0's own turn by construction
/// — the digivolve prompt installs.
#[test]
fn bt24_003_integrated_attack_breaks_security_and_offers_digivolve() {
    let mut runner = base();
    let host = runner.place_stack(0, &["BT24-003", "ST3-02"]);
    {
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "BT24-034")
            .unwrap();
        let next_idx = runner.game.next_card_index();
        let card = digimon_engine::card_source::CardSource::new(data_idx, 0, next_idx);
        runner.game.players[0].hand.push(card);
    }
    runner.game.players[0].security.clear();
    {
        let sec_data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "FILLER-DECK")
            .unwrap();
        let sec_idx = runner.game.next_card_index();
        runner.game.players[0]
            .security
            .push(digimon_engine::card_source::CardSource::new(
                sec_data_idx,
                0,
                sec_idx,
            ));
    }

    // The [Your Turn] gate reads against the CONTROLLER (player 0), so the
    // trigger must fire even though it is player 1 doing the attacking —
    // "[Your Turn]" on an inherited effect means the security OWNER's turn,
    // which by construction (default turn_player = 0) is now.
    assert_eq!(runner.game.turn_player(), 0);

    // Give player 1 something to attack with (Rush not required — placed
    // with turn_played_override 0 so it's eligible to attack).
    let attacker = runner.place_on_field(1, "ATTACKER", Some(0));

    let security_before = runner.security_count(0);
    runner.attack_player(attacker, 0, true);
    runner.game.drain_effect_queue();

    assert!(
        runner.security_count(0) < security_before,
        "the attack must break player 0's security"
    );
    assert!(
        runner.pending_selection().is_some()
            || runner.game.players[0].battle_area[host.index as usize]
                .top_card()
                .card_id(&runner.game.card_data)
                == "BT24-034",
        "the OnOwnSecurityRemoved observer must fire through the real combat \
         path and offer (or, if auto-resolved by an intervening prompt \
         drain, have already offered) the digivolve prompt"
    );

    // Drive to completion: resolve whatever the combat + observer chain
    // leaves pending, declining Tsunomon's optional digivolve if offered.
    while let Some(view) = runner.pending_selection_view() {
        let action = if view.is_optional {
            PASS
        } else {
            *view
                .valid_action_ids
                .first()
                .expect("mandatory selection has a legal action")
        };
        runner
            .execute_action(view.selecting_player, action)
            .expect("resolve pending selection");
        runner.game.drain_effect_queue();
    }
}
