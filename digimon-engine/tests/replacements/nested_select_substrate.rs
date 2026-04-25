//! Phase C — substrate-level tests for the parked-replacement slot and the
//! `EffectContext::cancel_leave` / `handle_replacement` / `redirect_replacement`
//! / `substitute_replacement` outcome-setters.
//!
//! These tests do NOT exercise end-to-end replacement flows — they manually
//! install `Game.parked_replacement` and verify that the outcome-setters
//! mutate the slot correctly. End-to-end coverage lives in the per-keyword
//! test files (`nested_select_save`, `nested_select_fragment`, etc.).

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind, Zone};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::{ParkedReplacement, ReplacementCause, ReplacementOutcome, ReplacementSubject};

fn fighter(id: &str) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(4),
        dp: Some(4000),
        play_cost: 4,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
    }
}

fn install_parked(game: &mut digimon_engine::game::Game, target: PermanentHandle) {
    game.install_parked_replacement_for_test(ParkedReplacement {
        subject: ReplacementSubject::Permanent(target),
        cause: ReplacementCause::OpponentEffect,
        original_destination: Some(Zone::Trash),
        source_card: CardHandle(0),
        source_permanent: None,
        controller: 0,
        outcome: ReplacementOutcome::None,
    });
}

#[test]
fn cancel_leave_writes_cancelled_outcome_to_parked_slot() {
    let mut r = DebugRunner::builder().add_card(fighter("X")).start();
    let target = r.place_on_field(0, "X", None);

    install_parked(&mut r.game, target);

    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, 0);
        ctx.cancel_leave();
    }

    let outcome = r
        .game
        .parked_replacement_outcome_for_test()
        .expect("slot still set");
    assert_eq!(
        outcome,
        ReplacementOutcome::Cancelled,
        "cancel_leave should write Cancelled outcome to parked slot"
    );
}

#[test]
fn handle_replacement_writes_custom_handled_to_parked_slot() {
    let mut r = DebugRunner::builder().add_card(fighter("X")).start();
    let target = r.place_on_field(0, "X", None);
    install_parked(&mut r.game, target);
    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, 0);
        ctx.handle_replacement();
    }
    let outcome = r.game.parked_replacement_outcome_for_test().expect("slot still set");
    assert_eq!(
        outcome,
        ReplacementOutcome::CustomHandled,
        "handle_replacement should write CustomHandled to parked slot"
    );
}

#[test]
fn redirect_replacement_writes_redirected_outcome_to_parked_slot() {
    let mut r = DebugRunner::builder().add_card(fighter("X")).start();
    let target = r.place_on_field(0, "X", None);
    install_parked(&mut r.game, target);
    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, 0);
        ctx.redirect_replacement(Zone::Hand);
    }
    let outcome = r.game.parked_replacement_outcome_for_test().expect("slot still set");
    assert_eq!(
        outcome,
        ReplacementOutcome::Redirected(Zone::Hand),
        "redirect_replacement(Hand) should write Redirected(Hand) to parked slot"
    );
}

#[test]
fn substitute_replacement_writes_substituted_outcome_to_parked_slot() {
    let mut r = DebugRunner::builder()
        .add_card(fighter("X"))
        .add_card(fighter("Y"))
        .start();
    let target = r.place_on_field(0, "X", None);
    let other = r.place_on_field(0, "Y", None);
    install_parked(&mut r.game, target);
    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, 0);
        ctx.substitute_replacement(ReplacementSubject::Permanent(other));
    }
    let outcome = r.game.parked_replacement_outcome_for_test().expect("slot still set");
    assert_eq!(
        outcome,
        ReplacementOutcome::Substituted(ReplacementSubject::Permanent(other)),
        "substitute_replacement should write Substituted(other) to parked slot"
    );
}
