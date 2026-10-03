//! BT24-009 Shamanmon — Lv.3 Red/Purple, Demon/Titan/TS, DP 1000, cost 3.
//!
//! Digivolve: Red Lv.2 / cost 1, Purple Lv.2 / cost 1,
//!   [Digivolve] [Tsunomon]/Lv.2 w/[TS] trait: Cost 0.
//! [On Play] By trashing 1 card with the [Demon], [Shaman] or [Titan] trait
//!   from your hand, <Draw 2>.
//! Inherited: [Your Turn] [Once Per Turn] When your hand is trashed from, this
//!   [Demon] or [Titan] trait Digimon may digivolve into [Titamon] or a
//!   [Titan] trait Digimon card in the trash with the digivolution cost
//!   reduced by 1.
//!
//! DCGO: BT24/Red/BT24_009.cs.

#![allow(dead_code, unused_imports)]

const CARD_ID: &str = "BT24-009";
const OTHER_COLOR_LV2: &str = "RED2";

#[path = "../bt26/support.rs"]
mod support;

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::debug_runner::{DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, EffectTiming, PlaySource};
use support::*;

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("card in embedded DSL pack")
        .from_dsl_yaml(HAND_TRASHER_YAML)
        .expect("trasher")
        .add_card(filler("FILLER"))
        .add_card(digimon("DEMON3", "Impmon", CardColor::Purple, 3, 3, &["Demon"]))
        .add_card(digimon("SHAMAN3", "Shaman", CardColor::Purple, 3, 3, &["Shaman"]))
        .add_card(digimon("TITAN3", "Gazimon", CardColor::Purple, 3, 3, &["Titan"]))
        .add_card(digimon("PLAIN3", "Plain", CardColor::Purple, 3, 3, &[]))
        .add_card(tamer("DEMON-TAMER", "Demon Tamer", CardColor::Purple, &["Demon"]))
        .add_card(tamer("TITAN-TAMER", "Titan Tamer", CardColor::Purple, &["Titan"]))
        .add_card(digimon_evo("TITAN4", "Big Titan", CardColor::Purple, 4, 5, &["Titan"], 3, 3))
        .add_card(digimon_evo("TITAN4B", "Big Titan B", CardColor::Purple, 4, 5, &["Titan"], 3, 3))
        .add_card(digimon_evo("DEMON4", "Big Demon", CardColor::Purple, 4, 5, &["Demon"], 3, 3))
        .add_card(digimon_evo("TITAMON", "Titamon", CardColor::Purple, 4, 5, &[], 3, 3))
        .add_card(digimon_evo("PLAIN4", "Plain Four", CardColor::Purple, 4, 5, &[], 3, 3))
        .add_card(digimon("TSUNOMON", "Tsunomon", CardColor::Yellow, 2, 0, &[]))
        .add_card(digimon("TS2", "Gabumon Baby", CardColor::Yellow, 2, 0, &["TS"]))
        .add_card(digimon("YELLOW2", "Yellow Two", CardColor::Yellow, 2, 0, &[]))
        .add_card(digimon("RED2", "Red Two", CardColor::Red, 2, 0, &[]))
        .add_card(digimon("BLUE2", "Blue Two", CardColor::Blue, 2, 0, &[]))
        .add_card(digimon("GREEN2", "Green Two", CardColor::Green, 2, 0, &[]))
        .add_card(digimon("PURPLE2", "Purple Two", CardColor::Purple, 2, 0, &[]))
        .deck(0, &["FILLER"; 8])
        .deck(1, &["FILLER"; 8])
        .memory(5)
}

fn setup() -> DebugRunner {
    let mut r = base().start();
    r.set_first_player(0);
    r
}

fn triggered(idx: usize) -> digimon_dsl::compiled::CompiledTriggeredClause {
    let r = setup();
    match r.compiled_card(CARD_ID).unwrap().effects[idx].clone() {
        CompiledClause::Triggered(t) => t,
        other => panic!("clause {idx} is not triggered: {other:?}"),
    }
}

/// Digivolve `CARD_ID` from hand onto a fresh `base_id` permanent; returns
/// `Some(memory paid)` on success, `None` when the engine rejects it.
fn digivolve_onto(base_id: &str) -> Option<i16> {
    let mut r = setup();
    let b = r.place_on_field(0, base_id, Some(0));
    push_hand(&mut r, 0, CARD_ID);
    let before = r.memory();
    let ok = r
        .game
        .digivolve_from_hand(0, 0, b.index as usize, PlaySource::ByHand);
    if !ok {
        return None;
    }
    assert_eq!(top_id(&r, b), CARD_ID);
    Some(before - r.memory())
}

// ─── Alt-digivolve / digivolve circles ───────────────────────────────────────

#[test]
fn bt24_009_digivolves_from_tsunomon_for_zero() {
    assert_eq!(digivolve_onto("TSUNOMON"), Some(0));
}

#[test]
fn bt24_009_digivolves_from_lv2_ts_for_zero() {
    assert_eq!(digivolve_onto("TS2"), Some(0));
}

#[test]
fn bt24_009_digivolves_from_purple_lv2_for_one() {
    assert_eq!(digivolve_onto("PURPLE2"), Some(1));
}

#[test]
fn bt24_009_digivolves_from_other_printed_color_lv2_for_one() {
    assert_eq!(digivolve_onto(OTHER_COLOR_LV2), Some(1));
}

#[test]
fn bt24_009_cannot_digivolve_from_plain_off_color_lv2() {
    assert_eq!(digivolve_onto("YELLOW2"), None);
}

// ─── Inherited [Your Turn][OPT] hand trashed → digivolve from trash ──────────

fn inherited_idx() -> usize {
    let r = setup();
    r.compiled_card(CARD_ID)
        .unwrap()
        .effects
        .iter()
        .position(|c| matches!(c, CompiledClause::Triggered(t) if t.scope == CompiledScope::Inherited))
        .expect("inherited clause")
}

#[test]
fn bt24_009_inherited_structure() {
    let t = triggered(inherited_idx());
    assert_eq!(t.when, vec![CompiledTiming::OnDiscardHand]);
    assert!(t.once_per_turn);
}

fn trash_my_hand(r: &mut DebugRunner, owner: u8) {
    push_hand(r, owner, "FILLER");
    let t = r.place_on_field(owner, "T-HANDTRASH", Some(0));
    fire(r, EffectTiming::OnPlay, t);
    pick_hand(r, owner, "FILLER");
}

#[test]
fn bt24_009_inherited_digivolves_demon_host_into_titan_in_trash() {
    let mut r = setup();
    let host = r.place_stack(0, &[CARD_ID, "DEMON3"]);
    push_trash(&mut r, 0, "TITAN4");
    push_trash(&mut r, 0, "PLAIN4");
    push_trash(&mut r, 0, "DEMON4");
    trash_my_hand(&mut r, 0);
    let v = r.pending_selection_view().expect("optional digivolve pick");
    assert!(v.is_optional, "'may digivolve'");
    assert_eq!(non_pass(&r).len(), 1, "only the [Titan] Digimon: {v:?}");
    pick_trash(&mut r, 0, "TITAN4");
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, host), "TITAN4");
    assert_eq!(r.memory(), 5 - (3 - 1), "digivolution cost reduced by 1");
}

#[test]
fn bt24_009_inherited_can_pick_titamon_by_name() {
    let mut r = setup();
    let host = r.place_stack(0, &[CARD_ID, "DEMON3"]);
    push_trash(&mut r, 0, "TITAMON");
    trash_my_hand(&mut r, 0);
    pick_trash(&mut r, 0, "TITAMON");
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, host), "TITAMON");
}

#[test]
fn bt24_009_inherited_titan_host_also_qualifies() {
    let mut r = setup();
    let host = r.place_stack(0, &[CARD_ID, "TITAN3"]);
    push_trash(&mut r, 0, "TITAN4");
    trash_my_hand(&mut r, 0);
    pick_trash(&mut r, 0, "TITAN4");
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, host), "TITAN4");
}

#[test]
fn bt24_009_inherited_shaman_host_does_not_qualify() {
    // [Shaman] is NOT one of the inherited host traits (Demon / Titan only).
    let mut r = setup();
    let host = r.place_stack(0, &[CARD_ID, "SHAMAN3"]);
    push_trash(&mut r, 0, "TITAN4");
    trash_my_hand(&mut r, 0);
    assert!(r.pending_selection_view().is_none());
    assert_eq!(top_id(&r, host), "SHAMAN3");
}

#[test]
fn bt24_009_inherited_requires_demon_or_titan_host() {
    let mut r = setup();
    let host = r.place_stack(0, &[CARD_ID, "PLAIN3"]);
    push_trash(&mut r, 0, "TITAN4");
    trash_my_hand(&mut r, 0);
    assert!(r.pending_selection_view().is_none());
    assert_eq!(top_id(&r, host), "PLAIN3");
}

#[test]
fn bt24_009_inherited_decline_refunds_then_once_per_turn_locks() {
    let mut r = setup();
    let host = r.place_stack(0, &[CARD_ID, "DEMON3"]);
    push_trash(&mut r, 0, "TITAN4");
    trash_my_hand(&mut r, 0);
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, host), "DEMON3");
    // Declined ⇒ the use is refunded: a second hand trash offers it again.
    push_hand(&mut r, 0, "FILLER");
    let t = r.perm_handle(0, 1);
    fire(&mut r, EffectTiming::OnPlay, t);
    pick_hand(&mut r, 0, "FILLER");
    pick_trash(&mut r, 0, "TITAN4");
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, host), "TITAN4");
    // Used ⇒ [Once Per Turn] lockout (the card is still a source under TITAN4).
    push_trash(&mut r, 0, "TITAN4B");
    push_hand(&mut r, 0, "FILLER");
    fire(&mut r, EffectTiming::OnPlay, t);
    pick_hand(&mut r, 0, "FILLER");
    assert!(
        r.pending_selection_view().is_none(),
        "second activation in the same turn must not be offered"
    );
    assert_eq!(top_id(&r, host), "TITAN4");
}

#[test]
fn bt24_009_inherited_not_on_opponents_turn() {
    let mut r = setup();
    r.set_first_player(1);
    let host = r.place_stack(0, &[CARD_ID, "DEMON3"]);
    push_trash(&mut r, 0, "TITAN4");
    trash_my_hand(&mut r, 0);
    assert!(r.pending_selection_view().is_none(), "[Your Turn] only");
    assert_eq!(top_id(&r, host), "DEMON3");
}

#[test]
fn bt24_009_inherited_ignores_opponent_hand_trash() {
    let mut r = setup();
    let host = r.place_stack(0, &[CARD_ID, "DEMON3"]);
    push_trash(&mut r, 0, "TITAN4");
    trash_my_hand(&mut r, 1);
    assert!(r.pending_selection_view().is_none());
    assert_eq!(top_id(&r, host), "DEMON3");
}

#[test]
fn bt24_009_inherited_not_from_face_up_top() {
    // Inherited only: face-up on top, the clause does not fire.
    let mut r = setup();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    push_trash(&mut r, 0, "TITAN4");
    trash_my_hand(&mut r, 0);
    assert!(r.pending_selection_view().is_none());
    assert_eq!(top_id(&r, me), CARD_ID);
}

// ─── Metadata ────────────────────────────────────────────────────────────────

#[test]
fn bt24_009_metadata_colors_and_traits() {
    let r = setup();
    let c = r.compiled_card(CARD_ID).unwrap();
    assert_eq!(c.level, Some(3));
    assert_eq!(c.color.len(), 2, "Red + Purple");
    assert_eq!(c.traits, vec!["Demon", "Titan", "TS"]);
    assert_eq!(c.effects.len(), 2);
}

// ─── [On Play] By trashing 1 [Demon]/[Shaman]/[Titan] card, <Draw 2> ────────

#[test]
fn bt24_009_on_play_structure() {
    let t = triggered(0);
    assert_eq!(t.when, vec![CompiledTiming::OnPlay]);
    assert_eq!(t.scope, CompiledScope::FaceUp);
    assert!(!t.once_per_turn);
}

#[test]
fn bt24_009_on_play_trash_demon_card_draws_two() {
    let mut r = setup();
    push_hand(&mut r, 0, CARD_ID);
    push_hand(&mut r, 0, "DEMON3");
    push_hand(&mut r, 0, "FILLER");
    r.play(0, 0).expect("play Shamanmon");
    let v = r.pending_selection_view().expect("optional trash cost");
    assert!(v.is_optional, "the 'By trashing' cost can be declined");
    assert_eq!(non_pass(&r).len(), 1, "FILLER has no qualifying trait");
    pick_hand(&mut r, 0, "DEMON3");
    let _ = r.auto_resolve();
    assert_eq!(trash_ids(&r, 0), vec!["DEMON3".to_string()]);
    assert_eq!(r.deck_size(0), 6, "<Draw 2>");
    assert_eq!(r.hand_size(0), 3);
}

#[test]
fn bt24_009_on_play_declining_cost_draws_nothing() {
    let mut r = setup();
    push_hand(&mut r, 0, CARD_ID);
    push_hand(&mut r, 0, "DEMON3");
    r.play(0, 0).expect("play Shamanmon");
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(hand_ids(&r, 0), vec!["DEMON3".to_string()]);
    assert_eq!(r.deck_size(0), 8);
    assert!(trash_ids(&r, 0).is_empty());
}

#[test]
fn bt24_009_on_play_accepts_demon_shaman_or_titan_any_card_kind() {
    let mut r = setup();
    push_hand(&mut r, 0, CARD_ID);
    for id in ["DEMON3", "SHAMAN3", "TITAN-TAMER", "DEMON-TAMER", "PLAIN3", "FILLER"] {
        push_hand(&mut r, 0, id);
    }
    r.play(0, 0).expect("play Shamanmon");
    assert_eq!(non_pass(&r).len(), 4);
    pick_hand(&mut r, 0, "TITAN-TAMER");
    let _ = r.auto_resolve();
    assert_eq!(trash_ids(&r, 0), vec!["TITAN-TAMER".to_string()]);
    assert_eq!(r.deck_size(0), 6);
}

#[test]
fn bt24_009_on_play_without_qualifying_card_does_nothing() {
    let mut r = setup();
    push_hand(&mut r, 0, CARD_ID);
    push_hand(&mut r, 0, "PLAIN3");
    r.play(0, 0).expect("play Shamanmon");
    assert!(r.pending_selection_view().is_none());
    assert_eq!(r.deck_size(0), 8);
    assert_eq!(hand_ids(&r, 0), vec!["PLAIN3".to_string()]);
}

#[test]
fn bt24_009_on_play_cost_trash_feeds_another_shamanmon_inherited() {
    // The cost is a hand trash: a second Shamanmon buried under a [Demon]
    // Digimon sees "your hand is trashed from" and may digivolve it.
    let mut r = setup();
    let host = r.place_stack(0, &[CARD_ID, "DEMON3"]);
    push_trash(&mut r, 0, "TITAN4");
    push_hand(&mut r, 0, CARD_ID);
    push_hand(&mut r, 0, "SHAMAN3");
    r.play(0, 0).expect("play Shamanmon");
    pick_hand(&mut r, 0, "SHAMAN3");
    let v = r.pending_selection_view().expect("inherited digivolve offer");
    assert!(v.is_optional);
    pick_trash(&mut r, 0, "TITAN4");
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, host), "TITAN4");
    // 8 − 2 (<Draw 2>) − 1 (the digivolution's own draw).
    assert_eq!(r.deck_size(0), 5, "the <Draw 2> still resolved");
    assert_eq!(r.memory(), 5 - 3 - 2);
}
