//! BT22-004 Wanyamon — Digi-Egg, Lv.2, Green, Cost 0 (Lesser, CS).
//!
//! Printed (official Bandai DB bundle + DCGO BT22_004.cs):
//! Inherited Effect [Your Turn] [Once Per Turn] When effects place [CS] trait
//! Digimon cards in this Digimon's digivolution cards, it may digivolve into a
//! [CS] trait Digimon card in the hand with the digivolution cost reduced by 1.
//!
//! Same shape as BT25-005 Pagumon (see bt25_005.rs for the full rationale).
//! The card had only a JSON stub before (hunt cluster
//! alt-digivolve-xros-req-cards); synthetic boards only.

use std::sync::Arc;

use digimon_engine::action::space::PASS;
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind, CardSourceRef, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const ID: &str = "BT22-004";

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

/// A green Lv.4 with a printed "Green Lv.3 / cost 3" circle.
fn lv4(id: &str, traits: &[&str]) -> CardData {
    let mut c = digimon(id, 4, traits);
    c.evo_costs = vec![EvoCost {
        card_color: CardColor::Green as u8,
        level: 3,
        memory_cost: 3,
    }];
    c
}

fn cs_option(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Option;
    c.colors = vec![CardColor::Green];
    c.level = None;
    c.dp = None;
    c.play_cost = 2;
    c.traits = vec!["CS".into()];
    c
}

/// The CAUSE: an [On Play] effect that places `count` cards of its
/// controller's hand under `target`.
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
                    let ok = ctx.place_as_bottom_source(CardSourceRef::Hand(ctx.player, 0), target, false);
                    assert!(ok, "placement must succeed");
                }
            })
            .build()]
    }
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(ID)
        .expect("BT22-004 YAML loads from the embedded pack")
        .add_card(digimon("HOST", 3, &["Beast"]))
        .add_card(digimon("PLACER", 3, &["Beast"]))
        .add_card(digimon("CS-DIGI", 3, &["CS"]))
        .add_card(digimon("PLAIN-DIGI", 3, &["Beast"]))
        .add_card(cs_option("CS-OPT"))
        .add_card(lv4("CS4", &["CS"]))
        .add_card(lv4("CS4B", &["CS"]))
        .add_card(lv4("PLAIN4", &["Beast"]))
        .add_card(digimon("F", 3, &["Beast"]))
        .deck(0, &["F"; 10])
        .deck(1, &["F"; 10])
        .memory(5)
}

fn host_with_wanyamon(r: &mut DebugRunner) -> PermanentHandle {
    r.game.turn_count = 1;
    r.game.turn_player_idx = 0;
    r.place_stack(0, &[ID, "HOST"])
}

fn tuck(r: &mut DebugRunner, player: PlayerId, count: usize, target: PermanentHandle) {
    r.register_effect("PLACER", Arc::new(PlaceHandUnder { count, target }));
    let placer = r.place_on_field(player, "PLACER", Some(0));
    r.game.fire_on_play(player, placer.index as usize);
}

fn top_id(r: &DebugRunner, h: PermanentHandle) -> String {
    r.game.players[h.player as usize].battle_area[h.index as usize]
        .top_card()
        .card_id(&r.game.card_data)
        .to_string()
}

fn accept_outer(r: &mut DebugRunner) {
    assert_eq!(r.pending_kind(), Some(SelectionKind::Replacement), "outer yes/no");
    assert!(r.pending_is_optional());
    r.accept_optional_trigger().expect("accept 'may digivolve'");
}

#[test]
fn bt22_004_cs_digimon_placed_under_it_offers_a_cost_minus_1_digivolve_into_a_cs_digimon() {
    let mut r = base().hand(0, &["CS-DIGI", "CS4", "CS4B", "PLAIN4"]).start();
    let host = host_with_wanyamon(&mut r);
    tuck(&mut r, 0, 1, host);
    accept_outer(&mut r);

    assert_eq!(r.pending_kind(), Some(SelectionKind::Hand));
    assert!(r.pending_is_optional(), "the pick is declinable (DCGO canNoSelect)");
    let view = r.pending_selection_view().unwrap();
    let picks: Vec<u16> = view.valid_action_ids.iter().copied().filter(|&a| a != PASS).collect();
    // Only the two [CS] Digimon cards in hand (PLAIN4 lacks the trait).
    assert_eq!(picks.len(), 2, "only [CS] trait Digimon: {picks:?}");
    let before = r.memory();
    r.execute_action(0, picks[0]).unwrap();
    assert!(top_id(&r, host).starts_with("CS4"), "got {}", top_id(&r, host));
    assert_eq!(before - r.memory(), 2, "printed cost 3 reduced by 1");
}

#[test]
fn bt22_004_non_cs_digimon_or_cs_option_does_not_trigger() {
    for placed in ["PLAIN-DIGI", "CS-OPT"] {
        let mut r = base().hand(0, &[placed, "CS4"]).start();
        let host = host_with_wanyamon(&mut r);
        tuck(&mut r, 0, 1, host);
        assert!(
            r.pending_selection().is_none(),
            "{placed}: only a [CS] trait DIGIMON card placed under it triggers"
        );
        assert_eq!(top_id(&r, host), "HOST");
    }
}

#[test]
fn bt22_004_is_inactive_on_the_opponents_turn() {
    let mut r = base().hand(0, &["CS-DIGI", "CS4"]).start();
    let host = host_with_wanyamon(&mut r);
    r.game.turn_player_idx = 1;
    tuck(&mut r, 0, 1, host);
    assert!(r.pending_selection().is_none(), "[Your Turn] only");
}

#[test]
fn bt22_004_declining_the_yes_no_refunds_the_once_per_turn() {
    let mut r = base().hand(0, &["CS-DIGI", "CS4", "CS-DIGI", "CS4B"]).start();
    let host = host_with_wanyamon(&mut r);
    tuck(&mut r, 0, 1, host);
    assert_eq!(r.pending_kind(), Some(SelectionKind::Replacement));
    r.decline_optional_trigger().expect("decline the yes/no");
    assert!(r.pending_selection().is_none());
    // A second placement the same turn triggers again (the OPT was refunded).
    tuck(&mut r, 0, 1, host);
    assert_eq!(r.pending_kind(), Some(SelectionKind::Replacement));
}
