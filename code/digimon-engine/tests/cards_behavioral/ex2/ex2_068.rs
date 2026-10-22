//! EX2-068 High-Speed Plug-In D — Option, Blue, Cost 2.
//!
//! # Card text (official Bandai DB — data/card_bundles/EX2-068.md; card image)
//!
//! While you have a Tamer in play, you may use this card without meeting its
//! color requirements.
//! [Main] For the turn, 1 of your Digimon gains <Jamming> (This Digimon can't be
//! deleted in battles against Security Digimon) and can't be blocked by your
//! opponent's Digimon.
//!
//! Security Effect: [Security] <Draw 1>. (Draw 1 card from your deck.) Then,
//! add this card to your hand.
//!
//! # DCGO C# reference
//! `Assets/Scripts/CardEffect/EX2/Blue/EX2_068.cs`:
//! - `IgnoreColorConditionClass` while `GetBattleAreaPermanents().Count(IsTamer)
//!   >= 1` → `use_requirement` over an own battle-area Tamer.
//! - OptionSkill: if you have a Digimon, `SelectPermanentEffect(maxCount 1,
//!   canNoSelect: false)` over your Digimon → `GainJamming(UntilEachTurnEnd)` +
//!   `GainCanNotBeBlocked(defenderCondition: null, UntilEachTurnEnd)`.
//! - SecuritySkill: `DrawClass(1)` then `AddThisCardToHand`.
//!
//! # Patterns this test covers
//! - D3 conditional Option color-requirement bypass (Tamer present) + negative
//! - mandatory own-Digimon pick → keyword grant + attacker-side "can't be
//!   blocked" (new `CannotBeBlocked` modifier) for the turn, with expiry
//! - [Security] draw then add this card to hand

use digimon_dsl::compiled::{CompiledCardKind, CompiledColor};
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{encode_attack, PLAY_HAND_START};
use digimon_engine::card_data::CardData;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, Expiry, GamePhase, Keyword, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::OptionPlayResult;

const CARD_ID: &str = "EX2-068";

fn digimon(id: &str, color: CardColor, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![color];
    c.level = Some(4);
    c.dp = Some(dp);
    c.play_cost = 5;
    c
}

fn tamer(id: &str, color: CardColor) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.colors = vec![color];
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX2-068 YAML must parse and compile")
        .add_card(digimon("BLUE-A", CardColor::Blue, 3000))
        .add_card(digimon("BLUE-B", CardColor::Blue, 3000))
        .add_card(digimon("RED-DIGI", CardColor::Red, 3000))
        .add_card(digimon("BLK", CardColor::Red, 9000))
        .add_card(digimon("SEC-BIG", CardColor::Red, 9000))
        .add_card(tamer("RED-TAMER", CardColor::Red))
        .add_card(digimon("FILL", CardColor::White, 1000))
}

fn hand_ids(r: &DebugRunner, p: u8) -> Vec<String> {
    r.game
        .player(p)
        .hand
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

fn hand_index(r: &DebugRunner, p: u8, id: &str) -> usize {
    r.game
        .player(p)
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == id)
        .expect("card in hand")
}

fn pick(r: &mut DebugRunner, target: PermanentHandle) {
    let action = encode_attack(0, target.index as u16);
    r.execute_action(0, action).expect("pick");
    let _ = r.auto_resolve();
}

/// Use EX2-068 from hand and pick `target` as the Digimon.
fn use_on(r: &mut DebugRunner, target: PermanentHandle) {
    let idx = hand_index(r, 0, CARD_ID);
    assert_eq!(
        r.game.play_option_from_hand(0, idx),
        OptionPlayResult::Pending
    );
    let v = r.pending_selection_view().expect("Digimon pick");
    assert!(!v.is_optional, "\"1 of your Digimon\" is not a may");
    // OwnField picks encode `encode_attack(0, slot)` (side implied by kind).
    let action = encode_attack(0, target.index as u16);
    assert!(
        v.valid_action_ids.contains(&action),
        "{target:?} is not a legal pick (valid {:?})",
        v.valid_action_ids
    );
    r.execute_action(0, action).expect("pick");
    let _ = r.auto_resolve();
}

// ─── Structure ──────────────────────────────────────────────────────────────

#[test]
fn ex2_068_is_blue_option_cost_2_with_tamer_bypass() {
    let r = builder().start();
    let c = r.compiled_card(CARD_ID).expect("registered");
    assert_eq!(c.kind, CompiledCardKind::Option);
    assert_eq!(c.color, vec![CompiledColor::Blue]);
    assert_eq!(c.cost, Some(2));
    assert!(c.use_requirement.is_some(), "Tamer-gated color bypass");
}

// ─── Color-requirement bypass ───────────────────────────────────────────────

#[test]
fn ex2_068_bypass_usable_off_color_with_a_tamer() {
    let mut r = builder()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"; 4])
        .memory(10)
        .start();
    r.place_on_field(0, "RED-TAMER", Some(0));
    r.place_on_field(0, "RED-DIGI", Some(0));
    r.game.enter_main_phase();
    assert_eq!(build_action_mask(&r.game, 0)[PLAY_HAND_START as usize], 1.0);
}

#[test]
fn ex2_068_no_bypass_without_a_tamer() {
    let mut r = builder()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"; 4])
        .memory(10)
        .start();
    r.place_on_field(0, "RED-DIGI", Some(0));
    r.game.enter_main_phase();
    assert_eq!(
        build_action_mask(&r.game, 0)[PLAY_HAND_START as usize],
        0.0,
        "no blue permanent and no Tamer → color requirement unmet"
    );
}

// ─── [Main] ─────────────────────────────────────────────────────────────────

fn main_runner() -> (DebugRunner, PermanentHandle, PermanentHandle) {
    let mut r = builder()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"; 4])
        .deck(1, &["FILL"; 4])
        .security(1, &["SEC-BIG", "SEC-BIG", "SEC-BIG"])
        .memory(10)
        .start();
    let a = r.place_on_field(0, "BLUE-A", Some(0));
    let b = r.place_on_field(0, "BLUE-B", Some(0));
    r.game.enter_main_phase();
    (r, a, b)
}

#[test]
fn ex2_068_main_offers_each_own_digimon_and_grants_only_the_pick() {
    let (mut r, a, b) = main_runner();
    let idx = hand_index(&r, 0, CARD_ID);
    assert_eq!(r.game.play_option_from_hand(0, idx), OptionPlayResult::Pending);
    let v = r.pending_selection_view().unwrap();
    assert_eq!(v.valid_action_ids.len(), 2, "both of your Digimon are legal");
    assert!(!v.is_optional);
    pick(&mut r, b);
    let (a2, b2) = (a, b);
    assert!(r.game.has_keyword(b2, Keyword::Jamming));
    assert!(r.game.modifiers.has(b2, ModifierType::CannotBeBlocked));
    assert!(!r.game.has_keyword(a2, Keyword::Jamming));
    assert!(!r.game.modifiers.has(a2, ModifierType::CannotBeBlocked));
    assert_eq!(r.memory(), 8, "use cost 2");
    assert!(r.game.player(0).trash.iter().any(|c| c.card_id(&r.game.card_data) == CARD_ID));
}

#[test]
fn ex2_068_chosen_digimon_cannot_be_blocked() {
    let (mut r, a, _b) = main_runner();
    let blk = r.place_on_field(1, "BLK", Some(1));
    r.game
        .modifiers
        .grant_keyword(blk, Keyword::Blocker, Expiry::Permanent, 1);
    use_on(&mut r, a);
    let sec_before = r.security_count(1);
    let _ = r.attack_player(a, 1, false);
    assert_ne!(r.current_phase(), GamePhase::BlockTiming);
    let _ = r.auto_resolve();
    assert_eq!(r.security_count(1), sec_before - 1, "attack reached security");
}

#[test]
fn ex2_068_unchosen_digimon_can_still_be_blocked() {
    let (mut r, a, b) = main_runner();
    let blk = r.place_on_field(1, "BLK", Some(1));
    r.game
        .modifiers
        .grant_keyword(blk, Keyword::Blocker, Expiry::Permanent, 1);
    use_on(&mut r, a);
    let res = r.attack_player(b, 1, false);
    assert_eq!(res, AttackResult::InProgress);
    assert_eq!(r.current_phase(), GamePhase::BlockTiming);
}

#[test]
fn ex2_068_chosen_digimon_survives_a_bigger_security_digimon() {
    let (mut r, a, b) = main_runner();
    use_on(&mut r, a);
    let _ = r.attack_player(a, 1, false);
    let _ = r.auto_resolve();
    assert!(
        r.game.player(0).battle_area.iter().any(|p| p.top_card().card_id(&r.game.card_data) == "BLUE-A"),
        "<Jamming>: not deleted in battle with a Security Digimon"
    );
    // Control: the unchosen Digimon loses the same battle.
    let _ = r.attack_player(b, 1, false);
    let _ = r.auto_resolve();
    assert!(
        !r.game.player(0).battle_area.iter().any(|p| p.top_card().card_id(&r.game.card_data) == "BLUE-B"),
        "without <Jamming> the 3000 DP Digimon is deleted by a 9000 DP Security Digimon"
    );
}

#[test]
fn ex2_068_grants_expire_at_end_of_turn() {
    let (mut r, a, _b) = main_runner();
    use_on(&mut r, a);
    assert!(r.game.modifiers.has(a, ModifierType::CannotBeBlocked));
    r.end_turn();
    let _ = r.auto_resolve();
    assert!(!r.game.has_keyword(a, Keyword::Jamming));
    assert!(!r.game.modifiers.has(a, ModifierType::CannotBeBlocked));
}

#[test]
fn ex2_068_main_with_no_digimon_still_resolves() {
    let mut r = builder()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"; 4])
        .memory(10)
        .start();
    r.place_on_field(0, "RED-TAMER", Some(0));
    r.game.enter_main_phase();
    let idx = hand_index(&r, 0, CARD_ID);
    let res = r.game.play_option_from_hand(0, idx);
    assert_ne!(res, OptionPlayResult::Invalid);
    let _ = r.auto_resolve();
    assert!(r.pending_selection_view().is_none());
    assert_eq!(r.memory(), 8);
}

// ─── [Security] <Draw 1>, then add this card to the hand ────────────────────

#[test]
fn ex2_068_security_draws_then_adds_itself_to_hand() {
    let mut r = builder()
        .security(1, &[CARD_ID])
        .deck(0, &["FILL"])
        .deck(1, &["FILL", "BLUE-B"])
        .memory(0)
        .start();
    let atk = r.place_on_field(0, "RED-DIGI", Some(0));
    let _ = r.attack_player(atk, 1, false);
    let _ = r.auto_resolve();
    let hand = hand_ids(&r, 1);
    assert_eq!(hand.len(), 2, "{hand:?}");
    assert!(hand.contains(&CARD_ID.to_string()));
    assert!(hand.contains(&"BLUE-B".to_string()), "drew the top card");
    assert!(r.game.player(1).trash.is_empty());
}
