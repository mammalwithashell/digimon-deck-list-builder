//! Source: `data/card_bundles/BT27-026.md` (printed text and official Q&A).
//!
//! Digivolution requirements (AUTHORITATIVE — official Bandai DB)
//! Standard digivolve cost circles
//! - Blue Lv.3 / cost 3
//! - Purple Lv.3 / cost 3
//! Special digivolution condition (DNA / alt-digivolve black text)
//! [Digivolve] Tamer w/[Kurata's Army] trait: Cost 2
//!
//! Printed text (official)
//! Effect
//! ＜Armor Purge＞ [On Play] [When Digivolving] [When Attacking] [Once Per Turn] You may trash any 2 digivolution cards from your opponent's Digimon. Then, by choosing 1 of your Digimon, delete the chosen Digimon and 1 of their Digimon can't suspend until their turn ends.
//!
//! Official Q&A
//! - No. Digivolve from the Tamer as such, and do not treat it as if it is a digivolving Digimon. "When digivolving" effects and "when a Digimon digivolves" effects don't trigger. Additionally, if a "Digimon can't digivolve" effect has activated, you can still digivolve from a Tamer with this digivolve requirement.

use super::bt27_common::*;
use digimon_engine::action::space::PLAY_HAND_START;
use digimon_engine::enums::{CardColor, Keyword};
use digimon_engine::selection::SelectionKind;

#[test]
fn armor_purge_can_save_the_base_or_be_declined() {
    armor_purge_in_battle("BT27-026");
}

#[test]
fn both_printed_standard_digivolution_routes_grant_keywords() {
    standard_routes("BT27-026", CardColor::Blue, 3, 3, &[Keyword::ArmorPurge]);
}

#[test]
#[ignore = "G-ENGINE-TAMER-BASE-AS-TAMER"]
fn tamer_evolution_costs_two_and_does_not_trigger() {
    tamer_evolution("BT27-026", 2, &[Keyword::ArmorPurge]);
}

#[test]
fn may_decline_choosing_a_digimon_to_delete_without_opposing_sources() {
    let mut r = start(builder("BT27-026").hand(0, &["BT27-026"]));
    r.place_on_field(1, HIGH, Some(0));
    action(&mut r, 0, PLAY_HAND_START);
    r.assert_prompt(SelectionKind::OwnField, true, 0);
    assert_eq!(candidates(&r), ["BT27-026"]);
    r.decline_prompt().unwrap();
    assert_eq!(r.game.players[0].battle_area.len(), 1);
    assert_eq!(r.game.players[1].battle_area.len(), 1);
    assert!(r.pending_selection().is_none());
}

#[test]
fn may_trash_two_sources_across_digimon_and_delete_an_ally_to_lock_an_enemy() {
    use digimon_engine::action::space::{encode_attack, SECURITY_TARGET};
    use digimon_engine::enums::ModifierType;
    const CARD: &str = "BT27-026";
    for trash_sources in [false, true] {
        let mut r = start(builder(CARD).hand(0, &[CARD]));
        r.place_on_field(0, EVOLUTION, Some(0));
        let enemy = r.place_stack(1, &[EVOLUTION, HIGH]);
        r.place_stack(1, &[BOUNDARY, BOUNDARY]);
        action(&mut r, 0, PLAY_HAND_START);
        r.assert_prompt(SelectionKind::EffectChoice, false, 0);
        r.execute_branch(usize::from(!trash_sources)).unwrap();
        if trash_sources {
            for entry in [BOUNDARY, EVOLUTION] {
                let kind = r.pending_selection().unwrap().kind.clone();
                assert!(matches!(kind, SelectionKind::SourceMulti { .. }));
                r.assert_prompt(kind, false, 0);
                r.choose_prompt_card(entry, None).unwrap();
            }
            assert_eq!(ids(&r, &r.game.players[1].trash), [BOUNDARY, EVOLUTION]);
        }
        r.assert_prompt(SelectionKind::OwnField, true, 0);
        assert_eq!(candidates(&r), [CARD, EVOLUTION]);
        r.choose_prompt_card(EVOLUTION, None).unwrap();
        r.assert_prompt(SelectionKind::OppField, false, 0);
        assert_eq!(candidates(&r), [BOUNDARY, HIGH]);
        r.choose_prompt_card(HIGH, None).unwrap();
        assert_eq!(ids(&r, &r.game.players[0].trash), [EVOLUTION]);
        assert!(r.game.modifiers.has(enemy, ModifierType::CannotSuspend));
        next_turn(&mut r);
        assert!(r.game.modifiers.has(enemy, ModifierType::CannotSuspend));
        assert_eq!(
            digimon_engine::build_action_mask(&r.game, 1)
                [encode_attack(enemy.index as u16, SECURITY_TARGET) as usize],
            0.0
        );
        next_turn(&mut r);
        assert!(!r.game.modifiers.has(enemy, ModifierType::CannotSuspend));
    }
}

#[test]
fn declining_the_digivolution_effect_leaves_the_attack_trigger_available() {
    let (mut r, host) = evolve_against_an_enemy("BT27-026");
    r.assert_prompt(SelectionKind::OwnField, true, 0);
    r.decline_prompt().unwrap();
    attack_security(&mut r, host);
    r.assert_prompt(SelectionKind::OwnField, true, 0);
    r.decline_prompt().unwrap();
    assert_eq!(r.security_count(1), 4);
    assert!(r.pending_selection().is_none());
}
