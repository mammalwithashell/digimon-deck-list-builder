//! BT26-100 Dark Field — Option, Purple/Black, use cost 3, Titan/TS.
//!
//! While you have no face-up security cards, you can ignore this card's color
//! requirements.
//! {Security} [All Turns] All of your [Titan] trait Digimon gain <Blocker>.
//! While you have a Digimon with [Plutomon] or [Titamon] in its name, they
//! also get +3000 DP.
//! [Main] Add your bottom security card to the hand and place this card face
//! up as the bottom security card. Then, you may play 1 level 4 or lower
//! [Titan] trait card from your hand or trash without paying the cost.
//! [Security] You may play 1 level 4 or lower [Titan] trait Digimon card from
//! your hand or trash without paying the cost.
//!
//! DCGO: BT26/Purple/BT26_100.cs.

use super::support::*;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, Keyword};

const CARD_ID: &str = "BT26-100";

fn builder() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-100")
        .add_card(filler("FILLER"))
        .add_card(digimon(
            "TITAN4",
            "Titan Four",
            CardColor::Purple,
            4,
            5,
            &["Titan"],
        ))
        .add_card(digimon(
            "TITAN5",
            "Titan Five",
            CardColor::Purple,
            5,
            7,
            &["Titan"],
        ))
        .add_card(digimon(
            "PLUTO",
            "Plutomon",
            CardColor::Purple,
            6,
            13,
            &["Titan"],
        ))
        .add_card(digimon("PLAIN", "Plain", CardColor::Red, 4, 5, &[]))
        .add_card(digimon("ATK", "Attacker", CardColor::Red, 5, 7, &[]))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(5)
}

fn use_option(r: &mut DebugRunner) {
    push_hand(r, 0, CARD_ID);
    let idx = r.game.players[0].hand.len() - 1;
    r.game.enter_main_phase();
    let _ = r.game.play_option_from_hand(0, idx);
}

#[test]
fn bt26_100_main_swaps_bottom_security_then_plays_titan_from_trash() {
    let mut r = builder().security(0, &["FILLER", "TITAN5"]).start();
    r.set_first_player(0);
    push_trash(&mut r, 0, "TITAN4");
    push_trash(&mut r, 0, "TITAN5");
    use_option(&mut r);
    // Bottom security (index 0) went to the hand; Dark Field is the new bottom, face up.
    assert_eq!(hand_ids(&r, 0), vec!["FILLER".to_string()]);
    let bottom = &r.game.players[0].security[0];
    assert_eq!(bottom.card_id(&r.game.card_data), CARD_ID);
    assert!(r.game.players[0]
        .face_up_security
        .contains(&bottom.card_index));
    let v = r.pending_selection_view().expect("optional play pick");
    assert!(v.is_optional);
    assert_eq!(non_pass(&r).len(), 1, "only the level-4 [Titan]");
    pick_first(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 0), vec!["TITAN4".to_string()]);
    assert_eq!(r.memory(), 5 - 3, "only the use cost was paid");
}

#[test]
fn bt26_100_cannot_ignore_colors_with_a_face_up_security_card() {
    let mut r = builder().security(0, &["FILLER", "TITAN5"]).start();
    r.set_first_player(0);
    let idx = r.game.players[0].security[1].card_index;
    r.game.players[0].face_up_security.insert(idx);
    push_hand(&mut r, 0, CARD_ID);
    r.game.enter_main_phase();
    let res = r.game.play_option_from_hand(0, 0);
    assert_eq!(
        format!("{res:?}"),
        "Invalid",
        "no purple/black card in play"
    );
    assert_eq!(r.hand_size(0), 1);
}

#[test]
fn bt26_100_face_up_security_grants_blocker_and_dp_with_plutomon() {
    let mut r = builder().security(0, &[CARD_ID, "FILLER"]).start();
    r.set_first_player(0);
    let idx = r.game.players[0].security[0].card_index;
    r.game.players[0].face_up_security.insert(idx);
    let titan = r.place_on_field(0, "TITAN4", Some(0));
    let plain = r.place_on_field(0, "PLAIN", Some(0));
    r.game.tick_declarative_effects();
    assert!(r.game.has_keyword(titan, Keyword::Blocker));
    assert!(!r.game.has_keyword(plain, Keyword::Blocker));
    assert_eq!(r.effective_dp(titan), Some(4000), "no Plutomon yet");
    let pluto = r.place_on_field(0, "PLUTO", Some(0));
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(titan), Some(7000));
    assert_eq!(r.effective_dp(pluto), Some(9000));
    assert_eq!(r.effective_dp(plain), Some(4000));
}

#[test]
fn bt26_100_face_down_security_grants_nothing() {
    let mut r = builder().security(0, &[CARD_ID, "FILLER"]).start();
    r.set_first_player(0);
    let titan = r.place_on_field(0, "TITAN4", Some(0));
    r.game.tick_declarative_effects();
    assert!(!r.game.has_keyword(titan, Keyword::Blocker));
}

#[test]
fn bt26_100_security_check_plays_titan_digimon_from_hand() {
    let mut r = builder().security(1, &[CARD_ID]).start();
    r.set_first_player(0);
    push_hand(&mut r, 1, "TITAN4");
    push_hand(&mut r, 1, "TITAN5");
    let atk = r.place_on_field(0, "ATK", Some(0));
    let _ = r.attack_player(atk, 1, false);
    let v = r.pending_selection_view().expect("security play pick");
    assert!(v.is_optional);
    assert_eq!(non_pass(&r).len(), 1);
    pick_first(&mut r, 1);
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 1).contains(&"TITAN4".to_string()));
}

// ─── Candidate filters: only cards the effect can actually play ─────────────
//
// DCGO BT26_100.cs:76-80 ([Main]) and :139-143 ([Security]) both end in
// `cardSource.HasPlayCost && CardEffectCommons.CanPlayAsNewPermanent(
// cardSource, payCost: false, activateClass)`, applied to the hand and the
// trash alike. CR 4.3 1-3-11-1: a card can be declared for play only if it is
// able to be played; 1-3-2: an impossible action is not carried out. So a
// [Titan] card is offered only with a free battle-area slot, no "can't play
// Digimon by effects" lock and — for a trash copy — no "can't play cards from
// the trash" lock. (Digi-Eggs are already excluded by the printed "Digimon"
// kind on [Security] and by the digimon/tamer kind gate on [Main].)

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

/// [Main] fixture: P0 will use Dark Field with a Lv.4 [Titan] in the trash.
fn main_fixture() -> DebugRunner {
    let mut r = builder().security(0, &["FILLER", "TITAN5"]).start();
    r.set_first_player(0);
    push_trash(&mut r, 0, "TITAN4");
    r
}

/// [Security] fixture: Dark Field is P1's only security card, P1 holds a
/// Lv.4 [Titan] Digimon in hand and another in trash, P0 has an attacker.
fn security_fixture() -> (DebugRunner, digimon_engine::permanent::PermanentHandle) {
    let mut r = builder().security(1, &[CARD_ID]).start();
    r.set_first_player(0);
    push_hand(&mut r, 1, "TITAN4");
    push_trash(&mut r, 1, "TITAN4");
    let atk = r.place_on_field(0, "ATK", Some(0));
    (r, atk)
}

/// Regression guard: the [Main] offers the trash [Titan].
#[test]
fn bt26_100_main_offers_playable_titan_from_trash() {
    let mut r = main_fixture();
    use_option(&mut r);
    assert_eq!(union_offered(&r), vec!["Trash:TITAN4".to_string()]);
}

/// [Main] with no free battle-area slot: nothing can be played → none offered.
#[test]
fn bt26_100_main_offers_nothing_when_battle_area_full() {
    let mut r = main_fixture();
    r.place_on_field(0, "FILLER", Some(0));
    r.game.rules.field_slots = r.game.players[0].battle_area.len() as u8;
    use_option(&mut r);
    let offered = union_offered(&r);
    assert!(offered.is_empty(), "no free slot → nothing can be chosen (got {offered:?})");
}

/// [Main] under "you can't play Digimon by effects": none offered.
#[test]
fn bt26_100_main_offers_nothing_under_digimon_effect_play_prohibition() {
    let mut r = main_fixture();
    add_player_lock(&mut r, 0, digimon_engine::enums::ModifierType::CannotPlayDigimonByEffect);
    use_option(&mut r);
    let offered = union_offered(&r);
    assert!(offered.is_empty(), "Digimon effect plays are locked (got {offered:?})");
}

/// Regression guard: the [Security] offers the hand and the trash copy.
#[test]
fn bt26_100_security_offers_titan_from_hand_and_trash() {
    let (mut r, atk) = security_fixture();
    let _ = r.attack_player(atk, 1, false);
    assert_eq!(
        union_offered(&r),
        vec!["Hand:TITAN4".to_string(), "Trash:TITAN4".to_string()]
    );
}

/// [Security] with no free battle-area slot: none offered.
#[test]
fn bt26_100_security_offers_nothing_when_battle_area_full() {
    let (mut r, atk) = security_fixture();
    r.place_on_field(1, "FILLER", Some(0));
    r.game.rules.field_slots = r.game.players[1].battle_area.len() as u8;
    let _ = r.attack_player(atk, 1, false);
    let offered = union_offered(&r);
    assert!(offered.is_empty(), "no free slot → nothing can be chosen (got {offered:?})");
}

/// [Security] under "you can't play cards from the trash": only the hand copy.
#[test]
fn bt26_100_security_skips_trash_copy_under_cannot_play_from_trash() {
    let (mut r, atk) = security_fixture();
    add_player_lock(&mut r, 1, digimon_engine::enums::ModifierType::CannotPlayFromTrash);
    let _ = r.attack_player(atk, 1, false);
    assert_eq!(union_offered(&r), vec!["Hand:TITAN4".to_string()]);
}
