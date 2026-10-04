//! BT26-104 Kunlun — Tamer, White, Shambala/SW/TB/TS.
//!
//! [Start of Your Main Phase] Gain 1 memory.
//! [On Play] By trashing 1 [Shambala] trait card from your hand, <Draw 2>.
//! [End of Your Turn] If you have a Digimon with the [Tentei Hachibushu] trait,
//! by suspending this Tamer, you may use 1 Option card with the [Shambala]
//! trait from your hand without paying the cost.
//! [Security] Play this card without paying the cost.
//!
//! DCGO: BT26/White/BT26_104.cs.

use std::sync::Arc;

use super::support::*;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::{CardEffect, Effect};

const CARD_ID: &str = "BT26-104";

struct OptionMainNoop;

impl CardEffect for OptionMainNoop {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::when_attacking(card)
            .option_main()
            .name("OptionMain no-op")
            .process(|_| {})
            .build()]
    }
}

fn option(id: &str, cost: u16, traits: &[&str]) -> digimon_engine::CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Option;
    c.play_cost = cost;
    c.colors = vec![CardColor::White];
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-104")
        .add_card(filler("FILLER"))
        .add_card(digimon(
            "SHAMBALA-D",
            "Shambala D",
            CardColor::White,
            3,
            3,
            &["Shambala"],
        ))
        .add_card(digimon("PLAIN", "Plain", CardColor::White, 3, 3, &["TB"]))
        .add_card(digimon(
            "TENTEI",
            "Tentei",
            CardColor::White,
            5,
            7,
            &["Tentei Hachibushu"],
        ))
        .add_card(option("SH-OPT", 7, &["Shambala"]))
        .add_card(option("TB-OPT", 1, &["TB"]))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(3)
        .start();
    r.register_effect("SH-OPT", Arc::new(OptionMainNoop));
    r.register_effect("TB-OPT", Arc::new(OptionMainNoop));
    r.set_first_player(0);
    r
}

#[test]
fn bt26_104_somp_gains_one_memory() {
    let mut r = setup();
    let k = r.place_on_field(0, CARD_ID, Some(0));
    let mem = r.memory();
    fire(&mut r, EffectTiming::StartOfYourMainPhase, k);
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), mem + 1);
}

#[test]
fn bt26_104_on_play_trash_shambala_draws_two() {
    let mut r = setup();
    push_hand(&mut r, 0, "PLAIN");
    push_hand(&mut r, 0, "SHAMBALA-D");
    let k = r.place_on_field(0, CARD_ID, Some(0));
    fire(&mut r, EffectTiming::OnPlay, k);
    let v = r.pending_selection_view().expect("optional trash pick");
    assert!(v.is_optional);
    assert_eq!(v.valid_action_ids.len(), 1, "only the Shambala card");
    pick_hand(&mut r, 0, "SHAMBALA-D");
    let _ = r.auto_resolve();
    assert_eq!(trash_ids(&r, 0), vec!["SHAMBALA-D".to_string()]);
    assert_eq!(r.hand_size(0), 3, "PLAIN + Draw 2");
}

#[test]
fn bt26_104_on_play_declined_does_not_draw() {
    let mut r = setup();
    push_hand(&mut r, 0, "SHAMBALA-D");
    let k = r.place_on_field(0, CARD_ID, Some(0));
    fire(&mut r, EffectTiming::OnPlay, k);
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(r.hand_size(0), 1);
    assert_eq!(r.trash_size(0), 0);
}

#[test]
fn bt26_104_on_play_needs_a_shambala_card() {
    let mut r = setup();
    push_hand(&mut r, 0, "PLAIN");
    let k = r.place_on_field(0, CARD_ID, Some(0));
    fire(&mut r, EffectTiming::OnPlay, k);
    let _ = r.auto_resolve();
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.hand_size(0), 1);
}

#[test]
fn bt26_104_eot_suspends_and_uses_shambala_option_free() {
    let mut r = setup();
    push_hand(&mut r, 0, "TB-OPT");
    push_hand(&mut r, 0, "SH-OPT");
    let k = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(0, "TENTEI", Some(0));
    let mem = r.memory();
    fire(&mut r, EffectTiming::EndOfYourTurn, k);
    assert!(r.pending_is_optional());
    r.accept_optional_trigger().expect("accept EOT");
    assert!(
        r.game.players[0].battle_area[k.index as usize].is_suspended,
        "suspend cost paid"
    );
    let v = r.pending_selection_view().expect("option pick");
    assert!(v.is_optional);
    assert_eq!(v.valid_action_ids.len(), 1, "only the Shambala Option");
    pick_hand(&mut r, 0, "SH-OPT");
    let _ = r.auto_resolve();
    assert!(trash_ids(&r, 0).contains(&"SH-OPT".to_string()));
    assert_eq!(hand_ids(&r, 0), vec!["TB-OPT".to_string()]);
    assert_eq!(r.memory(), mem, "used without paying the cost");
}

#[test]
fn bt26_104_eot_requires_tentei_hachibushu_digimon() {
    let mut r = setup();
    push_hand(&mut r, 0, "SH-OPT");
    let k = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(0, "PLAIN", Some(0));
    fire(&mut r, EffectTiming::EndOfYourTurn, k);
    let _ = r.auto_resolve();
    assert!(r.game.pending_selection.is_none());
    assert!(!r.game.players[0].battle_area[k.index as usize].is_suspended);
    assert_eq!(r.hand_size(0), 1);
}

#[test]
fn bt26_104_eot_declinable() {
    let mut r = setup();
    push_hand(&mut r, 0, "SH-OPT");
    let k = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(0, "TENTEI", Some(0));
    fire(&mut r, EffectTiming::EndOfYourTurn, k);
    r.decline_optional_trigger().expect("decline");
    let _ = r.auto_resolve();
    assert!(!r.game.players[0].battle_area[k.index as usize].is_suspended);
    assert_eq!(r.hand_size(0), 1);
}
