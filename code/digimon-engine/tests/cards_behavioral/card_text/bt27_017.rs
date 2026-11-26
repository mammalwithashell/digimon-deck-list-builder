//! Source: `data/card_bundles/BT27-017.md` (printed text and official Q&A).
//!
//! Digivolution requirements (AUTHORITATIVE — official Bandai DB)
//! Standard digivolve cost circles
//! - Red Lv.5 / cost 4
//! - Purple Lv.5 / cost 4
//! Special digivolution condition (DNA / alt-digivolve black text)
//! [Digivolve] [Stegomon] w/[Bio] trait: Cost 5 [Digivolve] Tamer w/[Kurata's Army] trait: Cost 7
//!
//! Printed text (official)
//! Effect
//! ＜Security A. +1＞ ＜Armor Purge＞ ＜Engage＞ (At the end of your turn, this Digimon may attack.) [On Play] [When Digivolving] Delete 1 of your opponent's Digimon with as much DP as this Digimon or less. [All Turns] This Digimon can't digivolve. [Opponent's Turn] While this Digimon is suspended, none of your opponent's Digimon can attack players.
//!
//! Official Q&A
//! - No. Digivolve from the Tamer as such, and do not treat it as if it is a digivolving Digimon. "When digivolving" effects and "when a Digimon digivolves" effects don't trigger. Additionally, if a "Digimon can't digivolve" effect has activated, you can still digivolve from a Tamer with this digivolve requirement.

use super::bt27_common::*;
use digimon_engine::action::space::PLAY_HAND_START;
use digimon_engine::enums::{CardColor, Keyword};
use digimon_engine::selection::SelectionKind;

#[test]
fn armor_purge_can_save_the_base_or_be_declined() {
    armor_purge_in_battle("BT27-017");
}

#[test]
fn both_printed_standard_digivolution_routes_grant_keywords() {
    standard_routes(
        "BT27-017",
        CardColor::Red,
        5,
        4,
        &[
            Keyword::ArmorPurge,
            Keyword::Engage,
            Keyword::SecurityAttackPlus(1),
        ],
    );
}

const CARD: &str = "BT27-017";

#[test]
fn on_play_deletes_at_most_its_own_dp() {
    let mut r = start(builder(CARD).hand(0, &[CARD]));
    r.place_on_field(1, BOUNDARY, Some(0));
    r.place_on_field(1, HIGH, Some(0));
    action(&mut r, 0, PLAY_HAND_START);
    r.assert_prompt(SelectionKind::OppField, false, 0);
    assert_eq!(candidates(&r), [BOUNDARY]);
    r.choose_prompt_card(BOUNDARY, None).unwrap();
    assert_eq!(r.game.players[1].battle_area.len(), 1);
    assert_eq!(r.trash_size(1), 1);
    assert!(r.pending_selection().is_none());
}

#[test]
fn bio_stegomon_route_costs_five_and_spinomon_cannot_digivolve() {
    let mut base = digimon_engine::debug_runner::make_test_card("TEXT-BIO-STEGOMON", "Stegomon");
    base.traits = vec!["Bio".into()];
    base.level = Some(4);
    let mut r = start(builder(CARD).add_card(base).hand(0, &[CARD]));
    let host = r.place_on_field(0, "TEXT-BIO-STEGOMON", Some(0));
    r.place_on_field(1, BOUNDARY, Some(0));
    r.place_on_field(1, HIGH, Some(0));
    action(
        &mut r,
        0,
        digimon_engine::action::space::encode_digivolve(0, host.index as u16),
    );
    assert_eq!(r.memory(), 5);
    r.assert_prompt(SelectionKind::OppField, false, 0);
    assert_eq!(candidates(&r), [BOUNDARY]);
    r.choose_prompt_card(BOUNDARY, None).unwrap();
    assert_eq!(ids(&r, &r.game.players[1].trash), [BOUNDARY]);
    assert_cannot_digivolve(&r, host);
}

#[test]
fn engage_can_attack_at_end_of_turn_and_checks_two_security() {
    use digimon_engine::action::space::{encode_attack, PASS, SECURITY_TARGET};
    for accept in [false, true] {
        let mut r = start(builder(CARD));
        let host = r.place_on_field(0, CARD, Some(0));
        action(&mut r, 0, PASS);
        assert!(r.pending_is_end_of_turn_keyword_decision());
        action(
            &mut r,
            0,
            if accept {
                encode_attack(host.index as u16, SECURITY_TARGET)
            } else {
                PASS
            },
        );
        assert_eq!(r.security_count(1), if accept { 3 } else { 5 });
        assert!(r.pending_selection().is_none());
    }
}

#[test]
fn suspended_spinomon_blocks_player_attacks_only_on_the_opponents_turn() {
    use digimon_engine::action::space::{encode_attack, SECURITY_TARGET};
    use digimon_engine::build_action_mask;
    let mut r = start(builder(CARD));
    let host = r.place_on_field(0, CARD, Some(0));
    let enemy = r.place_on_field(1, HIGH, Some(0));
    r.game.players[0].battle_area[host.index as usize].is_suspended = true;
    next_turn(&mut r);
    let mask = build_action_mask(&r.game, 1);
    assert_eq!(
        mask[encode_attack(enemy.index as u16, SECURITY_TARGET) as usize],
        0.0
    );
    assert_eq!(
        mask[encode_attack(enemy.index as u16, host.index as u16) as usize],
        1.0
    );
}

#[test]
fn tamer_evolution_does_not_trigger_when_digivolving() {
    tamer_evolution(
        CARD,
        7,
        &[
            Keyword::ArmorPurge,
            Keyword::Engage,
            Keyword::SecurityAttackPlus(1),
        ],
    );
}
