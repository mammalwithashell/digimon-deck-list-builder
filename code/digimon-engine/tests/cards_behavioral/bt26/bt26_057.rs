//! BT26-057 Bearcatmon // Penetrate Blow — DUAL (Digimon Lv.5 Black/Red
//! DP 8000 // Option Black use cost 4). Traits: Beastkin, Glowing Dawn,
//! BEATBREAK. Form: Ultimate. Attribute: Vaccine.
//! Digivolve: Black Lv.4 / 4; Red Lv.4 / 4; [Digivolve] Lv.4 w/[Glowing Dawn]
//! trait: Cost 3.
//!
//! [When Digivolving] By trashing the bottom face-down card from under any of
//! your Tamers, until your opponent's turn ends, their Digimon effects don't
//! affect this Digimon, and it gets +3000 DP.
//! [All Turns] [Once Per Turn] When attack targets change or effects trash
//! cards from under your Tamers, this Digimon may unsuspend.
//! Option: <Use Req. ([Glowing Dawn] trait)> [Main] <De-Digivolve 1> 1 of your
//! opponent's Digimon. Then, give 1 of their Digimon "[Start of Your Main
//! Phase] This Digimon attacks." until their turn ends. <Arts Digivolve>.
//! Official Q&A: the effect can be given, but if the Digimon isn't affected
//! by effects when it would trigger, it doesn't trigger.
//! Owner ruling 2026-10-20: "the bottom face-down card" is the LOWEST
//! face-down card, even with face-up cards below it.
//!
//! DCGO: BT26/Red/BT26_057.cs.

use super::support::*;
use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::{encode_attack, encode_digivolve, PASS, PLAY_HAND_START};
use digimon_engine::card_data::EvoCost;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectSourceKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::OptionPlayResult;

const CARD_ID: &str = "BT26-057";

/// Test-only "[On Play] By trashing the bottom face-down card from under any
/// of your Tamers, <Draw 1>" — an EFFECT trashing from under a Tamer.
const FD_TRASHER_YAML: &str = r#"
card: T-FDTRASH
name: FDTrasher
kind: digimon
level: 3
color: [white]
cost: 3
dp: 3000
traits: []
effects:
  - when: on_play
    summary: "[On Play] Trash the bottom face-down card from under one of your Tamers"
    process:
      - trash_bottom_face_down_source_under_tamer: { of: you }
"#;

/// Test-only opponent Digimon "[On Play] 1 of your opponent's Digimon gets
/// -5000 DP for the turn" — an opponent DIGIMON effect.
const OPP_DEBUFF_YAML: &str = r#"
card: T-DEBUFF
name: Debuffer
kind: digimon
level: 3
color: [red]
cost: 3
dp: 3000
traits: []
effects:
  - when: on_play
    summary: "[On Play] 1 of your opponent's Digimon gets -5000 DP for the turn"
    process:
      - select_opponent_permanent:
          bind_as: t
          filter: { kind: digimon }
          prompt: "-5000 DP"
      - add_dp_modifier: { target: t, value: -5000, expiry: end_of_turn }
"#;

fn setup() -> DebugRunner {
    let mut lv3 = digimon("OPP3", "Opp3", CardColor::Red, 3, 3, &[]);
    lv3.dp = Some(3000);
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-057")
        .from_dsl_yaml(FD_TRASHER_YAML)
        .expect("trasher")
        .from_dsl_yaml(OPP_DEBUFF_YAML)
        .expect("debuffer")
        .add_card(filler("FILLER"))
        .add_card(tamer("GDT", "GD Tamer", CardColor::Purple, &["Glowing Dawn"]))
        .add_card(tamer("PT", "Plain Tamer", CardColor::Red, &[]))
        .add_card(digimon("GD4", "GD Lv4", CardColor::Yellow, 4, 5, &["Glowing Dawn"]))
        .add_card(digimon("PLAIN4", "Plain Lv4", CardColor::Yellow, 4, 5, &[]))
        .add_card(digimon("BLK4", "Black Lv4", CardColor::Black, 4, 5, &[]))
        .add_card(digimon("RED4", "Red Lv4", CardColor::Red, 4, 5, &[]))
        .add_card(digimon("OPP4", "Opp4", CardColor::Red, 4, 5, &[]))
        .add_card(lv3)
        .add_card(digimon("ATK", "Attacker", CardColor::Red, 5, 7, &[]))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .security(0, &["FILLER"; 3])
        .security(1, &["FILLER"; 3])
        .memory(6)
        .start();
    r.set_first_player(0);
    r
}

fn suspended(r: &DebugRunner, h: PermanentHandle) -> bool {
    r.game.players[h.player as usize].battle_area[h.index as usize].is_suspended
}

fn set_suspended(r: &mut DebugRunner, h: PermanentHandle) {
    r.game.players[h.player as usize].battle_area[h.index as usize].is_suspended = true;
}

/// Stack a Tamer `tamer_id` over sources described bottom-first
/// (`true` = face-down FILLER, `false` = face-up FILLER).
fn tamer_with_stack(r: &mut DebugRunner, p: u8, tamer_id: &str, flags: &[bool]) -> PermanentHandle {
    let mut ids: Vec<&str> = vec!["FILLER"; flags.len()];
    ids.push(tamer_id);
    let t = r.place_stack(p, &ids);
    let stack = &mut r.game.players[p as usize].battle_area[t.index as usize].card_sources;
    for (s, &fd) in stack.iter_mut().zip(flags.iter()) {
        s.face_down = fd;
    }
    t
}

/// Resolve T-FDTRASH's [On Play] for `x`'s controller: it trashes the bottom
/// face-down card from under that player's (only) Tamer — the Tamer pick is
/// answered here.
fn effect_trash_under_tamer(r: &mut DebugRunner, x: PermanentHandle) {
    fire(r, EffectTiming::OnPlay, x);
    pick_first(r, x.player); // the Tamer whose face-down card is trashed
}

fn immune_to_opp_digimon(r: &DebugRunner, h: PermanentHandle) -> bool {
    r.game
        .permanent_is_unaffected_by_effect(h, 1 - h.player, EffectSourceKind::Digimon)
}

// ─── Structural ─────────────────────────────────────────────────────────────

#[test]
fn bt26_057_dual_metadata() {
    let r = setup();
    let c = r.compiled_card(CARD_ID).unwrap();
    assert_eq!(c.level, Some(5));
    assert_eq!(c.dp, Some(8000));
    let dbg = format!("{:?}", c);
    assert!(dbg.contains("ArtsDigivolve"), "<Arts Digivolve>");
    assert!(dbg.contains("Glowing Dawn"));
    assert!(dbg.contains("BEATBREAK"));
}

// ─── Alt digivolve ──────────────────────────────────────────────────────────

#[test]
fn bt26_057_digivolves_from_a_glowing_dawn_level_4_for_3() {
    let mut r = setup();
    r.game.enter_main_phase();
    let host = r.place_on_field(0, "GD4", Some(0));
    push_hand(&mut r, 0, CARD_ID);
    let action = encode_digivolve(0, host.index as u16);
    assert_eq!(build_action_mask(&r.game, 0)[action as usize], 1.0);
    let mem = r.memory();
    r.game.decode_action(action, 0);
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, host), CARD_ID);
    assert_eq!(r.memory(), mem - 3, "[Glowing Dawn] path costs 3");
}

#[test]
fn bt26_057_alt_digivolve_needs_the_glowing_dawn_trait() {
    let mut r = setup();
    r.game.enter_main_phase();
    // Yellow Lv.4 without [Glowing Dawn]: no Black/Red circle, no GD path.
    let host = r.place_on_field(0, "PLAIN4", Some(0));
    push_hand(&mut r, 0, CARD_ID);
    let action = encode_digivolve(0, host.index as u16);
    assert_eq!(build_action_mask(&r.game, 0)[action as usize], 0.0);
}

// ─── [When Digivolving] ─────────────────────────────────────────────────────

#[test]
fn bt26_057_wd_trashes_face_down_card_for_immunity_and_plus_3000() {
    let mut r = setup();
    let t = tamer_with_face_down(&mut r, 0, "GDT", 1);
    r.game.enter_main_phase();
    let host = r.place_on_field(0, "GD4", Some(0));
    push_hand(&mut r, 0, CARD_ID);
    r.game.decode_action(encode_digivolve(0, host.index as u16), 0);
    r.accept_optional_trigger().expect("optional cost");
    pick_first(&mut r, 0); // the Tamer
    let _ = r.auto_resolve();
    assert_eq!(sources(&r, t), 0, "face-down card trashed");
    assert_eq!(r.effective_dp(host), Some(11000), "+3000 DP");
    assert!(immune_to_opp_digimon(&r, host));
    assert!(
        !r.game
            .permanent_is_unaffected_by_effect(host, 0, EffectSourceKind::Digimon),
        "only the opponent's Digimon effects"
    );
}

#[test]
fn bt26_057_wd_immunity_blocks_opponent_digimon_effect() {
    let mut r = setup();
    tamer_with_face_down(&mut r, 0, "GDT", 1);
    let b = r.place_on_field(0, CARD_ID, Some(0));
    fire(&mut r, EffectTiming::WhenDigivolving, b);
    r.accept_optional_trigger().expect("accept");
    pick_first(&mut r, 0);
    let _ = r.auto_resolve();
    r.pass_turn(); // → P1
    let d = r.place_on_field(1, "T-DEBUFF", Some(0));
    fire(&mut r, EffectTiming::OnPlay, d);
    let _ = r.auto_resolve();
    assert_eq!(
        r.effective_dp(b),
        Some(11000),
        "the opponent's Digimon effect doesn't affect it"
    );
}

#[test]
fn bt26_057_wd_lasts_until_the_end_of_the_opponents_turn() {
    let mut r = setup();
    tamer_with_face_down(&mut r, 0, "GDT", 1);
    let b = r.place_on_field(0, CARD_ID, Some(0));
    fire(&mut r, EffectTiming::WhenDigivolving, b);
    r.accept_optional_trigger().expect("accept");
    pick_first(&mut r, 0);
    let _ = r.auto_resolve();
    r.pass_turn();
    assert_eq!(r.effective_dp(b), Some(11000), "opponent's turn");
    assert!(immune_to_opp_digimon(&r, b));
    r.pass_turn();
    assert_eq!(r.effective_dp(b), Some(8000), "expired");
    assert!(!immune_to_opp_digimon(&r, b));
}

#[test]
fn bt26_057_wd_declined_does_nothing() {
    let mut r = setup();
    let t = tamer_with_face_down(&mut r, 0, "GDT", 1);
    let b = r.place_on_field(0, CARD_ID, Some(0));
    fire(&mut r, EffectTiming::WhenDigivolving, b);
    r.decline_optional_trigger().expect("decline");
    let _ = r.auto_resolve();
    assert_eq!(sources(&r, t), 1);
    assert_eq!(r.effective_dp(b), Some(8000));
    assert!(!immune_to_opp_digimon(&r, b));
}

#[test]
fn bt26_057_wd_needs_a_face_down_card_under_a_tamer() {
    let mut r = setup();
    tamer_with_stack(&mut r, 0, "GDT", &[false]); // face-up only
    let b = r.place_on_field(0, CARD_ID, Some(0));
    fire(&mut r, EffectTiming::WhenDigivolving, b);
    let _ = r.auto_resolve();
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.effective_dp(b), Some(8000));
}

#[test]
fn bt26_057_wd_trashes_the_lowest_face_down_card_above_a_face_up_one() {
    // Owner ruling 2026-10-20: bottom face-down card = lowest face-down card.
    let mut r = setup();
    let t = tamer_with_stack(&mut r, 0, "GDT", &[false, true, true]);
    let b = r.place_on_field(0, CARD_ID, Some(0));
    fire(&mut r, EffectTiming::WhenDigivolving, b);
    r.accept_optional_trigger().expect("accept");
    pick_first(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(
        face_down_flags(&r, t),
        vec![false, true],
        "the face-up card stays; the lowest face-down card is trashed"
    );
    assert_eq!(r.effective_dp(b), Some(11000));
}

#[test]
fn bt26_057_wd_cannot_use_an_opponents_tamer() {
    let mut r = setup();
    tamer_with_face_down(&mut r, 1, "GDT", 1);
    let b = r.place_on_field(0, CARD_ID, Some(0));
    fire(&mut r, EffectTiming::WhenDigivolving, b);
    let _ = r.auto_resolve();
    assert!(r.game.pending_selection.is_none(), "only your Tamers");
    assert_eq!(r.effective_dp(b), Some(8000));
}

// ─── [All Turns] [Once Per Turn] may unsuspend ──────────────────────────────

#[test]
fn bt26_057_effect_trash_from_under_your_tamer_lets_it_unsuspend() {
    let mut r = setup();
    tamer_with_face_down(&mut r, 0, "GDT", 1);
    let b = r.place_on_field(0, CARD_ID, Some(0));
    set_suspended(&mut r, b);
    let x = r.place_on_field(0, "T-FDTRASH", Some(0));
    effect_trash_under_tamer(&mut r, x);
    r.accept_optional_trigger().expect("may unsuspend");
    let _ = r.auto_resolve();
    assert!(!suspended(&r, b));
}

#[test]
fn bt26_057_unsuspend_is_optional() {
    let mut r = setup();
    tamer_with_face_down(&mut r, 0, "GDT", 1);
    let b = r.place_on_field(0, CARD_ID, Some(0));
    set_suspended(&mut r, b);
    let x = r.place_on_field(0, "T-FDTRASH", Some(0));
    effect_trash_under_tamer(&mut r, x);
    r.decline_optional_trigger().expect("decline");
    let _ = r.auto_resolve();
    assert!(suspended(&r, b));
}

#[test]
fn bt26_057_unsuspend_triggers_in_the_opponents_turn_too() {
    let mut r = setup();
    tamer_with_face_down(&mut r, 0, "GDT", 1);
    let b = r.place_on_field(0, CARD_ID, Some(0));
    r.pass_turn(); // → P1's turn ([All Turns])
    set_suspended(&mut r, b);
    // An own effect run in the opponent's turn (fired directly).
    let x = r.place_on_field(0, "T-FDTRASH", Some(0));
    effect_trash_under_tamer(&mut r, x);
    r.accept_optional_trigger().expect("may unsuspend");
    let _ = r.auto_resolve();
    assert!(!suspended(&r, b));
}

#[test]
fn bt26_057_opponents_tamer_trash_does_not_trigger() {
    let mut r = setup();
    tamer_with_face_down(&mut r, 1, "GDT", 1);
    let b = r.place_on_field(0, CARD_ID, Some(0));
    set_suspended(&mut r, b);
    let x = r.place_on_field(1, "T-FDTRASH", Some(0));
    effect_trash_under_tamer(&mut r, x);
    let _ = r.auto_resolve();
    assert!(r.game.pending_selection.is_none());
    assert!(suspended(&r, b), "only YOUR Tamers");
}

#[test]
fn bt26_057_unsuspended_bearcatmon_is_not_offered() {
    let mut r = setup();
    tamer_with_face_down(&mut r, 0, "GDT", 2);
    let b = r.place_on_field(0, CARD_ID, Some(0));
    let x = r.place_on_field(0, "T-FDTRASH", Some(0));
    effect_trash_under_tamer(&mut r, x);
    let _ = r.auto_resolve();
    assert!(r.game.pending_selection.is_none(), "nothing to unsuspend");
    // The once-per-turn was not spent: a later trigger still works.
    set_suspended(&mut r, b);
    effect_trash_under_tamer(&mut r, x);
    r.accept_optional_trigger().expect("still available");
    let _ = r.auto_resolve();
    assert!(!suspended(&r, b));
}

#[test]
fn bt26_057_unsuspend_is_once_per_turn() {
    let mut r = setup();
    tamer_with_face_down(&mut r, 0, "GDT", 3);
    let b = r.place_on_field(0, CARD_ID, Some(0));
    set_suspended(&mut r, b);
    let x = r.place_on_field(0, "T-FDTRASH", Some(0));
    effect_trash_under_tamer(&mut r, x);
    r.accept_optional_trigger().expect("first");
    let _ = r.auto_resolve();
    set_suspended(&mut r, b);
    effect_trash_under_tamer(&mut r, x);
    let _ = r.auto_resolve();
    assert!(r.game.pending_selection.is_none());
    assert!(suspended(&r, b), "OPT spent");
    r.pass_turn();
    set_suspended(&mut r, b);
    effect_trash_under_tamer(&mut r, x);
    r.accept_optional_trigger().expect("lockout cleared");
    let _ = r.auto_resolve();
    assert!(!suspended(&r, b));
}

#[test]
fn bt26_057_attack_target_change_lets_it_unsuspend() {
    // A <Blocker> block changes the attack target (§ attack target switch).
    let mut r = setup();
    let b = r.place_on_field(0, CARD_ID, Some(0));
    set_suspended(&mut r, b);
    let atk = r.place_on_field(0, "ATK", Some(0));
    let blk = r.place_on_field(1, "OPP4", Some(0));
    r.game.modifiers.grant_keyword(
        blk,
        digimon_engine::enums::Keyword::Blocker,
        digimon_engine::enums::Expiry::Permanent,
        1,
    );
    r.attack_player(atk, 1, false);
    r.game
        .resolve_selection(1, encode_attack(0, blk.index as u16))
        .expect("P1 blocks");
    r.accept_optional_trigger()
        .expect("attack target changed → may unsuspend");
    let _ = r.auto_resolve();
    assert!(!suspended(&r, b));
}

// ─── Option face ────────────────────────────────────────────────────────────

fn use_option(r: &mut DebugRunner) {
    push_hand(r, 0, CARD_ID);
    let idx = r.game.players[0].hand.len() - 1;
    let _ = r.game.play_option_from_hand(0, idx);
}

#[test]
fn bt26_057_option_de_digivolves_then_grants_forced_attack() {
    let mut r = setup();
    r.place_on_field(0, "GDT", Some(0)); // [Glowing Dawn] use requirement
    let opp = r.place_stack(1, &["OPP4", "ATK"]);
    use_option(&mut r);
    pick_first(&mut r, 0); // <De-Digivolve 1> target
    let _ = r.declare_max_count();
    assert_eq!(top_id(&r, opp), "OPP4", "<De-Digivolve 1>");
    let v = r.pending_selection_view().expect("forced-attack grant target");
    assert!(!v.is_optional, "mandatory");
    pick_first(&mut r, 0);
    let _ = r.auto_resolve();
    assert!(!suspended(&r, opp));
    r.pass_turn(); // → P1: Start of Main Phase → it attacks
    let _ = r.auto_resolve();
    assert!(
        suspended(&r, opp),
        "\"[Start of Your Main Phase] This Digimon attacks.\""
    );
}

#[test]
fn bt26_057_option_grant_expires_after_their_turn() {
    let mut r = setup();
    r.place_on_field(0, "GDT", Some(0));
    let opp = r.place_stack(1, &["OPP4", "ATK"]);
    use_option(&mut r);
    pick_first(&mut r, 0);
    let _ = r.declare_max_count();
    pick_first(&mut r, 0);
    let _ = r.auto_resolve();
    r.pass_turn(); // P1 attacks
    let _ = r.auto_resolve();
    r.game.players[1].battle_area[opp.index as usize].is_suspended = false;
    r.pass_turn(); // → P0
    r.pass_turn(); // → P1 again: the grant has expired
    let _ = r.auto_resolve();
    assert!(!suspended(&r, opp), "no attack on the next turn");
}

#[test]
fn bt26_057_option_grant_does_not_trigger_on_an_unaffected_digimon() {
    // Official Q&A: the effect is given, but a Digimon not affected by
    // effects when it would trigger does not trigger it.
    let mut r = setup();
    r.place_on_field(0, "GDT", Some(0));
    let opp = r.place_on_field(1, "OPP4", Some(0));
    use_option(&mut r);
    pick_first(&mut r, 0); // De-Digivolve target (no digivolution cards)
    let _ = r.declare_max_count();
    pick_first(&mut r, 0); // grant
    let _ = r.auto_resolve();
    // "Isn't affected by your opponent's effects" (every effect category).
    r.game.modifiers.add(
        opp,
        digimon_engine::modifiers::ModifierEntry::simple(
            digimon_engine::enums::ModifierType::CannotBeAffected,
            0,
            digimon_engine::enums::Expiry::Permanent,
            1,
        )
        .with_effect_immunity_filter(digimon_engine::modifiers::EffectImmunityFilter {
            source_kind: None,
            controller: digimon_engine::modifiers::EffectControllerFilter::OpponentOnly,
        }),
    );
    assert!(r
        .game
        .permanent_is_unaffected_by_effect(opp, 0, EffectSourceKind::Option));
    r.pass_turn();
    let _ = r.auto_resolve();
    assert!(!suspended(&r, opp), "immune → the granted effect doesn't trigger");
}

// ─── Option face: <Use Req. ([Glowing Dawn] trait)> ─────────────────────────
// §4-22 colour requirement + the printed Use Req.: the black Option is usable
// with a black card OR a [Glowing Dawn] card on your field (DCGO
// `CardEffectFactory.UseRequirements(card, EqualsTraits("Glowing Dawn"))`).

/// The hand-play bit of the Option (DUAL hand play = Option use) after
/// pushing it into P0's hand in the main phase.
fn option_use_legal(r: &mut DebugRunner) -> bool {
    r.game.enter_main_phase();
    push_hand(r, 0, CARD_ID);
    let idx = r.game.players[0].hand.len() - 1;
    build_action_mask(&r.game, 0)[PLAY_HAND_START as usize + idx] == 1.0
}

#[test]
fn bt26_057_use_req_glowing_dawn_card_allows_the_option_without_black() {
    let mut r = setup();
    // A purple [Glowing Dawn] Tamer and a yellow Digimon: no black card.
    r.place_on_field(0, "GDT", Some(0));
    r.place_on_field(0, "PLAIN4", Some(0));
    assert!(
        option_use_legal(&mut r),
        "<Use Req. ([Glowing Dawn] trait)> satisfies the colour requirement"
    );
}

#[test]
fn bt26_057_option_is_illegal_without_black_or_glowing_dawn() {
    let mut r = setup();
    // The same board shape minus the [Glowing Dawn] card: a red plain Tamer
    // and a yellow plain Digimon.
    r.place_on_field(0, "PT", Some(0));
    r.place_on_field(0, "PLAIN4", Some(0));
    assert!(
        !option_use_legal(&mut r),
        "no black card and no [Glowing Dawn] card: not usable"
    );
}

#[test]
fn bt26_057_option_is_usable_with_a_black_card_alone() {
    let mut r = setup();
    r.place_on_field(0, "BLK4", Some(0));
    assert!(option_use_legal(&mut r), "the normal black colour requirement");
}

#[test]
fn bt26_057_opponents_glowing_dawn_card_does_not_satisfy_use_req() {
    let mut r = setup();
    r.place_on_field(0, "PT", Some(0));
    r.place_on_field(1, "GDT", Some(0));
    assert!(!option_use_legal(&mut r), "only YOUR [Glowing Dawn] cards");
}

// ─── Option face: <Arts Digivolve> (§4-20) ──────────────────────────────────
// Instead of being trashed after use, one of your cards on the field may
// digivolve into this DUAL card without paying the cost.

/// Use the Option from hand with no opponent Digimon (the [Main] picks have
/// no candidates) and return the memory right after paying its use cost.
fn use_option_to_arts(r: &mut DebugRunner) -> i16 {
    r.game.enter_main_phase();
    push_hand(r, 0, CARD_ID);
    let idx = r.game.players[0].hand.len() - 1;
    let mem = r.memory();
    let res = r.game.play_option_from_hand(0, idx);
    assert_ne!(res, OptionPlayResult::Invalid, "the Option is usable");
    mem - 4
}

fn arts_prompt_offers(r: &DebugRunner, base: PermanentHandle) -> bool {
    r.pending_selection_view().is_some_and(|v| {
        v.prompt.contains("Arts Digivolve")
            && v.is_optional
            && v.valid_action_ids
                .contains(&encode_attack(0, base.index as u16))
    })
}

#[test]
fn bt26_057_arts_digivolve_onto_a_glowing_dawn_level_4_for_free() {
    let mut r = setup();
    r.place_on_field(0, "GDT", Some(0));
    let base = r.place_on_field(0, "GD4", Some(0));
    let mem_after_use = use_option_to_arts(&mut r);
    assert!(arts_prompt_offers(&r, base), "Arts Digivolve offered");
    let trash = r.trash_size(0);
    r.execute_action(0, encode_attack(0, base.index as u16))
        .expect("accept Arts Digivolve");
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, base), CARD_ID, "Bearcatmon on top");
    assert_eq!(sources(&r, base), 1);
    assert_eq!(r.memory(), mem_after_use, "no digivolve cost paid");
    assert_eq!(r.trash_size(0), trash, "replaces the Option's trashing");
}

/// Install the printed digivolve circles (Black Lv.4 / 4, Red Lv.4 / 4 — the
/// production `cards.json` `dual.digimon.evo_costs`): `DebugRunner` back-fills only the
/// level+colour alt paths of a DSL card, not its printed circles.
fn with_printed_circles(r: &mut DebugRunner) {
    let idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == CARD_ID)
        .expect("BT26-057 in card_data");
    let circles = vec![
        EvoCost { card_color: CardColor::Black as u8, level: 4, memory_cost: 4 },
        EvoCost { card_color: CardColor::Red as u8, level: 4, memory_cost: 4 },
    ];
    // A DUAL's circles live on its Digimon face (`CardData::digivolution_costs`).
    let data = &mut r.game.card_data[idx];
    data.dual.as_mut().expect("DUAL").digimon.evo_costs = circles.clone();
    data.evo_costs = circles;
}

#[test]
fn bt26_057_arts_digivolve_onto_a_red_level_4() {
    // The printed Red Lv.4 digivolve condition also qualifies (cost ignored).
    let mut r = setup();
    with_printed_circles(&mut r);
    r.place_on_field(0, "GDT", Some(0)); // Use Req.
    let base = r.place_on_field(0, "RED4", Some(0));
    use_option_to_arts(&mut r);
    assert!(arts_prompt_offers(&r, base));
    r.execute_action(0, encode_attack(0, base.index as u16))
        .expect("accept");
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, base), CARD_ID);
}

#[test]
fn bt26_057_arts_digivolve_declined_trashes_the_option() {
    let mut r = setup();
    r.place_on_field(0, "GDT", Some(0));
    let base = r.place_on_field(0, "GD4", Some(0));
    use_option_to_arts(&mut r);
    assert!(arts_prompt_offers(&r, base));
    r.execute_action(0, PASS).expect("decline Arts Digivolve");
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, base), "GD4");
    assert_eq!(trash_ids(&r, 0), vec![CARD_ID.to_string()], "trashed");
}

#[test]
fn bt26_057_arts_digivolve_not_offered_without_a_legal_base() {
    let mut r = setup();
    r.place_on_field(0, "GDT", Some(0)); // Use Req.; a Tamer can't digivolve
    let plain = r.place_on_field(0, "PLAIN4", Some(0)); // yellow, no GD trait
    use_option_to_arts(&mut r);
    let _ = r.auto_resolve();
    assert!(!arts_prompt_offers(&r, plain));
    assert!(r.game.pending_selection.is_none(), "nothing to choose");
    assert_eq!(top_id(&r, plain), "PLAIN4");
    assert_eq!(trash_ids(&r, 0), vec![CARD_ID.to_string()]);
}

// ─── Self-combo: the WD cost trash triggers its own [All Turns] unsuspend ───

#[test]
fn bt26_057_wd_cost_trash_lets_a_suspended_bearcatmon_unsuspend() {
    let mut r = setup();
    let t = tamer_with_face_down(&mut r, 0, "GDT", 1);
    r.game.enter_main_phase();
    let host = r.place_on_field(0, "GD4", Some(0));
    set_suspended(&mut r, host); // e.g. it attacked earlier this turn
    push_hand(&mut r, 0, CARD_ID);
    r.game.decode_action(encode_digivolve(0, host.index as u16), 0);
    r.accept_optional_trigger().expect("[When Digivolving] cost");
    pick_first(&mut r, 0); // the Tamer
    assert_eq!(sources(&r, t), 0, "an effect trashed from under your Tamer");
    r.accept_optional_trigger()
        .expect("[All Turns] this Digimon may unsuspend");
    let _ = r.auto_resolve();
    assert!(!suspended(&r, host), "unsuspended");
    assert_eq!(r.effective_dp(host), Some(11000));
}
