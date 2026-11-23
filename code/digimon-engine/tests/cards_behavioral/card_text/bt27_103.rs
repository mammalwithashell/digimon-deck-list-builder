//! Source: `data/card_bundles/BT27-103.md` (printed conditions, effects, and official Q&A).
//!
//! ### Standard digivolve cost circles
//!   - Black Lv.6 / cost 6
//!   - Yellow Lv.6 / cost 6
//!   - Red Lv.6 / cost 6
//! ### Special digivolution condition (DNA / alt-digivolve black text)
//!   [DNA Digivolve] [Alphamon] + [Ouryumon]: Cost 0
//!
//! ## Printed text (official)
//! ### Effect
//! ＜Raid＞ ＜Piercing＞ ＜Blocker＞ ＜Partition ([Alphamon] & [Ouryumon])＞ [When Digivolving] ＜Recovery +1＞. Then, if DNA digivolving, none of your opponent's play cost 7 or higher Digimon can attack until their turn ends. [When Digivolving] [When Attacking] [Once Per Turn] You may activate 1 [Main] effect on this card's Option side.
//!
//! ### DUAL Effect
//! [Main] If during an attack, delete all of your opponent's lowest DP Digimon. Then, delete 1 of their Digimon.
//!
//! ### DUAL Rule
//! ＜Arts Digivolve＞ (Instead of trashing after use, 2 of your Digimon may DNA digivolve into this card.)
//!
//! ## Official Q&A
//! - Yes, you can. If an effect activates, it is to be fully resolved even if the card that activated the effect is removed from that area during the processing.

const CARD: &str = "BT27-103";

use super::common::*;
use digimon_engine::action::space::PLAY_HAND_START;

#[test]
fn arts_dna_uses_two_matching_field_materials_or_trashes_the_used_option() {
    use digimon_engine::action::space::encode_attack;
    use digimon_engine::enums::EffectTiming;
    use digimon_engine::events::GameEvent;
    use digimon_engine::selection::SelectionKind;

    for (stop_after, reverse) in [(0, false), (1, false), (2, false), (2, true)] {
        let mut alphamon = digimon("ALPHA", 6, 12000, false);
        alphamon.card_name = "Alphamon".into();
        let mut ouryumon = digimon("OURYU", 6, 12000, false);
        ouryumon.card_name = "Ouryumon".into();
        let mut r = start(
            builder(CARD)
                .add_card(alphamon)
                .add_card(ouryumon)
                .hand(0, &[CARD]),
        );
        let alphamon = r.place_on_field(0, "ALPHA", Some(0));
        let ouryumon = r.place_on_field(0, "OURYU", Some(0));
        r.place_on_field(0, "TEXT-6", Some(0));
        r.place_on_field(1, "TEXT-3", Some(0));
        action(&mut r, 0, PLAY_HAND_START);
        r.assert_prompt(SelectionKind::OppField, false, 0);
        assert_eq!(candidates(&r), ["TEXT-3"]);
        r.choose_prompt_card("TEXT-3", None).unwrap();
        r.assert_prompt(SelectionKind::OwnField, true, 0);
        assert_eq!(candidates(&r), ["ALPHA", "OURYU"]);
        assert_eq!(r.memory(), 4);
        if stop_after == 0 {
            r.decline_prompt().unwrap();
        } else {
            let (first, second, second_id) = if reverse {
                (ouryumon, alphamon, "ALPHA")
            } else {
                (alphamon, ouryumon, "OURYU")
            };
            action(&mut r, 0, encode_attack(0, first.index as u16));
            r.assert_prompt(SelectionKind::OwnField, true, 0);
            assert_eq!(candidates(&r), [second_id]);
            if stop_after == 1 {
                r.decline_prompt().unwrap();
            } else {
                let before_dna = r.event_checkpoint();
                action(&mut r, 0, encode_attack(0, second.index as u16));
                let events = r.events_of_kind(before_dna, |event| {
                    matches!(event, GameEvent::Digivolve { was_dna: true, .. })
                });
                let [GameEvent::Digivolve { from_stack_top, top_card_id, field_index, .. }] =
                    events.as_slice()
                else {
                    panic!("expected exactly one DNA Digivolve event, got {events:?}");
                };
                assert_eq!(from_stack_top, if reverse { "OURYU" } else { "ALPHA" });
                assert_eq!(top_card_id, CARD);
                assert_eq!(*field_index, 0);
                r.assert_prompt(SelectionKind::TriggerOrder, false, 0);
                let choices = r.pending_selection_view().unwrap().effect_choices.unwrap();
                let recover = choices.iter().find(|effect|
                    effect.timing == Some(EffectTiming::WhenDigivolving) && !effect.is_optional
                ).unwrap();
                action(&mut r, 0, recover.action_id);
                r.decline_prompt().unwrap();
            }
        }
        assert!(r.pending_selection().is_none());
        assert!(r.game.pending_option.is_none());
        assert_eq!(r.memory(), 4);
        if stop_after == 2 {
            assert_eq!(ids(&r, &r.game.players[0].battle_area[0].card_sources), ["OURYU", "ALPHA", CARD]);
            assert_eq!(r.security_count(0), 6);
            assert!(r.game.players[0].trash.is_empty());
            assert_eq!(r.battle_area_size(0), 2);
        } else {
            assert_eq!(r.battle_area_size(0), 3);
            assert_eq!(r.security_count(0), 5);
            assert_eq!(ids(&r, &r.game.players[0].trash), [CARD]);
        }
    }
}

#[test]
fn arts_dna_is_not_offered_for_a_single_legal_material_or_a_normal_evolution_base() {
    let mut alphamon = digimon("ALPHA", 6, 12000, false);
    alphamon.card_name = "Alphamon".into();
    let mut r = start(builder(CARD).add_card(alphamon).hand(0, &[CARD]));
    r.place_on_field(0, "ALPHA", Some(0));
    r.place_on_field(0, "TEXT-6", Some(0));
    r.place_on_field(1, "TEXT-3", Some(0));
    action(&mut r, 0, PLAY_HAND_START);
    r.choose_prompt_card("TEXT-3", None).unwrap();
    assert!(r.pending_selection().is_none());
    assert_eq!(r.battle_area_size(0), 2);
    assert_eq!(ids(&r, &r.game.players[0].trash), [CARD]);
}

#[test]
fn ouryuken_is_not_alphamon_for_regular_or_arts_dna() {
    use digimon_engine::action::space::DNA_DIGIVOLVE_START;

    let mut ouryumon = digimon("OURYU", 6, 12000, false);
    ouryumon.card_name = "Ouryumon".into();
    let mut r = start(builder(CARD).add_card(ouryumon).hand(0, &[CARD]));
    r.place_on_field(0, CARD, Some(0));
    r.place_on_field(0, "OURYU", Some(0));
    r.place_on_field(1, "TEXT-3", Some(0));

    let regular_dna = digimon_engine::build_action_mask(&r.game, 0)[DNA_DIGIVOLVE_START as usize];
    action(&mut r, 0, PLAY_HAND_START);
    r.choose_prompt_card("TEXT-3", None).unwrap();
    assert!(
        r.pending_selection().is_none(),
        "Ouryuken cannot be an Arts material: {:?}",
        r.pending_selection_view()
    );
    assert_eq!(regular_dna, 0.0, "Ouryuken cannot be a regular DNA material");
    assert_eq!(ids(&r, &r.game.players[0].trash), [CARD]);
}

#[test]
fn dna_digivolution_costs_zero_recovers_and_blocks_play_cost_seven_or_more_from_attacking() {
    use digimon_engine::action::space::{encode_attack, DNA_DIGIVOLVE_START};
    use digimon_engine::debug_runner::make_test_card;
    use digimon_engine::enums::EffectTiming;
    use digimon_engine::selection::SelectionKind;
    let mut expensive = digimon("EXPENSIVE", 5, 7000, false);
    expensive.play_cost = 7;
    let mut cheap = digimon("CHEAP", 5, 7000, false);
    cheap.play_cost = 6;
    let mut r = start(
        builder(CARD)
            .add_card(make_test_card("ALPHA", "Alphamon"))
            .add_card(make_test_card("OURYU", "Ouryumon"))
            .add_card(expensive)
            .add_card(cheap)
            .security(0, &["TEXT-FILLER"; 3])
            .hand(0, &[CARD]),
    );
    let alpha = r.place_on_field(0, "ALPHA", Some(0));
    let ouryu = r.place_on_field(0, "OURYU", Some(0));
    let expensive = r.place_on_field(1, "EXPENSIVE", Some(0));
    let cheap = r.place_on_field(1, "CHEAP", Some(0));
    action(&mut r, 0, DNA_DIGIVOLVE_START);
    r.assert_prompt(SelectionKind::Material, false, 0);
    assert_eq!(
        r.pending_selection().unwrap().valid_action_ids,
        [alpha.index as u16, ouryu.index as u16]
    );
    action(&mut r, 0, alpha.index as u16);
    r.assert_prompt(SelectionKind::Material, false, 0);
    assert_eq!(
        r.pending_selection().unwrap().valid_action_ids,
        [ouryu.index as u16]
    );
    action(&mut r, 0, ouryu.index as u16);
    r.assert_prompt(SelectionKind::TriggerOrder, false, 0);
    let choices = r.pending_selection_view().unwrap().effect_choices.unwrap();
    assert_eq!(choices.len(), 2);
    let recover = choices
        .iter()
        .find(|effect| effect.timing == Some(EffectTiming::WhenDigivolving) && !effect.is_optional)
        .unwrap();
    action(&mut r, 0, recover.action_id);
    r.decline_prompt().unwrap();
    assert_eq!(r.memory(), 10);
    assert_eq!(r.security_count(0), 4);
    assert_eq!(r.battle_area_size(0), 1);
    next_turn(&mut r);
    let mask = digimon_engine::build_action_mask(&r.game, 1);
    assert_eq!(
        mask[encode_attack(expensive.index as u16, 14) as usize],
        0.0
    );
    assert_eq!(mask[encode_attack(cheap.index as u16, 14) as usize], 1.0);
}

#[test]
fn option_outside_an_attack_deletes_only_the_chosen_digimon() {
    let mut r = start(builder(CARD).hand(0, &[CARD]));
    r.place_on_field(0, "TEXT-4", Some(0));
    r.place_on_field(1, "TEXT-3", Some(0));
    r.place_on_field(1, "TEXT-5", Some(0));
    action(&mut r, 0, PLAY_HAND_START);
    assert_eq!(candidates(&r), ["TEXT-3", "TEXT-5"]);
    r.choose_prompt_card("TEXT-5", None).unwrap();
    assert_eq!(ids(&r, &r.game.players[1].trash), ["TEXT-5"]);
    assert_eq!(r.battle_area_size(1), 1);
    assert!(r.pending_selection().is_none());
}

#[test]
fn attacking_may_refire_the_option_to_delete_all_tied_lowest_dp_then_one_more() {
    use digimon_engine::action::space::encode_attack;
    use digimon_engine::enums::Keyword;
    use digimon_engine::selection::SelectionKind;

    for accept in [false, true] {
        let mut r = start(builder(CARD));
        let attacker = r.place_on_field(0, CARD, Some(0));
        r.place_on_field(1, "TEXT-3", Some(0));
        r.place_on_field(1, "TEXT-3", Some(0));
        r.place_on_field(1, "TEXT-4", Some(0));
        r.place_on_field(1, "TEXT-5", Some(0));
        for keyword in [Keyword::Raid, Keyword::Piercing, Keyword::Blocker, Keyword::Partition] {
            assert!(r.game.has_keyword(attacker, keyword));
        }
        action(&mut r, 0, encode_attack(attacker.index as u16, 14));
        r.assert_prompt(SelectionKind::TriggerOrder, true, 0);
        let choices = r.pending_selection_view().unwrap().effect_choices.unwrap();
        let raid = choices.iter().find(|choice| choice.keyword == Some(Keyword::Raid)).unwrap();
        action(&mut r, 0, raid.action_id);
        r.assert_prompt(SelectionKind::OppField, true, 0);
        assert_eq!(candidates(&r), ["TEXT-5"]);
        r.decline_prompt().unwrap();
        r.assert_prompt(SelectionKind::Replacement, true, 0);
        if accept {
            r.accept_optional_trigger().unwrap();
            assert_eq!(ids(&r, &r.game.players[1].trash), ["TEXT-3", "TEXT-3"], "prompt={:?}", r.pending_selection_view());
            r.assert_prompt(SelectionKind::OppField, false, 0);
            assert_eq!(candidates(&r), ["TEXT-4", "TEXT-5"]);
            r.choose_prompt_card("TEXT-5", None).unwrap();
            assert_eq!(ids(&r, &r.game.players[1].trash), ["TEXT-3", "TEXT-3", "TEXT-5", "TEXT-FILLER"]);
        } else {
            r.decline_prompt().unwrap();
            assert_eq!(ids(&r, &r.game.players[1].trash), ["TEXT-FILLER"]);
        }
    }
}

#[test]
fn activated_option_finishes_even_if_ouryuken_is_deleted_during_resolution() {
    use digimon_engine::action::space::encode_attack;
    use digimon_engine::enums::Keyword;
    use digimon_engine::selection::SelectionKind;

    let mut r = start(builder(CARD).from_dsl_yaml(r#"
card: FIXTURE-INTERRUPT
name: Interrupting Digimon
kind: digimon
level: 3
dp: 1000
cost: 3
color: [black]
effects:
  - kind: replacement
    trigger: when_would_be_deleted
    optional: true
    active_when: { is_source: true }
    process:
      - select_opponent_permanent:
          bind_as: victim
          filter: { kind: digimon }
          prompt: "Delete the attacking Digimon"
      - delete_permanent: { target: victim }
      - cancel_replacement: {}
"#).unwrap());
    let attacker = r.place_on_field(0, CARD, Some(0));
    r.place_on_field(1, "FIXTURE-INTERRUPT", Some(0));
    r.place_on_field(1, "TEXT-4", Some(0));
    action(&mut r, 0, encode_attack(attacker.index.into(), 14));
    r.assert_prompt(SelectionKind::TriggerOrder, true, 0);
    let raid = r.pending_selection_view().unwrap().effect_choices.unwrap()
        .into_iter().find(|choice| choice.keyword == Some(Keyword::Raid)).unwrap();
    action(&mut r, 0, raid.action_id);
    r.assert_prompt(SelectionKind::OppField, true, 0);
    r.decline_prompt().unwrap();
    r.assert_prompt(SelectionKind::Replacement, true, 0);
    r.accept_optional_trigger().unwrap();
    r.assert_prompt(SelectionKind::Replacement, true, 1);
    r.accept_optional_trigger().unwrap();
    r.assert_prompt(SelectionKind::OppField, false, 1);
    assert_eq!(candidates(&r), [CARD]);
    r.choose_prompt_card(CARD, None).unwrap();
    assert_eq!(ids(&r, &r.game.players[0].trash), [CARD]);
    r.assert_prompt(SelectionKind::OppField, false, 0);
    assert_eq!(candidates(&r), ["FIXTURE-INTERRUPT", "TEXT-4"]);
    r.choose_prompt_card("TEXT-4", None).unwrap();
    assert_eq!(ids(&r, &r.game.players[1].trash), ["TEXT-4"]);
    assert_eq!(r.security_count(1), 5);
    assert!(r.pending_selection().is_none());
}

#[test]
fn partition_plays_alphamon_and_ouryumon_but_not_unrelated_sources() {
    use digimon_engine::selection::SelectionKind;
    for (has_both, accept) in [(true, true), (true, false), (false, false)] {
        let mut alpha = digimon("ALPHA", 6, 12000, false);
        alpha.card_name = "Alphamon".into();
        let mut ouryu = digimon("OURYU", 6, 12000, false);
        ouryu.card_name = "Ouryumon".into();
        let mut r = start(builder(CARD).add_card(alpha).add_card(ouryu).hand(1, &[CARD]));
        r.place_stack(0, &["TEXT-4", if has_both { "OURYU" } else { "TEXT-5" }, "ALPHA", CARD]);
        r.place_on_field(1, "TEXT-4", Some(0));
        next_turn(&mut r);
        action(&mut r, 1, PLAY_HAND_START);
        r.assert_prompt(SelectionKind::OppField, false, 1);
        assert_eq!(candidates(&r), [CARD]);
        r.choose_prompt_card(CARD, None).unwrap();
        if has_both {
            r.assert_prompt(SelectionKind::Replacement, true, 0);
            if accept {
                r.accept_optional_trigger().unwrap();
                r.assert_prompt(SelectionKind::CountCappedMultiSelect { min: 1, max: 1, picked: 0, distinct: false }, false, 0);
                assert_eq!(candidates(&r), ["ALPHA", "OURYU"]);
                r.choose_prompt_card("ALPHA", None).unwrap();
            } else {
                r.decline_prompt().unwrap();
            }
        }
        assert!(r.pending_selection().is_none());
        assert_eq!(r.battle_area_size(0), if accept { 2 } else { 0 });
        if accept {
            assert_eq!(ids(&r, &r.game.players[0].trash), ["TEXT-4", CARD]);
            let mut names: Vec<_> = r.game.players[0].battle_area.iter()
                .map(|p| p.top_card().card_id(&r.game.card_data))
                .collect();
            names.sort_unstable();
            assert_eq!(names, ["ALPHA", "OURYU"]);
        }
    }
}
