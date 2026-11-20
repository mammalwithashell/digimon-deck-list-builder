//! BT21-080 Hiro Amanokawa — Tamer, Red+Purple, Cost 4 (Hero).
//!
//! Printed (official Bandai DB bundle + DCGO BT21_080.cs):
//! - [Start of Your Main Phase] If your opponent has a Digimon, gain 1 memory.
//! - [Your Turn] When effects place digivolution cards under any of your Digimon
//!   with [Gammamon] in their texts or the [Hero] trait, by suspending this Tamer,
//!   <Draw 1> and gain 1 memory.
//! - Security: [Security] Play this card without paying the cost.
//!
//! Regression (hunt2 cluster alt-digivolve-cost-cards): no YAML spec, so none of
//! the three clauses existed. Synthetic boards only.

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardKind, CardSourceRef, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;
use digimon_engine::TriggerSource;
use std::sync::Arc;

const ID: &str = "BT21-080";

fn digimon(id: &str, name: &str, traits: &[&str], text: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(3000);
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c.effect_text = text.to_string();
    c
}

/// The CAUSE: an [On Play] effect that places P0's hand[0] under `target`.
struct PlaceHandUnder {
    target: PermanentHandle,
}
impl CardEffect for PlaceHandUnder {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let target = self.target;
        vec![Effect::on_play(card)
            .name("test placer: tuck a hand card under a permanent")
            .process(move |ctx| {
                let ok = ctx.place_as_bottom_source(CardSourceRef::Hand(ctx.player, 0), target, false);
                assert!(ok, "placement must succeed");
            })
            .build()]
    }
}

fn runner() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(ID)
        .expect("BT21-080 production YAML loads")
        .add_card(digimon("GAMMA-NAME", "Gammamon", &["Dragonkin"], ""))
        .add_card(digimon("GAMMA-TEXT", "Plain Rookie", &["Beast"], "[When Attacking] Gammamon gets +1000 DP."))
        .add_card(digimon("HERO", "Hero Rookie", &["Hero"], ""))
        .add_card(digimon("PLAIN", "Plain Rookie", &["Beast"], ""))
        .add_card(digimon("OPP", "Opp Rookie", &["Beast"], ""))
        .add_card(digimon("PLACER", "Placer", &["Beast"], ""))
        .add_card(digimon("OPP-PLACER", "Opp Placer", &["Beast"], ""))
        .add_card(make_test_card("TUCK", "Tuck Card"))
        .add_card(make_test_card("PAD", "PAD"))
        .hand(0, &["TUCK", "PAD"])
        .deck(0, &["PAD"; 10])
        .deck(1, &["PAD"; 10])
        .memory(5)
        .start();
    r.skip_mulligan();
    r.game.turn_count = 1;
    r.game.turn_player_idx = 0;
    r
}

/// Place `host_id` for P0, put Hiro in play, then have a P0 effect tuck
/// hand[0] under the host. Returns (hiro, host).
fn tuck_under(r: &mut DebugRunner, host_id: &str) -> (PermanentHandle, PermanentHandle) {
    let hiro = r.place_on_field(0, ID, None);
    let host = r.place_on_field(0, host_id, None);
    r.register_effect("PLACER", Arc::new(PlaceHandUnder { target: host }));
    let placer = r.place_on_field(0, "PLACER", None);
    r.game.fire_on_play(0, placer.index as usize);
    (hiro, host)
}

fn is_suspended(r: &DebugRunner, h: PermanentHandle) -> bool {
    r.game.players[h.player as usize].battle_area[h.index as usize].is_suspended
}

// ─── [Your Turn] suspend: draw 1 and gain 1 memory ───────────────────────────

#[test]
fn bt21_080_gammamon_name_host_lets_hiro_suspend_for_draw_and_memory() {
    let mut r = runner();
    let mem = r.memory();
    let hand_before = r.hand_size(0);
    let (hiro, _) = tuck_under(&mut r, "GAMMA-NAME");
    assert_eq!(r.pending_kind(), Some(SelectionKind::TriggerOrder), "lone optional trigger");
    assert!(r.pending_is_optional(), "\"by suspending this Tamer\" is optional");
    r.accept_optional_trigger().expect("accept");
    assert!(is_suspended(&r, hiro), "the Tamer is suspended as the cost");
    assert_eq!(r.hand_size(0), hand_before - 1 + 1, "tucked 1 card away, drew 1");
    assert_eq!(r.memory(), mem + 1);
}

#[test]
fn bt21_080_gammamon_in_text_and_hero_trait_hosts_both_qualify() {
    for host in ["GAMMA-TEXT", "HERO"] {
        let mut r = runner();
        let (hiro, _) = tuck_under(&mut r, host);
        assert_eq!(r.pending_kind(), Some(SelectionKind::TriggerOrder), "{host}");
        r.accept_optional_trigger().expect("accept");
        assert!(is_suspended(&r, hiro), "{host}");
    }
}

#[test]
fn bt21_080_declining_keeps_hiro_unsuspended_and_gives_nothing() {
    let mut r = runner();
    let mem = r.memory();
    let (hiro, _) = tuck_under(&mut r, "GAMMA-NAME");
    r.decline_optional_trigger().expect("decline");
    assert!(!is_suspended(&r, hiro));
    assert_eq!(r.memory(), mem);
}

#[test]
fn bt21_080_a_host_without_gammamon_text_or_hero_trait_does_not_trigger_it() {
    let mut r = runner();
    let (_, _) = tuck_under(&mut r, "PLAIN");
    assert!(r.pending_selection().is_none(), "plain host: no prompt");
}

#[test]
fn bt21_080_an_already_suspended_hiro_cannot_pay_the_cost() {
    let mut r = runner();
    let hiro = r.place_on_field(0, ID, None);
    r.game.players[0].battle_area[hiro.index as usize].is_suspended = true;
    let host = r.place_on_field(0, "GAMMA-NAME", None);
    r.register_effect("PLACER", Arc::new(PlaceHandUnder { target: host }));
    let placer = r.place_on_field(0, "PLACER", None);
    r.game.fire_on_play(0, placer.index as usize);
    assert!(
        r.pending_selection().is_none(),
        "a suspended Tamer cannot be suspended again"
    );
}

#[test]
fn bt21_080_only_your_turn() {
    let mut r = runner();
    r.game.turn_player_idx = 1;
    let _ = tuck_under(&mut r, "GAMMA-NAME");
    assert!(
        r.pending_selection().is_none(),
        "[Your Turn] clause is inactive on the opponent's turn"
    );
}

// ─── [Start of Your Main Phase] ──────────────────────────────────────────────

fn start_of_main(r: &mut DebugRunner, hiro: PermanentHandle) {
    r.game
        .enqueue_triggered(EffectTiming::StartOfYourMainPhase, TriggerSource::Permanent(hiro));
    r.game.drain_effect_queue();
}

#[test]
fn bt21_080_start_of_main_gains_memory_only_if_the_opponent_has_a_digimon() {
    let mut r = runner();
    let hiro = r.place_on_field(0, ID, None);
    let mem = r.memory();
    start_of_main(&mut r, hiro);
    assert_eq!(r.memory(), mem, "opponent has no Digimon");

    r.place_on_field(1, "OPP", None);
    start_of_main(&mut r, hiro);
    assert_eq!(r.memory(), mem + 1, "opponent has a Digimon");
}

// ─── [Security] ──────────────────────────────────────────────────────────────

#[test]
fn bt21_080_security_plays_the_tamer_without_paying() {
    let mut r = runner();
    let attacker = r.place_on_field(0, "HERO", Some(0));
    let idx = r.game.card_data.iter().position(|c| c.card_id == ID).unwrap();
    let iid = r.game.next_card_index();
    r.game.players[1].security.push(CardSource::new(idx, 1, iid));
    r.attack_player(attacker, 1, false);
    let _ = r.auto_resolve();
    assert!(
        r.game.players[1]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&r.game.card_data) == ID),
        "Hiro is played from security"
    );
}
