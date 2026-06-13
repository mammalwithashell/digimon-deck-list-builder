//! EX10-039 ChuuChuumon — Digimon, Lv.3, Purple, DP 1000, Cost 4.
//! Traits: [Beast, Bagra Army].
//!
//! # Card text (card image — verified)
//!
//! [Start of Your Main Phase] You may place 1 Digimon card with the
//! [Bagra Army] trait from your hand or trash as any of your [Bagra Army]
//! trait Digimon's bottom digivolution cards or under any of your
//! [Bagra Army] trait Tamers.
//!
//! [On Deletion] <Save> (You may place this card under one of your Tamers.)
//!
//! Inherited: When effects trash this card from a [Bagra Army] trait
//! Digimon's digivolution cards, <Draw 1>.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX10/Purple/EX10_039.cs

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming, Keyword};
use digimon_engine::selection::TriggerSource;

fn make_bagra_carrier(id: &str) -> CardData {
    let mut c = make_test_card(id, &format!("{id} Bagra Carrier"));
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.traits = vec!["Bagra Army".to_string()];
    c
}

#[test]
fn ex10_039_compiles_and_has_save() {
    let mut r = DebugRunner::builder()
        .dsl_card("EX10-039")
        .expect("EX10-039 YAML parses and compiles")
        .build();
    assert!(r.compiled_card("EX10-039").is_some());
    let h = r.place_on_field(0, "EX10-039", None);
    r.game.tick_declarative_effects();
    assert!(
        r.game.has_keyword(h, Keyword::Save),
        "[On Deletion] <Save> must be granted on field"
    );
}

/// [Start of Your Main Phase] (hand branch): place a [Bagra Army] Digimon
/// card from hand under a [Bagra Army] Tamer (Yuu Amano).
#[test]
fn ex10_039_start_of_main_places_bagra_card_from_hand_under_bagra_tamer() {
    let mut r = DebugRunner::builder()
        .dsl_card("EX10-039")
        .expect("EX10-039 loads")
        .dsl_card("BT10-093")
        .expect("BT10-093 Yuu Amano ([Bagra Army] Tamer) loads")
        .dsl_card("EX10-044")
        .expect("EX10-044 Damemon ([Bagra Army] Digimon) loads")
        .memory(10)
        .hand(0, &["EX10-044"])
        .start();
    r.skip_mulligan();
    r.set_first_player(0);

    let chuu = r.place_on_field(0, "EX10-039", Some(0));
    let yuu = r.place_on_field(0, "BT10-093", Some(0));
    let hand_before = r.hand_size(0);

    r.game.enqueue_triggered(
        EffectTiming::StartOfYourMainPhase,
        TriggerSource::Permanent(chuu),
    );
    r.game.drain_effect_queue();

    // Optional clause: accept gate, then zone choice → "From hand".
    let view = r
        .pending_selection_view()
        .expect("the optional clause's accept prompt must surface");
    assert!(view.is_optional, "the printed 'you may' clause is optional");
    if view.valid_action_ids.len() == 1 {
        let accept = view.valid_action_ids[0];
        r.execute_action(0, accept).expect("accept the clause");
    }
    let view = r
        .pending_selection_view()
        .expect("the zone-choice prompt must surface");
    let from_hand = view.valid_action_ids[0];
    r.execute_action(0, from_hand).expect("choose 'From hand'");

    // Pick Damemon from hand…
    let view = r
        .pending_selection_view()
        .expect("the hand pick must surface");
    let pick = view.valid_action_ids[0];
    r.execute_action(0, pick).expect("pick Damemon from hand");
    // …then Yuu Amano (a [Bagra Army] Tamer) as the destination — the
    // on-field ChuuChuumon ([Bagra Army] Digimon) is also a legal target,
    // so the destination must be steered explicitly.
    let view = r
        .pending_selection_view()
        .expect("the destination pick must surface");
    let want = digimon_engine::action::space::encode_attack(yuu.player as u16, yuu.index as u16);
    assert!(
        view.valid_action_ids.contains(&want),
        "the [Bagra Army] Tamer must be a legal destination"
    );
    r.execute_action(0, want).expect("place under Yuu Amano");
    let _ = r.auto_resolve();

    assert_eq!(r.hand_size(0), hand_before - 1, "Damemon must leave the hand");
    let yuu = r
        .game
        .players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&r.game.card_data) == "BT10-093")
        .expect("Yuu Amano on field");
    assert!(
        yuu.card_sources
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "EX10-044"),
        "Damemon must be placed under the [Bagra Army] Tamer"
    );
}

/// Inherited: trashed by an effect from a [Bagra Army] Digimon's digivolution
/// cards → Draw 1.
#[test]
fn ex10_039_inherited_draws_1_when_effect_trashes_it_from_bagra_stack() {
    let mut r = DebugRunner::builder()
        .dsl_card("EX10-039")
        .expect("EX10-039 loads")
        .add_card(make_bagra_carrier("CHU-CARRIER"))
        .add_card({
            let mut c = make_test_card("CHU-FILL", "Filler");
            c.card_kind = CardKind::Digimon;
            c
        })
        .deck(0, &["CHU-FILL"; 5])
        .memory(10)
        .start();
    r.skip_mulligan();
    r.set_first_player(0);

    // ChuuChuumon at the BOTTOM of a [Bagra Army] carrier's stack.
    let carrier = r.place_on_field(0, "CHU-CARRIER", Some(0));
    r.push_source(carrier, "EX10-039");
    // push_source appends under the top card; trash_one_source removes the
    // bottom source — which is EX10-039 here.
    let hand_before = r.hand_size(0);

    r.trash_one_source(carrier); // fires fire_digivolution_card_trashed

    assert_eq!(
        r.hand_size(0),
        hand_before + 1,
        "the inherited on-trash effect must Draw 1"
    );
}

/// Negative: the inherited draw is gated on the carrier having the
/// [Bagra Army] trait.
#[test]
fn ex10_039_inherited_does_not_draw_from_non_bagra_stack() {
    let mut r = DebugRunner::builder()
        .dsl_card("EX10-039")
        .expect("EX10-039 loads")
        .add_card({
            let mut c = make_test_card("CHU-PLAIN", "Plain Carrier");
            c.card_kind = CardKind::Digimon;
            c.level = Some(4);
            c
        })
        .add_card({
            let mut c = make_test_card("CHU-FILL2", "Filler");
            c.card_kind = CardKind::Digimon;
            c
        })
        .deck(0, &["CHU-FILL2"; 5])
        .memory(10)
        .start();
    r.skip_mulligan();
    r.set_first_player(0);

    let carrier = r.place_on_field(0, "CHU-PLAIN", Some(0));
    r.push_source(carrier, "EX10-039");
    let hand_before = r.hand_size(0);

    r.trash_one_source(carrier);

    assert_eq!(
        r.hand_size(0),
        hand_before,
        "no draw when the carrier lacks the [Bagra Army] trait"
    );
}
