//! BT13-030 UlforceVeedramon — Digimon, Lv.6, Blue, DP 11000, Cost 11.
//! Traits: Holy Warrior / Royal Knight. Form: Mega. Attribute: Vaccine.
//!
//! # Card text (cards.json / card image)
//!
//! ```text
//! [On Play] [When Digivolving] For each of your Digimon with the [Royal
//!   Knight] trait and each of your blue Tamers in play, trash the top 2
//!   digivolution cards of 1 of your opponent's Digimon.
//! [Your Turn] [Once Per Turn] When you play a Digimon with the [Royal Knight]
//!   trait or a blue Tamer, return 1 of your opponent's Digimon with no
//!   digivolution cards to its owner's hand.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT13/Blue/BT13_030.cs
//!
//! # Implemented slice
//! - [Your Turn][OPT] ally-played observer: when you play a [Royal Knight] or
//!   a blue Tamer, return 1 opponent Digimon with no digivolution cards to its
//!   owner's hand. Fires on `on_enter_field_anyone` (covers Digimon + Tamer
//!   plays); predicate (Royal Knight trait) OR (blue Tamer);
//!   `materials_count_lte: 0` target + `return_to_hand`. PRECEDENT: BT20-091.
//!
//! # Gap-routed (NOT implemented — see YAML stub)
//! - [On Play][When Digivolving] for-each-counted clause: trash top 2 sources
//!   of 1 opponent Digimon, repeated per own Royal Knight + per own blue Tamer
//!   — G-FOR-EACH-COUNTED-FIELD-OBJECTS (counted foreach where the count is
//!   derived from multiple object groups).

use digimon_dsl::compiled::{CompiledClause, CompiledTiming};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

const CARD_ID: &str = "BT13-030";

fn royal_knight(id: &str) -> CardData {
    let mut c = make_test_card(id, "RoyalKnightAlly");
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.colors = vec![CardColor::Blue];
    c.traits = vec!["Royal Knight".into()];
    c
}

fn blue_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, "BlueTamerAlly");
    c.card_kind = CardKind::Tamer;
    c.colors = vec![CardColor::Blue];
    c
}

fn opp_digimon_no_sources(id: &str) -> CardData {
    let mut c = make_test_card(id, "OppNoSources");
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(9000);
    c.colors = vec![CardColor::Purple];
    c
}

fn plain_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, "PlainAlly");
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.colors = vec![CardColor::Red];
    c
}

// ════════════════════════════════════════════════════════════════════════════
// Structural
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn bt13_030_loads_with_gap_stub() {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("load")
        .start();
}

#[test]
fn bt13_030_has_ally_played_bounce_observer() {
    let runner = DebugRunner::builder().dsl_card(CARD_ID).expect("load").start();
    let card = runner.compiled_card(CARD_ID).expect("compiled card");

    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnEnterFieldAnyone) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("ally-played observer exists");
    assert!(clause.once_per_turn, "[Once Per Turn]");
    assert!(
        !clause.optional,
        "the bounce is mandatory ('return 1 ...', not 'you may')"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// [Your Turn][OPT] ally-played bounce
// ════════════════════════════════════════════════════════════════════════════

/// POSITIVE: playing a [Royal Knight] Digimon bounces 1 opponent Digimon that
/// has no digivolution cards back to its owner's hand.
#[test]
fn bt13_030_royal_knight_play_bounces_sourceless_opponent() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("load")
        .add_card(royal_knight("RK-ALLY"))
        .add_card(opp_digimon_no_sources("OPP-NS"))
        .add_card(make_test_card("FILL", "Fill"))
        .hand(0, &["RK-ALLY"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .memory(10)
        .start();

    r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(1, "OPP-NS", Some(0));
    let opp_hand_before = r.game.players[1].hand.len();

    r.play(0, 0); // play the Royal Knight ally

    // The optional bounce prompt should install for the single sourceless
    // opponent Digimon.
    let view = r
        .pending_selection_view()
        .expect("bounce prompt installs on Royal Knight play");
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only the sourceless opponent Digimon is a legal bounce target"
    );
    r.execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("bounce the sourceless opponent Digimon");
    let _ = r.auto_resolve();

    assert!(
        r.game.players[1]
            .battle_area
            .iter()
            .all(|p| p.top_card().card_id(&r.game.card_data) != "OPP-NS"),
        "the sourceless opponent Digimon left the battle area"
    );
    assert_eq!(
        r.game.players[1].hand.len(),
        opp_hand_before + 1,
        "it returns to its owner's hand"
    );
}

/// POSITIVE: playing a blue Tamer also triggers the bounce.
#[test]
fn bt13_030_blue_tamer_play_triggers_bounce() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("load")
        .add_card(blue_tamer("BT-ALLY"))
        .add_card(opp_digimon_no_sources("OPP-NS"))
        .add_card(make_test_card("FILL", "Fill"))
        .hand(0, &["BT-ALLY"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .memory(10)
        .start();

    r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(1, "OPP-NS", Some(0));

    r.play(0, 0); // play the blue Tamer

    assert!(
        r.pending_selection().is_some(),
        "playing a blue Tamer triggers the bounce observer"
    );
}

/// NEGATIVE (event filter): playing a non-Royal-Knight, non-blue-Tamer Digimon
/// does NOT trigger the bounce.
#[test]
fn bt13_030_plain_play_does_not_trigger_bounce() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("load")
        .add_card(plain_digimon("PLAIN-ALLY"))
        .add_card(opp_digimon_no_sources("OPP-NS"))
        .add_card(make_test_card("FILL", "Fill"))
        .hand(0, &["PLAIN-ALLY"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .memory(10)
        .start();

    r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(1, "OPP-NS", Some(0));

    r.play(0, 0); // play a plain red Digimon

    assert!(
        r.pending_selection().is_none(),
        "a non-Royal-Knight / non-blue-Tamer play does not trigger the bounce"
    );
}

/// NEGATIVE (target filter): an opponent Digimon WITH digivolution cards is not
/// a legal bounce target, so no prompt installs.
#[test]
fn bt13_030_opponent_with_sources_is_not_a_legal_target() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("load")
        .add_card(royal_knight("RK-ALLY"))
        .add_card(opp_digimon_no_sources("OPP-TOP"))
        .add_card(plain_digimon("OPP-SRC"))
        .add_card(make_test_card("FILL", "Fill"))
        .hand(0, &["RK-ALLY"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .memory(10)
        .start();

    r.place_on_field(0, CARD_ID, Some(0));
    // Opponent Digimon with one digivolution card under it ⇒ has sources.
    r.place_stack(1, &["OPP-SRC", "OPP-TOP"]);

    r.play(0, 0); // play the Royal Knight ally

    assert!(
        r.pending_selection().is_none(),
        "an opponent Digimon WITH digivolution cards is not a legal target ⇒ no bounce prompt"
    );
    assert_eq!(
        r.battle_area_size(1),
        1,
        "the stacked opponent Digimon stays on the field"
    );
}

#[ignore = "pending: G-FOR-EACH-COUNTED-FIELD-OBJECTS — for each Royal Knight/blue Tamer, trash 2 sources from one opponent Digimon"]
#[test]
fn bt13_030_trashes_sources_for_each_royal_knight_and_blue_tamer() {}
