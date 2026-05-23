//! EX8-047 Sunarizamon
//!
//! Phase 2 Track E (2026-05-17): full clause coverage.
//! - [On Play] reveal-top-3 two-pick (Mineral/Rock + LIBERATOR) authored
//!   using the new `choose_from_reveal` + `order_remainder` verbs.
//! - Inherited source-trash delete clause unchanged from the prior pass.

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledStep, CompiledTiming};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::selection::SelectionKind;

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX8-047")
        .expect("EX8-047 YAML parses and compiles")
        .build()
}

#[test]
fn ex8_047_has_inherited_source_trash_delete_clause() {
    let runner = runner();
    let card = runner
        .compiled_card("EX8-047")
        .expect("EX8-047 compiled card present");

    let inherited = card.effects.iter().find_map(|clause| match clause {
        CompiledClause::Triggered(triggered)
            if triggered.scope == CompiledScope::Inherited
                && triggered.when == vec![CompiledTiming::OnDigivolutionCardTrashed] =>
        {
            Some(triggered)
        }
        _ => None,
    });

    assert!(
        inherited.is_some(),
        "EX8-047 must compile inherited source-trash timing"
    );
}

#[test]
fn ex8_047_source_trash_deletes_only_opponent_digimon_cost_four_or_less() {
    let mut host = make_test_card("ROCK-HOST", "Rock Host");
    host.traits.push("Rock".to_string());
    let mut cheap = make_test_card("CHEAP4", "Cheap Cost 4");
    cheap.play_cost = 4;
    let mut expensive = make_test_card("EXPENSIVE5", "Expensive Cost 5");
    expensive.play_cost = 5;

    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-047")
        .expect("EX8-047 YAML parses and compiles")
        .add_card(host)
        .add_card(cheap)
        .add_card(expensive)
        .build();

    let host = runner.place_on_field(0, "ROCK-HOST", None);
    let source = runner.push_source(host, "EX8-047");
    runner.place_on_field(1, "CHEAP4", None);
    runner.place_on_field(1, "EXPENSIVE5", None);

    let top = runner.top_card(host);
    {
        let mut ctx = EffectContext::new(&mut runner.game, top, Some(host), 0);
        ctx.trash_card_source(host, source);
    }

    assert_eq!(runner.pending_kind(), Some(SelectionKind::OppField));
    assert_eq!(
        runner.pending_action_count(),
        1,
        "only play-cost-4-or-less opponent Digimon should be selectable"
    );
    runner
        .auto_resolve()
        .expect("EX8-047 delete target resolves");

    let remaining: Vec<&str> = runner
        .game
        .player(1)
        .battle_area
        .iter()
        .map(|perm| perm.top_card().card_id(&runner.game.card_data))
        .collect();
    assert_eq!(remaining, vec!["EXPENSIVE5"]);
}

// ─────────────────────────────────────────────────────────────────────────────
// Phase 2 Track E (2026-05-17): [On Play] reveal/search two-pick clause.
// ─────────────────────────────────────────────────────────────────────────────

/// Structural: EX8-047 must have an [On Play] face-up triggered clause
/// whose process contains a `reveal_top_deck`, two `choose_from_reveal`
/// picks (Mineral/Rock + LIBERATOR), and an `order_remainder` placement.
#[test]
fn ex8_047_has_on_play_reveal_two_pick_clause() {
    let runner = runner();
    let card = runner
        .compiled_card("EX8-047")
        .expect("EX8-047 compiled card present");

    let on_play = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnPlay) => Some(t),
            _ => None,
        })
        .expect("EX8-047 must have an [On Play] clause");

    let has_reveal = on_play
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::RevealTopDeck { .. }));
    let choose_count = on_play
        .process
        .iter()
        .filter(|s| matches!(s, CompiledStep::ChooseFromReveal { .. }))
        .count();
    let has_order = on_play
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::OrderRemainder { .. }));

    assert!(has_reveal, "must reveal top 3");
    assert_eq!(
        choose_count, 2,
        "must have 2 choose_from_reveal picks (Mineral/Rock, LIBERATOR); got {choose_count}"
    );
    assert!(has_order, "must order_remainder for bottom placement");
}

/// Behavioral: with a Mineral card + a LIBERATOR card + a filler in the top
/// 3 of deck, both picks land their card in hand and the filler returns to
/// the deck bottom.
#[test]
fn ex8_047_on_play_picks_mineral_and_liberator_to_hand() {
    let mut mineral = make_test_card("MIN-1", "MIN-1");
    mineral.traits.push("Mineral".to_string());
    let mut liberator = make_test_card("LIB-1", "LIB-1");
    liberator.traits.push("LIBERATOR".to_string());
    let filler = make_test_card("FILL-1", "FILL-1");
    let holder = make_test_card("EX8_047_HOLDER", "EX8_047 Holder");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-047")
        .expect("EX8-047 YAML parses and compiles")
        .add_card(mineral)
        .add_card(liberator)
        .add_card(filler)
        .add_card(holder)
        .build();

    // Place a stand-in source card so we have a CardHandle to anchor the
    // EffectContext. Deck top: MIN-1, LIB-1, FILL-1.
    let holder_perm = runner.place_on_field(0, "EX8_047_HOLDER", None);
    let src = runner.top_card(holder_perm);
    {
        use digimon_engine::card_source::CardSource;
        for id in ["FILL-1", "LIB-1", "MIN-1"] {
            let data_idx = runner
                .game
                .card_data
                .iter()
                .position(|c| c.card_id == id)
                .unwrap();
            let card_index = runner.game.next_card_index();
            runner.game.players[0]
                .deck
                .push(CardSource::new(data_idx, 0, card_index));
        }
    }

    let deck_before = runner.game.players[0].deck.len();
    let compiled = runner.compiled_card("EX8-047").expect("compiled").clone();
    let on_play_process = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnPlay) => {
                Some(t.process.clone())
            }
            _ => None,
        })
        .expect("on_play process present");

    {
        let mut ctx = EffectContext::new(&mut runner.game, src, None, 0);
        run_steps(&on_play_process, &mut ctx, &mut Bindings::new());
    }

    // First pick: Mineral/Rock filter. MIN-1 is the only match — auto-resolve
    // picks it.
    runner
        .auto_resolve()
        .expect("EX8-047 on-play reveal flow resolves");

    let hand_ids: Vec<&str> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data))
        .collect();
    assert!(
        hand_ids.iter().any(|id| *id == "MIN-1"),
        "MIN-1 must land in hand after Mineral/Rock pick"
    );
    assert!(
        hand_ids.iter().any(|id| *id == "LIB-1"),
        "LIB-1 must land in hand after LIBERATOR pick"
    );

    // Deck net change: -2 (2 to hand, 1 returned to bottom).
    let deck_after = runner.game.players[0].deck.len();
    assert_eq!(
        deck_before - deck_after,
        2,
        "deck must shrink by 2 (2 added to hand, 1 returned to bottom)"
    );
    assert_eq!(
        runner.game.revealed_cards.len(),
        0,
        "reveal pool must be empty after order_remainder"
    );
}
