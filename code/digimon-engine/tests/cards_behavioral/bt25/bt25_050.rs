//! BT25-050 Kiwimon — Digimon, Lv.4, Green, DP 4000, Cost 4.
//! Traits: Vegetation, Iliad, TS.
//!
//! # Card text (cards.json — verbatim)
//!
//! [On Play] [When Digivolving] You may suspend 1 Digimon. Then, if there are
//! 2 or more suspended Digimon, 1 of your opponent's Digimon can't unsuspend
//! until their turn ends.
//!
//! Inherited: [Your Turn] All of your Digimon get +1000 DP.
//!
//! Printed digivolve box: Lv.3 w/[TS] trait / Cost 2 (cards.json evo_costs empty).
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Green/BT25_050.cs
//!
//! - Region "Digivolution Condition" (None): AddSelfDigivolutionRequirement-
//!   StaticEffect(HasTSTraits, cost 2, level 3).
//! - Region "Shared OP/WD": Step 1 optional suspend of ANY battle-area Digimon
//!   (canNoSelect: true). Step 2 gate `SuspendedDigimon count >= 2` (both
//!   players) AND an opponent Digimon exists → MANDATORY pick of 1 opp Digimon
//!   → GainCanNotUnsuspend(UntilOpponentTurnEnd).
//! - Region "ESS" (inherited): ChangeDPStaticEffect(+1000) over OWNER's Digimon,
//!   gated on owner-turn — "[Your Turn] All of your Digimon get +1000 DP."
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - F7 cannot-unsuspend lock (conditional on a board count)
//! - D4 inherited declarative aura (team-wide +DP, your-turn gated)
//! - E2 optional inner select (you-may-suspend) with PASS path

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, ModifierType, PlayerId};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT25-050";

// ─── Fixture helpers ─────────────────────────────────────────────────────────

fn make_digimon(id: &str, level: u8, dp: i32, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    card.evo_costs = vec![EvoCost {
        card_color: 3,
        level: level.saturating_sub(1),
        memory_cost: 2,
    }];
    card
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-050 YAML parses and compiles")
        .add_card(make_test_card("DECK-PAD", "Filler"))
        .add_card(make_digimon("OPP-A", 4, 5000, &["Beast"]))
        .add_card(make_digimon("OPP-B", 4, 4000, &["Beast"]))
        .add_card(make_digimon("OWN-A", 5, 4000, &["Beast"]))
        .deck(0, &["DECK-PAD"; 10])
        .deck(1, &["DECK-PAD"; 10])
}

fn is_suspended(runner: &DebugRunner, h: PermanentHandle) -> bool {
    runner.game.players[h.player as usize].battle_area[h.index as usize].is_suspended
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn bt25_050_yaml_has_printed_metadata() {
    let runner = base().start();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("BT25-050 must be present in embedded DSL pack");

    assert_eq!(card.name, "Kiwimon");
    assert_eq!(card.level, Some(4));
    assert_eq!(card.cost, Some(4));
    assert_eq!(card.dp, Some(4000));
    for trait_name in ["Vegetation", "Iliad", "TS"] {
        assert!(
            card.traits.contains(&trait_name.to_string()),
            "BT25-050 metadata must include trait {trait_name}"
        );
    }
}

#[test]
fn bt25_050_has_shared_on_play_when_digivolving_clause() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let clause = card
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
        .expect("BT25-050 must have a shared OnPlay/WhenDigivolving clause");

    assert_eq!(clause.scope, CompiledScope::FaceUp);
    // The lock (AddModifier) lives inside the count-gate `if` block, so recurse.
    fn has_add_modifier(steps: &[CompiledStep]) -> bool {
        steps.iter().any(|s| match s {
            CompiledStep::AddModifier { .. } => true,
            CompiledStep::If {
                then, else_branch, ..
            } => has_add_modifier(then) || has_add_modifier(else_branch),
            _ => false,
        })
    }
    assert!(
        has_add_modifier(&clause.process),
        "shared clause must add a CannotUnsuspend modifier (the lock step)"
    );
}

#[test]
fn bt25_050_has_inherited_dp_aura() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let aura = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
                dp_modifier,
                scope,
                ..
            }) => Some((*dp_modifier, *scope)),
            _ => None,
        })
        .expect("BT25-050 must have a DP aura clause");

    assert_eq!(aura.0, Some(1000), "inherited aura grants +1000 DP");
    assert_eq!(
        aura.1,
        CompiledScope::Inherited,
        "the DP aura must be inherited scope"
    );
}

// ─── Section 2 — Behavior: suspend + conditional lock ─────────────────────────

#[test]
fn bt25_050_on_play_installs_suspend_prompt() {
    let mut runner = base().hand(0, &[CARD_ID]).memory(6).start();
    runner.place_on_field(1, "OPP-A", Some(0));

    let _k = runner.play(0, 0).expect("play Kiwimon");

    assert!(
        runner.pending_selection().is_some(),
        "OnPlay must install the optional-suspend prompt"
    );
}

#[test]
fn bt25_050_suspend_pick_is_optional() {
    let mut runner = base().hand(0, &[CARD_ID]).memory(6).start();
    runner.place_on_field(1, "OPP-A", Some(0));

    let _k = runner.play(0, 0).expect("play Kiwimon");
    assert!(
        runner.pending_selection().is_some(),
        "prompt must install first"
    );
    assert!(
        runner.pending_is_optional(),
        "the you-may-suspend pick must expose PASS"
    );
}

/// Negative gate: declining the suspend leaves fewer than 2 suspended Digimon,
/// so no opponent permanent is locked.
#[test]
fn bt25_050_declining_suspend_with_fewer_than_two_suspended_does_not_lock() {
    let mut runner = base().hand(0, &[CARD_ID]).memory(6).start();
    let opp = runner.place_on_field(1, "OPP-A", Some(0));

    let _k = runner.play(0, 0).expect("play Kiwimon");
    runner
        .execute_action(0, PASS)
        .expect("decline the optional suspend");
    runner.auto_resolve().ok();

    assert!(
        !runner.modifiers().has(opp, ModifierType::CannotUnsuspend),
        "with fewer than 2 suspended Digimon, no opponent permanent is locked"
    );
}

/// Positive lock: with 2+ suspended Digimon, Kiwimon's OnPlay locks 1 opponent
/// Digimon from unsuspending.
///
/// BLOCKED on G-ENGINE-IF-AFTER-SELECTION-NOT-RESUMED: the count-gated lock
/// `if` step never installs because the engine does not execute a trailing
/// conditional (`if`) step when the process resumes after the interactive
/// `select_any_permanent` (suspend) resolves. Proven adjacent: replacing the
/// `if { count_gte ... }` wrapper with an UNCONDITIONAL lock fires correctly,
/// so the gap is specifically "conditional step after a parked selection is not
/// resumed." Locking unconditionally would be unfaithful (it must require 2+
/// suspended), so the card ships the faithful (count-gated) YAML and this test
/// is ignored until the engine gap closes. See docs/RUST_ENGINE_GAPS.md.
#[test]
#[ignore = "pending: G-ENGINE-IF-AFTER-SELECTION-NOT-RESUMED from docs/RUST_ENGINE_GAPS.md"]
fn bt25_050_two_suspended_locks_an_opponent_digimon() {
    let mut runner = base().hand(0, &[CARD_ID]).memory(6).start();
    let opp_a = runner.place_on_field(1, "OPP-A", Some(0));
    let opp_b = runner.place_on_field(1, "OPP-B", Some(0));
    runner.game.suspend(opp_a);
    runner.game.suspend(opp_b);

    let _k = runner.play(0, 0).expect("play Kiwimon");
    let suspend_action = runner
        .pending_selection_view()
        .unwrap()
        .valid_action_ids
        .into_iter()
        .find(|a| *a != PASS)
        .expect("a suspend target is offered");
    runner
        .execute_action(0, suspend_action)
        .expect("suspend the third Digimon");
    runner
        .auto_resolve()
        .expect("resolve the mandatory lock pick");

    let locked = runner.modifiers().has(opp_a, ModifierType::CannotUnsuspend)
        || runner.modifiers().has(opp_b, ModifierType::CannotUnsuspend);
    assert!(
        locked,
        "with 2+ suspended Digimon, an opponent Digimon must gain CannotUnsuspend"
    );
}

// ─── Section 3 — Behavior: inherited +1000 DP aura ────────────────────────────

/// Inherited aura: Kiwimon as a digivolution source under a carrier grants the
/// controller's Digimon +1000 DP on the controller's turn.
#[test]
fn bt25_050_inherited_aura_buffs_own_digimon_on_your_turn() {
    let mut runner = base().memory(6).start();
    // place_stack: last id is the top card. Kiwimon underneath OWN-A.
    let carrier = runner.place_stack(0, &["BT25-050", "OWN-A"]);
    // Filtered (team-wide) auras materialize on a declarative tick; place_stack
    // skips the play action, so tick explicitly before reading effective DP.
    runner.game.tick_declarative_effects();

    let dp = runner
        .effective_dp(carrier)
        .expect("carrier has effective DP");
    assert_eq!(
        dp, 5000,
        "inherited [Your Turn] aura must add +1000 DP to the controller's Digimon (base 4000)"
    );
}

/// Gate (negative): the inherited aura is inactive on the opponent's turn.
#[test]
fn bt25_050_inherited_aura_inactive_on_opponents_turn() {
    let mut runner = base().memory(6).start();
    let carrier = runner.place_stack(0, &["BT25-050", "OWN-A"]);
    runner.end_turn(); // now player 1's turn
    runner.game.tick_declarative_effects();

    let dp = runner
        .effective_dp(carrier)
        .expect("carrier has effective DP");
    assert_eq!(
        dp, 4000,
        "on the opponent's turn the [Your Turn] aura is inactive (base 4000)"
    );
}
