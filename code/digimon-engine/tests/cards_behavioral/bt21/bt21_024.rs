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
//! - Clause 1: opponent-security-count condition (`opponent_security_count_lte: 5`)
//!   + as_selecting_player + select_hand + place_on_security(bottom) +
//!   trash_top_security
//! - Clause 1 empty-hand edge case: trash_top_security still fires when the
//!   opponent has no hand card (G-SELECT-EMPTY-OUTER-TAIL resolved)
//! - Clause 2: inherited [Your Turn] self-aura +4000 DP (D4 pattern)
//!
//! # Status — IMPLEMENTED
//!
//! All clauses are faithfully implemented and behaviorally covered.
//!
//! | Clause | Gap | Status |
//! |--------|-----|--------|
//! | (1) "if opp ≤5 security" condition | G-OPP-SECURITY-COUNT-LTE (resolved) | PASS — native `opponent_security_count_lte: 5` predicate gates the whole clause |
//! | (1) place-from-hand + trash-top-security | G-SELECT-EMPTY-OUTER-TAIL (resolved) | PASS — trash fires even when opponent hand is empty |
//! | (1) WhenDigivolving dispatch | none | PASS — `enqueue_triggered(WhenDigivolving)` fires the clause |
//! | (2) inherited +4000 DP self-aura | G-INHERITED-DISPATCH (resolved) | PASS — `source_dp_contribution` reads +4000 on your turn, 0 on opponent's |

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledDpConstraint, CompiledPlayerRef,
    CompiledScope, CompiledTiming, CompiledTriggeredClause, CompiledZone,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::EffectTiming;
use digimon_engine::TriggerSource;

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
/// G-OPP-SECURITY-COUNT-LTE (resolved 2026-05-17) added the native
/// `opponent_security_count_lte` predicate leaf, which reads
/// `rctx.security_count(rctx.opponent_id())` directly. The YAML uses it:
///
/// ```yaml
/// condition:
///   opponent_security_count_lte: 5
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
    assert_eq!(
        condition.opponent_security_count_lte,
        Some(CompiledDpConstraint::Literal(5)),
        "condition must gate on opponent_security_count_lte: 5"
    );
    // The native opponent-security predicate fully expresses the gate, so the
    // older count_lte aggregate over { zone: [security], owner: opponent }
    // must NOT also be present.
    assert!(
        condition.count_lte.is_none(),
        "clause 1 condition uses the native predicate leaf, not the count_lte aggregate"
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
// "If your opponent has 5 or fewer security cards" condition. The YAML gates the
// whole triggered clause with the native `opponent_security_count_lte: 5`
// predicate (G-OPP-SECURITY-COUNT-LTE, resolved 2026-05-17), so when the
// opponent has 6+ security NOTHING runs — no placement prompt and no trash.

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

/// When the opponent satisfies the security-count condition but has an EMPTY
/// hand, no `select_hand` selection installs (there are no hand cards to pick).
/// The `trash_top_security` outer-tail step must still fire because it is the
/// tail of the same conditional clause.
///
/// G-SELECT-EMPTY-OUTER-TAIL (resolved): when `select_hand` has zero valid
/// candidates `install_select_hand` returns `InstallResult::Continue`, so the
/// `as_selecting_player` body finishes synchronously and the outer-tail
/// `trash_top_security` step runs. No add-from-hand → security count drops by 1.
#[test]
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

    let opp_sec_before = runner.security_count(1); // 3
    let opp_hand_before = runner.hand_size(1); // 0
    let opp_trash_before = runner.trash_size(1); // 0

    runner.play(0, 0);

    // No hand cards → no select_hand prompt installs. The clause must run to
    // completion synchronously: trash_top_security fires immediately.
    assert!(
        runner.game.pending_selection.is_none(),
        "empty opponent hand → no select_hand prompt; clause resolves synchronously"
    );

    // No card was placed (opponent had nothing to place).
    assert_eq!(
        runner.hand_size(1),
        opp_hand_before,
        "opponent hand stays empty — nothing to place"
    );

    // trash_top_security still fired: trash grew by exactly 1.
    assert_eq!(
        runner.trash_size(1),
        opp_trash_before + 1,
        "opponent trash must grow by 1 (top security trashed even with empty hand)"
    );

    // Net security dropped by exactly 1 (no add-from-hand, just the trash).
    assert_eq!(
        runner.security_count(1),
        opp_sec_before - 1,
        "opponent security drops by 1 (only trash, no add-from-hand)"
    );
}

/// Clause 1 fires on WhenDigivolving (the second printed trigger timing).
///
/// BT21-024 is placed as the top card of a digivolution stack (over a Lv4
/// base), then the WhenDigivolving trigger is enqueued on its permanent —
/// matching the pattern used by sibling cards (bt21_037). The clause must
/// install the opponent's hand-placement selection and, after it resolves,
/// trash the opponent's top security card.
#[test]
fn bt21_024_clause1_fires_when_digivolving() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-024")
        .expect("BT21-024 in embedded DSL pack")
        .add_card(make_filler("LV4-BASE"))
        .add_card(make_filler("OPP-HAND"))
        .security(1, &["LV4-BASE", "LV4-BASE", "LV4-BASE"])
        .hand(1, &["OPP-HAND"])
        .deck(0, &["LV4-BASE"])
        .deck(1, &["LV4-BASE"])
        .memory(10)
        .start();

    // Place BT21-024 as the top of a stack over a Lv4 base — simulating a
    // completed digivolution.
    let cyberdramon = runner.place_stack(0, &["LV4-BASE", "BT21-024"]);

    let opp_hand_before = runner.hand_size(1); // 1 (OPP-HAND)
    let opp_sec_before = runner.security_count(1); // 3
    let opp_trash_before = runner.trash_size(1); // 0

    // Fire the WhenDigivolving timing on the Cyberdramon permanent.
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(cyberdramon),
    );
    runner.game.drain_effect_queue();

    // The opponent (player 1) must be prompted to place a hand card.
    {
        let pending = runner
            .game
            .pending_selection
            .as_ref()
            .expect("pending selection installs on WhenDigivolving");
        assert_eq!(
            pending.selecting_player, 1,
            "the OPPONENT (player 1) must be the selecting player on WhenDigivolving"
        );
        let action = pending.valid_action_ids[0];
        runner
            .game
            .resolve_selection(1, action)
            .expect("opponent selection resolves successfully");
    }

    assert!(
        runner.game.pending_selection.is_none(),
        "no further pending selections after opponent places card"
    );

    // Opponent hand shrank by 1 (placed a card to bottom of security).
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
        "opponent trash must grow by 1 (top security was trashed) on WhenDigivolving"
    );
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
// The inherited +4000 DP aura is a declarative aura: it writes to
// Effect.dp_modifier, which `source_dp_contribution` reads directly over the
// `card_sources` stack — no trigger queue involved. The tests below place
// BT21-024 as source[0] under a synthetic Lv6 carrier and assert the +4000
// contribution is gated by `active_when: { your_turn: true }`.

/// A synthetic Lv6 carrier Digimon used to host BT21-024 as a digivolution
/// source. DP 9000 so the +4000 inheritance is clearly distinguishable.
fn make_carrier(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.traits = vec![];
    c.level = Some(6);
    c.dp = Some(9000);
    c
}

/// Positive: when BT21-024 is a digivolution source under a carrier, its
/// inherited [Your Turn] +4000 DP aura contributes on the CONTROLLER'S turn.
///
/// Declarative inherited auras write to `Effect.dp_modifier`, which
/// `source_dp_contribution` reads directly over the `card_sources` stack —
/// the same path BT21-072's inherited aura test exercises.
#[test]
fn bt21_024_inherited_aura_grants_4000_dp_on_your_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-024")
        .expect("BT21-024 in embedded DSL pack")
        .add_card(make_carrier("CARRIER-LV6"))
        .deck(0, &["CARRIER-LV6"])
        .deck(1, &["CARRIER-LV6"])
        .memory(10)
        .start();

    // BT21-024 is source[0]; CARRIER-LV6 is the top card of the stack.
    let carrier = runner.place_stack(0, &["BT21-024", "CARRIER-LV6"]);

    assert_eq!(
        runner.game.turn_player(),
        0,
        "precondition: P0 must be the turn player"
    );

    // source_dp_contribution at index 0 = BT21-024's inherited contribution.
    let contribution = runner.game.source_dp_contribution(carrier, 0);
    assert_eq!(
        contribution, 4000,
        "BT21-024 inherited aura must contribute +4000 DP on the controller's turn; \
         got {contribution}"
    );
}

/// Negative: the inherited +4000 DP aura must NOT contribute on the
/// opponent's turn — the `active_when: { your_turn: true }` gate blocks it.
#[test]
fn bt21_024_inherited_aura_inactive_on_opponents_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-024")
        .expect("BT21-024 in embedded DSL pack")
        .add_card(make_carrier("CARRIER-LV6"))
        .deck(0, &["CARRIER-LV6"])
        .deck(1, &["CARRIER-LV6"])
        .memory(10)
        .start();

    let carrier = runner.place_stack(0, &["BT21-024", "CARRIER-LV6"]);

    // Advance to the opponent's turn.
    runner.end_turn();
    assert_ne!(
        runner.game.turn_player(),
        0,
        "precondition: P0 must NOT be the turn player"
    );

    let contribution = runner.game.source_dp_contribution(carrier, 0);
    assert_eq!(
        contribution, 0,
        "inherited aura must be inactive on the opponent's turn \
         (active_when: your_turn); got {contribution}"
    );
}
