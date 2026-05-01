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
//! DCGO/Assets/Scripts/CardEffect/BT24/Red/BT24_018.cs (submodule not initialized)
//!
//! # Patterns this test file covers
//! - H3/H5/H-Progress: Progress + Piercing + Blocker keyword grants (declarative)
//! - H-ArmorPurge: ArmorPurge keyword grant (declarative; behavior in keyword_phase_d/)
//! - E2: [When Digivolving] optional select-security trash + optional unsuspend (clause e)
//! - F-OPT: [All Turns][OPT] OnOpponentSecurityRemoved → optional delete opp Digimon (clause f)
//! - F3: [All Turns][OPT] WouldLeave replacement (gaps: G-OPT-TRIGGERED, G-EVENT-TARGET-OWNER)

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};

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
        "clause (e) must be optional (card text says 'You may')"
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

/// Clause (g) is a Replacement declarative clause on when_would_leave_battle_area.
#[test]
fn bt24_018_has_would_leave_battle_area_replacement_clause() {
    let card = compiled("BT24-018");

    let has_replacement = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::Replacement {
                trigger, ..
            }) if trigger == "when_would_leave_battle_area"
        )
    });

    assert!(
        has_replacement,
        "BT24-018 must have a Replacement clause with trigger when_would_leave_battle_area \
         for clause (g)"
    );
}

// ─── SECTION 2 — Clause (e): [When Digivolving] security trash + unsuspend ───

/// Happy path: digivolving into BT24-018 with opponent having security →
/// optional prompt to trash opp security fires; accept + pick → opp loses 1 sec.
/// Then optional unsuspend prompt follows.
///
/// Pending: raw_rust fn bt24_018_trash_selected_security not yet registered.
#[test]
#[ignore = "pending: raw_rust fn bt24_018_trash_selected_security (G-TRASH-SELECTED-SECURITY)"]
fn bt24_018_when_digivolving_trash_security_accept() {
    // Scaffolding (pending G-TRASH-SELECTED-SECURITY):
    //
    // 1. Place a Lv6 base on P0's field.
    // 2. Give P1 3 security cards.
    // 3. Digivolve BT24-018 on top of the Lv6 base (digivolve_from_hand).
    // 4. Clause (e): optional prompt installs → accept.
    // 5. select_security prompt installs → pick the first legal card.
    // 6. raw_rust step bt24_018_trash_selected_security executes →
    //    P1 loses exactly 1 security card.
    // 7. Optional unsuspend sub-prompt installs.
    todo!("pending G-TRASH-SELECTED-SECURITY: bt24_018_trash_selected_security raw_rust fn")
}

/// Declining the optional clause (e) prompt → no security trashed, no
/// unsuspend prompt (optional-decline short-circuits the entire process).
#[test]
#[ignore = "pending: raw_rust fn bt24_018_trash_selected_security (G-TRASH-SELECTED-SECURITY)"]
fn bt24_018_when_digivolving_decline_trash_skips_chain() {
    // Scaffolding (pending G-TRASH-SELECTED-SECURITY):
    //
    // 1. Setup as above but with P1 having 2 security.
    // 2. Digivolve BT24-018.
    // 3. PASS the optional clause prompt → no further selection installs.
    // 4. P1 security count unchanged.
    todo!("pending G-TRASH-SELECTED-SECURITY: bt24_018_trash_selected_security raw_rust fn")
}

/// Accepting trash but then accepting the optional unsuspend sub-prompt
/// unsuspends BT24-018 on the field.
#[test]
#[ignore = "pending: raw_rust fn bt24_018_trash_selected_security (G-TRASH-SELECTED-SECURITY)"]
fn bt24_018_when_digivolving_accept_trash_then_accept_unsuspend() {
    // Scaffolding (pending G-TRASH-SELECTED-SECURITY):
    //
    // 1. Setup: P1 has 2 security. BT24-018 digivolves and is suspended.
    // 2. Accept trash prompt → pick security → trash fires.
    // 3. Optional unsuspend prompt installs → accept → BT24-018 unsuspends.
    todo!("pending G-TRASH-SELECTED-SECURITY: bt24_018_trash_selected_security raw_rust fn")
}

// ─── SECTION 3 — Clause (f): OnOpponentSecurityRemoved → delete Digimon ──────

/// When opponent's security is removed, clause (f) fires with an optional
/// prompt to delete 1 of their Digimon. Accept + pick → target deleted.
#[test]
#[ignore = "pending: G-OPT-TRIGGERED — triggered OPT on OnOpponentSecurityRemoved dispatch"]
fn bt24_018_on_opp_security_removed_delete_digimon_accept() {
    // Scaffolding (pending G-OPT-TRIGGERED):
    //
    // 1. Place BT24-018 on P0's field and an opp Digimon on P1's field.
    // 2. Give P1 2 security cards.
    // 3. Call ctx.trash_top_security(1) to fire OnOpponentSecurityRemoved.
    // 4. Clause (f) optional prompt installs → resolve_first_pending().
    // 5. select_opponent_permanent prompt for which Digimon to delete → pick.
    // 6. Opponent's Digimon count decreases by 1.
    todo!("pending G-OPT-TRIGGERED: clause f test")
}

/// Declining clause (f) optional prompt → no Digimon deleted.
#[test]
#[ignore = "pending: G-OPT-TRIGGERED — triggered OPT on OnOpponentSecurityRemoved dispatch"]
fn bt24_018_on_opp_security_removed_decline_leaves_digimon() {
    // Scaffolding (pending G-OPT-TRIGGERED):
    //
    // 1. Same setup as above.
    // 2. OnOpponentSecurityRemoved fires → optional prompt → PASS.
    // 3. Opponent's Digimon count unchanged.
    todo!("pending G-OPT-TRIGGERED: clause f decline test")
}

/// OPT lockout: clause (f) fires only once per turn even when multiple
/// security cards are removed in the same turn.
#[test]
#[ignore = "pending: G-OPT-TRIGGERED — OPT hash tracking for triggered observer clauses"]
fn bt24_018_clause_f_opt_lockout() {
    // Scaffolding (pending G-OPT-TRIGGERED):
    //
    // Two OnOpponentSecurityRemoved events in the same turn must produce
    // only one optional delete prompt (OPT locks out after the first).
    todo!("pending G-OPT-TRIGGERED: clause f OPT lockout")
}

// ─── SECTION 4 — Clause (g): WouldLeave replacement ─────────────────────────

/// Own Dragonkin Digimon would be deleted → replacement prompt installs.
/// Accept + pay cost (delete opp lowest DP) → Dragonkin stays on field.
#[test]
#[ignore = "pending: G-OPT-TRIGGERED + G-EVENT-TARGET-OWNER + raw_rust bt24_018_would_leave_replacement"]
fn bt24_018_own_dragonkin_would_leave_accept_pays_cost_and_stays() {
    // Scaffolding (pending both gaps):
    //
    // 1. Place BT24-018 (source) on P0's field.
    // 2. Place a Dragonkin ally on P0's field.
    // 3. Place a 3000 DP Digimon on P1's field.
    // 4. delete_permanent_with_effects on the Dragonkin ally.
    // 5. Replacement prompt installs (optional, REPLACEMENT_ACCEPT action).
    // 6. Accept → delete_permanent of P1's 3000 DP Digimon.
    // 7. Ally stays on P0's field; P1 Digimon count = 0.
    todo!("pending G-OPT-TRIGGERED + G-EVENT-TARGET-OWNER")
}

/// Styracomon itself has Dragonkin trait → clause (g) should self-trigger
/// when Styracomon would be deleted.
#[test]
#[ignore = "pending: G-OPT-TRIGGERED + G-EVENT-TARGET-OWNER + raw_rust bt24_018_would_leave_replacement"]
fn bt24_018_itself_as_dragonkin_triggers_own_replacement() {
    // Scaffolding (pending both gaps):
    //
    // 1. Place BT24-018 on P0's field (it IS Dragonkin).
    // 2. Place a 3000 DP Digimon on P1's field.
    // 3. delete_permanent_with_effects on BT24-018 itself.
    // 4. Replacement prompt installs → accept.
    // 5. P1 Digimon deleted; BT24-018 stays on P0's field.
    todo!("pending G-OPT-TRIGGERED + G-EVENT-TARGET-OWNER")
}

/// Declining the replacement prompt → own Digimon leaves normally.
#[test]
#[ignore = "pending: G-OPT-TRIGGERED + G-EVENT-TARGET-OWNER + raw_rust bt24_018_would_leave_replacement"]
fn bt24_018_decline_replacement_digimon_leaves_normally() {
    // Scaffolding (pending both gaps):
    //
    // 1. Place BT24-018 + Dragonkin ally on P0; opp Digimon on P1.
    // 2. delete_permanent_with_effects on ally → prompt → PASS.
    // 3. Ally deleted; P1 Digimon count unchanged.
    todo!("pending G-OPT-TRIGGERED + G-EVENT-TARGET-OWNER")
}

/// Clause (g) must NOT fire for an opponent's Reptile/Dragonkin Digimon leaving
/// (the "your" in card text limits trigger to own permanents only).
#[test]
#[ignore = "pending: G-OPT-TRIGGERED + G-EVENT-TARGET-OWNER + raw_rust bt24_018_would_leave_replacement"]
fn bt24_018_does_not_fire_for_opponent_dragonkin_leaving() {
    // Scaffolding (pending both gaps):
    //
    // 1. Place BT24-018 on P0; opponent Dragonkin on P1.
    // 2. delete_permanent_with_effects on P1's Dragonkin.
    // 3. No replacement prompt → P1 Dragonkin deleted normally.
    todo!("pending G-OPT-TRIGGERED + G-EVENT-TARGET-OWNER")
}

/// If opponent has NO Digimon (cost unpayable), clause (g) must not offer
/// the replacement prompt.
#[test]
#[ignore = "pending: G-OPT-TRIGGERED + G-EVENT-TARGET-OWNER + raw_rust bt24_018_would_leave_replacement"]
fn bt24_018_no_cost_target_replacement_does_not_install() {
    // Scaffolding (pending both gaps):
    //
    // 1. Place BT24-018 + Dragonkin ally on P0. P1 has NO Digimon.
    // 2. delete_permanent_with_effects on ally.
    // 3. No prompt installs; ally deleted normally (can't pay cost).
    todo!("pending G-OPT-TRIGGERED + G-EVENT-TARGET-OWNER")
}

/// OPT lockout: clause (g) fires only once per turn even when two own
/// Reptile/Dragonkin Digimon would leave in the same turn.
#[test]
#[ignore = "pending: G-OPT-TRIGGERED — OPT on replacement clauses not yet implemented"]
fn bt24_018_clause_g_opt_lockout() {
    // Scaffolding (pending G-OPT-TRIGGERED):
    //
    // 1. Two own Dragonkin Digimon + two opp Digimon.
    // 2. Delete first ally → prompt → accept → cost paid.
    // 3. Delete second ally in same turn → NO prompt (OPT locked).
    // 4. End turn + new turn → delete ally → prompt returns.
    todo!("pending G-OPT-TRIGGERED: clause g OPT lockout")
}
