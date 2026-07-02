//! BT22-041 Kentaurosmon — Digimon, Lv.6, Yellow, DP 12000, Cost 12.
//! Traits: Holy Warrior, Royal Knight, CS. Attribute: Vaccine.
//!
//! # Card text (cards.json — authoritative for printed text)
//!
//! ```text
//! When this card would be played, if there are 6 or fewer total cards in both
//!   players' security stacks, reduce the play cost by 6.
//! <Blocker>
//! <Barrier>
//! [On Play] [When Digivolving] You may place 1 yellow card from your hand as
//!   your top security card.
//! [All Turns] [Once Per Turn] When this Digimon suspends, by trashing your top
//!   security card, it unsuspends.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT22/Yellow/BT22_041.cs
//!
//! # Self-scoped on_suspend
//! "When this Digimon suspends" gates the OnSuspend observer to the event
//! permanent being THIS exact Kentaurosmon, via `event_permanent_is_source:
//! true` (the BT23-077 Sistermon Ciel pattern). Another permanent suspending
//! must NOT fire it.
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - D2 conditional cost reduction (total-security-sum gate, BT25-044 idiom)
//! - G3 self-scoped OnSuspend observer (`event_permanent_is_source`)
//! - E2 OPT + optional decline ("by trashing your top security card" is opt-in)

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledPredicate, CompiledScope, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::CardKind;

const CARD_ID: &str = "BT22-041";

fn pad(id: &str) -> CardData {
    make_test_card(id, id)
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT22-041 must load from embedded DSL pack")
        .add_card(pad("PAD"))
        .add_card({
            let mut c = make_test_card("OWN-OTHER", "OWN-OTHER");
            c.card_kind = CardKind::Digimon;
            c.level = Some(4);
            c.dp = Some(4000);
            c
        })
        .deck(0, &["PAD"; 12])
        .deck(1, &["PAD"; 12])
}

fn predicate_has_event_permanent_is_source(predicate: &CompiledPredicate) -> bool {
    predicate.event_permanent_is_source == Some(true)
        || predicate
            .all_of
            .iter()
            .any(predicate_has_event_permanent_is_source)
        || predicate
            .any_of
            .iter()
            .any(predicate_has_event_permanent_is_source)
        || predicate
            .none_of
            .iter()
            .any(predicate_has_event_permanent_is_source)
        || predicate
            .not
            .as_ref()
            .is_some_and(|p| predicate_has_event_permanent_is_source(p))
}

// ─── Structural / load ───────────────────────────────────────────────────────

#[test]
fn bt22_041_loads_keyword_security_place_slice() {
    base().start();
}

#[test]
fn bt22_041_has_cost_reduction_and_self_suspend_unsuspend_clauses() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled card");

    assert!(
        card.effects.iter().any(|c| matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction { .. })
        )),
        "total-security cost reduction clause present"
    );

    let on_suspend = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(t)
                if t.scope == CompiledScope::FaceUp
                    && t.when == vec![CompiledTiming::OnSuspend] =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("self-suspend unsuspend clause should be authored");
    assert!(on_suspend.once_per_turn, "[Once Per Turn]");
    assert!(
        on_suspend.optional,
        "'by trashing your top security card' is an opt-in cost"
    );
    assert!(
        on_suspend
            .condition
            .as_ref()
            .is_some_and(predicate_has_event_permanent_is_source),
        "OnSuspend must be gated to the suspending event permanent being this Kentaurosmon"
    );
}

// ─── Cost reduction: -6 when total security <= 6 ─────────────────────────────

#[test]
fn bt22_041_cost_reduced_by_6_when_total_security_6_or_fewer() {
    // own 3 + opp 3 = 6 total → reduce play cost by 6 (12 → 6). With 6 memory
    // available, the play succeeds and leaves memory at 0.
    let mut runner = base()
        .hand(0, &[CARD_ID])
        .security(0, &["PAD"; 3])
        .security(1, &["PAD"; 3])
        .memory(6)
        .start();

    runner
        .play(0, 0)
        .expect("play Kentaurosmon at reduced cost");

    assert_eq!(runner.battle_area_size(0), 1, "Kentaurosmon is in play");
    assert_eq!(
        runner.memory(),
        0,
        "cost reduced from 12 to 6: spent 6 of 6 memory (own+opp security == 6)"
    );
}

#[test]
fn bt22_041_no_cost_reduction_when_total_security_7_or_more() {
    // own 4 + opp 3 = 7 total → no reduction. With only 6 memory the full cost
    // of 12 cannot be paid from 0; memory should go negative (12 spent).
    let mut runner = base()
        .hand(0, &[CARD_ID])
        .security(0, &["PAD"; 4])
        .security(1, &["PAD"; 3])
        .memory(6)
        .start();

    runner.play(0, 0).expect("play Kentaurosmon at full cost");

    assert_eq!(
        runner.memory(),
        -6,
        "no reduction: full cost 12 spent from 6 memory → -6 (7 total security)"
    );
}

// ─── Self-suspend → trash top security → unsuspend ───────────────────────────

#[test]
fn bt22_041_self_suspend_offers_optional_unsuspend_via_security_trash() {
    let mut runner = base().security(0, &["PAD"; 3]).memory(10).start();
    let kentaurosmon = runner.place_on_field(0, CARD_ID, Some(0));

    runner.game.suspend(kentaurosmon);

    assert!(
        runner.pending_is_optional(),
        "'by trashing your top security card' must surface an opt-in (declinable) prompt"
    );
}

#[test]
fn bt22_041_self_suspend_accept_trashes_top_security_and_unsuspends() {
    let mut runner = base().security(0, &["PAD"; 3]).memory(10).start();
    let kentaurosmon = runner.place_on_field(0, CARD_ID, Some(0));
    let sec_before = runner.security_count(0);

    runner.game.suspend(kentaurosmon);
    assert!(runner.game.players[0].battle_area[0].is_suspended);

    // Accept the cost: drive the first legal (non-PASS) action.
    let view = runner
        .pending_selection_view()
        .expect("optional unsuspend prompt installs");
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("accept trashing top security");
    runner.auto_resolve().expect("finish unsuspend");

    assert_eq!(
        runner.security_count(0),
        sec_before - 1,
        "top security card is trashed as the cost"
    );
    assert!(
        !runner.game.players[0].battle_area[0].is_suspended,
        "Kentaurosmon unsuspends after paying the security cost"
    );
}

#[test]
fn bt22_041_self_suspend_decline_keeps_suspended_and_security_intact() {
    let mut runner = base().security(0, &["PAD"; 3]).memory(10).start();
    let kentaurosmon = runner.place_on_field(0, CARD_ID, Some(0));
    let sec_before = runner.security_count(0);

    runner.game.suspend(kentaurosmon);
    assert!(runner.pending_is_optional(), "decline must be legal");
    runner
        .execute_action(0, digimon_engine::action::space::PASS)
        .expect("decline the optional cost");
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.security_count(0),
        sec_before,
        "declining trashes no security"
    );
    assert!(
        runner.game.players[0].battle_area[0].is_suspended,
        "declining leaves Kentaurosmon suspended"
    );
}

#[test]
fn bt22_041_other_permanent_suspending_does_not_fire_clause() {
    // NEGATIVE: a different permanent suspending must NOT trigger Kentaurosmon's
    // self-scoped OnSuspend unsuspend clause.
    let mut runner = base().security(0, &["PAD"; 3]).memory(10).start();
    let _kentaurosmon = runner.place_on_field(0, CARD_ID, Some(0));
    let other = runner.place_on_field(0, "OWN-OTHER", Some(0));

    runner.game.suspend(other);

    assert!(
        runner.pending_selection().is_none(),
        "another permanent suspending must not trigger BT22-041's self-suspend clause"
    );
}

#[test]
fn bt22_041_self_suspend_with_no_security_installs_no_prompt() {
    // NEGATIVE: no top security card means the cost can't be paid, so no prompt.
    let mut runner = base().memory(10).start();
    let kentaurosmon = runner.place_on_field(0, CARD_ID, Some(0));

    runner.game.suspend(kentaurosmon);

    assert!(
        runner.pending_selection().is_none(),
        "self-suspend with no security to trash installs no unsuspend prompt"
    );
}
