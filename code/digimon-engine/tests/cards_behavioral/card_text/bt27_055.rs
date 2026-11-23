//! Source: `data/card_bundles/BT27-055.md` (printed text and official Q&A).
//!
//! Digivolution requirements (AUTHORITATIVE — official Bandai DB)
//! Standard digivolve cost circles
//! - Green Lv.5 / cost 4
//! - Purple Lv.5 / cost 4
//! Special digivolution condition (DNA / alt-digivolve black text)
//! [Digivolve] [Quetzalmon] w/[Bio] trait: Cost 5 [Digivolve] Tamer w/[Kurata's Army] trait: Cost 7
//!
//! Printed text (official)
//! Effect
//! ＜Piercing＞ ＜Vortex＞ ＜Armor Purge＞ [When Digivolving] [When Attacking] [Once Per Turn] Suspend up to 2 of your opponent's Digimon or Tamers. Then, 1 of their Digimon or Tamers can't unsuspend until their turn ends. [All Turns] [Once Per Turn] When any Digimon suspend, this Digimon may unsuspend. [All Turns] This Digimon can't digivolve.
//!
//! Official Q&A
//! - No. Digivolve from the Tamer as such, and do not treat it as if it is a digivolving Digimon. "When digivolving" effects and "when a Digimon digivolves" effects don't trigger. Additionally, if a "Digimon can't digivolve" effect has activated, you can still digivolve from a Tamer with this digivolve requirement.

use super::bt27_common::*;
use digimon_engine::action::space::{encode_attack, encode_digivolve, PASS, SECURITY_TARGET};
use digimon_engine::debug_runner::make_test_card;
use digimon_engine::enums::{CardColor, Keyword};
use digimon_engine::selection::SelectionKind;

#[test]
fn armor_purge_can_save_the_base_or_be_declined() {
    armor_purge_in_battle("BT27-055");
}

#[test]
fn both_printed_standard_digivolution_routes_grant_keywords() {
    standard_routes(
        "BT27-055",
        CardColor::Green,
        5,
        4,
        &[Keyword::ArmorPurge, Keyword::Piercing, Keyword::Vortex],
    );
}

#[test]
#[ignore = "G-ENGINE-TAMER-BASE-AS-TAMER"]
fn tamer_evolution_does_not_trigger_when_digivolving() {
    tamer_evolution(
        "BT27-055",
        7,
        &[Keyword::Piercing, Keyword::Vortex, Keyword::ArmorPurge],
    );
}

#[test]
fn bio_quetzalmon_evolves_for_five_and_locks_the_chosen_tamer() {
    let mut quetzalmon = make_test_card("TEXT-QUETZALMON", "Quetzalmon");
    quetzalmon.level = Some(4);
    quetzalmon.traits = vec!["Bio".into()];
    let mut r = start(
        builder("BT27-055")
            .add_card(quetzalmon)
            .hand(0, &["BT27-055"]),
    );
    let base = r.place_on_field(0, "TEXT-QUETZALMON", Some(0));
    r.place_on_field(1, TAMER, Some(0));
    let before = r.memory();
    action(&mut r, 0, encode_digivolve(0, base.index as u16));
    assert_eq!(before - r.memory(), 5);
    r.assert_prompt(SelectionKind::OppField, true, 0);
    assert_eq!(candidates(&r), [TAMER]);
    r.decline_prompt().unwrap();
    r.assert_prompt(SelectionKind::OppField, false, 0);
    assert_eq!(candidates(&r), [TAMER]);
    r.choose_prompt_card(TAMER, None).unwrap();
    assert!(r.game.modifiers.has(
        digimon_engine::permanent::PermanentHandle {
            player: 1,
            index: 0
        },
        digimon_engine::enums::ModifierType::CannotUnsuspend,
    ));
    assert!(r.pending_selection().is_none());
}

#[test]
fn digivolving_can_suspend_two_different_permanents_and_lock_one() {
    use digimon_engine::enums::ModifierType;
    const CARD: &str = "BT27-055";
    let mut base = make_test_card("TEXT-LVL5", "Green level 5");
    base.level = Some(5);
    base.colors = vec![CardColor::Green];
    let mut r = start(builder(CARD).add_card(base).hand(0, &[CARD]));
    let host = r.place_on_field(0, "TEXT-LVL5", Some(0));
    let enemy = r.place_on_field(1, HIGH, Some(0));
    let tamer = r.place_on_field(1, TAMER, Some(0));
    action(&mut r, 0, encode_digivolve(0, host.index as u16));
    for (id, offered) in [(HIGH, &[HIGH, TAMER][..]), (TAMER, &[TAMER][..])] {
        r.assert_prompt(SelectionKind::OppField, true, 0);
        assert_eq!(candidates(&r), offered);
        r.choose_prompt_card(id, None).unwrap();
    }
    r.assert_prompt(SelectionKind::OppField, false, 0);
    r.choose_prompt_card(HIGH, None).unwrap();
    assert!(r.game.players[1].battle_area.iter().all(|p| p.is_suspended));
    assert!(r.game.modifiers.has(enemy, ModifierType::CannotUnsuspend));
    assert_cannot_digivolve(&r, host);
    action(&mut r, 0, PASS);
    assert!(r.pending_is_end_of_turn_keyword_decision());
    action(&mut r, 0, PASS);
    action(&mut r, 1, PASS);
    assert!(r.game.players[1].battle_area[enemy.index as usize].is_suspended);
    assert!(!r.game.players[1].battle_area[tamer.index as usize].is_suspended);
    assert!(r.pending_selection().is_none());
}

#[test]
fn vortex_can_be_declined_or_attack_a_suspended_digimon_and_trigger_piercing() {
    const CARD: &str = "BT27-055";
    for accept in [false, true] {
        let mut base = make_test_card("TEXT-LVL5", "Green level 5");
        base.level = Some(5);
        base.colors = vec![CardColor::Green];
        let mut weak = make_test_card("TEXT-WEAK", "Weaker opponent");
        weak.dp = Some(3000);
        let mut r = start(builder(CARD).add_card(base).add_card(weak).hand(0, &[CARD]));
        let host = r.place_on_field(0, "TEXT-LVL5", Some(0));
        let enemy = r.place_on_field(1, "TEXT-WEAK", Some(0));
        action(&mut r, 0, encode_digivolve(0, host.index as u16));
        r.assert_prompt(SelectionKind::OppField, true, 0);
        r.choose_prompt_card("TEXT-WEAK", None).unwrap();
        r.assert_prompt(SelectionKind::OppField, false, 0);
        r.choose_prompt_card("TEXT-WEAK", None).unwrap();
        action(&mut r, 0, PASS);
        assert!(r.pending_is_end_of_turn_keyword_decision());
        assert!(!r
            .pending_selection()
            .unwrap()
            .valid_action_ids
            .contains(&encode_attack(host.index as u16, SECURITY_TARGET,)));
        action(
            &mut r,
            0,
            if accept {
                encode_attack(host.index as u16, enemy.index as u16)
            } else {
                PASS
            },
        );
        if accept {
            r.assert_prompt(SelectionKind::Replacement, true, 0);
            r.decline_optional_trigger().unwrap();
        }
        assert_eq!(r.game.players[1].battle_area.is_empty(), accept);
        assert_eq!(r.security_count(1), if accept { 4 } else { 5 });
        assert!(r.pending_selection().is_none());
    }
}

#[test]
fn another_digimons_suspension_can_unsuspend_lotosmon_once_per_turn() {
    for accept in [false, true] {
        let mut r = start(builder("BT27-055"));
        let host = r.place_on_field(0, "BT27-055", Some(0));
        let ally = r.place_on_field(0, HIGH, Some(0));
        let second = r.place_on_field(0, BOUNDARY, Some(0));
        r.game.players[0].battle_area[host.index as usize].is_suspended = true;
        action(&mut r, 0, encode_attack(ally.index as u16, SECURITY_TARGET));
        r.assert_prompt(SelectionKind::Replacement, true, 0);
        if accept {
            r.accept_optional_trigger().unwrap();
            assert!(!r.game.players[0].battle_area[host.index as usize].is_suspended);
            r.game.players[0].battle_area[host.index as usize].is_suspended = true;
        } else {
            r.decline_optional_trigger().unwrap();
            assert!(r.game.players[0].battle_area[host.index as usize].is_suspended);
        }
        action(
            &mut r,
            0,
            encode_attack(second.index as u16, SECURITY_TARGET),
        );
        if !accept {
            r.assert_prompt(SelectionKind::Replacement, true, 0);
            r.decline_optional_trigger().unwrap();
        }
        assert!(r.game.players[0].battle_area[host.index as usize].is_suspended);
        assert!(r.pending_selection().is_none());
    }
}

#[test]
fn when_attacking_can_lock_an_enemy_even_when_the_optional_suspension_is_declined() {
    let mut r = start(builder("BT27-055"));
    let host = r.place_on_field(0, "BT27-055", Some(0));
    let enemy = r.place_on_field(1, HIGH, Some(0));
    attack_security(&mut r, host);
    let mut saw_lock = false;
    let mut saw_unsuspend = false;
    while let Some(kind) = r.pending_selection().map(|p| p.kind.clone()) {
        match kind {
            SelectionKind::TriggerOrder | SelectionKind::EffectChoice
                if !saw_lock && !saw_unsuspend =>
            {
                r.assert_prompt(kind, false, 0);
                let pending = r.pending_selection().unwrap();
                let choices = pending.effect_choices.as_ref().unwrap();
                assert_eq!(choices.len(), 2);
                let unsuspend = choices
                    .iter()
                    .position(|c| c.timing == Some(digimon_engine::enums::EffectTiming::OnSuspend))
                    .unwrap();
                r.execute_branch(unsuspend).unwrap();
            }
            SelectionKind::Replacement if !saw_unsuspend => {
                r.assert_prompt(kind, true, 0);
                r.decline_optional_trigger().unwrap();
                saw_unsuspend = true;
            }
            SelectionKind::OppField if !saw_lock => {
                r.assert_prompt(kind, true, 0);
                assert_eq!(candidates(&r), [HIGH]);
                r.decline_prompt().unwrap();
                r.assert_prompt(SelectionKind::OppField, false, 0);
                r.choose_prompt_card(HIGH, None).unwrap();
                saw_lock = true;
            }
            _ => panic!("unexpected prompt: {:?}", r.pending_selection_view()),
        }
    }
    assert!(saw_lock && saw_unsuspend);
    assert!(r
        .game
        .modifiers
        .has(enemy, digimon_engine::enums::ModifierType::CannotUnsuspend));
    assert_eq!(r.security_count(1), 4);
}
