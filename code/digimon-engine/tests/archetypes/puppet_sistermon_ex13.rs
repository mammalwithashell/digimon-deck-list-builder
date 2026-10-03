//! EX13 "Puppet (Sistermon Awakened DUALs)" slice — archetype interaction tests.
//!
//! Model: `qa/archetype-qa/puppet-sistermon-awakened-ex13-model.md`. Each
//! `#[test]` maps 1:1 to a named combo there (C1, C1', C2, C3, C3', C4).
//!
//! Slice cards: EX13-066 Sistermon Noir (Awakened) (DUAL), EX13-035 KingEtemon.
//! EX13-065 Sistermon Blanc (Awakened) is BLOCKED in `validated_cards_dsl.json`
//! (G-NESTED-PARKED-REPLACEMENT, `qa/archetype-qa/engine-gaps.md`), so no combo
//! here names it — its combos are logged as blocked in the model doc.
//!
//! Per-card behaviour lives in `tests/cards_behavioral/ex13/ex13_0{35,66}.rs`;
//! this file asserts only the cross-card SYSTEM facts: the Option's
//! De-Digivolve turning a big Digimon into a [WD]-deletable one, `<Decode>`
//! replaying a source whose [On Play] fires, and KingEtemon's free plays
//! feeding its own aura threshold / budget.
//!
//! Every role is a real DSL card — no synthetic `make_test_card`:
//! - BT6-082 Sistermon Blanc (Lv.3, [Huckmon] in its text) = Arts / alt-path base,
//!   BT6-084 Sistermon Ciel = white Option enabler + name-route base,
//!   BT23-077 Sistermon Ciel (also [Sistermon Noir]) = Decode payload;
//! - EX13-027 Chuumon / EX13-028 Sukamon / EX10-039 ChuuChuumon = KingEtemon bodies;
//! - ST3-08 MagnaAngemon = real yellow Lv.5 base for KingEtemon's colour circle
//!   (no [Etemon]/[Sukamon] Lv.5 is implemented);
//! - vanilla ST1 Digimon (Dracomon L3/3, Birdramon L4/4, Greymon L4/5,
//!   Garudamon L5/6, Biyomon filler) = opponents / deck filler.
//!
//! No DCGO C# exists for EX13-035/065/066 (base submodule `b9a0638cd`);
//! official Bandai DB bundles (`data/card_bundles/<ID>.md`) + `general_rule.pdf`
//! (`<Decode>`, `<De-Digivolve>`, `<Arts Digivolve>`, `<Security A.>`) govern.

#![allow(dead_code)]

use std::collections::HashMap;
use std::path::PathBuf;

use digimon_engine::action::space::{encode_attack, PASS, PLAY_HAND_START, TRASH_EFFECT_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{GamePhase, ModifierType, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::{OptionPlayResult, SelectionKind};

use super::support::{dsl_builder, snapshot};

// ─── Card ids ────────────────────────────────────────────────────────────────

const NOIR: &str = "EX13-066";
const KING: &str = "EX13-035";
/// Lv.3, play cost 3, [Huckmon] in its printed text; [On Play] <Draw 1>.
const BLANC: &str = "BT6-082";
/// White Lv.4 [Sistermon Ciel] (Option colour enabler + Noir name route).
const CIEL: &str = "BT6-084";
/// Lv.4 [Sistermon Ciel], also named [Sistermon Noir]; [On Play] delete <=4.
const CIEL_NOIR: &str = "BT23-077";
const CHUUMON: &str = "EX13-027";
const SUKAMON: &str = "EX13-028";
/// Play cost 4, [Chuumon] in its name.
const CHUUCHUUMON: &str = "EX10-039";
/// Real yellow Lv.5 (play cost 7) — KingEtemon's Yellow Lv.5 circle.
const MAGNAANGEMON: &str = "ST3-08";
const BIYOMON: &str = "ST1-02";
/// Vanilla Lv.3, play cost 3.
const DRACOMON: &str = "ST1-04";
/// Vanilla Lv.4, play cost 4.
const BIRDRAMON: &str = "ST1-05";
/// Lv.4, play cost 5 (inherited <Security A. +1> only).
const GREYMON: &str = "ST1-07";
/// Lv.5, play cost 6, DP 7000 (its [WD] never fires — placed, not digivolved).
const GARUDAMON: &str = "ST1-08";

/// Cards whose printed text a filter scans ("[Huckmon] in its text").
const PRINTED: &[&str] = &[BLANC, CIEL, CIEL_NOIR, NOIR];
const REAL: &[&str] = &[
    NOIR, KING, BLANC, CIEL, CIEL_NOIR, CHUUMON, SUKAMON, CHUUCHUUMON, MAGNAANGEMON, BIYOMON,
    DRACOMON, BIRDRAMON, GREYMON, GARUDAMON,
];

// ─── Fixtures ────────────────────────────────────────────────────────────────

/// Production `CardData` for `id`, parsed from its committed per-card JSON.
fn printed(id: &str) -> CardData {
    let set = id.split('-').next().unwrap().to_lowercase();
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("cards")
        .join(set)
        .join(format!("{id}.json"));
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let wrapped = format!("{{\"{id}\": {raw}}}");
    let mut map: HashMap<String, CardData> =
        CardData::load_from_str(&wrapped).unwrap_or_else(|e| panic!("{id}: {e}"));
    map.remove(id).unwrap_or_else(|| panic!("{id} parsed"))
}

/// DSL-loaded cards carry empty printed text; Noir's "Lv.3 w/[Huckmon] in
/// text" route scans it in production, so copy the printed text in.
fn with_printed_text(runner: &mut DebugRunner) {
    for id in PRINTED {
        let p = printed(id);
        for c in runner.game.card_data.iter_mut().filter(|c| c.card_id == *id) {
            c.effect_text = p.effect_text.clone();
            c.inherited_text = p.inherited_text.clone();
            c.security_text = p.security_text.clone();
        }
    }
}

fn builder() -> DebugRunnerBuilder {
    dsl_builder(REAL)
        .deck(1, &[BIYOMON; 8])
        .security(0, &[BIYOMON; 3])
        .security(1, &[BIYOMON; 3])
}

/// Start in player 0's Main phase (turn 1).
fn start_main(b: DebugRunnerBuilder) -> DebugRunner {
    let mut r = b.start();
    r.skip_mulligan();
    with_printed_text(&mut r);
    r.game.turn_count = 1;
    r.game.current_phase = GamePhase::Main;
    r
}

fn field_ids(r: &DebugRunner, p: u8) -> Vec<String> {
    r.game
        .player(p)
        .battle_area
        .iter()
        .map(|perm| perm.top_card().card_id(&r.game.card_data).to_string())
        .collect()
}

fn zone_ids(r: &DebugRunner, p: u8, trash: bool) -> Vec<String> {
    let z = if trash { &r.game.player(p).trash } else { &r.game.player(p).hand };
    z.iter().map(|c| c.card_id(&r.game.card_data).to_string()).collect()
}

fn count(v: &[String], id: &str) -> usize {
    v.iter().filter(|x| *x == id).count()
}

fn hand_pos(r: &DebugRunner, id: &str) -> usize {
    zone_ids(r, 0, false)
        .iter()
        .position(|c| c == id)
        .unwrap_or_else(|| panic!("{id} in hand"))
}

fn trash_pos(r: &DebugRunner, id: &str) -> usize {
    zone_ids(r, 0, true)
        .iter()
        .position(|c| c == id)
        .unwrap_or_else(|| panic!("{id} in trash"))
}

fn top_id(r: &DebugRunner, h: PermanentHandle) -> String {
    r.game.player(h.player).battle_area[h.index as usize]
        .top_card()
        .card_id(&r.game.card_data)
        .to_string()
}

/// Drive the remaining prompts: accept the first non-PASS action on every
/// prompt (mandatory or optional), bounded so a loop surfaces as a panic.
fn accept_rest(r: &mut DebugRunner) {
    for _ in 0..40 {
        let Some(v) = r.pending_selection_view() else { return };
        let a = v
            .valid_action_ids
            .iter()
            .copied()
            .find(|&a| a != PASS)
            .unwrap_or(PASS);
        r.execute_action(v.selecting_player, a).unwrap();
    }
    panic!("selection loop did not terminate");
}

/// Use Noir's Option face from hand and drive it to the end:
/// - the union free-play pick takes `free_play` from the trash (or PASSes),
/// - the De-Digivolve target is `opp`,
/// - the `<Arts Digivolve>` offer digivolves onto `arts_base`,
/// - Noir's [WD] delete (if any target remains) is accepted.
/// Returns the order in which the prompts were seen, for diagnostics.
fn use_noir_option(
    r: &mut DebugRunner,
    free_play: Option<&str>,
    opp: PermanentHandle,
    arts_base: PermanentHandle,
) -> Vec<String> {
    let idx = hand_pos(r, NOIR);
    assert_eq!(r.game.play_option_from_hand(0, idx), OptionPlayResult::Pending);
    let arts_action = encode_attack(0, arts_base.index as u16);
    let mut seen = Vec::new();
    let mut arts_done = false;
    for _ in 0..40 {
        let Some(v) = r.pending_selection_view() else { break };
        seen.push(format!("{:?}", v.kind));
        let action = match v.kind {
            SelectionKind::UnionZone { .. } => match free_play {
                Some(id) => TRASH_EFFECT_START + trash_pos(r, id) as u16,
                None => PASS,
            },
            SelectionKind::OwnField
                if !arts_done && v.is_optional && v.valid_action_ids.contains(&arts_action) =>
            {
                arts_done = true;
                arts_action
            }
            SelectionKind::OppField if v.valid_action_ids.contains(&encode_attack(0, opp.index as u16)) => {
                encode_attack(0, opp.index as u16)
            }
            _ => v
                .valid_action_ids
                .iter()
                .copied()
                .find(|&a| a != PASS)
                .unwrap_or(PASS),
        };
        r.execute_action(v.selecting_player, action).unwrap();
    }
    assert!(r.pending_selection_view().is_none(), "Option chain resolved: {seen:?}");
    assert!(arts_done, "the <Arts Digivolve> offer was taken: {seen:?}");
    seen
}

// ─── C1 — Noir Option → De-Digivolve → Arts → [WD] delete ────────────────────

/// Combo C1 — "Noir Option -> De-Digivolve -> Arts -> [WD] delete".
///
/// - Cards: EX13-066 (Option face, hand) + BT6-082 Sistermon Blanc (trash) +
///   BT6-084 Sistermon Ciel (field; white enabler).
/// - Expected: the Option free-plays BT6-082 from trash (its [On Play] draws 1),
///   so you control 2 Digimon -> `<De-Digivolve 2>` strips the opponent's
///   Garudamon/Greymon off Dracomon (now play cost 3). Instead of trashing,
///   `<Arts Digivolve>` puts Noir on BT6-082 free (Lv.3 w/[Huckmon] in text), and
///   Noir's [WD] deletes the Dracomon — a Digimon that was play cost 6 when the
///   Option was used. Noir never touches the trash; memory paid = use cost 5 only.
/// - Sources: bundles EX13-066 / BT6-082; general_rule.pdf `<De-Digivolve>`,
///   `<Arts Digivolve>`; no DCGO C#.
#[test]
fn c1_noir_option_de_digivolve_then_arts_wd_deletes_the_stripped_digimon() {
    let mut r = start_main(builder().hand(0, &[NOIR]).deck(0, &[BIYOMON; 5]).memory(8));
    let ciel = r.place_on_field(0, CIEL, Some(0));
    r.inject_trash(0, BLANC);
    let opp = r.place_stack(1, &[DRACOMON, GREYMON, GARUDAMON]);
    // Blanc will land in the next battle-area slot.
    let blanc_slot = PermanentHandle { player: 0, index: ciel.index + 1 };
    let before = snapshot(&r);
    let mem = r.game.memory;

    use_noir_option(&mut r, Some(BLANC), opp, blanc_slot);
    let _ = r.auto_resolve();

    assert_eq!(field_ids(&r, 0), vec![CIEL.to_string(), NOIR.to_string()], "Noir sits on Blanc");
    let noir_perm = &r.game.player(0).battle_area[blanc_slot.index as usize];
    assert_eq!(noir_perm.stack_size(), 2, "Noir over BT6-082");
    assert!(field_ids(&r, 1).is_empty(), "[WD] deleted the de-digivolved Dracomon");
    let opp_trash = zone_ids(&r, 1, true);
    for id in [DRACOMON, GREYMON, GARUDAMON] {
        assert_eq!(count(&opp_trash, id), 1, "{id} in opp trash");
    }
    assert_eq!(count(&zone_ids(&r, 0, true), NOIR), 0, "Arts replaced the trash");
    assert_eq!(r.trash_size(0), 0, "Blanc left the trash, Noir never entered it");
    let after = snapshot(&r);
    assert_eq!(after.hand[0], before.hand[0] - 1 + 1, "Option left hand, Blanc's <Draw 1> drew");
    assert_eq!(after.deck[0], before.deck[0] - 1);
    assert_eq!(mem - r.game.memory, 5, "only the Option's use cost; Arts digivolve is free");
}

/// Combo C1' — unhappy path: without the free-played body the [WD] whiffs.
///
/// - Cards: EX13-066 (Option) + BT6-084 (field). The optional free play is
///   declined.
/// - Expected: 1 own Digimon -> `<De-Digivolve 1>` leaves Greymon (play cost 5)
///   on top; Arts onto BT6-084 still happens (name route), but Noir's [WD] has no
///   play cost <=4 target, so the opponent keeps its Digimon. De-Digivolve's N is
///   counted at resolution, after the optional play — the body is load-bearing.
#[test]
fn c1_unhappy_without_the_free_play_the_wd_has_no_target() {
    let mut r = start_main(builder().hand(0, &[NOIR]).deck(0, &[BIYOMON; 5]).memory(8));
    let ciel = r.place_on_field(0, CIEL, Some(0));
    r.inject_trash(0, BLANC);
    let opp = r.place_stack(1, &[DRACOMON, GREYMON, GARUDAMON]);

    use_noir_option(&mut r, None, opp, ciel);
    let _ = r.auto_resolve();

    assert_eq!(field_ids(&r, 0), vec![NOIR.to_string()], "Noir Arts-digivolved onto Ciel");
    assert_eq!(field_ids(&r, 1), vec![GREYMON.to_string()], "play cost 5 survives the [WD]");
    let opp_perm = &r.game.player(1).battle_area[opp.index as usize];
    assert_eq!(opp_perm.stack_size(), 2, "De-Digivolve 1 only");
    assert_eq!(zone_ids(&r, 0, true), vec![BLANC.to_string()], "Blanc stayed in trash");
}

// ─── C2 — Noir on Ciel → [WD] delete → Decode → [On Play] delete ────────────

/// Combo C2 — "Noir on Ciel -> [WD] delete -> Decode replays BT23-077 ->
/// [On Play] delete".
///
/// - Cards: EX13-066 (Digimon face, hand) + BT23-077 Sistermon Ciel (field).
/// - Expected: Noir digivolves onto BT23-077 for 1 (name route; BT23-077 is also
///   [Sistermon Noir]); [WD] deletes Dracomon (play cost 3). The opponent's
///   effect then deletes Noir: `<Decode>` plays BT23-077 out of Noir's sources
///   free, and BT23-077's [On Play] deletes Birdramon (play cost 4). Greymon
///   (play cost 5) is never a legal target and survives both.
/// - Sources: bundles EX13-066 / BT23-077; general_rule.pdf `<Decode>` (leave by
///   other than battle; the played card's [On Play] triggers).
#[test]
fn c2_noir_wd_delete_then_decode_replays_ciel_whose_on_play_deletes_again() {
    let mut r = start_main(builder().hand(0, &[NOIR]).deck(0, &[BIYOMON; 5]).memory(5));
    let base = r.place_on_field(0, CIEL_NOIR, Some(0));
    let dracomon = r.place_on_field(1, DRACOMON, Some(0));
    let birdramon = r.place_on_field(1, BIRDRAMON, Some(0));
    let greymon = r.place_on_field(1, GREYMON, Some(0));
    let mem = r.game.memory;

    r.game.digivolve_from_hand(0, hand_pos(&r, NOIR), base.index as usize, PlaySource::ByHand);
    assert_eq!(mem - r.game.memory, 1, "name route: cost 1");
    let v = r.pending_selection_view().expect("[WD] delete pick");
    assert_eq!(v.kind, SelectionKind::OppField);
    assert!(!v.valid_action_ids.contains(&encode_attack(0, greymon.index as u16)), "cost 5");
    assert!(v.valid_action_ids.contains(&encode_attack(0, birdramon.index as u16)));
    r.execute_action(0, encode_attack(0, dracomon.index as u16)).unwrap();
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, base), NOIR);
    let mut opp_now = field_ids(&r, 1);
    opp_now.sort();
    assert_eq!(opp_now, vec![BIRDRAMON.to_string(), GREYMON.to_string()]);

    // The opponent's effect deletes Noir → <Decode> → BT23-077 [On Play].
    r.game.delete_permanent_with_cause(base, ReplacementCause::OpponentEffect);
    let v = r.pending_selection_view().expect("<Decode> window");
    assert!(v.is_optional, "<Decode> is a 'may'");
    accept_rest(&mut r);

    assert_eq!(field_ids(&r, 0), vec![CIEL_NOIR.to_string()], "Decode played BT23-077");
    assert_eq!(field_ids(&r, 1), vec![GREYMON.to_string()], "BT23-077 [On Play] took Birdramon");
    assert_eq!(count(&zone_ids(&r, 0, true), NOIR), 1, "Noir itself was deleted");
    let opp_trash = zone_ids(&r, 1, true);
    assert_eq!(count(&opp_trash, DRACOMON), 1);
    assert_eq!(count(&opp_trash, BIRDRAMON), 1);
}

// ─── C3 — KingEtemon [WD] party → aura online ───────────────────────────────

fn king_digivolve(trash: &[&str], deck: &[&str]) -> (DebugRunner, PermanentHandle) {
    let mut r = start_main(builder().hand(0, &[KING]).deck(0, deck).memory(10));
    let base = r.place_on_field(0, MAGNAANGEMON, Some(0));
    for id in trash {
        r.inject_trash(0, id);
    }
    let opp = r.place_on_field(1, GARUDAMON, Some(0));
    let mem = r.game.memory;
    assert!(
        r.game.digivolve_from_hand(0, hand_pos(&r, KING), base.index as usize, PlaySource::ByHand)
            || r.pending_selection_view().is_some()
    );
    assert_eq!(mem - r.game.memory, 5, "Yellow Lv.5 circle: cost 5");
    (r, opp)
}

fn take_trash(r: &mut DebugRunner, id: &str) {
    let v = r.pending_selection_view().expect("free-play pick");
    assert!(matches!(v.kind, SelectionKind::UnionZone { .. }), "{:?}", v.kind);
    let a = TRASH_EFFECT_START + trash_pos(r, id) as u16;
    assert!(v.valid_action_ids.contains(&a), "{id} offered");
    r.execute_action(0, a).unwrap();
}

fn sec_attack(r: &DebugRunner, h: PermanentHandle) -> i32 {
    (r.game.modifiers.sum(h, ModifierType::SecurityAttackChange)
        + r.game.dynamic_security_attack_aura_bonus(h).unwrap_or(0)) as i32
}

/// Combo C3 — "KingEtemon [WD] party -> aura online".
///
/// - Cards: EX13-035 KingEtemon (hand) on ST3-08 MagnaAngemon + 2x EX13-028
///   Sukamon (trash).
/// - Expected: digivolve for 5; [WD] free-plays both Sukamon from the trash
///   (3 + 3 = 6, the base maximum). KingEtemon ([Etemon] in its name) + 2 Sukamon
///   = 3 -> the [All Turns] aura switches on: the opponent's Garudamon drops
///   7000 -> 4000 DP and gets `<Security A. -1>`.
/// - Sources: bundle EX13-035 (+ official Q&A); general_rule.pdf `<Security A.>`.
#[test]
fn c3_kingetemon_wd_plays_two_sukamon_and_switches_the_aura_on() {
    let (mut r, opp) = king_digivolve(&[SUKAMON, SUKAMON], &[BIYOMON; 5]);
    take_trash(&mut r, SUKAMON);
    take_trash(&mut r, SUKAMON);
    let _ = r.auto_resolve();
    r.game.tick_declarative_effects();

    let mut mine = field_ids(&r, 0);
    mine.sort();
    assert_eq!(mine, vec![SUKAMON.to_string(), SUKAMON.to_string(), KING.to_string()]);
    assert_eq!(r.trash_size(0), 0);
    assert_eq!(r.effective_dp(opp), Some(7000 - 3000), "aura -3000 DP");
    assert_eq!(sec_attack(&r, opp), -1, "aura <Security A. -1>");
}

/// Combo C3' — unhappy path: Chuumon feeds the deck but not the aura.
///
/// - Cards: EX13-035 on ST3-08 + EX13-027 Chuumon + EX13-028 Sukamon (trash);
///   deck = Sukamon x6.
/// - Expected: both bodies land (3 + 3 = 6) and Chuumon's own [On Play] fires off
///   the free play once KingEtemon's effect finishes (reveal 3 Sukamon: 1 to
///   hand, 1 trashed, 1 to the deck bottom) — but only 2 Digimon have
///   [Sukamon]/[Etemon] in their names (KingEtemon + Sukamon), so the opponent's
///   DP and security attack are untouched.
#[test]
fn c3_unhappy_chuumon_tutors_but_does_not_count_toward_the_aura() {
    let (mut r, opp) = king_digivolve(&[CHUUMON, SUKAMON], &[SUKAMON; 6]);
    take_trash(&mut r, CHUUMON);
    // Chuumon's [On Play] waits for KingEtemon's effect to finish.
    let hand0 = count(&zone_ids(&r, 0, false), SUKAMON);
    let deck0 = r.deck_size(0);
    take_trash(&mut r, SUKAMON);
    assert_eq!(r.trash_size(0), 0, "both free plays came out of the trash");
    let v = r.pending_selection_view().expect("Chuumon [On Play] reveal");
    assert!(matches!(v.kind, SelectionKind::RevealBucket { .. }), "{:?}", v.kind);
    accept_rest(&mut r);
    r.game.tick_declarative_effects();

    let mut mine = field_ids(&r, 0);
    mine.sort();
    assert_eq!(mine, vec![CHUUMON.to_string(), SUKAMON.to_string(), KING.to_string()]);
    assert_eq!(count(&zone_ids(&r, 0, false), SUKAMON), hand0 + 1, "Chuumon tutored a Sukamon");
    assert_eq!(zone_ids(&r, 0, true), vec![SUKAMON.to_string()], "and trashed 1 such card");
    assert_eq!(r.deck_size(0), deck0 - 2, "3 revealed, 1 returned to the bottom");
    assert_eq!(r.effective_dp(opp), Some(7000), "only 2 [Sukamon]/[Etemon]: aura off");
    assert_eq!(sec_attack(&r, opp), 0);
}

// ─── C4 — Return-10 widens the budget ────────────────────────────────────────

fn king_on_play_with_eleven_sukamon() -> DebugRunner {
    let mut r = start_main(
        builder()
            .hand(0, &[KING, CHUUCHUUMON])
            .deck(0, &[BIYOMON; 5])
            .memory(20),
    );
    for _ in 0..11 {
        r.inject_trash(0, SUKAMON);
    }
    r.play(0, hand_pos(&r, KING)).expect("play KingEtemon");
    let v = r.pending_selection_view().expect("return-10 choice first (official Q&A)");
    assert_eq!(v.kind, SelectionKind::EffectChoice);
    r
}

/// Combo C4 — "Return-10 widens the budget (ChuuChuumon + Sukamon = 7)".
///
/// - Cards: EX13-035 (played from hand) + EX10-039 ChuuChuumon (hand, play cost
///   4, [Chuumon] in its name) + 11x EX13-028 Sukamon (trash).
/// - Expected: returning 10 Sukamon to the deck bottom raises the maximum to 12,
///   so ChuuChuumon (4) and the 11th Sukamon (3) both land — 7 total, which the
///   base maximum of 6 can't fit. (Paired with the decline branch below.)
#[test]
fn c4_returning_ten_lets_chuuchuumon_and_sukamon_both_land() {
    let mut r = king_on_play_with_eleven_sukamon();
    let deck_before = r.deck_size(0);
    r.execute_branch(0).expect("return 10");
    while matches!(r.pending_kind(), Some(SelectionKind::CountCappedMultiSelect { .. })) {
        let v = r.pending_selection_view().unwrap();
        let a = v.valid_action_ids.iter().copied().find(|&a| a != PASS).unwrap();
        r.execute_action(0, a).unwrap();
    }
    assert_eq!(r.deck_size(0), deck_before + 10, "10 such cards to the deck bottom");
    assert_eq!(zone_ids(&r, 0, true), vec![SUKAMON.to_string()], "1 Sukamon left in trash");

    let v = r.pending_selection_view().expect("first pick");
    r.execute_action(0, PLAY_HAND_START + hand_pos(&r, CHUUCHUUMON) as u16).unwrap();
    let _ = v;
    take_trash(&mut r, SUKAMON);
    let _ = r.auto_resolve();

    let mut mine = field_ids(&r, 0);
    mine.sort();
    let mut want = vec![KING.to_string(), CHUUCHUUMON.to_string(), SUKAMON.to_string()];
    want.sort();
    assert_eq!(mine, want, "4 + 3 = 7 fits the raised maximum of 12");
}

/// Combo C4 — decline branch: keep the base maximum of 6 and the Sukamon no
/// longer fits beside ChuuChuumon (4 + 3 > 6).
#[test]
fn c4_unhappy_declining_the_return_leaves_no_room_for_the_sukamon() {
    let mut r = king_on_play_with_eleven_sukamon();
    r.execute_branch(1).expect("decline the return");
    assert_eq!(r.trash_size(0), 11);
    r.execute_action(0, PLAY_HAND_START + hand_pos(&r, CHUUCHUUMON) as u16).unwrap();
    if let Some(v) = r.pending_selection_view() {
        if matches!(v.kind, SelectionKind::UnionZone { .. }) {
            let a = TRASH_EFFECT_START + trash_pos(&r, SUKAMON) as u16;
            assert!(!v.valid_action_ids.contains(&a), "only 2 of the 6 maximum left");
            r.execute_action(0, PASS).unwrap();
        }
    }
    let _ = r.auto_resolve();
    let mut mine = field_ids(&r, 0);
    mine.sort();
    let mut want = vec![KING.to_string(), CHUUCHUUMON.to_string()];
    want.sort();
    assert_eq!(mine, want);
    assert_eq!(r.trash_size(0), 11);
}
