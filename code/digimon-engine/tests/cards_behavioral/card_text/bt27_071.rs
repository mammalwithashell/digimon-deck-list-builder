//! Source: `data/card_bundles/BT27-071.md` (printed text and official Q&A).
//!
//! Digivolution requirements (AUTHORITATIVE — official Bandai DB)
//! Standard digivolve cost circles
//! - Black Lv.5 / cost 4
//! - Purple Lv.5 / cost 4
//! Special digivolution condition (DNA / alt-digivolve black text)
//! [Digivolve] [Thunderbirdmon] w/[Bio] trait: Cost 5 [Digivolve] Tamer w/[Kurata's Army] trait: Cost 7
//!
//! Printed text (official)
//! Effect
//! ＜Reboot＞ ＜Blocker＞ ＜Armor Purge＞ [When Digivolving] Your opponent's Digimon effects don't affect this Digimon until their turn ends. [When Attacking] [Counter] [Once Per Turn] ＜De-Digivolve 2＞ 1 of your opponent's Digimon. [All Turns] This Digimon can't digivolve.
//!
//! Official Q&A
//! - No. Digivolve from the Tamer as such, and do not treat it as if it is a digivolving Digimon. "When digivolving" effects and "when a Digimon digivolves" effects don't trigger. Additionally, if a "Digimon can't digivolve" effect has activated, you can still digivolve from a Tamer with this digivolve requirement.

use super::bt27_common::*;
use digimon_engine::action::space::{encode_attack, SECURITY_TARGET};
use digimon_engine::count_choice::CountChoiceOperation;
use digimon_engine::enums::{CardColor, Keyword};
use digimon_engine::selection::SelectionKind;

fn declare_two(r: &mut digimon_engine::debug_runner::DebugRunner) {
    let pending = r.pending_selection().unwrap();
    assert!(!pending.is_optional);
    let action_id = pending
        .effect_choices
        .as_ref()
        .unwrap()
        .iter()
        .find(|entry| {
            matches!(entry.count_choice.map(|v| v.operation),
            Some(CountChoiceOperation::Commit(v)) if v.declared == 2)
        })
        .unwrap()
        .action_id;
    action(r, 0, action_id);
}

#[test]
fn armor_purge_can_save_the_base_or_be_declined() {
    armor_purge_in_battle("BT27-071");
}

#[test]
fn both_printed_standard_digivolution_routes_grant_keywords() {
    standard_routes(
        "BT27-071",
        CardColor::Black,
        5,
        4,
        &[Keyword::ArmorPurge, Keyword::Reboot, Keyword::Blocker],
    );
}

#[test]
fn tamer_evolution_does_not_trigger_when_digivolving() {
    tamer_evolution(
        "BT27-071",
        7,
        &[Keyword::Reboot, Keyword::Blocker, Keyword::ArmorPurge],
    );
}

#[test]
fn when_attacking_de_digivolves_an_opposing_stack_to_level_three() {
    let mut r = start(builder("BT27-071"));
    let attacker = r.place_on_field(0, "BT27-071", Some(0));
    let opponent = r.place_stack(1, &[BOUNDARY, EVOLUTION, EVOLUTION]);
    action(
        &mut r,
        0,
        encode_attack(attacker.index as u16, SECURITY_TARGET),
    );
    r.assert_prompt(SelectionKind::OppField, false, 0);
    assert_eq!(candidates(&r), [EVOLUTION]);
    r.choose_prompt_card(EVOLUTION, None).unwrap();
    declare_two(&mut r);
    let stack = &r.game.players[1].battle_area[opponent.index as usize];
    assert_eq!(stack.card_sources.len(), 1);
    assert_eq!(stack.top_card().card_id(&r.game.card_data), BOUNDARY);
    assert!(r.pending_selection().is_none());
}

#[test]
fn bio_thunderbirdmon_evolution_grants_only_opposing_digimon_effect_immunity() {
    use digimon_engine::debug_runner::make_test_card;
    use digimon_engine::enums::EffectSourceKind;
    let mut base = make_test_card("TEXT-BIO-THUNDERBIRD", "Thunderbirdmon");
    base.traits = vec!["Bio".into()];
    base.level = Some(4);
    let (mut r, host) = evolving("BT27-071", base, 5);
    assert_cannot_digivolve(&r, host);
    assert!(r
        .game
        .modifiers
        .is_unaffected_by(host, 1, EffectSourceKind::Digimon));
    assert!(!r
        .game
        .modifiers
        .is_unaffected_by(host, 0, EffectSourceKind::Digimon));
    assert!(!r
        .game
        .modifiers
        .is_unaffected_by(host, 1, EffectSourceKind::Tamer));
    assert!(!r
        .game
        .modifiers
        .is_unaffected_by(host, 1, EffectSourceKind::Option));
    next_turn(&mut r);
    assert!(r
        .game
        .modifiers
        .is_unaffected_by(host, 1, EffectSourceKind::Digimon));
    next_turn(&mut r);
    assert!(!r
        .game
        .modifiers
        .is_unaffected_by(host, 1, EffectSourceKind::Digimon));
}

#[test]
fn field_counter_de_digivolves_two_before_the_blocker_decision() {
    const CARD: &str = "BT27-071";
    let mut r = start(builder(CARD));
    let defender = r.place_on_field(0, CARD, Some(0));
    let attacker = r.place_stack(1, &[BOUNDARY, EVOLUTION, EVOLUTION]);
    let second = r.place_on_field(1, GIZMON, Some(0));
    next_turn(&mut r);
    action(
        &mut r,
        1,
        encode_attack(attacker.index as u16, SECURITY_TARGET),
    );
    r.assert_prompt(SelectionKind::Hand, true, 0);
    assert_eq!(
        r.pending_selection().unwrap().valid_action_ids,
        [encode_attack(0, defender.index as u16)]
    );
    action(&mut r, 0, encode_attack(0, defender.index as u16));
    r.assert_prompt(SelectionKind::OppField, false, 0);
    assert_eq!(candidates(&r), [EVOLUTION, GIZMON]);
    r.choose_prompt_card(EVOLUTION, None).unwrap();
    declare_two(&mut r);
    assert_eq!(
        ids(
            &r,
            &r.game.players[1].battle_area[attacker.index as usize].card_sources
        ),
        [BOUNDARY]
    );
    r.assert_prompt(SelectionKind::OwnField, true, 0);
    r.decline_prompt().unwrap();
    attack_security(&mut r, second);
    r.assert_prompt(SelectionKind::OwnField, true, 0);
    r.choose_prompt_card(CARD, None).unwrap();
    assert!(r.game.players[0].battle_area[defender.index as usize].is_suspended);
    assert_eq!(r.security_count(0), 4);
    assert!(r.pending_selection().is_none());
}

#[test]
fn reboot_unsuspends_darkdramon_at_the_start_of_the_opponents_turn() {
    let mut r = start(builder("BT27-071"));
    let host = r.place_on_field(0, "BT27-071", Some(0));
    attack_security(&mut r, host);
    assert!(r.game.players[0].battle_area[host.index as usize].is_suspended);
    next_turn(&mut r);
    assert!(!r.game.players[0].battle_area[host.index as usize].is_suspended);
    assert!(r.pending_selection().is_none());
}
