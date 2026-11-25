//! Printed text: data/card_bundles/BT27-010.md.
use super::common::*;
use digimon_engine::action::space::{
    EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_MAIN, FIELD_EFFECT_START,
};
use digimon_engine::build_action_mask;
use digimon_engine::selection::SelectionKind;

const CARD: &str = "BT27-010";

pub(super) fn main_action(slot: u8) -> u16 {
    FIELD_EFFECT_START + u16::from(slot) * EFFECTS_PER_PERMANENT + FIELD_EFFECT_SLOT_FOR_MAIN
}

#[test]
fn paying_one_refires_face_deletion_without_deleting_meramon_once_per_turn() {
    let mut r = start(builder(CARD));
    let meramon = r.place_on_field(0, CARD, Some(0));
    let body = r.top_card(meramon);
    r.place_on_field(1, "TEXT-4", Some(0));
    r.place_on_field(1, "TEXT-5", Some(0));
    let bit = main_action(meramon.index);
    action(&mut r, 0, bit);
    assert_eq!(r.memory(), 9);
    r.assert_prompt(SelectionKind::EffectChoice, true, 0);
    r.execute_branch(0).unwrap();
    r.assert_prompt(SelectionKind::OppField, false, 0);
    assert_eq!(candidates(&r), ["TEXT-4"]);
    r.choose_prompt_card("TEXT-4", None).unwrap();
    assert_eq!(ids(&r, &r.game.players[1].trash), ["TEXT-4"]);
    assert_eq!(r.top_card(meramon), body);
    assert_eq!(build_action_mask(&r.game, 0)[bit as usize], 0.0);
}

#[test]
fn declining_after_paying_still_spends_memory_and_the_main_use() {
    let mut r = start(builder(CARD));
    let meramon = r.place_on_field(0, CARD, Some(0));
    r.place_on_field(1, "TEXT-4", Some(0));
    let bit = main_action(meramon.index);
    action(&mut r, 0, bit);
    r.assert_prompt(SelectionKind::EffectChoice, true, 0);
    r.decline_prompt().unwrap();
    assert_eq!(r.memory(), 9);
    assert_eq!(r.battle_area_size(1), 1);
    assert_eq!(build_action_mask(&r.game, 0)[bit as usize], 0.0);
}

#[test]
fn both_paid_main_effects_can_refire_a_linked_deletion_effect() {
    const LINK: &str = "TEXT-DELETION-LINK";
    for card in [CARD, "BT27-012"] {
        let mut r = start(
            builder(card)
                .memory(5)
                .from_dsl_yaml(
                    r#"
card: TEXT-DELETION-LINK
name: Linked Digimon
kind: digimon
color: [red]
level: 4
dp: 4000
cost: 4
effects:
  - kind: link_requirement
    scope: inherited
    cost: 1
    filter: { kind: digimon }
  - scope: linked
    when: on_deletion
    process:
      - gain_memory: 2
  - when: on_deletion
    process:
      - gain_memory: 5
"#,
                )
                .unwrap(),
        );
        let carrier = r.place_on_field(0, card, Some(0));
        let top = r.top_card(carrier);
        let link = r.push_linked_owned(carrier, LINK, 0);
        action(&mut r, 0, main_action(carrier.index));
        assert_eq!(r.memory(), 4);
        r.assert_prompt(SelectionKind::EffectChoice, true, 0);
        let choices = r.pending_selection_view().unwrap().effect_choices.unwrap();
        assert_eq!(choices.len(), 2, "the linked card's face effect is not offered");
        let branch = choices
            .iter()
            .position(|choice| choice.source_card == Some(link))
            .expect("the linked [On Deletion] effect is offered");
        r.execute_branch(branch).unwrap();
        assert_eq!(r.memory(), 6);
        assert_eq!(r.top_card(carrier), top);
        assert_eq!(
            r.game.players[0].battle_area[carrier.index as usize].linked_cards[0].handle(),
            link
        );
    }
}

#[test]
fn actual_deletion_triggers_both_face_and_inherited_clauses() {
    for inherited in [false, true] {
        let mut r = start(builder(CARD));
        let carrier = if inherited {
            r.place_stack(0, &[CARD, "TEXT-5"])
        } else {
            r.place_on_field(0, CARD, Some(0))
        };
        r.place_on_field(1, "TEXT-4", Some(0));
        r.place_on_field(1, "TEXT-5", Some(0));
        r.game.delete_permanent_with_effects(carrier);
        r.assert_prompt(SelectionKind::OppField, false, 0);
        assert_eq!(candidates(&r), ["TEXT-4"]);
        r.choose_prompt_card("TEXT-4", None).unwrap();
        assert_eq!(r.battle_area_size(0), 0);
        assert_eq!(ids(&r, &r.game.players[1].trash), ["TEXT-4"]);
    }
}
