//! ST23-10 Pristimon — Digimon, Lv.3, Black, DP 2000, Cost 3.
//! Traits: Puppet, Glowing Dawn, BEATBREAK. Attribute: Vaccine.
//!
//! # Card text (data/cards.json — verbatim)
//! [On Play] By placing 1 card from your hand face down under any of your Tamers
//! with the [Glowing Dawn] trait, ＜Draw 2＞.
//! Inherited: ＜Blocker＞.
//!
//! Printed digivolve box: [Digivolve] Lv.2 w/[Glowing Dawn] trait: Cost 0.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/ST23/Black/ST23_10.cs
//!
//! # Patterns (RUST_DSL_TEST_API §4.3)
//! - face-down hand-stash substrate (place_selected_card_under_tamer)
//! - "by placing … , <Draw 2>" cost-then-payoff (draw gated on the place)
//! - H5 inherited <Blocker> keyword grant

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

const CARD_ID: &str = "ST23-10";

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn gd_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![CardColor::Black];
    c.traits = vec!["Glowing Dawn".to_string(), "BEATBREAK".to_string()];
    c
}

fn plain_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![CardColor::Black];
    c
}

fn filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Black];
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
        .expect("ST23-10 in embedded DSL pack")
        .add_card(gd_tamer("GD-TAMER"))
        .add_card(plain_tamer("PLAIN-TAMER"))
        .add_card(filler("FILLER"))
        .deck(0, &["FILLER"; 10])
        .deck(1, &["FILLER"; 10])
        .memory(8)
        .start()
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn st23_10_metadata_and_alt_path() {
    let runner = base();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.name, "Pristimon");
    assert_eq!(card.level, Some(3));
    assert_eq!(card.cost, Some(3));
    let gd = card.alt_paths.iter().any(|p| {
        p.from
            .as_ref()
            .and_then(|f| f.trait_has.as_deref())
            .map(|t| t == "Glowing Dawn")
            .unwrap_or(false)
    });
    assert!(gd, "Lv.2 [Glowing Dawn] alt-path present");
}

#[test]
fn st23_10_has_on_play_clause_and_inherited_blocker() {
    let runner = base();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    let has_op = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnPlay)
        )
    });
    assert!(has_op, "[On Play] clause present");
    let has_blocker = card.effects.iter().any(|c| matches!(
        c,
        CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { keyword, scope, .. })
            if keyword == "Blocker" && *scope == CompiledScope::Inherited
    ));
    assert!(has_blocker, "inherited <Blocker> present");
}

// ─── Section 2 — Behavioral ──────────────────────────────────────────────────

/// POSITIVE: with a [Glowing Dawn] Tamer on field and a hand card, the On Play
/// places the chosen hand card face-down under the Tamer and draws 2.
#[test]
fn st23_10_places_hand_card_face_down_and_draws_two() {
    let mut runner = base();
    runner.set_first_player(0);
    let tamer = runner.place_on_field(0, "GD-TAMER", Some(0));
    push_to_hand(&mut runner, 0, "FILLER"); // the stash card
    let arma = runner.place_on_field(0, CARD_ID, Some(0));

    let hand_before = runner.hand_size(0);
    let deck_before = runner.deck_size(0);
    let tamer_sources_before = runner.game.players[0].battle_area[tamer.index as usize]
        .card_sources
        .len();

    runner.fire_on_play(0, arma.index as usize);

    // First prompt: choose the hand card to stash.
    let v = runner
        .pending_selection_view()
        .expect("hand-card pick installs");
    runner.execute_action(0, v.valid_action_ids[0]).unwrap();
    // Second prompt: choose the [Glowing Dawn] Tamer.
    let v2 = runner
        .pending_selection_view()
        .expect("Tamer pick installs after the hand pick");
    runner.execute_action(0, v2.valid_action_ids[0]).unwrap();
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.game.players[0].battle_area[tamer.index as usize]
            .card_sources
            .len(),
        tamer_sources_before + 1,
        "the stash card was placed under the Tamer"
    );
    assert!(
        runner.game.players[0].battle_area[tamer.index as usize].card_sources[0].face_down,
        "the placed source is face-down at the bottom of the Tamer's stack"
    );
    // Hand: -1 stash card, then +2 from <Draw 2> = net +1.
    assert_eq!(
        runner.hand_size(0),
        hand_before - 1 + 2,
        "1 card stashed, then 2 drawn (net +1)"
    );
    assert_eq!(
        runner.deck_size(0),
        deck_before - 2,
        "<Draw 2> drew 2 cards"
    );
}

/// DECLINE the hand-card pick → no placement, no draw (the cost was not paid).
#[test]
fn st23_10_declining_hand_pick_does_nothing() {
    let mut runner = base();
    runner.set_first_player(0);
    let tamer = runner.place_on_field(0, "GD-TAMER", Some(0));
    push_to_hand(&mut runner, 0, "FILLER");
    let arma = runner.place_on_field(0, CARD_ID, Some(0));

    let hand_before = runner.hand_size(0);
    let deck_before = runner.deck_size(0);
    let tamer_sources_before = runner.game.players[0].battle_area[tamer.index as usize]
        .card_sources
        .len();

    runner.fire_on_play(0, arma.index as usize);
    let v = runner
        .pending_selection_view()
        .expect("hand-card pick installs");
    assert!(v.is_optional, "the placement is optional");
    runner
        .execute_action(0, digimon_engine::action::space::PASS)
        .expect("decline the placement");
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.game.players[0].battle_area[tamer.index as usize]
            .card_sources
            .len(),
        tamer_sources_before,
        "declined ⇒ nothing placed"
    );
    assert_eq!(runner.hand_size(0), hand_before, "declined ⇒ no draw");
    assert_eq!(runner.deck_size(0), deck_before, "declined ⇒ no draw");
}

/// NEGATIVE (gate): no [Glowing Dawn] Tamer on field ⇒ the clause is gated out,
/// no prompt and no draw.
#[test]
fn st23_10_no_glowing_dawn_tamer_no_prompt() {
    let mut runner = base();
    runner.set_first_player(0);
    // A plain (non-Glowing-Dawn) Tamer is on field.
    runner.place_on_field(0, "PLAIN-TAMER", Some(0));
    push_to_hand(&mut runner, 0, "FILLER");
    let arma = runner.place_on_field(0, CARD_ID, Some(0));
    let deck_before = runner.deck_size(0);

    runner.fire_on_play(0, arma.index as usize);
    let _ = runner.auto_resolve();

    assert!(
        runner.game.pending_selection.is_none(),
        "no [Glowing Dawn] Tamer ⇒ the clause is gated out"
    );
    assert_eq!(runner.deck_size(0), deck_before, "no draw");
}
