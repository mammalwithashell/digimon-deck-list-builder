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
//! The §3-4-7 breeding-area tests (end of file) stage a Digimon in the
//! breeding area and check that its effects, and the conditions it could
//! meet, don't count unless the effect references the breeding area. Every
//! other test starts from a card placed face-down under a Tamer (§4-7-9) and
//! moves it to another area. A card moved to another area is a new card that
//! keeps none of its states (§3-1-3-1-2) and is placed by the new area's rule
//! (§3-1-3-5); a face-down card that kept its face on the field would be
//! trashed by the rule check (§17-1-3-2-4), the failure behind official Q&A
//! Q6958 (G-ENGINE-PLAY-FROM-TRASH-OF-JUST-TRASHED-SOURCE-NOT-PLAYED).

use crate::fixtures;
use crate::support::*;
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{
    encode_attack, encode_digivolve, BREEDING_TARGET, SOURCE_SELECT_START,
};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{GamePhase, PlaySource, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

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
    // An any-player field pick encodes (controller, slot) in the attack-id
    // block (the same idiom as bt24_040's `encode_any_permanent`).
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
/// 2026-10-20: the chosen Digimon, either player's per the unqualified printed
/// text, goes to its owner's security) and BT25-044 Junomon.
///
/// This test exercises only the `disposition: none` path. The
/// `disposition: cancel` (replacement-cost) route is pinned by
/// `cards_behavioral::bt24_040_protection_paid_with_opponent_digimon_places_it_in_opponent_security`
/// and `…_opponent_cost_body_is_checked_from_opponent_security_into_owner_trash`,
/// not by this test.
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

// ─── §3-4-7 breeding area ───────────────────────────────────────────────────

/// Move the stack `card_ids` (bottom first) into `player`'s breeding area.
/// Call before placing anything on that player's field (the stack is staged
/// in the battle area first, then taken out of it).
fn breed_stack(r: &mut DebugRunner, player: PlayerId, card_ids: &[&str]) {
    let staged = r.place_stack(player, card_ids);
    let perm = r.game.players[player as usize]
        .battle_area
        .remove(staged.index as usize);
    r.game.players[player as usize].breeding_area = Some(perm);
}

/// P0 with `breeding` (bottom first) in the breeding area, a `LV5_PLAIN`
/// attacker per entry of `attackers` (each `Some(card)` puts that card under
/// it), and P1 with four no-text security cards.
fn security_removal_board(
    breeding: &[&str],
    attackers: &[Option<&str>],
) -> (DebugRunner, Vec<PermanentHandle>) {
    let mut r = fixtures::builder(&[fixtures::ch03_cards()])
        .deck(0, &[FILL; 8])
        .deck(1, &[FILL; 8])
        .security(1, &["AREA_TAMER"; 4])
        .memory(0)
        .start();
    breed_stack(&mut r, 0, breeding);
    let handles = attackers
        .iter()
        .map(|under| match under {
            Some(card) => r.place_stack(0, &[card, "LV5_PLAIN"]),
            None => r.place_on_field(0, "LV5_PLAIN", Some(0)),
        })
        .collect();
    (r, handles)
}

/// Attack P1's security with `attacker`, resolve everything it triggers, and
/// return the memory P0 gained.
fn attack_and_count_memory(r: &mut DebugRunner, attacker: PermanentHandle) -> i16 {
    let before = r.memory();
    let sec_before = r.security_count(1);
    r.attack_player(attacker, 1, false);
    // The order of simultaneous memory gains doesn't change the total.
    for _ in 0..8 {
        match r.pending_selection() {
            Some(sel) if sel.kind == SelectionKind::TriggerOrder => {
                let (player, action) = (sel.selecting_player, sel.valid_action_ids[0]);
                r.execute_action(player, action).expect("order pick");
            }
            Some(_) => step_prompt(r),
            None => break,
        }
    }
    assert!(r.pending_selection().is_none(), "the attack settled");
    assert_eq!(r.security_count(1) + 1, sec_before, "one security card removed");
    r.memory() - before
}

/// §3-4-7-4 "Effects on cards in breeding areas can't trigger or activate,
/// except for effects that explicitly specify or reference breeding areas."
///
/// "[Your Turn] When your opponent's security stack is removed from, gain 1
/// memory." is printed on P0's breeding-area top card (`BRD_OBS_LV3`) and
/// inherited from the Digi-Egg under it (`BRD_OBS_EGG`, the Gigimon BT21-001
/// shape; DCGO `BT21_001.cs` requires `IsExistOnBattleAreaDigimon`). A second
/// `BRD_OBS_EGG` is under P0's battle-area attacker. One security removal
/// triggers only the battle-area copy: 1 memory, no choice of order.
#[test]
fn breeding_area_effects_without_a_breeding_reference_do_not_trigger() {
    cite!(
        "3-4-7-4",
        "6fc804d056e130e6d192ba06065b044d621aa24b516d77345c7733fdb4fa877f"
    );
    let (mut r, attackers) =
        security_removal_board(&["BRD_OBS_EGG", "BRD_OBS_LV3"], &[Some("BRD_OBS_EGG")]);
    let before = r.memory();
    r.attack_player(attackers[0], 1, false);
    assert!(
        r.pending_selection().is_none(),
        "only the battle-area inherited effect triggers, so nothing to order: {:?}",
        r.pending_selection()
    );
    assert_eq!(r.memory() - before, 1, "the battle-area copy only");
}

/// §3-4-7-4's exception: "[Breeding] [Your Turn] [Once Per Turn] When your
/// opponent's security stack is removed from, gain 2 memory."
/// (`BRD_SCOPED_EGG`, the BT20-083 / BT22-079 shape) specifies the breeding
/// area, so it triggers from under P0's breeding-area Digimon, and does not
/// trigger from under a battle-area Digimon.
#[test]
fn breeding_scoped_effect_triggers_only_in_the_breeding_area() {
    cite!(
        "3-4-7-4",
        "6fc804d056e130e6d192ba06065b044d621aa24b516d77345c7733fdb4fa877f"
    );
    let (mut r, attackers) =
        security_removal_board(&["BRD_SCOPED_EGG", FILL], &[Some("BRD_SCOPED_EGG")]);
    assert_eq!(
        attack_and_count_memory(&mut r, attackers[0]),
        2,
        "the breeding-area copy only"
    );
}

/// §3-4-7-4 for a persistent effect: "[All Turns] All of your opponent's
/// Digimon get -1000 DP." on P0's breeding-area Digimon (`BRD_AURA_LV3`)
/// doesn't apply; the same effect specifying the breeding area
/// (`BRD_AURA_SCOPED_LV3`, "[Breeding] [All Turns] ...") does.
#[test]
fn breeding_area_persistent_effect_applies_only_with_a_breeding_reference() {
    cite!(
        "3-4-7-4",
        "6fc804d056e130e6d192ba06065b044d621aa24b516d77345c7733fdb4fa877f"
    );
    for (breeding, want) in [("BRD_AURA_LV3", 7000), ("BRD_AURA_SCOPED_LV3", 6000)] {
        let mut r = fixtures::builder(&[fixtures::ch03_cards()])
            .deck(0, &[FILL; 4])
            .deck(1, &[FILL; 4])
            .memory(3)
            .start();
        breed_stack(&mut r, 0, &[breeding]);
        let target = r.place_on_field(1, "LV5_PLAIN", Some(0));
        r.game.tick_declarative_effects();
        assert_eq!(r.dp_of(target), Some(want), "{breeding} in the breeding area");
    }
}

/// §3-4-7-6 "Trigger conditions can't be met by cards in breeding areas,
/// except for effects that explicitly specify or reference breeding areas.
/// (Example: Even if your Digimon in the breeding area digivolves during your
/// turn, a "[Your Turn] When your Digimon digivolves, ..." effect doesn't
/// trigger.)"
///
/// P0's `OBS_DIGIVOLVE_MEM` reads "[All Turns] When a Digimon digivolves, gain
/// 1 memory." Digivolving P0's breeding-area Digimon into `AREA_LV4` (cost 2)
/// leaves P0 at 5 - 2 = 3 memory; the same digivolution in the battle area
/// triggers it (5 - 2 + 1 = 4).
#[test]
fn breeding_area_digivolution_does_not_meet_a_digivolve_trigger_condition() {
    cite!(
        "3-4-7-6",
        "28cf3f5f581997bd3dd8df2fdac718cd968e073c9431706ec612efceda45cd52"
    );
    for in_breeding in [true, false] {
        let mut r = fixtures::builder(&[fixtures::ch03_cards(), fixtures::ch15_04_cards()])
            .hand(0, &["AREA_LV4"])
            .deck(0, &[FILL; 4])
            .deck(1, &[FILL; 4])
            .memory(5)
            .start();
        let target = if in_breeding {
            breed_stack(&mut r, 0, &[FILL]);
            BREEDING_TARGET
        } else {
            r.place_on_field(0, FILL, Some(0)).index as u16
        };
        r.place_on_field(0, "OBS_DIGIVOLVE_MEM", Some(0));
        r.game.decode_action(encode_digivolve(0, target), 0);
        assert!(
            !hand_ids(&r, 0).contains(&"AREA_LV4".to_string()),
            "digivolved into AREA_LV4"
        );
        resolve_prompts(&mut r);
        let (want, place) = if in_breeding {
            (3, "breeding")
        } else {
            (4, "battle")
        };
        assert_eq!(r.memory(), want, "{place}-area digivolution");
    }
}

/// §3-4-7-7 "Activation conditions can't be met by cards in breeding areas,
/// except for effects that explicitly specify or reference breeding areas.
/// (Example: Even if your opponent has a Digimon in the breeding area at the
/// start of your main phase, a "[Start of Your Main Phase] If your opponent
/// has a Digimon, gain 1 memory" effect doesn't trigger.)"
///
/// The rule's example (`START_IF_OPP_DIGIMON_MEM`) with P1's only Digimon in
/// the breeding area gains nothing; with it in the battle area, 1 memory.
#[test]
fn opponents_breeding_area_digimon_does_not_meet_an_activation_condition() {
    cite!(
        "3-4-7-7",
        "9b432c5203ad07cce9b89a358a08db4ad93961731447a2d17920b8bb35890c2f"
    );
    for in_breeding in [true, false] {
        let mut r = fixtures::builder(&[fixtures::ch03_cards()])
            .deck(0, &[FILL; 4])
            .deck(1, &[FILL; 4])
            .memory(3)
            .start();
        if in_breeding {
            breed_stack(&mut r, 1, &[FILL]);
        } else {
            r.place_on_field(1, FILL, Some(0));
        }
        r.place_on_field(0, "START_IF_OPP_DIGIMON_MEM", Some(0));
        r.set_phase(GamePhase::Breeding);
        r.game.enter_main_phase();
        resolve_prompts(&mut r);
        let want = if in_breeding { 3 } else { 4 };
        assert_eq!(r.memory(), want, "P1's Digimon in breeding: {in_breeding}");
    }
}

/// §3-4-7-7 for an activated effect: "[Main] [Once Per Turn] If you have a
/// Digimon, gain 1 memory." (`MAIN_IF_DIGIMON_MEM`) with P0's only Digimon in
/// the breeding area gains nothing (whether or not its activation can be
/// declared); with it in the battle area, 1 memory.
#[test]
fn own_breeding_area_digimon_does_not_meet_a_main_activation_condition() {
    cite!(
        "3-4-7-7",
        "9b432c5203ad07cce9b89a358a08db4ad93961731447a2d17920b8bb35890c2f"
    );
    for in_breeding in [true, false] {
        let mut r = fixtures::builder(&[fixtures::ch03_cards()])
            .deck(0, &[FILL; 4])
            .deck(1, &[FILL; 4])
            .memory(3)
            .start();
        if in_breeding {
            breed_stack(&mut r, 0, &[FILL]);
        } else {
            r.place_on_field(0, FILL, Some(0));
        }
        let tamer = r.place_on_field(0, "MAIN_IF_DIGIMON_MEM", Some(0));
        if r.current_phase() != GamePhase::Main {
            r.game.enter_main_phase();
        }
        let action = field_main_action(tamer);
        if build_action_mask(&r.game, 0)[action as usize] > 0.0 {
            r.game.decode_action(action, 0);
            resolve_prompts(&mut r);
        } else {
            assert!(in_breeding, "the battle-area Digimon meets the condition");
        }
        let want = if in_breeding { 3 } else { 4 };
        assert_eq!(r.memory(), want, "P0's Digimon in breeding: {in_breeding}");
    }
}

/// §15-14-1-5-1 "The number of uses for an [X Per Turn] effect will be reset
/// in the following circumstances. The turn changed."
///
/// P0 has `OPT_SEC_MEM` ("[Your Turn] [Once Per Turn] When your opponent's
/// security stack is removed from, gain 1 memory.") in the battle area and
/// `BRD_SCOPED_EGG` ("[Breeding] [Your Turn] [Once Per Turn] ... gain 2
/// memory.") under its breeding-area Digimon. The first removal of P0's turn
/// triggers both (+3), the second neither; after P1's turn, P0's next turn
/// triggers both again, the breeding-area use included.
#[test]
fn per_turn_uses_reset_when_the_turn_changes_in_both_field_areas() {
    cite!(
        "15-14-1-5-1",
        "3a540488a0a2ce4fc2a55778faddcbdad589c95ebcce125c98e68809f61c274f"
    );
    let (mut r, attackers) = security_removal_board(&["BRD_SCOPED_EGG", FILL], &[None, None]);
    r.place_on_field(0, "OPT_SEC_MEM", Some(0));
    assert_eq!(attack_and_count_memory(&mut r, attackers[0]), 3, "first use");
    assert_eq!(
        attack_and_count_memory(&mut r, attackers[1]),
        0,
        "[Once Per Turn]"
    );
    r.pass_turn();
    resolve_prompts(&mut r);
    assert_eq!(r.turn_player(), 1);
    r.pass_turn();
    resolve_prompts(&mut r);
    assert_eq!(r.turn_player(), 0);
    if r.current_phase() != GamePhase::Main {
        r.game.enter_main_phase();
        resolve_prompts(&mut r);
    }
    assert_eq!(
        attack_and_count_memory(&mut r, attackers[0]),
        3,
        "both uses reset with the turn change"
    );
}
