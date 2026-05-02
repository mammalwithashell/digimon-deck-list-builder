//! BT24-012 Dimetromon — Digimon, Lv4, Red, DP4000, Cost4.
//!
//! # Card text (cards.json)
//!
//! `＜Blocker＞`
//! `[All Turns] When any of your other Digimon with the [Reptile] or [Dragonkin]`
//! `trait would leave the battle area by your opponent's effects, by returning this`
//! `Digimon to the hand, they don't leave.`
//!
//! **Inherited:**
//! `[Your Turn] [Once Per Turn] When your opponent's security stack is removed from,`
//! `gain 1 memory.`
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT24/Red/BT24_012.cs
//!
//! # Patterns this test file covers
//! - H5: Blocker keyword grant (declarative GrantKeyword)
//! - F3: [All Turns] WouldLeaveBattleArea replacement — "other" Digimon scope,
//!       cost = return self to hand (raw_rust: G-EVENT-TARGET-OWNER gap)
//! - G-INHERITED-DISPATCH: inherited on_opponent_security_removed OPT clause
//!   (structural YAML ships; behavioral tests #[ignore]'d for G-INHERITED-DISPATCH
//!   and G-OPT-TRIGGERED)
//!
//! # Known gaps
//! - **G-EVENT-TARGET-OWNER**: no DSL predicate to filter the leaving permanent
//!   by controller (must be "your" Digimon) — raw_rust workaround owns this check.
//!   Additionally, no predicate for "by your opponent's effects" (removal cause
//!   attribution). The raw_rust fn implements both gates.
//! - **G-INHERITED-DISPATCH**: inherited `on_opponent_security_removed` firing
//!   is not wired up for digivolution-stack sources; behavioral tests for
//!   clause (c) are `#[ignore]`.
//! - **G-OPT-TRIGGERED**: OPT lockout enforcement on triggered effects is not
//!   wired; inherited clause (c) OPT lockout test is `#[ignore]`.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};

use super::super::dsl_card_data::compiled;

// ─── §1  Structural assertions ────────────────────────────────────────────────

/// Dimetromon compiles with a GrantKeyword clause for Blocker.
#[test]
fn bt24_012_has_blocker_grant_keyword() {
    let card = compiled("BT24-012");

    let keyword_grants: Vec<String> = card
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

    assert!(
        keyword_grants.iter().any(|k| k == "Blocker"),
        "BT24-012 must have a GrantKeyword clause for Blocker; got: {:?}",
        keyword_grants
    );
}

/// Dimetromon has one RawRust declarative clause for the [All Turns]
/// would-leave-battle-area replacement (clause b).
#[test]
fn bt24_012_has_raw_rust_replacement_clause() {
    let card = compiled("BT24-012");

    let raw_rust_clauses: Vec<String> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::RawRust { fn_name, .. }) => {
                Some(fn_name.clone())
            }
            _ => None,
        })
        .collect();

    assert!(
        raw_rust_clauses
            .iter()
            .any(|n| n == "bt24_012_would_leave_replacement"),
        "BT24-012 must have a RawRust declarative clause named \
         'bt24_012_would_leave_replacement' for clause (b); got: {:?}",
        raw_rust_clauses
    );
}

/// Dimetromon has one inherited triggered clause: on_opponent_security_removed,
/// Once Per Turn (clause c).
#[test]
fn bt24_012_has_inherited_on_opponent_security_removed_clause() {
    let card = compiled("BT24-012");

    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) if t.scope == CompiledScope::Inherited => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnOpponentSecurityRemoved));

    let clause = clause.expect(
        "BT24-012 must have an inherited triggered clause on OnOpponentSecurityRemoved (clause c)",
    );

    assert!(clause.once_per_turn, "clause (c) must be [Once Per Turn]");

    // Clause (c) is NOT optional: "When ... gain 1 memory" is mandatory.
    assert!(
        !clause.optional,
        "clause (c) is mandatory (no 'you may' in printed text); optional must be false"
    );
}

/// Dimetromon has exactly two face-up effects: one GrantKeyword (Blocker)
/// and one RawRust (clause b). It has one inherited effect: clause (c).
#[test]
fn bt24_012_total_clause_count() {
    let card = compiled("BT24-012");

    let face_up_count = card
        .effects
        .iter()
        .filter(
            |c| !matches!(c, CompiledClause::Triggered(t) if t.scope == CompiledScope::Inherited),
        )
        .count();

    let inherited_count = card
        .effects
        .iter()
        .filter(
            |c| matches!(c, CompiledClause::Triggered(t) if t.scope == CompiledScope::Inherited),
        )
        .count();

    assert_eq!(
        face_up_count, 2,
        "expected 2 face-up clauses (GrantKeyword Blocker + RawRust replacement); got {}",
        face_up_count
    );
    assert_eq!(
        inherited_count, 1,
        "expected 1 inherited clause (on_opponent_security_removed OPT); got {}",
        inherited_count
    );
}

// ─── §2  Condition gating ─────────────────────────────────────────────────────

/// Blocker positive: when BT24-012 is on field, it has the Blocker keyword active.
#[test]
fn bt24_012_blocker_keyword_active_on_field() {
    use digimon_engine::enums::Keyword;

    let mut runner = DebugRunner::builder()
        .dsl_card("BT24-012")
        .expect("BT24-012 YAML parses without error")
        .hand(0, &["BT24-012"])
        .memory(6)
        .start();

    let perm = runner.place_on_field(0, "BT24-012", None);

    assert!(
        runner.game.has_keyword(perm, Keyword::Blocker),
        "BT24-012 must have Blocker keyword active when on field"
    );
}

/// Blocker negative: a friendly Digimon on field does NOT have Blocker
/// when BT24-012 is NOT on field (Blocker is not a global aura — it grants
/// Blocker to THIS card only).
#[test]
fn bt24_012_blocker_does_not_leak_to_other_digimon() {
    use digimon_engine::enums::Keyword;

    // A dummy Digimon with no inherent Blocker.
    let dummy = make_test_card("DUMMY-LV4", "DummyLv4");

    let mut runner = DebugRunner::builder()
        .dsl_card("BT24-012")
        .expect("BT24-012 YAML parses without error")
        .add_card(dummy)
        .hand(0, &["DUMMY-LV4"])
        .memory(6)
        .start();

    // Only DUMMY-LV4 is on field — BT24-012 is not on field.
    let dummy_perm = runner.place_on_field(0, "DUMMY-LV4", None);

    assert!(
        !runner.game.has_keyword(dummy_perm, Keyword::Blocker),
        "Dummy Digimon must NOT have Blocker when BT24-012 is not on field"
    );
}

// ─── §3  Behavioral: clause (b) replacement — would-leave guard ───────────────
//
// All clause-(b) tests are #[ignore]'d pending:
//   - G-EVENT-TARGET-OWNER: no cause filter for "by opponent's effects"
//   - The raw_rust fn (bt24_012_would_leave_replacement) must be registered
//     and fully implement: self-exclusion, Reptile/Dragonkin trait check,
//     optional bounce prompt, and cancel_replacement on accept.

/// Clause (b) positive: when an allied Reptile Digimon would leave by opponent
/// effect, the replacement fires an optional prompt.
#[test]
#[ignore = "pending: G-EVENT-TARGET-OWNER — removal cause filter (opponent effect vs. own effect) + raw_rust fn bt24_012_would_leave_replacement"]
fn bt24_012_allied_reptile_would_leave_by_opp_effect_triggers_prompt() {
    // Setup:
    // 1. P0: BT24-012 (Dimetromon) on field.
    // 2. P0: A Reptile ally Digimon on field.
    // 3. P1 effect deletes the ally.
    // 4. Assert: optional replacement prompt installs.
    // 5. Accept: Dimetromon returns to P0's hand; ally stays on field.
    todo!("pending G-EVENT-TARGET-OWNER + raw_rust fn")
}

/// Clause (b) positive: accepting the replacement pays the cost — Dimetromon
/// moves to hand, ally stays on field.
#[test]
#[ignore = "pending: G-EVENT-TARGET-OWNER + raw_rust fn bt24_012_would_leave_replacement"]
fn bt24_012_accept_replacement_bounces_self_and_ally_stays() {
    todo!("pending G-EVENT-TARGET-OWNER: accept branch — self bounce + ally stays")
}

/// Clause (b) positive (Dragonkin variant): same logic for a Dragonkin ally.
#[test]
#[ignore = "pending: G-EVENT-TARGET-OWNER + raw_rust fn bt24_012_would_leave_replacement"]
fn bt24_012_dragonkin_ally_would_leave_triggers_prompt() {
    todo!("pending G-EVENT-TARGET-OWNER: Dragonkin trait coverage")
}

/// Clause (b) negative: a Digimon WITHOUT Reptile or Dragonkin trait would
/// leave — replacement must NOT fire.
#[test]
#[ignore = "pending: raw_rust fn bt24_012_would_leave_replacement (trait filter)"]
fn bt24_012_non_trait_ally_leaving_no_replacement() {
    // An Aquan or plain Digimon being deleted by opponent effect should
    // not trigger the replacement prompt.
    todo!("pending raw_rust trait filter")
}

/// Clause (b) negative: Dimetromon ITSELF would leave — must NOT fire
/// (card text says "other Digimon").
#[test]
#[ignore = "pending: raw_rust fn bt24_012_would_leave_replacement (self-exclusion)"]
fn bt24_012_self_leaving_does_not_trigger_replacement() {
    // Even though Dimetromon has Reptile trait, "other Digimon" excludes self.
    todo!("pending raw_rust self-exclusion")
}

/// Clause (b) negative: declining the optional replacement — ally still leaves,
/// Dimetromon stays on field.
#[test]
#[ignore = "pending: G-EVENT-TARGET-OWNER + raw_rust fn bt24_012_would_leave_replacement"]
fn bt24_012_decline_replacement_ally_leaves_self_stays() {
    // PASS the optional prompt → ally leaves, Dimetromon hand size unchanged.
    todo!("pending G-EVENT-TARGET-OWNER: decline branch")
}

/// Clause (b) negative: ally would leave by OWN effect (e.g. player's own
/// deletion effect) — the replacement must NOT fire.
/// ("by your opponent's effects" gate — G-EVENT-TARGET-OWNER gap blocks
/// implementing this cleanly without cause attribution).
#[test]
#[ignore = "pending: G-EVENT-TARGET-OWNER — cause attribution required to gate out own-effect removals"]
fn bt24_012_own_effect_removal_does_not_trigger_replacement() {
    todo!("pending G-EVENT-TARGET-OWNER: own-effect exclusion")
}

// ─── §4  Behavioral: clause (c) inherited on_opponent_security_removed ────────
//
// All clause-(c) behavioral tests are #[ignore]'d pending:
//   - G-INHERITED-DISPATCH: inherited triggers not fired from digivolution stack
//   - G-OPT-TRIGGERED: OPT lockout enforcement for triggered effects

/// Clause (c) positive: when this card is under a Digimon and opponent's
/// security is removed on P0's turn, P0 gains 1 memory.
#[test]
#[ignore = "pending: G-INHERITED-DISPATCH — inherited triggered effects not fired from digivolution stack"]
fn bt24_012_inherited_gain_memory_on_opp_security_removed() {
    // Setup:
    // 1. Place BT24-012 as a digivolution source under a Lv5 Digimon for P0.
    // 2. Trash P1's top security card (fires OnOpponentSecurityRemoved for P0).
    // 3. Assert: P0 memory +1.
    todo!("pending G-INHERITED-DISPATCH")
}

/// Clause (c) negative: when BT24-012 is standalone on field (not under
/// another Digimon), the INHERITED clause does not fire.
#[test]
#[ignore = "pending: G-INHERITED-DISPATCH — inherited dispatch required first"]
fn bt24_012_inherited_clause_does_not_fire_when_not_in_stack() {
    // BT24-012 is face-up on field, not a digivolution source.
    // Opponent security removed → the inherited clause should NOT fire
    // (printed text says "Inherited" — only active when below another Digimon).
    todo!("pending G-INHERITED-DISPATCH")
}

/// Clause (c) OPT lockout: fires only once per turn.
#[test]
#[ignore = "pending: G-OPT-TRIGGERED — OPT hash tracking for triggered observer clauses"]
fn bt24_012_inherited_clause_opt_blocks_second_activation() {
    // Two successive security removals in the same turn:
    // only one memory gain should fire (OPT locks after first).
    todo!("pending G-OPT-TRIGGERED")
}

/// Clause (c) OPT resets after end of turn.
#[test]
#[ignore = "pending: G-OPT-TRIGGERED — OPT reset across turns"]
fn bt24_012_inherited_clause_opt_resets_after_turn() {
    todo!("pending G-OPT-TRIGGERED: OPT reset")
}

/// Clause (c) is [Your Turn] scoped: must NOT fire on the opponent's turn.
#[test]
#[ignore = "pending: G-INHERITED-DISPATCH — inherited dispatch required first"]
fn bt24_012_inherited_clause_does_not_fire_on_opponent_turn() {
    // On P1's turn, removing P1's security should not give P0 memory gain.
    // P0 is out of turn; the [Your Turn] gate prevents firing.
    todo!("pending G-INHERITED-DISPATCH + your_turn gate")
}
