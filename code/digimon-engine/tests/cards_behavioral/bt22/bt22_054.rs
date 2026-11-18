//! BT22-054 Hagurumon — Digimon, Lv.3, Black/Yellow, DP 1000, Cost 3 (Machine,
//! CS).
//!
//! Printed (official Bandai DB bundle + DCGO BT22_054.cs):
//! - Evo box: Lv.2 Black / cost 1 OR Lv.2 Yellow / cost 1; "Digivolve Lv.2
//!   w/[CS] trait: Cost 0".
//! - [Your Turn] [Once Per Turn] When effects place Digimon cards with the [CS]
//!   trait in this Digimon's digivolution cards, 1 of your opponent's Digimon
//!   gets -3000 DP for the turn.
//! - Inherited: [Main] [Once Per Turn] By placing this [CS] trait Digimon's top
//!   stacked card as its bottom digivolution card, <Draw 1>.
//!
//! The card had only a JSON stub (alt route only in `xros_req`). Synthetic
//! boards only.

use crate::alt_digivolve_support::*;
use std::sync::Arc;

use digimon_engine::action::space::ATTACK_START;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind, CardSourceRef, GamePhase};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const ID: &str = "BT22-054";

#[test]
fn bt22_054_black_or_yellow_lv2_circle_costs_1() {
    assert_single_route(ID, base("K2", "Black Rookie", 2, &[BLACK], &["Lesser"]), 1);
    assert_single_route(ID, base("Y2", "Yellow Rookie", 2, &[YELLOW], &["Lesser"]), 1);
}

#[test]
fn bt22_054_cs_lv2_of_any_colour_costs_0_and_prompts_on_a_circle_colour() {
    assert_single_route(ID, base("CS2", "Blue CS", 2, &[BLUE], &["CS"]), 0);
    assert_cost_choice(ID, base("KCS2", "Black CS", 2, &[BLACK], &["CS"]), 0, 1);
}

#[test]
fn bt22_054_other_bases_have_no_route() {
    assert_not_offered(ID, base("B2", "Blue Rookie", 2, &[BLUE], &["Lesser"]));
    assert_not_offered(ID, base("CS3", "CS Champion", 3, &[BLACK], &["CS"]));
}

fn digimon(id: &str, level: u8, dp: i32, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Green];
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = 4;
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
        .expect("BT22-054 production YAML loads")
        .add_card(digimon("PLACER", 3, 3000, &["Beast"]))
        .add_card(digimon("CS-DIGI", 3, 3000, &["CS"]))
        .add_card(digimon("PLAIN-DIGI", 3, 3000, &["Beast"]))
        .add_card(digimon("OPP", 4, 5000, &["Beast"]))
        .add_card(digimon("CS-TOP", 4, 4000, &["CS"]))
        .add_card(digimon("F", 3, 1000, &["Beast"]))
        .deck(0, &["F"; 10])
        .deck(1, &["F"; 10])
        .memory(5)
}

fn tuck_runner(hand: &[&str], your_turn: bool) -> (DebugRunner, PermanentHandle, PermanentHandle) {
    let mut r = builder().hand(0, hand).start();
    r.skip_mulligan();
    r.game.turn_count = 1;
    r.game.turn_player_idx = if your_turn { 0 } else { 1 };
    let host = r.place_on_field(0, ID, Some(0));
    let opp = r.place_on_field(1, "OPP", Some(0));
    (r, host, opp)
}

fn tuck(r: &mut DebugRunner, target: PermanentHandle) {
    r.register_effect("PLACER", Arc::new(PlaceHandUnder { target }));
    let placer = r.place_on_field(0, "PLACER", Some(0));
    r.game.fire_on_play(0, placer.index as usize);
}

#[test]
fn bt22_054_cs_digimon_placed_under_it_gives_an_opponent_digimon_minus_3000_dp() {
    let (mut r, host, opp) = tuck_runner(&["CS-DIGI"], true);
    tuck(&mut r, host);
    let view = r.pending_selection_view().expect("mandatory target pick parks");
    assert!(!view.is_optional, "DCGO canNoSelect: false");
    assert_eq!(view.kind, SelectionKind::OppField);
    r.execute_action(0, ATTACK_START + opp.index as u16).unwrap();
    assert_eq!(r.effective_dp(opp), Some(5000 - 3000));
    r.end_turn();
    assert_eq!(r.effective_dp(opp), Some(5000), "for the turn only");
}

#[test]
fn bt22_054_non_cs_card_opponents_turn_or_no_target_does_not_trigger() {
    let (mut r, host, _opp) = tuck_runner(&["PLAIN-DIGI"], true);
    tuck(&mut r, host);
    assert!(r.pending_selection().is_none(), "needs a [CS] Digimon card placed");

    let (mut r, host, _opp) = tuck_runner(&["CS-DIGI"], false);
    tuck(&mut r, host);
    assert!(r.pending_selection().is_none(), "[Your Turn] only");
}

#[test]
fn bt22_054_inherited_main_rotates_the_cs_top_card_to_the_bottom_and_draws_1() {
    let mut r = builder().start();
    r.skip_mulligan();
    r.game.turn_count = 1;
    r.game.current_phase = GamePhase::Main;
    let host = r.place_stack(0, &[ID, "CS-TOP"]);
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
