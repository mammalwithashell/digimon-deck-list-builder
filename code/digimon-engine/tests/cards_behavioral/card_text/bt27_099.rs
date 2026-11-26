//! Source: `data/card_bundles/BT27-099.md` (printed conditions, effects, and official Q&A).
//!
//! ### Standard digivolve cost circles
//!   - (none — DigiEgg / Lv.2 base, or DNA/special only)
//!
//! ## Printed text (official)
//! ### Effect
//! When effects trash this card from the security stack, activate its [Security] effects. [Main] 1 of your opponent's Digimon gets -6000 DP for the turn. Then, add your top security card to the hand and place this card as the top security card.
//!
//! ### Security Effect
//! [Security] Place 1 of your opponent's Digimon with the lowest DP as their top security card.
//!
//! ## Official Q&A
//! - No, it is not immediately deleted at this point in time. All activated effects are resolved, after which a rule check occurs. At that point in time, all Digimon with 0 DP are deleted simultaneously.

const CARD: &str = "BT27-099";

#[test]
fn a_card_effect_trashing_surtr_brander_from_security_activates_its_security_effect() {
    use digimon_engine::enums::EffectTiming;
    let mut r = start(
        builder(CARD)
            .dsl_card("BT27-043")
            .unwrap()
            .security(0, &[CARD]),
    );
    let attacker = r.place_on_field(0, "BT27-043", Some(0));
    r.place_on_field(1, "TEXT-3", Some(0));
    r.place_on_field(1, "TEXT-5", Some(0));
    action(&mut r, 0, encode_attack(attacker.index as u16, 14));
    r.accept_optional_trigger().unwrap();
    let pick = r.pending_selection_view().unwrap();
    let discard = pick
        .effect_choices
        .unwrap()
        .into_iter()
        .find(|choice| choice.timing == Some(EffectTiming::OnDiscardSecurity))
        .unwrap();
    action(&mut r, 0, discard.action_id);
    assert_eq!(candidates(&r), ["TEXT-3"]);
    r.choose_prompt_card("TEXT-3", None).unwrap();
    if r.pending_is_optional() {
        r.decline_prompt().unwrap();
    }
    assert_eq!(r.security_count(0), 0);
    assert_eq!(ids(&r, &r.game.players[0].trash), [CARD]);
    assert_eq!(r.battle_area_size(1), 1);
    assert_eq!(r.security_count(1), 5);
    assert_eq!(
        r.game.players[1]
            .trash
            .last()
            .unwrap()
            .card_id(&r.game.card_data),
        "TEXT-3"
    );
}

use super::common::*;
use digimon_engine::action::space::{encode_attack, PLAY_HAND_START};

#[test]
fn main_reduces_dp_and_replaces_the_top_security_after_adding_it_to_hand() {
    let mut r = start(builder(CARD).hand(0, &[CARD]));
    r.place_on_field(0, "TEXT-4", Some(0));
    let target = r.place_on_field(1, "TEXT-5", Some(0));
    action(&mut r, 0, PLAY_HAND_START);
    assert_eq!(candidates(&r), ["TEXT-5"]);
    r.choose_prompt_card("TEXT-5", None).unwrap();
    assert_eq!(r.effective_dp(target), Some(1000));
    assert_eq!(ids(&r, &r.game.players[0].hand), ["TEXT-FILLER"]);
    assert_eq!(r.security_count(0), 5);
    assert_eq!(
        r.game.players[0]
            .security
            .last()
            .unwrap()
            .card_id(&r.game.card_data),
        CARD
    );
}

#[test]
fn zero_dp_rule_check_starts_after_surtr_branders_security_exchange() {
    use digimon_engine::selection::SelectionKind;

    let mut r = start(
        builder(CARD)
            .from_dsl_yaml(
                r#"
card: FIXTURE-VICTIM
name: Rule check observer
kind: digimon
level: 5
dp: 6000
color: [black]
cost: 5
effects:
  - kind: replacement
    trigger: when_would_be_deleted
    optional: true
    active_when: { is_source: true }
    process:
      - cancel_replacement: {}
"#,
            )
            .unwrap()
            .hand(0, &[CARD]),
    );
    r.place_on_field(0, "TEXT-4", Some(0));
    let victim = r.place_on_field(1, "FIXTURE-VICTIM", Some(0));
    action(&mut r, 0, PLAY_HAND_START);
    r.assert_prompt(SelectionKind::OppField, false, 0);
    assert_eq!(candidates(&r), ["FIXTURE-VICTIM"]);
    r.choose_prompt_card("FIXTURE-VICTIM", None).unwrap();
    r.assert_prompt(SelectionKind::Replacement, true, 1);
    assert_eq!(r.effective_dp(victim), Some(0));
    assert_eq!(ids(&r, &r.game.players[0].hand), ["TEXT-FILLER"]);
    assert_eq!(ids(&r, &r.game.players[0].security).last().unwrap(), CARD);
    r.decline_prompt().unwrap();
    assert_eq!(r.battle_area_size(1), 0);
    assert_eq!(ids(&r, &r.game.players[1].trash), ["FIXTURE-VICTIM"]);
}

#[test]
fn security_check_places_lowest_dp_digimon_on_top_of_its_owners_security() {
    let mut r = start(builder(CARD).security(0, &[CARD]));
    let attacker = r.place_on_field(1, "TEXT-5", Some(0));
    r.place_on_field(1, "TEXT-3", Some(0));
    next_turn(&mut r);
    action(&mut r, 1, encode_attack(attacker.index as u16, 14));
    assert_eq!(candidates(&r), ["TEXT-3"]);
    r.choose_prompt_card("TEXT-3", None).unwrap();
    assert_eq!(
        r.game.players[1]
            .security
            .last()
            .unwrap()
            .card_id(&r.game.card_data),
        "TEXT-3"
    );
    assert_eq!(r.battle_area_size(1), 1);
}
