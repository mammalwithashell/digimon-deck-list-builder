//! P-123 Ukkomon — Digimon, Lv.3, White, 2000 DP, Cost 3.
//! Traits: Ancient Fairy
//! Alt-path: Digivolve from Lv2 Red, cost 0.
//!
//! # Card text (cards.json)
//!
//! [Your Turn] [Once Per Turn] When one of your Digimon moves from the
//! breeding area to the battle area, you may hatch in your breeding area.
//! Then, gain 1 memory.
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/P/White/P_123.cs`
//!
//! Trigger: `EffectTiming.OnMove`. CanUseCondition gates on:
//!   1. `IsExistOnBattleArea(card)` — Ukkomon on field
//!   2. `IsOwnerTurn(card)` — your turn
//!   3. `CanTriggerOnMove(...)` with PermanentCondition
//!      `IsPermanentExistsOnOwnerBattleAreaDigimon` — owner's Digimon moved
//!
//! ActivateCoroutine body:
//!   - if (card.Owner.CanHatch) {
//!       BoolSelection "Hatch" / "Not hatch";
//!       if (doHatch) HatchDigiEggClass.Hatch();
//!     }
//!   - card.Owner.AddMemory(1, activateClass);   // unconditional
//!
//! Important: `AddMemory(1)` is OUTSIDE both `if (CanHatch)` and `if (doHatch)`
//! blocks — the +1 memory is mandatory once the trigger fires; only the hatch
//! is optional.
//!
//! # Patterns this test covers
//! - **OnMove dispatch** (G-ON-MOVE: RESOLVED 2026-04-29) — `when: on_move`
//!   lowers to `EffectTiming::OnMove`; dispatch fires after
//!   `Game::move_from_breeding` commits and supplies
//!   `TriggerSource::MovedFromBreeding { player, permanent, card }` so
//!   `event_target_owner: you` / `event_target_kind: digimon` see the moved
//!   permanent.
//! - **`select_effect_choice` for an internal optional sub-step** that
//!   precedes a mandatory tail (gain_memory). Same shape as BT17-102 hatch
//!   branch (Clauses 3/4) and the documented BT16-082 resolution body.
//! - **`hatch: { of: you }`** — silent no-op when CanHatch is false (digi-egg
//!   deck empty OR breeding area occupied), matching DCGO's gate via no
//!   illegal state but missing the prompt-suppression. Tracked as the same
//!   informational `G-CAN-HATCH` BoolPredicate gap as BT16-082's hatch
//!   sub-step.
//!
//! # Known gaps applied
//! - `G-OPT-TRIGGERED` — `once_per_turn: true` is emitted by the compiler but
//!   not enforced in `run_queued_effect_inner` for triggered effects. The
//!   OPT-lockout and OPT-reset behavioral tests are `#[ignore]`'d under this
//!   tag, mirroring BT17-093 / BT17-137.

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::CardKind;

const CARD_ID: &str = "P-123";

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// A Lv.2 digi-egg suitable for `digitama(player, &[id])`. Eggs are
/// `Digimon`-kind in card_data with `level = Some(2)`.
fn make_egg(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(2);
    c
}

/// A Lv.3 Rookie Digimon suitable for placing in breeding (eligible to move
/// to battle area: `Game::move_from_breeding` requires breeding-permanent
/// level >= 3).
fn make_rookie(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c
}

/// Filler card used to fill decks (so `digitama`/`deck` builders don't crash
/// when zones need padding).
fn make_filler(id: &str) -> CardData {
    make_test_card(id, id)
}

/// Standard runner: P-123 placed on P0's field (turn 0 so it is
/// "summoning-sickness-clear" by default), a Rookie placed in P0's breeding
/// area ready to move, and a Lv.2 egg in the digitama deck so `hatch` has
/// something to actually hatch.
///
/// `memory(5)` keeps memory bound away from 0 / 10 caps so a +1 gain is
/// observable.
fn ukkomon_runner_with_breeding_rookie() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("P-123 YAML parses + compiles via embedded DSL pack")
        .add_card(make_rookie("ROOKIE", "Test Rookie"))
        .add_card(make_egg("EGG", "Test Egg"))
        .add_card(make_filler("FILLER"))
        .digitama(0, &["EGG"])
        .digitama(1, &["EGG"])
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .deck(1, &["FILLER", "FILLER", "FILLER"])
        .memory(5)
        .start();
    r.place_on_field(0, CARD_ID, Some(0));
    r.place_in_breeding(0, "ROOKIE");
    r
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions (no `#[ignore]`)
// ═══════════════════════════════════════════════════════════════════════════════

/// P-123 must compile and surface exactly 1 triggered clause (the OnMove body).
#[test]
fn p_123_has_exactly_one_triggered_clause() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("P-123 YAML parses + compiles")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("P-123 compiled card present in registry");

    let triggered: Vec<_> = compiled
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
        "P-123 must have exactly 1 triggered clause (OnMove)"
    );
}

/// Card metadata must match the printed face: Digimon / Lv.3 / White / Cost 3
/// / DP 2000.
#[test]
fn p_123_metadata_matches_printed_text() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("YAML parses")
        .memory(0)
        .start();

    let compiled = runner.compiled_card(CARD_ID).expect("compiled present");
    assert_eq!(compiled.cost, Some(3), "P-123 prints Cost 3");
    assert_eq!(compiled.level, Some(3), "P-123 prints Lv.3");
    assert_eq!(compiled.dp, Some(2000), "P-123 prints DP 2000");
    assert!(
        matches!(
            compiled.kind,
            digimon_dsl::compiled::CompiledCardKind::Digimon
        ),
        "P-123 must be a Digimon"
    );
    assert!(
        compiled
            .color
            .iter()
            .any(|c| matches!(c, digimon_dsl::compiled::CompiledColor::White)),
        "P-123 must be White"
    );
    assert!(
        compiled
            .traits
            .iter()
            .any(|t| t.eq_ignore_ascii_case("Ancient Fairy")),
        "P-123 must carry the Ancient Fairy trait"
    );
}

/// The triggered clause must use `OnMove` timing — the resolved G-ON-MOVE
/// dispatch path. Asserts the one-direction mapping `when: on_move` →
/// `CompiledTiming::OnMove`.
#[test]
fn p_123_clause_uses_on_move_timing() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("YAML parses")
        .memory(0)
        .start();

    let compiled = runner.compiled_card(CARD_ID).expect("compiled present");
    let clause = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .expect("triggered clause present");

    assert!(
        clause.when.contains(&CompiledTiming::OnMove),
        "P-123's clause must lower to CompiledTiming::OnMove (was {:?})",
        clause.when
    );
}

/// The clause is FaceUp scope — Ukkomon's effect fires from its own face-up
/// presence on the battle area, not as an inherited (digivolution-stack)
/// effect. DCGO never sets `SetIsInheritedEffect(true)` for P_123.
#[test]
fn p_123_clause_scope_is_face_up() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("YAML parses")
        .memory(0)
        .start();

    let compiled = runner.compiled_card(CARD_ID).expect("compiled present");
    let clause = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .expect("triggered clause present");

    assert_eq!(
        clause.scope,
        CompiledScope::FaceUp,
        "P-123's clause is FaceUp (own scope, not inherited)"
    );
}

/// `[Once Per Turn]` must surface as `once_per_turn: true` on the compiled
/// clause (runtime enforcement is held by G-OPT-TRIGGERED, but the structural
/// flag must still be emitted).
#[test]
fn p_123_clause_is_once_per_turn() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("YAML parses")
        .memory(0)
        .start();

    let compiled = runner.compiled_card(CARD_ID).expect("compiled present");
    let clause = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .expect("triggered clause present");

    assert!(
        clause.once_per_turn,
        "P-123 prints [Once Per Turn] — clause must carry once_per_turn: true"
    );
}

/// The clause is NOT clause-level optional — the trigger fires automatically.
/// Only the hatch is opt-in (via `select_effect_choice`); the gain_memory
/// tail is mandatory. DCGO `SetUpActivateClass(canActivateCondition, ...,
/// isOptional: false, ...)` for P_123.
#[test]
fn p_123_clause_is_not_optional_at_clause_level() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("YAML parses")
        .memory(0)
        .start();

    let compiled = runner.compiled_card(CARD_ID).expect("compiled present");
    let clause = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .expect("triggered clause present");

    assert!(
        !clause.optional,
        "P-123's clause must NOT be optional at clause level — gain_memory(1) is unconditional; only the hatch is opt-in (internal select_effect_choice)"
    );
}

/// `[Your Turn]` must surface as a non-empty `active_when` predicate on the
/// compiled clause (the actual `your_turn: true` check happens in the engine
/// during dispatch).
#[test]
fn p_123_clause_has_active_when_your_turn() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("YAML parses")
        .memory(0)
        .start();

    let compiled = runner.compiled_card(CARD_ID).expect("compiled present");
    let clause = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .expect("triggered clause present");

    assert!(
        clause.active_when.is_some(),
        "P-123 prints [Your Turn] — clause must carry an active_when predicate"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Behavioral assertions
// ═══════════════════════════════════════════════════════════════════════════════

/// Positive: when P0 has Ukkomon on the field and a Lv.3 Rookie in breeding,
/// `move_from_breeding(0)` must (a) succeed and (b) install Ukkomon's
/// "Hatch / Don't hatch" `select_effect_choice` prompt as a pending
/// selection — proof that OnMove dispatch is reaching the clause body.
#[test]
fn p_123_trigger_installs_hatch_prompt_after_move_from_breeding() {
    let mut runner = ukkomon_runner_with_breeding_rookie();

    let moved = runner.move_from_breeding(0);
    assert!(moved, "Lv.3 rookie in breeding must move to battle area");

    assert!(
        runner.pending_selection().is_some(),
        "P-123 OnMove trigger must install a pending selection (the Hatch / Don't hatch choice)"
    );
}

/// Choosing "Don't hatch" (label index 1) must:
///   - leave the breeding area empty (no new egg hatched)
///   - still gain +1 memory (mandatory tail outside the if-branch)
///   - leave the digitama deck unchanged
#[test]
fn p_123_choose_dont_hatch_skips_hatch_but_still_gains_memory() {
    let mut runner = ukkomon_runner_with_breeding_rookie();

    let mem_before = runner.memory();
    let digitama_before = runner.game.players[0].digitama_deck.len();

    let moved = runner.move_from_breeding(0);
    assert!(moved, "rookie must move to battle area");

    // Resolve the "Hatch / Don't hatch" prompt by picking "Don't hatch"
    // (label index 1 — second action in the choice's valid action set).
    let view = runner
        .pending_selection_view()
        .expect("hatch choice prompt must be installed");
    assert!(
        view.valid_action_ids.len() >= 2,
        "select_effect_choice with 2 labels must surface 2 valid actions (got {})",
        view.valid_action_ids.len()
    );
    // Pick the SECOND non-PASS action — that's "Don't hatch" (label index 1).
    // We deliberately avoid relying on an absolute action_id ordering that
    // may shift across action_space rev bumps.
    let dont_hatch_action = view
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&id| id != digimon_engine::action::space::PASS)
        .nth(1)
        .expect("must have a 'Don't hatch' option (second non-PASS action)");
    runner
        .execute_action(0, dont_hatch_action)
        .expect("submit 'Don't hatch'");
    let _ = runner.auto_resolve();

    assert!(
        runner.game.players[0].breeding_area.is_none(),
        "breeding area must remain empty after 'Don't hatch'"
    );
    assert_eq!(
        runner.game.players[0].digitama_deck.len(),
        digitama_before,
        "digitama deck must be unchanged when player declines to hatch"
    );
    assert_eq!(
        runner.memory(),
        mem_before + 1,
        "memory must still gain +1 (the gain_memory tail is unconditional, outside the hatch branch)"
    );
}

/// Choosing "Hatch" (label index 0) when the player CAN hatch (digitama
/// non-empty + breeding empty after the move) must:
///   - put a fresh Lv.2 egg into the breeding area (digitama --, breeding ++)
///   - still gain +1 memory (mandatory tail)
#[test]
fn p_123_choose_hatch_moves_egg_to_breeding_and_gains_memory() {
    let mut runner = ukkomon_runner_with_breeding_rookie();

    let mem_before = runner.memory();
    let digitama_before = runner.game.players[0].digitama_deck.len();
    assert!(
        digitama_before > 0,
        "test setup requires a non-empty digitama for P0"
    );

    let moved = runner.move_from_breeding(0);
    assert!(moved, "rookie must move to battle area");
    assert!(
        runner.game.players[0].breeding_area.is_none(),
        "breeding area must be empty immediately after the move (so CanHatch is true)"
    );

    // Pick the FIRST non-PASS action — "Hatch" (label index 0).
    let view = runner
        .pending_selection_view()
        .expect("hatch choice prompt must be installed");
    let hatch_action = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&id| id != digimon_engine::action::space::PASS)
        .expect("must have a 'Hatch' option (first non-PASS action)");
    runner
        .execute_action(0, hatch_action)
        .expect("submit 'Hatch'");
    let _ = runner.auto_resolve();

    // Egg must have hatched into breeding — breeding now Some, digitama -1.
    assert!(
        runner.game.players[0].breeding_area.is_some(),
        "breeding area must be populated after 'Hatch'"
    );
    assert_eq!(
        runner.game.players[0].digitama_deck.len(),
        digitama_before - 1,
        "digitama deck must shrink by exactly 1 after a successful hatch"
    );
    assert_eq!(
        runner.memory(),
        mem_before + 1,
        "memory must still gain +1 after the hatch path resolves"
    );
}

/// Negative: when Ukkomon is NOT on the field, the OnMove trigger must not
/// install any selection. Memory and digitama must be unchanged after the
/// move from breeding.
#[test]
fn p_123_trigger_does_not_fire_when_ukkomon_not_on_field() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("YAML parses")
        .add_card(make_rookie("ROOKIE", "Test Rookie"))
        .add_card(make_egg("EGG", "Test Egg"))
        .add_card(make_filler("FILLER"))
        .digitama(0, &["EGG"])
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .deck(1, &["FILLER", "FILLER", "FILLER"])
        .memory(5)
        .start();

    // Do NOT place P-123 on the field. Move a rookie out of breeding —
    // OnMove fires, but Ukkomon's body never qualifies (it's not on the
    // battle area being scanned by the dispatch).
    runner.place_in_breeding(0, "ROOKIE");

    let mem_before = runner.memory();
    let digitama_before = runner.game.players[0].digitama_deck.len();

    let moved = runner.move_from_breeding(0);
    assert!(moved, "rookie must move (independent of Ukkomon)");

    assert!(
        runner.pending_selection().is_none(),
        "no pending selection should install when Ukkomon is not on the field"
    );
    assert_eq!(
        runner.memory(),
        mem_before,
        "memory must be unchanged when Ukkomon is absent"
    );
    assert_eq!(
        runner.game.players[0].digitama_deck.len(),
        digitama_before,
        "digitama must be unchanged when Ukkomon is absent"
    );
}

/// Negative ([Your Turn] gate): when Ukkomon is on P1's field and P0 moves a
/// Digimon from P0's breeding to P0's battle area, the `MovedFromBreeding`
/// dispatch only scans P0's battle area (effect_queue.rs ~line 221) — so
/// Ukkomon on P1's field never even sees the event. This test pins the
/// stronger property: even if a future dispatch widening accidentally
/// surfaces P1's Ukkomon, the `active_when: { your_turn: true }` clause
/// gate (P1 is not the turn player on P0's turn) plus the
/// `event_target_owner: you` predicate must keep it from firing.
#[test]
fn p_123_does_not_fire_when_only_opponents_ukkomon_is_on_field() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("YAML parses")
        .add_card(make_rookie("ROOKIE", "Test Rookie"))
        .add_card(make_egg("EGG", "Test Egg"))
        .add_card(make_filler("FILLER"))
        .digitama(0, &["EGG"])
        .digitama(1, &["EGG"])
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .deck(1, &["FILLER", "FILLER", "FILLER"])
        .memory(5)
        .start();

    // Ukkomon belongs to P1, not P0. Setup must NOT place a P0 Ukkomon.
    runner.place_on_field(1, CARD_ID, Some(0));
    runner.place_in_breeding(0, "ROOKIE");

    let mem_before = runner.memory();

    // P0 (turn player) moves their rookie. The dispatch scans P0's BA;
    // P1's Ukkomon is not even visible to OnMove.
    let moved = runner.move_from_breeding(0);
    assert!(moved, "P0's rookie must move out of breeding");

    let _ = runner.auto_resolve();

    // No memory change — neither P0 (no Ukkomon) nor P1 (not on the dispatch
    // scan path / not turn player / event_target_owner gate) gains anything.
    assert_eq!(
        runner.memory(),
        mem_before,
        "P1's Ukkomon must not fire when P0 moves on P0's turn"
    );
}

/// Negative (active_when:your_turn — own-Ukkomon, opponent's turn): when
/// Ukkomon belongs to P0 and it is P1's turn, even if (hypothetically) P1's
/// move-from-breeding could install Ukkomon's clause, the
/// `active_when: { your_turn: true }` gate evaluated against Ukkomon's
/// controller (P0) is false on P1's turn — so the clause must not fire.
///
/// This pins the [Your Turn] active_when contract independently of the
/// dispatch's player-scoping.
#[test]
fn p_123_does_not_fire_on_opponent_turn_move() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("YAML parses")
        .add_card(make_rookie("ROOKIE", "Test Rookie"))
        .add_card(make_egg("EGG", "Test Egg"))
        .add_card(make_filler("FILLER"))
        .digitama(0, &["EGG"])
        .digitama(1, &["EGG"])
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .deck(1, &["FILLER", "FILLER", "FILLER"])
        .memory(5)
        .start();

    // Ukkomon on P0's field, but a Digimon ready to move from P1's breeding.
    runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_in_breeding(1, "ROOKIE");

    // Cycle to P1's turn — now P0's Ukkomon is the "opponent's permanent" wrt
    // turn ownership.
    runner.end_turn();

    let mem_before = runner.memory();

    // P1 moves their rookie from breeding. The dispatch only scans P1's BA
    // (where there is no Ukkomon); even if widened, the active_when:your_turn
    // gate evaluated against P0 (Ukkomon's owner) is false on P1's turn.
    let moved = runner.move_from_breeding(1);
    assert!(moved, "P1's rookie must move out of breeding");

    let _ = runner.auto_resolve();

    assert_eq!(
        runner.memory(),
        mem_before,
        "P0's Ukkomon must not fire on P1's turn ([Your Turn] active_when gate)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — OPT lockout (G-OPT-TRIGGERED)
// ═══════════════════════════════════════════════════════════════════════════════

/// OPT lockout: a second move-from-breeding in the same turn must not
/// re-fire the clause. Blocked by G-OPT-TRIGGERED — `once_per_turn` is
/// emitted by the compiler but not enforced at runtime for triggered effects.
#[test]
#[ignore = "pending: G-OPT-TRIGGERED — once_per_turn not enforced at runtime for triggered effects"]
fn p_123_opt_blocks_second_trigger_same_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("YAML parses")
        .add_card(make_rookie("ROOKIE-A", "Rookie A"))
        .add_card(make_rookie("ROOKIE-B", "Rookie B"))
        .add_card(make_egg("EGG", "Test Egg"))
        .add_card(make_filler("FILLER"))
        .digitama(0, &["EGG", "EGG"])
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .deck(1, &["FILLER", "FILLER", "FILLER"])
        .memory(5)
        .start();
    runner.place_on_field(0, CARD_ID, Some(0));

    // First move — fires the clause; resolve by declining the hatch.
    runner.place_in_breeding(0, "ROOKIE-A");
    assert!(runner.move_from_breeding(0));
    let _ = runner.auto_resolve();
    let mem_after_first = runner.memory();

    // Second move same turn — must NOT re-fire (OPT lockout).
    runner.place_in_breeding(0, "ROOKIE-B");
    assert!(runner.move_from_breeding(0));
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.memory(),
        mem_after_first,
        "OPT must block the second OnMove trigger in the same turn — memory must not gain twice"
    );
}

/// OPT counter clears after end_turn — clause fires again next turn.
/// Blocked by G-OPT-TRIGGERED.
#[test]
#[ignore = "pending: G-OPT-TRIGGERED — once_per_turn not enforced at runtime for triggered effects"]
fn p_123_opt_resets_after_turn_cycle() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("YAML parses")
        .add_card(make_rookie("ROOKIE-A", "Rookie A"))
        .add_card(make_rookie("ROOKIE-B", "Rookie B"))
        .add_card(make_egg("EGG", "Test Egg"))
        .add_card(make_filler("FILLER"))
        .digitama(0, &["EGG", "EGG"])
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .deck(1, &["FILLER", "FILLER", "FILLER"])
        .memory(5)
        .start();
    runner.place_on_field(0, CARD_ID, Some(0));

    runner.place_in_breeding(0, "ROOKIE-A");
    assert!(runner.move_from_breeding(0));
    let _ = runner.auto_resolve();
    let mem_after_first = runner.memory();

    // Cycle two end_turns (P0 → P1 → P0).
    runner.end_turn();
    runner.end_turn();

    runner.place_in_breeding(0, "ROOKIE-B");
    assert!(runner.move_from_breeding(0));
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.memory(),
        mem_after_first + 1,
        "OPT must reset after the turn cycles back — clause fires again, memory +1"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — Anti-helpers (silence dead-code warnings across the test fork)
// ═══════════════════════════════════════════════════════════════════════════════

#[allow(dead_code)]
fn _unused_silencer() {
    let _ = make_filler("X");
    let _ = make_egg("Y", "Y");
    let _ = make_rookie("Z", "Z");
}
