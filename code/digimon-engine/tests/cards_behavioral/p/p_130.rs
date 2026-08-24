//! P-130 Lui Ohwada — Tamer, Black (color 4), Cost 3, no traits.
//!
//! # Card text (cards.json)
//!
//! **[On Play]** You may move 1 of your level 3 or higher Digimon from the
//! breeding area to the battle area.
//!
//! **[Your Turn]** When one of your Digimon moves from the breeding area to
//! the battle area, by suspending this Tamer, gain 1 memory.
//!
//! **[Security]** Play this card without paying the cost.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/P/White/P_130.cs
//!
//! # Patterns this test covers
//! - **B3**: Trigger-on-event tamer (on_move: MovedFromBreeding)
//! - **E2**: Optional/cost-gating (Clause 2 — suspend as cost)
//! - **Security**: [Security] play-from-security
//! - **Structural**: 3 triggered clauses (on_play, on_move, on_security)
//!
//! # Known gaps applied
//! - **G-MOVE-BREEDING-DSL**: Resolved here with the `move_from_breeding` DSL
//!   step over `ctx.move_from_breeding_by_effect(player)`.
//! - **G-SELECT-BREEDING-FILTER**: Already resolved; `select_own_breeding_permanent`
//!   supports `filter`, which gates the level-3+ prompt.
//! - **G-OPT-TRIGGERED**: `once_per_turn` is NOT printed for Clause 2 — the
//!   suspend cost naturally limits activation. OPT tests are irrelevant here.

use digimon_dsl::compiled::{CompiledCardKind, CompiledClause, CompiledColor, CompiledTiming};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::CardKind;

const CARD_ID: &str = "P-130";

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn make_rookie(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c
}

fn make_filler(id: &str) -> CardData {
    make_test_card(id, id)
}

fn make_egg(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(2);
    c
}

/// Standard runner: P-130 on P0's field, a Lv.3 Rookie in P0's breeding area.
/// Memory set to 5 so +1 gain is observable.
fn lui_runner_with_breeding_rookie() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("P-130 YAML parses + compiles via embedded DSL pack")
        .add_card(make_rookie("ROOKIE"))
        .add_card(make_egg("EGG"))
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
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

/// P-130 must compile and surface exactly 3 triggered clauses:
/// on_play, on_move, on_security.
///
/// Note: [On Play] clause is in the YAML as a gap-documented BLOCKED clause
/// (no process steps, only a comment). It still emits a CompiledClause::Triggered
/// with OnPlay timing — structural count must be 3.
#[test]
fn p_130_has_exactly_three_triggered_clauses() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("P-130 YAML parses + compiles")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("P-130 compiled card present in registry");

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
        3,
        "P-130 must have exactly 3 triggered clauses (on_play, on_move, on_security); found {}",
        triggered.len()
    );
}

/// Card metadata must match the printed face: Tamer / Black / Cost 3.
#[test]
fn p_130_metadata_matches_printed_text() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("YAML parses")
        .memory(0)
        .start();

    let compiled = runner.compiled_card(CARD_ID).expect("compiled present");
    assert_eq!(compiled.name, "Lui Ohwada", "P-130 name must be Lui Ohwada");
    assert_eq!(compiled.cost, Some(3), "P-130 prints Cost 3");
    assert!(
        compiled
            .color
            .iter()
            .any(|c| matches!(c, CompiledColor::Black)),
        "P-130 must be Black"
    );
    assert_eq!(
        compiled.kind,
        CompiledCardKind::Tamer,
        "P-130 must be a Tamer"
    );
    assert!(compiled.traits.is_empty(), "P-130 has no printed traits");
}

/// The on_move clause must be present with CompiledTiming::OnMove, FaceUp scope,
/// not optional at clause level, and carry an active_when predicate ([Your Turn]).
#[test]
fn p_130_on_move_clause_shape() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("YAML parses")
        .memory(0)
        .start();

    let compiled = runner.compiled_card(CARD_ID).expect("compiled present");
    let on_move_clause = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnMove) => Some(t),
            _ => None,
        })
        .expect("P-130 must have an OnMove triggered clause");

    // §15-7-1 names this exact shape: "Optional processing conditions
    // include text such as 'by X, Y.'" -- and its worked example is literally
    // "[On Deletion] By trashing 1 card in the hand, gain 1 memory". §15-7-4:
    // "A player can choose whether or not to execute the content of optional
    // processing conditions." So "by suspending this Tamer, gain 1 memory" is
    // declinable, and rule 17 requires the decline to reach the action space.
    //
    // This assertion used to be inverted, on the theory that "the 'cost' is
    // the suspend, which prevents re-triggering" -- that explains why no
    // once_per_turn is printed, but it does not make the payment mandatory.
    // DCGO agrees with the manual: P_130.cs passes isOptional=true.
    assert!(
        on_move_clause.optional,
        "Clause 2 is an optional processing condition (§15-7-1/§15-7-4): the player may decline to suspend"
    );
    assert!(
        on_move_clause.active_when.is_some(),
        "P-130 [Your Turn] on_move clause must carry active_when predicate"
    );
}

/// The on_security clause must be present with CompiledTiming::OnSecurity.
#[test]
fn p_130_on_security_clause_present() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("YAML parses")
        .memory(0)
        .start();

    let compiled = runner.compiled_card(CARD_ID).expect("compiled present");
    let has_security_clause = compiled.effects.iter().any(|c| match c {
        CompiledClause::Triggered(t) => t.when.contains(&CompiledTiming::OnSecurity),
        _ => false,
    });

    assert!(
        has_security_clause,
        "P-130 must have an OnSecurity triggered clause"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — [On Play] Clause 1
// ═══════════════════════════════════════════════════════════════════════════════

/// [On Play] — positive: P-130 played with a Lv.3 Rookie in breeding.
#[test]
fn p_130_on_play_may_prompt_to_move_lv3_plus_from_breeding() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("YAML parses")
        .add_card(make_rookie("ROOKIE"))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .deck(1, &["FILLER", "FILLER", "FILLER"])
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();

    runner.place_in_breeding(0, "ROOKIE");
    let field_before = runner.battle_area_size(0);

    runner.play(0, 0).expect("play P-130");

    // Should offer a pending selection to move the breeding Digimon.
    assert!(
        runner.pending_selection().is_some(),
        "P-130 [On Play] must offer optional move of Lv.3+ breeding Digimon"
    );
    runner.auto_resolve().expect("resolve on play");

    assert_eq!(
        runner.battle_area_size(0),
        field_before + 2, // P-130 itself + moved Rookie
        "accepting should add the Rookie to battle area"
    );
    assert!(
        runner.game.players[0].breeding_area.is_none(),
        "breeding area must be empty after promotion"
    );
}

/// [On Play] — decline path: declining must leave breeding area unchanged.
#[test]
fn p_130_on_play_decline_leaves_breeding_unchanged() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("YAML parses")
        .add_card(make_rookie("ROOKIE"))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .deck(1, &["FILLER", "FILLER", "FILLER"])
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();

    runner.place_in_breeding(0, "ROOKIE");
    runner.play(0, 0).expect("play P-130");

    // Decline via PASS.
    runner
        .execute_action(0, digimon_engine::action::space::PASS)
        .expect("decline breeding move");
    runner.auto_resolve().expect("finish on play");

    assert!(
        runner.game.players[0].breeding_area.is_some(),
        "declining must leave breeding area occupied"
    );
}

/// [On Play] — no candidate: when breeding area is empty, no selection is shown.
#[test]
fn p_130_on_play_no_selection_when_breeding_empty() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("YAML parses")
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .deck(1, &["FILLER", "FILLER", "FILLER"])
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();

    // No breeding Digimon placed.
    runner.play(0, 0).expect("play P-130");

    assert!(
        runner.pending_selection().is_none(),
        "when breeding area is empty, no selection should install for P-130 On Play"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — [Your Turn] Clause 2: on_move trigger
// ═══════════════════════════════════════════════════════════════════════════════

/// Positive: when P0 has P-130 (unsuspended) on the field and a Lv.3 Rookie in
/// breeding, `move_from_breeding(0)` must fire P-130's trigger, which suspends
/// P-130 and then gains 1 memory.
#[test]
fn p_130_your_turn_trigger_suspends_self_and_gains_memory_on_move() {
    let mut runner = lui_runner_with_breeding_rookie();
    let mem_before = runner.memory();

    // Verify P-130 is unsuspended before the move.
    let field_before = runner.battle_area_size(0);
    assert!(field_before >= 1, "P-130 must be on field");

    let moved = runner.move_from_breeding(0);
    assert!(moved, "Lv.3 Rookie must move from breeding to battle area");

    // ACCEPT the optional processing condition ("by suspending this Tamer").
    // The prompt must exist -- rule 17 -- so this test drives the accept
    // branch explicitly rather than letting auto_resolve pick for it.
    accept_optional_cost(&mut runner);
    let _ = runner.auto_resolve();

    // P-130 must now be suspended (the cost was paying suspend).
    let p130_perm = runner.game.players[0]
        .battle_area
        .iter()
        .find(|perm| perm.top_card().card_id(&runner.game.card_data).eq(CARD_ID))
        .expect("P-130 must still be on field after trigger");
    assert!(
        p130_perm.is_suspended,
        "P-130 must be suspended after [Your Turn] trigger fires (suspend is the cost)"
    );

    // Memory must have increased by 1.
    assert_eq!(
        runner.memory(),
        mem_before + 1,
        "P-130 [Your Turn] trigger must gain exactly 1 memory"
    );
}

/// Negative (suspended gate): when P-130 is already suspended, the condition
/// `is_unsuspended: true` must prevent the trigger from firing (can't pay the
/// suspend cost). Memory must not increase.
#[test]
fn p_130_your_turn_trigger_does_not_fire_when_suspended() {
    let mut runner = lui_runner_with_breeding_rookie();

    // Suspend P-130 manually before the move.
    let p130_index = runner.game.players[0]
        .battle_area
        .iter()
        .position(|perm| perm.top_card().card_id(&runner.game.card_data).eq(CARD_ID))
        .expect("P-130 must be on field");
    runner.game.players[0].battle_area[p130_index].is_suspended = true;

    let mem_before = runner.memory();

    let moved = runner.move_from_breeding(0);
    assert!(moved, "Rookie must still move regardless");

    let _ = runner.auto_resolve();

    assert_eq!(
        runner.memory(),
        mem_before,
        "P-130 must NOT gain memory when already suspended (condition gate prevents firing)"
    );
}

/// Negative (opponent's turn): when it is P1's turn and P1 moves a Digimon
/// from breeding, P0's P-130 must not fire. The `active_when: { your_turn: true }`
/// gate and the dispatch's player-scoping both prevent firing.
#[test]
fn p_130_your_turn_trigger_does_not_fire_on_opponent_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("YAML parses")
        .add_card(make_rookie("ROOKIE"))
        .add_card(make_egg("EGG"))
        .add_card(make_filler("FILLER"))
        .digitama(0, &["EGG"])
        .digitama(1, &["EGG"])
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .deck(1, &["FILLER", "FILLER", "FILLER"])
        .memory(5)
        .start();

    // P-130 belongs to P0 (unsuspended).
    runner.place_on_field(0, CARD_ID, Some(0));
    // P1 has a Rookie in their breeding area.
    runner.place_in_breeding(1, "ROOKIE");

    // Cycle to P1's turn.
    runner.end_turn();

    let mem_before = runner.memory();

    // P1 moves their Rookie. The dispatch scans P1's BA (no P-130); even if
    // widened, active_when:your_turn is false for P0's P-130 on P1's turn.
    let moved = runner.move_from_breeding(1);
    assert!(moved, "P1's Rookie must move");

    let _ = runner.auto_resolve();

    assert_eq!(
        runner.memory(),
        mem_before,
        "P0's P-130 must NOT fire on P1's turn"
    );
}

/// Negative (P-130 not on field): when P-130 is not in the battle area, the
/// condition gate `any_permanent: card_number_is P-130, is_unsuspended: true`
/// evaluates false, so the trigger must not fire.
#[test]
fn p_130_your_turn_trigger_does_not_fire_when_not_on_field() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("YAML parses")
        .add_card(make_rookie("ROOKIE"))
        .add_card(make_egg("EGG"))
        .add_card(make_filler("FILLER"))
        .digitama(0, &["EGG"])
        .digitama(1, &["EGG"])
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .deck(1, &["FILLER", "FILLER", "FILLER"])
        .memory(5)
        .start();

    // Do NOT place P-130 on the field. P0 has a Rookie in breeding.
    runner.place_in_breeding(0, "ROOKIE");

    let mem_before = runner.memory();

    let moved = runner.move_from_breeding(0);
    assert!(moved, "Rookie must move even without P-130");

    let _ = runner.auto_resolve();

    assert_eq!(
        runner.memory(),
        mem_before,
        "P-130 must not gain memory when it is not on the field"
    );
}

/// Positive (event_target_owner gate): P-130 on P0's field, P0 moves their own
/// Digimon. The `event_target_owner: you` predicate confirms the moved Digimon
/// belongs to the tamer's controller (P0 = P-130's owner = P0 = move's player).
/// Memory must increase by 1.
#[test]
fn p_130_your_turn_trigger_fires_for_own_digimon_move() {
    let mut runner = lui_runner_with_breeding_rookie();
    let mem_before = runner.memory();

    let moved = runner.move_from_breeding(0);
    assert!(moved);

    let _ = runner.auto_resolve();

    assert_eq!(
        runner.memory(),
        mem_before + 1,
        "P-130 must gain 1 memory when own Digimon moves from breeding"
    );
}

/// Cost-firing state test: when the trigger fires, P-130 must be suspended
/// (the suspend is the cost, fired via `suspend: { target: source }`).
/// We verify via `is_suspended` flag rather than a GameEvent variant, since
/// no `OnSuspend` GameEvent variant exists in the current events.rs schema.
#[test]
fn p_130_trigger_suspends_tamer_as_cost() {
    let mut runner = lui_runner_with_breeding_rookie();

    // Verify P-130 starts unsuspended.
    let p130_idx = runner.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&runner.game.card_data).eq(CARD_ID))
        .expect("P-130 on field");
    assert!(
        !runner.game.players[0].battle_area[p130_idx].is_suspended,
        "P-130 must start unsuspended"
    );

    let moved = runner.move_from_breeding(0);
    assert!(moved);
    let _ = runner.auto_resolve();

    let p130_idx2 = runner.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&runner.game.card_data).eq(CARD_ID))
        .expect("P-130 still on field after trigger");
    assert!(
        runner.game.players[0].battle_area[p130_idx2].is_suspended,
        "P-130 must be suspended after [Your Turn] trigger fires (suspend is the cost)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — [Security] Clause 3
// ═══════════════════════════════════════════════════════════════════════════════

/// Security: when P-130 is in P1's security stack and P0 attacks P1, P-130
/// must trigger its [Security] effect and play itself onto P1's battle area
/// without cost.
#[test]
fn p_130_security_plays_itself_without_cost() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("YAML parses")
        .add_card(make_rookie("ATTACKER"))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .deck(1, &["FILLER", "FILLER", "FILLER"])
        .security(1, &[CARD_ID])
        .memory(10)
        .start();

    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));

    runner.attack_player(attacker, 1, false);
    let _ = runner.auto_resolve();

    // P-130 must be on P1's battle area.
    let on_field = runner.game.players[1]
        .battle_area
        .iter()
        .any(|perm| perm.top_card().card_id(&runner.game.card_data).eq(CARD_ID));
    assert!(
        on_field,
        "P-130 must be on P1's battle area after Security effect fires"
    );
    assert_eq!(
        runner.security_count(1),
        0,
        "P-130 must leave security when played by the Security effect"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 5 — Dead-code silencer
// ═══════════════════════════════════════════════════════════════════════════════

#[allow(dead_code)]
fn _unused_silencer() {
    let _ = make_filler("X");
    let _ = make_egg("Y");
    let _ = make_rookie("Z");
}

/// The accept half of P-130's optional suspend cost: take the first non-PASS
/// action on the pending prompt.
fn accept_optional_cost(runner: &mut DebugRunner) {
    let view = runner
        .pending_selection_view()
        .expect("the optional processing condition must surface a prompt (rule 17)");
    let action = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|a| *a != digimon_engine::action::space::PASS)
        .expect("the accept branch must be offered");
    runner
        .execute_action(view.selecting_player, action)
        .expect("accept the suspend cost");
}

/// §15-7-4: "A player can choose whether or not to execute the content of
/// optional processing conditions." Declining "by suspending this Tamer" must
/// leave BOTH halves undone -- the Tamer stays unsuspended and no memory is
/// gained -- because §15-7-2 says that if the optional condition's content
/// isn't executed, "the processing after the conditions can't be executed".
///
/// This is the branch the engine had no way to reach: the clause fired
/// unconditionally, so the suspend was auto-paid. Found by the DCGO
/// card-clause exam (`qa/dcgo-exams/EX12/P-130-effect0.yaml`), whose harness
/// note read "our engine auto-resolved the prompt".
#[test]
fn p_130_your_turn_trigger_may_be_declined_leaving_tamer_unsuspended() {
    let mut runner = lui_runner_with_breeding_rookie();
    let mem_before = runner.memory();

    let moved = runner.move_from_breeding(0);
    assert!(moved, "Lv.3 Rookie must move from breeding to battle area");

    let view = runner
        .pending_selection_view()
        .expect("the optional processing condition must surface a prompt (rule 17)");
    runner
        .execute_action(view.selecting_player, digimon_engine::action::space::PASS)
        .expect("declining must be reachable from the action space");
    let _ = runner.auto_resolve();

    let p130_perm = runner.game.players[0]
        .battle_area
        .iter()
        .find(|perm| perm.top_card().card_id(&runner.game.card_data).eq(CARD_ID))
        .expect("P-130 must still be on field");
    assert!(
        !p130_perm.is_suspended,
        "declining the optional cost must NOT suspend the Tamer"
    );
    assert_eq!(
        runner.memory(),
        mem_before,
        "with the optional condition declined, the processing after it can't execute"
    );
}
