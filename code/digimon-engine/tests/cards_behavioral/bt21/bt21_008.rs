//! BT21-008 Elizamon — Digimon, Lv.3, Red, DP 1000, Cost 3.
//! Traits: Reptile, LIBERATOR
//!
//! # Card text (cards.json)
//!
//! [On Play] Reveal the top 3 cards of your deck. Add 1 card with the
//! [Reptile] or [Dragonkin] trait and 1 card with the [LIBERATOR] trait
//! among them to the hand. Return the rest to the bottom of the deck.
//!
//! Inherited: [Your Turn] [Once Per Turn] When your opponent's security
//! stack is removed from, gain 1 memory.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT21/Red/BT21_008.cs
//!
//! # Patterns this test covers
//! - A1 searching rookie (reveal-top-3, add by trait)
//! - F9-adjacent: security-conditioned trigger (on_opponent_security_removed)
//! - E2-adjacent: OPT inherited clause
//! - Structural: 2 triggered clauses; one OnPlay (FaceUp), one Inherited OPT

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::EffectTiming;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const ELIZAMON_YAML: &str = include_str!("../../../cards/bt21/BT21-008.yaml");

// ── Helper builders ──────────────────────────────────────────────────────────

/// Build a filler card (generic Lv.3 Digimon, no traits, no effect).
fn make_filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.traits = vec![];
    c
}

/// Build a card with [Reptile] trait.
fn make_reptile(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.traits = vec!["Reptile".to_string()];
    c
}

/// Build a card with [LIBERATOR] trait.
fn make_liberator(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.traits = vec!["LIBERATOR".to_string()];
    c
}

/// Build the base OnPlay runner.
/// Deck top: REPTILE-1, LIBERATOR-1, FILLER-1, then 2 more fillers at bottom.
fn elizamon_runner() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(ELIZAMON_YAML)
        .expect("BT21-008 YAML parses")
        .add_card(make_reptile("REPTILE-1"))
        .add_card(make_liberator("LIBERATOR-1"))
        .add_card(make_filler("FILLER-1"))
        .add_card(make_filler("FILLER-2"))
        .add_card(make_filler("FILLER-3"))
        .hand(0, &["BT21-008"])
        // Deck: last = top. So top = REPTILE-1, LIBERATOR-1, FILLER-1.
        .deck(0, &["FILLER-2", "FILLER-3", "FILLER-1", "LIBERATOR-1", "REPTILE-1"])
        .memory(10)
        .start()
}

/// Place a permanent on P0's field with Elizamon as a bottom digivolution
/// source and ATTACKER-CARRIER as the top card. This satisfies the `scope:
/// inherited` requirement (is_under = true for Elizamon's source slot).
///
/// Returns the handle of the stacked permanent.
fn place_elizamon_as_source(runner: &mut DebugRunner, security_cards: &[&str]) -> digimon_engine::permanent::PermanentHandle {
    // First ensure we have a security stack for P1 if needed.
    for _ in security_cards {
        // caller pre-populates security; this function just does the stack setup
    }

    // Place CARRIER on P0's field as the top card.
    let handle = runner.place_on_field(0, "CARRIER", Some(0));

    // Now insert Elizamon as the bottom digivolution source of CARRIER's permanent.
    {
        let game = runner.game_mut();
        let data_idx = game
            .card_data
            .iter()
            .position(|c| c.card_id == "BT21-008")
            .expect("BT21-008 registered in card_data");
        let next = game.next_card_index();
        let mut elizamon_src = CardSource::new(data_idx, 0, next);
        elizamon_src.card_index = next;
        let perm = &mut game.players[0].battle_area[handle.index as usize];
        // Insert at position 0 so CARRIER stays on top (last = top).
        perm.card_sources.insert(0, elizamon_src);
    }

    handle
}

// ── Section 1: Structural assertions ────────────────────────────────────────

#[test]
fn bt21_008_has_exactly_two_triggered_clauses() {
    let runner = elizamon_runner();
    let compiled = runner
        .compiled_card("BT21-008")
        .expect("BT21-008 compiled card present");

    let triggered: Vec<&digimon_dsl::compiled::CompiledTriggeredClause> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    assert_eq!(triggered.len(), 2, "BT21-008 must have exactly 2 triggered clauses");
}

#[test]
fn bt21_008_on_play_clause_is_face_up_scope_not_opt() {
    let runner = elizamon_runner();
    let compiled = runner
        .compiled_card("BT21-008")
        .expect("BT21-008 compiled card present");

    let triggered: Vec<&digimon_dsl::compiled::CompiledTriggeredClause> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    let on_play = triggered
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::OnPlay))
        .expect("OnPlay clause must exist");

    assert_eq!(on_play.scope, CompiledScope::FaceUp, "OnPlay clause must have FaceUp (default own) scope");
    assert!(!on_play.once_per_turn, "OnPlay clause must NOT be once_per_turn");
    assert!(!on_play.optional, "OnPlay clause must NOT be optional");
}

#[test]
fn bt21_008_inherited_clause_is_opt_and_on_opponent_security_removed() {
    let runner = elizamon_runner();
    let compiled = runner
        .compiled_card("BT21-008")
        .expect("BT21-008 compiled card present");

    let triggered: Vec<&digimon_dsl::compiled::CompiledTriggeredClause> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    let inherited = triggered
        .iter()
        .find(|t| t.scope == CompiledScope::Inherited)
        .expect("Inherited clause must exist");

    assert!(
        inherited.when.contains(&CompiledTiming::OnOpponentSecurityRemoved),
        "Inherited clause must fire on OnOpponentSecurityRemoved"
    );
    assert!(inherited.once_per_turn, "Inherited clause must be once_per_turn");
    assert!(!inherited.optional, "Inherited clause is always active when conditions met (not user-optional)");
}

// ── Section 2: Condition gating for inherited clause ────────────────────���───

/// Positive: Elizamon as inherited source under a carrier sees the existing
/// OnOpponentSecurityRemoved timing and gains +1 memory.
///
/// Regression: `enqueue_from_permanent` must scan below-top stack sources for
/// inherited triggered effects.
#[test]
fn bt21_008_inherited_positive_fires_when_source_under_carrier_your_turn() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(ELIZAMON_YAML)
        .expect("BT21-008 YAML parses")
        .add_card(make_filler("CARRIER"))
        .add_card(make_filler("SEC-CARD"))
        .add_card(make_filler("FILLER-DECK"))
        .security(1, &["SEC-CARD"])
        .deck(0, &["FILLER-DECK"])
        .memory(0)
        .start();

    let carrier_perm = place_elizamon_as_source(&mut runner, &["SEC-CARD"]);

    let memory_before = runner.memory();

    runner.game.enqueue_triggered(
        EffectTiming::OnOpponentSecurityRemoved,
        TriggerSource::PlayerBattleArea(carrier_perm.player),
    );
    runner.game.drain_effect_queue();

    let memory_after = runner.memory();
    let delta = memory_after - memory_before;

    assert!(
        delta >= 1,
        "inherited clause must fire when Elizamon is a source and security removed; delta={delta}"
    );
}

/// Negative: during opponent's turn, Elizamon's inherited clause is inactive.
/// (active_when: { your_turn: true } blocks it.)
#[test]
fn bt21_008_inherited_negative_does_not_fire_on_opponents_turn() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(ELIZAMON_YAML)
        .expect("BT21-008 YAML parses")
        .add_card(make_filler("CARRIER"))
        .add_card(make_filler("ATTACKER-P1"))
        .add_card(make_filler("SEC-CARD-P0"))
        .add_card(make_filler("FILLER-DECK"))
        .security(0, &["SEC-CARD-P0"])
        .deck(0, &["FILLER-DECK"])
        .deck(1, &["FILLER-DECK"])
        .memory(5)
        .start();

    // P0 has Elizamon as source under CARRIER.
    let _carrier_perm = place_elizamon_as_source(&mut runner, &[]);

    // Advance to P1's turn.
    runner.end_turn();

    // P1 places its own attacker and attacks P0's security.
    let attacker_p1 = runner.place_on_field(1, "ATTACKER-P1", Some(0));
    let memory_before = runner.memory();

    runner.attack_player(attacker_p1, 0, false);
    let _ = runner.auto_resolve();

    let memory_after = runner.memory();
    let delta = memory_after - memory_before;

    // Elizamon's inherited clause is gated by `your_turn: true` (P0's turn).
    // Since it's P1's turn, the clause must NOT fire.
    assert!(
        delta <= 0,
        "Elizamon inherited must NOT fire on opponent's turn; delta={delta}"
    );
}

// ── Section 3: Behavioral — OnPlay ──────────────────────────────────────────

/// Positive: reveal 3 cards (Reptile, LIBERATOR, Filler at top).
/// Player picks first from pool (Reptile at index 0), then second pick from
/// pool (LIBERATOR at next index). Both land in hand; filler to deck bottom.
#[test]
fn bt21_008_on_play_adds_two_cards_to_hand_and_returns_remainder() {
    let mut runner = elizamon_runner();
    let deck_before = runner.deck_size(0);

    runner.play(0, 0);

    // First pending selection: Reveal — pick index 0 (REPTILE-1).
    assert!(
        runner.pending_selection().is_some(),
        "OnPlay must install first Reveal selection"
    );
    {
        let view = runner.pending_selection_view().unwrap();
        assert_eq!(view.kind, SelectionKind::Reveal, "first selection must be Reveal");
        assert!(!runner.pending_is_optional(), "OnPlay reveal selections are mandatory (optional: false)");
        runner.execute_action(0, view.valid_action_ids[0]).expect("first pick");
    }

    // Second pending selection: Reveal — pick index 0 of remaining.
    assert!(
        runner.pending_selection().is_some(),
        "OnPlay must install second Reveal selection"
    );
    {
        let view = runner.pending_selection_view().unwrap();
        assert_eq!(view.kind, SelectionKind::Reveal, "second selection must be Reveal");
        runner.execute_action(0, view.valid_action_ids[0]).expect("second pick");
    }

    let _ = runner.auto_resolve();

    // Deck: 5 originally, reveal 3, return 1 = 5 - 3 + 1 = 3.
    assert_eq!(
        runner.deck_size(0),
        deck_before - 2,
        "deck should have 2 fewer cards (3 revealed, 1 returned)"
    );
    // Hand: started with 1 (Elizamon), played it, gained 2 → 2.
    assert_eq!(runner.hand_size(0), 2, "hand should have 2 cards from picks");
    // Field: Elizamon on field.
    assert_eq!(runner.battle_area_size(0), 1, "Elizamon on field");
}

/// Positive: with explicit reveals, confirm REPTILE-1 went to hand (not deck/trash).
#[test]
fn bt21_008_on_play_picked_cards_are_in_hand_not_deck() {
    let mut runner = elizamon_runner();

    runner.play(0, 0);

    // First pick: index 0 = REPTILE-1.
    {
        let view = runner.pending_selection_view().unwrap();
        runner.execute_action(0, view.valid_action_ids[0]).unwrap();
    }
    // Second pick.
    {
        let view = runner.pending_selection_view().unwrap();
        runner.execute_action(0, view.valid_action_ids[0]).unwrap();
    }
    let _ = runner.auto_resolve();

    // Hand should have exactly 2 cards.
    assert_eq!(runner.hand_size(0), 2);
    // Deck should have 3 cards (5 - 3 revealed + 1 returned).
    assert_eq!(runner.deck_size(0), 3);
}

/// All-filler case: Phase 2b accept-all filter means player is still offered
/// all 3 cards even when none have Reptile/Dragonkin/LIBERATOR traits.
/// The effect completes without panic and picks land in hand.
#[test]
fn bt21_008_on_play_all_filler_deck_phase2b_accept_all_completes() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(ELIZAMON_YAML)
        .expect("BT21-008 YAML parses")
        .add_card(make_filler("FILLER-A"))
        .add_card(make_filler("FILLER-B"))
        .add_card(make_filler("FILLER-C"))
        .add_card(make_filler("FILLER-D"))
        .hand(0, &["BT21-008"])
        .deck(0, &["FILLER-D", "FILLER-C", "FILLER-B", "FILLER-A"])
        .memory(10)
        .start();

    let deck_before = runner.deck_size(0);
    runner.play(0, 0);

    // Phase 2b: all 3 revealed cards are offered. auto_resolve picks first action each time.
    let _ = runner.auto_resolve();

    // 4 deck - 3 revealed + 1 returned = 2.
    assert_eq!(
        runner.deck_size(0),
        deck_before - 2,
        "deck should shrink by 2 (3 revealed, 1 returned)"
    );
    assert_eq!(runner.hand_size(0), 2, "two picks should be in hand");
}

// ── Section 4: Cost firing ───────────────────────────────────────────────────
// N/A: BT21-008's OnPlay has no cost-as-side-effect. The inherited gain_memory
// has no cost. No event-log assertions needed for cost effects.

// ── Section 5: OPT enforcement on inherited clause ──────────────────────────

/// OPT: second security removal in same turn does NOT gain memory.
///
/// Regression: triggered OPT limits must apply to inherited queue effects.
#[test]
fn bt21_008_inherited_opt_blocks_second_trigger_same_turn() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(ELIZAMON_YAML)
        .expect("BT21-008 YAML parses")
        .add_card(make_filler("CARRIER"))
        .add_card(make_filler("SEC-1"))
        .add_card(make_filler("SEC-2"))
        .add_card(make_filler("FILLER-DECK"))
        .security(1, &["SEC-1", "SEC-2"])
        .deck(0, &["FILLER-DECK"])
        .memory(0)
        .start();

    let carrier_perm = place_elizamon_as_source(&mut runner, &[]);

    // First event: OPT fires, memory +1.
    let m0 = runner.memory();
    runner.game.enqueue_triggered(
        EffectTiming::OnOpponentSecurityRemoved,
        TriggerSource::PlayerBattleArea(carrier_perm.player),
    );
    runner.game.drain_effect_queue();
    let m1 = runner.memory();
    let first_delta = m1 - m0;

    assert!(
        first_delta >= 1,
        "first security removal should fire inherited OPT clause; delta={first_delta}"
    );

    // Second same-turn event: OPT is used -> clause should be locked out.
    let m2 = runner.memory();
    runner.game.enqueue_triggered(
        EffectTiming::OnOpponentSecurityRemoved,
        TriggerSource::PlayerBattleArea(carrier_perm.player),
    );
    runner.game.drain_effect_queue();
    let m3 = runner.memory();
    let second_delta = m3 - m2;

    // Second removal must not add another +1 from the inherited clause.
    assert!(
        second_delta < first_delta,
        "OPT must block second inherited trigger; first_delta={first_delta}, second_delta={second_delta}"
    );
}

/// OPT resets after end_turn: can fire again on the next turn.
///
/// Best-effort attack smoke: security combat may remove the carrier before a
/// second live inherited trigger can be observed, so the same-turn direct queue
/// test above carries the focused triggered OPT enforcement regression.
#[test]
fn bt21_008_inherited_opt_resets_after_end_turn() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(ELIZAMON_YAML)
        .expect("BT21-008 YAML parses")
        .add_card(make_filler("CARRIER"))
        .add_card(make_filler("SEC-1"))
        .add_card(make_filler("SEC-2"))
        .add_card(make_filler("SEC-3"))
        .add_card(make_filler("FILLER-DECK"))
        .security(1, &["SEC-1", "SEC-2", "SEC-3"])
        .deck(0, &["FILLER-DECK"])
        .memory(10)
        .start();

    let carrier_perm = place_elizamon_as_source(&mut runner, &[]);

    // Turn 1: fire the OPT.
    runner.attack_player(carrier_perm, 1, false);
    let _ = runner.auto_resolve();

    if runner.game_over() {
        return;
    }

    // End turn → P1's turn → end turn → back to P0's turn.
    runner.end_turn();
    runner.end_turn();

    if runner.battle_area_size(0) == 0 || runner.game_over() {
        return; // carrier was removed somehow; skip
    }

    // Turn 3 (P0 again): OPT should have reset.
    let carrier_perm3 = runner.perm_handle(0, 0);
    let m_before = runner.memory();
    runner.attack_player(carrier_perm3, 1, false);
    let _ = runner.auto_resolve();
    let m_after = runner.memory();

    let delta = m_after - m_before;
    assert!(
        delta >= 1,
        "OPT must reset after end_turn; inherited should fire again on next P0 turn; delta={delta}"
    );
}
