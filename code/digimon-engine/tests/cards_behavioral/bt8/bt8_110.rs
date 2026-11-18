//! BT8-110 Armor Texture! — Option, cost 0, white.
//!
//! Printed text (official Bandai DB bundle + DCGO BT8_110.cs):
//! - While you have an [Armor Form] trait Digimon, you can ignore this card's
//!   color requirements.
//! - [Main] Trash the top card of 1 of your Digimon with the [Armor Form] trait.
//!   Then, you may digivolve 1 of your Digimon into a Digimon card in your hand
//!   with the [Armor Form] trait for its digivolution cost. If you do, unsuspend
//!   that Digimon.
//! - Security Effect [Security] You may play 1 level 3 Digimon card with the
//!   [Free] trait from your hand or trash without paying its memory cost.
//!
//! Synthetic boards only (DCGO replay hunt round 2, cluster reveal-bucket-cards).
#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use crate::reveal_bucket_support::*;
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::selection::OptionPlayResult;

fn armor_form(id: &str, level: u8) -> CardData {
    let mut c = digimon(id, "Armor Digimon", level, 5000, 5, vec![CardColor::Blue], &["Armor Form"]);
    if level == 4 {
        c = with_evo(c, CardColor::Blue, 3, 3);
    }
    c
}

fn runner(hand: &[&str]) -> DebugRunner {
    let r = DebugRunner::builder()
        .dsl_card("BT8-110")
        .expect("BT8-110 loads")
        .add_card(armor_form("ARMOR-TOP", 4))
        .add_card(armor_form("ARMOR-EVO", 4))
        .add_card(digimon("LOW", "Low Rookie", 3, 3000, 3, vec![CardColor::Blue], &[]))
        .add_card(digimon("PLAIN4", "Plain Four", 4, 4000, 4, vec![CardColor::Blue], &[]))
        .add_card(digimon("FREE3", "Free Rookie", 3, 3000, 3, vec![CardColor::Red], &["Free"]))
        .add_card(digimon("FREE4", "Free Champion", 4, 4000, 4, vec![CardColor::Red], &["Free"]))
        .add_card(digimon("ATK", "Attacker", 4, 5000, 5, vec![CardColor::Red], &[]))
        .add_card(digimon("FILL", "Filler", 3, 1000, 3, vec![CardColor::Red], &[]))
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .hand(0, hand)
        .memory(10)
        .start();
    into_main(r, 0, 10)
}

#[test]
fn bt8_110_ignores_color_requirements_only_while_you_have_an_armor_form_digimon() {
    // No [Armor Form] Digimon (only a plain blue one): a white Option cannot be used.
    let mut r = runner(&["BT8-110"]);
    r.place_on_field(0, "LOW", Some(0));
    assert_eq!(
        r.game.play_option_from_hand(0, 0),
        OptionPlayResult::Invalid,
        "white requirement unmet and no Armor Form Digimon to ignore it"
    );
    // With an [Armor Form] Digimon the color requirement is ignored.
    let mut r = runner(&["BT8-110"]);
    r.place_stack(0, &["LOW", "ARMOR-TOP"]);
    assert_ne!(r.game.play_option_from_hand(0, 0), OptionPlayResult::Invalid);
}

#[test]
fn bt8_110_main_trashes_the_top_card_then_digivolves_for_the_printed_cost_and_unsuspends() {
    let mut r = runner(&["BT8-110", "ARMOR-EVO"]);
    let host = r.place_stack(0, &["LOW", "ARMOR-TOP"]);
    r.game.players[0].battle_area[host.index as usize].is_suspended = true;
    let memory = r.memory();
    let _ = r.game.play_option_from_hand(0, 0);
    // Trash the top card of the single [Armor Form] Digimon (mandatory), then
    // optionally digivolve (base, then the hand card).
    let mut steps = 0;
    while let Some(view) = r.pending_selection_view() {
        steps += 1;
        assert!(steps < 8, "unexpected prompt loop");
        let pick = view.valid_action_ids.iter().copied().find(|a| *a != PASS).unwrap();
        r.execute_action(view.selecting_player, pick).unwrap();
    }
    assert!(trash_ids(&r, 0).contains(&"ARMOR-TOP".to_string()), "the top card was trashed");
    assert!(
        field_ids(&r, 0).contains(&"ARMOR-EVO".to_string()),
        "digivolved into the [Armor Form] card from hand"
    );
    assert_eq!(memory - r.memory(), 3, "for its printed digivolution cost (3)");
    assert!(
        !r.game.players[0].battle_area[host.index as usize].is_suspended,
        "'if you do, unsuspend that Digimon'"
    );
}

#[test]
fn bt8_110_declining_the_digivolve_does_not_unsuspend() {
    let mut r = runner(&["BT8-110", "ARMOR-EVO"]);
    let host = r.place_stack(0, &["LOW", "ARMOR-TOP"]);
    r.game.players[0].battle_area[host.index as usize].is_suspended = true;
    let _ = r.game.play_option_from_hand(0, 0);
    // First prompt(s): resolve the mandatory trash pick; then decline the optional base pick.
    let mut declined = false;
    let mut steps = 0;
    while let Some(view) = r.pending_selection_view() {
        steps += 1;
        assert!(steps < 8);
        if view.is_optional {
            r.execute_action(view.selecting_player, PASS).unwrap();
            declined = true;
        } else {
            let pick = view.valid_action_ids[0];
            r.execute_action(view.selecting_player, pick).unwrap();
        }
    }
    assert!(declined, "the digivolve is a 'you may'");
    assert!(trash_ids(&r, 0).contains(&"ARMOR-TOP".to_string()), "the trash still happened");
    assert!(
        r.game.players[0].battle_area[host.index as usize].is_suspended,
        "no digivolve: still suspended"
    );
}

#[test]
fn bt8_110_cannot_trash_the_last_card_of_a_digimon() {
    // An [Armor Form] Digimon with a single card has no digivolution card to
    // trash, so it is not a legal pick.
    let mut r = runner(&["BT8-110"]);
    r.place_on_field(0, "ARMOR-TOP", Some(0));
    let _ = r.game.play_option_from_hand(0, 0);
    let _ = r.auto_resolve();
    assert!(
        !trash_ids(&r, 0).contains(&"ARMOR-TOP".to_string()),
        "the only card of the Digimon is never trashed"
    );
    assert_eq!(field_ids(&r, 0), vec!["ARMOR-TOP"]);
}

#[test]
fn bt8_110_security_plays_a_level_3_free_trait_digimon_from_hand_or_trash() {
    use digimon_engine::card_source::CardSource;
    let mut r = runner(&[]);
    let attacker = r.place_on_field(0, "ATK", Some(0));
    put_in_security(&mut r, 1, "BT8-110", false);
    r.add_to_hand(1, "FREE4");
    r.inject_trash(1, "FREE3");
    r.attack_player(attacker, 1, false);
    let view = r.pending_selection_view().expect("the [Security] union pick");
    assert!(view.is_optional, "'you may'");
    assert_eq!(
        view.valid_action_ids.iter().filter(|a| **a != PASS).count(),
        1,
        "only the level 3 [Free] Digimon (the trash copy) qualifies; the level 4 does not"
    );
    let pick = view.valid_action_ids.iter().copied().find(|a| *a != PASS).unwrap();
    r.execute_action(view.selecting_player, pick).unwrap();
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 1).contains(&"FREE3".to_string()), "played from the trash");
}
