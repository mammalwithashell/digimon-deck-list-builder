//! BT27-018: official Bandai card text.
//!
//! ### Standard digivolve cost circles
//!   - Red Lv.5 / cost 3
//! ### Special digivolution condition (DNA / alt-digivolve black text)
//!   [Digivolve] Lv.5 w/[Gemmon] in text: Cost 3
//!
//! ## Printed text (official)
//! ### Effect
//! ＜Rush＞ ＜Progress＞ [When Digivolving] [When Attacking] [Once Per Turn] For every 8000 DP this Digimon has, you may activate the effect below: ・Delete 1 of your opponent's lowest DP Digimon. [All Turns] [Once Per Turn] When any of your opponent's Digimon are deleted, trash their top security card.
//!
//! ### Special Play Condition
//! Assembly -5: Lv.5 × Lv.4 × Lv.3, all red w/[Gemmon] in text
//!
//! ## Official Q&A
//! - It refers to a card that contains the specified text or icon in its name, traits, effects, inherited effects, (Rule), digivolve requirements, DNA Digivolve, DigiXros requirements, Burst Digivolve, App Fusion, Link, or Assembly requirements. For example, "a card with [Knightmon] in its text" would include cards with the name [DarkKnightmon] and cards with the text [Knightmon] in their effects.

use super::bt27_gemmon_fixtures::{assemble, fixture, gemmon_text_digivolution};
use super::common::{action, candidates, start};
use digimon_engine::action::space::{encode_attack, encode_digivolve, SECURITY_TARGET};
use digimon_engine::debug_runner::make_test_card_with_level;
use digimon_engine::enums::Keyword;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;

#[test]
fn rubeusdramon_alternate_digivolution_finds_gemmon_in_requirements() {
    gemmon_text_digivolution("BT27-018", 5, 3);
}

#[test]
fn rubeusdramon_digivolution_deletes_lowest_dp_and_trashes_security() {
    let mut r = start(fixture(&["BT27-014", "BT27-018"]).hand(0, &["BT27-018"]));
    let host = r.place_stack(0, &["BT27-014"]);
    r.place_stack(1, &["ALYSION"]);
    r.place_stack(1, &["FILLER"]);
    action(&mut r, 0, encode_digivolve(0, host.index as u16));
    r.assert_prompt(SelectionKind::Replacement, true, 0);
    r.accept_optional_trigger().unwrap();
    r.assert_prompt(SelectionKind::OppField, false, 0);
    assert_eq!(candidates(&r), ["FILLER"]);
    r.choose_prompt_card("FILLER", None).unwrap();
    assert_eq!(r.game.players[1].battle_area.len(), 1);
    assert_eq!(r.security_count(1), 2);
}

#[test]
fn rubeusdramon_can_decline_deletion_then_use_it_when_attacking() {
    let mut r = start(fixture(&["BT27-014", "BT27-018"]).hand(0, &["BT27-018"]));
    let host = r.place_stack(0, &["BT27-014"]);
    r.place_stack(1, &["FILLER"]);
    action(&mut r, 0, encode_digivolve(0, host.index as u16));
    r.decline_prompt().unwrap();
    assert_eq!(r.game.players[1].battle_area.len(), 1);
    assert!(r.game.has_keyword(host, Keyword::Rush));
    assert!(r.game.has_keyword(host, Keyword::Progress));
    action(&mut r, 0, encode_attack(host.index as u16, SECURITY_TARGET));
    r.accept_optional_trigger().unwrap();
    r.choose_prompt_card("FILLER", None).unwrap();
    assert!(r.game.players[1].battle_area.is_empty());
    assert_eq!(r.security_count(1), 1);
    let later = r.place_stack(1, &["FILLER"]);
    r.game
        .delete_permanent_with_cause(later, ReplacementCause::OwnEffect);
    assert_eq!(r.security_count(1), 1);
}

#[test]
fn rubeusdramon_assembles_three_red_gemmon_text_levels() {
    let mut level_five = make_test_card_with_level("RED-5", "Level Five", 5);
    level_five.requirement_text = "[Digivolve] Lv.4 w/[Gemmon] in text: Cost 3".into();
    let mut level_four = make_test_card_with_level("RED-4", "Level Four", 4);
    level_four.inherited_text = "This Digimon with [Gemmon] in its text gains Raid.".into();
    let builder = fixture(&["BT27-018", "BT27-006"])
        .add_card(level_five)
        .add_card(level_four);
    assemble(builder, "BT27-018", ["RED-5", "RED-4", "BT27-006"]);
}

#[test]
fn rubeusdramon_at_16000_dp_recalculates_the_lowest_target_after_each_deletion() {
    let mut r = start(fixture(&["BT27-006", "BT27-018"]));
    let attacker = r.place_stack(0, &["BT27-006", "BT27-006", "BT27-018"]);
    for target in ["ICE", "FILLER", "ALYSION"] {
        r.place_stack(1, &[target]);
    }
    assert_eq!(r.effective_dp(attacker), Some(16000));
    action(
        &mut r,
        0,
        encode_attack(attacker.index as u16, SECURITY_TARGET),
    );
    r.accept_optional_trigger().unwrap();
    assert_eq!(candidates(&r), ["FILLER"]);
    r.choose_prompt_card("FILLER", None).unwrap();
    assert_eq!(candidates(&r), ["ALYSION", "ICE"]);
    r.choose_prompt_card("ALYSION", None).unwrap();
    assert_eq!(r.game.players[1].battle_area.len(), 1);
    r.assert_prompt(SelectionKind::TriggerOrder, false, 0);
    r.execute_branch(0).unwrap();
    assert_eq!(r.security_count(1), 1);
}
