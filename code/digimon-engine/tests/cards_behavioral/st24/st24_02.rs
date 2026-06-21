//! ST24-02 Gaomon — Digimon, Lv.3, Blue, DP 2000, Cost 3.
//! Traits: Beast, DATA SQUAD.
//!
//! # Card text (data/cards.json — verbatim)
//! [On Play] By placing 1 card from your hand face down under any of your Tamers
//!   with the [DATA SQUAD] trait, ＜Draw 2＞.
//! Inherited [When Attacking] [Once Per Turn] If your hand has 7 or fewer cards,
//!   ＜Draw 1＞.
//!
//! Printed digivolve box (image/DCGO — OMITTED from cards.json effect text):
//!   [Digivolve] Lv.2 w/[DATA SQUAD] trait: Cost 0.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/ST24/Blue/ST24_02.cs
//!
//! # Patterns — sister to ST23-10 (trait-swapped) + inherited conditional draw
//! - face-down hand-stash substrate (place_selected_card_under_tamer)
//! - "by placing … , <Draw 2>" cost-then-payoff (draw gated on the place)
//! - inherited [When Attacking][OPT] hand-size-gated <Draw 1> (your turn)

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

const CARD_ID: &str = "ST24-02";

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn ds_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![CardColor::Blue];
    c.traits = vec!["DATA SQUAD".to_string()];
    c
}

fn plain_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![CardColor::Blue];
    c
}

fn carrier_lv3(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Blue];
    c.level = Some(3);
    c.dp = Some(2000);
    c.play_cost = 3;
    c.traits = vec!["DATA SQUAD".to_string()];
    c
}

fn filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Blue];
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

fn clear_hand(runner: &mut DebugRunner, p: u8) {
    runner.game.players[p as usize].hand.clear();
}

fn base() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST24-02 in embedded DSL pack")
        .add_card(ds_tamer("DS-TAMER"))
        .add_card(plain_tamer("PLAIN-TAMER"))
        .add_card(carrier_lv3("CARRIER"))
        .add_card(filler("FILLER"))
        .deck(0, &["FILLER"; 12])
        .deck(1, &["FILLER"; 12])
        .memory(8)
        .start()
}

fn fire(r: &mut DebugRunner, timing: EffectTiming, handle: PermanentHandle) {
    r.game
        .enqueue_triggered(timing, TriggerSource::Permanent(handle));
    r.game.drain_effect_queue();
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn st24_02_metadata_and_alt_path() {
    let runner = base();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.name, "Gaomon");
    assert_eq!(card.level, Some(3));
    assert_eq!(card.cost, Some(3));
    let ds = card.alt_paths.iter().any(|p| {
        p.from
            .as_ref()
            .and_then(|f| f.trait_has.as_deref())
            .map(|t| t == "DATA SQUAD")
            .unwrap_or(false)
    });
    assert!(ds, "Lv.2 [DATA SQUAD] alt-path present");
}

#[test]
fn st24_02_has_on_play_and_inherited_when_attacking() {
    let runner = base();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    let has_op = card.effects.iter().any(|c| {
        matches!(c, CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnPlay) && t.scope != CompiledScope::Inherited)
    });
    assert!(has_op, "[On Play] clause present");
    let has_inherited_wa = card.effects.iter().any(|c| {
        matches!(c, CompiledClause::Triggered(t)
            if t.scope == CompiledScope::Inherited
            && t.when.contains(&CompiledTiming::WhenAttacking)
            && t.once_per_turn)
    });
    assert!(has_inherited_wa, "inherited [When Attacking][OPT] draw present");
}

// ─── Section 2 — On Play (place under DATA SQUAD Tamer + Draw 2) ──────────────

#[test]
fn st24_02_places_hand_card_face_down_and_draws_two() {
    let mut runner = base();
    runner.set_first_player(0);
    let tamer = runner.place_on_field(0, "DS-TAMER", Some(0));
    push_to_hand(&mut runner, 0, "FILLER"); // the stash card
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
    runner.execute_action(0, v.valid_action_ids[0]).unwrap();
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
    assert_eq!(
        runner.hand_size(0),
        hand_before - 1 + 2,
        "1 card stashed, then 2 drawn (net +1)"
    );
    assert_eq!(runner.deck_size(0), deck_before - 2, "<Draw 2> drew 2 cards");
}

#[test]
fn st24_02_declining_hand_pick_does_nothing() {
    let mut runner = base();
    runner.set_first_player(0);
    let tamer = runner.place_on_field(0, "DS-TAMER", Some(0));
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

#[test]
fn st24_02_no_data_squad_tamer_no_prompt() {
    let mut runner = base();
    runner.set_first_player(0);
    runner.place_on_field(0, "PLAIN-TAMER", Some(0));
    push_to_hand(&mut runner, 0, "FILLER");
    let arma = runner.place_on_field(0, CARD_ID, Some(0));
    let deck_before = runner.deck_size(0);

    runner.fire_on_play(0, arma.index as usize);
    let _ = runner.auto_resolve();

    assert!(
        runner.game.pending_selection.is_none(),
        "no [DATA SQUAD] Tamer ⇒ the clause is gated out"
    );
    assert_eq!(runner.deck_size(0), deck_before, "no draw");
}

// ─── Section 3 — Inherited [When Attacking][OPT] conditional <Draw 1> ─────────

/// POSITIVE: hand ≤7 on your turn ⇒ the inherited draw fires.
#[test]
fn st24_02_inherited_draw_when_hand_seven_or_fewer() {
    let mut r = base();
    r.set_first_player(0);
    let carrier = r.place_stack(0, &[CARD_ID, "CARRIER"]);
    clear_hand(&mut r, 0);
    for _ in 0..5 {
        push_to_hand(&mut r, 0, "FILLER");
    }
    let hand_before = r.hand_size(0); // 5 ≤ 7
    let deck_before = r.deck_size(0);

    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    let _ = r.auto_resolve();

    assert_eq!(r.hand_size(0), hand_before + 1, "hand ≤7 ⇒ <Draw 1> fired");
    assert_eq!(r.deck_size(0), deck_before - 1, "drew 1 from deck");
}

/// NEGATIVE (gate): hand >7 ⇒ the inherited draw does not fire.
#[test]
fn st24_02_inherited_draw_not_when_hand_over_seven() {
    let mut r = base();
    r.set_first_player(0);
    let carrier = r.place_stack(0, &[CARD_ID, "CARRIER"]);
    clear_hand(&mut r, 0);
    for _ in 0..8 {
        push_to_hand(&mut r, 0, "FILLER");
    }
    let hand_before = r.hand_size(0); // 8 > 7
    let deck_before = r.deck_size(0);

    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    let _ = r.auto_resolve();

    assert_eq!(r.hand_size(0), hand_before, "hand >7 ⇒ no draw");
    assert_eq!(r.deck_size(0), deck_before, "hand >7 ⇒ no draw");
}
