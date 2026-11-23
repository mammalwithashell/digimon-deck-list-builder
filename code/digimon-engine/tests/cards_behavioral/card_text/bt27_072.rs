//! Source: `data/card_bundles/BT27-072.md` (printed conditions, effects, and official Q&A).
//!
//! ### Standard digivolve cost circles
//!   - Black Lv.5 / cost 3
//! ### Special digivolution condition (DNA / alt-digivolve black text)
//!   [Digivolve] Lv.6 w/[Mamemon] in name w/o [X Antibody] trait: Cost 1
//!
//! ## Printed text (official)
//! ### Effect
//! [When Digivolving] [When Attacking] [Once Per Turn] Activate 1 [On Deletion] effect on 1 Digimon card with [Mamemon] in its name in this Digimon's digivolution cards as this Digimon's effect. [All Turns] All of your Digimon with [Mamemon] in their names gain ＜Alliance＞, ＜Reboot＞ and ＜Blocker＞.
//!
//! ## Official Q&A
//! - No, you can't.

const CARD: &str = "BT27-072";

#[test]
fn attacking_uses_a_mamemon_sources_deletion_effect_without_deleting_the_host() {
    use digimon_engine::action::space::encode_attack;
    let mut r = start(builder(CARD).dsl_card("BT27-065").unwrap());
    let attacker = r.place_stack(0, &["BT27-065", CARD]);
    r.place_on_field(1, "TEXT-3", Some(0));
    r.place_on_field(1, "TEXT-5", Some(0));
    action(&mut r, 0, encode_attack(attacker.index as u16, 14));
    assert_eq!(candidates(&r), ["BT27-065"]);
    r.choose_prompt_card("BT27-065", None).unwrap();
    assert_eq!(candidates(&r), ["TEXT-3"]);
    r.choose_prompt_card("TEXT-3", None).unwrap();
    assert_eq!(ids(&r, &r.game.players[1].trash), ["TEXT-3", "TEXT-FILLER"]);
    assert_eq!(r.battle_area_size(0), 1);
    assert_eq!(r.game.players[0].battle_area[0].card_sources.len(), 2);
    assert!(r.pending_selection().is_none());
}

use super::common::*;
use digimon_engine::{debug_runner::make_test_card, enums::Keyword};

#[test]
fn digivolving_refires_the_mamemon_source_effect_only_once_even_if_it_attacks() {
    use digimon_engine::action::space::{encode_attack, encode_digivolve};
    use digimon_engine::selection::SelectionKind;

    let mut mamemon = digimon("BASE", 6, 11000, false);
    mamemon.card_name = "PrinceMamemon".into();
    let mut r = start(
        builder(CARD)
            .dsl_card("BT27-065")
            .unwrap()
            .add_card(mamemon)
            .hand(0, &[CARD]),
    );
    let base = r.place_stack(0, &["BT27-065", "BASE"]);
    r.place_on_field(1, "TEXT-3", Some(0));
    r.place_on_field(1, "TEXT-5", Some(0));
    action(&mut r, 0, encode_digivolve(0, base.index.into()));
    r.assert_prompt(SelectionKind::Material, false, 0);
    assert_eq!(candidates(&r), ["BT27-065"]);
    r.choose_prompt_card("BT27-065", None).unwrap();
    r.assert_prompt(SelectionKind::OppField, false, 0);
    assert_eq!(candidates(&r), ["TEXT-3"]);
    r.choose_prompt_card("TEXT-3", None).unwrap();
    assert_eq!(r.memory(), 9);
    action(&mut r, 0, encode_attack(base.index.into(), 14));
    assert!(r.pending_selection().is_none());
    assert_eq!(r.battle_area_size(1), 1);
    assert_eq!(ids(&r, &r.game.players[1].trash), ["TEXT-3", "TEXT-FILLER"]);
}

#[test]
fn aura_gives_three_keywords_to_your_mamemon_only_on_both_turns() {
    let mamemon = make_test_card("MAMEMON", "Mamemon");
    let mut r = start(builder(CARD).add_card(mamemon));
    r.place_on_field(0, CARD, Some(0));
    let ally = r.place_on_field(0, "MAMEMON", Some(0));
    let unrelated = r.place_on_field(0, "TEXT-5", Some(0));
    let opponent = r.place_on_field(1, "MAMEMON", Some(0));
    r.game.tick_declarative_effects();
    for _ in 0..2 {
        for keyword in [Keyword::Alliance, Keyword::Reboot, Keyword::Blocker] {
            assert!(r.game.has_keyword(ally, keyword));
            assert!(!r.game.has_keyword(unrelated, keyword));
            assert!(!r.game.has_keyword(opponent, keyword));
        }
        next_turn(&mut r);
    }
}

#[test]
fn attack_cannot_refire_a_mamemon_with_no_printed_deletion_effect() {
    use digimon_engine::action::space::encode_attack;
    let mut r = start(builder(CARD).add_card(make_test_card("MAMEMON", "Mamemon")));
    let attacker = r.place_stack(0, &["MAMEMON", CARD]);
    r.place_on_field(1, "TEXT-3", Some(0));
    action(&mut r, 0, encode_attack(attacker.index as u16, 14));
    assert!(r.pending_selection().is_none());
    assert_eq!(r.battle_area_size(1), 1);
    assert_eq!(r.security_count(1), 4);
}
