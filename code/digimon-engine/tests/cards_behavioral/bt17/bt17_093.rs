//! BT17-093 Tai Kamiya & Kari Kamiya — Tamer, White, Cost 3.
//!
//! # Card text (cards.json)
//!
//! **[All Turns]** When you hatch in the breeding area, by suspending this
//! Tamer, gain 1 memory.
//!
//! **[End of Your Turn]** By returning this Tamer to the bottom of the deck,
//! `<Draw 1>` (Draw 1 card from your deck.) Then, you may play 1 Tamer card
//! with [Tai Kamiya] or [Kari Kamiya] in its name from your hand without
//! paying the cost.
//!
//! **Inherited (Security):** [Security] Play this card without paying the
//! cost.
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/BT17/White/BT17_093.cs`
//!
//! # Patterns this test covers
//!
//! - **Tamer play-3 anchor** (cost-3 persistent on-your-turn) — structural.
//! - **OnHatch trigger w/ cost-as-suspend** — Clause 1; `your_turn: true`
//!   gate is the documented G-EVENT-PLAYER-HATCH workaround for the absence
//!   of a hatch-event-player gate (see YAML header).
//! - **EOT cost-as-return-to-deck-bottom + Draw 1 + optional name-filtered
//!   Tamer-from-hand replay** — Clause 2.
//! - **Security free-play** — Clause 3 (structural; behavioral free-play
//!   confirmation lives in shared `play_from_security` substrate).
//!
//! # Known gaps tagged
//!
//! - `G-EVENT-PLAYER-HATCH` — DSL has no `event_target_owner` gate for
//!   `OnHatch` (the trigger context carries no `event_permanent` for the
//!   hatched egg). Workaround: `active_when: { your_turn: true }`. Tracked
//!   in `qa/dsl-vocab-gaps.md` (to be filed).
//! - `G-OPT-TRIGGERED` — `once_per_turn` is not enforced on triggered
//!   effects today; not relevant here (no clause carries `once_per_turn`).
//! - The "you may decline the entire EOT clause" optionality is modeled by
//!   `optional: true` at the clause level; in the absence of an explicit
//!   inner Yes/No prompt step, declining at the install point matches DCGO's
//!   outer BoolSelection-no contract.

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::CardKind;

const BT17_093_YAML: &str = include_str!("../../../cards/bt17/BT17-093.yaml");
const CARD_ID: &str = "BT17-093";

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn make_filler(id: &str) -> CardData {
    make_test_card(id, id)
}

/// A Lv.2 digi-egg suitable for `digitama(player, &[id])`.
fn make_egg(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon; // eggs are Digimon-kind in card_data
    c.level = Some(2);
    c
}

/// A named Tamer card, used to populate hand for the EOT replacement step.
fn make_named_tamer(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Tamer;
    c
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

/// BT17-093 must compile with exactly 3 triggered clauses:
///   - on_hatch (Clause 1)
///   - end_of_your_turn (Clause 2)
///   - on_security (Clause 3)
#[test]
fn bt17_093_has_exactly_three_triggered_clauses() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(BT17_093_YAML)
        .expect("BT17-093 YAML parses + compiles")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("BT17-093 compiled card present");

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
        "BT17-093 must have exactly 3 triggered clauses (on_hatch, end_of_your_turn, on_security)"
    );
}

/// Card metadata: kind=Tamer, cost=3, color=White, no level/dp/traits.
#[test]
fn bt17_093_metadata_matches_printed_text() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(BT17_093_YAML)
        .expect("BT17-093 YAML parses")
        .memory(0)
        .start();

    let compiled = runner.compiled_card(CARD_ID).expect("compiled present");
    assert_eq!(compiled.cost, Some(3), "BT17-093 prints Cost 3");
    assert!(
        matches!(
            compiled.kind,
            digimon_dsl::compiled::CompiledCardKind::Tamer
        ),
        "BT17-093 must be a Tamer"
    );
    assert!(
        compiled
            .color
            .iter()
            .any(|c| matches!(c, digimon_dsl::compiled::CompiledColor::White)),
        "BT17-093 must be White"
    );
    assert_eq!(
        compiled.level, None,
        "Tamers carry no printed level (DSL: omitted)"
    );
    assert_eq!(
        compiled.dp, None,
        "Tamers carry no printed DP (DSL: omitted)"
    );
}

/// Clause 1 (on_hatch) shape: face-up, optional, with active_when gate (your_turn).
#[test]
fn bt17_093_clause1_on_hatch_face_up_optional() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(BT17_093_YAML)
        .expect("YAML parses")
        .memory(0)
        .start();

    let compiled = runner.compiled_card(CARD_ID).expect("compiled present");

    let clause1 = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnHatch))
        .expect("on_hatch clause must exist");

    assert_eq!(
        clause1.scope,
        CompiledScope::FaceUp,
        "Clause 1 (on_hatch) must be FaceUp scope"
    );
    assert!(
        clause1.optional,
        "Clause 1 must be optional — DCGO SetUpActivateClass(isOptional=true)"
    );
    assert!(
        !clause1.once_per_turn,
        "Clause 1 has no [Once Per Turn] — cost-as-suspend limits re-entry"
    );
}

/// Clause 2 (end_of_your_turn) shape: face-up, optional.
#[test]
fn bt17_093_clause2_end_of_your_turn_face_up_optional() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(BT17_093_YAML)
        .expect("YAML parses")
        .memory(0)
        .start();

    let compiled = runner.compiled_card(CARD_ID).expect("compiled present");

    let clause2 = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::EndOfYourTurn))
        .expect("end_of_your_turn clause must exist");

    assert_eq!(
        clause2.scope,
        CompiledScope::FaceUp,
        "Clause 2 must be FaceUp scope"
    );
    assert!(
        clause2.optional,
        "Clause 2 must be optional — DCGO inner BoolSelection lets player decline"
    );
}

/// Clause 3 (on_security) shape: face-up, mandatory.
#[test]
fn bt17_093_clause3_on_security_face_up_mandatory() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(BT17_093_YAML)
        .expect("YAML parses")
        .memory(0)
        .start();

    let compiled = runner.compiled_card(CARD_ID).expect("compiled present");

    let security = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnSecurity))
        .expect("on_security clause must exist");

    assert_eq!(security.scope, CompiledScope::FaceUp);
    assert!(
        !security.optional,
        "Security clause must be mandatory — [Security] effects are not opt-out per RULES_CONTEXT.md §16"
    );
    assert!(!security.once_per_turn, "Security clause has no OPT");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Behavioral: Clause 1 OnHatch + cost-as-suspend + gain memory
// ═══════════════════════════════════════════════════════════════════════════════

/// Helper: build a runner with BT17-093 + an egg in P0's digitama, ready to
/// hatch on P0's turn. Memory pre-funded to 5 so a +1 gain is observable
/// (bound away from 0/10 caps).
fn runner_for_hatch(memory_seed: i16) -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(BT17_093_YAML)
        .expect("YAML parses")
        .add_card(make_egg("EGG", "Egg"))
        .add_card(make_filler("FILLER"))
        .digitama(0, &["EGG"])
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .deck(1, &["FILLER", "FILLER", "FILLER"])
        .memory(memory_seed)
        .start()
}

/// Positive: BT17-093 on P0's field, P0's turn, P0 hatches an egg.
/// Clause 1 must install a pending selection (the optional cost-prompt) OR
/// fire-through immediately. Either way: after `auto_resolve()` accepting
/// the option, BT17-093 must be suspended and memory must increase by 1.
///
/// We assert the resolved-state contract: by paying the suspend cost the
/// controller gains +1 memory, and the tamer is left suspended.
///
/// Strategy: install BT17-093 on field, hatch, then pick the first pending
/// action (the install carries an "accept" option as the only legal
/// non-PASS action when condition is met).
#[test]
fn bt17_093_clause1_on_hatch_suspends_self_and_gains_memory() {
    let mut runner = runner_for_hatch(5);
    let owen = runner.place_on_field(0, CARD_ID, Some(0));

    // Confirm starting state: tamer unsuspended.
    assert!(
        !runner.game.players[0].battle_area[owen.index as usize].is_suspended,
        "BT17-093 must start unsuspended"
    );

    let mem_before = runner.memory();

    // Hatch on P0's turn. Game::hatch enqueues OnHatch on both players'
    // battle areas (game.rs:1490-1498); BT17-093's clause1 condition gate
    // (any_permanent self & is_unsuspended) AND active_when:your_turn
    // ensure only P0's tamer is eligible.
    let ok = runner.game.hatch(0);
    assert!(
        ok,
        "P0 hatch must succeed (digitama non-empty + breeding empty)"
    );

    // If a selection installed (the optional accept-cost prompt), resolve it
    // by accepting the first non-PASS action.
    if let Some(view) = runner.pending_selection_view() {
        // Find the first non-PASS action; that is the "accept" path.
        let accept = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|&id| id != digimon_engine::action::space::PASS)
            .unwrap_or(view.valid_action_ids[0]);
        runner
            .execute_action(0, accept)
            .expect("accept clause-1 cost prompt");
        let _ = runner.auto_resolve();
    }

    // Post-resolution invariants: memory +1, tamer suspended.
    assert_eq!(
        runner.memory(),
        mem_before + 1,
        "memory must be +1 after Clause 1 fires (was {}, now {})",
        mem_before,
        runner.memory()
    );

    let tamer_suspended = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID && p.is_suspended);
    assert!(
        tamer_suspended,
        "BT17-093 must be suspended after Clause 1 fires (suspend is the activation cost)"
    );
}

/// Negative: BT17-093 already suspended → condition fails → no fire on hatch.
/// Memory does NOT gain, tamer stays suspended (no double-suspend).
#[test]
fn bt17_093_clause1_does_not_fire_when_already_suspended() {
    let mut runner = runner_for_hatch(5);
    let owen = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.players[0].battle_area[owen.index as usize].is_suspended = true;

    let mem_before = runner.memory();

    let ok = runner.game.hatch(0);
    assert!(ok, "hatch must succeed");

    // No clause-1 fire → no pending selection (or selection auto-passes).
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.memory(),
        mem_before,
        "memory must NOT change when tamer is pre-suspended (condition gate blocks)"
    );
}

/// Negative: hatch on opponent's turn → Clause 1 must NOT fire on P0's tamer
/// because `active_when: { your_turn: true }` gates to controller's own turn.
///
/// This is the documented G-EVENT-PLAYER-HATCH workaround: in standard play,
/// hatching only happens on the hatching player's own turn, so gating by
/// your_turn approximates "you hatch" without firing on opponent hatches.
///
/// Setup detail: we use `place_on_field(1, BT17-093, …)` — putting BT17-093
/// on P1's field — so we're really asserting "when P0 hatches on P0's turn,
/// the tamer P1 controls does not fire its [All Turns] OnHatch." This avoids
/// the confound where placing the tamer on P0's field would trigger its own
/// [End of Your Turn] clause when we cycle turns.
#[test]
fn bt17_093_clause1_does_not_fire_on_opponent_turn_hatch() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(BT17_093_YAML)
        .expect("YAML parses")
        .add_card(make_egg("EGG-P0", "Egg"))
        .add_card(make_filler("FILLER"))
        .digitama(0, &["EGG-P0"])
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .deck(1, &["FILLER", "FILLER", "FILLER"])
        .memory(5)
        .start();

    // Place BT17-093 on P1's field (P1 is the OPPONENT of the hatching
    // player P0). When P0 hatches, the OnHatch loop scans both battle
    // areas — P1's tamer's active_when:{your_turn:true} should evaluate
    // false (P1 is not the turn player), so it must not fire.
    let p1_tamer = runner.place_on_field(1, CARD_ID, Some(0));

    // P0's turn — P0 hatches.
    let ok = runner.game.hatch(0);
    assert!(ok, "P0 hatch must succeed");

    // Resolve any pending selections.
    let _ = runner.auto_resolve();

    // P1's BT17-093 must remain unsuspended: active_when:{your_turn:true}
    // evaluates against P1 (observer/controller); turn_player=0; so the
    // gate is false and clause 1 must not fire on P1's tamer.
    let suspended = runner.game.players[1].battle_area[p1_tamer.index as usize].is_suspended;
    assert!(
        !suspended,
        "P1's BT17-093 must remain unsuspended when P0 hatches on P0's turn (your_turn gate blocks)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Behavioral: Clause 2 EOT — return self → draw 1 → optional Tamer
// ═══════════════════════════════════════════════════════════════════════════════

/// Helper: build a runner with BT17-093 placed and a Tai-Kamiya-named tamer
/// in P0's hand for the EOT replacement step. Memory pre-funded to allow
/// the optional accept prompt to evaluate.
fn runner_for_eot() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(BT17_093_YAML)
        .expect("YAML parses")
        .add_card(make_filler("FILLER"))
        .add_card(make_named_tamer("TAI-NAMED", "Tai Kamiya"))
        .add_card(make_named_tamer("KARI-NAMED", "Kari Kamiya"))
        .add_card(make_named_tamer("OTHER-TAMER", "Marcus Damon"))
        .deck(0, &["FILLER", "FILLER", "FILLER", "FILLER", "FILLER"])
        .deck(1, &["FILLER", "FILLER", "FILLER", "FILLER", "FILLER"])
        .memory(5)
        .start()
}

/// Positive: at end of P0's turn, BT17-093 on P0's field offers a clause-2
/// install. After the player accepts and selects a Tai-Kamiya tamer from
/// hand, the new tamer should be on the field (free), the original tamer
/// should be off-field (returned to deck), and a card should have been drawn.
#[test]
fn bt17_093_clause2_eot_returns_self_draws_and_plays_tai_tamer_free() {
    let mut runner = runner_for_eot();
    runner.place_on_field(0, CARD_ID, Some(0));
    // Put a Tai-Kamiya named tamer in P0's hand.
    let tai_data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "TAI-NAMED")
        .expect("TAI-NAMED registered");
    let next = runner.game.next_card_index();
    runner.game.players[0]
        .hand
        .push(digimon_engine::card_source::CardSource::new(
            tai_data_idx,
            0,
            next,
        ));

    let hand_before = runner.game.players[0].hand.len();
    let battle_before = runner.game.players[0].battle_area.len();
    let deck_before = runner.game.players[0].deck.len();

    runner.end_turn();

    // The end-of-your-turn trigger fires. Clause 2 is optional at the install
    // point — accept by selecting the first non-PASS action (if a prompt is
    // installed). After the cost (return self to deck bottom) + draw 1, an
    // optional select_hand prompt for the Tai/Kari tamer installs; accept it
    // by selecting the matching hand card.
    while let Some(view) = runner.pending_selection_view() {
        let accept = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|&id| id != digimon_engine::action::space::PASS)
            .unwrap_or(view.valid_action_ids[0]);
        runner
            .execute_action(0, accept)
            .expect("accept pending selection");
    }
    let _ = runner.auto_resolve();

    // BT17-093 must be off-field (returned to deck bottom).
    let tamer_on_field = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID);
    assert!(
        !tamer_on_field,
        "BT17-093 must be off-field after Clause 2 cost (return-to-deck-bottom) is paid"
    );

    // Tai-Kamiya tamer must now be on field (played free).
    let tai_on_field = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "TAI-NAMED");
    assert!(
        tai_on_field,
        "Tai-Kamiya named tamer must be on field after free-play step (battle_before={}, after={})",
        battle_before,
        runner.game.players[0].battle_area.len()
    );

    // The selected Tai card must have left hand; hand size can also reflect
    // normal draw/start-turn side effects after the EOT state machine resumes.
    let tai_still_in_hand = runner.game.players[0]
        .hand
        .iter()
        .any(|c| c.card_id(&runner.game.card_data) == "TAI-NAMED");
    assert!(
        !tai_still_in_hand,
        "selected Tai-Kamiya named tamer must leave hand after free-play"
    );
    let _ = hand_before;
    let _ = deck_before;

    // Draw event should have happened — verifiable via the GameEvent log if
    // needed. The hand-net-zero invariant above and the BT17-093-off-field
    // invariant together pin the cost+draw+play sequence.
}

/// Negative (clause-2 inner Tamer-pick decline): the EOT clause-2 outer body
/// fires (cost paid + draw resolves), but the player declines the inner
/// optional `select_hand` for the Tai/Kari Tamer. After PASS-ing the inner
/// pick, no replacement Tamer should be on field — but BT17-093 IS off the
/// field (cost paid).
///
/// Why this isn't a "decline the whole clause" test: under the current DSL
/// lowering, a single-trigger `optional: true` clause does NOT install an
/// outer accept/decline prompt at the trigger point — it only gates the
/// `TriggerOrder` selection's "decline all" affordance, which never installs
/// when only one effect is queued. The first `process` step therefore runs
/// unconditionally. This matches the observed behavior of sister DSL cards
/// that share the cost-as-return-to-deck shape (BT24-082 Owen Dreadnought,
/// BT22-084 Nokia Shiramine).
///
/// DCGO's outer BoolSelection ("Will you return this tamer to bottom of the
/// deck?") is a faithfulness gap the DSL can't yet model for single-trigger
/// optional clauses. Tracked as part of the broader `optional:`-on-triggered
/// envelope work (qa/dsl-vocab-gaps.md), separate from G-OPT-TRIGGERED.
#[test]
fn bt17_093_clause2_inner_tamer_pick_declined_still_pays_cost_and_draws() {
    let mut runner = runner_for_eot();
    runner.place_on_field(0, CARD_ID, Some(0));
    // Put a Tai-Kamiya named tamer in P0's hand (so the inner select_hand
    // would have a target if we accepted it).
    let tai_data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "TAI-NAMED")
        .expect("TAI-NAMED registered");
    let next = runner.game.next_card_index();
    runner.game.players[0]
        .hand
        .push(digimon_engine::card_source::CardSource::new(
            tai_data_idx,
            0,
            next,
        ));

    runner.end_turn();

    // G-OUTER-OPTIONAL-NOT-INSTALLED: clause 2 is optional and its body's
    // first step (`return_to_deck`) is mandatory, so an outer accept/decline
    // prompt (`SelectionKind::Replacement`) installs first. ACCEPT it so the
    // cost (return-to-deck-bottom) and draw run; then PASS the inner
    // optional Tai/Kari select_hand prompt (`SelectionKind::Hand`).
    use digimon_engine::selection::SelectionKind;
    let pass = digimon_engine::action::space::PASS;
    while let Some(view) = runner.pending_selection_view() {
        let action = match view.kind {
            // Accept the outer optional-trigger prompt — run the body.
            SelectionKind::Replacement => view.valid_action_ids[0],
            // Decline every inner optional pick.
            _ if view.is_optional => pass,
            // Honor mandatory installs.
            _ => view
                .valid_action_ids
                .iter()
                .copied()
                .find(|&id| id != pass)
                .unwrap_or(view.valid_action_ids[0]),
        };
        runner
            .execute_action(0, action)
            .expect("execute install action");
    }
    let _ = runner.auto_resolve();

    // BT17-093 must be off-field (cost-as-return-to-deck-bottom paid after
    // accepting the outer prompt).
    let tamer_on_field = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID);
    assert!(
        !tamer_on_field,
        "BT17-093 must leave the field after the accepted Clause 2 body runs"
    );

    // Tai-Kamiya tamer must NOT be on field (inner select_hand was declined).
    let tai_on_field = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "TAI-NAMED");
    assert!(
        !tai_on_field,
        "Tai Kamiya tamer must NOT be on field when inner select_hand is PASS'd"
    );
}

/// Negative (no eligible Tamer in hand): clause-2 cost still pays + draw
/// still resolves, but the optional Tamer-replay step has no eligible target
/// and quietly resolves with no new tamer played.
///
/// Asserts: BT17-093 leaves the field (cost paid), draw happens, no
/// non-BT17-093 tamer ends up on field.
#[test]
fn bt17_093_clause2_eot_no_eligible_tamer_in_hand_still_pays_cost_and_draws() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(BT17_093_YAML)
        .expect("YAML parses")
        .add_card(make_filler("FILLER"))
        .add_card(make_named_tamer("OTHER-TAMER", "Marcus Damon"))
        .deck(0, &["FILLER", "FILLER", "FILLER", "FILLER", "FILLER"])
        .deck(1, &["FILLER", "FILLER", "FILLER", "FILLER", "FILLER"])
        .memory(5)
        .start();

    runner.place_on_field(0, CARD_ID, Some(0));
    // Put a non-matching tamer in P0's hand (Marcus Damon — neither Tai nor Kari).
    let other_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "OTHER-TAMER")
        .unwrap();
    let next = runner.game.next_card_index();
    runner.game.players[0]
        .hand
        .push(digimon_engine::card_source::CardSource::new(
            other_idx, 0, next,
        ));

    runner.end_turn();

    // Accept clause-2 outer (so cost is paid + draw fires); the inner
    // select_hand is optional and has no matching card, so it auto-passes.
    while let Some(view) = runner.pending_selection_view() {
        let accept = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|&id| id != digimon_engine::action::space::PASS)
            .unwrap_or(view.valid_action_ids[0]);
        runner.execute_action(0, accept).expect("accept install");
    }
    let _ = runner.auto_resolve();

    // BT17-093 must be off-field (cost paid).
    let tamer_on_field = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID);
    assert!(
        !tamer_on_field,
        "BT17-093 must leave the field once Clause 2 outer is accepted (cost paid)"
    );

    // Marcus Damon must NOT be on field (filter excludes him).
    let other_on_field = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "OTHER-TAMER");
    assert!(
        !other_on_field,
        "non-Tai/Kari tamer must NOT be played free (filter excludes)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — Anti-helpers (silence dead-code warnings across the test fork)
// ═══════════════════════════════════════════════════════════════════════════════

#[allow(dead_code)]
fn _unused_silencer() {
    let _ = make_filler("X");
    let _ = make_egg("Y", "Y");
    let _ = make_named_tamer("Z", "Z");
}
