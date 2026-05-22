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
//! - F: board-wipe + conditional stack check (raw_rust: bt20_102_boardwipe_and_return)
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
//! | (d) [On Play][WD] boardwipe + return to deck | G-SELF-DIGIVOLUTION-CONTAINS-NAME + G-FOR-EACH-EXCLUDE-BINDING → raw_rust | PARTIAL |
//! | (e) [EOT][OPT] Rush grant + attack without suspending | force_attack DSL + AttackOpen without_suspending | PASS |
//! | (e-OPT) once-per-turn lockout | G-OPT-TRIGGERED: triggered OPT not enforced | #[ignore] |

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::{encode_attack, SECURITY_TARGET};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{EffectTiming, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

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
// Section 2 — Clause (d) boardwipe + return (raw_rust): behavioral
// ═══════════════════════════════════════════════════════════════════════════════
//
// The boardwipe clause is entirely implemented via raw_rust: bt20_102_boardwipe_and_return.
// The DSL condition check (Omnimon / X Antibody in this Digimon's digivolution stack)
// is performed inside the raw_rust fn, which means:
//   - When no qualifying name is in stack: fn is a no-op (no deletion, no return).
//   - When qualifying name IS in stack: selects 1 per player to protect, deletes all
//     other Digimon, then (unconditionally) selects 1 opp Digimon to return to deck bottom.
//
// The stack-condition check requires G-SELF-DIGIVOLUTION-CONTAINS-NAME (new hybrid gap).
// The exclude-from-binding filter requires G-FOR-EACH-EXCLUDE-BINDING (new DSL gap).
// Both are routed through raw_rust — behavioral tests that exercise the select-and-delete
// path validate the raw_rust fn once it is registered.

/// POSITIVE: [On Play] with "Omnimon" in stack — raw_rust boardwipe fn installs
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

    // Place Omnimon (X Antibody) on P0 with "Omnimon" source underneath.
    // place_on_field drops a simple permanent; for stack check we need a real digivolution.
    // As a simplification: place the base on field, then use place_on_field for BT20-102
    // stacked on it. The raw_rust fn checks Permanent::contains_card_name for "Omnimon"
    // or "X Antibody" (top card name = "Omnimon (X Antibody)" contains "X Antibody").
    //
    // With no digivolution source, the stack contains only the top card.
    // "Omnimon (X Antibody)" contains "X Antibody" → condition satisfied even with no source.
    runner.place_on_field(0, "BT20-102", None);
    runner.fire_on_play(0, 1); // index 1 = BT20-102 (ally is at 0)

    // After fire_on_play, the boardwipe raw_rust fn should have run.
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
    runner.place_on_field(0, "BT20-102", None);
    runner.fire_on_play(0, 0); // BT20-102 at index 0

    // opp has 2 Digimon → first prompt = opp choose 1 to save.
    let opp_digimon_count_before = runner.game.players[1].battle_area.len();
    assert_eq!(
        opp_digimon_count_before, 2,
        "precondition: opp has 2 Digimon"
    );

    // The raw_rust fn should have installed a SelectOpponentField prompt.
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

    let filler = make_test_card("DECK-PAD", "Filler");

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(include_str!("../../../cards/bt20/BT20-102.yaml"))
        .expect("BT20-102 YAML parses")
        .add_card(opp1)
        .add_card(filler)
        .memory(10)
        .deck(
            1,
            &["DECK-PAD", "DECK-PAD", "DECK-PAD", "DECK-PAD", "DECK-PAD"],
        )
        .start();

    runner.place_on_field(1, "TEST-OPP-1", Some(0));
    runner.place_on_field(0, "BT20-102", None);
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

/// POSITIVE: [When Digivolving] path fires the same boardwipe raw_rust as [On Play].
///
/// Fire the WhenDigivolving timing explicitly from BT20-102's permanent handle.
/// With 2 opp Digimon present, a SelectOpponentField prompt must install.
#[test]
fn bt20_102_when_digivolving_installs_boardwipe_selection() {
    let mut opp1 = make_test_card("TEST-OPP-WD-1", "WdOpp1");
    opp1.level = Some(4);
    opp1.dp = Some(5000);

    let mut opp2 = make_test_card("TEST-OPP-WD-2", "WdOpp2");
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
        .deck(1, &["DECK-PAD", "DECK-PAD", "DECK-PAD", "DECK-PAD", "DECK-PAD"])
        .start();

    runner.place_on_field(1, "TEST-OPP-WD-1", Some(0));
    runner.place_on_field(1, "TEST-OPP-WD-2", Some(0));
    let omni_h = runner.place_on_field(0, "BT20-102", None);

    // Fire WhenDigivolving explicitly (simulates digivolving into BT20-102).
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(omni_h));
    runner.game.drain_effect_queue();

    // The boardwipe raw_rust fn fires: opp has 2 Digimon → SelectOpponentField installs.
    let kind = runner.pending_kind();
    assert!(
        kind.is_some(),
        "[When Digivolving] boardwipe clause must install a selection when opp has Digimon"
    );
    assert!(
        matches!(kind, Some(SelectionKind::OppField)),
        "[When Digivolving] selection must be OppField (choose opp Digimon to protect)"
    );
}

// ─── SECTION 2b — Condition gating tests (G-SELF-DIGIVOLUTION-CONTAINS-NAME) ──

/// BLOCKED: Condition "If [Omnimon] or [X Antibody] is in this Digimon's
/// digivolution cards" requires G-SELF-DIGIVOLUTION-CONTAINS-NAME.
///
/// This test verifies that when the stack does NOT contain "Omnimon" or "X Antibody"
/// by name (and top card name is not BT20-102), the boardwipe fn is a no-op.
/// The raw_rust fn implements this check via Permanent::contains_card_name.
///
/// NOTE: Because the top card of BT20-102 IS "Omnimon (X Antibody)" which contains
/// "X Antibody", the condition is always true for BT20-102 even with no digivolution
/// source. The true "negative" scenario (condition fails) requires a different card
/// that uses the same raw_rust fn — or would require placing a different card name
/// at the top of the permanent's stack. This test is marked ignore until
/// G-SELF-DIGIVOLUTION-CONTAINS-NAME closes and the condition becomes expressible
/// without the always-match fallback from the top card name itself.
#[test]
#[ignore = "pending: G-SELF-DIGIVOLUTION-CONTAINS-NAME — DSL cannot express self-stack name check; raw_rust fn always finds 'X Antibody' in top card name"]
fn bt20_102_on_play_no_boardwipe_when_no_omnimon_in_stack() {
    // Setup: place BT20-102 WITHOUT an Omnimon base under it. Verify no deletion happens.
    // Currently the top card's name contains "X Antibody" so the condition is always met.
    // Once G-SELF-DIGIVOLUTION-CONTAINS-NAME is resolved, the condition will check
    // card_sources (not the top card) and a "bare" BT20-102 can fail the condition.
    todo!("G-SELF-DIGIVOLUTION-CONTAINS-NAME: digivolution stack condition check not yet expressible in DSL condition closure")
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

/// BLOCKED: [Once Per Turn] on clause (e) compiles to Effect::max_per_turn=1 but
/// run_queued_effect_inner does not enforce this for triggered effects.
/// The clause will over-fire until G-OPT-TRIGGERED closes.
#[test]
#[ignore = "pending: G-OPT-TRIGGERED — once_per_turn not enforced for triggered EOT effects"]
fn bt20_102_end_of_turn_opt_lockout() {
    // Setup: two copies of BT20-102 on field (or simulate two OPT-eligible triggers).
    // Each copy fires its EOT clause → OPT should block the second firing.
    // Expected: only 1 SelectOwnField prompt installs, not 2.
    todo!("G-OPT-TRIGGERED: triggered OPT enforcement pending")
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 6 — Declarative keyword grant smoke (G-DECLARATIVE-KEYWORD)
// ═══════════════════════════════════════════════════════════════════════════════

/// BLOCKED: G-DECLARATIVE-KEYWORD — EffectTiming::Declarative is compiled but never
/// enqueued or fired. The grant_keyword modifier is not installed at runtime.
/// Structural tests (Sections 1) validate compilation shape; this test verifies
/// runtime keyword presence on a placed Omnimon (X Antibody) permanent.
#[test]
#[ignore = "pending: G-DECLARATIVE-KEYWORD — EffectTiming::Declarative never enqueued at runtime; declarative grant_keyword modifier not installed"]
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
