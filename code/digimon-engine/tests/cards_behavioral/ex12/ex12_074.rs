use crate::dsl_card_data::compiled;

use digimon_engine::enums::CardColor;
use digimon_engine::selection::OptionPlayResult;
use digimon_engine::selection::SelectionKind;

use super::support::{
    field_contains, hand_contains, hand_index, plain_digimon, select_first_non_pass,
    select_hand_card, tb_digimon, top_card_id, with_evo_cost, DebugRunner,
};

const CARD_ID: &str = "EX12-074";

#[test]
fn ex12_074_has_shambala_use_requirement() {
    let card = compiled(CARD_ID);
    let requirement = card
        .use_requirement
        .as_ref()
        .and_then(|pred| pred.any_field_permanent.as_ref())
        .expect("EX12-074 prints Use Req. ([Shambala] trait)");

    assert_eq!(
        requirement.predicate.trait_has.as_deref(),
        Some("Shambala"),
        "Use Req. should look for an own [Shambala] trait permanent"
    );
}

#[test]
fn ex12_074_main_moves_bottom_security_places_self_face_up_and_plays_shambala() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-074 YAML loads")
        .add_card(plain_digimon("WHITE", CardColor::White, 3, 1000))
        .add_card(plain_digimon("BOTTOM", CardColor::Yellow, 3, 3000))
        .add_card(tb_digimon("PLAY", CardColor::Yellow, 4, 5000))
        .hand(0, &[CARD_ID, "PLAY"])
        .security(0, &["BOTTOM"])
        .memory(10)
        .start();

    runner.place_on_field(0, "WHITE", Some(0));
    let option_slot = hand_index(&runner, 0, CARD_ID);
    assert_eq!(
        runner.game.play_option_from_hand(0, option_slot),
        OptionPlayResult::Pending
    );
    select_hand_card(&mut runner, 0, "PLAY");
    runner.auto_resolve().expect("resolve EX12-074 main");

    assert!(hand_contains(&runner, 0, "BOTTOM"));
    assert!(field_contains(&runner, 0, "PLAY"));
    assert_eq!(
        runner.game.players[0].security[0].card_id(&runner.game.card_data),
        CARD_ID
    );
    let security_index = runner.game.players[0].security[0].card_index;
    assert!(
        runner.game.players[0]
            .face_up_security
            .contains(&security_index),
        "EX12-074 should be face-up in bottom security"
    );
}

#[test]
fn ex12_074_face_up_security_digivolves_attacking_shambala_from_hand_cost_reduced() {
    let evo = with_evo_cost(
        tb_digimon("SHAMBALA-EVO", CardColor::Yellow, 5, 7000),
        CardColor::Yellow,
        4,
        3,
    );

    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-074 YAML loads")
        .add_card(tb_digimon("ATTACKER", CardColor::Yellow, 4, 5000))
        .add_card(evo)
        .add_card(plain_digimon("SEC", CardColor::Purple, 3, 3000))
        .hand(0, &["SHAMBALA-EVO"])
        .security(0, &[CARD_ID])
        .security(1, &["SEC"])
        .memory(5)
        .start();

    let security_index = runner.game.players[0].security[0].card_index;
    runner.game.players[0]
        .face_up_security
        .insert(security_index);
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));

    runner.attack_player(attacker, 1, false);
    select_hand_card(&mut runner, 0, "SHAMBALA-EVO");
    runner
        .auto_resolve()
        .expect("resolve EX12-074 security digivolve");

    assert_eq!(
        top_card_id(&runner, 0, attacker.index as usize),
        "SHAMBALA-EVO"
    );
}

#[test]
fn ex12_074_security_plays_low_cost_shambala_from_hand_or_trash() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-074 YAML loads")
        .add_card(plain_digimon("ATTACKER", CardColor::Purple, 4, 6000))
        .add_card(tb_digimon("SEC-PLAY", CardColor::Yellow, 4, 5000))
        .hand(1, &["SEC-PLAY"])
        .security(1, &[CARD_ID])
        .memory(0)
        .start();
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));

    runner.attack_player(attacker, 1, false);
    let view = runner
        .pending_selection_view()
        .expect("EX12-074 security effect should prompt");
    assert!(
        matches!(view.kind, SelectionKind::UnionZone { .. }),
        "security effect should use a hand-or-trash union-zone pick"
    );
    select_first_non_pass(&mut runner);
    runner
        .auto_resolve()
        .expect("resolve EX12-074 security free play");

    assert!(field_contains(&runner, 1, "SEC-PLAY"));
}

// ─── {Security} [Once Per Turn] (G-ENGINE-SECURITY-ZONE-OPT-UNTRACKED) ──────

/// P0: a face-up EX12-074 as its only security card, two [Shambala] attackers
/// and two [Shambala] digivolution targets in hand; P1: three plain security
/// cards. Returns the runner and the two attackers.
fn ex12_074_opt_board() -> (
    DebugRunner,
    digimon_engine::permanent::PermanentHandle,
    digimon_engine::permanent::PermanentHandle,
) {
    let evo = with_evo_cost(
        tb_digimon("SHAMBALA-EVO", CardColor::Yellow, 5, 7000),
        CardColor::Yellow,
        4,
        3,
    );
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-074 YAML loads")
        .add_card(tb_digimon("ATTACKER", CardColor::Yellow, 4, 5000))
        .add_card(evo)
        .add_card(plain_digimon("SEC", CardColor::Purple, 3, 3000))
        .hand(0, &["SHAMBALA-EVO", "SHAMBALA-EVO"])
        .security(0, &[CARD_ID])
        .security(1, &["SEC", "SEC", "SEC"])
        .deck(0, &["SEC"; 6])
        .deck(1, &["SEC"; 6])
        .memory(10)
        .start();
    let security_index = runner.game.players[0].security[0].card_index;
    runner.game.players[0]
        .face_up_security
        .insert(security_index);
    let a1 = runner.place_on_field(0, "ATTACKER", Some(0));
    let a2 = runner.place_on_field(0, "ATTACKER", Some(0));
    (runner, a1, a2)
}

/// CR 4.3 §15-14-1-1 / §15-14-1-2: EX12-074's {Security} [Your Turn]
/// [Once Per Turn] digivolve is used once, so a second [Shambala] attack the
/// same turn is not offered it, with a legal target still in hand.
#[test]
fn ex12_074_security_digivolve_is_offered_once_per_turn() {
    let (mut runner, a1, a2) = ex12_074_opt_board();
    runner.attack_player(a1, 1, false);
    select_hand_card(&mut runner, 0, "SHAMBALA-EVO");
    runner.auto_resolve().expect("resolve the first digivolve");
    assert_eq!(top_card_id(&runner, 0, a1.index as usize), "SHAMBALA-EVO");
    assert!(hand_contains(&runner, 0, "SHAMBALA-EVO"), "a second target remains");

    runner.attack_player(a2, 1, false);
    assert!(
        runner.game.players[0].battle_area[a2.index as usize].is_suspended,
        "the second [Shambala] attack was declared"
    );
    assert!(
        runner.pending_selection().is_none(),
        "the [Once Per Turn] is used: no second offer this turn: {:?}",
        runner.pending_selection()
    );
    assert_eq!(top_card_id(&runner, 0, a2.index as usize), "ATTACKER");
}

/// Official Q&A Q1818 / §15-14-1-4: declining the whole-effect "may" is
/// choosing not to activate it, so the next [Shambala] attack the same turn
/// is offered it again (the folded-"may" refund, for a {Security} clause).
#[test]
fn ex12_074_declined_security_digivolve_is_offered_again_same_turn_q1818() {
    let (mut runner, a1, a2) = ex12_074_opt_board();
    runner.attack_player(a1, 1, false);
    let sel = runner.pending_selection().expect("the first offer");
    assert!(sel.is_optional);
    runner
        .execute_action(0, digimon_engine::action::space::PASS)
        .expect("decline");
    runner.auto_resolve().expect("finish the first attack");
    assert_eq!(top_card_id(&runner, 0, a1.index as usize), "ATTACKER");

    runner.attack_player(a2, 1, false);
    assert!(
        runner.pending_selection().is_some(),
        "Q1818: the declined [Once Per Turn] effect is offered again"
    );
}

/// §15-14-1-5-1: the use count resets when the turn changes, so on P0's next
/// turn the [Shambala] attack is offered the digivolve again.
#[test]
fn ex12_074_security_digivolve_rearms_next_turn() {
    let (mut runner, a1, a2) = ex12_074_opt_board();
    runner.attack_player(a1, 1, false);
    select_hand_card(&mut runner, 0, "SHAMBALA-EVO");
    runner.auto_resolve().expect("resolve the first digivolve");

    runner.end_turn(); // -> P1
    runner.end_turn(); // -> P0
    assert_eq!(runner.game.turn_player(), 0);
    assert!(!runner.game.game_over);
    runner.attack_player(a2, 1, false);
    assert!(
        runner.pending_selection().is_some(),
        "a new turn re-arms the [Once Per Turn]"
    );
}

/// DCGO offers only hand cards that can actually digivolve onto the attacker at
/// the reduced cost (`CanPlayCardTargetFrame`) and opens no pick when there are
/// none, so the recording has no row for it. A Lv.6 attacker with only a Lv.5
/// [Shambala] Digimon (which needs a Lv.4) in hand therefore parks nothing.
#[test]
fn ex12_074_security_digivolve_offers_no_pick_when_no_hand_card_can_digivolve_the_attacker() {
    let needs_lv4 = with_evo_cost(
        tb_digimon("NEEDS-LV4", CardColor::Yellow, 5, 7000),
        CardColor::Yellow,
        4,
        3,
    );
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-074 YAML loads")
        .add_card(tb_digimon("ATTACKER", CardColor::Yellow, 6, 9000))
        .add_card(needs_lv4)
        .add_card(plain_digimon("SEC", CardColor::Purple, 3, 3000))
        .hand(0, &["NEEDS-LV4"])
        .security(0, &[CARD_ID])
        .security(1, &["SEC"])
        .memory(5)
        .start();
    let security_index = runner.game.players[0].security[0].card_index;
    runner.game.players[0]
        .face_up_security
        .insert(security_index);
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));

    runner.attack_player(attacker, 1, false);

    assert!(
        runner.pending_selection().is_none(),
        "no hand card can digivolve the Lv.6 attacker, so nothing is offered: {:?}",
        runner.pending_selection()
    );
    assert_eq!(top_card_id(&runner, 0, attacker.index as usize), "ATTACKER");
}

/// With both a legal and an illegal [Shambala] Digimon in hand, the pick lists
/// only the legal one.
#[test]
fn ex12_074_security_digivolve_pick_lists_only_legal_digivolutions() {
    let legal = with_evo_cost(
        tb_digimon("LEGAL-EVO", CardColor::Yellow, 5, 7000),
        CardColor::Yellow,
        4,
        3,
    );
    let illegal = with_evo_cost(
        tb_digimon("ILLEGAL-EVO", CardColor::Yellow, 5, 7000),
        CardColor::Yellow,
        3,
        3,
    );
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-074 YAML loads")
        .add_card(tb_digimon("ATTACKER", CardColor::Yellow, 4, 5000))
        .add_card(legal)
        .add_card(illegal)
        .add_card(plain_digimon("SEC", CardColor::Purple, 3, 3000))
        .hand(0, &["ILLEGAL-EVO", "LEGAL-EVO"])
        .security(0, &[CARD_ID])
        .security(1, &["SEC"])
        .memory(5)
        .start();
    let security_index = runner.game.players[0].security[0].card_index;
    runner.game.players[0]
        .face_up_security
        .insert(security_index);
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));

    runner.attack_player(attacker, 1, false);
    let view = runner
        .pending_selection_view()
        .expect("one legal digivolution exists, so the pick is offered");
    let picks: Vec<u16> = view
        .valid_action_ids
        .iter()
        .copied()
        .filter(|id| *id != digimon_engine::action::space::PASS)
        .collect();
    assert_eq!(
        picks.len(),
        1,
        "only LEGAL-EVO can digivolve the Lv.4 attacker: {picks:?}"
    );
}

// ─── [Main] "play 1 [Shambala] card from hand with the play cost reduced by 3" ─
//
// DCGO EX12_074.cs:88-92 offers `EqualsTraits("Shambala") &&
// CanPlayAsNewPermanent(cardSource, payCost: true, fixedCost: cost - 3)`
// (GameContextDeterminarion.cs:419-433: not an Option, an empty frame, and
// `CanPlayCardTargetFrame`, which refuses a play prohibition and a cost above
// `MaxMemoryCost`, CardSource.cs:1150-1158). A pick the play would then refuse is
// an illegal RL choice.

fn shambala_digimon(id: &str, play_cost: u16) -> digimon_engine::card_data::CardData {
    let mut card = tb_digimon(id, CardColor::Yellow, 5, 7000);
    card.play_cost = play_cost;
    card
}

fn shambala_tamer(id: &str) -> digimon_engine::card_data::CardData {
    let mut card = super::support::tamer(id, CardColor::Yellow);
    card.traits = vec!["Shambala".to_string()];
    card
}

/// Use EX12-074's [Main] from hand (memory 10, use cost 3 → 7 left) with a white
/// Digimon in the battle area and `hand` cards beside it; returns the hand pick's
/// offered ids (empty when no pick opened). `prepare` runs before the use.
fn main_play_offers(hand: &[&str], prepare: impl FnOnce(&mut DebugRunner)) -> (DebugRunner, Vec<String>) {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-074 YAML loads")
        .add_card(plain_digimon("WHITE", CardColor::White, 3, 1000))
        .add_card(plain_digimon("BOTTOM", CardColor::Yellow, 3, 3000))
        .add_card(shambala_digimon("SH-20", 20))
        .add_card(shambala_digimon("SH-21", 21))
        .add_card(shambala_digimon("SH-5", 5))
        .add_card(shambala_tamer("SH-TAMER"))
        .hand(0, &[CARD_ID])
        .security(0, &["BOTTOM"])
        .memory(10)
        .start();
    runner.place_on_field(0, "WHITE", Some(0));
    for id in hand {
        runner.add_to_hand(0, id);
    }
    prepare(&mut runner);
    let option_slot = hand_index(&runner, 0, CARD_ID);
    // `Pending` when the hand pick parks; with no candidate the [Main] resolves
    // straight through (the result is then not `Pending`).
    let result = runner.game.play_option_from_hand(0, option_slot);
    assert_ne!(result, OptionPlayResult::Invalid, "EX12-074 is usable");
    let offered = offered_hand_pick(&runner);
    (runner, offered)
}

fn offered_hand_pick(runner: &DebugRunner) -> Vec<String> {
    use digimon_engine::action::space::{HAND_EFFECT_START, PASS, PLAY_HAND_START};
    let Some(view) = runner.pending_selection_view() else {
        return Vec::new();
    };
    if view.kind != SelectionKind::Hand {
        return Vec::new();
    }
    let hand: Vec<String> = runner.game.players[view.selecting_player as usize]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    let mut out: Vec<String> = view
        .valid_action_ids
        .iter()
        .filter(|&&a| a != PASS)
        .filter_map(|&a| {
            [PLAY_HAND_START, HAND_EFFECT_START]
                .iter()
                .filter_map(|&s| a.checked_sub(s))
                .find(|&i| (i as usize) < hand.len() && i < 30)
        })
        .map(|i| hand[i as usize].clone())
        .collect();
    out.sort();
    out.dedup();
    out
}

/// After the use cost, 7 memory pays at most 17 (CR 4.3 memory floor −10):
/// SH-20 costs 17 after the −3 and is offered; SH-21 (18) is not.
#[test]
fn ex12_074_main_play_offers_only_shambala_cards_affordable_at_minus_3() {
    let (_runner, offered) = main_play_offers(&["SH-20", "SH-21"], |_| {});
    assert_eq!(offered, vec!["SH-20".to_string()]);
}

/// "You can't play Digimon by effects": the [Shambala] Digimon is not offered,
/// the [Shambala] Tamer still is.
#[test]
fn ex12_074_main_play_excludes_shambala_digimon_under_effect_play_prohibition() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::PlayerModifierEntry;
    let (_runner, offered) = main_play_offers(&["SH-5", "SH-TAMER"], |r| {
        r.game.modifiers.add_player_modifier(
            0,
            PlayerModifierEntry::simple(ModifierType::CannotPlayDigimonByEffect, 0, Expiry::Permanent, None, 1),
        );
    });
    assert_eq!(offered, vec!["SH-TAMER".to_string()]);
}

/// A full battle area (no empty frame): nothing can be played, no pick opens.
#[test]
fn ex12_074_main_play_offers_nothing_when_battle_area_full() {
    let (mut runner, offered) = main_play_offers(&["SH-5", "SH-TAMER"], |r| {
        r.game.rules.field_slots = r.game.players[0].battle_area.len() as u8;
    });
    assert!(offered.is_empty(), "no free frame: {offered:?}");
    runner.auto_resolve().expect("resolve EX12-074 main");
    assert!(hand_contains(&runner, 0, "SH-5") && hand_contains(&runner, 0, "SH-TAMER"));
    assert!(hand_contains(&runner, 0, "BOTTOM"), "the bottom security card was still added to hand");
    assert_eq!(
        runner.game.players[0].security[0].card_id(&runner.game.card_data),
        CARD_ID,
        "EX12-074 still became the bottom security card"
    );
}

// ─── [Security] free play of a cost ≤5 [Shambala] card from hand or trash ────
//
// DCGO EX12_074.cs:129-135 `CanPlayCondition` = `HasPlayCost && cost <= 5 &&
// [Shambala] && CanPlayAsNewPermanent(payCost: false)`, and the effect asks
// nothing when neither zone has a candidate (EX12_074.cs:139-142).

fn security_board(prepare: impl FnOnce(&mut DebugRunner)) -> DebugRunner {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-074 YAML loads")
        .add_card(plain_digimon("ATTACKER", CardColor::Purple, 4, 6000))
        .add_card(plain_digimon("P1-DIGI", CardColor::Purple, 3, 3000))
        .add_card(tb_digimon("SEC-PLAY", CardColor::Yellow, 4, 5000))
        .hand(1, &["SEC-PLAY"])
        .security(1, &[CARD_ID])
        .memory(0)
        .start();
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));
    prepare(&mut runner);
    runner.attack_player(attacker, 1, false);
    runner
}

fn union_pick_pending(runner: &DebugRunner) -> bool {
    runner
        .pending_selection_view()
        .is_some_and(|v| matches!(v.kind, SelectionKind::UnionZone { .. }))
}

/// "You can't play Digimon by effects" on the security owner: the only
/// [Shambala] card is not playable, so no hand-or-trash pick opens.
#[test]
fn ex12_074_security_play_offers_nothing_under_effect_play_prohibition() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::PlayerModifierEntry;
    let runner = security_board(|r| {
        r.game.modifiers.add_player_modifier(
            1,
            PlayerModifierEntry::simple(ModifierType::CannotPlayDigimonByEffect, 0, Expiry::Permanent, None, 1),
        );
    });
    assert!(
        !union_pick_pending(&runner),
        "a prohibited play is not offered: {:?}",
        runner.pending_selection_view()
    );
    assert!(!field_contains(&runner, 1, "SEC-PLAY"));
}

/// The security owner's battle area is full: no pick opens.
#[test]
fn ex12_074_security_play_offers_nothing_when_battle_area_full() {
    let runner = security_board(|r| {
        r.place_on_field(1, "P1-DIGI", Some(0));
        r.game.rules.field_slots = 1;
    });
    assert!(
        !union_pick_pending(&runner),
        "no free frame: nothing is offered: {:?}",
        runner.pending_selection_view()
    );
    assert!(!field_contains(&runner, 1, "SEC-PLAY"));
}
