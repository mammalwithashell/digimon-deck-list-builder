//! Chapter 3 "Areas" (Comprehensive Rules Ver. 4.3): each area's placement
//! rule for a card that arrives in it.
//!
//! Every test asserts the RULE, never the engine's current output. A test the
//! engine fails is `#[ignore]`d with the gap id that explains it (discover then
//! pin); see `docs/RUST_ENGINE_GAPS.md` and `qa/rules/cr_obligations_status.yaml`.
//!
//! Fixture cards: `fixtures/common/*.yaml` + `fixtures/ch03/*.yaml`, one
//! synthetic card per file, authored to printed-style text (each clause's
//! `summary:` is that text).
//!
//! Every test starts from a card placed face-down under a Tamer (§4-7-9) and
//! moves it to another area. A card moved to another area is a new card that
//! keeps none of its states (§3-1-3-1-2) and is placed by the new area's rule
//! (§3-1-3-5); a face-down card that kept its face on the field would be
//! trashed by the rule check (§17-1-3-2-4), the failure behind official Q&A
//! Q6958 (G-ENGINE-PLAY-FROM-TRASH-OF-JUST-TRASHED-SOURCE-NOT-PLAYED).

use crate::fixtures;
use crate::support::*;
use digimon_engine::action::space::{encode_attack, SOURCE_SELECT_START};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{PlaySource, PlayerId};
use digimon_engine::permanent::PermanentHandle;

/// P0's `AREA_TAMER` with `stashed` face-down under it, `option` in P0's
/// hand, and a vanilla Lv.3 Digimon on P0's field (a digivolution base).
fn stash_board(stashed: &str, option: &str) -> (DebugRunner, PermanentHandle, PermanentHandle) {
    let mut r = fixtures::builder(&[fixtures::ch03_cards()])
        .hand(0, &[option])
        .deck(0, &[FILL; 4])
        .memory(5)
        .start();
    let tamer = r.place_on_field(0, "AREA_TAMER", Some(0));
    attach_face_down(&mut r, tamer, stashed);
    let base = r.place_on_field(0, FILL, Some(0));
    (r, tamer, base)
}

/// Use `option` and resolve its pick of the only card under the Tamer.
fn use_on_the_stashed_card(r: &mut DebugRunner, option: &str) {
    use_option(r, 0, option);
    let sel = r.pending_selection().expect("the pick of the card under the Tamer");
    let pick = sel
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a >= SOURCE_SELECT_START)
        .unwrap_or_else(|| panic!("the face-down card is offered: {sel:?}"));
    let player = sel.selecting_player;
    r.execute_action(player, pick).expect("pick the card under the Tamer");
    resolve_prompts(r);
}

fn find<'a>(cards: &'a [CardSource], r: &DebugRunner, card_id: &str) -> &'a CardSource {
    cards
        .iter()
        .find(|c| c.card_id(&r.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} not found"))
}

fn index_of(cards: &[CardSource], r: &DebugRunner, card_id: &str) -> usize {
    cards
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} not found"))
}

// ─── §3-6-3 ─────────────────────────────────────────────────────────────────

/// §3-6-3 "The trash is a public area. Cards are placed face-up in a stack in
/// this area."
///
/// "[Main] Trash the bottom face-down card from under 1 of your Tamers."
/// trashes the face-down `AREA_LV3`: it is face-up in the trash, so a later
/// play from the trash puts a face-up card on the field (Q6958).
///
/// The public movement observation (`Game::observe_public_movement`) watches
/// battle-area stacks only (`observation_card` over `battle_area`); it does
/// not expose the trash, so there is no trash visibility to assert there. The
/// face-down card was hidden while under the Tamer (§4-7-10), as it must be.
#[test]
fn face_down_card_trashed_from_under_a_tamer_is_face_up_in_the_trash() {
    cite!(
        "3-6-3",
        "8e35bf9d5cc0f7d42721dc0b7653eae3ea2459a5291aee2ee9a9eb24173172cc"
    );
    let (mut r, tamer, _) = stash_board("AREA_LV3", "MAIN_TRASH_UNDER");
    use_option(&mut r, 0, "MAIN_TRASH_UNDER");
    resolve_prompts(&mut r); // the Tamer pick
    assert_eq!(
        r.game.players[0].battle_area[tamer.index as usize].card_sources.len(),
        1,
        "the face-down card left the Tamer"
    );
    let trashed = find(&r.game.players[0].trash, &r, "AREA_LV3");
    assert!(!trashed.face_down, "the trashed card is face-up in the trash");
}

// ─── §3-1-3-8 ───────────────────────────────────────────────────────────────

/// P0 holds `MAIN_SOURCELESS_TO_SECURITY`; each player has one security card
/// and one sourceless `AREA_LV4` on the field. Uses the Option and picks
/// `chosen`'s `AREA_LV4`; returns both security stacks, bottom first.
fn place_sourceless_digimon_of(chosen: PlayerId) -> (Vec<String>, Vec<String>) {
    let mut r = fixtures::builder(&[fixtures::ch03_cards()])
        .hand(0, &["MAIN_SOURCELESS_TO_SECURITY"])
        .deck(0, &[FILL; 4])
        .deck(1, &[FILL; 4])
        .security(0, &["AREA_LV3"])
        .security(1, &["AREA_LV3"])
        .memory(5)
        .start();
    let own = r.place_on_field(0, "AREA_LV4", Some(0));
    let opp = r.place_on_field(1, "AREA_LV4", Some(0));
    let pick = if chosen == 0 { own } else { opp };
    use_option(&mut r, 0, "MAIN_SOURCELESS_TO_SECURITY");
    let sel = r.pending_selection().expect("the Digimon pick");
    let action = encode_attack(pick.player as u16, pick.index as u16);
    for h in [own, opp] {
        assert!(
            sel.valid_action_ids
                .contains(&encode_attack(h.player as u16, h.index as u16)),
            "either player's sourceless Digimon may be chosen: {sel:?}"
        );
    }
    let player = sel.selecting_player;
    r.execute_action(player, action).expect("pick the Digimon");
    resolve_prompts(&mut r);
    assert!(
        r.game.players[chosen as usize].battle_area.is_empty(),
        "the chosen Digimon left the field"
    );
    (
        card_ids(&r, &r.game.players[0].security),
        card_ids(&r, &r.game.players[1].security),
    )
}

/// §3-1-3-8 "When card text specifies that a card is to be placed in an area
/// without specifying which player's area, the card is placed in the area of
/// the player who owns the card."
///
/// "[Main] Place 1 Digimon with no digivolution cards as the bottom security
/// card." names no player's security stack, so the chosen Digimon goes to the
/// bottom of its OWNER's stack: your own Digimon to yours, the opponent's to
/// the opponent's, never the user's by default. Text that does name the stack
/// ("... at the top of your security stack", `MAIN_SRC_TO_SECURITY`) is
/// authored `of: you`; text that names none is authored `to_owner: true`.
/// Production shapes: BT24-040 Venusmon's leave-replacement cost (owner ruling
/// 2026-10-20: the chosen Digimon goes to its owner's security, "other
/// Digimon" being either player's per Q5781) and BT25-044 Junomon.
#[test]
fn card_placed_in_an_unnamed_security_stack_goes_to_its_owners_stack() {
    cite!(
        "3-1-3-8",
        "6e64bb68d43209eb187b242f20fcd67fca67dc817b5dffa7448efa34ebb97b92"
    );
    let (own_security, opp_security) = place_sourceless_digimon_of(0);
    assert_eq!(
        own_security,
        owned(&["AREA_LV4", "AREA_LV3"]),
        "your own Digimon goes to the bottom of YOUR security stack"
    );
    assert_eq!(opp_security, owned(&["AREA_LV3"]));

    let (own_security, opp_security) = place_sourceless_digimon_of(1);
    assert_eq!(
        opp_security,
        owned(&["AREA_LV4", "AREA_LV3"]),
        "the opponent's Digimon goes to the bottom of THEIR security stack"
    );
    assert_eq!(
        own_security,
        owned(&["AREA_LV3"]),
        "the user's security stack is not the default destination"
    );
}

// ─── §3-7-2 ─────────────────────────────────────────────────────────────────

/// §3-7-2 "The security stack is a private area. Cards in this area are placed
/// face-down and spread out so that the number of cards can be seen." (§3-1-3-5
/// gives the same example: a card placed at the security stack with no face
/// specified is placed face down.)
///
/// "[Main] Place 1 card from under 1 of your Tamers at the top of your security
/// stack." moves the face-down `AREA_LV3` to the security stack. The engine
/// records a security card's face by `Player::face_up_security` (absent =
/// face-down), so the card is face-down there; the face-down state it had as a
/// digivolution card is not carried along (§3-1-3-1-2): the per-card flag is
/// the field-stack state, and a stale one would follow the card out of the
/// security stack.
#[test]
fn face_down_card_placed_in_security_is_face_down_there() {
    cite!(
        "3-7-2",
        "cdc05aa98bea875decb0e8006e9382602211756f91fafff3fb02ed33d53101a9"
    );
    let (mut r, tamer, _) = stash_board("AREA_LV3", "MAIN_SRC_TO_SECURITY");
    // A stale face-up record for the card (as if an earlier stint in the
    // security stack had left it): the card that now enters is a new card
    // (§3-1-3-1-2), placed face down.
    let stale = r.game.players[0].battle_area[tamer.index as usize].card_sources[0].card_index;
    r.game.players[0].face_up_security.insert(stale);
    let security_before = r.game.players[0].security.len();
    use_on_the_stashed_card(&mut r, "MAIN_SRC_TO_SECURITY");
    assert_eq!(
        r.game.players[0].battle_area[tamer.index as usize].card_sources.len(),
        1,
        "the face-down card left the Tamer"
    );
    let security = &r.game.players[0].security;
    assert_eq!(security.len(), security_before + 1);
    let top = security.last().expect("a security card");
    assert_eq!(top.card_id(&r.game.card_data), "AREA_LV3", "placed at the top");
    assert!(
        !r.game.players[0].face_up_security.contains(&top.card_index),
        "the security card is face-down, a stale face-up record notwithstanding"
    );
    assert!(
        !top.face_down,
        "the card keeps no face-down state from under the Tamer (§3-1-3-1-2)"
    );
}

// ─── §3-2-2 / §3-3-2 ────────────────────────────────────────────────────────

/// §3-2-2 "The deck is a private area. Cards are placed face-down in this
/// area."
///
/// "[Main] Return 1 card from under 1 of your Tamers to the bottom of the
/// deck." returns the face-down `AREA_LV3`. The deck's face is the area's own
/// (every deck card is face-down); the card keeps no face-down state from
/// under the Tamer (§3-1-3-1-2), so wherever it goes next -- even under a
/// permanent by a step that does not set a face -- it is not counted as a
/// face-down card (a "face-down card under your Tamer" reference, §4-7-9).
#[test]
fn face_down_card_returned_to_the_deck_keeps_no_face_down_state() {
    cite!(
        "3-2-2",
        "65a3b5591b7fbf7f3ae2a0b2b36ad006a5fa09f324147eaf2d2493b5f1a17dad"
    );
    let (mut r, tamer, _) = stash_board("AREA_LV3", "MAIN_SRC_TO_DECK");
    use_on_the_stashed_card(&mut r, "MAIN_SRC_TO_DECK");
    assert_eq!(
        r.game.players[0].battle_area[tamer.index as usize].card_sources.len(),
        1,
        "the face-down card left the Tamer"
    );
    let bottom = r.game.players[0].deck.first().expect("a deck card");
    assert_eq!(bottom.card_id(&r.game.card_data), "AREA_LV3", "at the bottom");
    assert!(
        !bottom.face_down,
        "the card keeps no face-down state from under the Tamer (§3-1-3-1-2)"
    );
}

/// §3-3-2 "The Digi-egg deck is a private area. Cards are placed face-down in
/// this area."
///
/// The same Option on a face-down Digi-Egg card under the Tamer: it goes to the
/// bottom of the Digi-Egg deck and keeps no face-down state (§3-1-3-1-2).
#[test]
fn face_down_digi_egg_returned_to_the_digi_egg_deck_keeps_no_face_down_state() {
    cite!(
        "3-3-2",
        "7602375d087f0672f32f7a31ab5aa49248726bf8ae9356001ac48dabda789b48"
    );
    let (mut r, tamer, _) = stash_board("AREA_EGG", "MAIN_SRC_TO_DECK");
    use_on_the_stashed_card(&mut r, "MAIN_SRC_TO_DECK");
    assert_eq!(
        r.game.players[0].battle_area[tamer.index as usize].card_sources.len(),
        1,
        "the face-down card left the Tamer"
    );
    let bottom = r.game.players[0]
        .digitama_deck
        .first()
        .expect("a Digi-Egg deck card");
    assert_eq!(bottom.card_id(&r.game.card_data), "AREA_EGG", "at the bottom");
    assert!(
        !bottom.face_down,
        "the card keeps no face-down state from under the Tamer (§3-1-3-1-2)"
    );
}

// ─── §3-4-2 ─────────────────────────────────────────────────────────────────

/// §3-4-2 "The field is a public area. Cards are placed face-up in this area."
/// (§3-5-3: the hand is the owner's private area, which they may always look
/// at -- a hand card carries no face-down placement.)
///
/// "[Main] Return 1 card from under 1 of your Tamers to the hand." returns the
/// face-down `AREA_LV3`; it is then played from the hand. The new Digimon is
/// face-up and the rule check does not trash it as a face-down card on the
/// field (§17-1-3-2-4).
#[test]
fn face_down_card_returned_to_hand_then_played_is_face_up_on_the_field() {
    cite!(
        "3-4-2",
        "ed5470b75927579dcf142852519fc337a2919a54a6819e333ee621b13e09c9b8"
    );
    cite!(
        "3-5-3",
        "61786f30b22566aa865de13557c7a68aaa3b02b6608374874ef258e938e3c126"
    );
    let (mut r, _, _) = stash_board("AREA_LV3", "MAIN_SRC_TO_HAND");
    use_on_the_stashed_card(&mut r, "MAIN_SRC_TO_HAND");
    let in_hand = find(&r.game.players[0].hand, &r, "AREA_LV3");
    assert!(!in_hand.face_down, "a hand card carries no face-down placement");
    let hand_index = index_of(&r.game.players[0].hand, &r, "AREA_LV3");
    r.play(0, hand_index).expect("AREA_LV3 is played");
    resolve_prompts(&mut r);
    let played = r.game.players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&r.game.card_data) == "AREA_LV3")
        .expect("the played Digimon survived the rule check (§17-1-3-2-4)");
    assert!(!played.top_card().face_down, "it is face-up on the field");
    assert!(
        !trash_ids(&r, 0).contains(&"AREA_LV3".to_string()),
        "it was not trashed: {:?}",
        trash_ids(&r, 0)
    );
}

/// §3-4-2, through digivolution: the face-down `AREA_LV4` returned from under
/// the Tamer to the hand is digivolved onto P0's red Lv.3. The new top card is
/// face-up and the Digimon is not trashed by the rule check (§17-1-3-2-4).
#[test]
fn face_down_card_digivolved_into_is_a_face_up_top_card() {
    cite!(
        "3-4-2",
        "ed5470b75927579dcf142852519fc337a2919a54a6819e333ee621b13e09c9b8"
    );
    let (mut r, _, base) = stash_board("AREA_LV4", "MAIN_SRC_TO_HAND");
    use_on_the_stashed_card(&mut r, "MAIN_SRC_TO_HAND");
    let hand_index = index_of(&r.game.players[0].hand, &r, "AREA_LV4");
    assert!(
        r.game
            .digivolve_from_hand(0, hand_index, base.index as usize, PlaySource::ByHand),
        "AREA_LV4 digivolves onto the red Lv.3"
    );
    resolve_prompts(&mut r);
    let digivolved = r.game.players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&r.game.card_data) == "AREA_LV4")
        .expect("the digivolved Digimon survived the rule check (§17-1-3-2-4)");
    assert!(!digivolved.top_card().face_down, "its new top card is face-up");
    assert!(
        !trash_ids(&r, 0).contains(&"AREA_LV4".to_string()),
        "it was not trashed: {:?}",
        trash_ids(&r, 0)
    );
}
