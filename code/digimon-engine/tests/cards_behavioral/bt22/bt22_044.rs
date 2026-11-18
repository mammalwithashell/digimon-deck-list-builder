//! BT22-044 Palmon — Digimon, Lv.3, Green/Red, DP 1000, Cost 3 (Vegetation,
//! CS).
//!
//! Printed (official Bandai DB bundle + DCGO BT22_044.cs):
//! - Evo box: Lv.2 Green / cost 1 OR Lv.2 Red / cost 1; "Digivolve Lv.2 w/[CS]
//!   trait: Cost 0".
//! - [Your Turn] [Once Per Turn] When effects place Digimon cards with the [CS]
//!   trait in this Digimon's digivolution cards, gain 1 memory.
//! - Inherited: [Main] [Once Per Turn] By placing this [CS] trait Digimon's top
//!   stacked card as its bottom digivolution card, <Draw 1>.
//!
//! Regression (hunt cluster alt-digivolve-xros-req-cards): no YAML spec, so the
//! [CS] cost-0 route lived only in `xros_req` — a Green/Red [CS] Lv.2 base got
//! no cost-choice prompt and an off-colour [CS] base could not digivolve.
//! Synthetic boards only.

use crate::alt_digivolve_support::*;
use std::sync::Arc;

use digimon_engine::action::{
    build_action_mask,
    space::{EFFECTS_PER_PERMANENT, FIELD_EFFECT_START},
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind, CardSourceRef, EffectTiming, GamePhase};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

const ID: &str = "BT22-044";

// ─── Digivolution routes ──────────────────────────────────────────────────────

#[test]
fn bt22_044_green_or_red_lv2_circle_costs_1() {
    assert_single_route(ID, base("G2", "Green Rookie", 2, &[GREEN], &["Lesser"]), 1);
    assert_single_route(ID, base("R2", "Red Rookie", 2, &[RED], &["Lesser"]), 1);
}

#[test]
fn bt22_044_cs_lv2_of_any_colour_costs_0() {
    assert_single_route(ID, base("CS2", "Blue CS", 2, &[BLUE], &["CS"]), 0);
}

#[test]
fn bt22_044_green_cs_lv2_prompts_cost_0_vs_1() {
    assert_cost_choice(ID, base("GCS2", "Green CS", 2, &[GREEN], &["CS"]), 0, 1);
    assert_cost_choice(ID, base("RCS2", "Red CS", 2, &[RED], &["CS"]), 0, 1);
}

#[test]
fn bt22_044_other_bases_have_no_route() {
    assert_not_offered(ID, base("B2", "Blue Rookie", 2, &[BLUE], &["Lesser"]));
    assert_not_offered(ID, base("CS3", "CS Champion", 3, &[GREEN], &["CS"]));
}

// ─── [Your Turn][OPT] gain 1 memory ───────────────────────────────────────────

fn digimon(id: &str, level: u8, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Green];
    c.level = Some(level);
    c.dp = Some(3000);
    c.play_cost = 4;
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

struct PlaceHandUnder {
    count: usize,
    target: PermanentHandle,
}
impl CardEffect for PlaceHandUnder {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let count = self.count;
        let target = self.target;
        vec![Effect::on_play(card)
            .name("test placer: tuck hand cards under a permanent")
            .process(move |ctx| {
                for _ in 0..count {
                    assert!(ctx.place_as_bottom_source(CardSourceRef::Hand(ctx.player, 0), target, false));
                }
            })
            .build()]
    }
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(ID)
        .expect("BT22-044 production YAML loads")
        .add_card(digimon("PLACER", 3, &["Beast"]))
        .add_card(digimon("CS-DIGI", 3, &["CS"]))
        .add_card(digimon("PLAIN-DIGI", 3, &["Beast"]))
        .add_card(digimon("CS-TOP", 4, &["CS"]))
        .add_card(digimon("PLAIN-TOP", 4, &["Beast"]))
        .add_card(digimon("F", 3, &["Beast"]))
        .deck(0, &["F"; 10])
        .deck(1, &["F"; 10])
        .memory(5)
}

fn tuck(r: &mut DebugRunner, count: usize, target: PermanentHandle) {
    r.register_effect("PLACER", Arc::new(PlaceHandUnder { count, target }));
    let placer = r.place_on_field(0, "PLACER", Some(0));
    r.game.fire_on_play(0, placer.index as usize);
}

#[test]
fn bt22_044_gains_1_memory_when_an_effect_places_a_cs_digimon_card_under_it() {
    let mut r = builder().hand(0, &["CS-DIGI"]).start();
    r.game.turn_count = 1;
    r.game.turn_player_idx = 0;
    let palmon = r.place_on_field(0, ID, Some(0));
    let before = r.memory();
    tuck(&mut r, 1, palmon);
    assert_eq!(r.memory(), before + 1);
}

#[test]
fn bt22_044_a_non_cs_digimon_card_placed_under_it_gains_nothing() {
    let mut r = builder().hand(0, &["PLAIN-DIGI"]).start();
    r.game.turn_count = 1;
    r.game.turn_player_idx = 0;
    let palmon = r.place_on_field(0, ID, Some(0));
    let before = r.memory();
    tuck(&mut r, 1, palmon);
    assert_eq!(r.memory(), before);
}

#[test]
fn bt22_044_is_once_per_turn_and_your_turn_only() {
    let mut r = builder().hand(0, &["CS-DIGI", "CS-DIGI"]).start();
    r.game.turn_count = 1;
    r.game.turn_player_idx = 0;
    let palmon = r.place_on_field(0, ID, Some(0));
    let before = r.memory();
    tuck(&mut r, 1, palmon);
    tuck(&mut r, 1, palmon);
    assert_eq!(r.memory(), before + 1, "[Once Per Turn]");

    let mut r = builder().hand(0, &["CS-DIGI"]).start();
    r.game.turn_count = 1;
    r.game.turn_player_idx = 1;
    let palmon = r.place_on_field(0, ID, Some(0));
    let before = r.memory();
    tuck(&mut r, 1, palmon);
    assert_eq!(r.memory(), before, "[Your Turn] only");
}

// ─── Inherited [Main][OPT]: top stacked card to the bottom, Draw 1 ────────────

fn main_bit(host: PermanentHandle) -> usize {
    (FIELD_EFFECT_START + host.index as u16 * EFFECTS_PER_PERMANENT + 2) as usize
}

fn main_runner(top: &str) -> (DebugRunner, PermanentHandle) {
    let mut r = builder().start();
    r.skip_mulligan();
    r.game.turn_count = 1;
    r.game.current_phase = GamePhase::Main;
    // Palmon is the bottom source; `top` is the visible top card.
    let host = r.place_stack(0, &[ID, top]);
    (r, host)
}

fn stack_ids(r: &DebugRunner, h: PermanentHandle) -> Vec<String> {
    r.game.players[h.player as usize].battle_area[h.index as usize]
        .card_sources
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

#[test]
fn bt22_044_inherited_main_rotates_the_top_card_to_the_bottom_and_draws_1() {
    let (mut r, host) = main_runner("CS-TOP");
    assert!(
        build_action_mask(&r.game, 0)[main_bit(host)] > 0.0,
        "a [CS] top card over a digivolution card offers the inherited [Main]"
    );
    let hand_before = r.hand_size(0);
    let mem_before = r.memory();
    r.game
        .enqueue_triggered(EffectTiming::MainOnField, TriggerSource::Permanent(host));
    r.game.drain_effect_queue();
    let _ = r.auto_resolve();
    assert_eq!(
        stack_ids(&r, host),
        vec!["CS-TOP".to_string(), ID.to_string()],
        "the top stacked card became the bottom digivolution card; Palmon is the new top"
    );
    assert_eq!(r.hand_size(0), hand_before + 1, "<Draw 1>");
    // Official Q&A: Palmon becoming the top card by this effect still triggers
    // its [Your Turn] clause ("a [CS] Digimon card placed under it by an effect").
    assert_eq!(r.memory(), mem_before + 1);
}

#[test]
fn bt22_044_inherited_main_needs_a_cs_top_card() {
    let (r, host) = main_runner("PLAIN-TOP");
    assert_eq!(
        build_action_mask(&r.game, 0)[main_bit(host)],
        0.0,
        "the top card must carry [CS]"
    );
}
