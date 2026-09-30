//! BT26-091 Yoshino Fujieda — Tamer, Green, DATA SQUAD.
//!
//! [Start of Your Main Phase] By placing 1 [DATA SQUAD] trait card from your
//! hand face down under this Tamer, <Draw 1> and gain 1 memory.
//! [Your Turn] When any of your opponent's Digimon or Tamers suspend, or effects
//! trash cards from under this Tamer, by suspending this Tamer, 1 of your
//! Digimon may digivolve into a [Vegetation], [Fairy] or [DATA SQUAD] trait
//! Digimon card in the hand with the cost reduced by 1.
//! [Security] Play this card without paying the cost.
//!
//! DCGO: BT26/Green/BT26_091.cs. The reactive trigger is driven by real card
//! effects: BT26-036 Lalamon's inherited suspend and ST24-12 Falcomon's
//! "trash the bottom face-down card from under any of your Tamers".

use super::support::*;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT26-091";

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-091")
        .dsl_card("BT26-036")
        .expect("Lalamon")
        .dsl_card("ST24-12")
        .expect("Falcomon")
        .add_card(filler("FILLER"))
        .add_card(digimon("DS-CARD", "DS", CardColor::Green, 3, 3, &["DATA SQUAD"]))
        .add_card(digimon("PLAIN", "Plain", CardColor::Green, 3, 3, &["Beast"]))
        // Green Lv.4 Fairy with a Green Lv.3 / cost 3 circle.
        .add_card({
            let mut c = digimon("FAIRY4", "Lilamon-ish", CardColor::Green, 4, 5, &["Fairy"]);
            c.evo_costs = vec![digimon_engine::card_data::EvoCost {
                card_color: CardColor::Green as u8,
                level: 3,
                memory_cost: 3,
            }];
            c
        })
        .add_card(digimon("BASE3", "Base", CardColor::Green, 3, 3, &[]))
        .add_card(digimon("OPP", "Opp", CardColor::Red, 4, 5, &[]))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(3)
        .start();
    r.set_first_player(0);
    r
}

#[test]
fn bt26_091_somp_places_ds_card_draws_and_gains_memory() {
    let mut r = setup();
    let y = r.place_on_field(0, CARD_ID, Some(0));
    push_hand(&mut r, 0, "DS-CARD");
    let mem = r.memory();
    fire(&mut r, EffectTiming::StartOfYourMainPhase, y);
    let v = r.pending_selection_view().expect("optional hand pick");
    assert!(v.is_optional);
    pick_hand(&mut r, 0, "DS-CARD");
    let _ = r.auto_resolve();
    assert_eq!(sources(&r, y), 1);
    let placed = &r.game.players[0].battle_area[y.index as usize].card_sources[0];
    assert!(placed.face_down);
    assert_eq!(placed.card_id(&r.game.card_data), "DS-CARD");
    assert_eq!(hand_ids(&r, 0), vec!["FILLER".to_string()], "<Draw 1>");
    assert_eq!(r.memory(), mem + 1);
}

#[test]
fn bt26_091_somp_declined_does_nothing() {
    let mut r = setup();
    let y = r.place_on_field(0, CARD_ID, Some(0));
    push_hand(&mut r, 0, "DS-CARD");
    let mem = r.memory();
    fire(&mut r, EffectTiming::StartOfYourMainPhase, y);
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(sources(&r, y), 0);
    assert_eq!(r.hand_size(0), 1);
    assert_eq!(r.memory(), mem);
}

#[test]
fn bt26_091_somp_needs_a_data_squad_card_in_hand() {
    let mut r = setup();
    let y = r.place_on_field(0, CARD_ID, Some(0));
    push_hand(&mut r, 0, "PLAIN");
    fire(&mut r, EffectTiming::StartOfYourMainPhase, y);
    let _ = r.auto_resolve();
    assert!(r.game.pending_selection.is_none());
    assert_eq!(sources(&r, y), 0);
}

fn lalamon_suspends_opponent(r: &mut DebugRunner) -> PermanentHandle {
    let carrier = r.place_stack(0, &["BT26-036", "BASE3"]);
    r.place_on_field(1, "OPP", Some(0));
    fire(r, EffectTiming::WhenAttacking, carrier);
    pick_first(r, 0); // suspend OPP
    carrier
}

#[test]
fn bt26_091_opponent_suspend_digivolves_for_one_less() {
    let mut r = setup();
    let y = r.place_on_field(0, CARD_ID, Some(0));
    push_hand(&mut r, 0, "FAIRY4");
    let base = lalamon_suspends_opponent(&mut r);
    // Yoshino's optional trigger: accept, pay the suspend cost.
    r.accept_optional_trigger().expect("Yoshino triggers on the opponent suspend");
    assert!(r.game.players[0].battle_area[y.index as usize].is_suspended, "suspend cost");
    pick_first(&mut r, 0); // the Digimon to digivolve (only BASE3 stack qualifies)
    pick_hand(&mut r, 0, "FAIRY4");
    let _ = r.auto_resolve();
    let top = r.game.players[0].battle_area[base.index as usize]
        .top_card()
        .card_id(&r.game.card_data)
        .to_string();
    assert_eq!(top, "FAIRY4");
    assert_eq!(r.memory(), 3 - 2, "circle cost 3 reduced by 1");
}

#[test]
fn bt26_091_trigger_is_declinable_and_keeps_tamer_unsuspended() {
    let mut r = setup();
    let y = r.place_on_field(0, CARD_ID, Some(0));
    push_hand(&mut r, 0, "FAIRY4");
    lalamon_suspends_opponent(&mut r);
    r.decline_optional_trigger().expect("decline");
    let _ = r.auto_resolve();
    assert!(!r.game.players[0].battle_area[y.index as usize].is_suspended);
    assert_eq!(r.hand_size(0), 1);
}

#[test]
fn bt26_091_effect_trash_from_under_this_tamer_triggers() {
    let mut r = setup();
    let y = tamer_with_face_down(&mut r, 0, CARD_ID, 1);
    r.place_on_field(0, "BASE3", Some(0));
    push_hand(&mut r, 0, "FAIRY4");
    push_trash(&mut r, 0, "DS-CARD");
    let falco = r.place_on_field(0, "ST24-12", Some(0));
    r.fire_on_play(0, falco.index as usize);
    r.accept_optional_trigger().expect("Falcomon");
    pick_first(&mut r, 0); // Yoshino is the only Tamer with a face-down card
    pass(&mut r, 0); // decline Falcomon's optional trash-return pick
    // Yoshino's optional trigger (it follows Falcomon's resolution).
    r.accept_optional_trigger().expect("Yoshino triggers on the effect trash");
    assert!(r.game.players[0].battle_area[y.index as usize].is_suspended);
    let v = r.pending_selection_view().expect("digivolve base pick");
    assert!(v.is_optional, "'may digivolve'");
    pick_first(&mut r, 0);
    pick_hand(&mut r, 0, "FAIRY4");
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&"FAIRY4".to_string()));
    let _ = y;
}

#[test]
fn bt26_091_not_on_opponents_turn() {
    let mut r = setup();
    let y = r.place_on_field(0, CARD_ID, Some(0));
    push_hand(&mut r, 0, "FAIRY4");
    r.set_first_player(1);
    let _ = lalamon_suspends_opponent(&mut r);
    let _ = r.auto_resolve();
    assert!(!r.game.players[0].battle_area[y.index as usize].is_suspended);
    assert_eq!(r.hand_size(0), 1);
}
