//! BT10-093 Yuu Amano — Tamer, Purple, Cost 3. Traits: [General, Bagra Army].
//!
//! # Card text (card image — verified)
//!
//! [All Turns][Once Per Turn] When a purple card is placed under this Tamer,
//! <Draw 1> and gain 1 memory. — ***OMITTED (PARTIAL)***: no
//! "card placed under this permanent" trigger timing in the DSL
//! (G-DSL-ON-CARD-PLACED-UNDER-TRIGGER, qa/dsl-vocab-gaps.md).
//!
//! [Your Turn][Once Per Turn] When you would play 1 level 4 or higher Digimon
//! card with [Bagra Army] in its traits, by placing up to 3 purple Digimon
//! cards from under your Tamers in the played Digimon card's digivolution
//! cards, reduce the play cost of that Digimon by 2 for each card placed.
//!
//! [Security] Play this card without paying its memory cost.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT10/Purple/BT10_093.cs
//!
//! The full DigiXros interaction (pre-attach ordering + cost stacking with
//! DarknessBagramon's [DigiXros -3]) is pinned by judge-quiz Q29
//! (`tests/judge_quiz/e_partition_digixros.rs`). This file covers the hook on
//! a plain (non-DigiXros) ≥Lv4 [Bagra Army] play.

use digimon_engine::action::space::{PASS, SOURCE_SELECT_START};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::selection::SelectionKind;

fn yuu_runner() -> DebugRunner {
    // EX10-031 DarkKnightmon (Lv5, [Bagra Army], cost 7) is the played card;
    // EX10-039 ChuuChuumon (purple Digimon) sits under Yuu Amano.
    let mut r = DebugRunner::builder()
        .dsl_card("BT10-093")
        .expect("BT10-093 Yuu Amano loads")
        .dsl_card("EX10-031")
        .expect("EX10-031 DarkKnightmon loads")
        .dsl_card("EX10-039")
        .expect("EX10-039 ChuuChuumon loads")
        .memory(10)
        .hand(0, &["EX10-031"])
        .start();
    r.skip_mulligan();
    r.set_first_player(0);
    let yuu = r.place_on_field(0, "BT10-093", Some(0));
    r.push_source(yuu, "EX10-039");
    r
}

#[test]
fn bt10_093_compiles_and_is_in_pack() {
    let r = DebugRunner::builder()
        .dsl_card("BT10-093")
        .expect("BT10-093 YAML parses and compiles")
        .build();
    assert!(r.compiled_card("BT10-093").is_some());
}

/// The would-play hook fires on a plain ≥Lv4 [Bagra Army] Digimon play:
/// accepting it and placing ChuuChuumon reduces the cost by 2 and puts the
/// card in the played Digimon's digivolution cards.
#[test]
fn bt10_093_hook_places_under_tamer_card_and_reduces_cost_by_2() {
    let mut r = yuu_runner();
    let memory_before = r.memory();

    let _ = r.play(0, 0);

    // Optional accept prompt for the [Your Turn][Once Per Turn] hook.
    let view = r
        .pending_selection_view()
        .expect("Yuu Amano's would-play hook must surface");
    assert!(view.is_optional, "the printed 'by placing...' hook is optional");
    if !view.valid_action_ids.iter().any(|&a| a >= SOURCE_SELECT_START) {
        let accept = view.valid_action_ids[0];
        r.execute_action(0, accept).expect("accept the hook");
    }

    // Pick ChuuChuumon from under Yuu Amano, then close the up-to-3 select.
    let view = r
        .pending_selection_view()
        .expect("the under-Tamer source pick must surface");
    let pick = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a >= SOURCE_SELECT_START)
        .expect("ChuuChuumon must be selectable from under Yuu Amano");
    r.execute_action(0, pick).expect("pick ChuuChuumon");
    if r
        .pending_selection_view()
        .is_some_and(|v| matches!(v.kind, SelectionKind::SourceMulti { .. }))
    {
        r.execute_action(0, PASS).expect("close the multi-select");
    }
    // DarkKnightmon's own [On Play] (select a Digimon to shield) resolves after.
    let _ = r.auto_resolve();

    // Cost 7 − 2 = 5 paid.
    assert_eq!(
        r.memory(),
        memory_before - 5,
        "DarkKnightmon must cost 7 − 2 = 5 after one placed card"
    );

    // ChuuChuumon is now a digivolution card of the played DarkKnightmon...
    let dk = r
        .game
        .players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&r.game.card_data) == "EX10-031")
        .expect("DarkKnightmon must be on the field");
    assert!(
        dk.card_sources
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "EX10-039"),
        "ChuuChuumon must be placed in DarkKnightmon's digivolution cards"
    );
    // ...and no longer under Yuu Amano.
    let yuu = r
        .game
        .players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&r.game.card_data) == "BT10-093")
        .expect("Yuu Amano remains on field");
    assert!(
        !yuu.card_sources
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "EX10-039"),
        "ChuuChuumon must leave Yuu Amano's source stack"
    );
}

/// Declining the optional hook plays the Digimon at full printed cost and
/// leaves the under-Tamer card in place.
#[test]
fn bt10_093_declining_hook_plays_at_full_cost() {
    let mut r = yuu_runner();
    let memory_before = r.memory();

    let _ = r.play(0, 0);

    // Decline at the accept prompt (or PASS the source select if it surfaced
    // directly).
    if let Some(view) = r.pending_selection_view() {
        assert!(view.is_optional);
        r.decline_optional_trigger()
            .or_else(|_| r.execute_action(0, PASS))
            .expect("decline Yuu Amano's hook");
    }
    let _ = r.auto_resolve();

    assert_eq!(
        r.memory(),
        memory_before - 7,
        "declining the hook must pay DarkKnightmon's full printed cost 7"
    );
    let yuu = r
        .game
        .players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&r.game.card_data) == "BT10-093")
        .expect("Yuu Amano remains on field");
    assert!(
        yuu.card_sources
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "EX10-039"),
        "ChuuChuumon must stay under Yuu Amano when the hook is declined"
    );
}
