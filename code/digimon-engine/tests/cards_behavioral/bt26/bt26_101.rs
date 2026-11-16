//! BT26-101 Cross Arts — Option, White, cost 4 (ADAMAS / TS).
//!
//! <Use Req. ([TS] trait)> [Main] If you have a Tamer with [Dan Yuki] or [Kanan
//! Yuki] in its name, all of your [TS] trait Digimon gain <Blocker> and +3000
//! DP until your opponent's turn ends. Then, activate 1 of the effects below:
//! ・Delete 1 of your opponent's Digimon with as much DP as 1 of your [TS] trait
//!   Digimon or less. ・1 of your [TS] trait Digimon unsuspends.
//! [Security] You may play 1 play cost 4 or lower [TS] trait card from your hand
//! or trash without paying the cost.
//!
//! DCGO: BT26/White/BT26_101.cs.

use super::support::*;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, Keyword};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT26-101";

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-101")
        .add_card(filler("FILLER"))
        .add_card(tamer("DAN", "Dan Yuki", CardColor::White, &["TS"]))
        .add_card(tamer("OTHER", "Someone", CardColor::White, &["TS"]))
        .add_card(digimon("TS5", "Ts Five", CardColor::Red, 5, 7, &["TS"]))
        .add_card(digimon("PLAIN5", "Plain Five", CardColor::Red, 5, 7, &[]))
        .add_card(digimon("OPP6", "Opp Six", CardColor::Blue, 6, 9, &[]))
        .add_card(digimon("OPP8", "Opp Eight", CardColor::Blue, 8, 9, &[]))
        .add_card(digimon("TS3", "Ts Three", CardColor::Red, 3, 3, &["TS"]))
        .add_card(digimon(
            "TS9",
            "Ts Nine Cost",
            CardColor::Red,
            5,
            9,
            &["TS"],
        ))
        .add_card(digimon("ATK", "Attacker", CardColor::Blue, 5, 7, &[]))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .security(0, &["FILLER"; 3])
        .security(1, &["FILLER"; 3])
        .memory(10)
        .start();
    r.set_first_player(0);
    r
}

fn use_arts(r: &mut DebugRunner) {
    push_hand(r, 0, CARD_ID);
    let idx = r.game.players[0].hand.len() - 1;
    let _ = r.game.play_option_from_hand(0, idx);
}

/// Answer an effect-choice prompt by its 0-based option index.
fn choose(r: &mut DebugRunner, i: usize) {
    let v = r.pending_selection_view().expect("effect choice");
    let mut opts: Vec<u16> = v
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != digimon_engine::action::space::PASS)
        .collect();
    opts.sort();
    r.execute_action(0, opts[i]).unwrap();
}

fn suspended(r: &DebugRunner, h: PermanentHandle) -> bool {
    r.game.players[h.player as usize].battle_area[h.index as usize].is_suspended
}

#[test]
fn bt26_101_with_dan_yuki_ts_digimon_gain_blocker_and_3000_then_delete() {
    let mut r = setup();
    r.place_on_field(0, "DAN", Some(0));
    let ts = r.place_on_field(0, "TS5", Some(0));
    let plain = r.place_on_field(0, "PLAIN5", Some(0));
    r.place_on_field(1, "OPP6", Some(0));
    r.place_on_field(1, "OPP8", Some(0));
    use_arts(&mut r);
    assert_eq!(r.effective_dp(ts), Some(8000));
    assert!(r.game.has_keyword(ts, Keyword::Blocker));
    assert_eq!(r.effective_dp(plain), Some(5000), "not [TS]");
    assert!(!r.game.has_keyword(plain, Keyword::Blocker));
    choose(&mut r, 0); // delete
    pick_first(&mut r, 0); // compare: TS5 (8000) is the only [TS] Digimon
                           // Both OPP6 (6000) and OPP8 (8000) are <= 8000.
    let v = r.pending_selection_view().expect("delete pick");
    assert_eq!(
        v.valid_action_ids
            .iter()
            .filter(|&&a| a != digimon_engine::action::space::PASS)
            .count(),
        2
    );
    pick_first(&mut r, 0);
    assert_eq!(r.game.players[1].battle_area.len(), 1);
    // The buff lasts through the opponent's turn.
    r.end_turn();
    assert_eq!(r.effective_dp(ts), Some(8000));
}

#[test]
fn bt26_101_without_named_tamer_no_buff_and_compare_uses_printed_dp() {
    let mut r = setup();
    r.place_on_field(0, "OTHER", Some(0));
    let ts = r.place_on_field(0, "TS5", Some(0));
    r.place_on_field(1, "OPP6", Some(0));
    use_arts(&mut r);
    assert_eq!(r.effective_dp(ts), Some(5000));
    assert!(!r.game.has_keyword(ts, Keyword::Blocker));
    choose(&mut r, 0);
    pick_first(&mut r, 0);
    // OPP6 (6000) > 5000 → nothing to delete.
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.game.players[1].battle_area.len(), 1);
}

#[test]
fn bt26_101_unsuspend_branch() {
    let mut r = setup();
    r.place_on_field(0, "OTHER", Some(0));
    let ts = r.place_on_field(0, "TS5", Some(0));
    r.game.players[0].battle_area[ts.index as usize].is_suspended = true;
    use_arts(&mut r);
    choose(&mut r, 1);
    pick_first(&mut r, 0);
    assert!(!suspended(&r, ts));
}

#[test]
fn bt26_101_security_plays_cost_four_or_lower_ts_from_trash_free() {
    let mut r = setup();
    let atk = r.place_on_field(1, "ATK", Some(0));
    push_trash(&mut r, 0, "TS3");
    push_trash(&mut r, 0, "TS9");
    push_hand(&mut r, 0, "PLAIN5");
    let idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == CARD_ID)
        .unwrap();
    let next = r.game.next_card_index();
    r.game.players[0]
        .security
        .push(digimon_engine::card_source::CardSource::new(idx, 0, next));
    r.set_first_player(1);
    let mem = r.memory();
    r.attack_player(atk, 0, false);
    let v = r.pending_selection_view().expect("security play prompt");
    assert!(v.is_optional, "'you may play'");
    drain_first(&mut r);
    let _ = r.auto_resolve();
    assert!(
        field_ids(&r, 0).contains(&"TS3".to_string()),
        "{:?}",
        field_ids(&r, 0)
    );
    assert!(
        trash_ids(&r, 0).contains(&"TS9".to_string()),
        "cost 5 not eligible"
    );
    assert_eq!(r.memory(), mem, "without paying the cost");
}

#[test]
fn bt26_101_buff_is_continuous_over_later_ts_digimon() {
    let mut r = setup();
    r.place_on_field(0, "DAN", Some(0));
    r.place_on_field(0, "TS5", Some(0));
    use_arts(&mut r);
    choose(&mut r, 1);
    pick_first(&mut r, 0);
    // A [TS] Digimon entering during the window is covered (DCGO player effect).
    let late = r.place_on_field(0, "TS3", None);
    r.game.tick_declarative_effects();
    assert!(r.game.has_keyword(late, Keyword::Blocker));
    assert_eq!(r.effective_dp(late), Some(3000 + 3000));
    // Expires when the opponent's turn ends.
    r.end_turn();
    r.end_turn();
    assert!(!r.game.has_keyword(late, Keyword::Blocker));
    assert_eq!(r.effective_dp(late), Some(3000));
}

// ─── [Security] candidate filter: only cards the effect can actually play ───
//
// DCGO BT26_101.cs:169-171 — `CanPlayCondition` = `HasPlayCost &&
// GetCostItself <= 4 && HasTSTraits && CardEffectCommons.CanPlayAsNewPermanent(
// cardSource, payCost: false, activateClass)`, for the hand and the trash.
// CR 4.3 1-3-11-1: a card can be declared for play only if it is able to be
// played; 1-3-2: an impossible action is not carried out. So a [TS] card is
// offered only with a free battle-area slot and no lock on playing its kind
// by effects (and, for the trash copy, no "can't play cards from the trash").

/// The union-pick candidates as sorted "Zone:card_id" strings (empty when no
/// union pick is pending).
fn union_offered(r: &DebugRunner) -> Vec<String> {
    use digimon_engine::owned_card_choice::CardChoiceOperation;
    use digimon_engine::selection::SelectionKind;
    let Some(pending) = r.game.pending_selection.as_ref() else {
        return Vec::new();
    };
    if !matches!(pending.kind, SelectionKind::UnionZone { .. }) {
        return Vec::new();
    }
    let owner = pending
        .card_choice
        .as_ref()
        .expect("union pick owns a card-choice page");
    let mut out: Vec<String> = owner
        .view(&r.game, pending.selecting_player)
        .rows
        .into_iter()
        .filter(|row| matches!(row.operation, CardChoiceOperation::CommitCard))
        .map(|row| format!("{:?}:{}", row.zone.expect("zone"), row.card_id.unwrap_or_default()))
        .collect();
    out.sort();
    out
}

fn add_player_lock(r: &mut DebugRunner, p: u8, modifier: digimon_engine::enums::ModifierType) {
    use digimon_engine::enums::Expiry;
    use digimon_engine::modifiers::PlayerModifierEntry;
    r.game
        .modifiers
        .add_player_modifier(p, PlayerModifierEntry::simple(modifier, 0, Expiry::Permanent, None, p));
}

/// Cross Arts on top of P0's security, a cost-3 [TS] Tamer in P0's hand, a
/// cost-3 [TS] Digimon in P0's trash, and a P1 attacker on P1's turn.
fn security_fixture() -> (DebugRunner, PermanentHandle) {
    let mut r = setup();
    let atk = r.place_on_field(1, "ATK", Some(0));
    push_hand(&mut r, 0, "OTHER");
    push_trash(&mut r, 0, "TS3");
    let idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == CARD_ID)
        .unwrap();
    let next = r.game.next_card_index();
    r.game.players[0]
        .security
        .push(digimon_engine::card_source::CardSource::new(idx, 0, next));
    r.set_first_player(1);
    (r, atk)
}

/// Regression guard: the [TS] Tamer in hand and the [TS] Digimon in trash
/// are both offered.
#[test]
fn bt26_101_security_offers_ts_tamer_and_digimon() {
    let (mut r, atk) = security_fixture();
    r.attack_player(atk, 0, false);
    assert_eq!(
        union_offered(&r),
        vec!["Hand:OTHER".to_string(), "Trash:TS3".to_string()]
    );
}

/// No free battle-area slot: none offered.
#[test]
fn bt26_101_security_offers_nothing_when_battle_area_full() {
    let (mut r, atk) = security_fixture();
    r.place_on_field(0, "FILLER", Some(0));
    r.game.rules.field_slots = r.game.players[0].battle_area.len() as u8;
    r.attack_player(atk, 0, false);
    let offered = union_offered(&r);
    assert!(offered.is_empty(), "no free slot → nothing can be chosen (got {offered:?})");
}

/// "You can't play Tamers by effects": the [TS] Tamer is not offered.
#[test]
fn bt26_101_security_skips_tamer_under_tamer_effect_play_prohibition() {
    let (mut r, atk) = security_fixture();
    add_player_lock(&mut r, 0, digimon_engine::enums::ModifierType::CannotPlayTamerByEffect);
    r.attack_player(atk, 0, false);
    assert_eq!(union_offered(&r), vec!["Trash:TS3".to_string()]);
}

/// "You can't play cards from the trash": the trash copy is dropped.
#[test]
fn bt26_101_security_skips_trash_card_under_cannot_play_from_trash() {
    let (mut r, atk) = security_fixture();
    add_player_lock(&mut r, 0, digimon_engine::enums::ModifierType::CannotPlayFromTrash);
    r.attack_player(atk, 0, false);
    assert_eq!(union_offered(&r), vec!["Hand:OTHER".to_string()]);
}
