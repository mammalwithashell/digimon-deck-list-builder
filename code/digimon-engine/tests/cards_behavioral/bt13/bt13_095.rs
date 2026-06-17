//! BT13-095 Marcus Damon — Tamer, Red/Yellow, Cost 5.
//!
//! # Card text (cards.json — authoritative for printed text)
//!
//! ```text
//! [Start of Your Turn] If you have 2 or less memory, set it to 3.
//! [On Play] You may suspend this Tamer.
//! [All Turns] When this Tamer becomes suspended, 1 of your opponent's Digimon
//!   gets -3000 DP for the turn. Then, if one of your Digimon has [Agumon] or
//!   [Greymon] in its name, gain 1 memory.
//! [Security] Play this card without paying the cost.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT13/Red/BT13_095.cs
//!
//! # Self-scoped on_suspend
//! "When this Tamer becomes suspended" gates the OnSuspend observer to the event
//! permanent being THIS exact Marcus, modeled via `event_permanent_is_source:
//! true` (the BT23-077 Sistermon Ciel pattern). Another permanent suspending
//! must NOT fire it.
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - G3 self-scoped OnSuspend observer (`event_permanent_is_source`)
//! - Mandatory opponent Digimon -3000 DP selection
//! - Conditional memory gain gated on an own [Agumon]/[Greymon] Digimon

use digimon_dsl::compiled::{
    CompiledClause, CompiledPredicate, CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::encode_attack;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "BT13-095";

fn make_digimon(id: &str, name: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(4000);
    card.colors = vec![CardColor::Red];
    card
}

fn make_tamer(id: &str, name: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Tamer;
    card.colors = vec![CardColor::Red];
    card
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT13-095 must load from embedded DSL pack")
        .add_card(make_digimon("AGUMON-DIGI", "Agumon"))
        .add_card(make_digimon("GREYMON-DIGI", "Greymon"))
        .add_card(make_digimon("PLAIN-DIGI", "Veemon"))
        .add_card(make_digimon("OPP-DIGI", "Numemon"))
        .add_card(make_digimon("OPP-DIGI2", "Goblimon"))
        .add_card(make_tamer("OTHER-TAMER", "Sora Takenouchi"))
        .memory(5)
}

fn encode_permanent(handle: PermanentHandle) -> u16 {
    encode_attack(0, handle.index as u16)
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

// ─── Structural ──────────────────────────────────────────────────────────────

#[test]
fn bt13_095_has_memory_on_play_security_and_self_suspend_clauses() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled card");

    for timing in [
        CompiledTiming::StartOfYourTurn,
        CompiledTiming::OnPlay,
        CompiledTiming::OnSecurity,
    ] {
        assert!(
            card.effects.iter().any(|clause| matches!(
                clause,
                CompiledClause::Triggered(t) if t.when.contains(&timing)
            )),
            "BT13-095 must have {timing:?} clause"
        );
    }

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
        .expect("self-suspend debuff clause should be authored");
    assert!(
        on_suspend
            .condition
            .as_ref()
            .is_some_and(predicate_has_event_permanent_is_source),
        "OnSuspend must be gated to the suspending event permanent being this Marcus"
    );
}

// ─── On Play (regression — existing behavior preserved) ──────────────────────

#[test]
fn bt13_095_on_play_can_suspend_self() {
    let mut runner = base().hand(0, &[CARD_ID]).start();

    runner.play(0, 0).expect("play Marcus");
    let _ = runner.auto_resolve();

    assert!(
        runner.game.players[0].battle_area[0].is_suspended,
        "BT13-095 On Play should be able to suspend Marcus"
    );
}

// ─── Self-suspend: positive + negative gating ────────────────────────────────

#[test]
fn bt13_095_self_suspend_prompts_for_opponent_digimon_debuff() {
    let mut runner = base().start();
    let marcus = runner.place_on_field(0, CARD_ID, Some(0));
    let opp_a = runner.place_on_field(1, "OPP-DIGI", Some(0));
    let opp_b = runner.place_on_field(1, "OPP-DIGI2", Some(0));

    runner.game.suspend(marcus);

    let view = runner
        .pending_selection_view()
        .expect("self-suspend installs opponent Digimon debuff selection");
    assert_eq!(view.kind, SelectionKind::OppField);
    assert!(
        !view.is_optional,
        "printed '-3000 DP' debuff is mandatory when a legal target exists"
    );
    let mut expected = vec![encode_permanent(opp_a), encode_permanent(opp_b)];
    expected.sort();
    let mut got = view.valid_action_ids.clone();
    got.sort();
    assert_eq!(got, expected, "both opponent Digimon are legal -3000 targets");
}

#[test]
fn bt13_095_other_permanent_suspending_does_not_fire_clause() {
    // NEGATIVE: a different permanent suspending must NOT trigger Marcus's
    // self-scoped OnSuspend observer.
    let mut runner = base().start();
    let _marcus = runner.place_on_field(0, CARD_ID, Some(0));
    let other_tamer = runner.place_on_field(0, "OTHER-TAMER", Some(0));
    runner.place_on_field(1, "OPP-DIGI", Some(0));

    runner.game.suspend(other_tamer);

    assert!(
        runner.pending_selection().is_none(),
        "another permanent suspending must not trigger BT13-095's self-suspend clause"
    );
}

#[test]
fn bt13_095_self_suspend_applies_minus_3000_to_chosen_opponent_digimon() {
    let mut runner = base().start();
    let marcus = runner.place_on_field(0, CARD_ID, Some(0));
    let opp = runner.place_on_field(1, "OPP-DIGI", Some(0));

    let dp_before = runner.effective_dp(opp).expect("opp dp");
    runner.game.suspend(marcus);
    let view = runner
        .pending_selection_view()
        .expect("debuff selection installs");
    runner
        .execute_action(view.selecting_player, encode_permanent(opp))
        .expect("choose opponent Digimon");
    runner.auto_resolve().expect("finish self-suspend effect");

    assert_eq!(
        runner.game.modifiers.sum(opp, ModifierType::ChangeDp),
        -3000,
        "chosen opponent Digimon gets -3000 DP"
    );
    assert_eq!(
        runner.effective_dp(opp).expect("opp dp"),
        dp_before - 3000,
        "effective DP drops by 3000"
    );
}

// ─── Conditional memory gain ─────────────────────────────────────────────────

#[test]
fn bt13_095_self_suspend_gains_memory_with_agumon_or_greymon_digimon() {
    let mut runner = base().start();
    let marcus = runner.place_on_field(0, CARD_ID, Some(0));
    let opp = runner.place_on_field(1, "OPP-DIGI", Some(0));
    // An own Digimon with [Greymon] in its name satisfies the conditional gain.
    runner.place_on_field(0, "GREYMON-DIGI", Some(0));

    let memory_before = runner.memory();
    runner.game.suspend(marcus);
    let view = runner
        .pending_selection_view()
        .expect("debuff selection installs");
    runner
        .execute_action(view.selecting_player, encode_permanent(opp))
        .expect("choose opponent Digimon");
    runner.auto_resolve().expect("finish self-suspend effect");

    assert_eq!(
        runner.memory() - memory_before,
        1,
        "gain 1 memory when you have a [Greymon]-named Digimon"
    );
}

#[test]
fn bt13_095_self_suspend_no_memory_without_agumon_or_greymon_digimon() {
    // NEGATIVE half of the conditional: a plain own Digimon (no Agumon/Greymon
    // in its name) does not satisfy the memory gain.
    let mut runner = base().start();
    let marcus = runner.place_on_field(0, CARD_ID, Some(0));
    let opp = runner.place_on_field(1, "OPP-DIGI", Some(0));
    runner.place_on_field(0, "PLAIN-DIGI", Some(0));

    let memory_before = runner.memory();
    runner.game.suspend(marcus);
    let view = runner
        .pending_selection_view()
        .expect("debuff selection installs");
    runner
        .execute_action(view.selecting_player, encode_permanent(opp))
        .expect("choose opponent Digimon");
    runner.auto_resolve().expect("finish self-suspend effect");

    assert_eq!(
        runner.memory(),
        memory_before,
        "no memory gain without an [Agumon]/[Greymon] Digimon"
    );
}

#[test]
fn bt13_095_self_suspend_with_no_opponent_digimon_installs_no_prompt() {
    // NEGATIVE: this Marcus suspends but the opponent controls no Digimon, so
    // the mandatory -3000 debuff has nothing to target and no selection installs.
    let mut runner = base().start();
    let marcus = runner.place_on_field(0, CARD_ID, Some(0));

    runner.game.suspend(marcus);

    assert!(
        runner.pending_selection().is_none(),
        "self-suspend with no opponent Digimon installs no debuff prompt"
    );
}
