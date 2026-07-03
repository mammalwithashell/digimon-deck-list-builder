//! BT7-032 Pulsemon — Digimon, Lv.3, Yellow, DP 2000, Cost 3.
//! Traits: Beastkin. Form: Rookie. Attribute: Vaccine.
//!
//! # Card text (data/card_bundles/BT7-032.md — official Bandai DB, verbatim)
//!
//! ```text
//! Inherited Effect
//! [When Attacking][Once Per Turn] If you have 3 security cards, gain 2 memory.
//! ```
//!
//! (`data/cards.json` `effect_description_eng` carries the same clause under
//! the "Effect" field instead of a separate "Inherited Effect" field, but the
//! card image (`BT7-032.webp`) confirms the printed layout puts this text in
//! the yellow "Inherited Effect" box — i.e. `scope: inherited`.)
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT7/Yellow/BT7_032.cs
//!
//! ```csharp
//! if (timing == EffectTiming.OnAllyAttack)
//! {
//!     activateClass.SetUpICardEffect("If you have 3 security cards, gain 2 memory", CanUseCondition, card);
//!     activateClass.SetUpActivateClass(CanActivateCondition, ActivateCoroutine, 1, false, EffectDiscription());
//!     activateClass.SetIsInheritedEffect(true);
//!     activateClass.SetHashString("Memory+2_BT7_032");
//!     ...
//!     bool CanUseCondition(Hashtable hashtable) =>
//!         CardEffectCommons.IsExistOnBattleArea(card) && CardEffectCommons.CanTriggerOnAttack(hashtable, card);
//!     bool CanActivateCondition(Hashtable hashtable) => CardEffectCommons.IsExistOnBattleArea(card);
//!     IEnumerator ActivateCoroutine(Hashtable hashtable)
//!     {
//!         if (card.Owner.SecurityCards.Count == 3)
//!         {
//!             yield return ... card.Owner.AddMemory(2, activateClass);
//!         }
//!         else
//!         {
//!             activateClass.RemoveUse();
//!         }
//!     }
//! }
//! ```
//!
//! ## OPT semantics cross-check (RemoveUse-on-fail)
//!
//! DCGO's turn-loop (`TurnStateMachine.cs` / `AutoProcessing.cs`) calls
//! `RegisterUseEffectThisTurn` BEFORE `ActivateCoroutine` runs — i.e. the OPT
//! slot is provisionally marked used, then `ActivateCoroutine` either (a)
//! gains 2 memory when `SecurityCards.Count == 3`, or (b) calls `RemoveUse()`
//! to UN-record that provisional mark when the count check fails. Net
//! observable behavior: the OPT is truly consumed only when memory is
//! actually gained; when the count check fails, the ability may still fire on
//! a LATER [When Attacking] trigger this turn if the security count changes.
//! This is semantically identical to the engine's standard
//! `once_per_turn: true` + clause-level `condition: { security_count: eq 3 }`
//! pattern, where `effect.condition` gates BEFORE the OPT record in
//! `effect_queue.rs::run_queued_effect_inner` — so a failed condition never
//! consumes the once-per-turn slot in the first place. Both produce the same
//! final state (OPT spent iff memory gained), matching DCGO's sibling cards
//! BT12-002 (DemiVeemon) and BT12-016 (WarGrowlmon) which use the identical
//! `condition:`-gated OPT shape for a coroutine-internal DCGO check. No
//! `refund_opt` step is needed (that primitive is for clauses whose body
//! executes non-condition side effects BEFORE the DCGO fail-check point;
//! Pulsemon's check IS the entire body, so a clause-level `condition:` is the
//! faithful and idiomatic encoding — see `docs/RUST_ENGINE_API.md` line 175
//! "activation_cost... Cost-failure consumes OPT slot" for the contrasting
//! case where OPT-on-fail-consumption DOES apply (a true activation cost),
//! which this card does not have).
//!
//! # Patterns this test covers (docs/RUST_DSL_TEST_API.md §4.3)
//! - G4 Inherited When Attacking (rookie Digimon under a later digivolution,
//!   analogous to the DigiEgg case but for a mid-stack card)
//! - E2-adjacent: OPT gated by a board-state condition (own security count),
//!   mandatory body (no player choice — "gain 2 memory" has no "you may")
//! - D-adjacent: `security_count` exact-equality condition (`security_count:
//!   { op: eq, value: 3 }` — the DSL's canonical comparator; there is no
//!   legacy flat `security_count_eq` field, only `_lte`/`_gte`)
//!
//! # No event-log test
//! `ctx.gain_memory` is a direct state mutation (`code/digimon-engine/src/
//! effect_context/action/lifecycle.rs`) — it does not emit a `GameEvent`.
//! There is no `GameEventKind::GainMemory` variant, so behavioral coverage
//! reads `runner.memory()` deltas directly instead of the event log (mirrors
//! BT12-002's zone-size-delta idiom for `Draw 1`, which also has no
//! dedicated `GameEvent`).

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};

// ─── Runner factories ─────────────────────────────────────────────────────────

/// Minimal runner: Pulsemon registered in the embedded DSL pack, a filler
/// carrier card to stack it under (own scope stays untested — the printed
/// text is inherited-only), and a filler opponent Digimon to attack.
fn base_runner(security: &[&str]) -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT7-032")
        .expect("BT7-032 YAML must be present in the embedded DSL pack")
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(make_test_card("OPP-DIGI", "Opponent Digimon"))
        .add_card(make_test_card("SEC-1", "Security Filler 1"))
        .add_card(make_test_card("SEC-2", "Security Filler 2"))
        .add_card(make_test_card("SEC-3", "Security Filler 3"))
        .add_card(make_test_card("SEC-4", "Security Filler 4"))
        .security(0, security)
        .memory(0)
        .start()
}

/// Place Pulsemon as the bottom digivolution source under CARRIER on P0's
/// field (mirrors BT12-002's `place_demivoemon_as_source` idiom via the
/// higher-level `place_stack` helper: last element = top card).
fn place_pulsemon_as_source(runner: &mut DebugRunner) -> digimon_engine::permanent::PermanentHandle {
    runner.place_stack(0, &["BT7-032", "CARRIER"])
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn bt7_032_has_exactly_one_triggered_clause() {
    let runner = base_runner(&[]);
    let compiled = runner
        .compiled_card("BT7-032")
        .expect("BT7-032 compiled card present");

    let triggered: Vec<&digimon_dsl::compiled::CompiledTriggeredClause> = compiled
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
        "BT7-032 must have exactly one triggered clause"
    );
}

#[test]
fn bt7_032_clause_is_scope_inherited() {
    let runner = base_runner(&[]);
    let compiled = runner
        .compiled_card("BT7-032")
        .expect("BT7-032 compiled card present");

    let clause = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .expect("triggered clause must exist");

    assert_eq!(
        clause.scope,
        CompiledScope::Inherited,
        "printed text is in the 'Inherited Effect' box; DCGO SetIsInheritedEffect(true)"
    );
}

#[test]
fn bt7_032_clause_when_is_when_attacking() {
    let runner = base_runner(&[]);
    let compiled = runner
        .compiled_card("BT7-032")
        .expect("BT7-032 compiled card present");

    let clause = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .expect("triggered clause must exist");

    assert!(
        clause.when.contains(&CompiledTiming::WhenAttacking),
        "BT7-032 clause must have when: WhenAttacking; got {:?}",
        clause.when
    );
}

#[test]
fn bt7_032_clause_is_once_per_turn() {
    let runner = base_runner(&[]);
    let compiled = runner
        .compiled_card("BT7-032")
        .expect("BT7-032 compiled card present");

    let clause = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .expect("triggered clause must exist");

    assert!(
        clause.once_per_turn,
        "BT7-032 clause must be once_per_turn ([Once Per Turn] in printed text)"
    );
}

#[test]
fn bt7_032_clause_is_not_optional() {
    // Printed text: "If you have 3 security cards, gain 2 memory." — no "you
    // may". DCGO: ActivateCoroutine runs AddMemory unconditionally once the
    // count check passes; no accept/decline prompt (isOptional not set /
    // SetUpActivateClass's 4th positional arg is `false`).
    let runner = base_runner(&[]);
    let compiled = runner
        .compiled_card("BT7-032")
        .expect("BT7-032 compiled card present");

    let clause = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .expect("triggered clause must exist");

    assert!(
        !clause.optional,
        "BT7-032 clause must NOT be optional — mandatory gain-2-memory, no 'you may'"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Condition gating (security_count == 3, exact equality)
// ═══════════════════════════════════════════════════════════════════════════════

/// POSITIVE: exactly 3 own security cards → inherited When Attacking fires → +2 memory.
#[test]
fn bt7_032_fires_gain_memory_when_exactly_three_security() {
    let mut runner = base_runner(&["SEC-1", "SEC-2", "SEC-3"]);
    let carrier = place_pulsemon_as_source(&mut runner);
    runner.place_on_field(1, "OPP-DIGI", Some(0));

    let memory_before = runner.memory();

    runner.attack_player(carrier, 1, false);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.memory(),
        memory_before + 2,
        "exactly 3 security cards must gain 2 memory"
    );
}

/// NEGATIVE: fewer than 3 (2) own security cards → condition fails → no memory gain.
#[test]
fn bt7_032_no_fire_with_two_security() {
    let mut runner = base_runner(&["SEC-1", "SEC-2"]);
    let carrier = place_pulsemon_as_source(&mut runner);
    runner.place_on_field(1, "OPP-DIGI", Some(0));

    let memory_before = runner.memory();

    runner.attack_player(carrier, 1, false);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.memory(),
        memory_before,
        "2 security cards (< 3) must NOT gain memory — security_count is EXACT equality, not gte"
    );
}

/// NEGATIVE: MORE than 3 (4) own security cards → condition fails → no memory
/// gain. This is the critical exact-equality assertion distinguishing
/// `security_count: { op: eq, value: 3 }` from a `gte: 3` mistake — DCGO
/// checks `Count == 3`, not `Count >= 3`.
#[test]
fn bt7_032_no_fire_with_four_security() {
    let mut runner = base_runner(&["SEC-1", "SEC-2", "SEC-3", "SEC-4"]);
    let carrier = place_pulsemon_as_source(&mut runner);
    runner.place_on_field(1, "OPP-DIGI", Some(0));

    let memory_before = runner.memory();

    runner.attack_player(carrier, 1, false);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.memory(),
        memory_before,
        "4 security cards (> 3) must NOT gain memory — must be EXACTLY 3, per DCGO Count == 3"
    );
}

/// NEGATIVE: zero own security cards → condition fails → no memory gain.
#[test]
fn bt7_032_no_fire_with_zero_security() {
    let mut runner = base_runner(&[]);
    let carrier = place_pulsemon_as_source(&mut runner);
    runner.place_on_field(1, "OPP-DIGI", Some(0));

    let memory_before = runner.memory();

    runner.attack_player(carrier, 1, false);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.memory(),
        memory_before,
        "0 security cards must NOT gain memory"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Behavioral: mandatory, no pending selection
// ═══════════════════════════════════════════════════════════════════════════════

/// The effect is fully deterministic (no player choice anywhere in the printed
/// text) — after it resolves, no pending selection should remain from this
/// clause itself.
#[test]
fn bt7_032_gain_memory_requires_no_pending_selection() {
    let mut runner = base_runner(&["SEC-1", "SEC-2", "SEC-3"]);
    let carrier = place_pulsemon_as_source(&mut runner);
    runner.place_on_field(1, "OPP-DIGI", Some(0));

    runner.attack_player(carrier, 1, false);
    let _ = runner.auto_resolve();

    assert!(
        runner.pending_selection().is_none(),
        "gain 2 memory is deterministic and must not leave a pending selection after resolve"
    );
}

/// Exact delta: gaining memory must be +2, never +1 or +4 (guards against an
/// authoring slip on the `gain_memory` amount).
#[test]
fn bt7_032_gain_memory_delta_is_exactly_two() {
    let mut runner = base_runner(&["SEC-1", "SEC-2", "SEC-3"]);
    let carrier = place_pulsemon_as_source(&mut runner);
    runner.place_on_field(1, "OPP-DIGI", Some(0));

    let memory_before = runner.memory();
    runner.attack_player(carrier, 1, false);
    let _ = runner.auto_resolve();

    let delta = runner.memory() - memory_before;
    assert_eq!(delta, 2, "gain must be exactly 2 memory; delta={delta}");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — OPT lockout
// ═══════════════════════════════════════════════════════════════════════════════

/// OPT: with the condition satisfied both times, a second attack in the SAME
/// turn must not gain memory again. Total gain across both attacks is +2, not
/// +4.
#[test]
fn bt7_032_opt_locks_out_second_attack_same_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT7-032")
        .expect("BT7-032 YAML must be present in the embedded DSL pack")
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(make_test_card("OPP-DIGI-1", "Opponent Digimon 1"))
        .add_card(make_test_card("OPP-DIGI-2", "Opponent Digimon 2"))
        .add_card(make_test_card("SEC-1", "Security Filler 1"))
        .add_card(make_test_card("SEC-2", "Security Filler 2"))
        .add_card(make_test_card("SEC-3", "Security Filler 3"))
        .add_card(make_test_card("OPP-SEC-1", "Opp Security Filler 1"))
        .add_card(make_test_card("OPP-SEC-2", "Opp Security Filler 2"))
        .add_card(make_test_card("OPP-SEC-3", "Opp Security Filler 3"))
        .add_card(make_test_card("OPP-SEC-4", "Opp Security Filler 4"))
        .security(0, &["SEC-1", "SEC-2", "SEC-3"])
        .security(1, &["OPP-SEC-1", "OPP-SEC-2", "OPP-SEC-3", "OPP-SEC-4"])
        .memory(0)
        .start();

    let carrier = runner.place_stack(0, &["BT7-032", "CARRIER"]);
    runner.place_on_field(1, "OPP-DIGI-1", Some(0));

    let memory_before = runner.memory();

    // First attack: OPT fires, +2 memory.
    runner.attack_player(carrier, 1, false);
    let _ = runner.auto_resolve();
    let memory_after_first = runner.memory();
    assert_eq!(
        memory_after_first,
        memory_before + 2,
        "first attack must fire the OPT gain (security still == 3)"
    );

    if runner.game_over() {
        return; // opponent security exhausted; OPT test cannot proceed further
    }

    // Second attack same turn: security count is still 3 (attacking a
    // Digimon, not the player's security stack, so P0's own security is
    // unaffected), but OPT must block a repeat.
    let carrier_again = runner.perm_handle(0, 0);
    runner.attack_player(carrier_again, 1, false);
    let _ = runner.auto_resolve();
    let memory_after_second = runner.memory();

    assert_eq!(
        memory_after_second, memory_after_first,
        "[Once Per Turn] must block the second WhenAttacking gain in the same turn"
    );
}

/// OPT clears after end_turn round-trip: attack on the next turn (still with
/// exactly 3 security) fires again.
#[test]
fn bt7_032_opt_clears_after_end_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT7-032")
        .expect("BT7-032 YAML must be present in the embedded DSL pack")
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(make_test_card("OPP-DIGI", "Opponent Digimon"))
        .add_card(make_test_card("SEC-1", "Security Filler 1"))
        .add_card(make_test_card("SEC-2", "Security Filler 2"))
        .add_card(make_test_card("SEC-3", "Security Filler 3"))
        .add_card(make_test_card("OPP-SEC-1", "Opp Security Filler 1"))
        .add_card(make_test_card("OPP-SEC-2", "Opp Security Filler 2"))
        .add_card(make_test_card("OPP-SEC-3", "Opp Security Filler 3"))
        .add_card(make_test_card("OPP-SEC-4", "Opp Security Filler 4"))
        .add_card(make_test_card("OPP-SEC-5", "Opp Security Filler 5"))
        .security(0, &["SEC-1", "SEC-2", "SEC-3"])
        .security(1, &["OPP-SEC-1", "OPP-SEC-2", "OPP-SEC-3", "OPP-SEC-4", "OPP-SEC-5"])
        .memory(0)
        .start();

    let carrier = runner.place_stack(0, &["BT7-032", "CARRIER"]);
    runner.place_on_field(1, "OPP-DIGI", Some(0));

    // Turn 1 attack: OPT fires.
    let memory_t1_before = runner.memory();
    runner.attack_player(carrier, 1, false);
    let _ = runner.auto_resolve();
    let memory_t1_after = runner.memory();
    assert_eq!(
        memory_t1_after,
        memory_t1_before + 2,
        "turn 1 attack must fire the OPT gain"
    );

    if runner.game_over() {
        return;
    }

    // End P0's turn, advance to P1's turn, then back to P0's turn.
    runner.end_turn(); // P1's turn begins
    runner.end_turn(); // P0's turn begins again

    if runner.game_over() {
        return;
    }

    let battle_area_len = runner.game.players[0].battle_area.len();
    if battle_area_len == 0 {
        // Carrier didn't survive — OPT-clears assertion cannot proceed further.
        return;
    }
    let carrier2 = runner.perm_handle(0, 0);

    // Security count for P0 is unaffected by attacking (attacks target P1's
    // security), so it is still exactly 3 on turn 2.
    let memory_t2_before = runner.memory();
    runner.attack_player(carrier2, 1, false);
    let _ = runner.auto_resolve();
    let memory_t2_after = runner.memory();

    assert_eq!(
        memory_t2_after,
        memory_t2_before + 2,
        "OPT must clear after end_turn: turn 2 attack must fire again"
    );
}
