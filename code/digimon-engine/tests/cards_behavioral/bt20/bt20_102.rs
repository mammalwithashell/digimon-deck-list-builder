//! BT20-102 Omnimon (X Antibody) — Digimon, Lv.7, DP16000, Cost16, Red/Blue.
//! Traits: Holy Warrior, X Antibody, Royal Knight, LIBERATOR
//!
//! # Card text (cards.json)
//!
//! ＜Raid＞ ＜Piercing＞ ＜Blocker＞
//! [On Play][When Digivolving] If [Omnimon] or [X Antibody] is in this Digimon's
//! digivolution cards, choose 1 of both players' Digimon and delete all other
//! Digimon. Then, return 1 of your opponent's Digimon to the bottom of the deck.
//! [End of Your Turn][Once Per Turn] 1 of your Digimon may gain ＜Rush＞ for the
//! turn and attack without suspending.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT20/Blue/BT20_102.cs
//!
//! # Alt-path
//! When digivolution source's top card name contains "Omnimon", digivolve for cost 2.
//!
//! # Patterns this test file covers (RUST_DSL_TEST_API.md §4.3)
//! - D4: declarative keyword grants (Raid, Piercing, Blocker)
//! - F: board-wipe + conditional stack check (pure DSL: for_each + binding_absent/not_in_binding)
//! - H3: Rush keyword grant for the turn (grant_keyword + expiry)
//! - E2: optional end-of-turn clause (OPT gated)
//!
//! # Known gaps and test status
//!
//! | Clause | Gap | Status |
//! |--------|-----|--------|
//! | (a) Raid declarative grant | G-DECLARATIVE-KEYWORD: never enqueued at runtime | compiled/structural only |
//! | (b) Piercing declarative grant | G-DECLARATIVE-KEYWORD | compiled/structural only |
//! | (c) Blocker declarative grant | G-DECLARATIVE-KEYWORD | compiled/structural only |
//! | (d) [On Play][WD] boardwipe + return to deck | pure DSL — condition `self_digivolution_sources_contain_name` (G-SELF-DIGIVOLUTION-CONTAINS-NAME-SOURCES-ONLY resolved); body `for_each` + `binding_absent`/`not_in_binding` exclusion (G-FOR-EACH-EXCLUDE-BINDING resolved 2026-05-20) | PASS |
//! | (e) [EOT][OPT] Rush grant + attack without suspending | force_attack DSL + AttackOpen without_suspending | PASS |
//! | (e-OPT) once-per-turn lockout | G-OPT-TRIGGERED: triggered OPT not enforced | #[ignore] |

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::{encode_attack, SECURITY_TARGET};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::Keyword;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

use crate::dsl_card_data::compiled;

// ─── SECTION 1 — Structural assertions ───────────────────────────────────────

/// BT20-102 must compile with exactly 3 declarative GrantKeyword clauses:
/// Raid, Piercing, Blocker.
#[test]
fn bt20_102_has_three_grant_keyword_clauses() {
    let card = compiled("BT20-102");

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
        3,
        "Raid, Piercing, Blocker must each produce one GrantKeyword clause; got: {:?}",
        keyword_grants
    );
}

/// The three granted keywords are exactly: Raid, Piercing, Blocker.
#[test]
fn bt20_102_keyword_grants_are_raid_piercing_blocker() {
    let card = compiled("BT20-102");

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

    for expected in &["Raid", "Piercing", "Blocker"] {
        assert!(
            keywords.contains(*expected),
            "keyword grant '{}' missing; got: {:?}",
            expected,
            keywords
        );
    }
}

/// BT20-102 must compile with exactly 2 triggered clauses:
///   0: [on_play, when_digivolving] boardwipe clause (d)
///   1: [end_of_your_turn] Rush clause (e)
#[test]
fn bt20_102_has_two_triggered_clauses() {
    let card = compiled("BT20-102");

    let triggered_count = card
        .effects
        .iter()
        .filter(|c| matches!(c, CompiledClause::Triggered(_)))
        .count();

    assert_eq!(
        triggered_count, 2,
        "BT20-102 must have exactly 2 triggered clauses; got {triggered_count}"
    );
}

/// Clause (d): triggered on [OnPlay, WhenDigivolving], FaceUp scope, not optional.
#[test]
fn bt20_102_clause_d_is_on_play_when_digivolving_not_optional() {
    let card = compiled("BT20-102");

    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| {
            t.when.contains(&CompiledTiming::OnPlay)
                && t.when.contains(&CompiledTiming::WhenDigivolving)
        });

    let clause = clause.expect("[OnPlay, WhenDigivolving] triggered clause (d) must exist");

    assert_eq!(
        clause.scope,
        CompiledScope::FaceUp,
        "clause (d) must be FaceUp scope"
    );
    assert!(
        !clause.optional,
        "clause (d) is not optional — 'choose 1' is mandatory when condition is met"
    );
    assert!(!clause.once_per_turn, "clause (d) has no OPT restriction");
}

/// Clause (e): triggered on [EndOfYourTurn], FaceUp scope, optional, once_per_turn.
#[test]
fn bt20_102_clause_e_is_end_of_your_turn_optional_opt() {
    let card = compiled("BT20-102");

    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::EndOfYourTurn));

    let clause = clause.expect("[EndOfYourTurn] triggered clause (e) must exist");

    assert_eq!(
        clause.scope,
        CompiledScope::FaceUp,
        "clause (e) must be FaceUp scope"
    );
    assert!(
        clause.optional,
        "clause (e) is optional (card text: 'may gain')"
    );
    assert!(
        clause.once_per_turn,
        "clause (e) is [Once Per Turn] — must have once_per_turn flag"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Clause (d) boardwipe + return (pure DSL): behavioral
// ═══════════════════════════════════════════════════════════════════════════════
//
// The boardwipe clause is now PURE DSL (migrated 2026-05-20 — the raw_rust
// escape `bt20_102_boardwipe_and_return` was removed). The clause `condition:`
// (`self_digivolution_sources_contain_name`) gates the whole clause:
//   - When no qualifying name is in stack: the clause never fires (no
//     selection prompt, no deletion, no return).
//   - When a qualifying name IS in stack: the process steps run — two
//     `select_*_permanent` protect-picks, a `for_each` delete of every
//     unprotected Digimon, then a `select_opponent_permanent` + `return_to_deck`
//     bottom.
//
// The stack-condition check uses G-SELF-DIGIVOLUTION-CONTAINS-NAME-SOURCES-ONLY
// (resolved). The "delete all OTHER Digimon" exclusion uses a `for_each` whose
// `over` predicate wraps each protect-binding as
// `any_of: [{ binding_absent: <save> }, { not_in_binding: <save> }]`
// (G-FOR-EACH-EXCLUDE-BINDING resolved 2026-05-20 — pure DSL).

/// POSITIVE: [On Play] with "Omnimon" in stack — the DSL boardwipe clause installs
/// a SelectOpponentField prompt (choose 1 to protect).
///
/// Setup: place Omnimon (X Antibody) on field with digivolution source named "Omnimon".
/// Both players have additional Digimon. Expect pending selection for opp to protect.
#[test]
fn bt20_102_on_play_with_omnimon_in_stack_installs_selection() {
    let mut base = make_test_card("TEST-OMNIMON-SOURCE", "Omnimon");
    base.level = Some(6);
    base.dp = Some(12000);

    let mut opp1 = make_test_card("TEST-OPP-1", "OppDigimon1");
    opp1.level = Some(4);
    opp1.dp = Some(5000);

    let mut opp2 = make_test_card("TEST-OPP-2", "OppDigimon2");
    opp2.level = Some(4);
    opp2.dp = Some(4000);

    let mut own_ally = make_test_card("TEST-OWN-ALLY", "OwnAlly");
    own_ally.level = Some(5);
    own_ally.dp = Some(6000);

    let filler = make_test_card("DECK-PAD", "Filler");

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(include_str!("../../../cards/bt20/BT20-102.yaml"))
        .expect("BT20-102 YAML parses")
        .add_card(base)
        .add_card(opp1)
        .add_card(opp2)
        .add_card(own_ally)
        .add_card(filler)
        .memory(10)
        .deck(0, &["DECK-PAD", "DECK-PAD", "DECK-PAD"])
        .deck(1, &["DECK-PAD", "DECK-PAD", "DECK-PAD"])
        .start();

    // Place ally on P0, two opp Digimon on P1.
    runner.place_on_field(0, "TEST-OWN-ALLY", None);
    runner.place_on_field(1, "TEST-OPP-1", Some(0));
    runner.place_on_field(1, "TEST-OPP-2", Some(0));

    // Place BT20-102 ON TOP of an "Omnimon"-named digivolution source. The
    // clause `condition` uses `self_digivolution_sources_contain_name`, which
    // scans ONLY the source cards beneath the carrier (NOT the carrier's own
    // top card) — so an "Omnimon" source genuinely satisfies the condition.
    runner.place_stack(0, &["TEST-OMNIMON-SOURCE", "BT20-102"]);
    runner.fire_on_play(0, 1); // index 1 = BT20-102 (ally is at 0)

    // After fire_on_play, the boardwipe clause should have run.
    // Since opp has 2+ Digimon, a SelectOpponentField prompt installs to choose 1 to protect.
    let kind = runner.pending_kind();
    assert!(
        kind.is_some(),
        "boardwipe clause should install a selection prompt when opp has Digimon"
    );
}

/// POSITIVE: [On Play] boardwipe deletes all opponent Digimon except the saved one.
#[test]
fn bt20_102_on_play_boardwipe_deletes_non_saved_opp_digimon() {
    let mut opp1 = make_test_card("TEST-OPP-1", "OppDigimon1");
    opp1.level = Some(4);
    opp1.dp = Some(5000);

    let mut opp2 = make_test_card("TEST-OPP-2", "OppDigimon2");
    opp2.level = Some(4);
    opp2.dp = Some(4000);

    let mut base = make_test_card("TEST-OMNIMON-SOURCE", "Omnimon");
    base.level = Some(6);
    base.dp = Some(12000);

    let filler = make_test_card("DECK-PAD", "Filler");

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(include_str!("../../../cards/bt20/BT20-102.yaml"))
        .expect("BT20-102 YAML parses")
        .add_card(opp1)
        .add_card(opp2)
        .add_card(base)
        .add_card(filler)
        .memory(10)
        .deck(
            1,
            &["DECK-PAD", "DECK-PAD", "DECK-PAD", "DECK-PAD", "DECK-PAD"],
        )
        .start();

    runner.place_on_field(1, "TEST-OPP-1", Some(0));
    runner.place_on_field(1, "TEST-OPP-2", Some(0));
    // BT20-102 on top of an "Omnimon" source so the clause condition holds.
    runner.place_stack(0, &["TEST-OMNIMON-SOURCE", "BT20-102"]);
    runner.fire_on_play(0, 0); // BT20-102 at index 0

    // opp has 2 Digimon → first prompt = opp choose 1 to save.
    let opp_digimon_count_before = runner.game.players[1].battle_area.len();
    assert_eq!(
        opp_digimon_count_before, 2,
        "precondition: opp has 2 Digimon"
    );

    // The boardwipe clause should have installed a SelectOpponentField prompt.
    assert!(
        runner.pending_kind().is_some(),
        "SelectOpponentField prompt must be installed"
    );

    // Resolve: save first legal opp Digimon.
    {
        let action = runner
            .game
            .pending_selection
            .as_ref()
            .unwrap()
            .valid_action_ids[0];
        runner
            .game
            .resolve_selection(0, action)
            .expect("resolve opp save");
    }

    // Now own Digimon save prompt: P0 has only BT20-102, so it is auto-resolved
    // or prompt installs for own. Resolve it to BT20-102 itself.
    if runner.pending_kind().is_some() {
        let action = runner
            .game
            .pending_selection
            .as_ref()
            .unwrap()
            .valid_action_ids[0];
        runner
            .game
            .resolve_selection(0, action)
            .expect("resolve own save");
    }

    // After both saves, non-saved Digimon should be deleted.
    // P1 should have exactly 1 Digimon remaining (the saved one).
    // Then the return-to-deck prompt installs.
    let opp_digimon_after_delete = runner.game.players[1].battle_area.len();
    assert_eq!(
        opp_digimon_after_delete, 1,
        "all non-saved opp Digimon must be deleted; expected 1 remaining, got {opp_digimon_after_delete}"
    );
}

/// POSITIVE: [On Play] unconditional return-to-deck-bottom prompt installs for opp Digimon
/// after the boardwipe resolves, even when condition was met (DCGO: outside IsOmniOrXAntiSource block).
#[test]
fn bt20_102_on_play_return_to_deck_prompt_installs_after_boardwipe() {
    let mut opp1 = make_test_card("TEST-OPP-1", "OppDigimon1");
    opp1.level = Some(4);
    opp1.dp = Some(5000);

    let mut base = make_test_card("TEST-OMNIMON-SOURCE", "Omnimon");
    base.level = Some(6);
    base.dp = Some(12000);

    let filler = make_test_card("DECK-PAD", "Filler");

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(include_str!("../../../cards/bt20/BT20-102.yaml"))
        .expect("BT20-102 YAML parses")
        .add_card(opp1)
        .add_card(base)
        .add_card(filler)
        .memory(10)
        .deck(
            1,
            &["DECK-PAD", "DECK-PAD", "DECK-PAD", "DECK-PAD", "DECK-PAD"],
        )
        .start();

    runner.place_on_field(1, "TEST-OPP-1", Some(0));
    // BT20-102 on top of an "Omnimon" source so the clause condition holds.
    runner.place_stack(0, &["TEST-OMNIMON-SOURCE", "BT20-102"]);
    runner.fire_on_play(0, 0);

    // With 1 opp Digimon: opp-save prompt may auto-resolve (only 1 target = auto-selected
    // or prompt fires with single valid action). Drive to completion.
    // Then own save prompt (only BT20-102). Then delete step (no others to delete).
    // Then return-to-deck prompt installs for remaining opp Digimon.
    //
    // Resolve all intermediate prompts.
    let mut iterations = 0;
    while runner.pending_kind().is_some() && iterations < 10 {
        let action = runner
            .game
            .pending_selection
            .as_ref()
            .unwrap()
            .valid_action_ids[0];
        runner.game.resolve_selection(0, action).expect("resolve");
        iterations += 1;

        // Stop at a SelectOpponentField that follows the boardwipe (the return prompt).
        // We detect this by checking if opp's battle area is now empty (boardwipe ran)
        // and the prompt is still SelectOpponentField.
        if runner.game.players[1].battle_area.is_empty() {
            break; // boardwipe deleted the only opp Digimon: no return prompt needed
        }
        // If there's a remaining opp Digimon and a pending prompt, that's the return prompt.
        if !runner.game.players[1].battle_area.is_empty()
            && matches!(runner.pending_kind(), Some(SelectionKind::OppField))
        {
            break; // Return-to-deck prompt is installed
        }
    }

    // Either opp had no remaining Digimon (all deleted, return step is no-op) or
    // the return prompt is installed. Assert the postcondition is consistent.
    let opp_digimon = runner.game.players[1].battle_area.len();
    if opp_digimon > 0 {
        assert!(
            matches!(runner.pending_kind(), Some(SelectionKind::OppField)),
            "when opp still has Digimon after boardwipe, return-to-deck prompt must install"
        );
    }
    // No assertion fails if opp has 0 remaining: the return step simply skips.
}

// ─── SECTION 2b — Condition gating (G-SELF-DIGIVOLUTION-CONTAINS-NAME-SOURCES-ONLY) ──

/// G-SELF-DIGIVOLUTION-CONTAINS-NAME-SOURCES-ONLY (RESOLVED 2026-05-19):
/// the clause condition "If [Omnimon] or [X Antibody] is in this Digimon's
/// digivolution cards" scans ONLY the digivolution source cards beneath the
/// carrier — NOT the carrier's own top card. A BT20-102 played WITHOUT any
/// digivolution source (its only card is the top card "Omnimon (X
/// Antibody)") must NOT satisfy the condition: the carrier's own name is
/// excluded from the sources-only scan. So the board-wipe must NOT fire.
///
/// The pre-existing `self_digivolution_contains_name` predicate scans the
/// top card too — under it, BT20-102 ALWAYS self-matches "Omnimon" /
/// "X Antibody" and this negative case was inexpressible. The new
/// `self_digivolution_sources_contain_name` predicate makes it expressible.
#[test]
fn bt20_102_on_play_no_boardwipe_when_no_omnimon_in_stack() {
    let mut opp1 = make_test_card("TEST-OPP-1", "OppDigimon1");
    opp1.level = Some(4);
    opp1.dp = Some(5000);

    let mut opp2 = make_test_card("TEST-OPP-2", "OppDigimon2");
    opp2.level = Some(4);
    opp2.dp = Some(4000);

    let filler = make_test_card("DECK-PAD", "Filler");

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(include_str!("../../../cards/bt20/BT20-102.yaml"))
        .expect("BT20-102 YAML parses")
        .add_card(opp1)
        .add_card(opp2)
        .add_card(filler)
        .memory(10)
        .deck(
            1,
            &["DECK-PAD", "DECK-PAD", "DECK-PAD", "DECK-PAD", "DECK-PAD"],
        )
        .start();

    runner.place_on_field(1, "TEST-OPP-1", Some(0));
    runner.place_on_field(1, "TEST-OPP-2", Some(0));
    // BT20-102 placed BARE — no digivolution source. Its only card is the
    // top card "Omnimon (X Antibody)", which the sources-only scan excludes.
    runner.place_on_field(0, "BT20-102", None);
    runner.fire_on_play(0, 0);

    // The clause `condition` (`self_digivolution_sources_contain_name`)
    // fails — no Omnimon / X Antibody in the (empty) source list — so the
    // board-wipe never runs and no selection prompt installs.
    assert!(
        runner.pending_kind().is_none(),
        "bare BT20-102 (no digivolution source) must NOT satisfy the \
         'Omnimon/X Antibody in digivolution cards' condition — no board-wipe \
         prompt should install"
    );
    // Both opponent Digimon must still be on the field (no deletion).
    assert_eq!(
        runner.game.players[1].battle_area.len(),
        2,
        "no opponent Digimon should be deleted when the stack condition fails"
    );
}

/// G-SELF-DIGIVOLUTION-CONTAINS-NAME-SOURCES-ONLY (RESOLVED): companion
/// positive — a BT20-102 on top of a NON-Omnimon, non-X-Antibody source
/// also fails the condition (the source name does not match either).
#[test]
fn bt20_102_on_play_no_boardwipe_when_unrelated_source_in_stack() {
    let mut opp1 = make_test_card("TEST-OPP-1", "OppDigimon1");
    opp1.level = Some(4);
    opp1.dp = Some(5000);

    let mut base = make_test_card("TEST-PLAIN-SOURCE", "WarGreymon");
    base.level = Some(6);
    base.dp = Some(12000);

    let filler = make_test_card("DECK-PAD", "Filler");

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(include_str!("../../../cards/bt20/BT20-102.yaml"))
        .expect("BT20-102 YAML parses")
        .add_card(opp1)
        .add_card(base)
        .add_card(filler)
        .memory(10)
        .deck(
            1,
            &["DECK-PAD", "DECK-PAD", "DECK-PAD", "DECK-PAD", "DECK-PAD"],
        )
        .start();

    runner.place_on_field(1, "TEST-OPP-1", Some(0));
    // BT20-102 on top of an unrelated "WarGreymon" source.
    runner.place_stack(0, &["TEST-PLAIN-SOURCE", "BT20-102"]);
    runner.fire_on_play(0, 0);

    assert!(
        runner.pending_kind().is_none(),
        "BT20-102 with only a non-Omnimon/non-X-Antibody source must NOT \
         satisfy the digivolution-cards condition"
    );
    assert_eq!(
        runner.game.players[1].battle_area.len(),
        1,
        "no opponent Digimon should be deleted when the stack condition fails"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Clause (e): [End of Your Turn][OPT] Rush grant
// ═══════════════════════════════════════════════════════════════════════════════

fn runner_with_omnimon_on_field() -> DebugRunner {
    let mut opp = make_test_card("TEST-OPP", "OppDigimon");
    opp.level = Some(4);
    opp.dp = Some(5000);

    let mut own_ally = make_test_card("TEST-ALLY", "AllyDigimon");
    own_ally.level = Some(5);
    own_ally.dp = Some(7000);

    let filler = make_test_card("DECK-PAD", "Filler");

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(include_str!("../../../cards/bt20/BT20-102.yaml"))
        .expect("BT20-102 YAML parses")
        .add_card(opp)
        .add_card(own_ally)
        .add_card(filler)
        .memory(10)
        .deck(
            0,
            &["DECK-PAD", "DECK-PAD", "DECK-PAD", "DECK-PAD", "DECK-PAD"],
        )
        .deck(
            1,
            &["DECK-PAD", "DECK-PAD", "DECK-PAD", "DECK-PAD", "DECK-PAD"],
        )
        .security(1, &["DECK-PAD", "DECK-PAD"])
        .start();

    runner.place_on_field(0, "BT20-102", None);
    runner.place_on_field(0, "TEST-ALLY", None);
    runner.place_on_field(1, "TEST-OPP", Some(0));
    runner
}

/// POSITIVE: [End of Turn] with own Digimon present — SelectOwnField prompt installs.
#[test]
fn bt20_102_end_of_turn_installs_selection_when_own_digimon_present() {
    let mut runner = runner_with_omnimon_on_field();

    runner.game.end_turn();

    // The [End of Your Turn] clause is optional ("1 of your Digimon may gain
    // Rush ...") and its body's first step is a mandatory select_own_permanent,
    // so an outer accept/decline prompt installs first
    // (G-OUTER-OPTIONAL-NOT-INSTALLED). Accept it.
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Replacement),
        "[EOT] optional clause must install an outer accept/decline prompt"
    );
    runner
        .accept_optional_trigger()
        .expect("accept the outer optional-trigger prompt");

    let kind = runner.pending_kind();
    assert!(
        kind.is_some(),
        "[EOT] clause must install a selection when P0 has a Digimon"
    );
    assert!(
        matches!(kind, Some(SelectionKind::OwnField)),
        "selection kind must be OwnField for select_own_permanent"
    );
}

/// POSITIVE: After selecting the ally Digimon, it gains Rush for the turn.
#[test]
fn bt20_102_end_of_turn_selected_digimon_gains_rush() {
    let mut runner = runner_with_omnimon_on_field();

    runner.game.end_turn();

    // Accept the outer optional-trigger prompt (G-OUTER-OPTIONAL-NOT-INSTALLED).
    runner
        .accept_optional_trigger()
        .expect("accept the outer optional-trigger prompt");

    assert!(
        runner.pending_kind().is_some(),
        "selection must be installed"
    );

    // Pick first valid action (own Digimon: BT20-102 at 0 or ally at 1).
    let action = {
        let pending = runner.game.pending_selection.as_ref().unwrap();
        assert!(
            !pending.valid_action_ids.is_empty(),
            "at least one eligible own Digimon"
        );
        pending.valid_action_ids[0]
    };
    runner
        .game
        .resolve_selection(0, action)
        .expect("selection resolves");

    // The selected Digimon must now have Rush.
    // The selected permanent is whichever index matches the first valid action.
    // We verify that at least one of P0's Digimon has Rush.
    let any_has_rush = runner.game.players[0]
        .battle_area
        .iter()
        .enumerate()
        .any(|(idx, _)| {
            let handle = PermanentHandle {
                player: 0,
                index: idx as u8,
            };
            runner.game.has_keyword(handle, Keyword::Rush)
        });

    assert!(
        any_has_rush,
        "selected Digimon must have Rush after BT20-102 EOT effect resolves"
    );
}

/// POSITIVE: Rush granted by BT20-102 EOT effect expires at end of turn.
#[test]
fn bt20_102_end_of_turn_rush_expires_end_of_turn() {
    let mut runner = runner_with_omnimon_on_field();

    // P0 ends their turn; EOT clause fires.
    runner.game.end_turn();

    // Accept the outer optional-trigger prompt (G-OUTER-OPTIONAL-NOT-INSTALLED).
    runner
        .accept_optional_trigger()
        .expect("accept the outer optional-trigger prompt");

    // Resolve selection: choose the ally, then attack security. The helper
    // seeds opponent security so the game stays alive long enough to observe expiry.
    let choose_ally = encode_attack(0, 1);
    if let Some(ref pending) = runner.game.pending_selection {
        assert!(pending.valid_action_ids.contains(&choose_ally));
        runner
            .game
            .resolve_selection(0, choose_ally)
            .expect("resolve EOT selection");
    }

    // The accepted BT20-102 trigger now immediately opens the mandatory
    // attack-without-suspending target prompt. Finish that attack before
    // advancing the turn for expiry checks.
    let attack_player = encode_attack(1, SECURITY_TARGET);
    if let Some(ref pending) = runner.game.pending_selection {
        assert!(pending.valid_action_ids.contains(&attack_player));
        runner
            .game
            .resolve_selection(0, attack_player)
            .expect("resolve BT20-102 attack target");
    }

    // Rush is active for some own Digimon.
    let _ally_handle = PermanentHandle {
        player: 0,
        index: 1,
    }; // BT20-102@0, ally@1
       // Don't assert which specific index has Rush; just confirm the expiry mechanics.
       // Advance to P1's turn end to trigger expiry.
    runner.game.end_turn(); // P1 ends their turn; expire_end_of_turn(1) fires

    // Rush must have expired.
    let any_has_rush_after =
        runner.game.players[0]
            .battle_area
            .iter()
            .enumerate()
            .any(|(idx, _)| {
                let handle = PermanentHandle {
                    player: 0,
                    index: idx as u8,
                };
                runner.game.has_keyword(handle, Keyword::Rush)
            });

    assert!(
        !any_has_rush_after,
        "Rush granted by BT20-102 must expire at end of the turn it was granted"
    );
}

/// NEGATIVE: [End of Turn] — NO selection installs when P0 has NO Digimon.
/// (Though BT20-102 itself is a Digimon — setup without placing it on field.)
#[test]
fn bt20_102_end_of_turn_no_selection_when_no_own_digimon() {
    let filler = make_test_card("DECK-PAD", "Filler");

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(include_str!("../../../cards/bt20/BT20-102.yaml"))
        .expect("BT20-102 YAML parses")
        .add_card(filler)
        .memory(5)
        .deck(0, &["DECK-PAD", "DECK-PAD", "DECK-PAD"])
        .deck(1, &["DECK-PAD", "DECK-PAD", "DECK-PAD"])
        .start();

    // Do NOT place BT20-102 on field — it fires from hand/deck, not field.
    // EOT trigger requires BT20-102 to be on the field (FaceUp scope).
    // With nothing on field, no EOT effect fires.
    runner.game.end_turn();

    assert!(
        runner.pending_kind().is_none(),
        "no EOT selection when BT20-102 is not on field (or no own Digimon)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — "Attack without suspending"
// ═══════════════════════════════════════════════════════════════════════════════

/// POSITIVE: After the optional EOT trigger is accepted and a Digimon is chosen,
/// that Digimon must enter the normal attack target prompt without suspending.
#[test]
fn bt20_102_end_of_turn_selected_digimon_attacks_without_suspending() {
    let mut runner = runner_with_omnimon_on_field();
    let attacker = PermanentHandle {
        player: 0,
        index: 1,
    };
    let choose_ally = encode_attack(0, attacker.index as u16);

    runner.game.end_turn();

    // Accept the outer optional-trigger prompt (G-OUTER-OPTIONAL-NOT-INSTALLED).
    runner
        .accept_optional_trigger()
        .expect("accept the outer optional-trigger prompt");

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("EOT choice should be pending");
    assert!(
        pending.valid_action_ids.contains(&choose_ally),
        "ally Digimon should be a legal BT20-102 target"
    );
    runner
        .game
        .resolve_selection(0, choose_ally)
        .expect("resolve BT20-102 selected Digimon");

    let attack_player = encode_attack(attacker.index as u16, SECURITY_TARGET);
    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("force_attack should install an attack-target prompt");
    assert_eq!(
        pending.selecting_player, 0,
        "chosen Digimon's controller should choose the attack target"
    );
    assert!(
        !pending.is_optional,
        "attack after accepting BT20-102's optional EOT trigger is mandatory"
    );
    assert!(
        pending.valid_action_ids.contains(&attack_player),
        "chosen Digimon should be able to attack the opponent player"
    );

    runner
        .game
        .resolve_selection(0, attack_player)
        .expect("BT20-102 forced attack target resolves");

    assert!(
        !runner.game.players[attacker.player as usize].battle_area[attacker.index as usize]
            .is_suspended,
        "BT20-102's selected Digimon must remain unsuspended after attacking without suspending"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 5 — OPT lockout — BLOCKED (G-OPT-TRIGGERED)
// ═══════════════════════════════════════════════════════════════════════════════

/// Clause (e) OPT lockout — printed [Once Per Turn]. The end-of-your-turn
/// "1 of your Digimon may gain Rush" clause may fire at most once per turn.
/// Firing `EndOfYourTurn` a second time in the same turn must NOT install a
/// second prompt; the per-turn OPT reset (`Permanent::new_turn`) re-arms it.
///
/// OPT enforcement for permanent-sourced triggered effects is wired in
/// `run_queued_effect_inner` (`effect_queue.rs` ~1980-2016).
#[test]
fn bt20_102_end_of_turn_opt_lockout() {
    use digimon_engine::enums::EffectTiming;
    use digimon_engine::selection::TriggerSource;

    let mut runner = runner_with_omnimon_on_field();
    // BT20-102 is placed at index 0 by `runner_with_omnimon_on_field`.
    let omnimon = PermanentHandle {
        player: 0,
        index: 0,
    };

    // ── First fire: the EOT clause installs its outer optional prompt.
    runner.game.enqueue_triggered(
        EffectTiming::EndOfYourTurn,
        TriggerSource::Permanent(omnimon),
    );
    runner.game.drain_effect_queue();
    assert!(
        runner.game.pending_selection.is_some(),
        "first EndOfYourTurn fire must install the optional Rush prompt"
    );
    // Resolve the whole prompt chain (the body running consumes the OPT).
    let mut steps = 0;
    while runner.game.pending_selection.is_some() && steps < 12 {
        let (player, action) = {
            let pending = runner.game.pending_selection.as_ref().unwrap();
            (pending.selecting_player, pending.valid_action_ids[0])
        };
        runner.game.resolve_selection(player, action).ok();
        runner.game.drain_effect_queue();
        steps += 1;
    }

    // ── Second fire, SAME turn: OPT lockout — no prompt installs.
    runner.game.enqueue_triggered(
        EffectTiming::EndOfYourTurn,
        TriggerSource::Permanent(omnimon),
    );
    runner.game.drain_effect_queue();
    assert!(
        runner.game.pending_selection.is_none(),
        "OPT lockout must block the second EndOfYourTurn activation in the same turn"
    );

    // ── OPT reset: `Permanent::new_turn()` clears `effect_activations` — the
    // same per-turn reset `begin_turn()` performs for the turn player.
    runner.game.players[0].battle_area[omnimon.index as usize].new_turn();

    runner.game.enqueue_triggered(
        EffectTiming::EndOfYourTurn,
        TriggerSource::Permanent(omnimon),
    );
    runner.game.drain_effect_queue();
    assert!(
        runner.game.pending_selection.is_some(),
        "after the per-turn OPT reset the clause must fire again"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 6 — Declarative keyword grant smoke (G-DECLARATIVE-KEYWORD)
// ═══════════════════════════════════════════════════════════════════════════════

/// Declarative `grant_keyword` clauses (Raid, Piercing, Blocker) install
/// at runtime: `tick_declarative_effects` runs each FaceUp declarative
/// grant_keyword process, and `game.has_keyword` finds the installed
/// keyword modifier. Structural tests (Section 1) validate compilation
/// shape; this test verifies runtime keyword presence on a placed
/// Omnimon (X Antibody) permanent.
#[test]
fn bt20_102_has_raid_piercing_blocker_as_runtime_keywords() {
    let filler = make_test_card("DECK-PAD", "Filler");
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(include_str!("../../../cards/bt20/BT20-102.yaml"))
        .expect("BT20-102 YAML parses")
        .add_card(filler)
        .memory(10)
        .start();

    runner.place_on_field(0, "BT20-102", None);
    let handle = PermanentHandle {
        player: 0,
        index: 0,
    };

    assert!(
        runner.game.has_keyword(handle, Keyword::Raid),
        "BT20-102 must have Raid at runtime (pending G-DECLARATIVE-KEYWORD)"
    );
    assert!(
        runner.game.has_keyword(handle, Keyword::Piercing),
        "BT20-102 must have Piercing at runtime (pending G-DECLARATIVE-KEYWORD)"
    );
    assert!(
        runner.game.has_keyword(handle, Keyword::Blocker),
        "BT20-102 must have Blocker at runtime (pending G-DECLARATIVE-KEYWORD)"
    );
}
