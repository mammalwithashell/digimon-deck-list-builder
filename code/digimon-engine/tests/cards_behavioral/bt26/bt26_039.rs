//! BT26-039 Sunflowmon — Lv.4 Green, Vegetation/DATA SQUAD.
//!
//! [On Play] [When Digivolving] If you have 1 or fewer Tamers, you may play 1
//! [Yoshino Fujieda] from your hand without paying the cost.
//! Inherited: [When Attacking] [Once Per Turn] 1 of your opponent's Digimon
//! can't unsuspend until their turn ends.
//!
//! DCGO: BT26/Green/BT26_039.cs.

use super::support::*;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming, ModifierType};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT26-039";

fn setup() -> (DebugRunner, PermanentHandle) {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-039 in embedded DSL pack")
        .add_card(filler("FILLER"))
        .add_card(tamer(
            "YOSHINO",
            "Yoshino Fujieda",
            CardColor::Green,
            &["DATA SQUAD"],
        ))
        .add_card(tamer(
            "OTHER-DS-T",
            "Keenan Crier",
            CardColor::Purple,
            &["DATA SQUAD"],
        ))
        .add_card(digimon("OPP", "Opp", CardColor::Red, 4, 5, &[]))
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 5])
        .memory(3)
        .start();
    r.set_first_player(0);
    let h = r.place_on_field(0, CARD_ID, Some(0));
    (r, h)
}

#[test]
fn bt26_039_plays_yoshino_free() {
    let (mut r, h) = setup();
    push_hand(&mut r, 0, "YOSHINO");
    let mem = r.memory();
    r.fire_on_play(0, h.index as usize);
    let v = r.pending_selection_view().expect("optional Yoshino play");
    assert!(v.is_optional);
    pick_hand(&mut r, 0, "YOSHINO");
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&"YOSHINO".to_string()));
    assert_eq!(r.memory(), mem, "free");
}

#[test]
fn bt26_039_only_yoshino_by_name() {
    let (mut r, h) = setup();
    push_hand(&mut r, 0, "OTHER-DS-T");
    r.fire_on_play(0, h.index as usize);
    let _ = r.auto_resolve();
    assert!(r.game.pending_selection.is_none());
    assert_eq!(
        r.hand_size(0),
        1,
        "a different [DATA SQUAD] Tamer is not playable"
    );
}

#[test]
fn bt26_039_gated_by_two_tamers() {
    let (mut r, h) = setup();
    r.place_on_field(0, "OTHER-DS-T", Some(0));
    r.place_on_field(0, "OTHER-DS-T", Some(0));
    push_hand(&mut r, 0, "YOSHINO");
    r.fire_on_play(0, h.index as usize);
    let _ = r.auto_resolve();
    assert_eq!(r.hand_size(0), 1, "2 Tamers ⇒ no free play");
}

#[test]
fn bt26_039_when_digivolving_also_offers() {
    let (mut r, h) = setup();
    push_hand(&mut r, 0, "YOSHINO");
    fire(&mut r, EffectTiming::WhenDigivolving, h);
    assert!(r.pending_selection_view().is_some());
}

// ─── Hand-pick candidate legality (select-hand playability audit) ────────────
// DCGO BT26_039.cs:33-36 offers only `IsTamer && EqualsCardName("Yoshino
// Fujieda") && CanPlayAsNewPermanent(card, payCost: false, activateClass)`,
// and the effect activates only when such a card is in hand
// (`SharedAdditionalActivateCondition`, :38-40). A Yoshino the play would
// refuse is never offered (CR 4.3 1-3-2).

/// "You can't play Tamers by effects": Yoshino is not offered.
#[test]
fn bt26_039_not_offered_when_tamer_effect_play_is_prohibited() {
    use digimon_engine::enums::Expiry;
    use digimon_engine::modifiers::PlayerModifierEntry;
    let (mut r, h) = setup();
    push_hand(&mut r, 0, "YOSHINO");
    r.game.modifiers.add_player_modifier(
        0,
        PlayerModifierEntry::simple(ModifierType::CannotPlayTamerByEffect, 0, Expiry::Permanent, None, 1),
    );
    r.fire_on_play(0, h.index as usize);
    assert!(
        r.pending_selection_view().is_none(),
        "no playable Yoshino → nothing offered"
    );
    let _ = r.auto_resolve();
    assert_eq!(hand_ids(&r, 0), vec!["YOSHINO".to_string()]);
}

/// A full battle area (DCGO needs an empty frame): Yoshino is not offered.
#[test]
fn bt26_039_not_offered_when_battle_area_full() {
    let (mut r, h) = setup();
    push_hand(&mut r, 0, "YOSHINO");
    r.game.rules.field_slots = r.game.players[0].battle_area.len() as u8;
    fire(&mut r, EffectTiming::WhenDigivolving, h);
    assert!(
        r.pending_selection_view().is_none(),
        "no free slot → nothing offered"
    );
    let _ = r.auto_resolve();
    assert_eq!(hand_ids(&r, 0), vec!["YOSHINO".to_string()]);
}

#[test]
fn bt26_039_inherited_locks_unsuspend() {
    let (mut r, _) = setup();
    let carrier = r.place_stack(0, &[CARD_ID, "FILLER"]);
    let opp = r.place_on_field(1, "OPP", Some(0));
    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    let _ = r.auto_resolve();
    assert!(r.modifiers().has(opp, ModifierType::CannotUnsuspend));
}
