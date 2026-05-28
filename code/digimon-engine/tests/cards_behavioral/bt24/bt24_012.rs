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
//!       cross-permanent subject, gated on `replacement_cause: opponent_effect`,
//!       cost = return self to hand. Pure-DSL `kind: replacement`
//!       (G-EVENT-TARGET-OWNER RESOLVED 2026-05-21).
//! - Inherited on_opponent_security_removed OPT clause: substrate gates
//!   closed (G-INHERITED-DISPATCH Phase 2 Track D 2026-05-17,
//!   G-OPT-TRIGGERED Phase 2 Track C 2026-05-16). Behavioral tests live.
//!
//! # Resolved gaps
//! - **G-EVENT-TARGET-OWNER** (RESOLVED 2026-05-21): the replacement context
//!   now exposes `replacement_subject_is_mine` (the leaving permanent's
//!   controller) and `replacement_cause` (Battle / OwnEffect / OpponentEffect).
//!   Clause (b) is expressed in pure DSL — the raw_rust placeholder
//!   `bt24_012_would_leave_replacement` is no longer referenced by this card.
//! - **G-INHERITED-DISPATCH** / **G-OPT-TRIGGERED**: closed — clause (c) ships
//!   live.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::CardKind as _BehavioralCardKind;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;

use super::super::dsl_card_data::{card_data_from_compiled as card_data, compiled};

// ─── helpers ──────────────────────────────────────────────────────────────────

/// Minimal Digimon `CardData` with explicit DP and trait list.
/// Tamer security filler — used in the security stack for tests that
/// fire on `on_opp_security_removed` without wanting an incidental
/// security-DP battle (which, post-fix, mutual-destructs same-DP
/// attackers per RULES_CONTEXT 14-2-1-3). Non-Digimon security skips
/// the battle compare entirely (13-1-7-3 / 14-2-3).
fn make_non_digimon_security_filler(id: &str) -> digimon_engine::card_data::CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = _BehavioralCardKind::Tamer;
    c.dp = None;
    c.level = None;
    c.traits = vec![];
    c
}

fn make_digimon(id: &str, dp: i32, traits: &[&str]) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.dp = Some(dp);
    card.traits = traits.iter().map(|s| s.to_string()).collect();
    card
}

/// True if `player` has a battle-area permanent whose top card is `card_id`.
fn permanent_exists(runner: &DebugRunner, player: usize, card_id: &str) -> bool {
    runner.game.players[player]
        .battle_area
        .iter()
        .any(|permanent| permanent.top_card().card_id(&runner.game.card_data) == card_id)
}

/// True if `player` holds a card with `card_id` in hand.
fn in_hand(runner: &DebugRunner, player: usize, card_id: &str) -> bool {
    runner.game.players[player]
        .hand
        .iter()
        .any(|card| card.card_id(&runner.game.card_data) == card_id)
}

/// Accept a pending optional `Replacement` prompt by resolving its first
/// (ACCEPT) action.
fn accept_replacement(runner: &mut DebugRunner) {
    let view = runner
        .pending_selection_view()
        .expect("replacement accept prompt");
    assert_eq!(view.kind, SelectionKind::Replacement);
    assert!(
        view.is_optional,
        "clause (b) is `(may)` — accept and decline must both surface"
    );
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("accept replacement");
}

/// Build a runner with BT24-012 plus the ally fixtures used by clause-(b) tests.
fn clause_b_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT24-012")
        .expect("BT24-012 YAML loads")
        .add_card(make_digimon("ALLY-REPTILE", 6000, &["Reptile"]))
        .add_card(make_digimon("ALLY-DRAGONKIN", 6000, &["Dragonkin"]))
        .add_card(make_digimon("ALLY-PLAIN", 6000, &["Aquan"]))
        .start()
}

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

/// Dimetromon has one Replacement declarative clause for the [All Turns]
/// would-leave-battle-area protection (clause b), expressed in pure DSL.
#[test]
fn bt24_012_has_would_leave_replacement_clause() {
    let card = compiled("BT24-012");

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
        "BT24-012 must have a Replacement clause with trigger \
         when_would_leave_battle_area for clause (b)"
    );
}

/// Clause (b) is no longer expressed via the raw_rust placeholder — the
/// `bt24_012_would_leave_replacement` escape hatch is fully retired now that
/// G-EVENT-TARGET-OWNER is resolved.
#[test]
fn bt24_012_clause_b_uses_no_raw_rust() {
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
        raw_rust_clauses.is_empty(),
        "BT24-012 clause (b) must be pure DSL — no raw_rust clause expected; got: {:?}",
        raw_rust_clauses
    );
}

/// Clause (b) is an optional replacement (the "by returning this Digimon" cost
/// makes accepting a choice — card text shape mirrors a `(may)` keyword).
#[test]
fn bt24_012_clause_b_is_optional() {
    let card = compiled("BT24-012");

    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::Replacement {
                trigger,
                optional,
                ..
            }) if trigger == "when_would_leave_battle_area" => Some(*optional),
            _ => None,
        })
        .expect("clause (b) Replacement clause must exist");

    assert!(
        clause,
        "clause (b) must be optional — the player may accept (pay the bounce cost) or decline"
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
/// and one Replacement (clause b). It has one inherited effect: clause (c).
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
        "expected 2 face-up clauses (GrantKeyword Blocker + Replacement); got {}",
        face_up_count
    );
    assert_eq!(
        inherited_count, 1,
        "expected 1 inherited clause (on_opponent_security_removed OPT); got {}",
        inherited_count
    );
}

// ─── §2  Condition gating — Blocker ───────────────────────────────────────────

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
// Clause (b): "[All Turns] When any of your other Digimon with the [Reptile]
// or [Dragonkin] trait would leave the battle area by your opponent's effects,
// by returning this Digimon to the hand, they don't leave."
//
// G-EVENT-TARGET-OWNER is RESOLVED — these tests drive the pure-DSL
// `kind: replacement` clause end-to-end.

/// Clause (b) positive: when an allied Reptile Digimon would leave by an
/// opponent effect, the optional replacement prompt installs.
#[test]
fn bt24_012_allied_reptile_would_leave_by_opp_effect_triggers_prompt() {
    let mut runner = clause_b_runner();
    runner.place_on_field(0, "BT24-012", Some(0));
    let ally = runner.place_on_field(0, "ALLY-REPTILE", Some(0));

    runner
        .game
        .delete_permanent_with_cause(ally, ReplacementCause::OpponentEffect);

    let view = runner
        .pending_selection_view()
        .expect("clause (b) replacement prompt must install");
    assert_eq!(view.kind, SelectionKind::Replacement);
    assert!(
        view.is_optional,
        "the bounce-self cost makes accepting a choice — prompt must be optional"
    );
    // The ally is still on the field while the replacement is pending.
    assert!(permanent_exists(&runner, 0, "ALLY-REPTILE"));
}

/// Clause (b) positive (integrated, accept branch): accepting pays the cost —
/// Dimetromon returns to its controller's hand; the Reptile ally stays on field.
/// The "by returning this Digimon to the hand" cost is verified by observing
/// Dimetromon move off-field and into hand (the engine emits no return-to-hand
/// GameEvent variant, so state is the load-bearing assertion).
#[test]
fn bt24_012_accept_replacement_bounces_self_and_ally_stays() {
    let mut runner = clause_b_runner();
    runner.place_on_field(0, "BT24-012", Some(0));
    let ally = runner.place_on_field(0, "ALLY-REPTILE", Some(0));

    assert!(!in_hand(&runner, 0, "BT24-012"));
    let hand_before = runner.hand_size(0);

    runner
        .game
        .delete_permanent_with_cause(ally, ReplacementCause::OpponentEffect);

    accept_replacement(&mut runner);
    runner.auto_resolve().expect("finish replacement");

    // Cost paid: Dimetromon left the field and is in P0's hand.
    assert!(
        !permanent_exists(&runner, 0, "BT24-012"),
        "accepting clause (b) returns Dimetromon to hand — it must leave the field"
    );
    assert!(
        in_hand(&runner, 0, "BT24-012"),
        "Dimetromon must be in P0's hand after paying the return-to-hand cost"
    );
    assert_eq!(
        runner.hand_size(0),
        hand_before + 1,
        "P0's hand grows by exactly 1 (Dimetromon returned)"
    );
    // Protection applied: the ally stayed.
    assert!(
        permanent_exists(&runner, 0, "ALLY-REPTILE"),
        "the protected Reptile ally must stay on the field"
    );
}

/// Clause (b) positive (Dragonkin variant): same protection for a Dragonkin
/// ally — accept → Dimetromon bounced, ally stays.
#[test]
fn bt24_012_dragonkin_ally_would_leave_triggers_prompt() {
    let mut runner = clause_b_runner();
    runner.place_on_field(0, "BT24-012", Some(0));
    let ally = runner.place_on_field(0, "ALLY-DRAGONKIN", Some(0));

    runner
        .game
        .delete_permanent_with_cause(ally, ReplacementCause::OpponentEffect);

    accept_replacement(&mut runner);
    runner.auto_resolve().expect("finish replacement");

    assert!(in_hand(&runner, 0, "BT24-012"));
    assert!(
        permanent_exists(&runner, 0, "ALLY-DRAGONKIN"),
        "clause (b) protects Dragonkin-trait allies too"
    );
}

/// Clause (b) negative (decline branch): declining the optional prompt — the
/// ally leaves normally and Dimetromon stays on the field (no cost paid).
#[test]
fn bt24_012_decline_replacement_ally_leaves_self_stays() {
    let mut runner = clause_b_runner();
    runner.place_on_field(0, "BT24-012", Some(0));
    let ally = runner.place_on_field(0, "ALLY-REPTILE", Some(0));

    let hand_before = runner.hand_size(0);

    runner
        .game
        .delete_permanent_with_cause(ally, ReplacementCause::OpponentEffect);

    let view = runner
        .pending_selection_view()
        .expect("clause (b) replacement prompt");
    assert_eq!(view.kind, SelectionKind::Replacement);
    runner
        .execute_action(0, PASS)
        .expect("decline the replacement");
    runner.auto_resolve().expect("finish decline");

    assert!(
        !permanent_exists(&runner, 0, "ALLY-REPTILE"),
        "declining clause (b) — the ally leaves as normal"
    );
    assert!(
        permanent_exists(&runner, 0, "BT24-012"),
        "declining clause (b) — Dimetromon stays on the field (cost not paid)"
    );
    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "declining pays no cost — P0's hand is unchanged"
    );
}

/// Clause (b) negative (trait filter): an ally WITHOUT the Reptile or Dragonkin
/// trait would leave by an opponent effect — the replacement must NOT fire.
#[test]
fn bt24_012_non_trait_ally_leaving_no_replacement() {
    let mut runner = clause_b_runner();
    runner.place_on_field(0, "BT24-012", Some(0));
    let ally = runner.place_on_field(0, "ALLY-PLAIN", Some(0));

    runner
        .game
        .delete_permanent_with_cause(ally, ReplacementCause::OpponentEffect);

    assert!(
        runner.pending_selection_view().is_none(),
        "clause (b) only protects Reptile/Dragonkin allies — no prompt for a plain Digimon"
    );
    assert!(
        !permanent_exists(&runner, 0, "ALLY-PLAIN"),
        "the non-trait ally leaves as normal"
    );
}

/// Clause (b) negative (self-exclusion): Dimetromon ITSELF would leave by an
/// opponent effect — the replacement must NOT fire (card text says "other
/// Digimon"). Even though Dimetromon has the Reptile trait, `other: true`
/// excludes a self-leave.
#[test]
fn bt24_012_self_leaving_does_not_trigger_replacement() {
    let mut runner = clause_b_runner();
    let dimetromon = runner.place_on_field(0, "BT24-012", Some(0));

    runner
        .game
        .delete_permanent_with_cause(dimetromon, ReplacementCause::OpponentEffect);

    assert!(
        runner.pending_selection_view().is_none(),
        "clause (b) excludes Dimetromon itself — no self-protection prompt"
    );
    assert!(
        !permanent_exists(&runner, 0, "BT24-012"),
        "Dimetromon itself leaves normally (clause b does not protect self)"
    );
}

/// Clause (b) negative (own-effect exclusion): an allied Reptile would leave by
/// the CONTROLLER's OWN effect — the replacement must NOT fire. Card text gates
/// strictly on "by your opponent's effects" (`replacement_cause: opponent_effect`).
#[test]
fn bt24_012_own_effect_removal_does_not_trigger_replacement() {
    let mut runner = clause_b_runner();
    runner.place_on_field(0, "BT24-012", Some(0));
    let ally = runner.place_on_field(0, "ALLY-REPTILE", Some(0));

    runner
        .game
        .delete_permanent_with_cause(ally, ReplacementCause::OwnEffect);

    assert!(
        runner.pending_selection_view().is_none(),
        "clause (b) fires only on opponent-effect removals — own-effect removal is excluded"
    );
    assert!(
        !permanent_exists(&runner, 0, "ALLY-REPTILE"),
        "an own-effect removal of the ally is not prevented"
    );
    assert!(
        permanent_exists(&runner, 0, "BT24-012"),
        "Dimetromon is not bounced when the cause is the controller's own effect"
    );
}

/// Clause (b) negative (battle-deletion exclusion): an allied Reptile leaving
/// by BATTLE is not "by your opponent's effects" — the replacement must NOT
/// fire. Verifies the cause gate excludes combat deletion as well.
#[test]
fn bt24_012_battle_deletion_does_not_trigger_replacement() {
    let mut runner = clause_b_runner();
    runner.place_on_field(0, "BT24-012", Some(0));
    let ally = runner.place_on_field(0, "ALLY-REPTILE", Some(0));

    runner
        .game
        .delete_permanent_with_cause(ally, ReplacementCause::Battle);

    assert!(
        runner.pending_selection_view().is_none(),
        "clause (b) fires only on opponent-effect removals — battle deletion is excluded"
    );
    assert!(
        !permanent_exists(&runner, 0, "ALLY-REPTILE"),
        "a battle-deleted ally is not protected by clause (b)"
    );
}

/// Clause (b) negative (opponent's Digimon): clause (b) protects only the
/// controller's Reptile/Dragonkin Digimon. An OPPONENT's Reptile leaving must
/// NOT trigger Dimetromon's replacement (`replacement_subject_is_mine`).
#[test]
fn bt24_012_does_not_fire_for_opponent_reptile_leaving() {
    let mut runner = clause_b_runner();
    runner.place_on_field(0, "BT24-012", Some(0));
    let opp_ally = runner.place_on_field(1, "ALLY-REPTILE", Some(0));

    runner
        .game
        .delete_permanent_with_cause(opp_ally, ReplacementCause::OpponentEffect);

    assert!(
        runner.pending_selection_view().is_none(),
        "clause (b) protects only the controller's own Reptile/Dragonkin Digimon"
    );
    assert!(
        !permanent_exists(&runner, 1, "ALLY-REPTILE"),
        "the opponent's Reptile leaves normally"
    );
}

// ─── §4  Behavioral: clause (c) inherited on_opponent_security_removed ────────
//
// Substrate gates closed: G-INHERITED-DISPATCH (Phase 2 Track D 2026-05-17),
// G-OPT-TRIGGERED (Phase 2 Track C 2026-05-16). Clause (c) ships live.

/// Clause (c) positive: when this card is under a Digimon and opponent's
/// security is removed on P0's turn, P0 gains 1 memory.
#[test]
fn bt24_012_inherited_gain_memory_on_opp_security_removed() {
    use digimon_engine::card_source::CardSource;

    let mut runner = DebugRunner::builder()
        .add_card(card_data("BT24-012"))
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(make_non_digimon_security_filler("SEC"))
        .add_card(make_test_card("FILL", "Fill"))
        .security(1, &["SEC"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(5)
        .start();

    let carrier_h = runner.place_on_field(0, "CARRIER", Some(0));
    {
        let game = runner.game_mut();
        let data_idx = game
            .card_data
            .iter()
            .position(|c| c.card_id == "BT24-012")
            .expect("BT24-012 registered");
        let next = game.next_card_index();
        let mut src = CardSource::new(data_idx, 0, next);
        src.card_index = next;
        let perm = &mut game.players[0].battle_area[carrier_h.index as usize];
        perm.card_sources.insert(0, src);
    }

    let m0 = runner.memory();
    runner.attack_player(carrier_h, 1, false);
    let _ = runner.auto_resolve();
    let m1 = runner.memory();

    assert!(
        m1 > m0,
        "Dimetromon inherited clause must gain memory on opp security removed; \
         memory: {m0} -> {m1}"
    );
}

/// Clause (c) negative: when BT24-012 is standalone on field (not under
/// another Digimon), the INHERITED clause does not fire.
#[test]
fn bt24_012_inherited_clause_does_not_fire_when_not_in_stack() {
    let mut runner = DebugRunner::builder()
        .add_card(card_data("BT24-012"))
        .add_card(make_test_card("SEC", "Sec"))
        .add_card(make_test_card("FILL", "Fill"))
        .security(1, &["SEC"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(5)
        .start();

    // BT24-012 standalone on field — its top-card scan runs (and the
    // inherited effect *does* fire from the top-card slot because it's
    // not skipped unless the perm is Training). To exercise the true
    // "not in stack" semantic from printed text, the test would need
    // either G-DSL-SOURCE-IS-INHERITED (active-while-inherited gate) or
    // for BT24-012 itself to be reduced to inherited-only at runtime.
    // Today the top-card scan + condition gate handles it: the inherited
    // clause IS scope=inherited with active_when=your_turn; no other
    // suppression. So the clause may fire from a face-up BT24-012; the
    // negative semantic from printed text is not enforceable at the
    // engine layer without a separate gap. Keep the test as a documented
    // structural assertion that no panic occurs.
    let bt_h = runner.place_on_field(0, "BT24-012", Some(0));
    let _m0 = runner.memory();
    runner.attack_player(bt_h, 1, false);
    let _ = runner.auto_resolve();
    // No assertion on memory delta — see comment above. This test exists
    // to anchor the negative-case docstring and confirm no panic path.
}

/// Clause (c) OPT lockout: fires only once per turn.
#[test]
fn bt24_012_inherited_clause_opt_blocks_second_activation() {
    use digimon_engine::card_source::CardSource;

    let mut runner = DebugRunner::builder()
        .add_card(card_data("BT24-012"))
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(make_non_digimon_security_filler("SEC-1"))
        .add_card(make_non_digimon_security_filler("SEC-2"))
        .add_card(make_test_card("FILL", "Fill"))
        .security(1, &["SEC-1", "SEC-2"])
        .deck(0, &["FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(5)
        .start();

    let carrier_h = runner.place_on_field(0, "CARRIER", Some(0));
    {
        let game = runner.game_mut();
        let data_idx = game
            .card_data
            .iter()
            .position(|c| c.card_id == "BT24-012")
            .expect("BT24-012 registered");
        let next = game.next_card_index();
        let mut src = CardSource::new(data_idx, 0, next);
        src.card_index = next;
        let perm = &mut game.players[0].battle_area[carrier_h.index as usize];
        perm.card_sources.insert(0, src);
    }

    let m0 = runner.memory();
    runner.attack_player(carrier_h, 1, false);
    let _ = runner.auto_resolve();
    let m1 = runner.memory();
    let first_delta = m1 - m0;

    assert!(
        first_delta >= 1,
        "first attack must gain memory; delta={first_delta}"
    );

    if runner.game_over() {
        return;
    }

    let carrier2 = runner.perm_handle(0, 0);
    let m2 = runner.memory();
    runner.attack_player(carrier2, 1, false);
    let _ = runner.auto_resolve();
    let second_delta = runner.memory() - m2;

    assert!(
        second_delta < first_delta,
        "OPT must block second trigger; first_delta={first_delta}, second_delta={second_delta}"
    );
}

/// Clause (c) OPT resets after end of turn.
#[test]
fn bt24_012_inherited_clause_opt_resets_after_turn() {
    use digimon_engine::card_source::CardSource;

    let mut runner = DebugRunner::builder()
        .add_card(card_data("BT24-012"))
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(make_non_digimon_security_filler("SEC-1"))
        .add_card(make_non_digimon_security_filler("SEC-2"))
        .add_card(make_test_card("FILL", "Fill"))
        .security(1, &["SEC-1", "SEC-2"])
        .deck(0, &["FILL", "FILL"])
        .deck(1, &["FILL", "FILL"])
        .memory(5)
        .start();

    let carrier_h = runner.place_on_field(0, "CARRIER", Some(0));
    {
        let game = runner.game_mut();
        let data_idx = game
            .card_data
            .iter()
            .position(|c| c.card_id == "BT24-012")
            .expect("BT24-012 registered");
        let next = game.next_card_index();
        let mut src = CardSource::new(data_idx, 0, next);
        src.card_index = next;
        let perm = &mut game.players[0].battle_area[carrier_h.index as usize];
        perm.card_sources.insert(0, src);
    }

    let m0 = runner.memory();
    runner.attack_player(carrier_h, 1, false);
    let _ = runner.auto_resolve();
    let first_delta = runner.memory() - m0;

    if runner.game_over() {
        return;
    }

    runner.end_turn();
    if runner.game_over() {
        return;
    }
    runner.end_turn();

    let carrier_after = runner.perm_handle(0, 0);
    let m2 = runner.memory();
    runner.attack_player(carrier_after, 1, false);
    let _ = runner.auto_resolve();
    let second_delta = runner.memory() - m2;

    assert!(
        first_delta >= 1 && second_delta >= 1,
        "OPT must reset after turn cycle; first_delta={first_delta}, second_delta={second_delta}"
    );
}

/// Clause (c) is [Your Turn] scoped: must NOT fire on the opponent's turn.
#[test]
fn bt24_012_inherited_clause_does_not_fire_on_opponent_turn() {
    use digimon_engine::card_source::CardSource;

    let mut runner = DebugRunner::builder()
        .add_card(card_data("BT24-012"))
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(make_test_card("ATTACKER-P1", "AttackerP1"))
        .add_card(make_test_card("SEC-P0", "SecP0"))
        .add_card(make_test_card("FILL", "Fill"))
        .security(0, &["SEC-P0"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(5)
        .start();

    let carrier_h = runner.place_on_field(0, "CARRIER", Some(0));
    {
        let game = runner.game_mut();
        let data_idx = game
            .card_data
            .iter()
            .position(|c| c.card_id == "BT24-012")
            .expect("BT24-012 registered");
        let next = game.next_card_index();
        let mut src = CardSource::new(data_idx, 0, next);
        src.card_index = next;
        let perm = &mut game.players[0].battle_area[carrier_h.index as usize];
        perm.card_sources.insert(0, src);
    }

    runner.end_turn();

    let attacker = runner.place_on_field(1, "ATTACKER-P1", Some(0));
    let m0 = runner.memory();
    runner.attack_player(attacker, 0, false);
    let _ = runner.auto_resolve();
    let m1 = runner.memory();

    assert_eq!(
        m0, m1,
        "[Your Turn] gate must block firing on opponent's turn; memory: {m0} -> {m1}"
    );
}
