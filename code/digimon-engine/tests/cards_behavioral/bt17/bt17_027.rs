//! BT17-027 MetalGarurumon — Digimon, Lv.6, Blue, DP 11000, Cost 11.
//! Traits: Cyborg
//! Evo: Lv.5 Blue / cost 3 (printed `xros_req` says w/[Garurumon] in name)
//!
//! # Card text (cards.json — verbatim)
//!
//! When this card would be played, if you have a Tamer with [Matt Ishida] in
//! its name, reduce the play cost by 3.
//! [On Play] [When Digivolving] Activate 1 of the effects below:
//!   ・1 of your opponent's Digimon or Tamers can't suspend until the end of
//!     their turn.
//!   ・1 of your [Agumon] may digivolve into [WarGreymon] in your hand,
//!     ignoring its digivolution requirements and without paying the cost.
//!
//! Inherited Effect [When Attacking] [Once Per Turn]
//!   If this Digimon has [Omnimon] in its name, unsuspend this Digimon.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT17/Blue/BT17_027.cs
//!
//! # Patterns covered (sister to BT17-015)
//! - D2 BeforePayCost cost_reduction with `when_playing_this: true` and an
//!   `any_permanent { kind: tamer, name_contains: "Matt Ishida" }` condition.
//! - E1 Branch-choice [On Play][When Digivolving] (2-way EffectChoice).
//! - F-CannotSuspend `add_modifier { CannotSuspend }` on opp Digimon/Tamer
//!   `expiry: end_of_opponents_turn` (lock branch).
//! - F2 Effect-initiated DNA-free digivolve from hand (digivolve branch),
//!   gated on G-EFFECT-INITIATED-DIGIVOLVE-FROM-HAND-WITH-PERMANENT-TARGET.
//! - G4 Inherited [When Attacking][Once Per Turn] gated on top-card name
//!   ("Omnimon"-name) → `unsuspend: { target: source }`. Negative case is
//!   blocked on G-DSL-SOURCE-NAME-CONTAINS.
//!
//! # Faithfulness diff vs. card text
//!
//! | Card-text element                                                       | YAML clause                                                              | Status         |
//! |-------------------------------------------------------------------------|--------------------------------------------------------------------------|----------------|
//! | "When this card would be played, if Matt Ishida tamer, -3 cost"         | `cost_reduction { when_playing_this: true, condition: any_permanent { tamer, name_contains: "Matt Ishida" }, amount: 3 }` | OK             |
//! | "[On Play][When Digivolving] choose 1: …"                                | `when: [on_play, when_digivolving]` + `select_effect_choice { 2 labels }` | OK             |
//! | "1 opp Digimon/Tamer can't suspend until end of their turn"             | `select_opponent_permanent { any_of: kind digimon|tamer }` + `add_modifier { CannotSuspend, expiry: end_of_opponents_turn }` | OK             |
//! | "1 of your [Agumon] may digivolve into [WarGreymon] in hand free"       | `select_own_permanent { name_contains: "Agumon" }` + `select_hand { name_contains: "WarGreymon" }` + `effect_initiated_digivolve { ignore_requirements, cost: 0 }` | OK *           |
//! | Inherited "[When Attacking][OPT] If [Omnimon]-name, unsuspend self"     | `scope: inherited`, `when: when_attacking`, `once_per_turn: true`, `condition: { source_name_contains: "Omnimon" }`, `unsuspend { target: source }` | DSL-GAP        |
//!
//! ## Known gaps (carry-overs from BT17-015 audit, see validated_cards_dsl.json)
//!
//! - **G-DSL-SOURCE-NAME-CONTAINS** — `source_name_contains` predicate parses
//!   and compiles but is not evaluated. The inherited [When Attacking] gate
//!   degenerates to `true`, so the unsuspend fires regardless of top-card name.
//!   Negative test below is `#[ignore]`'d on this gap.
//!
//! - **G-EFFECT-INITIATED-DIGIVOLVE-FROM-HAND-WITH-PERMANENT-TARGET** — the
//!   chain `select_own_permanent → select_hand → effect_initiated_digivolve
//!   { target: <binding> }` terminates after the permanent pick; the hand
//!   prompt never installs. Branch 1 behavioral test is `#[ignore]`'d on
//!   this gap.
//!
//! ## Faithfulness micro-issue (mirrors BT17-015)
//!
//! `name_contains: "Agumon"` / `name_contains: "WarGreymon"` use substring
//! matching (DSL has no exact-name predicate). DCGO uses `EqualsCardName`
//! (exact). E.g., `name_contains: "Agumon"` would also match "MetalGreymon"
//! through "Agumon"-derived names if any existed; in practice this is rarely
//! load-bearing for these specific names. Same shape in BT17-015.
//!
//! Tamer-name gate: `name_contains: "Matt Ishida"` matches "Matt Ishida" and
//! any superset (e.g. "Tai Kamiya & Matt Ishida"). DCGO additionally matches
//! the no-space variant `"MattIshida"` (BT17_027.cs:62). Not test-asserted.

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
    CompiledTriggeredClause,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, ModifierType, PlayerId};
use digimon_engine::events::GameEvent;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

// ─── Fixture helpers ────────────────────────────────────────────────────────

fn make_tamer(id: &str, name: &str) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Tamer;
    card
}

fn make_opp_digimon(id: &str, name: &str, dp: i32) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Digimon;
    card.level = Some(5);
    card.dp = Some(dp);
    card
}

fn make_opp_tamer(id: &str, name: &str) -> CardData {
    make_tamer(id, name)
}

fn make_agumon(id: &str) -> CardData {
    let mut card = make_test_card(id, "Agumon");
    card.card_kind = CardKind::Digimon;
    card.level = Some(3);
    card.dp = Some(3000);
    card
}

fn make_wargreymon(id: &str) -> CardData {
    let mut card = make_test_card(id, "WarGreymon");
    card.card_kind = CardKind::Digimon;
    card.level = Some(6);
    card.dp = Some(11000);
    card.play_cost = 12;
    card
}

fn make_omnimon_top(id: &str) -> CardData {
    // Top-card stand-in whose name contains "Omnimon" — used to ride above a
    // BT17-027 source so the inherited [When Attacking] clause's name gate
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
fn bt17_027_compiles_with_two_triggered_and_one_cost_reduction_declarative() {
    let runner = DebugRunner::builder()
        .dsl_card("BT17-027")
        .expect("BT17-027 in embedded DSL pack")
        .memory(0)
        .start();
    let compiled = runner.compiled_card("BT17-027").expect("compiled");

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

    assert_eq!(
        triggered, 2,
        "expected 2 triggered clauses (OnPlay/WhenDigi + inherited WhenAttacking)"
    );
    assert_eq!(cost_reduction, 1, "expected 1 cost_reduction declarative");
    assert_eq!(
        compiled.effects.len(),
        3,
        "total compiled clauses must be 3 (1 cost_reduction + 2 triggered)"
    );
}

#[test]
fn bt17_027_has_on_play_when_digivolving_clause_face_up_mandatory() {
    let runner = DebugRunner::builder()
        .dsl_card("BT17-027")
        .expect("BT17-027 in embedded DSL pack")
        .memory(0)
        .start();
    let compiled = runner.compiled_card("BT17-027").expect("compiled");

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
    assert!(
        !clause.optional,
        "branch-choice itself is mandatory once triggered"
    );
    assert!(
        !clause.once_per_turn,
        "no [Once Per Turn] on the branch-choice clause"
    );
}

#[test]
fn bt17_027_has_inherited_when_attacking_opt_clause() {
    let runner = DebugRunner::builder()
        .dsl_card("BT17-027")
        .expect("BT17-027 in embedded DSL pack")
        .memory(0)
        .start();
    let compiled = runner.compiled_card("BT17-027").expect("compiled");

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

    assert!(
        clause.once_per_turn,
        "[When Attacking] inherited must be once_per_turn"
    );
    assert_eq!(clause.scope, CompiledScope::Inherited);
}

#[test]
fn bt17_027_has_lv5_cost3_digivolve_alt_path() {
    let runner = DebugRunner::builder()
        .dsl_card("BT17-027")
        .expect("BT17-027 in embedded DSL pack")
        .memory(0)
        .start();
    let compiled = runner.compiled_card("BT17-027").expect("compiled");

    let standard = compiled.alt_paths.iter().find(|p| {
        p.kind == digimon_dsl::compiled::CompiledAltPathKind::Digivolve
            && matches!(
                p.cost,
                Some(digimon_dsl::compiled::CompiledCost::Literal(3))
            )
    });
    assert!(
        standard.is_some(),
        "must have Lv.5 / cost 3 digivolve alt-path"
    );
}

// ─── Section 2 — Cost-reduction condition gating ────────────────────────────

/// Negative: with NO Matt Ishida tamer on field, full 11 cost is paid.
#[test]
fn bt17_027_cost_reduction_blocked_without_matt_ishida_tamer() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT17-027")
        .expect("BT17-027 in embedded DSL pack")
        .add_card(make_opp_digimon("OPP-DIGI", "OppDigi", 5000))
        .hand(0, &["BT17-027"])
        .memory(15)
        .start();

    runner.place_on_field(1, "OPP-DIGI", None);

    let mem_before = runner.memory();
    runner.play(0, 0);
    let cost = mem_before - runner.memory();

    assert_eq!(
        cost, 11,
        "with no Matt Ishida tamer, full printed cost (11) should be paid; got {cost}"
    );
}

/// Positive: with a Matt Ishida tamer on field, cost is reduced by 3 → 8.
#[test]
fn bt17_027_cost_reduction_applies_with_matt_ishida_tamer() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT17-027")
        .expect("BT17-027 in embedded DSL pack")
        .add_card(make_tamer("MATT-TAMER", "Matt Ishida"))
        .add_card(make_opp_digimon("OPP-DIGI", "OppDigi", 5000))
        .hand(0, &["BT17-027"])
        .memory(15)
        .start();

    runner.place_on_field(0, "MATT-TAMER", None);
    runner.place_on_field(1, "OPP-DIGI", None);

    let mem_before = runner.memory();
    runner.play(0, 0);
    let cost = mem_before - runner.memory();

    assert_eq!(
        cost, 8,
        "with a Matt Ishida tamer, cost should be 11-3=8; got {cost}"
    );
}

/// Negative: an unrelated Tamer on field must NOT trigger the reduction —
/// guards against `name_contains` false positives.
#[test]
fn bt17_027_cost_reduction_blocked_by_unrelated_tamer() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT17-027")
        .expect("BT17-027 in embedded DSL pack")
        .add_card(make_tamer("OTHER-TAMER", "Tai Kamiya"))
        .add_card(make_opp_digimon("OPP-DIGI", "OppDigi", 5000))
        .hand(0, &["BT17-027"])
        .memory(15)
        .start();

    runner.place_on_field(0, "OTHER-TAMER", None);
    runner.place_on_field(1, "OPP-DIGI", None);

    let mem_before = runner.memory();
    runner.play(0, 0);
    let cost = mem_before - runner.memory();

    assert_eq!(
        cost, 11,
        "Tai Kamiya tamer must NOT count for [Matt Ishida] gate; got cost {cost}"
    );
}

/// Cost-reduction must not leak to other cards being played from hand.
#[test]
fn bt17_027_cost_reduction_does_not_leak_to_other_plays() {
    let mut other = make_test_card("OTHER-DIGI", "OtherDigi");
    other.card_kind = CardKind::Digimon;
    other.play_cost = 8;
    other.colors = vec![digimon_engine::enums::CardColor::Blue];
    other.level = Some(5);
    other.dp = Some(7000);

    let mut runner = DebugRunner::builder()
        .dsl_card("BT17-027")
        .expect("BT17-027 in embedded DSL pack")
        .add_card(make_tamer("MATT-TAMER", "Matt Ishida"))
        .add_card(other)
        .hand(0, &["OTHER-DIGI"])
        .memory(15)
        .start();

    runner.place_on_field(0, "MATT-TAMER", None);

    let mem_before = runner.memory();
    runner.play(0, 0);
    let cost = mem_before - runner.memory();

    assert_eq!(
        cost, 8,
        "Matt Ishida rebate must not leak to OTHER-DIGI; expected full 8, got {cost}"
    );
}

// ─── Section 3 — [On Play][When Digivolving] branch-choice behavioral ───────

fn play_and_install_branch_choice(runner: &mut DebugRunner, hand_idx: usize) -> SelectionKind {
    runner.play(0, hand_idx).expect("BT17-027 plays");
    runner
        .pending_kind()
        .expect("EffectChoice prompt must install on OnPlay")
}

/// Branch 0 (CannotSuspend): selecting branch 0 then a single opp Digimon
/// installs a `CannotSuspend` modifier on it. The state observable here is
/// `is_suspended` toggling: if we manually call suspend() pre-modifier and
/// post-modifier, only the pre-modifier suspension actually toggles the
/// is_suspended flag. To keep things simple and reliable, we assert the
/// modifier is installed on the chosen permanent.
#[test]
fn bt17_027_on_play_branch_0_locks_opp_digimon_cannot_suspend() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT17-027")
        .expect("BT17-027 in embedded DSL pack")
        .add_card(make_opp_digimon("OPP-DIGI", "OppDigi", 5000))
        .hand(0, &["BT17-027"])
        .memory(15)
        .start();

    let opp_handle = runner.place_on_field(1, "OPP-DIGI", None);

    let kind = play_and_install_branch_choice(&mut runner, 0);
    assert_eq!(
        kind,
        SelectionKind::EffectChoice,
        "must be 2-way EffectChoice"
    );

    runner.execute_branch(0).expect("pick CannotSuspend branch");
    runner.auto_resolve().expect("auto-resolve target pick");

    // Opp digimon is still on board (CannotSuspend doesn't delete).
    let opp_after = runner.battle_area_size(1);
    assert!(
        opp_after >= 1,
        "lock branch must NOT delete the opp Digimon"
    );

    // Inspect the chosen opp permanent for the CannotSuspend modifier via the
    // game-scoped modifier registry.
    let has_cannot_suspend = runner
        .game
        .modifiers
        .has(opp_handle, ModifierType::CannotSuspend);
    assert!(
        has_cannot_suspend,
        "CannotSuspend modifier must be installed on the chosen opp Digimon"
    );
}

/// Branch 0 alternate target type — opp Tamer should also be a legal pick
/// for the lock branch (printed text says "Digimon or Tamers").
#[test]
fn bt17_027_on_play_branch_0_can_target_opp_tamer() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT17-027")
        .expect("BT17-027 in embedded DSL pack")
        .add_card(make_opp_tamer("OPP-TAMER", "SomeTamer"))
        .hand(0, &["BT17-027"])
        .memory(15)
        .start();

    let opp_handle = runner.place_on_field(1, "OPP-TAMER", None);

    let kind = play_and_install_branch_choice(&mut runner, 0);
    assert_eq!(kind, SelectionKind::EffectChoice);
    runner.execute_branch(0).expect("pick CannotSuspend branch");
    let _ = runner.auto_resolve();

    let has_cannot_suspend = runner
        .game
        .modifiers
        .has(opp_handle, ModifierType::CannotSuspend);
    assert!(
        has_cannot_suspend,
        "CannotSuspend must be installable on opp Tamer (printed text covers Tamers)"
    );
}

/// Branch 1 (Digivolve): selecting branch 1 then an Agumon on field then a
/// WarGreymon in hand, the WarGreymon stacks onto Agumon for free.
///
/// **BLOCKED**: same shape as BT17-015 branch 1 — the chain
/// `select_own_permanent → select_hand → effect_initiated_digivolve
/// { target: <binding> }` terminates after the permanent pick; the hand-pick
/// prompt never installs and the digivolve verb never executes.
/// Logged as G-EFFECT-INITIATED-DIGIVOLVE-FROM-HAND-WITH-PERMANENT-TARGET.
#[test]
#[ignore = "BLOCKED: G-EFFECT-INITIATED-DIGIVOLVE-FROM-HAND-WITH-PERMANENT-TARGET — \
            select_own_permanent + select_hand + effect_initiated_digivolve(target: <binding>) \
            chain terminates after the permanent pick; the hand-pick prompt never installs \
            and the digivolve verb never executes (carried over from BT17-015 audit). \
            Compare BT21-013 which uses target: self and works."]
fn bt17_027_on_play_branch_1_digivolves_agumon_into_wargreymon_free() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT17-027")
        .expect("BT17-027 in embedded DSL pack")
        .add_card(make_agumon("MY-AGU"))
        .add_card(make_wargreymon("MY-WG"))
        .hand(0, &["BT17-027", "MY-WG"])
        .memory(15)
        .start();

    let agu_handle = runner.place_on_field(0, "MY-AGU", None);
    let battle_before = runner.battle_area_size(0);
    let hand_before = runner.hand_size(0);

    runner.play(0, 0).expect("BT17-027 plays");
    let kind = runner.pending_kind().expect("branch prompt installs");
    assert_eq!(kind, SelectionKind::EffectChoice);
    runner.execute_branch(1).expect("pick Digivolve branch");
    let _ = runner.auto_resolve();

    let agu_perm = &runner.game.players[0].battle_area[agu_handle.index as usize];
    let top = agu_perm.top_card();
    assert_eq!(
        top.card_id(&runner.game.card_data),
        "MY-WG",
        "WarGreymon must be the top card of the (former) Agumon stack \
         after the digivolve branch resolves"
    );
}

// ─── Section 4 — Inherited [When Attacking] condition gating ────────────────

/// Stack BT17-027 under an Omnimon-named top card so the printed name gate
/// "[Omnimon] in name" matches the actual top card. Returns the carrier
/// permanent handle (the battle-area slot containing the stack).
fn place_omnimon_over_bt17_027(runner: &mut DebugRunner, player: PlayerId) -> PermanentHandle {
    runner.place_stack(player, &["BT17-027", "OMNI-TOP"])
}

/// Positive condition: stack with Omnimon at top + BT17-027 below — inherited
/// trigger fires and unsuspends the stack carrier. We test by suspending the
/// attacker pre-attack (manually via the engine API) and asserting that after
/// the attack resolves it ends unsuspended.
///
/// For attacks, attackers normally suspend as part of declaring the attack,
/// then the `WhenAttacking` trigger fires. So we just need to assert the
/// attacker is unsuspended by the trigger.
#[test]
fn bt17_027_inherited_when_attacking_omnimon_top_unsuspends_self() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT17-027")
        .expect("BT17-027 in embedded DSL pack")
        .add_card(make_omnimon_top("OMNI-TOP"))
        .add_card(make_filler_digimon("DEF-FILLER"))
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let attacker = place_omnimon_over_bt17_027(&mut runner, 0);
    let defender = runner.place_on_field(1, "DEF-FILLER", Some(0));

    runner.attack_digimon(attacker, defender, false);
    let _ = runner.auto_resolve();

    // After the attack, the inherited [When Attacking] trigger should have
    // unsuspended the attacker. (Attacks suspend the attacker as part of
    // declaring the attack, then triggers fire.)
    let attacker_perm = &runner.game.players[0].battle_area[attacker.index as usize];
    assert!(
        !attacker_perm.is_suspended,
        "Omnimon-named attacker must be unsuspended by inherited trigger after attack"
    );
}

/// Negative condition: stack has only MetalGarurumon (BT17-027) at top — name
/// gate must block the unsuspend. **BLOCKED**: under DSL-GAP
/// G-DSL-SOURCE-NAME-CONTAINS, the predicate is parsed but never evaluated;
/// the clause fires anyway and incorrectly unsuspends the attacker.
#[test]
#[ignore = "BLOCKED: G-DSL-SOURCE-NAME-CONTAINS — source_name_contains predicate is parsed and \
            compiled but never evaluated in code/digimon-engine/src/dsl_cards/predicate.rs; \
            the inherited [When Attacking] gate degenerates to true and incorrectly unsuspends \
            the attacker even when top card is not Omnimon-named."]
fn bt17_027_inherited_when_attacking_metalgarurumon_top_does_not_unsuspend() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT17-027")
        .expect("BT17-027 in embedded DSL pack")
        .add_card(make_filler_digimon("DEF-FILLER"))
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    // BT17-027 alone at top — printed name "MetalGarurumon" does NOT contain "Omnimon".
    let attacker = runner.place_on_field(0, "BT17-027", Some(0));
    let defender = runner.place_on_field(1, "DEF-FILLER", Some(0));

    runner.attack_digimon(attacker, defender, false);
    let _ = runner.auto_resolve();

    // Without the Omnimon-name gate the attacker should remain suspended.
    let attacker_perm = &runner.game.players[0].battle_area[attacker.index as usize];
    assert!(
        attacker_perm.is_suspended,
        "MetalGarurumon-only top card must NOT trigger the [Omnimon]-name unsuspend"
    );
}
