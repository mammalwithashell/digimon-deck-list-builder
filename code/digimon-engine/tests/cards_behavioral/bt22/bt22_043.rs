//! BT22-043 Terriermon — Digimon, Lv.3, Green/Yellow, DP 1000, Cost 3 (Beast,
//! CS).
//!
//! Printed (official Bandai DB bundle + DCGO BT22_043.cs):
//! - Evo box: Lv.2 Green / cost 1 OR Lv.2 Yellow / cost 1; "Digivolve Lv.2
//!   w/[CS] trait: Cost 0".
//! - [Your Turn] [Once Per Turn] When effects place Digimon cards with the [CS]
//!   trait in this Digimon's digivolution cards, if you have 1 or fewer Tamers,
//!   you may play 1 Tamer card with the [CS] trait from your hand without paying
//!   the cost.
//! - Inherited: [Main] [Once Per Turn] By placing this [CS] trait Digimon's top
//!   stacked card as its bottom digivolution card, <Draw 1>.
//!
//! The card had only a JSON stub (alt route only in `xros_req`). Synthetic
//! boards only.

use crate::alt_digivolve_support::*;
use std::sync::Arc;

use digimon_engine::action::{
    build_action_mask,
    space::{EFFECTS_PER_PERMANENT, FIELD_EFFECT_START, PASS},
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind, CardSourceRef, GamePhase};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const ID: &str = "BT22-043";

#[test]
fn bt22_043_green_or_yellow_lv2_circle_costs_1() {
    assert_single_route(ID, base("G2", "Green Rookie", 2, &[GREEN], &["Lesser"]), 1);
    assert_single_route(ID, base("Y2", "Yellow Rookie", 2, &[YELLOW], &["Lesser"]), 1);
}

#[test]
fn bt22_043_cs_lv2_of_any_colour_costs_0_and_prompts_on_a_circle_colour() {
    assert_single_route(ID, base("CS2", "Blue CS", 2, &[BLUE], &["CS"]), 0);
    assert_cost_choice(ID, base("GCS2", "Green CS", 2, &[GREEN], &["CS"]), 0, 1);
}

#[test]
fn bt22_043_other_bases_have_no_route() {
    assert_not_offered(ID, base("B2", "Blue Rookie", 2, &[BLUE], &["Lesser"]));
    assert_not_offered(ID, base("CS3", "CS Champion", 3, &[GREEN], &["CS"]));
}

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

fn tamer(id: &str, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![CardColor::Green];
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

struct PlaceHandUnder {
    target: PermanentHandle,
}
impl CardEffect for PlaceHandUnder {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let target = self.target;
        vec![Effect::on_play(card)
            .name("test placer: tuck hand card under a permanent")
            .process(move |ctx| {
                assert!(ctx.place_as_bottom_source(CardSourceRef::Hand(ctx.player, 0), target, false));
            })
            .build()]
    }
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(ID)
        .expect("BT22-043 production YAML loads")
        .add_card(digimon("PLACER", 3, &["Beast"]))
        .add_card(digimon("CS-DIGI", 3, &["CS"]))
        .add_card(digimon("PLAIN-DIGI", 3, &["Beast"]))
        .add_card(digimon("CS-TOP", 4, &["CS"]))
        .add_card(digimon("F", 3, &["Beast"]))
        .add_card(tamer("CS-TAMER", &["CS"]))
        .add_card(tamer("PLAIN-TAMER", &["Hero"]))
        .add_card(tamer("FIELD-TAMER", &["Hero"]))
        .deck(0, &["F"; 10])
        .deck(1, &["F"; 10])
        .memory(5)
}

fn tuck_runner(hand: &[&str], own_tamers: usize) -> (DebugRunner, PermanentHandle) {
    let mut r = builder().hand(0, hand).start();
    r.skip_mulligan();
    r.game.turn_count = 1;
    r.game.turn_player_idx = 0;
    let host = r.place_on_field(0, ID, Some(0));
    for _ in 0..own_tamers {
        r.place_on_field(0, "FIELD-TAMER", Some(0));
    }
    (r, host)
}

fn tuck(r: &mut DebugRunner, target: PermanentHandle) {
    r.register_effect("PLACER", Arc::new(PlaceHandUnder { target }));
    let placer = r.place_on_field(0, "PLACER", Some(0));
    r.game.fire_on_play(0, placer.index as usize);
}

#[test]
fn bt22_043_cs_digimon_placed_under_it_offers_a_free_cs_tamer_when_you_have_at_most_one_tamer() {
    // hand: [CS-DIGI (tucked), CS-TAMER, PLAIN-TAMER]
    let (mut r, host) = tuck_runner(&["CS-DIGI", "CS-TAMER", "PLAIN-TAMER"], 1);
    tuck(&mut r, host);
    assert_eq!(r.pending_kind(), Some(SelectionKind::Replacement), "outer yes/no");
    r.accept_optional_trigger().expect("accept");
    let view = r.pending_selection_view().expect("hand pick");
    assert_eq!(view.kind, SelectionKind::Hand);
    assert!(view.is_optional);
    let picks: Vec<u16> = view.valid_action_ids.iter().copied().filter(|&a| a != PASS).collect();
    assert_eq!(picks.len(), 1, "only the [CS] Tamer: {picks:?}");
    let before = r.memory();
    r.execute_action(0, picks[0]).unwrap();
    assert_eq!(r.memory(), before, "played without paying the cost");
    assert!(r
        .game
        .players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&r.game.card_data) == "CS-TAMER"));
}

#[test]
fn bt22_043_two_tamers_or_no_cs_tamer_or_a_non_cs_card_does_not_trigger() {
    let (mut r, host) = tuck_runner(&["CS-DIGI", "CS-TAMER"], 2);
    tuck(&mut r, host);
    assert!(r.pending_selection().is_none(), "2 Tamers: \"1 or fewer\"");

    let (mut r, host) = tuck_runner(&["CS-DIGI", "PLAIN-TAMER"], 0);
    tuck(&mut r, host);
    assert!(r.pending_selection().is_none(), "no [CS] Tamer in hand");

    let (mut r, host) = tuck_runner(&["PLAIN-DIGI", "CS-TAMER"], 0);
    tuck(&mut r, host);
    assert!(r.pending_selection().is_none(), "the placed Digimon card must carry [CS]");
}

// ─── Inherited [Main][OPT] ────────────────────────────────────────────────────

fn main_bit(host: PermanentHandle) -> usize {
    (FIELD_EFFECT_START + host.index as u16 * EFFECTS_PER_PERMANENT + 2) as usize
}

#[test]
fn bt22_043_inherited_main_rotates_the_cs_top_card_to_the_bottom_and_draws_1() {
    let mut r = builder().start();
    r.skip_mulligan();
    r.game.turn_count = 1;
    r.game.current_phase = GamePhase::Main;
    let host = r.place_stack(0, &[ID, "CS-TOP"]);
    assert!(build_action_mask(&r.game, 0)[main_bit(host)] > 0.0);
    let hand_before = r.hand_size(0);
    r.game.enqueue_triggered(
        digimon_engine::enums::EffectTiming::MainOnField,
        digimon_engine::selection::TriggerSource::Permanent(host),
    );
    r.game.drain_effect_queue();
    let _ = r.auto_resolve();
    let ids: Vec<String> = r.game.players[0].battle_area[host.index as usize]
        .card_sources
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect();
    assert_eq!(ids, vec!["CS-TOP".to_string(), ID.to_string()]);
    assert_eq!(r.hand_size(0), hand_before + 1, "<Draw 1>");
}
