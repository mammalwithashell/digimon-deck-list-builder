//! BT24-018 Styracomon — Digimon, Lv.7, Red, DP14000, Cost14.
//!
//! # Card text (cards.json)
//!
//! `＜Progress＞ ＜Piercing＞ ＜Blocker＞ ＜Armor Purge＞`
//! `[When Digivolving] You may trash any 1 of your opponent's security cards.`
//! `Then, this Digimon may unsuspend.`
//! `[All Turns] [Once Per Turn] When your opponent's security stack is removed`
//! `from, you may delete 1 of their Digimon.`
//! `[All Turns] [Once Per Turn] When any of your [Reptile] or [Dragonkin]`
//! `trait Digimon would leave the battle area, by deleting 1 of your opponent's`
//! `lowest DP Digimon, they don't leave.`
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT24/Red/BT24_018.cs
//!
//! # Patterns this test file covers
//! - H-Progress/H-Piercing/H5-Blocker: keyword grants (declarative)
//! - H-ArmorPurge: ArmorPurge keyword grant (declarative)
//! - E2: [When Digivolving] optional trash-opponent-security + optional
//!   `unsuspend` sub-clauses (clause e — two independent `when_digivolving` clauses)
//! - F-OPT: [All Turns][OPT] OnOpponentSecurityRemoved → optional delete opp Digimon (clause f)
//! - F3: [All Turns][OPT] WhenWouldLeaveBattleArea replacement (clause g)
//!
//! # Clause / gap status
//!
//! | Clause | Status | Gap |
//! |--------|--------|-----|
//! | (a)-(d) Progress / Piercing / Blocker / ArmorPurge | IMPLEMENTED | — |
//! | (e) sub-clause (a) trash any 1 opp security card | IMPLEMENTED | G-TRASH-SELECTED-SECURITY resolved 2026-05-21 |
//! | (e) sub-clause (b) "this Digimon may unsuspend"   | IMPLEMENTED | — |
//! | (f) [All Turns][OPT] opp security removed → delete | IMPLEMENTED | G-OPT-TRIGGERED closed (Phase 2 Track C) |
//! | (g) [All Turns][OPT] would-leave replacement       | IMPLEMENTED | G-EVENT-TARGET-OWNER + G-PRED-DP-LTE resolved |

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::EffectTiming;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::{SelectionKind, TriggerSource};

use crate::dsl_card_data::{card_data_from_compiled, compiled};

// ─── helpers ──────────────────────────────────────────────────────────────────

fn resolve_first_pending(runner: &mut DebugRunner) {
    let (player, action) = {
        let pending = runner
            .game
            .pending_selection
            .as_ref()
            .expect("a pending selection must be installed");
        (pending.selecting_player, pending.valid_action_ids[0])
    };
    runner
        .game
        .resolve_selection(player, action)
        .expect("resolve_selection succeeds");
}

fn pass_pending(runner: &mut DebugRunner) {
    let player = runner
        .game
        .pending_selection
        .as_ref()
        .expect("selection installed")
        .selecting_player;
    runner
        .game
        .resolve_selection(player, PASS)
        .expect("decline resolves");
}

fn make_digimon(id: &str, dp: i32, traits: &[&str]) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.dp = Some(dp);
    card.traits = traits.iter().map(|s| s.to_string()).collect();
    card
}

fn permanent_exists(runner: &DebugRunner, player: usize, card_id: &str) -> bool {
    runner.game.players[player]
        .battle_area
        .iter()
        .any(|permanent| permanent.top_card().card_id(&runner.game.card_data) == card_id)
}

// ─── SECTION 1 — Structural assertions ───────────────────────────────────────

/// Styracomon compiles with exactly 4 GrantKeyword declarative clauses:
/// Progress, Piercing, Blocker, ArmorPurge.
#[test]
fn bt24_018_has_four_keyword_grants() {
    let card = compiled("BT24-018");

    let keyword_grants: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                ..
            }) => Some(keyword.clone()),
            _ => None,
        })
        .collect();

    assert_eq!(
        keyword_grants.len(),
        4,
        "Progress, Piercing, Blocker, ArmorPurge must each produce one GrantKeyword clause; \
         got: {:?}",
        keyword_grants
    );
}

/// The four granted keywords are exactly: Progress, Piercing, Blocker, ArmorPurge.
#[test]
fn bt24_018_keyword_grants_are_progress_piercing_blocker_armor_purge() {
    let card = compiled("BT24-018");

    let keywords: std::collections::HashSet<String> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                ..
            }) => Some(keyword.clone()),
            _ => None,
        })
        .collect();

    for expected in &["Progress", "Piercing", "Blocker", "ArmorPurge"] {
        assert!(
            keywords.contains(*expected),
            "keyword grant '{}' missing; got: {:?}",
            expected,
            keywords
        );
    }
}

/// Clause (e) is a triggered clause on WhenDigivolving that is optional.
#[test]
fn bt24_018_has_when_digivolving_clause_that_is_optional() {
    let card = compiled("BT24-018");

    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::WhenDigivolving));

    let clause = clause.expect("WhenDigivolving triggered clause (e) must exist");
    assert!(
        clause.optional,
        "clause (e) must be optional (card text says 'may')"
    );
    assert_eq!(
        clause.scope,
        CompiledScope::FaceUp,
        "clause (e) is a face-up (own) effect"
    );
}

/// Clause (f) is a triggered OPT clause on OnOpponentSecurityRemoved that is
/// optional and fires on all turns.
#[test]
fn bt24_018_has_on_opponent_security_removed_clause_opt_and_optional() {
    let card = compiled("BT24-018");

    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnOpponentSecurityRemoved));

    let clause = clause.expect("OnOpponentSecurityRemoved triggered clause (f) must exist");
    assert!(clause.optional, "clause (f) must be optional");
    assert!(
        clause.once_per_turn,
        "clause (f) must be [Once Per Turn] / OPT"
    );
}

/// Clause (g) is a Replacement declarative clause on when_would_leave_battle_area
/// and is once-per-turn.
#[test]
fn bt24_018_has_would_leave_battle_area_replacement_clause() {
    let card = compiled("BT24-018");

    let replacement = card.effects.iter().find_map(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::Replacement {
            trigger,
            once_per_turn,
            optional,
            ..
        }) if trigger == "when_would_leave_battle_area" => Some((*once_per_turn, *optional)),
        _ => None,
    });

    let (once_per_turn, optional) = replacement.expect(
        "BT24-018 must have a Replacement clause with trigger when_would_leave_battle_area \
         for clause (g)",
    );
    assert!(once_per_turn, "clause (g) is [Once Per Turn]");
    assert!(
        optional,
        "clause (g) is optional ('you may'-style replacement)"
    );
}

// ─── SECTION 2 — Clause (e): [When Digivolving] unsuspend + blocked trash ────

/// Clause (e) sub-clause (b): when BT24-018 digivolves while suspended, the
/// optional [When Digivolving] prompt installs and is declinable.
#[test]
fn bt24_018_when_digivolving_installs_optional_prompt() {
    let mut runner = bt24_018_clause_e_runner();
    let styracomon = runner.place_on_field(0, "BT24-018", Some(0));
    // BT24-018 enters suspended so the unsuspend body has an observable effect.
    runner.game.players[0].battle_area[styracomon.index as usize].is_suspended = true;

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(styracomon),
    );
    runner.game.drain_effect_queue();

    let view = runner
        .pending_selection_view()
        .expect("clause (e) optional prompt must install");
    assert!(
        view.is_optional,
        "[When Digivolving] 'may unsuspend' must be declinable"
    );
}

/// Clause (e) sub-clause (b): accepting the optional prompt unsuspends BT24-018.
#[test]
fn bt24_018_when_digivolving_accept_unsuspends_self() {
    let mut runner = bt24_018_clause_e_runner();
    let styracomon = runner.place_on_field(0, "BT24-018", Some(0));
    runner.game.players[0].battle_area[styracomon.index as usize].is_suspended = true;

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(styracomon),
    );
    runner.game.drain_effect_queue();

    // Accept the optional prompt → unsuspend body runs.
    resolve_first_pending(&mut runner);
    runner.auto_resolve().ok();

    assert!(
        !runner.game.players[0].battle_area[styracomon.index as usize].is_suspended,
        "accepting [When Digivolving] must unsuspend BT24-018"
    );
}

/// Clause (e) sub-clause (b): declining the optional prompt leaves BT24-018
/// suspended.
#[test]
fn bt24_018_when_digivolving_decline_leaves_suspended() {
    let mut runner = bt24_018_clause_e_runner();
    let styracomon = runner.place_on_field(0, "BT24-018", Some(0));
    runner.game.players[0].battle_area[styracomon.index as usize].is_suspended = true;

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(styracomon),
    );
    runner.game.drain_effect_queue();

    // Decline the optional prompt → unsuspend body is skipped.
    pass_pending(&mut runner);
    runner.auto_resolve().ok();

    assert!(
        runner.game.players[0].battle_area[styracomon.index as usize].is_suspended,
        "declining [When Digivolving] must leave BT24-018 suspended"
    );
}

/// Clause (e) sub-clause (a) "You may trash any 1 of your opponent's security
/// cards" — G-TRASH-SELECTED-SECURITY resolved 2026-05-21. The
/// `trash_selected_security` DSL verb consumes a `select_security` binding and
/// `EffectContext::trash_security_card` trashes that exact card by stable
/// handle, so any security position — not just the top — can be chosen.
///
/// Clause (e) is modelled as two `when_digivolving` clauses (trash, then
/// unsuspend) so each printed "may" keeps its own optionality; firing both
/// installs a harmless `TriggerOrder` prompt (the two actions are independent,
/// so the order never changes the outcome). The test drives every prompt.
#[test]
fn bt24_018_when_digivolving_trash_chosen_opponent_security() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT24-018")
        .expect("BT24-018 YAML loads")
        .add_card(make_digimon("SEC", 1000, &[]))
        .deck(0, &["SEC"; 5])
        .deck(1, &["SEC"; 5])
        .security(1, &["SEC", "SEC", "SEC"])
        .start();
    let styracomon = runner.place_on_field(0, "BT24-018", Some(0));

    assert_eq!(
        runner.security_count(1),
        3,
        "precondition: opponent has 3 security cards"
    );

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(styracomon),
    );
    runner.game.drain_effect_queue();

    // Drive every prompt, picking valid_action_ids[0] throughout — for the
    // trash `select_security` that is the BOTTOM (index-0, non-top) security
    // card, proving an arbitrary security position can be trashed.
    let mut saw_security_prompt = false;
    let mut steps = 0;
    while runner.game.pending_selection.is_some() && steps < 20 {
        let (player, action, is_security) = {
            let p = runner.game.pending_selection.as_ref().unwrap();
            (
                p.selecting_player,
                p.valid_action_ids[0],
                matches!(p.kind, SelectionKind::Security),
            )
        };
        if is_security {
            saw_security_prompt = true;
        }
        runner.game.resolve_selection(player, action).ok();
        runner.game.drain_effect_queue();
        steps += 1;
    }

    assert!(
        saw_security_prompt,
        "the trash clause must install a security-stack selection over the \
         opponent's 3 cards"
    );
    assert_eq!(
        runner.security_count(1),
        2,
        "exactly 1 opponent security card was trashed via trash_selected_security"
    );
}

// ─── SECTION 3 — Clause (f): OnOpponentSecurityRemoved → delete Digimon ──────

/// Clause (f): when opponent's security stack is removed, the optional
/// accept/decline gate installs ('you may'). Accepting opens the
/// `select_opponent_permanent` (OppField) prompt; picking deletes the target.
#[test]
fn bt24_018_on_opp_security_removed_delete_digimon_accept() {
    let mut runner = bt24_018_clause_f_runner();
    let styracomon = runner.place_on_field(0, "BT24-018", Some(0));
    runner.place_on_field(1, "OPP-DIGI", Some(0));

    let opp_before = runner.battle_area_size(1);

    // Fire OnOpponentSecurityRemoved from BT24-018's battle area (the attacker's
    // battle area is where this observer is reachable).
    runner.game.enqueue_triggered(
        EffectTiming::OnOpponentSecurityRemoved,
        TriggerSource::Permanent(styracomon),
    );
    runner.game.drain_effect_queue();

    // The "you may" outer gate installs first and is declinable.
    let gate = runner
        .pending_selection_view()
        .expect("clause (f) optional gate must install");
    assert!(
        gate.is_optional,
        "clause (f) gate is 'you may' — declinable"
    );

    // Accept the gate → the opponent-Digimon selection opens.
    resolve_first_pending(&mut runner);
    let target = runner
        .pending_selection_view()
        .expect("clause (f) delete-target selection must install after accept");
    assert_eq!(
        target.kind,
        SelectionKind::OppField,
        "clause (f) selects an opponent Digimon"
    );

    // Pick the opponent Digimon.
    resolve_first_pending(&mut runner);
    runner.auto_resolve().ok();

    assert!(
        runner.battle_area_size(1) < opp_before,
        "accepting clause (f) must delete 1 opponent Digimon"
    );
    assert!(!permanent_exists(&runner, 1, "OPP-DIGI"));
}

/// Clause (f): declining the optional prompt deletes nothing.
#[test]
fn bt24_018_on_opp_security_removed_decline_leaves_digimon() {
    let mut runner = bt24_018_clause_f_runner();
    let styracomon = runner.place_on_field(0, "BT24-018", Some(0));
    runner.place_on_field(1, "OPP-DIGI", Some(0));

    let opp_before = runner.battle_area_size(1);

    runner.game.enqueue_triggered(
        EffectTiming::OnOpponentSecurityRemoved,
        TriggerSource::Permanent(styracomon),
    );
    runner.game.drain_effect_queue();

    // Decline the optional prompt.
    pass_pending(&mut runner);
    runner.auto_resolve().ok();

    assert_eq!(
        runner.battle_area_size(1),
        opp_before,
        "declining clause (f) must leave the opponent's Digimon untouched"
    );
    assert!(permanent_exists(&runner, 1, "OPP-DIGI"));
}

/// Clause (f) negative: when the opponent has no Digimon, no delete prompt
/// installs (the select step has no candidate).
#[test]
fn bt24_018_on_opp_security_removed_no_digimon_no_prompt() {
    let mut runner = bt24_018_clause_f_runner();
    let styracomon = runner.place_on_field(0, "BT24-018", Some(0));
    // No opponent Digimon placed.

    runner.game.enqueue_triggered(
        EffectTiming::OnOpponentSecurityRemoved,
        TriggerSource::Permanent(styracomon),
    );
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection_view().is_none(),
        "clause (f) must not install a prompt when the opponent has no Digimon"
    );
}

/// Clause (f) OPT lockout: two OnOpponentSecurityRemoved events in the same
/// turn produce only one delete prompt. After end_turn the lockout clears.
#[test]
fn bt24_018_clause_f_opt_lockout() {
    let mut runner = bt24_018_clause_f_runner();
    let styracomon = runner.place_on_field(0, "BT24-018", Some(0));
    runner.place_on_field(1, "OPP-DIGI", Some(0));
    runner.place_on_field(1, "OPP-DIGI-2", Some(0));

    // First trigger → prompt installs → resolve (delete one).
    runner.game.enqueue_triggered(
        EffectTiming::OnOpponentSecurityRemoved,
        TriggerSource::Permanent(styracomon),
    );
    runner.game.drain_effect_queue();
    assert!(
        runner.pending_selection_view().is_some(),
        "first clause (f) trigger must install the delete prompt"
    );
    resolve_first_pending(&mut runner);
    runner.auto_resolve().ok();

    // Second trigger same turn → OPT-locked, no prompt.
    runner.game.enqueue_triggered(
        EffectTiming::OnOpponentSecurityRemoved,
        TriggerSource::Permanent(styracomon),
    );
    runner.game.drain_effect_queue();
    assert!(
        runner.pending_selection_view().is_none(),
        "second clause (f) trigger in the same turn must be OPT-locked"
    );

    // After a full turn cycle the OPT slot resets.
    runner.end_turn(); // → P1's turn
    runner.end_turn(); // → back to P0
    runner.game.enqueue_triggered(
        EffectTiming::OnOpponentSecurityRemoved,
        TriggerSource::Permanent(styracomon),
    );
    runner.game.drain_effect_queue();
    assert!(
        runner.pending_selection_view().is_some(),
        "clause (f) OPT lockout must clear after a turn cycle"
    );
}

// ─── SECTION 4 — Clause (g): WhenWouldLeaveBattleArea replacement ────────────

/// Own Dragonkin Digimon would be deleted → replacement prompt installs.
/// Accept + pay cost (delete opp lowest DP) → Dragonkin stays on field.
#[test]
fn bt24_018_own_dragonkin_would_leave_accept_pays_cost_and_stays() {
    let mut runner = bt24_018_clause_g_runner();
    runner.place_on_field(0, "BT24-018", Some(0));
    let ally = runner.place_on_field(0, "ALLY-DRAGONKIN", Some(0));
    runner.place_on_field(1, "LOW-DP", Some(0));

    runner
        .game
        .delete_permanent_with_cause(ally, ReplacementCause::OpponentEffect);

    accept_replacement(&mut runner);
    let cost = runner
        .pending_selection_view()
        .expect("lowest-DP cost prompt");
    assert_eq!(cost.kind, SelectionKind::OppField);
    assert_eq!(cost.valid_action_ids.len(), 1);
    runner
        .execute_action(0, cost.valid_action_ids[0])
        .expect("delete opponent lowest DP");
    runner.auto_resolve().expect("finish replacement");

    assert!(permanent_exists(&runner, 0, "ALLY-DRAGONKIN"));
    assert!(!permanent_exists(&runner, 1, "LOW-DP"));
}

/// Styracomon itself has Dragonkin trait → clause (g) should self-trigger
/// when Styracomon would be deleted.
#[test]
fn bt24_018_itself_as_dragonkin_triggers_own_replacement() {
    let mut runner = bt24_018_clause_g_runner();
    let styracomon = runner.place_on_field(0, "BT24-018", Some(0));
    runner.place_on_field(1, "LOW-DP", Some(0));

    runner
        .game
        .delete_permanent_with_cause(styracomon, ReplacementCause::OpponentEffect);

    accept_replacement(&mut runner);
    let cost = runner
        .pending_selection_view()
        .expect("lowest-DP cost prompt");
    runner
        .execute_action(0, cost.valid_action_ids[0])
        .expect("delete opponent lowest DP");
    runner.auto_resolve().expect("finish replacement");

    assert!(permanent_exists(&runner, 0, "BT24-018"));
    assert!(!permanent_exists(&runner, 1, "LOW-DP"));
}

/// Declining the replacement prompt → own Digimon leaves normally.
#[test]
fn bt24_018_decline_replacement_digimon_leaves_normally() {
    let mut runner = bt24_018_clause_g_runner();
    runner.place_on_field(0, "BT24-018", Some(0));
    let ally = runner.place_on_field(0, "ALLY-DRAGONKIN", Some(0));
    runner.place_on_field(1, "LOW-DP", Some(0));

    runner
        .game
        .delete_permanent_with_cause(ally, ReplacementCause::OpponentEffect);

    let view = runner.pending_selection_view().expect("replacement prompt");
    assert_eq!(view.kind, SelectionKind::Replacement);
    runner.execute_action(0, PASS).expect("decline replacement");
    runner.auto_resolve().expect("finish decline");

    assert!(!permanent_exists(&runner, 0, "ALLY-DRAGONKIN"));
    assert!(permanent_exists(&runner, 1, "LOW-DP"));
}

/// Clause (g) must NOT fire for an opponent's Reptile/Dragonkin Digimon leaving
/// (the "your" in card text limits trigger to own permanents only).
#[test]
fn bt24_018_does_not_fire_for_opponent_dragonkin_leaving() {
    let mut runner = bt24_018_clause_g_runner();
    runner.place_on_field(0, "BT24-018", Some(0));
    let opponent = runner.place_on_field(1, "ALLY-DRAGONKIN", Some(0));
    runner.place_on_field(1, "LOW-DP", Some(0));

    runner
        .game
        .delete_permanent_with_cause(opponent, ReplacementCause::OpponentEffect);

    assert!(
        runner.pending_selection_view().is_none(),
        "BT24-018 only protects its controller's Reptile/Dragonkin Digimon"
    );
    assert!(!permanent_exists(&runner, 1, "ALLY-DRAGONKIN"));
}

/// If opponent has NO Digimon (cost unpayable), clause (g) must not offer
/// the replacement prompt.
#[test]
fn bt24_018_no_cost_target_replacement_does_not_install() {
    let mut runner = bt24_018_clause_g_runner();
    runner.place_on_field(0, "BT24-018", Some(0));
    let ally = runner.place_on_field(0, "ALLY-DRAGONKIN", Some(0));

    runner
        .game
        .delete_permanent_with_cause(ally, ReplacementCause::OpponentEffect);

    assert!(
        runner.pending_selection_view().is_none(),
        "unpayable replacement must not install"
    );
    assert!(!permanent_exists(&runner, 0, "ALLY-DRAGONKIN"));
}

/// OPT lockout: clause (g) fires only once per turn even when two own
/// Reptile/Dragonkin Digimon would leave in the same turn.
#[test]
fn bt24_018_clause_g_opt_lockout() {
    let mut runner = bt24_018_clause_g_runner();
    runner.place_on_field(0, "BT24-018", Some(0));
    let first = runner.place_on_field(0, "ALLY-DRAGONKIN", Some(0));
    let second = runner.place_on_field(0, "ALLY-REPTILE", Some(0));
    runner.place_on_field(1, "LOW-DP", Some(0));
    runner.place_on_field(1, "HIGH-DP", Some(0));

    runner
        .game
        .delete_permanent_with_cause(first, ReplacementCause::OpponentEffect);
    accept_replacement(&mut runner);
    let cost = runner
        .pending_selection_view()
        .expect("first lowest-DP cost prompt");
    runner
        .execute_action(0, cost.valid_action_ids[0])
        .expect("delete first cost body");
    runner.auto_resolve().expect("finish first replacement");

    runner
        .game
        .delete_permanent_with_cause(second, ReplacementCause::OpponentEffect);

    assert!(
        runner.pending_selection_view().is_none(),
        "second matching leave in the same turn is OPT-locked"
    );
    assert!(!permanent_exists(&runner, 0, "ALLY-REPTILE"));
}

/// Clause (g)'s cost — "delete 1 of your opponent's lowest DP Digimon" — must
/// offer ONLY the lowest-DP opponent Digimon, not every opponent Digimon.
/// Regression for the obsolete `dp_lte` aggregate predicate (G-PRED-DP-LTE),
/// which silently passed every candidate; the fix uses `selector: lowest_dp`.
#[test]
fn bt24_018_clause_g_cost_offers_only_lowest_dp_opponent_digimon() {
    let mut runner = bt24_018_clause_g_runner();
    runner.place_on_field(0, "BT24-018", Some(0));
    let ally = runner.place_on_field(0, "ALLY-DRAGONKIN", Some(0));
    runner.place_on_field(1, "LOW-DP", Some(0));
    runner.place_on_field(1, "HIGH-DP", Some(0));

    runner
        .game
        .delete_permanent_with_cause(ally, ReplacementCause::OpponentEffect);

    accept_replacement(&mut runner);
    let cost = runner
        .pending_selection_view()
        .expect("lowest-DP cost prompt");
    assert_eq!(cost.kind, SelectionKind::OppField);
    assert_eq!(
        cost.valid_action_ids.len(),
        1,
        "only the lowest-DP opponent Digimon (LOW-DP, 3000) may be offered, \
         not HIGH-DP (9000)"
    );
    runner
        .execute_action(0, cost.valid_action_ids[0])
        .expect("delete opponent lowest DP");
    runner.auto_resolve().expect("finish replacement");

    assert!(permanent_exists(&runner, 0, "ALLY-DRAGONKIN"));
    assert!(!permanent_exists(&runner, 1, "LOW-DP"));
    assert!(
        permanent_exists(&runner, 1, "HIGH-DP"),
        "the higher-DP opponent Digimon must be untouched"
    );
}

/// When several opponent Digimon are tied for lowest DP, clause (g) must offer
/// ALL of them — the controller chooses, no auto-pick — while still excluding
/// strictly-higher-DP Digimon.
#[test]
fn bt24_018_clause_g_cost_offers_all_tied_lowest_dp_no_auto_pick() {
    let mut runner = bt24_018_clause_g_runner();
    runner.place_on_field(0, "BT24-018", Some(0));
    let ally = runner.place_on_field(0, "ALLY-DRAGONKIN", Some(0));
    runner.place_on_field(1, "LOW-DP", Some(0));
    runner.place_on_field(1, "LOW-DP-2", Some(0));
    runner.place_on_field(1, "HIGH-DP", Some(0));

    runner
        .game
        .delete_permanent_with_cause(ally, ReplacementCause::OpponentEffect);

    accept_replacement(&mut runner);
    let cost = runner
        .pending_selection_view()
        .expect("lowest-DP cost prompt");
    assert_eq!(
        cost.valid_action_ids.len(),
        2,
        "both DP-3000 Digimon are tied for lowest and must both be offered; \
         HIGH-DP (9000) must be excluded; no auto-pick"
    );
    runner
        .execute_action(0, cost.valid_action_ids[0])
        .expect("delete one tied-lowest opponent Digimon");
    runner.auto_resolve().expect("finish replacement");

    assert!(permanent_exists(&runner, 0, "ALLY-DRAGONKIN"));
    assert_eq!(
        runner.battle_area_size(1),
        2,
        "exactly one tied-lowest Digimon deleted; the other low + HIGH-DP remain"
    );
    assert!(permanent_exists(&runner, 1, "HIGH-DP"));
}

// ─── runner builders ─────────────────────────────────────────────────────────

fn bt24_018_clause_e_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT24-018")
        .expect("BT24-018 YAML loads")
        .start()
}

fn bt24_018_clause_f_runner() -> DebugRunner {
    // Decks + security for both players so an end_turn cycle does not deck-out
    // and end the game before the OPT slot resets (see G-OPT-RESET-VIA-ATTACK-CYCLE
    // closure note in qa/resolved-gaps.md).
    DebugRunner::builder()
        .dsl_card("BT24-018")
        .expect("BT24-018 YAML loads")
        .add_card(make_digimon("OPP-DIGI", 5000, &[]))
        .add_card(make_digimon("OPP-DIGI-2", 6000, &[]))
        .add_card(make_digimon("FILLER", 1000, &[]))
        .deck(0, &["FILLER"; 30])
        .deck(1, &["FILLER"; 30])
        .security(0, &["FILLER"; 5])
        .security(1, &["FILLER"; 5])
        .start()
}

fn bt24_018_clause_g_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT24-018")
        .expect("BT24-018 YAML loads")
        .add_card(make_digimon("ALLY-DRAGONKIN", 6000, &["Dragonkin"]))
        .add_card(make_digimon("ALLY-REPTILE", 6000, &["Reptile"]))
        .add_card(make_digimon("LOW-DP", 3000, &[]))
        .add_card(make_digimon("LOW-DP-2", 3000, &[]))
        .add_card(make_digimon("HIGH-DP", 9000, &[]))
        .start()
}

fn accept_replacement(runner: &mut DebugRunner) {
    let view = runner
        .pending_selection_view()
        .expect("replacement accept prompt");
    assert_eq!(view.kind, SelectionKind::Replacement);
    assert!(view.is_optional);
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("accept replacement");
}

// ─── SECTION — Alt digivolve "[Digivolve] While you have [Owen Dreadnought],
//     [Lamiamon]: Cost 6" ─────────────────────────────────────────────────────
//
// Official evo box (card_bundles/BT24-018.md): the alt path digivolves FROM a
// [Lamiamon] (Lv.5 red — the standard Lv.6-red circle does NOT cover it) for
// cost 6, gated on having [Owen Dreadnought] on your battle area. DCGO
// BT24_018.cs: `condition: HasMatchConditionOwnersPermanent(card,
// HasOwenDreadnought)` (IsPermanentExistsOnOwnerBattleArea + TopCard.
// EqualsCardName("Owen Dreadnought")), `permanentCondition: targetPermanent.
// TopCard.EqualsCardName("Lamiamon")` — EXACT names on both gates, and the
// Owen check is a CONDITION, not a second digivolve-from path.

use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::encode_digivolve;
use digimon_engine::enums::{CardColor, CardKind, GamePhase, PlaySource};

/// A Lamiamon-shaped base: Lv.5 red Digimon named exactly "Lamiamon"
/// (BT24-016 / BT21-025 print Lv.5 red Dragonkin/LIBERATOR).
fn lamiamon(id: &str) -> digimon_engine::card_data::CardData {
    let mut c = make_test_card(id, "Lamiamon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(7000);
    c.colors = vec![CardColor::Red];
    c.traits = vec!["Dragonkin".to_string(), "LIBERATOR".to_string()];
    c
}

/// An [Owen Dreadnought] Tamer (BT18-087 / BT21-081 / BT24-082 / EX11-054).
fn owen(id: &str) -> digimon_engine::card_data::CardData {
    let mut c = make_test_card(id, "Owen Dreadnought");
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![CardColor::Red];
    c.traits = vec!["LIBERATOR".to_string()];
    c
}

/// Runner: BT24-018 in hand, the given permanents placed on player 0's field
/// (in order, slots 0..). Returns the runner.
fn alt_path_runner(field: Vec<digimon_engine::card_data::CardData>) -> DebugRunner {
    let ids: Vec<String> = field.iter().map(|c| c.card_id.clone()).collect();
    let mut b = DebugRunner::builder()
        .dsl_card("BT24-018")
        .expect("BT24-018 YAML loads");
    for c in field {
        b = b.add_card(c);
    }
    let mut r = b.hand(0, &["BT24-018"]).memory(10).start();
    r.game.turn_count = 1;
    r.game.current_phase = GamePhase::Main;
    for id in &ids {
        r.place_on_field(0, id, Some(0));
    }
    r
}

/// Positive: with [Owen Dreadnought] on the field, Styracomon digivolves from
/// [Lamiamon] (Lv.5 — unreachable via the standard Lv.6-red circle) for 6.
#[test]
fn bt24_018_digivolves_from_lamiamon_with_owen_for_cost_6() {
    let mut runner = alt_path_runner(vec![lamiamon("LAMIA"), owen("OWEN")]);

    let mask = build_action_mask(&runner.game, 0);
    assert!(
        mask[encode_digivolve(0, 0) as usize] > 0.0,
        "with Owen Dreadnought present, the [Lamiamon] alt digivolve must be \
         offered in the action mask"
    );

    let mem_before = runner.game.memory;
    let proceeded = runner
        .game
        .digivolve_from_hand(0, 0, 0, PlaySource::ByHand);
    assert!(
        proceeded,
        "the single applicable route (alt path, cost 6) must digivolve directly"
    );
    assert_eq!(
        runner.game.players[0].battle_area[0]
            .top_card()
            .card_id(&runner.game.card_data),
        "BT24-018",
        "Styracomon must be stacked on Lamiamon"
    );
    assert_eq!(
        mem_before - runner.game.memory,
        6,
        "the [Lamiamon] alt path costs exactly 6 memory"
    );
}

/// Gate negative: WITHOUT [Owen Dreadnought], the [Lamiamon] alt path is not
/// available — no digivolve route exists onto the Lv.5 Lamiamon at all.
#[test]
fn bt24_018_not_offered_from_lamiamon_without_owen() {
    let mut runner = alt_path_runner(vec![lamiamon("LAMIA")]);

    let mask = build_action_mask(&runner.game, 0);
    assert!(
        mask[encode_digivolve(0, 0) as usize] == 0.0,
        "the alt path is gated on \"While you have [Owen Dreadnought]\" — with \
         no Owen on the field the digivolve must NOT be offered"
    );
    assert!(
        !runner.game.digivolve_from_hand(0, 0, 0, PlaySource::ByHand),
        "executing the digivolve without Owen must be rejected"
    );
}

/// Exact-name negative: DCGO gates the source with EqualsCardName("Lamiamon").
/// A base named "Lamiamon X" must NOT satisfy the alt path.
#[test]
fn bt24_018_not_offered_from_near_name_lamiamon_base() {
    let mut lamia_x = lamiamon("LAMIA-X");
    lamia_x.card_name = "Lamiamon X".to_string();
    let runner = alt_path_runner(vec![lamia_x, owen("OWEN")]);

    let mask = build_action_mask(&runner.game, 0);
    assert!(
        mask[encode_digivolve(0, 0) as usize] == 0.0,
        "\"Lamiamon X\" must NOT satisfy the exact-name [Lamiamon] gate \
         (DCGO EqualsCardName)"
    );
}

/// The Owen requirement is a CONDITION, not a digivolve-from target: the alt
/// path must never offer digivolving Styracomon FROM the Owen Dreadnought
/// Tamer itself.
#[test]
fn bt24_018_not_offered_from_owen_dreadnought_itself() {
    let runner = alt_path_runner(vec![owen("OWEN"), lamiamon("LAMIA")]);

    let mask = build_action_mask(&runner.game, 0);
    assert!(
        mask[encode_digivolve(0, 0) as usize] == 0.0,
        "Owen Dreadnought is the alt path's CONDITION, not its source — \
         digivolving from the Tamer must not be offered"
    );
}
