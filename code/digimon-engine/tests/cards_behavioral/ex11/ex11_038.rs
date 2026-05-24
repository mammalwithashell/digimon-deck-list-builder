//! EX11-038 Sunarizamon
//!
//! # Printed card text (authoritative — cards.json `effect_description_eng`)
//!
//! "[When Moving] [On Play] By trashing 1 [Mineral] or [Rock] trait card from
//!  your hand or your Digimon's digivolution cards, <Draw 1>."
//!
//! "[Inherited] When effects trash this card from a [Mineral] or [Rock] trait
//!  Digimon's digivolution cards, <Draw 1>."
//!
//! # Implementation status
//!
//! Clause 0 ([When Moving] / [On Play]):
//!   IMPLEMENTED — uses `select_union_zone` over hand ∪ own Digimon sources,
//!   then `trash_union_bound` to pay the selected origin-preserving cost.
//!   OnMove is split into its own clause with `event_permanent_is_source: true`
//!   so Sunarizamon only reacts to itself moving.
//!
//! Clause 1 ([Inherited] source-trash draw):
//!   IMPLEMENTED — `on_digivolution_card_trashed` with host-trait and
//!   trashed-source-id conditions, `draw: { of: you, count: 1 }` body.

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::action::space::{encode_source_select, PLAY_HAND_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX11-038")
        .expect("EX11-038 YAML parses and compiles")
        .build()
}

fn trait_card(card_id: &str, trait_name: &str) -> CardData {
    let mut card = make_test_card(card_id, card_id);
    card.traits.push(trait_name.to_string());
    card
}

#[test]
fn ex11_038_has_inherited_source_trash_draw_clause() {
    let runner = runner();
    let card = runner
        .compiled_card("EX11-038")
        .expect("EX11-038 compiled card present");

    assert!(card.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Triggered(triggered)
            if triggered.scope == CompiledScope::Inherited
                && triggered.when == vec![CompiledTiming::OnDigivolutionCardTrashed]
    )));
}

#[test]
fn ex11_038_source_trash_draws_one() {
    let mut host = make_test_card("ROCK-HOST", "Rock Host");
    host.traits.push("Rock".to_string());
    let filler = make_test_card("FILLER", "Filler");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-038")
        .expect("EX11-038 YAML parses and compiles")
        .add_card(host)
        .add_card(filler)
        .deck(0, &["FILLER", "FILLER"])
        .build();

    let host = runner.place_on_field(0, "ROCK-HOST", None);
    let source = runner.push_source(host, "EX11-038");
    let hand_before = runner.hand_size(0);

    let top = runner.top_card(host);
    {
        let mut ctx = EffectContext::new(&mut runner.game, top, Some(host), 0);
        ctx.trash_card_source(host, source);
    }

    assert_eq!(runner.hand_size(0), hand_before + 1);
}

// ─── Clause 0: [When Moving] / [On Play] ────────────────────────────────────

#[test]
fn ex11_038_has_on_play_when_moving_draw_clause() {
    let runner = runner();
    let card = runner
        .compiled_card("EX11-038")
        .expect("EX11-038 compiled card present");

    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    assert!(
        triggered
            .iter()
            .any(|t| t.when.contains(&CompiledTiming::OnPlay)),
        "EX11-038 must have an OnPlay clause once the gap closes; got: {triggered:?}"
    );
    assert!(
        triggered
            .iter()
            .any(|t| t.when.contains(&CompiledTiming::OnMove)),
        "EX11-038 must have an OnMove clause once the gap closes; got: {triggered:?}"
    );
}

#[test]
fn ex11_038_on_play_trash_hand_mineral_draws_one() {
    let filler = make_test_card("FILLER", "Filler");
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-038")
        .expect("EX11-038 YAML parses and compiles")
        .add_card(trait_card("MIN-HAND", "Mineral"))
        .add_card(filler)
        .hand(0, &["EX11-038", "MIN-HAND"])
        .deck(0, &["FILLER"])
        .memory(10)
        .build();

    runner.play(0, 0).expect("play EX11-038");
    let prompt = runner
        .pending_selection_view()
        .expect("EX11-038 hand/source cost prompt opens");
    assert!(
        prompt.valid_action_ids.contains(&PLAY_HAND_START),
        "Mineral card in hand must be a payable cost candidate"
    );

    runner
        .execute_action(0, PLAY_HAND_START)
        .expect("trash Mineral card from hand");
    runner.auto_resolve().expect("resolve draw");

    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "MIN-HAND"),
        "selected hand card should be trashed as the cost"
    );
    assert!(
        runner.game.players[0]
            .hand
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "FILLER"),
        "successful cost should draw 1"
    );
}

#[test]
fn ex11_038_on_play_trash_own_source_rock_draws_one() {
    let filler = make_test_card("FILLER", "Filler");
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-038")
        .expect("EX11-038 YAML parses and compiles")
        .add_card(trait_card("ROCK-SRC", "Rock"))
        .add_card(trait_card("HOST", "Mineral"))
        .add_card(filler)
        .hand(0, &["EX11-038"])
        .deck(0, &["FILLER"])
        .memory(10)
        .build();
    let host = runner.place_stack(0, &["ROCK-SRC", "HOST"]);

    runner.play(0, 0).expect("play EX11-038");
    let source_action = encode_source_select(host.index as u16, 0).expect("encode source action");
    let prompt = runner
        .pending_selection_view()
        .expect("EX11-038 hand/source cost prompt opens");
    assert!(
        prompt.valid_action_ids.contains(&source_action),
        "Rock source under an own Digimon must be a payable cost candidate"
    );

    runner
        .execute_action(0, source_action)
        .expect("trash Rock source");
    runner.auto_resolve().expect("resolve draw");

    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "ROCK-SRC"),
        "selected source card should be trashed as the cost"
    );
    assert!(
        runner.game.players[0]
            .hand
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "FILLER"),
        "successful cost should draw 1"
    );
}

#[test]
fn ex11_038_on_play_no_eligible_card_no_draw() {
    let filler = make_test_card("FILLER", "Filler");
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-038")
        .expect("EX11-038 YAML parses and compiles")
        .add_card(trait_card("HOST", "Reptile"))
        .add_card(filler)
        .hand(0, &["EX11-038"])
        .deck(0, &["FILLER"])
        .memory(10)
        .build();
    runner.place_stack(0, &["HOST"]);

    runner.play(0, 0).expect("play EX11-038");
    runner.auto_resolve().expect("unpayable cost no-ops");

    assert!(
        runner.pending_selection_view().is_none(),
        "no eligible Mineral/Rock card must not open a cost prompt"
    );
    assert_eq!(
        runner.hand_size(0),
        0,
        "unpayable cost must not draw from deck"
    );
}
