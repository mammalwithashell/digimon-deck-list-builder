//! Source: `data/card_bundles/BT27-062.md` (printed text).
//!
//! ### Effect
//! [When Attacking] [Once Per Turn] You may play or use 1 card with [Knightmon] in its text from your hand with the cost reduced by 2.
//!
//! ### Inherited Effect
//! [All Turns] [Once Per Turn] When this Digimon would leave the battle area, by deleting 1 of your other Digimon with [Knightmon] in its text, it doesn't leave.
//!
//! ## Official Q&A
//! - It refers to a card that contains the specified text or icon in its name, traits, effects, inherited effects, (Rule), digivolve requirements, DNA Digivolve, DigiXros requirements, Burst Digivolve, App Fusion, Link, or Assembly requirements. For example, "a card with [Knightmon] in its text" would include cards with the name [DarkKnightmon] and cards with the text [Knightmon] in their effects.

use super::common::*;
use digimon_engine::debug_runner::make_test_card;
use digimon_engine::selection::SelectionKind;

const CARD: &str = "BT27-062";

#[test]
fn attacking_can_play_an_affordable_knightmon_text_card_at_discount() {
    let mut knight = make_test_card("KNIGHT", "Knightmon");
    knight.play_cost = 4;
    let mut r = start(
        builder(CARD)
            .add_card(knight)
            .hand(0, &["KNIGHT", "TEXT-4"]),
    );
    let carrier = r.place_on_field(0, CARD, Some(0));
    r.attack_player(carrier, 1, false);
    r.assert_prompt(SelectionKind::Hand, true, 0);
    assert_eq!(candidates(&r), ["KNIGHT"]);
    r.choose_prompt_card("KNIGHT", None).unwrap();
    assert_eq!(r.memory(), 8);
    assert_eq!(r.hand_size(0), 1);
}

#[test]
fn cannot_reduce_play_cost_allows_the_full_cost_and_excludes_unaffordable_cards() {
    for affordable in [false, true] {
        let mut knight = make_test_card("KNIGHT", "Knightmon");
        knight.play_cost = if affordable { 4 } else { 11 };
        let mut setup = builder(CARD)
            .dsl_card("ST12-03")
            .unwrap()
            .add_card(knight)
            .hand(0, &["KNIGHT"]);
        if !affordable {
            setup = setup.memory(0);
        }
        let mut r = start(setup);
        let carrier = r.place_on_field(0, CARD, Some(0));
        r.place_on_field(1, "ST12-03", Some(0));
        r.attack_player(carrier, 1, false);
        if affordable {
            r.assert_prompt(SelectionKind::Hand, true, 0);
            assert_eq!(candidates(&r), ["KNIGHT"]);
            r.choose_prompt_card("KNIGHT", None).unwrap();
            assert_eq!(r.memory(), 6);
        } else {
            assert!(r.pending_selection().is_none());
            assert_eq!(r.hand_size(0), 1);
        }
    }
}

#[test]
fn inherited_prevention_requires_a_different_knightmon_text_digimon() {
    let knight = make_test_card("KNIGHT", "Knightmon");
    let mut r = start(builder(CARD).add_card(knight));
    let carrier = r.place_stack(0, &[CARD, "TEXT-5"]);
    r.place_on_field(0, "TEXT-3", Some(0));
    r.place_on_field(0, "KNIGHT", Some(0));
    let enemy = r.place_on_field(1, "TEXT-6", Some(0));
    r.battle_digimon(carrier, enemy);
    assert!(r.pending_selection().unwrap().is_optional);
    if r.pending_selection().unwrap().kind == SelectionKind::Replacement {
        r.accept_optional_trigger().unwrap();
    }
    assert_eq!(candidates(&r), ["KNIGHT"]);
    r.choose_prompt_card("KNIGHT", None).unwrap();
    assert_eq!(r.battle_area_size(0), 2);
    assert_eq!(ids(&r, &r.game.players[0].trash), ["KNIGHT"]);
}

/// Merge check (claude/mf3-opt into claude/matrix-bug-fixes-2): the owner's
/// 2026-11-24 ruling lets four "by deleting ..." clauses offer a payer that
/// can't be deleted, as their DCGO scripts do (Chuumon (EX13-027), Sukamon
/// (EX13-028), Kotemon (EX13-048), Gladimon (EX13-052);
/// `offer_unpayable_deletion_payers`). Maildramon (BT27-062) has the same
/// shape but no DCGO script yet, so it keeps CR 4.3 15-7-4: with the only
/// [Knightmon]-text payer unable to be deleted, the clause is not offered and
/// the Digimon leaves.
#[test]
fn inherited_prevention_is_not_offered_when_the_only_payer_cannot_be_deleted() {
    let knight = make_test_card("KNIGHT", "Knightmon");
    let mut r = start(builder(CARD).add_card(knight));
    let carrier = r.place_stack(0, &[CARD, "TEXT-5"]);
    let payer = r.place_on_field(0, "KNIGHT", Some(0));
    r.game.modifiers.add(
        payer,
        digimon_engine::modifiers::ModifierEntry::simple(
            digimon_engine::enums::ModifierType::CannotBeDestroyedByEffect,
            0,
            digimon_engine::enums::Expiry::Permanent,
            payer.player,
        ),
    );
    let enemy = r.place_on_field(1, "TEXT-6", Some(0));
    r.battle_digimon(carrier, enemy);
    assert!(
        r.pending_selection().is_none(),
        "no payable [Knightmon]-text Digimon: the clause is not offered"
    );
    let mut trash = ids(&r, &r.game.players[0].trash);
    trash.sort();
    assert_eq!(trash, [CARD, "TEXT-5"], "the carrier left");
    assert_eq!(r.battle_area_size(0), 1, "only the protected payer is left");
}

#[test]
fn attacking_can_use_a_knightmon_text_option_at_discount_or_decline() {
    const OPTION: &str = "TEXT-KNIGHTMON-OPTION";
    for accept in [false, true] {
        let mut r = start(
            builder(CARD)
                .from_dsl_yaml(
                    r#"
card: TEXT-KNIGHTMON-OPTION
name: Knightmon Command
kind: option
color: [black]
cost: 5
effects:
  - text: "[Main] Draw 1."
    when: main
    process:
      - draw: { of: you, count: 1 }
"#,
                )
                .unwrap()
                .hand(0, &[OPTION]),
        );
        let carrier = r.place_on_field(0, CARD, Some(0));
        r.attack_player(carrier, 1, false);
        r.assert_prompt(SelectionKind::Hand, true, 0);
        assert_eq!(candidates(&r), [OPTION]);
        if accept {
            r.choose_prompt_card(OPTION, None).unwrap();
            assert_eq!(r.memory(), 7);
            assert_eq!(ids(&r, &r.game.players[0].trash), [OPTION]);
            assert_eq!(ids(&r, &r.game.players[0].hand), ["TEXT-FILLER"]);
        } else {
            r.decline_prompt().unwrap();
            assert_eq!(r.memory(), 10);
            assert_eq!(ids(&r, &r.game.players[0].hand), [OPTION]);
        }
    }
}
