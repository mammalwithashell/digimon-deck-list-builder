//! Source: `data/card_bundles/BT27-082.md`.
//!
//! ## Digivolution requirements (AUTHORITATIVE — official Bandai DB)
//! ### Standard digivolve cost circles
//!   - Purple Lv.5 / cost 3
//! ### Special digivolution condition (DNA / alt-digivolve black text)
//!   [Digivolve] Lv.5 w/[Glowing Dawn] trait: Cost 3
//!
//! ## Printed text (official)
//! ### Effect
//! ＜Execute＞ ＜Ascension＞ (When this Digimon is deleted, you may place this card as the top security card.) [Security] [On Deletion] Delete 1 of your opponent's Digimon with the lowest level. If this effect didn't delete, trash their top security card. [When Digivolving] [When Attacking] [Once Per Turn] By trashing 2 bottom face-down cards from under any of your Tamers, activate 1 [Main] effect on this card's Option side.
//!
//! ### DUAL Effect
//! ＜Use Req. ([Glowing Dawn] trait)＞ (Specified conditions meet color requirements.) [Main] Delete 1 of your opponent's level 6 or higher Digimon. Then, you may play 1 play cost 4 or lower [BEATBREAK] trait card from your trash without paying the cost.
//!
//! ### DUAL Rule
//! ＜Arts Digivolve＞ (Instead of trashing after use, your cards may digivolve into this card without paying the cost.)
//!
//! ## Official Q&A
//! - Yes, its [Security] effect activates, then it battles.

use super::common::ids;
use super::common::{action, builder, candidates, start};
use digimon_engine::action::space::{encode_attack, SECURITY_TARGET};
use digimon_engine::selection::SelectionKind;

#[test]
fn nosferamon_security_deletes_the_lowest_level_digimon() {
    let mut r = start(builder("BT27-082").security(1, &["BT27-082"]));
    let attacker = r.place_on_field(0, "TEXT-6", Some(0));
    r.place_on_field(0, "TEXT-3", Some(0));
    action(
        &mut r,
        0,
        encode_attack(attacker.index as u16, SECURITY_TARGET),
    );
    r.assert_prompt(SelectionKind::OppField, false, 1);
    assert_eq!(candidates(&r), ["TEXT-3"]);
    let target = r.prompt_card_candidates()[0].0;
    action(&mut r, 1, target);
    assert!(ids(&r, &r.game.players[0].trash).contains(&"TEXT-3".to_owned()));
    assert_eq!(r.security_count(0), 5);
}

#[test]
fn execute_attacks_at_end_of_turn_then_ascension_can_move_the_deleted_card_to_security() {
    use digimon_engine::enums::Keyword;
    for ascend in [false, true] {
        let mut r = start(builder("BT27-082"));
        let nosferamon = r.place_on_field(0, "BT27-082", Some(0));
        r.game.end_turn();
        assert_eq!(
            r.game.pending_end_of_turn_keyword_decision(),
            Some(Keyword::Execute)
        );
        r.execute_action(0, encode_attack(nosferamon.index as u16, SECURITY_TARGET))
            .unwrap();
        assert!(r.game.players[0].battle_area.is_empty());
        r.assert_prompt(SelectionKind::TriggerOrder, false, 0);
        let choices = r
            .pending_selection()
            .unwrap()
            .effect_choices
            .as_ref()
            .unwrap();
        let deletion = choices
            .iter()
            .position(|choice| choice.keyword.is_none())
            .unwrap();
        r.execute_branch(deletion).unwrap();
        assert_eq!(r.security_count(1), 3);
        r.assert_prompt(SelectionKind::EffectChoice, false, 0);
        r.execute_branch(usize::from(!ascend)).unwrap();
        assert_eq!(r.security_count(0), 5 + usize::from(ascend));
        assert_eq!(r.security_count(1), 3);
        if ascend {
            assert_eq!(
                ids(&r, &r.game.players[0].security).last().unwrap(),
                "BT27-082"
            );
        }
    }
}

#[test]
fn glowing_dawn_satisfies_option_use_without_matching_color_and_allows_arts_digivolve() {
    use super::common::digimon;
    use digimon_engine::enums::CardColor;
    for accept in [false, true] {
        let mut off_color = digimon("TEXT-OFF-COLOR", 5, 6000, true);
        off_color.colors = vec![CardColor::Blue];
        let mut r = start(
            builder("BT27-082")
                .add_card(off_color)
                .hand(0, &["BT27-082"]),
        );
        let host = r.place_on_field(0, "TEXT-OFF-COLOR", Some(0));
        action(&mut r, 0, 0);
        r.assert_prompt(SelectionKind::OwnField, true, 0);
        if accept {
            r.choose_prompt_card("TEXT-OFF-COLOR", None).unwrap();
        } else {
            r.decline_prompt().unwrap();
        }
        assert_eq!(r.memory(), 5);
        let stack = &r.game.players[0].battle_area[host.index as usize].card_sources;
        assert_eq!(stack.len(), 1 + usize::from(accept));
        assert_eq!(
            stack.last().unwrap().card_id(&r.game.card_data),
            if accept { "BT27-082" } else { "TEXT-OFF-COLOR" }
        );
        assert_eq!(
            ids(&r, &r.game.players[0].trash).contains(&"BT27-082".to_owned()),
            !accept
        );
    }
}

#[test]
fn printed_standard_and_glowing_dawn_routes_charge_their_listed_costs() {
    use digimon_engine::enums::CardColor::Purple;
    super::common::digivolve_paths("BT27-082", 5, &[(Purple, 3)], 3);
}

#[test]
fn deletion_without_an_opposing_digimon_trashes_top_security() {
    use digimon_engine::replacement::ReplacementCause;
    let mut r = start(builder("BT27-082"));
    let nosferamon = r.place_on_field(0, "BT27-082", Some(0));
    let security = r.security_count(1);
    assert!(r
        .game
        .has_keyword(nosferamon, digimon_engine::enums::Keyword::Execute));
    assert!(r
        .game
        .has_keyword(nosferamon, digimon_engine::enums::Keyword::Ascension));
    r.game
        .delete_permanent_with_cause(nosferamon, ReplacementCause::OpponentEffect);
    r.game.drain_effect_queue();
    if r.pending_kind() == Some(SelectionKind::TriggerOrder) {
        r.execute_branch(0).unwrap();
    }
    r.assert_prompt(SelectionKind::EffectChoice, false, 0);
    r.execute_branch(1).unwrap();
    assert_eq!(r.security_count(1), security - 1);
}

#[test]
fn attacking_can_pay_two_bottom_cards_to_delete_a_level_six_and_play_from_trash() {
    use super::common::{digimon, pay_tamer_sources, stocked_tamer};
    let mut cheap = digimon("TEXT-CHEAP", 4, 3000, true);
    cheap.play_cost = 4;
    let mut costly = digimon("TEXT-COSTLY", 5, 7000, true);
    costly.play_cost = 5;
    let mut r = start(builder("BT27-082").add_card(cheap).add_card(costly));
    stocked_tamer(&mut r, "TEXT-TAMER");
    let nosferamon = r.place_on_field(0, "BT27-082", Some(0));
    r.place_on_field(1, "TEXT-6", Some(0));
    r.place_on_field(1, "TEXT-5", Some(0));
    for id in ["TEXT-CHEAP", "TEXT-COSTLY", "TEXT-OTHER"] {
        r.inject_trash(0, id);
    }
    let memory = r.memory();
    action(
        &mut r,
        0,
        encode_attack(nosferamon.index as u16, SECURITY_TARGET),
    );
    r.assert_prompt(SelectionKind::Replacement, true, 0);
    r.accept_optional_trigger().unwrap();
    pay_tamer_sources(&mut r, 0, "TEXT-TAMER", 2);
    r.assert_prompt(SelectionKind::OppField, false, 0);
    assert_eq!(candidates(&r), ["TEXT-6"]);
    r.choose_prompt_card("TEXT-6", None).unwrap();
    r.assert_prompt(SelectionKind::Trash, true, 0);
    assert_eq!(candidates(&r), ["TEXT-CHEAP"]);
    r.choose_prompt_card("TEXT-CHEAP", None).unwrap();
    assert_eq!(
        ids(&r, &r.game.players[0].battle_area[2].card_sources),
        ["TEXT-CHEAP"]
    );
    assert_eq!(r.memory(), memory);
}
