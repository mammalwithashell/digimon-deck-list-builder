//! Source: `data/card_bundles/BT27-065.md` (printed conditions, effects, and official Q&A).
//!
//! ### Standard digivolve cost circles
//!   - Black Lv.4 / cost 4
//!   - Red Lv.4 / cost 4
//! ### Special digivolution condition (DNA / alt-digivolve black text)
//!   [Digivolve] Lv.5 w/[Mamemon] in name w/o [X Antibody] trait: Cost 1
//!
//! ## Printed text (official)
//! ### Effect
//! [When Digivolving] If a card with [Mamemon] in its name is in this Digimon's digivolution cards, 1 of your Digimon with [Mamemon] in its name may gain ＜Rush＞ for the turn and attack. At the end of that attack, delete it. [On Deletion] Delete 1 of your opponent's lowest DP Digimon. If this effect didn't delete, trash their top security card.
//!
//! ### Inherited Effect
//! [All Turns] [Once Per Turn] When any of your other Digimon with [Mamemon] in their names are deleted, delete 1 of your opponent's Digimon with the lowest play cost.
//!
//! ## Official Q&A
//! - No. The Digimon given by this effect must attack if possible.

const CARD: &str = "BT27-065";

use super::common::*;
use digimon_engine::action::space::encode_attack;
use digimon_engine::selection::SelectionKind;

#[test]
fn deletion_in_battle_trashes_security_if_no_opponent_digimon_exists() {
    let mut security = digimon("SECURITY", 6, 12000, false);
    security.play_cost = 12;
    let mut r = start(
        builder(CARD)
            .add_card(security)
            .security(1, &["SECURITY", "SECURITY"]),
    );
    let attacker = r.place_on_field(0, CARD, Some(0));
    action(&mut r, 0, encode_attack(attacker.index as u16, 14));
    assert!(r.pending_selection().is_none());
    assert_eq!(r.battle_area_size(0), 0);
    assert_eq!(r.security_count(1), 0);
}

#[test]
fn deletion_in_battle_deletes_an_opponents_lowest_dp_digimon_instead_of_trashing_security() {
    use digimon_engine::selection::SelectionKind;

    let mut r = start(
        builder(CARD)
            .add_card(digimon("FATAL", 7, 20000, false))
            .security(1, &["TEXT-FILLER", "FATAL"]),
    );
    let attacker = r.place_on_field(0, CARD, Some(0));
    r.place_on_field(1, "TEXT-4", Some(0));
    r.place_on_field(1, "TEXT-3", Some(0));
    action(&mut r, 0, encode_attack(attacker.index.into(), 14));
    r.assert_prompt(SelectionKind::OppField, false, 0);
    assert_eq!(candidates(&r), ["TEXT-3"]);
    r.choose_prompt_card("TEXT-3", None).unwrap();
    assert_eq!(ids(&r, &r.game.players[0].trash), [CARD]);
    let mut trash = ids(&r, &r.game.players[1].trash);
    trash.sort_unstable();
    assert_eq!(trash, ["FATAL", "TEXT-3"]);
    assert_eq!(r.security_count(1), 1);
}

#[test]
fn mamemon_source_allows_a_new_ally_to_rush_and_forces_the_attack_after_the_pick() {
    use digimon_engine::action::space::{encode_digivolve, PASS};
    use digimon_engine::enums::Keyword;

    for (named_source, choose_attacker) in [(false, false), (true, false), (true, true)] {
        let mut base = digimon("BASE", if named_source { 5 } else { 4 }, 4000, false);
        base.colors = vec![digimon_engine::enums::CardColor::Black];
        if named_source {
            base.card_name = "Mamemon".into();
        }
        let mut ally = digimon("ALLY", 4, 4000, false);
        ally.card_name = "BigMamemon".into();
        let card_data_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/cards.json");
        let real_card = digimon_engine::CardData::load_from_file(&card_data_path)
            .unwrap().remove(CARD).unwrap();
        let mut r = start(builder(CARD).add_card(real_card).add_card(base).add_card(ally).hand(0, &[CARD]));
        let base = r.place_on_field(0, "BASE", Some(0));
        let ally = r.place_on_field(0, "ALLY", Some(r.game.turn_count));
        action(&mut r, 0, encode_digivolve(0, base.index as u16));
        if named_source {
            assert_eq!(candidates(&r), ["ALLY", CARD]);
            if choose_attacker {
                r.choose_prompt_card("ALLY", None).unwrap();
                assert!(r.game.has_keyword(ally, Keyword::Rush));
                assert_eq!(digimon_engine::build_action_mask(&r.game, 0)[PASS as usize], 0.0);
                action(&mut r, 0, encode_attack(ally.index as u16, 14));
            } else {
                r.decline_prompt().unwrap();
            }
        }
        assert!(r.pending_selection().is_none());
        assert_eq!(r.battle_area_size(0), if choose_attacker { 1 } else { 2 });
        assert_eq!(r.security_count(1), if choose_attacker { 4 } else { 5 });
        if choose_attacker {
            assert_eq!(ids(&r, &r.game.players[0].trash), ["ALLY"]);
        }
    }
}

#[test]
fn inherited_deleting_another_mamemon_targets_lowest_play_cost_not_lowest_dp() {
    let mut ally = digimon("ALLY", 4, 4000, false);
    ally.card_name = "BigMamemon".into();
    let mut cheap = digimon("CHEAP", 6, 14000, false);
    cheap.play_cost = 2;
    let mut r = start(
        builder(CARD)
            .add_card(ally)
            .add_card(cheap)
            .add_card(digimon("SECURITY", 7, 17000, false))
            .security(1, &["SECURITY"; 3]),
    );
    r.place_stack(0, &[CARD, "TEXT-6"]);
    let unrelated = r.place_on_field(0, "TEXT-4", Some(0));
    action(&mut r, 0, encode_attack(unrelated.index as u16, 14));
    assert!(r.pending_selection().is_none());
    assert_eq!(r.battle_area_size(0), 1);
    assert_eq!(r.security_count(1), 2);
    let mamemon = r.place_on_field(0, "ALLY", Some(0));
    r.place_on_field(1, "TEXT-3", Some(0));
    r.place_on_field(1, "CHEAP", Some(0));
    action(&mut r, 0, encode_attack(mamemon.index as u16, 14));
    r.assert_prompt(SelectionKind::OppField, false, 0);
    assert_eq!(candidates(&r), ["CHEAP"]);
    r.choose_prompt_card("CHEAP", None).unwrap();
    let trash = ids(&r, &r.game.players[1].trash);
    assert_eq!(trash.iter().filter(|id| id.as_str() == "CHEAP").count(), 1);
    let second = r.place_on_field(0, "ALLY", Some(0));
    action(&mut r, 0, encode_attack(second.index as u16, 14));
    assert!(r.pending_selection().is_none());
    assert_eq!(r.battle_area_size(0), 1);
    assert_eq!(r.security_count(1), 0);
    assert_eq!(r.battle_area_size(1), 1);
}
