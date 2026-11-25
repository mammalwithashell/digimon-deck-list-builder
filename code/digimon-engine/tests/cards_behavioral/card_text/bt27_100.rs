//! Source: `data/card_bundles/BT27-100.md` (printed conditions, effects, and official Q&A).
//!
//! ### Standard digivolve cost circles
//!   - (none — DigiEgg / Lv.2 base, or DNA/special only)
//!
//! ## Printed text (official)
//! ### Effect
//! [Main] Give 1 of your opponent's Digimon "[Start of Your Main Phase] This Digimon attacks." until their turn ends. Then, place this card in the battle area. [All Turns] When any of your Digimon with [Craniamon] in their names suspend, ＜Delay＞. ・＜De-Digivolve 1＞ all of your opponent's Digimon. Then, delete all of their Digimon with the lowest play cost.
//!
//! ### Security Effect
//! [Security] Delete all of your opponent's Digimon with the lowest play cost. Then, add this card to the hand.
//!
//! ## Official Q&A
//! - You can give that effect to such a Digimon, but if it isn't affected by effects upon the trigger timing, the gained effect won't trigger.

const CARD: &str = "BT27-100";

#[test]
fn main_places_the_option_and_makes_the_selected_digimon_attack_at_its_next_main_phase() {
    use digimon_engine::action::space::PLAY_HAND_START;
    let mut r = start(builder(CARD).hand(0, &[CARD]));
    r.place_on_field(0, "TEXT-4", Some(0));
    let target = r.place_on_field(1, "TEXT-5", Some(0));
    action(&mut r, 0, PLAY_HAND_START);
    assert_eq!(candidates(&r), ["TEXT-5"]);
    r.choose_prompt_card("TEXT-5", None).unwrap();
    assert_eq!(
        r.game.players[0].battle_area[1]
            .top_card()
            .card_id(&r.game.card_data),
        CARD
    );
    next_turn(&mut r);
    let attack = r
        .pending_selection_view()
        .expect("Choose forced attack target");
    assert_eq!(attack.selecting_player, 1);
    assert!(!attack.is_optional);
    action(&mut r, 1, encode_attack(target.index as u16, 14));
    assert_eq!(r.security_count(0), 4);
    assert!(r.game.players[1].battle_area[0].is_suspended);
}

use super::common::*;
use digimon_engine::action::space::encode_attack;

#[test]
fn a_digimon_unaffected_by_opponent_options_can_be_chosen_but_is_not_forced_to_attack() {
    use digimon_engine::action::space::PLAY_HAND_START;
    use digimon_engine::enums::{EffectSourceKind, Expiry};
    use digimon_engine::modifiers::ModifierEntry;

    let mut r = start(builder(CARD).hand(0, &[CARD]));
    r.place_on_field(0, "TEXT-4", Some(0));
    let target = r.place_on_field(1, "TEXT-5", Some(0));
    r.game.modifiers.add(
        target,
        ModifierEntry::cannot_be_affected_by_opponents_source_kind(
            EffectSourceKind::Option,
            Expiry::Permanent,
            1,
        ),
    );
    action(&mut r, 0, PLAY_HAND_START);
    assert_eq!(candidates(&r), ["TEXT-5"]);
    r.choose_prompt_card("TEXT-5", None).unwrap();
    assert_eq!(ids(&r, &r.game.players[0].battle_area[1].card_sources), [CARD]);

    next_turn(&mut r);
    assert!(r.pending_selection().is_none());
    assert_eq!(r.security_count(0), 5);
    assert!(!r.game.players[1].battle_area[0].is_suspended);
}

#[test]
fn delay_from_craniamon_suspending_de_digivolves_before_deleting_the_lowest_cost() {
    use digimon_engine::action::space::{PASS, PLAY_HAND_START};
    use digimon_engine::debug_runner::make_test_card;
    let mut cheap = digimon("CHEAP", 3, 2000, false);
    cheap.play_cost = 3;
    let mut costly = digimon("COSTLY", 5, 7000, false);
    costly.play_cost = 7;
    let mut high = digimon("HIGH", 6, 12000, false);
    high.play_cost = 10;
    let mut r = start(
        builder(CARD)
            .add_card(make_test_card("CRANIA", "Craniamon"))
            .add_card(cheap)
            .add_card(costly)
            .add_card(high)
            .hand(0, &[CARD]),
    );
    r.place_on_field(0, "TEXT-4", Some(0));
    let crania = r.place_on_field(0, "CRANIA", Some(0));
    r.place_stack(1, &["CHEAP", "COSTLY"]);
    let force = r.place_on_field(1, "HIGH", Some(0));
    action(&mut r, 0, PLAY_HAND_START);
    assert_eq!(candidates(&r), ["COSTLY", "HIGH"]);
    r.choose_prompt_card("HIGH", None).unwrap();
    next_turn(&mut r);
    action(&mut r, 1, encode_attack(force.index as u16, 14));
    action(&mut r, 1, PASS);
    if r.current_phase() == digimon_engine::enums::GamePhase::Breeding {
        action(&mut r, 0, PASS);
    }
    action(&mut r, 0, encode_attack(crania.index as u16, 14));
    r.accept_optional_trigger().unwrap();
    assert_eq!(&ids(&r, &r.game.players[1].trash)[..2], ["COSTLY", "CHEAP"]);
    assert!(ids(&r, &r.game.players[0].trash).contains(&CARD.to_string()));
    assert_eq!(r.battle_area_size(1), 1);
}

#[test]
fn security_deletes_all_tied_lowest_play_cost_digimon_and_returns_to_hand() {
    let mut cheap = digimon("CHEAP", 3, 2000, false);
    cheap.play_cost = 3;
    let mut costly = digimon("COSTLY", 5, 7000, false);
    costly.play_cost = 7;
    let mut r = start(
        builder(CARD)
            .add_card(cheap)
            .add_card(costly)
            .security(0, &[CARD]),
    );
    let attacker = r.place_on_field(1, "CHEAP", Some(0));
    r.place_on_field(1, "CHEAP", Some(0));
    r.place_on_field(1, "COSTLY", Some(0));
    next_turn(&mut r);
    action(&mut r, 1, encode_attack(attacker.index as u16, 14));
    assert!(r.pending_selection().is_none());
    assert_eq!(ids(&r, &r.game.players[0].hand), [CARD]);
    assert_eq!(ids(&r, &r.game.players[1].trash), ["CHEAP", "CHEAP"]);
    assert_eq!(r.battle_area_size(1), 1);
}
