//! BT17-015 WarGreymon — Digimon, Lv.6, Red, DP 11000, Cost 11.
//! Traits: Dragonkin
//! Evo: Lv.5 Red / cost 3
//!
//! # Card text (cards.json — verbatim)
//!
//! When this card would be played, if you have a Tamer with [Tai Kamiya] in
//! its name, reduce the play cost by 3.
//! [On Play] [When Digivolving] Activate 1 of the effects below:
//!   ・Delete 1 of your opponent's Digimon with 8000 DP or less.
//!   ・1 of your [Gabumon] may digivolve into [MetalGarurumon] in your hand,
//!     ignoring its digivolution requirements and without paying the cost.
//!
//! Inherited Effect [When Attacking] [Once Per Turn]
//!   If this Digimon has [Omnimon] in its name, trash the top card of your
//!   opponent's security stack.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT17/Red/BT17_015.cs
//!
//! # Patterns covered
//! - D2 BeforePayCost cost_reduction with `when_playing_this: true` and an
//!   `any_permanent { kind: tamer, name_contains: "Tai Kamiya" }` condition.
//! - E1 Branch-choice [On Play][When Digivolving] (2-way EffectChoice).
//! - F1 Delete opp digimon with `dp_lte: 8000` filter (delete branch).
//! - F2 Effect-initiated DNA-free digivolve from hand (digivolve branch).
//! - G4 Inherited [When Attacking][Once Per Turn] gated on top-card name
//!   ("Omnimon"-name) trashing top opp security.
//!
//! # Spotlight reference
//! `docs/RUST_DSL_TEST_API.md` §5 names BT17-015 as a worked example for
//! branch-choice + cost reduction. (The §9 "D1 Conditional DP buff" mention
//! is a doc cross-wire — BT17-015 has no DP buff; the test below tracks the
//! actual printed text.)
//!
//! # Faithfulness diff vs. card text
//!
//! | Card-text element                                                   | YAML clause                                                              | Status         |
//! |---------------------------------------------------------------------|--------------------------------------------------------------------------|----------------|
//! | "When this card would be played, if Tai Kamiya tamer, -3 cost"      | `cost_reduction { when_playing_this: true, condition: any_permanent { tamer, name_contains: "Tai Kamiya" }, amount: 3 }` | OK             |
//! | "[On Play][When Digivolving] choose 1: …"                            | `when: [on_play, when_digivolving]` + `select_effect_choice { 2 labels }` | OK             |
//! | "Delete 1 opp Digimon (≤8000 DP)"                                   | `select_opponent_permanent { kind: digimon, dp_lte: 8000 }` + `delete_permanent` | OK             |
//! | "1 of your [Gabumon] may digivolve into [MetalGarurumon] in hand …" | `select_own_permanent { name_contains: "Gabumon" }` + `select_hand { name_contains: "MetalGarurumon" }` + `effect_initiated_digivolve { ignore_requirements, cost: 0 }` | OK *           |
//! | Inherited "[When Attacking][OPT] If [Omnimon]-name … trash top sec" | `scope: inherited`, `when: when_attacking`, `once_per_turn: true`, `condition: { source_name_contains: "Omnimon" }`, `trash_top_security { of: opponent }` | DSL-GAP — see below |
//!
//! ## DSL-GAP — `source_name_contains` predicate not evaluated  [G-DSL-SOURCE-NAME-CONTAINS]
//!
//! `source_name_contains` parses (`digimon-dsl/src/predicate.rs:101`) and
//! compiles through (`digimon-dsl/src/compile.rs:472`), but the engine
//! predicate evaluator (`code/digimon-engine/src/dsl_cards/predicate.rs`)
//! never inspects it; it appears only as an "unsupported leaf" sentinel for
//! card-zone filters in `formula_eval.rs:799`. For a triggered clause's
//! `condition: { source_name_contains: "Omnimon" }` evaluated via
//! `lower_triggered.rs:97` with `PredicateSubject::None`, the predicate
//! degenerates to `true`. Net effect: the inherited When Attacking clause
//! fires regardless of the top card's name — a faithfulness break of the
//! "[Omnimon]-name" gate.
//!
//! This is the same shape as `G-SELF-DIGIVOLUTION-CONTAINS-NAME` in
//! `qa/archetype-qa/engine-gaps.md` (BT20-102) — the engine has
//! `Permanent::contains_card_name`; the gap is predicate evaluator
//! coverage + subject threading from `lower_triggered`.
//!
//! Tests below split:
//!   - The positive case (`bt17_015_inherited_when_attacking_omnimon_top_trashes_security`)
//!     should pass either way and is the regression that the wiring fires.
//!   - The negative case (`bt17_015_inherited_when_attacking_wargreymon_top_does_not_trash_security`)
//!     is `#[ignore]`'d on the gap — under the gap it incorrectly trashes
//!     security, so asserting "no trash" would fail until the gap is closed.
//!
//! ## Other known gaps relevant to this card
//! - **G-OPT-TRIGGERED** (qa/archetype-qa/engine-gaps.md:169) — fixed for
//!   permanent-backed inherited triggered effects on 2026-04-29; the same-turn
//!   OPT lockout test below passes. The cross-turn reset via a fresh attack
//!   does NOT re-fire (see G-OPT-RESET-VIA-ATTACK-CYCLE below) — distinct
//!   from the queue-level reset that already passes via bt21_008.
//! - **G-DSL-SOURCE-NAME-CONTAINS** (newly logged here) — predicate parsed
//!   and compiled but evaluator never inspects it. Affects every YAML clause
//!   that uses `source_name_contains: <name>` to gate behavior on the source
//!   permanent's top-card name. Same shape as G-SELF-DIGIVOLUTION-CONTAINS-NAME
//!   (engine-gaps.md:381) — the engine has `Permanent::contains_card_name`;
//!   the gap is evaluator coverage in
//!   `code/digimon-engine/src/dsl_cards/predicate.rs` plus subject threading
//!   in `lower_triggered.rs` for triggered-clause `condition:` blocks.
//! - **G-EFFECT-INITIATED-DIGIVOLVE-FROM-HAND-WITH-PERMANENT-TARGET** (newly
//!   logged here) — the chain `select_own_permanent { bind_as: base } →
//!   select_hand { bind_as: evo } → effect_initiated_digivolve { target: base,
//!   from_hand: evo }` terminates after the permanent pick; the hand-pick
//!   prompt never installs and the digivolve verb never executes. The
//!   sibling `target: self` form (BT21-013) works, so the gap is the
//!   interaction between binding-targeted `effect_initiated_digivolve` and
//!   the chained `select_hand` step. Affects branch 1 of BT17-015's
//!   [On Play][When Digivolving] clause.
//! - **G-OPT-RESET-VIA-ATTACK-CYCLE** (newly observed here) — inherited
//!   [When Attacking][OPT] does not re-fire on a fresh attack after a full
//!   P0→P1→P0 turn cycle, even though the queue-level reset works (see
//!   bt21_008). May be activation-counter persistence or attacker-side
//!   reset; needs distinct triage from G-OPT-TRIGGERED.
//! - **No-approximations Tamer-name gate** (faithfulness micro-issue) —
//!   `name_contains: "Tai Kamiya"` matches both "Tai Kamiya" and any Tamer
//!   with "Tai Kamiya" as a substring (e.g. dual Tamers like
//!   "Tai Kamiya & Matt Ishida"). DCGO intentionally also matches the
//!   no-space variant `"TaiKamiya"` (see `BT17_015.cs:62`); current YAML
//!   only matches the spaced form. Not test-asserted (would need a Tamer
//!   with the no-space form to discriminate).

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
    CompiledTriggeredClause,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, PlayerId};
use digimon_engine::events::GameEvent;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

// ─── Fixture helpers ────────────────────────────────────────────────────────

fn make_tamer(id: &str, name: &str) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Tamer;
    card
}

fn make_lv4_filler(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(5000);
    card
}

fn make_opp_digimon(id: &str, name: &str, dp: i32) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Digimon;
    card.level = Some(5);
    card.dp = Some(dp);
    card
}

fn make_gabumon(id: &str) -> CardData {
    let mut card = make_test_card(id, "Gabumon");
    card.card_kind = CardKind::Digimon;
    card.level = Some(3);
    card.dp = Some(3000);
    card
}

fn make_metalgarurumon(id: &str) -> CardData {
    let mut card = make_test_card(id, "MetalGarurumon");
    card.card_kind = CardKind::Digimon;
    card.level = Some(6);
    card.dp = Some(11000);
    card.play_cost = 12;
    card
}

fn make_omnimon_top(id: &str) -> CardData {
    // Top-card stand-in whose name contains "Omnimon" — used to ride above a
    // BT17-015 source so the inherited [When Attacking] clause's name gate
    // (printed text) sees an Omnimon-named top card.
    let mut card = make_test_card(id, "Omnimon");
    card.card_kind = CardKind::Digimon;
    card.level = Some(7);
    card.dp = Some(13000);
    card
}

fn make_filler_digimon(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(6);
    card.dp = Some(11000);
    card
}

// ─── Section 1 — Structural assertions ──────────────────────────────────────

#[test]
fn bt17_015_compiles_with_two_triggered_and_one_cost_reduction_declarative() {
    let runner = DebugRunner::builder()
        .dsl_card("BT17-015")
        .expect("BT17-015 in embedded DSL pack")
        .memory(0)
        .start();
    let compiled = runner.compiled_card("BT17-015").expect("compiled");

    let triggered = compiled
        .effects
        .iter()
        .filter(|c| matches!(c, CompiledClause::Triggered(_)))
        .count();
    let cost_reduction = compiled
        .effects
        .iter()
        .filter(|c| {
            matches!(
                c,
                CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction { .. })
            )
        })
        .count();

    assert_eq!(triggered, 2, "expected 2 triggered clauses (OnPlay/WhenDigi + inherited WhenAttacking)");
    assert_eq!(cost_reduction, 1, "expected 1 cost_reduction declarative");
    assert_eq!(
        compiled.effects.len(),
        3,
        "total compiled clauses must be 3 (1 cost_reduction + 2 triggered)"
    );
}

#[test]
fn bt17_015_has_on_play_when_digivolving_clause_face_up_mandatory() {
    let runner = DebugRunner::builder()
        .dsl_card("BT17-015")
        .expect("BT17-015 in embedded DSL pack")
        .memory(0)
        .start();
    let compiled = runner.compiled_card("BT17-015").expect("compiled");

    let clause: &CompiledTriggeredClause = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnPlay)
                    && t.when.contains(&CompiledTiming::WhenDigivolving) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("must have [On Play][When Digivolving] clause");

    assert_eq!(clause.scope, CompiledScope::FaceUp);
    assert!(!clause.optional, "branch-choice itself is mandatory once triggered");
    assert!(!clause.once_per_turn, "no [Once Per Turn] on the branch-choice clause");
}

#[test]
fn bt17_015_has_inherited_when_attacking_opt_clause() {
    let runner = DebugRunner::builder()
        .dsl_card("BT17-015")
        .expect("BT17-015 in embedded DSL pack")
        .memory(0)
        .start();
    let compiled = runner.compiled_card("BT17-015").expect("compiled");

    let clause: &CompiledTriggeredClause = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if matches!(t.scope, CompiledScope::Inherited)
                    && t.when.contains(&CompiledTiming::WhenAttacking) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("must have inherited [When Attacking] clause");

    assert!(clause.once_per_turn, "[When Attacking] inherited must be once_per_turn");
    assert_eq!(clause.scope, CompiledScope::Inherited);
}

#[test]
fn bt17_015_has_lv5_cost3_digivolve_alt_path() {
    let runner = DebugRunner::builder()
        .dsl_card("BT17-015")
        .expect("BT17-015 in embedded DSL pack")
        .memory(0)
        .start();
    let compiled = runner.compiled_card("BT17-015").expect("compiled");

    let standard = compiled.alt_paths.iter().find(|p| {
        p.kind == digimon_dsl::compiled::CompiledAltPathKind::Digivolve
            && matches!(
                p.cost,
                Some(digimon_dsl::compiled::CompiledCost::Literal(3))
            )
    });
    assert!(standard.is_some(), "must have Lv.5 / cost 3 digivolve alt-path");
}

// ─── Section 2 — Cost-reduction condition gating (split positive + negative) ─

/// Negative: with NO Tai Kamiya tamer on field, full 11 cost is paid.
#[test]
fn bt17_015_cost_reduction_blocked_without_tai_kamiya_tamer() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT17-015")
        .expect("BT17-015 in embedded DSL pack")
        .add_card(make_opp_digimon("OPP-DIGI", "OppDigi", 5000))
        .hand(0, &["BT17-015"])
        .memory(15)
        .start();

    // Place an opp digimon so the [On Play] branch-choice has a legal target —
    // play_from_hand should still succeed even without one, but having a target
    // keeps the play path identical between this and the positive test.
    runner.place_on_field(1, "OPP-DIGI", None);

    let mem_before = runner.memory();
    runner.play(0, 0);
    let cost = mem_before - runner.memory();

    assert_eq!(
        cost, 11,
        "with no Tai Kamiya tamer, full printed cost (11) should be paid; got {cost}"
    );
}

/// Positive: with a Tai Kamiya tamer on field, cost is reduced by 3 → 8.
#[test]
fn bt17_015_cost_reduction_applies_with_tai_kamiya_tamer() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT17-015")
        .expect("BT17-015 in embedded DSL pack")
        .add_card(make_tamer("TAI-TAMER", "Tai Kamiya"))
        .add_card(make_opp_digimon("OPP-DIGI", "OppDigi", 5000))
        .hand(0, &["BT17-015"])
        .memory(15)
        .start();

    runner.place_on_field(0, "TAI-TAMER", None);
    runner.place_on_field(1, "OPP-DIGI", None);

    let mem_before = runner.memory();
    runner.play(0, 0);
    let cost = mem_before - runner.memory();

    assert_eq!(
        cost, 8,
        "with a Tai Kamiya tamer, cost should be 11-3=8; got {cost}"
    );
}

/// Negative: an unrelated Tamer on field must NOT trigger the reduction —
/// guards against `name_contains` false positives.
#[test]
fn bt17_015_cost_reduction_blocked_by_unrelated_tamer() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT17-015")
        .expect("BT17-015 in embedded DSL pack")
        .add_card(make_tamer("OTHER-TAMER", "Matt Ishida"))
        .add_card(make_opp_digimon("OPP-DIGI", "OppDigi", 5000))
        .hand(0, &["BT17-015"])
        .memory(15)
        .start();

    runner.place_on_field(0, "OTHER-TAMER", None);
    runner.place_on_field(1, "OPP-DIGI", None);

    let mem_before = runner.memory();
    runner.play(0, 0);
    let cost = mem_before - runner.memory();

    assert_eq!(
        cost, 11,
        "Matt Ishida tamer must NOT count for [Tai Kamiya] gate; got cost {cost}"
    );
}

/// Cost-reduction must not leak to other cards being played from hand while
/// only BT17-015 has the rebate clause.
#[test]
fn bt17_015_cost_reduction_does_not_leak_to_other_plays() {
    let mut other = make_test_card("OTHER-DIGI", "OtherDigi");
    other.card_kind = CardKind::Digimon;
    other.play_cost = 8;
    other.colors = vec![digimon_engine::enums::CardColor::Red];
    other.level = Some(5);
    other.dp = Some(7000);

    let mut runner = DebugRunner::builder()
        .dsl_card("BT17-015")
        .expect("BT17-015 in embedded DSL pack")
        .add_card(make_tamer("TAI-TAMER", "Tai Kamiya"))
        .add_card(other)
        .hand(0, &["OTHER-DIGI"])
        .memory(15)
        .start();

    runner.place_on_field(0, "TAI-TAMER", None);

    let mem_before = runner.memory();
    runner.play(0, 0);
    let cost = mem_before - runner.memory();

    assert_eq!(
        cost, 8,
        "Tai Kamiya rebate must not leak to OTHER-DIGI; expected full 8, got {cost}"
    );
}

// ─── Section 3 — [On Play][When Digivolving] branch-choice behavioral ───────

fn play_and_install_branch_choice(
    runner: &mut DebugRunner,
    hand_idx: usize,
) -> SelectionKind {
    runner.play(0, hand_idx).expect("BT17-015 plays");
    runner
        .pending_kind()
        .expect("EffectChoice prompt must install on OnPlay")
}

/// Branch 0 (Delete): selecting branch 0 then a single ≤8000-DP opp Digimon
/// removes that Digimon from the opponent's battle area.
#[test]
fn bt17_015_on_play_branch_0_deletes_opp_digimon_dp_lte_8000() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT17-015")
        .expect("BT17-015 in embedded DSL pack")
        .add_card(make_opp_digimon("OPP-7K", "OppLow7K", 7000))
        .hand(0, &["BT17-015"])
        .memory(15)
        .start();

    runner.place_on_field(1, "OPP-7K", None);
    let opp_before = runner.battle_area_size(1);

    let kind = play_and_install_branch_choice(&mut runner, 0);
    assert_eq!(kind, SelectionKind::EffectChoice, "must be 2-way EffectChoice");

    runner.execute_branch(0).expect("pick Delete branch");
    runner.auto_resolve().expect("auto-resolve target pick");

    // Opp digimon should be deleted; battle area count drops by 1.
    let opp_after = runner.battle_area_size(1);
    assert_eq!(
        opp_after,
        opp_before - 1,
        "Delete branch must remove the ≤8000-DP opp Digimon"
    );
}

/// The Delete branch must not offer opp Digimon with DP > 8000 as targets.
/// Boundary-positive: a 9000-DP opp Digimon is illegal.
#[test]
fn bt17_015_on_play_branch_0_cannot_target_dp_above_8000() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT17-015")
        .expect("BT17-015 in embedded DSL pack")
        .add_card(make_opp_digimon("OPP-9K", "OppHigh9K", 9000))
        .hand(0, &["BT17-015"])
        .memory(15)
        .start();

    runner.place_on_field(1, "OPP-9K", None);
    let opp_before = runner.battle_area_size(1);

    runner.play(0, 0).expect("BT17-015 plays");
    let kind = runner.pending_kind().expect("branch prompt installs");
    assert_eq!(kind, SelectionKind::EffectChoice);
    runner.execute_branch(0).expect("pick Delete branch");

    // Auto-resolve will pick the first legal action — if the dp_lte gate is
    // honored, no opp permanent is legal, and the prompt should resolve to a
    // no-op (no deletion). If the gate is broken, the 9000-DP opp would be
    // wrongly deleted.
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.battle_area_size(1),
        opp_before,
        "9000-DP opp Digimon must not be a legal target for the ≤8000-DP delete"
    );
}

/// Branch 1 (Digivolve): selecting branch 1 then a Gabumon on field then a
/// MetalGarurumon in hand, the MetalGarurumon stacks onto Gabumon for free.
///
/// **BLOCKED (observed)**: under current engine behavior, after the player
/// picks branch 1 then picks the Gabumon permanent (`OwnField` prompt), the
/// chained `select_hand` prompt for MetalGarurumon does not install — the
/// process closure terminates after the permanent pick and
/// `effect_initiated_digivolve` never executes. MetalGarurumon stays in
/// hand and Gabumon stays bare. The same shape works for `target: self`
/// (BT21-013), so the bug is specific to chaining `select_own_permanent` →
/// `select_hand` → `effect_initiated_digivolve` with `target: <binding>`.
/// Logged as DSL-GAP G-EFFECT-INITIATED-DIGIVOLVE-FROM-HAND-WITH-PERMANENT-TARGET.
#[test]
#[ignore = "BLOCKED: G-EFFECT-INITIATED-DIGIVOLVE-FROM-HAND-WITH-PERMANENT-TARGET — \
            select_own_permanent + select_hand + effect_initiated_digivolve(target: <binding>) \
            chain terminates after the permanent pick; the hand-pick prompt never installs \
            and the digivolve verb never executes (observed 2026-05-03 with current cards.pack). \
            Compare BT21-013 which uses target: self and works."]
fn bt17_015_on_play_branch_1_digivolves_gabumon_into_metalgarurumon_free() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT17-015")
        .expect("BT17-015 in embedded DSL pack")
        .add_card(make_gabumon("MY-GABU"))
        .add_card(make_metalgarurumon("MY-MG"))
        .hand(0, &["BT17-015", "MY-MG"])
        .memory(15)
        .start();

    let gabu_handle = runner.place_on_field(0, "MY-GABU", None);
    let battle_before = runner.battle_area_size(0);
    let hand_before = runner.hand_size(0);

    runner.play(0, 0).expect("BT17-015 plays");
    let kind = runner.pending_kind().expect("branch prompt installs");
    assert_eq!(kind, SelectionKind::EffectChoice);
    runner.execute_branch(1).expect("pick Digivolve branch");
    let _ = runner.auto_resolve();

    // The branch 1 body chains three prompts: own-permanent (Gabumon) →
    // hand (MetalGarurumon) → effect_initiated_digivolve. If the chain runs
    // to completion, MetalGarurumon ends up as the top card of the Gabumon
    // stack and leaves hand. The primary observable is the stack composition
    // (more robust than hand-size, which would also be affected by other
    // chained selections / draw effects).
    let gabu_perm = &runner.game.players[0].battle_area[gabu_handle.index as usize];
    let top = gabu_perm.top_card();
    assert_eq!(
        top.card_id(&runner.game.card_data),
        "MY-MG",
        "MetalGarurumon must be the top card of the (former) Gabumon stack \
         after the digivolve branch resolves"
    );
}

// ─── Section 4 — Inherited [When Attacking] condition gating (split) ────────

/// Stack BT17-015 under an Omnimon-named top card so the printed name gate
/// "[Omnimon] in name" matches the actual top card. Returns the carrier
/// permanent handle (the battle-area slot containing the stack).
fn place_omnimon_over_bt17_015(runner: &mut DebugRunner, player: PlayerId) -> PermanentHandle {
    runner.place_stack(player, &["BT17-015", "OMNI-TOP"])
}

/// Positive condition: stack with Omnimon at top + BT17-015 below — inherited
/// trigger fires and trashes 1 from opp security.
#[test]
fn bt17_015_inherited_when_attacking_omnimon_top_trashes_security() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT17-015")
        .expect("BT17-015 in embedded DSL pack")
        .add_card(make_omnimon_top("OMNI-TOP"))
        .add_card(make_filler_digimon("DEF-FILLER"))
        .add_card(make_test_card("SEC-FILLER", "SecFiller"))
        .security(1, &["SEC-FILLER", "SEC-FILLER", "SEC-FILLER"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let attacker = place_omnimon_over_bt17_015(&mut runner, 0);
    let defender = runner.place_on_field(1, "DEF-FILLER", Some(0));

    let sec_before = runner.security_count(1);
    assert!(sec_before >= 1, "test setup needs at least 1 opp security");

    runner.attack_digimon(attacker, defender, false);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.security_count(1),
        sec_before - 1,
        "Omnimon-named attacker must trash 1 opp security via inherited"
    );
}

/// Negative condition: stack has only WarGreymon (BT17-015) at top — name gate
/// must block the trash. **BLOCKED**: under DSL-GAP G-DSL-SOURCE-NAME-CONTAINS,
/// the predicate is parsed but never evaluated; the clause fires anyway.
#[test]
#[ignore = "BLOCKED: G-DSL-SOURCE-NAME-CONTAINS — source_name_contains predicate is parsed and \
            compiled but never evaluated in code/digimon-engine/src/dsl_cards/predicate.rs; \
            the inherited [When Attacking] gate degenerates to true and incorrectly trashes \
            opp security even when top card is not Omnimon-named."]
fn bt17_015_inherited_when_attacking_wargreymon_top_does_not_trash_security() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT17-015")
        .expect("BT17-015 in embedded DSL pack")
        .add_card(make_filler_digimon("DEF-FILLER"))
        .add_card(make_test_card("SEC-FILLER", "SecFiller"))
        .security(1, &["SEC-FILLER", "SEC-FILLER", "SEC-FILLER"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    // BT17-015 alone at top — printed name "WarGreymon" does NOT contain "Omnimon".
    let attacker = runner.place_on_field(0, "BT17-015", Some(0));
    let defender = runner.place_on_field(1, "DEF-FILLER", Some(0));

    let sec_before = runner.security_count(1);
    runner.attack_digimon(attacker, defender, false);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.security_count(1),
        sec_before,
        "WarGreymon-only top card must NOT trigger the [Omnimon]-name inherited"
    );
}

// ─── Section 5 — Inherited [When Attacking] OPT enforcement ─────────────────

/// OPT: a second attack in the same turn must NOT trash a second security.
/// Relies on G-OPT-TRIGGERED being closed for permanent-backed triggered
/// effects (fixed 2026-04-29; see qa/archetype-qa/engine-gaps.md:174).
#[test]
fn bt17_015_inherited_when_attacking_opt_blocks_second_activation() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT17-015")
        .expect("BT17-015 in embedded DSL pack")
        .add_card(make_omnimon_top("OMNI-TOP"))
        .add_card(make_filler_digimon("DEF-1"))
        .add_card(make_filler_digimon("DEF-2"))
        .add_card(make_test_card("SEC-FILLER", "SecFiller"))
        .security(1, &["SEC-FILLER", "SEC-FILLER", "SEC-FILLER", "SEC-FILLER"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let attacker = place_omnimon_over_bt17_015(&mut runner, 0);
    let def1 = runner.place_on_field(1, "DEF-1", Some(0));
    let sec_before = runner.security_count(1);

    runner.attack_digimon(attacker, def1, false);
    let _ = runner.auto_resolve();
    let sec_after_first = runner.security_count(1);
    assert_eq!(
        sec_after_first,
        sec_before - 1,
        "first attack must trash 1 opp security"
    );

    // Second attack same turn — even with another defender available — OPT
    // must lock out a second trash. The defender's controller may have changed
    // turn state; guard against that by checking attacker is still alive.
    if runner.battle_area_size(0) <= attacker.index as usize {
        // Attacker died (security counter, removal, etc) — no follow-up possible.
        return;
    }

    let def2 = runner.place_on_field(1, "DEF-2", Some(0));
    runner.attack_digimon(attacker, def2, false);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.security_count(1),
        sec_after_first,
        "OPT must prevent the inherited from firing twice in one turn"
    );
}

/// OPT resets after end_turn → opponent turn → back to controller.
///
/// **BLOCKED (observed)**: under current attack/turn cycling in the engine,
/// the second-turn attack does not produce a second security-trash even
/// after a full P0 → P1 → P0 cycle. Either the inherited OPT activation
/// counter is not reset on `end_turn`, or the attacker / inherited dispatch
/// is not re-armed across the cycle. Sibling: bt21_008 covers the reset
/// path via direct `enqueue_triggered` rather than a live attack, so the
/// reset itself works at the queue level — the gap is the attack-driven
/// re-fire on turn N+1. Logged as G-OPT-RESET-VIA-ATTACK-CYCLE.
#[test]
#[ignore = "BLOCKED: G-OPT-RESET-VIA-ATTACK-CYCLE — inherited [When Attacking][OPT] does not \
            re-fire on a fresh attack after a full P0→P1→P0 cycle, even though the queue-level \
            reset works (see bt21_008 sibling). Distinct from the OPT-lockout same-turn test \
            (bt17_015_inherited_when_attacking_opt_blocks_second_activation), which passes."]
fn bt17_015_inherited_when_attacking_opt_resets_after_end_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT17-015")
        .expect("BT17-015 in embedded DSL pack")
        .add_card(make_omnimon_top("OMNI-TOP"))
        .add_card(make_filler_digimon("DEF-1"))
        .add_card(make_filler_digimon("DEF-2"))
        .add_card(make_test_card("SEC-FILLER", "SecFiller"))
        .security(
            1,
            &["SEC-FILLER", "SEC-FILLER", "SEC-FILLER", "SEC-FILLER", "SEC-FILLER"],
        )
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let attacker = place_omnimon_over_bt17_015(&mut runner, 0);
    let def1 = runner.place_on_field(1, "DEF-1", Some(0));

    runner.attack_digimon(attacker, def1, false);
    let _ = runner.auto_resolve();
    let sec_after_first = runner.security_count(1);

    if runner.game_over() {
        return; // attack ended the game; reset assertion not reachable
    }

    runner.end_turn();
    runner.end_turn();

    if runner.battle_area_size(0) <= attacker.index as usize {
        return;
    }

    let def2 = runner.place_on_field(1, "DEF-2", Some(0));
    runner.attack_digimon(attacker, def2, false);
    let _ = runner.auto_resolve();

    assert!(
        runner.security_count(1) < sec_after_first,
        "OPT must reset across turn boundary; security must drop again"
    );
}

// ─── Section 6 — Event-log assertion for security trash ─────────────────────

/// Event-log smoke: the inherited When-Attacking trash must surface either as
/// a `GameEvent::Trash` entry (when wired) or as a hard state delta on
/// `security_count`. `Trash` is documented as "not emitted yet" in
/// `events.rs:78`, so this test asserts the state delta and only counts
/// events without strict assertion. Mirrors bt17_018's pattern.
#[test]
fn bt17_015_inherited_security_trash_observable_in_state() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT17-015")
        .expect("BT17-015 in embedded DSL pack")
        .add_card(make_omnimon_top("OMNI-TOP"))
        .add_card(make_filler_digimon("DEF-FILLER"))
        .add_card(make_test_card("SEC-FILLER", "SecFiller"))
        .security(1, &["SEC-FILLER", "SEC-FILLER", "SEC-FILLER"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let attacker = place_omnimon_over_bt17_015(&mut runner, 0);
    let defender = runner.place_on_field(1, "DEF-FILLER", Some(0));

    let sec_before = runner.security_count(1);
    let cp = runner.event_checkpoint();

    runner.attack_digimon(attacker, defender, false);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.security_count(1),
        sec_before - 1,
        "primary state delta — opp security drops by 1"
    );

    // Soft event-log inspection (Trash variant currently not emitted; record
    // for future when wiring lands). Don't assert strictly — keep the test
    // green under both wired and non-wired states.
    let events = runner.events_since(cp);
    let _trash_events = events
        .iter()
        .filter(|e| matches!(e, GameEvent::Trash { .. }))
        .count();
}
