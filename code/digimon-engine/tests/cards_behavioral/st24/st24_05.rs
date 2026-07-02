//! ST24-05 GeoGreymon — Digimon, Lv.4, Yellow/Red, DP 6000, Cost 5.
//! Traits: Dinosaur, DATA SQUAD. Attribute: Vaccine.
//!
//! # Card text (data/cards.json — verbatim, matches DCGO)
//! [On Play] [When Digivolving] If you have 1 or fewer Tamers, you may play 1
//!   Tamer card with the [DATA SQUAD] trait from your hand without paying the
//!   cost.
//! Inherited [Your Turn]: This Digimon gets +2000 DP.
//!
//! Alt-digivolve (image/DCGO — cards.json digivolve_cost null):
//!   Lv.3 from (Agumon-in-name AND Dinosaur trait) OR [DATA SQUAD] trait: Cost 2.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/ST24/Yellow/ST24_05.cs
//!
//! # Patterns — sister to ST23-07 (conditional free-Tamer-play, trait-swapped)

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

const CARD_ID: &str = "ST24-05";

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn ds_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![CardColor::Yellow];
    c.traits = vec!["DATA SQUAD".to_string()];
    c
}

fn plain_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![CardColor::Yellow];
    c
}

fn filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c.level = Some(3);
    c.dp = Some(2000);
    c.play_cost = 3;
    c
}

fn push_to_hand(runner: &mut DebugRunner, p: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card_id {card_id}"));
    let next_idx = runner.game.next_card_index();
    runner.game.players[p as usize]
        .hand
        .push(CardSource::new(data_idx, p, next_idx));
}

fn base() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST24-05 in embedded DSL pack")
        .add_card(ds_tamer("DS-TAMER"))
        .add_card(plain_tamer("PLAIN-TAMER"))
        .add_card(filler("FILLER"))
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 5])
        .memory(8)
        .start()
}

fn tamer_count(runner: &DebugRunner, p: usize) -> usize {
    runner.game.players[p]
        .battle_area
        .iter()
        .filter(|perm| perm.top_card().card_kind(&runner.game.card_data) == CardKind::Tamer)
        .count()
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn st24_05_metadata_and_composite_alt_path() {
    let runner = base();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.name, "GeoGreymon");
    assert_eq!(card.level, Some(4));
    assert_eq!(card.cost, Some(5));
    assert_eq!(card.dp, Some(6000));
    // The composite alt-path (Agumon+Dinosaur OR DATA SQUAD) must compile;
    // assert at least one alt-path exists with literal cost 2.
    assert!(
        card.alt_paths.iter().any(|p| matches!(
            p.cost,
            Some(digimon_dsl::compiled::CompiledCost::Literal(2))
        )),
        "Lv.3 cost-2 alt-path present"
    );
}

#[test]
fn st24_05_has_op_wd_clause_and_inherited_dp() {
    let runner = base();
    let card = runner.compiled_card(CARD_ID).expect("compiled");

    let has_op_wd = card.effects.iter().any(|c| {
        matches!(c, CompiledClause::Triggered(t)
            if t.when.contains(&CompiledTiming::OnPlay)
            && t.when.contains(&CompiledTiming::WhenDigivolving))
    });
    assert!(has_op_wd, "[On Play][When Digivolving] clause present");

    let has_inherited_dp = card.effects.iter().any(|c| {
        matches!(c, CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
            scope, dp_modifier: Some(2000), ..
        }) if *scope == CompiledScope::Inherited)
    });
    assert!(
        has_inherited_dp,
        "inherited [Your Turn] +2000 DP aura present"
    );
}

// ─── Section 2 — Behavioral: conditional free-Tamer-play ─────────────────────

/// POSITIVE: with 0 Tamers and a [DATA SQUAD] Tamer in hand, the play offers a
/// free Tamer play; taking it puts the Tamer on the field at no memory cost.
#[test]
fn st24_05_plays_ds_tamer_free_when_zero_tamers() {
    let mut runner = base();
    runner.set_first_player(0);
    push_to_hand(&mut runner, 0, "DS-TAMER");

    let geo = runner.place_on_field(0, CARD_ID, Some(0));
    let mem_before = runner.memory();
    let tamers_before = tamer_count(&runner, 0);
    runner.fire_on_play(0, geo.index as usize);

    let view = runner
        .pending_selection_view()
        .expect("the optional free-Tamer play must surface");
    assert!(view.is_optional, "'you may play' ⇒ declinable");
    runner.execute_action(0, view.valid_action_ids[0]).unwrap();
    let _ = runner.auto_resolve();

    assert_eq!(
        tamer_count(&runner, 0),
        tamers_before + 1,
        "the [DATA SQUAD] Tamer was played"
    );
    assert_eq!(
        runner.memory(),
        mem_before,
        "played without paying the cost"
    );
}

#[test]
fn st24_05_free_tamer_play_is_declinable() {
    let mut runner = base();
    runner.set_first_player(0);
    push_to_hand(&mut runner, 0, "DS-TAMER");
    let geo = runner.place_on_field(0, CARD_ID, Some(0));
    let mem_before = runner.memory();
    let hand_before = runner.hand_size(0);
    runner.fire_on_play(0, geo.index as usize);

    let view = runner
        .pending_selection_view()
        .expect("optional pick surfaces");
    assert!(view.is_optional);
    runner
        .execute_action(0, digimon_engine::action::space::PASS)
        .expect("decline");
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "declined ⇒ Tamer stays in hand"
    );
    assert_eq!(runner.memory(), mem_before, "declined ⇒ no change");
}

/// NEGATIVE (Tamer-count gate): with 2 Tamers already on field, the clause is
/// gated out.
#[test]
fn st24_05_no_play_when_two_or_more_tamers() {
    let mut runner = base();
    runner.set_first_player(0);
    push_to_hand(&mut runner, 0, "DS-TAMER");
    runner.place_on_field(0, "PLAIN-TAMER", Some(0));
    runner.place_on_field(0, "PLAIN-TAMER", Some(0));
    let geo = runner.place_on_field(0, CARD_ID, Some(0));
    let hand_before = runner.hand_size(0);
    runner.fire_on_play(0, geo.index as usize);
    let _ = runner.auto_resolve();

    assert!(
        runner.game.pending_selection.is_none(),
        ">1 Tamer ⇒ the clause is gated out, no free-play prompt"
    );
    assert_eq!(runner.hand_size(0), hand_before, "no Tamer played");
}

/// EXACTLY 1 Tamer is still "1 or fewer" ⇒ the free play is offered.
#[test]
fn st24_05_play_offered_with_exactly_one_tamer() {
    let mut runner = base();
    runner.set_first_player(0);
    push_to_hand(&mut runner, 0, "DS-TAMER");
    runner.place_on_field(0, "PLAIN-TAMER", Some(0)); // exactly 1
    let geo = runner.place_on_field(0, CARD_ID, Some(0));
    runner.fire_on_play(0, geo.index as usize);

    let view = runner
        .pending_selection_view()
        .expect("1 Tamer is still '1 or fewer' ⇒ the free play surfaces");
    assert!(view.is_optional);
}

/// NEGATIVE (trait filter): a non-[DATA SQUAD] Tamer in hand is not a legal pick.
#[test]
fn st24_05_non_data_squad_tamer_not_playable() {
    let mut runner = base();
    runner.set_first_player(0);
    push_to_hand(&mut runner, 0, "PLAIN-TAMER");
    let geo = runner.place_on_field(0, CARD_ID, Some(0));
    let hand_before = runner.hand_size(0);
    runner.fire_on_play(0, geo.index as usize);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "a non-[DATA SQUAD] Tamer is not playable by this effect"
    );
}
