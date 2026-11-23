//! Source: `data/card_bundles/BT27-052.md`.
//!
//! ## Digivolution requirements (AUTHORITATIVE — official Bandai DB)
//! ### Standard digivolve cost circles
//!   - Green Lv.5 / cost 5
//!   - Black Lv.5 / cost 5
//! ### Special digivolution condition (DNA / alt-digivolve black text)
//!   [Digivolve] Lv.5 w/[Glowing Dawn] trait: Cost 4 [Digivolve] [Atratusmon] w/o play cost: Cost 2
//!
//! ## Printed text (official)
//! ### Effect
//! ＜Security A. +1＞ ＜Reboot＞ ＜Blocker＞ [On Play] [When Digivolving] [When Attacking] [Once Per Turn] This Digimon may battle 1 of your opponent's Digimon. [On Play] [When Digivolving] By trashing 2 bottom face-down cards from under any of your Tamers, you may play or use 1 play or use cost 6 or lower [Glowing Dawn] trait card from your hand or your Digimon's digivolution cards without paying the cost.
//!
//! ### Special Play Condition
//! Assembly -5: Lv.5 × Lv.4 × Lv.3, all w/[Glowing Dawn] trait
//!
//! ## Official Q&A
//! - The effects trigger simultaneously, so the player can choose the activation order.

use super::common::{action, builder, candidates, start};
use digimon_engine::selection::SelectionKind;

#[test]
fn atratusmon_can_battle_an_unsuspended_digimon_without_attacking() {
    let mut r = start(builder("BT27-052").memory(20).hand(0, &["BT27-052"]));
    r.place_on_field(1, "TEXT-3", Some(0));
    action(&mut r, 0, 0);
    r.assert_prompt(SelectionKind::Replacement, true, 0);
    r.accept_optional_trigger().unwrap();
    r.assert_prompt(SelectionKind::OppField, false, 0);
    assert_eq!(candidates(&r), ["TEXT-3"]);
    let chosen = r.prompt_card_candidates()[0].0;
    action(&mut r, 0, chosen);
    assert!(r.game.players[1].battle_area.is_empty());
    assert!(!r.game.players[0].battle_area[0].is_suspended);
    assert_eq!(r.security_count(1), 5);
}

#[test]
fn without_play_cost_atratusmon_evolves_for_two_regardless_of_level_and_color() {
    use digimon_engine::action::space::encode_digivolve;
    let mut r = start(
        builder("BT27-052")
            .from_dsl_yaml(
                r#"
card: TEXT-DUAL-ATRATUSMON
name: Atratusmon
kind: dual
level: 6
color: [blue]
cost: 5
dp: 12000
dual:
  digimon:
    level: 6
    dp: 12000
    colors: [blue]
    effects: []
  option:
    colors: [blue]
    use_cost: 5
    effects: []
"#,
            )
            .unwrap()
            .hand(0, &["BT27-052"]),
    );
    let base = r.place_on_field(0, "TEXT-DUAL-ATRATUSMON", Some(0));
    action(&mut r, 0, encode_digivolve(0, base.index as u16));
    assert_eq!(r.memory(), 8);
    assert_eq!(
        super::common::ids(&r, &r.game.players[0].battle_area[0].card_sources),
        ["TEXT-DUAL-ATRATUSMON", "BT27-052"]
    );
}

#[test]
fn battle_on_digivolution_consumes_the_same_once_per_turn_as_when_attacking() {
    use digimon_engine::action::space::{encode_attack, encode_digivolve, SECURITY_TARGET};
    let mut r = start(builder("BT27-052").hand(0, &["BT27-052"]));
    let host = r.place_on_field(0, "TEXT-5", Some(0));
    r.place_on_field(1, "TEXT-3", Some(0));
    r.place_on_field(1, "TEXT-4", Some(0));
    action(&mut r, 0, encode_digivolve(0, host.index as u16));
    r.assert_prompt(SelectionKind::EffectChoice, false, 0);
    r.execute_branch(0).unwrap();
    r.assert_prompt(SelectionKind::Replacement, true, 0);
    r.accept_optional_trigger().unwrap();
    r.assert_prompt(SelectionKind::OppField, false, 0);
    assert_eq!(candidates(&r), ["TEXT-3", "TEXT-4"]);
    r.choose_prompt_card("TEXT-3", None).unwrap();
    action(&mut r, 0, encode_attack(host.index as u16, SECURITY_TARGET));
    assert!(r.pending_selection().is_none());
    assert_eq!(r.game.players[1].battle_area.len(), 1);
    assert_eq!(r.security_count(1), 3);
}

#[test]
fn uses_a_glowing_dawn_option_from_hand_at_no_cost_after_payment() {
    use super::common::{pay_tamer_sources, stocked_tamer};
    let mut r = start(
        builder("BT27-052")
            .from_dsl_yaml(
                r#"
card: TEXT-GLOWING-OPTION
name: Test Glowing Dawn Option
kind: option
cost: 6
color: [green]
traits: [Glowing Dawn]
effects:
  - when: main
    process:
      - draw: { of: you, count: 1 }
"#,
            )
            .unwrap()
            .memory(10)
            .hand(0, &["BT27-052", "TEXT-GLOWING-OPTION"]),
    );
    stocked_tamer(&mut r, "TEXT-TAMER");
    r.place_on_field(0, "TEXT-3", Some(0));
    let deck = r.game.players[0].deck.len();
    action(&mut r, 0, 0);
    r.assert_prompt(SelectionKind::Replacement, true, 0);
    let memory = r.memory();
    r.accept_optional_trigger().unwrap();
    pay_tamer_sources(&mut r, 0, "TEXT-TAMER", 2);
    assert!(matches!(
        r.pending_kind(),
        Some(SelectionKind::UnionZone { .. })
    ));
    assert_eq!(candidates(&r), ["TEXT-GLOWING-OPTION"]);
    r.choose_prompt_card("TEXT-GLOWING-OPTION", None).unwrap();
    assert_eq!(r.game.players[0].deck.len(), deck - 1);
    assert!(super::common::ids(&r, &r.game.players[0].trash)
        .contains(&"TEXT-GLOWING-OPTION".to_owned()));
    assert!(
        memory < 0,
        "Atratusmon must be paid for even when the Option is free"
    );
}

#[test]
fn printed_standard_and_glowing_dawn_routes_charge_their_listed_costs() {
    use digimon_engine::enums::CardColor::{Black, Green};
    super::common::digivolve_paths("BT27-052", 5, &[(Green, 5), (Black, 5)], 4);
}

#[test]
fn assembly_requires_one_glowing_dawn_digimon_of_each_level_and_reduces_cost_by_five() {
    let mut r = start(builder("BT27-052").hand(0, &["BT27-052"]));
    let before = r.memory();
    for id in ["TEXT-OTHER", "TEXT-3", "TEXT-5", "TEXT-4"] {
        r.inject_trash(0, id);
    }
    action(&mut r, 0, 0);
    for id in ["TEXT-5", "TEXT-4", "TEXT-3"] {
        assert!(matches!(
            r.pending_kind(),
            Some(SelectionKind::CountCappedMultiSelect {
                max: 1,
                picked: 0,
                ..
            })
        ));
        assert_eq!(r.pending_is_optional(), id == "TEXT-5");
        assert_eq!(candidates(&r), [id]);
        r.choose_prompt_card(id, None).unwrap();
    }
    let stack = &r.game.players[0].battle_area[0].card_sources;
    assert_eq!(
        super::common::ids(&r, stack),
        ["TEXT-3", "TEXT-4", "TEXT-5", "BT27-052"]
    );
    assert_eq!(before - r.memory(), 8);
    assert_eq!(
        super::common::ids(&r, &r.game.players[0].trash),
        ["TEXT-OTHER"]
    );
}

#[test]
fn on_play_offers_both_simultaneous_effects_and_can_play_a_material() {
    let mut r = start(builder("BT27-052").memory(20).hand(0, &["BT27-052"]));
    super::common::stocked_tamer(&mut r, "TEXT-TAMER");
    r.place_stack(0, &["TEXT-3", "TEXT-4"]);
    r.place_on_field(1, "TEXT-3", Some(0));
    action(&mut r, 0, 0);
    r.assert_prompt(SelectionKind::TriggerOrder, true, 0);
    r.execute_branch(1).unwrap();
    r.assert_prompt(SelectionKind::Replacement, true, 0);
    r.accept_optional_trigger().unwrap();
    super::common::pay_tamer_sources(&mut r, 0, "TEXT-TAMER", 2);
    assert!(matches!(
        r.pending_kind(),
        Some(SelectionKind::UnionZone { .. })
    ));
    assert_eq!(candidates(&r), ["TEXT-3"]);
    r.choose_prompt_card("TEXT-3", None).unwrap();
    r.assert_prompt(SelectionKind::Replacement, true, 0);
    r.decline_optional_trigger().unwrap();
    assert_eq!(
        super::common::ids(&r, &r.game.players[0].battle_area[1].card_sources),
        ["TEXT-4"]
    );
    assert_eq!(
        super::common::ids(&r, &r.game.players[0].battle_area[3].card_sources),
        ["TEXT-3"]
    );
    let atratusmon = digimon_engine::permanent::PermanentHandle {
        player: 0,
        index: 2,
    };
    assert!(r
        .game
        .has_keyword(atratusmon, digimon_engine::enums::Keyword::Blocker));
    assert!(r
        .game
        .has_keyword(atratusmon, digimon_engine::enums::Keyword::Reboot));
    assert_eq!(r.game.security_attack_keyword_bonus(atratusmon), 1);
}
