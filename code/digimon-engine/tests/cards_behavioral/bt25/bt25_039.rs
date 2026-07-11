//! BT25-039 Sirenmon — Digimon, Lv.5, Yellow/Green, DP 6000, Cost 6.
//! Traits: Shaman, Iliad, TS. Form: Ultimate. Attribute: Data.
//! Standard digivolution: Lv.4 Yellow / cost 4 AND Lv.4 Green / cost 4
//! (printed evo box, a genuine two-color evo circle).
//!
//! # Card text (data/card_bundles/BT25-039.md + card image — verbatim)
//!
//! [Digivolve] Lv.4 w/[TS] trait: Cost 3
//!
//! {Security} [End of Your Turn] You may play 1 [Ceresmon] from your hand
//! with the cost reduced by 7. If this effect played, you may place this
//! card as the played Digimon's bottom digivolution card.
//! [All Turns] When any of your other [Shaman] or [Iliad] trait Digimon or
//! Tamers would leave the battle area other than by your effects, by
//! deleting this Digimon, they don't leave.
//! [On Deletion] You may place this card face up as the bottom security card.
//!
//! Inherited: [Opponent's Turn] [Once Per Turn] When one of your opponent's
//! Digimon attacks, you may change the attack target to 1 of your suspended
//! Digimon.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Yellow/BT25_039.cs
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - F9/security: scope:security end_of_your_turn turn-boundary trigger on a
//!   card that stays in the persistent security stack (NEW pattern — not
//!   on_discard_security)
//! - Cost-reduced (not free) play-from-hand with a `bind_as` reference to the
//!   played permanent, then a self-placement tail (bottom digivolution card)
//! - Cross-permanent replacement: delete self to prevent an OTHER own
//!   Shaman/Iliad Digimon/Tamer from leaving the battle area, gated on
//!   `other: true` (self-exclusion) and `none_of: [replacement_cause: own_effect]`
//!   ("other than by your effects")
//! - [On Deletion] optional post-trash self-placement into security
//!   (event_card idiom, EX10-020 precedent)
//! - Inherited [Opponent's Turn][OPT] attack-redirect to a suspended own
//!   Digimon (BT25-055 idiom)
//! - Alt-digivolution Lv.4 [TS] -> cost 3, plus BOTH standard Lv.4/color
//!   circles (two-color evo)

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::{HAND_EFFECT_START, PASS, REPLACEMENT_ACCEPT};
use digimon_engine::card_data::CardData;
use digimon_engine::combat::{AttackInitiator, AttackOpen, TargetConstraint};
use digimon_engine::debug_runner::{
    make_test_card, make_test_card_with_level, DebugRunner, DebugRunnerBuilder,
};
use digimon_engine::enums::CardColor;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::{AttackTarget, SelectionKind};

const CARD_ID: &str = "BT25-039";

// ─── Fixture helpers ─────────────────────────────────────────────────────────

/// Ceresmon (BT3-056 shape): Lv.6, play cost 12. Reduced by 7 -> net cost 5.
fn ceresmon(card_id: &str) -> CardData {
    let mut card = make_test_card_with_level(card_id, "Ceresmon", 6);
    card.colors = vec![CardColor::Purple];
    card.dp = Some(9000);
    card.play_cost = 12;
    card
}

/// A non-Ceresmon Digimon — must NOT be a legal pick for the security clause.
fn other_digimon(card_id: &str) -> CardData {
    let mut card = make_test_card_with_level(card_id, "NotCeresmon", 5);
    card.colors = vec![CardColor::Blue];
    card.dp = Some(5000);
    card.play_cost = 6;
    card
}

/// An own [Shaman] Digimon — a legal cost-pool AND protected-subject fixture
/// for the [All Turns] leave-prevention clause.
fn shaman_digimon(card_id: &str) -> CardData {
    let mut card = make_test_card_with_level(card_id, card_id, 4);
    card.colors = vec![CardColor::Yellow];
    card.dp = Some(4000);
    card.traits = vec!["Shaman".to_string()];
    card
}

/// An own Digimon with neither [Shaman] nor [Iliad] — must NOT be protected.
fn plain_digimon(card_id: &str) -> CardData {
    let mut card = make_test_card_with_level(card_id, card_id, 4);
    card.colors = vec![CardColor::Blue];
    card.dp = Some(4000);
    card.traits = vec!["Beast".to_string()];
    card
}

fn opp_digimon(card_id: &str) -> CardData {
    let mut card = make_test_card_with_level(card_id, card_id, 5);
    card.colors = vec![CardColor::Red];
    card.dp = Some(9000);
    card
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-039 YAML parses and compiles")
        .add_card(ceresmon("CERESMON"))
        .add_card(other_digimon("OTHER-DIGI"))
        .add_card(shaman_digimon("SHAMAN-A"))
        .add_card(plain_digimon("PLAIN-A"))
        .add_card(opp_digimon("OPP-DIGI"))
        .add_card(make_test_card_with_level("FILL", "Filler", 3))
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn bt25_039_yaml_has_printed_metadata() {
    let runner = base().start();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("BT25-039 must be in the embedded DSL pack");

    assert_eq!(card.name, "Sirenmon");
    assert_eq!(card.level, Some(5));
    assert_eq!(card.dp, Some(6000));
    assert_eq!(card.cost, Some(6));
    for trait_name in ["Shaman", "Iliad", "TS"] {
        assert!(
            card.traits.contains(&trait_name.to_string()),
            "BT25-039 metadata must include trait {trait_name}"
        );
    }
}

#[test]
fn bt25_039_has_both_color_standard_digivolve_paths() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let has_yellow = card.alt_paths.iter().any(|p| {
        p.kind == CompiledAltPathKind::Digivolve
            && p.cost == Some(CompiledCost::Literal(4))
            && p.from.as_ref().is_some_and(|f| {
                f.level_eq == Some(4)
                    && f.color_is == Some(digimon_dsl::compiled::CompiledColor::Yellow)
            })
    });
    let has_green = card.alt_paths.iter().any(|p| {
        p.kind == CompiledAltPathKind::Digivolve
            && p.cost == Some(CompiledCost::Literal(4))
            && p.from.as_ref().is_some_and(|f| {
                f.level_eq == Some(4)
                    && f.color_is == Some(digimon_dsl::compiled::CompiledColor::Green)
            })
    });
    assert!(
        has_yellow,
        "BT25-039 must have a Lv.4 Yellow -> cost 4 path"
    );
    assert!(has_green, "BT25-039 must have a Lv.4 Green -> cost 4 path");
}

#[test]
fn bt25_039_has_ts_alt_digivolution_path() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let has_ts = card.alt_paths.iter().any(|p| {
        p.kind == CompiledAltPathKind::Digivolve
            && p.cost == Some(CompiledCost::Literal(3))
            && p.from
                .as_ref()
                .is_some_and(|f| f.level_eq == Some(4) && f.trait_has.as_deref() == Some("TS"))
    });
    assert!(has_ts, "BT25-039 must have a Lv.4 [TS] -> cost 3 alt-path");
}

#[test]
fn bt25_039_has_security_end_of_turn_clause() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    let has_security_eot = triggered.iter().any(|t| {
        t.scope == CompiledScope::Security && t.when.contains(&CompiledTiming::EndOfYourTurn)
    });
    assert!(
        has_security_eot,
        "must have {{Security}}[End of Your Turn] clause"
    );
}

#[test]
fn bt25_039_has_leave_prevention_replacement_clause() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let has_replacement = card.effects.iter().any(|c| match c {
        CompiledClause::Declarative(d) => match d {
            digimon_dsl::compiled::CompiledDeclarativeClause::Replacement {
                optional,
                process,
                trigger,
                ..
            } => {
                // Declarative `cost: { delete_self: true }` + `outcome: prevent`
                // lowers to the single self-contained DeleteSelfAndCancelLeave
                // step (NOT an explicit [DeletePermanent, CancelReplacement]
                // list, which the DelaySelfCancelFlow recognizer would
                // intercept and silently no-op for a Digimon carrier).
                *optional
                    && trigger == "when_would_leave_battle_area"
                    && process
                        .iter()
                        .any(|s| matches!(s, CompiledStep::DeleteSelfAndCancelLeave))
            }
            _ => false,
        },
        _ => false,
    });
    assert!(
        has_replacement,
        "must have a delete-self-to-prevent-leave replacement clause"
    );
}

#[test]
fn bt25_039_has_on_deletion_clause() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let has_on_deletion = card.effects.iter().any(|c| match c {
        CompiledClause::Triggered(t) => t.when.contains(&CompiledTiming::OnDeletion) && t.optional,
        _ => false,
    });
    assert!(
        has_on_deletion,
        "must have an optional [On Deletion] clause"
    );
}

#[test]
fn bt25_039_has_inherited_redirect_clause() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnOpponentAttack) => {
                Some(t)
            }
            _ => None,
        })
        .expect("must have inherited OnOpponentAttack redirect clause");
    assert_eq!(
        clause.scope,
        CompiledScope::Inherited,
        "redirect is inherited"
    );
    assert!(clause.once_per_turn, "redirect clause is OPT");
    assert!(clause.optional, "redirect clause is 'you may'");
}

// ─── Section 2 — {Security}[End of Your Turn]: structure + gating ───────────

/// Positive: with Sirenmon in the OWNER's security stack and Ceresmon in
/// hand, ending the owner's turn must install the optional hand-select
/// prompt. Note: memory stays at its default (0) throughout this file's
/// `end_turn()` tests — a nonzero memory lead for player 0 flips to negative
/// for player 1 after rotation (`rotate_turn_player`'s seesaw flip) and the
/// real "memory < 0 auto-ends the turn" rule (`Game::check_turn_end`) then
/// cascades an extra full round before the resolve call returns. Keeping
/// memory at 0 avoids that cascade so each test's assertions describe a
/// single, stable turn transition.
#[test]
fn bt25_039_end_of_turn_fires_while_in_security() {
    let mut runner = base()
        .hand(0, &["CERESMON"])
        .security(0, &["FILL", CARD_ID])
        .start();

    runner.end_turn();

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Hand),
        "the {{Security}}[End of Your Turn] clause must install the optional Ceresmon pick"
    );
    assert!(
        runner.pending_is_optional(),
        "the security-stack pick is 'you may'"
    );
    runner.execute_action(0, PASS).ok();
}

/// Negative: with Sirenmon NOT in security (e.g. still in hand), ending the
/// turn must not fire the security-scoped clause at all.
#[test]
fn bt25_039_end_of_turn_does_not_fire_outside_security() {
    let mut runner = base().hand(0, &[CARD_ID, "CERESMON"]).start();

    runner.end_turn();

    assert!(
        runner.pending_selection().is_none(),
        "a card sitting in hand (not security) must not fire the {{Security}} clause"
    );
}

/// Negative: with no Ceresmon in hand, the optional pick has no legal
/// candidate and must not install / must cleanly no-op.
#[test]
fn bt25_039_end_of_turn_no_ceresmon_in_hand_is_a_clean_no_op() {
    let mut runner = base()
        .hand(0, &["OTHER-DIGI"])
        .security(0, &["FILL", CARD_ID])
        .start();

    runner.end_turn();
    // Either no prompt installs, or an optional prompt with no legal
    // non-PASS target installs and can be declined cleanly.
    if runner.pending_selection().is_some() {
        runner
            .decline_optional_trigger()
            .expect("decline must be legal when nothing else qualifies");
    }
    assert!(
        runner.game.players[0]
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "OTHER-DIGI"),
        "OTHER-DIGI remains in hand, unplayed"
    );
}

// ─── Section 3 — {Security}[End of Your Turn]: behavioral outcomes ─────────

/// Accepting the Ceresmon pick plays it at cost -7 (net cost 5, not free) and
/// removes Ceresmon from hand. Memory is funded locally, right before the
/// pick resolves, via `set_memory` on the runner — this still triggers the
/// real memory-seesaw auto-pass cascade once the turn rotates (see the
/// note on `bt25_039_end_of_turn_fires_while_in_security`), so this test
/// asserts CERESMON's specific presence/cost-delta rather than absolute
/// hand size (which an extra auto-completed round would also change via a
/// subsequent draw).
#[test]
fn bt25_039_accepting_pick_plays_ceresmon_at_reduced_cost() {
    let mut runner = base()
        .hand(0, &["CERESMON"])
        .security(0, &["FILL", CARD_ID])
        .start();
    runner.end_turn();

    let view = runner
        .pending_selection_view()
        .expect("hand-pick prompt installed");
    assert_eq!(view.kind, SelectionKind::Hand);
    runner.game.set_memory(20);
    let memory_before = runner.memory();

    // Pick Ceresmon (the only hand card offered).
    let ceresmon_action = view.valid_action_ids[0];
    runner
        .execute_action(0, ceresmon_action)
        .expect("pick Ceresmon");
    // Net cost is 12 - 7 = 5, NOT free (0). Memory must have decreased by 5
    // at the moment of the play, before any further cascade/rotation.
    assert_eq!(
        runner.memory(),
        memory_before - 5,
        "Ceresmon plays at printed cost (12) reduced by 7 = 5, not for free"
    );
    assert!(
        !runner.game.players[0]
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "CERESMON"),
        "Ceresmon left the hand"
    );
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "CERESMON"),
        "Ceresmon was played to the battle area"
    );
    runner.auto_resolve().ok();
}

/// Declining the optional Ceresmon pick (PASS) leaves Ceresmon in hand and
/// the security stack untouched.
#[test]
fn bt25_039_declining_pick_plays_nothing() {
    let mut runner = base()
        .hand(0, &["CERESMON"])
        .security(0, &["FILL", CARD_ID])
        .start();
    runner.end_turn();

    assert_eq!(runner.pending_kind(), Some(SelectionKind::Hand));
    runner
        .execute_action(0, PASS)
        .expect("declining the optional Ceresmon pick is legal");

    assert!(
        runner.game.players[0]
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "CERESMON"),
        "Ceresmon stays in hand on decline"
    );
    assert_eq!(
        runner.game.players[0].security.len(),
        2,
        "security stack unchanged on decline"
    );
}

/// Accepting the Ceresmon play, then accepting the follow-up "place this
/// card as the played Digimon's bottom digivolution card" prompt, must move
/// Sirenmon out of security and onto Ceresmon's digivolution stack.
#[test]
fn bt25_039_accepting_placement_moves_self_under_played_ceresmon() {
    let mut runner = base()
        .hand(0, &["CERESMON"])
        .security(0, &["FILL", CARD_ID])
        .start();
    runner.end_turn();

    let view = runner
        .pending_selection_view()
        .expect("hand pick installed");
    runner.game.set_memory(20);
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("pick Ceresmon");

    // Follow-up: EffectChoice "place under" (label 0) vs "don't place" (label 1).
    let kind = runner.pending_kind().expect("follow-up placement prompt");
    assert_eq!(kind, SelectionKind::EffectChoice);
    runner
        .execute_branch(0)
        .expect("choose to place under Ceresmon");

    let security_after = runner.game.players[0].security.len();
    assert_eq!(
        security_after, 1,
        "Sirenmon left the security stack (only FILL remains)"
    );
    let ceresmon_perm = runner.game.players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&runner.game.card_data) == "CERESMON")
        .expect("Ceresmon on the battle area");
    assert!(
        ceresmon_perm
            .card_sources
            .iter()
            .any(|s| s.card_id(&runner.game.card_data) == CARD_ID),
        "Sirenmon must be a digivolution source under the played Ceresmon"
    );
    runner.auto_resolve().ok();
}

/// Accepting the Ceresmon play but DECLINING the follow-up placement prompt
/// leaves Sirenmon in the security stack.
#[test]
fn bt25_039_declining_placement_leaves_self_in_security() {
    let mut runner = base()
        .hand(0, &["CERESMON"])
        .security(0, &["FILL", CARD_ID])
        .start();
    runner.end_turn();

    let view = runner
        .pending_selection_view()
        .expect("hand pick installed");
    runner.game.set_memory(20);
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("pick Ceresmon");

    assert_eq!(runner.pending_kind(), Some(SelectionKind::EffectChoice));
    runner
        .execute_branch(1)
        .expect("choose NOT to place under Ceresmon");

    assert_eq!(
        runner.game.players[0].security.len(),
        2,
        "Sirenmon stays in security when the placement is declined"
    );
    runner.auto_resolve().ok();
}

/// A non-Ceresmon Digimon in hand must not be offered by the security clause.
#[test]
fn bt25_039_non_ceresmon_hand_card_is_not_offered() {
    let mut runner = base()
        .hand(0, &["OTHER-DIGI"])
        .security(0, &["FILL", CARD_ID])
        .start();
    runner.end_turn();

    if let Some(view) = runner.pending_selection_view() {
        assert!(
            view.valid_action_ids.is_empty() || runner.pending_is_optional(),
            "with no legal Ceresmon target the pick must be a clean decline-only prompt"
        );
        runner.execute_action(0, PASS).ok();
    }
    assert!(
        runner.game.players[0]
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "OTHER-DIGI"),
        "OTHER-DIGI stays in hand, unplayed"
    );
}

// ─── Section 4 — [All Turns] leave-prevention replacement ───────────────────

/// Positive: an OTHER own [Shaman]-trait Digimon would be deleted by an
/// opponent effect. Sirenmon's replacement offers to delete itself instead.
#[test]
fn bt25_039_offers_self_delete_to_protect_other_shaman_digimon() {
    let mut runner = base().start();
    let victim = runner.place_on_field(0, "SHAMAN-A", Some(0));
    let siren = runner.place_on_field(0, CARD_ID, Some(0));

    runner
        .game
        .delete_permanent_with_cause(victim, ReplacementCause::OpponentEffect);

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("Sirenmon's leave-prevention replacement must offer to fire");
    assert_eq!(pending.kind, SelectionKind::Replacement);
    assert!(pending.valid_action_ids.contains(&REPLACEMENT_ACCEPT));

    runner
        .game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept Sirenmon's self-delete protection");

    assert!(
        field_contains(&runner, 0, "SHAMAN-A"),
        "accepting keeps the threatened Shaman Digimon on the field"
    );
    assert!(
        !field_contains(&runner, 0, CARD_ID),
        "Sirenmon paid the cost by deleting itself"
    );
}

/// Declining the replacement lets the original deletion proceed.
#[test]
fn bt25_039_declining_protection_allows_original_deletion() {
    let mut runner = base().start();
    let victim = runner.place_on_field(0, "SHAMAN-A", Some(0));
    let siren = runner.place_on_field(0, CARD_ID, Some(0));

    runner
        .game
        .delete_permanent_with_cause(victim, ReplacementCause::OpponentEffect);
    assert!(runner.game.pending_selection.is_some());

    runner
        .game
        .resolve_selection(0, PASS)
        .expect("decline Sirenmon's self-delete protection");

    assert!(
        !field_contains(&runner, 0, "SHAMAN-A"),
        "declining allows the Shaman Digimon to be deleted"
    );
    assert!(
        field_contains(&runner, 0, CARD_ID),
        "declining does not pay Sirenmon's self-delete cost"
    );
}

/// Negative: a plain (non-Shaman/non-Iliad) own Digimon must NOT be
/// protected by this clause.
#[test]
fn bt25_039_does_not_protect_non_shaman_iliad_digimon() {
    let mut runner = base().start();
    let victim = runner.place_on_field(0, "PLAIN-A", Some(0));
    let siren = runner.place_on_field(0, CARD_ID, Some(0));

    runner
        .game
        .delete_permanent_with_cause(victim, ReplacementCause::OpponentEffect);

    assert!(
        runner.game.pending_selection.is_none(),
        "a Digimon without [Shaman]/[Iliad] must not trigger Sirenmon's protection"
    );
    assert!(!field_contains(&runner, 0, "PLAIN-A"));
    assert!(field_contains(&runner, 0, CARD_ID));
}

/// Negative: Sirenmon itself would leave (it also carries Shaman/Iliad) —
/// the "your OTHER ... Digimon" self-exclusion must prevent it from
/// protecting itself via this clause. The leave-prevention replacement must
/// not park a prompt for this deletion (the self-exclusion gates it out at
/// the replacement-candidate stage) — but Sirenmon's OWN, separate
/// `[On Deletion]` ability (a different clause entirely: "you may place
/// this card face up as the bottom security card") legitimately fires once
/// the normal deletion completes, so this test declines THAT prompt rather
/// than asserting no prompt installs at all.
#[test]
fn bt25_039_does_not_protect_itself() {
    let mut runner = base().start();
    let siren = runner.place_on_field(0, CARD_ID, Some(0));

    runner
        .game
        .delete_permanent_with_cause(siren, ReplacementCause::OpponentEffect);
    runner.game.drain_effect_queue();

    // Sirenmon is deleted normally (the leave-prevention replacement did not
    // fire for its own removal) — its [On Deletion] clause now offers the
    // separate face-up-security placement (surfaced as an outer optional
    // accept/decline prompt); decline it.
    if runner.game.pending_selection.is_some() {
        assert!(
            runner.pending_is_optional(),
            "only Sirenmon's unrelated, optional [On Deletion] prompt may be pending here"
        );
        runner.decline_optional_trigger().ok();
    }
    assert!(
        !field_contains(&runner, 0, CARD_ID),
        "with no self-protection available, Sirenmon is deleted normally"
    );
}

/// Negative: "other than by your effects" — when the OWNER's own effect
/// causes the leave, the protection must not fire.
#[test]
fn bt25_039_does_not_protect_against_own_effect_deletion() {
    let mut runner = base().start();
    let victim = runner.place_on_field(0, "SHAMAN-A", Some(0));
    let siren = runner.place_on_field(0, CARD_ID, Some(0));

    runner
        .game
        .delete_permanent_with_cause(victim, ReplacementCause::OwnEffect);

    assert!(
        runner.game.pending_selection.is_none(),
        "a deletion caused by the OWNER's own effect must not be protected (\"other than by your effects\")"
    );
    assert!(!field_contains(&runner, 0, "SHAMAN-A"));
    assert!(field_contains(&runner, 0, CARD_ID));
}

/// Clone-safety (rule 28 / make-engine-cloneable): the parked replacement
/// accept prompt must survive `Game::clone` and resolve on the CLONE via the
/// resumable data VM (no closure-park panic). Mirrors the GREEN fixture's
/// `bt25_039_clone_safe_mid_park` (tests/dsl/protect_others_leave_replacement.rs)
/// on the real card.
#[test]
fn bt25_039_protection_prompt_is_clone_safe_mid_park() {
    let mut runner = base().start();
    let victim = runner.place_on_field(0, "SHAMAN-A", Some(0));
    let _siren = runner.place_on_field(0, CARD_ID, Some(0));

    runner
        .game
        .delete_permanent_with_cause(victim, ReplacementCause::OpponentEffect);
    assert!(
        runner.game.pending_selection.is_some(),
        "parked accept prompt"
    );

    // Clone the parked game and drive the CLONE to accept — no closure-park
    // panic, and the clone's protection resolves fully.
    let mut cloned = runner.game.clone();
    let (aid, sp) = {
        let p = cloned
            .pending_selection
            .as_ref()
            .expect("clone kept the prompt");
        (p.valid_action_ids[0], p.selecting_player)
    };
    cloned.resolve_selection(sp, aid).expect("clone accepts");
    // Unlike the GREEN fixture's synthetic carrier, the REAL Sirenmon has its
    // own [On Deletion] clause ("you may place this card face up as the
    // bottom security card") which fires after the delete-self cost is paid —
    // a follow-up optional prompt on the clone. Decline it (still on the
    // clone — proving the whole chained flow is closure-free).
    if let Some(p) = cloned.pending_selection.as_ref() {
        assert!(
            p.is_optional,
            "only Sirenmon's optional [On Deletion] follow-up may be pending"
        );
        let (decline_sp, decline_aid) = (p.selecting_player, PASS);
        cloned
            .resolve_selection(decline_sp, decline_aid)
            .expect("clone declines the [On Deletion] follow-up");
    }
    assert!(cloned.pending_selection.is_none(), "clone resolved");
    assert!(
        cloned.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&cloned.card_data) == "SHAMAN-A"),
        "clone protected the subject via the resumable VM"
    );
    assert!(
        !cloned.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&cloned.card_data) == CARD_ID),
        "clone's Sirenmon paid the delete-self cost"
    );
}

/// Negative: an opponent's Shaman/Iliad Digimon (not the owner's) must not
/// be protected by Sirenmon's clause (the printed text says "your other").
#[test]
fn bt25_039_does_not_protect_opponents_digimon() {
    let mut runner = base().start();
    let opp_victim = runner.place_on_field(1, "SHAMAN-A", Some(0));
    let siren = runner.place_on_field(0, CARD_ID, Some(0));

    runner
        .game
        .delete_permanent_with_cause(opp_victim, ReplacementCause::OpponentEffect);

    assert!(
        runner.game.pending_selection.is_none(),
        "Sirenmon must not protect an opponent's Digimon"
    );
    assert!(!field_contains(&runner, 1, "SHAMAN-A"));
    assert!(field_contains(&runner, 0, CARD_ID));
}

// ─── Section 5 — [On Deletion] optional self-to-security ───────────────────

/// Accepting the [On Deletion] prompt moves Sirenmon (now in trash,
/// post-batched-deletion) to the bottom of the owner's security stack,
/// face up.
#[test]
fn bt25_039_on_deletion_accept_places_self_face_up_at_security_bottom() {
    let mut runner = base().start();
    let siren = runner.place_on_field(0, CARD_ID, Some(0));

    runner
        .game
        .delete_permanent_with_cause(siren, ReplacementCause::OpponentEffect);

    // Drain to reach the OnDeletion drain (post-trash, rule 25).
    runner.game.drain_effect_queue();

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("[On Deletion] optional prompt must install");
    assert!(pending.is_optional, "[On Deletion] is 'you may'");

    runner
        .accept_optional_trigger()
        .expect("accept the [On Deletion] self-security placement");
    runner.auto_resolve().ok();

    let security = &runner.game.players[0].security;
    assert_eq!(
        security.first().map(|c| c.card_id(&runner.game.card_data)),
        Some(CARD_ID),
        "Sirenmon must be the BOTTOM security card (index 0) after acceptance"
    );
    assert!(
        runner.game.players[0]
            .face_up_security
            .contains(&security[0].card_index),
        "the placed card must be face UP"
    );
}

/// Declining the [On Deletion] prompt leaves Sirenmon in the trash.
#[test]
fn bt25_039_on_deletion_decline_leaves_self_in_trash() {
    let mut runner = base().start();
    let siren = runner.place_on_field(0, CARD_ID, Some(0));

    runner
        .game
        .delete_permanent_with_cause(siren, ReplacementCause::OpponentEffect);
    runner.game.drain_effect_queue();

    assert!(runner.game.pending_selection.is_some());
    runner
        .decline_optional_trigger()
        .expect("decline the [On Deletion] self-security placement");

    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == CARD_ID),
        "declining leaves Sirenmon in the trash"
    );
    assert!(
        runner.game.players[0].security.is_empty(),
        "declining does not add anything to security"
    );
}

// ─── Section 6 — Inherited [Opponent's Turn][OPT] attack redirect ──────────
//
// "Inherited Effect" abilities only function while the carrier is BURIED as
// a digivolution source under a different top card — NOT while it stands
// alone as its own permanent's top card (`Game::activation_counts_for_
// permanent`'s `is_under != effect.inherited` gate; BT19-065 precedent,
// `bt19_065_inherited_redirect_does_not_fire_when_standalone_top_card`).
// Every test below buries Sirenmon under a filler top card via `place_stack`.
//
// `opponents_turn: true` (this clause's `active_when`) reads
// `Game::turn_player()`, which is driven by `turn_player_idx` (rotated by
// `end_turn`/`rotate_turn_player`) — NOT by directly assigning `turn_count`
// (a display counter `opponents_turn` never reads). `runner.end_turn()`
// (memory left at its 0 default, so the seesaw flip doesn't go negative and
// cascade an extra auto-passed round — see the note in Section 3) rotates
// turn_player_idx from player 0 to player 1, making this genuinely "player
// 1's turn" (the opponent of player 0, who owns Sirenmon) before the attack
// fires.

/// Accepting the redirect on an opponent's attack switches the defender to
/// 1 of the controller's suspended Digimon (security is not checked). The
/// clause's FIRST process step is itself an optional `select_own_permanent`
/// (`G-OUTER-OPTIONAL-NOT-INSTALLED`: an inner-optional first step supplies
/// its own decline path, so there is no SEPARATE outer accept/decline
/// followed by a second target-pick prompt — `accept_optional_trigger`
/// resolves the single `OwnField` selection directly via its first legal
/// action id).
#[test]
fn bt25_039_inherited_redirect_switches_attack_to_suspended_own_digimon() {
    let mut runner = base().security(0, &["FILL"]).start();
    runner.end_turn(); // rotate to player 1's turn

    // Sirenmon buried under a filler top card so its Inherited Effect is
    // live; a separate suspended Digimon is the redirect target.
    let _stack = runner.place_stack(0, &[CARD_ID, "PLAIN-A"]);
    let suspended_target = runner.place_on_field(0, "SHAMAN-A", Some(0));
    runner.game.suspend(suspended_target);
    let attacker = runner.place_on_field(1, "OPP-DIGI", Some(0));
    let security_before = runner.game.players[0].security.len();

    runner.game.begin_attack_open(AttackOpen {
        attacker,
        initiator: AttackInitiator::Effect {
            source: None,
            optional: false,
        },
        suspend_attacker: false,
        ignore_summoning_sickness: false,
        target_constraint: TargetConstraint::Forced(AttackTarget::Player(0)),
        allow_cancel: false,
        cost_upgrade: None,
    });

    assert!(
        runner.pending_is_optional(),
        "the inherited redirect is an optional accept/decline"
    );
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::OwnField),
        "the single select_own_permanent prompt offers the suspended target directly"
    );
    runner
        .accept_optional_trigger()
        .expect("accept the redirect (resolves the sole legal suspended target)");
    runner.auto_resolve().ok();

    assert_eq!(
        runner.game.players[0].security.len(),
        security_before,
        "redirecting the attack must prevent the security check"
    );
    assert!(
        !runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "SHAMAN-A"),
        "the redirected attack (9000 DP attacker vs 4000 DP SHAMAN-A) deletes the redirect target"
    );
}

/// Declining the redirect lets the attack proceed to the original defender
/// (the player's security).
#[test]
fn bt25_039_inherited_redirect_decline_leaves_attack_on_player() {
    let mut runner = base().security(0, &["FILL"]).start();
    runner.end_turn(); // rotate to player 1's turn

    let _stack = runner.place_stack(0, &[CARD_ID, "PLAIN-A"]);
    let suspended_target = runner.place_on_field(0, "SHAMAN-A", Some(0));
    runner.game.suspend(suspended_target);
    let attacker = runner.place_on_field(1, "OPP-DIGI", Some(0));
    let security_before = runner.game.players[0].security.len();

    runner.game.begin_attack_open(AttackOpen {
        attacker,
        initiator: AttackInitiator::Effect {
            source: None,
            optional: false,
        },
        suspend_attacker: false,
        ignore_summoning_sickness: false,
        target_constraint: TargetConstraint::Forced(AttackTarget::Player(0)),
        allow_cancel: false,
        cost_upgrade: None,
    });

    assert!(runner.pending_is_optional());
    runner
        .decline_optional_trigger()
        .expect("decline the inherited redirect prompt");
    runner.auto_resolve().ok();

    assert!(
        runner.game.players[0].security.len() <= security_before,
        "declining lets the attack resolve against the player's security as normal"
    );
}

/// Negative: with NO suspended own Digimon, the redirect must not offer
/// (nothing to redirect to).
#[test]
fn bt25_039_inherited_redirect_does_not_offer_with_no_suspended_digimon() {
    let mut runner = base().security(0, &["FILL"]).start();
    runner.end_turn(); // rotate to player 1's turn

    let _stack = runner.place_stack(0, &[CARD_ID, "PLAIN-A"]);
    // No suspended Digimon on player 0's field.
    let attacker = runner.place_on_field(1, "OPP-DIGI", Some(0));

    runner.game.begin_attack_open(AttackOpen {
        attacker,
        initiator: AttackInitiator::Effect {
            source: None,
            optional: false,
        },
        suspend_attacker: false,
        ignore_summoning_sickness: false,
        target_constraint: TargetConstraint::Forced(AttackTarget::Player(0)),
        allow_cancel: false,
        cost_upgrade: None,
    });

    // With no legal suspended target, the OPT clause must not install an
    // actionable prompt (or must expose only a decline).
    if runner.pending_selection().is_some() {
        runner.decline_optional_trigger().ok();
        runner.auto_resolve().ok();
    }
    assert_eq!(
        runner.game.players[0].security.len(),
        0,
        "with no redirect available, the attack resolves normally against the player's (empty) security"
    );
}

/// OPT lockout: a second opponent attack in the same turn must not re-offer
/// the redirect.
#[test]
fn bt25_039_inherited_redirect_is_once_per_turn_lockout() {
    let mut runner = base().security(0, &["FILL", "FILL"]).start();
    runner.end_turn(); // rotate to player 1's turn

    let _stack = runner.place_stack(0, &[CARD_ID, "PLAIN-A"]);
    let suspended_target = runner.place_on_field(0, "SHAMAN-A", Some(0));
    runner.game.suspend(suspended_target);
    let attacker = runner.place_on_field(1, "OPP-DIGI", Some(0));

    // First attack: accept the redirect.
    runner.game.begin_attack_open(AttackOpen {
        attacker,
        initiator: AttackInitiator::Effect {
            source: None,
            optional: false,
        },
        suspend_attacker: false,
        ignore_summoning_sickness: false,
        target_constraint: TargetConstraint::Forced(AttackTarget::Player(0)),
        allow_cancel: false,
        cost_upgrade: None,
    });
    if runner.pending_is_optional() {
        runner.accept_optional_trigger().ok();
        runner.auto_resolve().ok();
    }

    // Unsuspend + re-suspend so a target is available again, then attack a
    // second time in the SAME turn — the OPT lockout must block re-offering.
    runner.game.unsuspend(suspended_target);
    runner.game.suspend(suspended_target);
    let attacker2 = runner.place_on_field(1, "OPP-DIGI", Some(1));
    let security_before = runner.game.players[0].security.len();

    runner.game.begin_attack_open(AttackOpen {
        attacker: attacker2,
        initiator: AttackInitiator::Effect {
            source: None,
            optional: false,
        },
        suspend_attacker: false,
        ignore_summoning_sickness: false,
        target_constraint: TargetConstraint::Forced(AttackTarget::Player(0)),
        allow_cancel: false,
        cost_upgrade: None,
    });

    assert!(
        runner.pending_selection().is_none()
            || !runner.pending_is_optional()
            || runner.pending_kind() != Some(SelectionKind::OwnField),
        "a second opponent attack in the same turn must not re-offer the OPT redirect"
    );
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn field_contains(r: &DebugRunner, player: u8, card_id: &str) -> bool {
    r.game.player(player).battle_area.iter().any(|perm| {
        perm.card_sources
            .iter()
            .any(|card| card.card_id(&r.game.card_data) == card_id)
    })
}
