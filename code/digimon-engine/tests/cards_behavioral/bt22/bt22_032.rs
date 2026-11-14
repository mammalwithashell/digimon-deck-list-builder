//! BT22-032 ShoeShoemon.
//! Printed text: [On Deletion] You may play 1 level 3 Digimon card with the
//! [Puppet] trait from your hand without paying the cost. Inherited: [When
//! Attacking] [Once Per Turn] 1 of your opponent's Digimon gets -2000 DP for
//! the turn.

use digimon_engine::action::space::{PASS, PLAY_HAND_START};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::selection::SelectionKind;

#[test]
fn bt22_032_on_deletion_may_play_level3_puppet_from_hand() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT22-032")
        .expect("BT22-032 YAML loads")
        .add_card(make_puppet_level3("PUPPET-L3"))
        .hand(0, &["PUPPET-L3"])
        .memory(0)
        .start();
    let shoe = runner.place_on_field(0, "BT22-032", Some(0));

    runner.game.delete_permanent_with_cause(
        shoe,
        digimon_engine::replacement::ReplacementCause::OpponentEffect,
    );

    assert!(
        runner.pending_is_optional(),
        "play-from-hand prompt is optional"
    );
    let view = runner
        .pending_selection_view()
        .expect("level 3 Puppet prompt");
    assert_eq!(view.kind, SelectionKind::Hand);
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("choose level 3 Puppet");
    runner.auto_resolve().expect("finish play");

    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "PUPPET-L3"),
        "selected level 3 Puppet enters battle area"
    );
}

#[test]
fn bt22_032_on_deletion_decline_keeps_hand_card() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT22-032")
        .expect("BT22-032 YAML loads")
        .add_card(make_puppet_level3("PUPPET-L3"))
        .hand(0, &["PUPPET-L3"])
        .memory(0)
        .start();
    let shoe = runner.place_on_field(0, "BT22-032", Some(0));

    runner.game.delete_permanent_with_cause(
        shoe,
        digimon_engine::replacement::ReplacementCause::OpponentEffect,
    );
    runner
        .execute_action(0, PASS)
        .expect("decline optional play");

    assert_eq!(runner.hand_size(0), 1, "declining leaves Puppet in hand");
}

fn make_puppet_level3(id: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.level = Some(3);
    card.traits = vec!["Puppet".to_string()];
    card
}

// ─── Hand-pick legality: only cards the free play can actually play ─────────
// DCGO BT22_032.cs:39-45 `IsPuppetLevel3Card` = IsExistOnHand && IsDigimon &&
// IsLevel3 && HasPuppetTraits — it carries NO `CanPlayAsNewPermanent`, so DCGO
// is looser than the rules here: it offers a card the follow-up
// `PlayPermanentCards` then refuses (a DUAL: CR 4.3 7-1-1 plays only a card with
// a play cost, and EX13-065 prints "D"; 4-6-2 — a DUAL is digivolved or used). CR 4.3 1-3-2 (an impossible action is not carried
// out) + 15-10-2-1 (choose "as many cards up to X as possible") — the pick is
// restricted to cards the play can perform, the same `CanPlayAsNewPermanent`
// check the sibling Puppet cards (BT22_042.cs:72, BT22_088.cs:44) apply. YAML:
// `can_play_from_effect_without_cost: true`.

fn make_filler_digimon(id: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.level = Some(4);
    card
}

/// Delete ShoeShoemon (plus the optional pre-placed extra permanents) and
/// return the card ids its [On Deletion] hand pick offers (empty when no hand
/// pick installs).
fn offered_on_deletion(
    runner: &mut DebugRunner,
    shoe: digimon_engine::permanent::PermanentHandle,
) -> Vec<String> {
    runner
        .game
        .delete_permanent_with_cause(shoe, digimon_engine::replacement::ReplacementCause::OpponentEffect);
    for _ in 0..4 {
        match runner.pending_kind() {
            Some(SelectionKind::Hand) => {
                let hand: Vec<String> = runner.game.players[0]
                    .hand
                    .iter()
                    .map(|c| c.card_id(&runner.game.card_data).to_string())
                    .collect();
                let view = runner.pending_selection_view().expect("hand pick");
                return view
                    .valid_action_ids
                    .iter()
                    .copied()
                    .filter(|&id| id != PASS)
                    .map(|id| hand[(id - PLAY_HAND_START) as usize].clone())
                    .collect();
            }
            Some(_) if runner.pending_is_optional() => {
                runner.accept_optional_trigger().expect("accept optional prompt")
            }
            _ => return Vec::new(),
        }
    }
    Vec::new()
}

fn shoe_runner(hand: &[&str]) -> (DebugRunner, digimon_engine::permanent::PermanentHandle) {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT22-032")
        .expect("BT22-032 YAML loads")
        .dsl_card("EX13-065")
        .expect("EX13-065 YAML loads")
        .add_card(make_puppet_level3("PUPPET-L3"))
        .add_card(make_filler_digimon("FILLER-L4"))
        .hand(0, hand)
        .memory(0)
        .start();
    let shoe = runner.place_on_field(0, "BT22-032", Some(0));
    (runner, shoe)
}

/// Sistermon Blanc (Awakened) (EX13-065) is a level 3 [Puppet] DUAL card. A DUAL
/// can't be played (CR 4.3 7-1-1: its play cost prints "D"; 4-6-2), so it is not a candidate; the plain level 3
/// Puppet still is.
#[test]
fn bt22_032_on_deletion_does_not_offer_dual_puppet_level3() {
    let (mut runner, shoe) = shoe_runner(&["EX13-065", "PUPPET-L3"]);
    let offered = offered_on_deletion(&mut runner, shoe);
    assert_eq!(
        offered,
        vec!["PUPPET-L3".to_string()],
        "the DUAL Sistermon Blanc (Awakened) (EX13-065) must not be offered for a play"
    );
}

/// Only the DUAL in hand: no candidate, nothing is played, it stays in hand.
#[test]
fn bt22_032_on_deletion_dual_only_hand_offers_no_pick() {
    let (mut runner, shoe) = shoe_runner(&["EX13-065"]);
    let offered = offered_on_deletion(&mut runner, shoe);
    assert!(offered.is_empty(), "no playable candidate: {offered:?}");
    let _ = runner.auto_resolve();
    assert!(runner.game.players[0].battle_area.is_empty());
    assert_eq!(runner.hand_size(0), 1, "EX13-065 stays in hand");
}

/// "You can't play Digimon by effects": the level 3 Puppet is not offered.
#[test]
fn bt22_032_on_deletion_offers_nothing_when_effect_play_is_prohibited() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::PlayerModifierEntry;
    let (mut runner, shoe) = shoe_runner(&["PUPPET-L3"]);
    runner.game.modifiers.add_player_modifier(
        0,
        PlayerModifierEntry::simple(ModifierType::CannotPlayDigimonByEffect, 0, Expiry::Permanent, None, 1),
    );
    let offered = offered_on_deletion(&mut runner, shoe);
    assert!(offered.is_empty(), "play prohibited ⇒ no candidate: {offered:?}");
    let _ = runner.auto_resolve();
    assert_eq!(runner.hand_size(0), 1, "the Puppet stays in hand");
}

/// A full battle area (DCGO `CanPlayAsNewPermanent` needs an empty frame): the
/// level 3 Puppet is not offered.
#[test]
fn bt22_032_on_deletion_offers_nothing_when_battle_area_full() {
    let (mut runner, shoe) = shoe_runner(&["PUPPET-L3"]);
    runner.place_on_field(0, "FILLER-L4", Some(0));
    // After ShoeShoemon leaves, the filler alone fills a 1-slot battle area.
    runner.game.rules.field_slots = 1;
    let offered = offered_on_deletion(&mut runner, shoe);
    assert!(offered.is_empty(), "full battle area ⇒ no candidate: {offered:?}");
    let _ = runner.auto_resolve();
    assert_eq!(runner.hand_size(0), 1, "the Puppet stays in hand");
}
