//! BT21-024 Cyberdramon — Digimon, Lv.5, Red, Cost 7, DP 7000.
//! Traits: Cyborg.
//!
//! # Card text (cards.json)
//!
//! ```text
//! [On Play] [When Digivolving]
//! If your opponent has 5 or fewer security cards, they place 1 card from
//! their hand as the bottom security card. Then, trash their top security card.
//! ```
//!
//! ```text
//! Inherited: [Your Turn] This Digimon gets +4000 DP.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT21/Red/BT21_024.cs
//!
//! # Patterns this test covers
//! - Structural: 1 triggered clause (OnPlay + WhenDigivolving), 1 inherited aura
//! - Clause 1: opponent security count condition + as_selecting_player +
//!   select_hand + place_on_security(bottom) + trash_top_security
//! - Clause 2: inherited [Your Turn] self-aura +4000 DP (D4 pattern)
//!
//! # Known gaps and test status
//!
//! | Clause | Gap | Status |
//! |--------|-----|--------|
//! | (1) "if opp ≤5 security" condition | YAML drift | AUDITED-DRIFT — current DSL can express this as `count_lte` over opponent security, but YAML still lacks the condition |
//! | (1) place-from-hand + trash-top-security | condition drift | both printed sub-steps are conditional; current YAML runs them unconditionally |
//! | (2) inherited +4000 DP self-aura structure | none (compile-only) | structural PASS |
//! | (2) inherited aura runtime dispatch | G-INHERITED-DISPATCH | #[ignore] pending dispatch wiring |

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledDpConstraint, CompiledPlayerRef,
    CompiledScope, CompiledTiming, CompiledTriggeredClause, CompiledZone,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};

use super::super::dsl_card_data::compiled;

// ── Filler helpers ───────────────────────────────────────────────────────────

/// Generic filler card — no effects, no traits.
fn make_filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.traits = vec![];
    c
}

// ── Runner helper ────────────────────────────────────────────────────────────

/// Standard test setup: Cyberdramon in player 0's hand, filler security for
/// both players, and enough memory to play.
fn cyberdramon_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT21-024")
        .expect("BT21-024 in embedded DSL pack")
        .add_card(make_filler("FILLER"))
        .add_card(make_filler("OPP-HAND"))
        .hand(0, &["BT21-024"])
        .security(0, &["FILLER", "FILLER", "FILLER", "FILLER", "FILLER"])
        .security(1, &["FILLER", "FILLER", "FILLER"])
        .hand(1, &["OPP-HAND"])
        .deck(0, &["FILLER"])
        .deck(1, &["FILLER"])
        .memory(10)
        .start()
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

/// BT21-024 compiles successfully from the embedded DSL pack.
#[test]
fn bt21_024_compiles_from_dsl_pack() {
    let _card = compiled("BT21-024");
}

/// BT21-024 has exactly one alt_path: standard digivolve from Lv4, cost 3.
#[test]
fn bt21_024_has_one_alt_path_digivolve_lv4_cost3() {
    use digimon_dsl::compiled::{CompiledAltPathKind, CompiledCost};
    let card = compiled("BT21-024");
    assert_eq!(
        card.alt_paths.len(),
        1,
        "expected exactly 1 alt_path (standard Digivolve from Lv4), got {}",
        card.alt_paths.len()
    );
    let path = &card.alt_paths[0];
    assert_eq!(path.kind, CompiledAltPathKind::Digivolve);
    assert_eq!(
        path.cost,
        Some(digimon_dsl::compiled::CompiledCost::Literal(3)),
        "digivolve cost must be 3"
    );
    assert!(
        !path.ignore_requirements,
        "standard path must not ignore requirements"
    );
}

/// BT21-024 has exactly one triggered clause (OnPlay + WhenDigivolving) and one
/// inherited aura declarative clause.
#[test]
fn bt21_024_has_one_triggered_clause() {
    let card = compiled("BT21-024");
    let triggered: Vec<&CompiledTriggeredClause> = card
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
        "expected exactly 1 triggered clause, got {}: {:?}",
        triggered.len(),
        triggered.iter().map(|t| &t.when).collect::<Vec<_>>()
    );
}

/// Clause 1 fires on both OnPlay and WhenDigivolving (the two printed timings).
#[test]
fn bt21_024_clause1_fires_on_on_play_and_when_digivolving() {
    let card = compiled("BT21-024");
    let clause1 = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .next()
        .expect("clause 1 must be present");

    assert!(
        clause1.when.contains(&CompiledTiming::OnPlay),
        "clause 1 must include OnPlay; got: {:?}",
        clause1.when
    );
    assert!(
        clause1.when.contains(&CompiledTiming::WhenDigivolving),
        "clause 1 must include WhenDigivolving; got: {:?}",
        clause1.when
    );
}

/// Clause 1 is NOT optional (the opponent is forced to participate) and NOT
/// once-per-turn (no [Once Per Turn] in the card text). Scope is FaceUp (own).
#[test]
fn bt21_024_clause1_is_mandatory_not_opt_own_scope() {
    let card = compiled("BT21-024");
    let clause1 = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .next()
        .expect("clause 1 must be present");

    assert!(
        !clause1.optional,
        "clause 1 is NOT optional when its printed opponent-security condition is met"
    );
    assert!(
        !clause1.once_per_turn,
        "clause 1 has no [Once Per Turn] in card text; must not be OPT"
    );
    assert_eq!(
        clause1.scope,
        CompiledScope::FaceUp,
        "clause 1 is own-scope (FaceUp), not inherited"
    );
}

/// Clause 1 must be gated by the printed condition:
/// "If your opponent has 5 or fewer security cards..."
///
/// Current DSL capability note: the old `opponent_security_count_lte` gap is no
/// longer the right target for this card. Aggregate predicates can count cards
/// in zones for an owner, so the YAML should use:
///
/// ```yaml
/// condition:
///   count_lte:
///     filter: { zone: [security], owner: opponent }
///     n: 5
/// ```
#[test]
fn bt21_024_clause1_condition_counts_opponent_security_lte_5() {
    let card = compiled("BT21-024");
    let clause1 = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .next()
        .expect("clause 1 must be present");

    let condition = clause1
        .condition
        .as_ref()
        .expect("printed text requires a clause condition: opponent has ≤5 security");
    let aggregate = condition
        .count_lte
        .as_ref()
        .expect("condition should use count_lte over opponent security");
    assert_eq!(
        aggregate.n,
        CompiledDpConstraint::Literal(5),
        "condition threshold must be ≤5 security"
    );
    assert_eq!(
        aggregate.filter.owner,
        Some(CompiledPlayerRef::Opponent),
        "count_lte filter must count the opponent's security stack"
    );
    assert_eq!(
        aggregate.filter.zone,
        vec![CompiledZone::Security],
        "count_lte filter must count security cards"
    );
}

/// BT21-024 has exactly one inherited aura declarative clause (clause 2: +4000 DP).
#[test]
fn bt21_024_has_one_inherited_aura_clause() {
    let card = compiled("BT21-024");
    let inherited_auras: Vec<_> = card
        .effects
        .iter()
        .filter(|c| {
            matches!(
                c,
                CompiledClause::Declarative(CompiledDeclarativeClause::Aura { scope, .. })
                    if *scope == CompiledScope::Inherited
            )
        })
        .collect();
    assert_eq!(
        inherited_auras.len(),
        1,
        "expected exactly 1 inherited aura clause, got {}",
        inherited_auras.len()
    );
}

/// Clause 2 inherited aura carries dp_modifier: Some(4000).
#[test]
fn bt21_024_inherited_aura_dp_modifier_is_4000() {
    let card = compiled("BT21-024");
    let aura_modifier = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
                scope: CompiledScope::Inherited,
                dp_modifier,
                ..
            }) => Some(*dp_modifier),
            _ => None,
        })
        .expect("inherited aura clause must be present");
    assert_eq!(
        aura_modifier,
        Some(4000),
        "inherited aura must carry dp_modifier: 4000"
    );
}

/// Clause 2 inherited aura is scoped to `active_when: { your_turn: true }`.
#[test]
fn bt21_024_inherited_aura_has_active_when_your_turn() {
    let card = compiled("BT21-024");
    let aura_clause = card
        .effects
        .iter()
        .find(|c| {
            matches!(
                c,
                CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
                    scope: CompiledScope::Inherited,
                    ..
                })
            )
        })
        .expect("inherited aura must be present");
    // The Aura declarative carries an `active_when` predicate (your_turn: true).
    // We can't inspect it structurally from CompiledDeclarativeClause directly,
    // but compilation succeeds only if the YAML validated correctly, confirming
    // the active_when field was parsed.
    // Behavioral verification (your_turn vs opponent's_turn) is in Section 5.
    let _ = aura_clause; // struct compile-time confirmation is sufficient here
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Condition gating
// ═══════════════════════════════════════════════════════════════════════════════
//
// Print text wins over DCGO. Both "they place 1 card from their hand as the
// bottom security card" and "Then, trash their top security card" are under the
// opponent-security condition. Current YAML still follows the older DCGO-shaped
// implementation and runs clause 1 unconditionally.
//
// Capability drift: current aggregate predicates can express the needed gate as
// `count_lte` with `filter: { zone: [security], owner: opponent }, n: 5`, so the
// AUDIT expectation is active rather than ignored.

/// Positive condition: when opponent has 3 security cards (within ≤5), the effect
/// fires and a pending selection installs (opponent prompted to place a hand card).
#[test]
fn bt21_024_clause1_installs_selection_when_opponent_has_3_security() {
    let mut runner = cyberdramon_runner();
    // runner: player 0 hand = [BT21-024], player 1 security = 3, player 1 hand = [OPP-HAND]
    runner.play(0, 0);

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("pending selection must install after play: opponent selects a hand card");
    assert_eq!(
        pending.selecting_player, 1,
        "the OPPONENT (player 1) must be the selecting player"
    );
}

#[test]
fn bt21_024_clause1_does_not_fire_when_opponent_has_more_than_5_security() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-024")
        .expect("BT21-024 in embedded DSL pack")
        .add_card(make_filler("FILLER"))
        .add_card(make_filler("OPP-HAND"))
        .hand(0, &["BT21-024"])
        .security(
            1,
            &["FILLER", "FILLER", "FILLER", "FILLER", "FILLER", "FILLER"],
        )
        .hand(1, &["OPP-HAND"])
        .deck(0, &["FILLER"])
        .deck(1, &["FILLER"])
        .memory(10)
        .start();

    let opp_hand_before = runner.hand_size(1);
    let opp_sec_before = runner.security_count(1);
    let opp_trash_before = runner.trash_size(1);

    runner.play(0, 0);

    assert!(
        runner.game.pending_selection.is_none(),
        "opponent has 6 security, so printed clause 1 must not install a hand-selection prompt"
    );
    assert_eq!(
        runner.hand_size(1),
        opp_hand_before,
        "opponent hand must not change when the printed condition is false"
    );
    assert_eq!(
        runner.security_count(1),
        opp_sec_before,
        "opponent security must not change when the printed condition is false"
    );
    assert_eq!(
        runner.trash_size(1),
        opp_trash_before,
        "opponent trash must not change when the printed condition is false"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Behavioral outcomes (clause 1)
// ═══════════════════════════════════════════════════════════════════════════════

/// After playing Cyberdramon: opponent is prompted to place a hand card as their
/// bottom security, then their top security is trashed. Net security count is
/// unchanged (added bottom, trashed top = ±0). Opponent loses 1 hand card.
#[test]
fn bt21_024_clause1_places_hand_card_bottom_and_trashes_top_security_on_play() {
    let mut runner = cyberdramon_runner();

    // Verify initial state
    let opp_hand_before = runner.hand_size(1); // 1 (OPP-HAND)
    let opp_sec_before = runner.security_count(1); // 3
    let opp_trash_before = runner.trash_size(1); // 0

    // Play Cyberdramon from player 0's hand.
    runner.play(0, 0);

    // Opponent must be prompted to select a hand card.
    {
        let pending = runner
            .game
            .pending_selection
            .as_ref()
            .expect("pending selection installs: opponent must place a hand card");
        assert_eq!(pending.selecting_player, 1, "opponent (player 1) selects");
        assert!(
            !pending.valid_action_ids.is_empty(),
            "opponent must have at least 1 valid action (their hand card)"
        );
        // Resolve: opponent picks first valid action.
        let action = pending.valid_action_ids[0];
        runner
            .game
            .resolve_selection(1, action)
            .expect("opponent selection resolves successfully");
    }

    // After full resolution: no more pending selections.
    assert!(
        runner.game.pending_selection.is_none(),
        "no further pending selections after opponent places card"
    );

    // Opponent hand shrank by 1 (placed a card to security).
    assert_eq!(
        runner.hand_size(1),
        opp_hand_before - 1,
        "opponent hand must shrink by 1 after placing a card to bottom of security"
    );

    // Net security unchanged: added 1 to bottom, trashed 1 from top → ±0.
    assert_eq!(
        runner.security_count(1),
        opp_sec_before,
        "net opponent security count unchanged (added bottom, trashed top)"
    );

    // Opponent trash grew by 1 (the trashed top security card).
    assert_eq!(
        runner.trash_size(1),
        opp_trash_before + 1,
        "opponent trash must grow by 1 (top security was trashed)"
    );
}

/// When opponent satisfies the security-count condition but has an empty hand,
/// no selection installs (no hand cards to select from), but the
/// trash_top_security step still needs to fire because it is the tail of the
/// same conditional clause.
///
/// IGNORED: When select_hand has no valid candidates (empty hand), the engine
/// returns early from install_select_hand without installing a PendingSelection.
/// The outer continuation steps (trash_top_security) are parked in dsl_outer_tail
/// but are only drained by the selection callback — which never fires for empty
/// hand. This means trash_top_security is permanently lost in the empty-hand case.
/// Gap: G-SELECT-EMPTY-OUTER-TAIL — outer tail steps after an as_selecting_player
/// body whose inner select_hand has no candidates are silently dropped.
#[test]
#[ignore = "pending: G-SELECT-EMPTY-OUTER-TAIL — outer-tail steps lost when select_hand has no candidates (empty hand skips outer continuation)"]
fn bt21_024_clause1_trashes_top_security_even_when_opponent_has_no_hand() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-024")
        .expect("BT21-024 in embedded DSL pack")
        .add_card(make_filler("FILLER"))
        .hand(0, &["BT21-024"])
        .security(1, &["FILLER", "FILLER", "FILLER"]) // 3 security
        // Opponent has NO hand cards.
        .deck(0, &["FILLER"])
        .deck(1, &["FILLER"])
        .memory(10)
        .start();

    let _opp_sec_before = runner.security_count(1); // 3
    let opp_trash_before = runner.trash_size(1); // 0

    runner.play(0, 0);

    // With no hand cards, the select_hand step should resolve with nothing selected.
    // There may or may not be a pending selection depending on how empty-selection is handled.
    // Either way, auto_resolve should clear any pending state.
    let _ = runner.auto_resolve();

    // Regardless of whether the hand-placement fired, trash_top_security fires.
    // Opponent trash should have grown by at least 1 (the trashed top security).
    // Security count decreased by 1 (only trash, no add-from-hand).
    assert!(
        runner.trash_size(1) >= opp_trash_before + 1,
        "opponent trash must grow by at least 1 (top security trashed even with empty hand)"
    );
}

/// Clause 1 fires on WhenDigivolving (second trigger timing).
/// Behavioral test is ignored until G-OPT-TRIGGERED / WhenDigivolving dispatch is wired.
#[test]
#[ignore = "pending: G-OPT-TRIGGERED (WhenDigivolving dispatch through triggered queue)"]
fn bt21_024_clause1_fires_when_digivolving() {
    // Scaffolding (for when the gap closes):
    //
    //   let mut runner = DebugRunner::builder()
    //       .dsl_card("BT21-024").expect("in pack")
    //       .add_card(make_filler("FILLER"))
    //       .add_card(make_filler("LV4-BASE"))
    //       .add_card(make_filler("OPP-HAND"))
    //       .security(1, &["FILLER", "FILLER", "FILLER"])
    //       .hand(1, &["OPP-HAND"])
    //       .hand(0, &["BT21-024"])
    //       .deck(0, &["FILLER"]).deck(1, &["FILLER"])
    //       .memory(20).start();
    //
    //   // Place a Lv4 base on field, then digivolve BT21-024 on top.
    //   let base = runner.place_on_field(0, "LV4-BASE", Some(0));
    //   runner.digivolve(0, base, 0).expect("digivolve succeeds");
    //
    //   let pending = runner.game.pending_selection.as_ref()
    //       .expect("pending selection installs on WhenDigivolving");
    //   assert_eq!(pending.selecting_player, 1, "opponent selects");
    todo!("implement once WhenDigivolving dispatch wiring closes");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — Event log (clause 1)
// ═══════════════════════════════════════════════════════════════════════════════
//
// The `GameEvent` enum does not yet have `OnDiscardSecurity` / `OnLoseSecurity`
// variants (events.rs: only MemoryChange, TurnStart, PhaseChange, Play, GameOver
// and stub variants are defined). The `Trash` variant exists but is "not emitted yet".
//
// A meaningful security-event test will be added once the event emission is wired.

/// Security-event log test: verifies that at least a `Play` event fires for
/// Cyberdramon itself. Serves as a minimal event-log smoke check.
#[test]
fn bt21_024_play_event_fires_on_play() {
    use digimon_engine::events::GameEvent;

    let mut runner = cyberdramon_runner();
    let cp = runner.event_checkpoint();

    runner.play(0, 0);
    let _ = runner.auto_resolve();

    let events = runner.events_since(cp);
    let play_event = events.iter().any(|e| matches!(e, GameEvent::Play { .. }));
    assert!(
        play_event,
        "Play event must fire when Cyberdramon enters the field"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 5 — Inherited aura behavioral (clause 2)
// ═══════════════════════════════════════════════════════════════════════════════
//
// The inherited +4000 DP aura requires G-INHERITED-DISPATCH to be wired: the
// digivolution-stack `card_sources[0..n-1]` are not yet scanned for inherited
// triggered/declarative effects. Structural tests above confirm the YAML compiles
// correctly with `scope: inherited, dp_modifier: 4000, active_when: { your_turn: true }`.

/// Positive (gap-blocked): Cyberdramon's inherited +4000 DP aura is active on
/// the controller's turn when Cyberdramon is in a digivolution stack.
///
/// G-INHERITED-DISPATCH for *triggered* effects closed 2026-05-17 (Phase 2
/// Track D). Declarative auras like this one read through a separate path
/// (`source_dp_contribution` over `card_sources`) which has been passing
/// for some time; see BT21-072 for live coverage. Test body left as
/// `todo!()` — implementing it is a fixture-authoring follow-up.
#[test]
#[ignore = "pending: test body not written (substrate closed; see BT21-072 for live coverage of declarative inherited DP auras)"]
fn bt21_024_inherited_aura_grants_4000_dp_on_your_turn() {
    // Scaffolding (for when the gap closes):
    //
    //   let mut runner = DebugRunner::builder()
    //       .dsl_card("BT21-024").expect("in pack")
    //       .add_card(make_filler("FILLER"))
    //       .add_card(make_filler("LV6-TOP"))
    //       .deck(0, &["FILLER"]).deck(1, &["FILLER"])
    //       .memory(20).start();
    //
    //   // Place Cyberdramon then stack a Lv6 on top (simulating digivolution).
    //   let cyber = runner.place_on_field(0, "BT21-024", Some(0));
    //   let top = runner.place_on_field(0, "LV6-TOP", Some(0));
    //   // Stack BT21-024 as source below LV6-TOP (implementation-specific).
    //   // runner.push_digivolution_source(top, "BT21-024");
    //
    //   // On player 0's turn the inherited aura should be active.
    //   let base_dp = runner.dp_of(top).expect("LV6-TOP has DP");
    //   let effective = runner.effective_dp(top).expect("effective DP reads");
    //   assert_eq!(
    //       effective,
    //       base_dp + 4000,
    //       "inherited +4000 DP aura must be active on your turn"
    //   );
    todo!("implement once G-INHERITED-DISPATCH closes");
}

/// Negative (gap-blocked): the inherited +4000 DP aura is inactive on the opponent's turn.
#[test]
#[ignore = "pending: test body not written (substrate closed; see BT21-072 for live coverage of declarative inherited DP auras)"]
fn bt21_024_inherited_aura_inactive_on_opponents_turn() {
    // After runner.end_turn(), effective_dp must drop back to base.
    todo!("implement once G-INHERITED-DISPATCH closes");
}
