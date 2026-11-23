//! Printed text: data/card_bundles/BT27-012.md.
use super::bt27_010::main_action;
use super::common::*;
use digimon_engine::selection::SelectionKind;

const CARD: &str = "BT27-012";

#[test]
fn selectable_face_and_inherited_deletion_effects_keep_their_sources() {
    for choose_inherited in [false, true] {
        let mut r = start(builder(CARD).dsl_card("BT27-010").unwrap());
        let skull = r.place_stack(0, &["BT27-010", CARD]);
        let sources = &r.game.players[0].battle_area[skull.index as usize].card_sources;
        let chosen_source = sources[usize::from(!choose_inherited)].handle();
        r.place_on_field(1, "TEXT-3", Some(0));
        r.place_on_field(1, "TEXT-4", Some(0));
        r.place_on_field(1, "TEXT-5", Some(0));
        action(&mut r, 0, main_action(skull.index));
        r.assert_prompt(SelectionKind::EffectChoice, true, 0);
        let choices = r.pending_selection_view().unwrap().effect_choices.unwrap();
        assert_eq!(choices.len(), 2);
        let branch = choices
            .iter()
            .position(|choice| choice.source_card == Some(chosen_source))
            .unwrap();
        r.execute_branch(branch).unwrap();
        r.assert_prompt(SelectionKind::OppField, false, 0);
        if choose_inherited {
            assert_eq!(candidates(&r), ["TEXT-3", "TEXT-4"]);
            r.choose_prompt_card("TEXT-4", None).unwrap();
            assert_eq!(ids(&r, &r.game.players[1].trash), ["TEXT-4"]);
        } else {
            assert_eq!(candidates(&r), ["TEXT-3"]);
            r.choose_prompt_card("TEXT-3", None).unwrap();
            assert_eq!(ids(&r, &r.game.players[1].trash), ["TEXT-3"]);
        }
        assert_eq!(r.battle_area_size(0), 1);
        assert_eq!(r.memory(), 9);
    }
}

#[test]
fn skullmeramon_inheritance_can_be_refired_to_trash_opponent_security() {
    let mut r = start(builder(CARD).dsl_card("BT27-010").unwrap());
    let carrier = r.place_stack(0, &[CARD, "BT27-010"]);
    let source = r.game.players[0].battle_area[carrier.index as usize].card_sources[0].handle();
    action(&mut r, 0, main_action(carrier.index));
    r.assert_prompt(SelectionKind::EffectChoice, true, 0);
    let choices = r.pending_selection_view().unwrap().effect_choices.unwrap();
    let branch = choices
        .iter()
        .position(|choice| choice.source_card == Some(source))
        .unwrap();
    r.execute_branch(branch).unwrap();
    assert_eq!(r.game.players[1].security.len(), 4);
    assert_eq!(r.battle_area_size(0), 1);
}

#[test]
fn deleting_a_carrier_with_skullmeramon_under_it_trashes_the_top_security_card() {
    let mut r = start(builder(CARD).security(1, &["TEXT-4", "TEXT-5"]));
    let top = r.game.players[1].security.last().unwrap().handle();
    let carrier = r.place_stack(0, &[CARD, "TEXT-6"]);
    r.game.delete_permanent_with_effects(carrier);
    assert_eq!(r.game.players[1].security.len(), 1);
    assert_eq!(r.game.players[1].trash.last().unwrap().handle(), top);
    assert_eq!(r.battle_area_size(0), 0);
}
