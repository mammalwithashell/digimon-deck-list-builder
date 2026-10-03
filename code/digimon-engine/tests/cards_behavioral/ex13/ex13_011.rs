//! EX13-011 BaoHuckmon — Digimon, Lv.4, Red/White, 5000 DP, cost 5.
//! Traits: Dinosaur. Champion / Data.
//! Digivolve: Red Lv.3 cost 2; [Digivolve] Lv.3 w/[Huckmon] in text: Cost 2.
//!
//! # Card text (official Bandai DB bundle `data/card_bundles/EX13-011.md`)
//!
//! <Raid> [On Play] [When Digivolving] If you have 1 or fewer Tamers, you may
//! play 1 [Mon] from your hand without paying the cost.
//!
//! Inherited Effect: [Your Turn] This Digimon gets +2000 DP.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX13/Red/EX13_011.cs
//! - Raid; On Play / When Digivolving: `OwnerHas1OrLessTamers` && a hand card
//!   `EqualsCardName("Mon")` playable for free → optional pick, `PlayPermanentCards(payCost: false)`.
//! - Inherited `ChangeSelfDPStaticEffect(+2000)` on the owner's turn.
//!
//! # Patterns
//! - H: printed <Raid>.  - C: special digivolve condition (text-gated).
//! - B2: Tamer-count-gated free play from hand.  - G4: inherited DP aura.

#![allow(dead_code, unused_imports)]

use super::orphan_support::*;
use digimon_dsl::compiled::{CompiledClause, CompiledColor, CompiledScope, CompiledTiming};
use digimon_engine::action::space::encode_digivolve;
use digimon_engine::enums::{CardColor, Keyword};

const CARD_ID: &str = "EX13-011";

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-011 YAML loads")
        .add_card({
            let mut t = tamer("MON", CardColor::White, 4);
            t.card_name = "Mon".into();
            t
        })
        .add_card({
            let mut t = tamer("MONZA", CardColor::White, 4);
            t.card_name = "Monzaemon".into();
            t
        })
        .add_card(tamer("T1", CardColor::Red, 2))
        .add_card(tamer("T2", CardColor::Red, 2))
        .add_card(named("RED-L3", "Redmon", CardColor::Red, 3, 3000))
        .add_card(named("WHITE-HUCK-L3", "Huckmon", CardColor::White, 3, 2000))
        .add_card(named("WHITE-L3", "Whitemon", CardColor::White, 3, 3000))
        .add_card(named("TOP", "Topmon", CardColor::Red, 5, 7000))
        .add_card(named("FILL", "Filler", CardColor::Red, 3, 3000))
}

fn start(hand: &[&str], mem: i16) -> DebugRunner {
    builder()
        .hand(0, hand)
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(mem)
        .start()
}

fn digivolve_onto(r: &mut DebugRunner, base: digimon_engine::permanent::PermanentHandle) {
    let slot = hand_index(r, 0, CARD_ID) as u16;
    r.game.decode_action(encode_digivolve(slot, base.index as u16), 0);
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn ex13_011_printed_metadata_paths_and_raid() {
    let mut r = start(&[], 5);
    let c = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!((c.level, c.dp, c.cost), (Some(4), Some(5000), Some(5)));
    assert_eq!(c.color, vec![CompiledColor::Red, CompiledColor::White]);
    assert_eq!(c.traits, vec!["Dinosaur".to_string()]);
    assert_eq!(c.alt_paths.len(), 2, "Red Lv.3 circle + Lv.3 w/[Huckmon] in text");
    let h = r.place_on_field(0, CARD_ID, Some(0));
    assert!(r.game.has_keyword(h, Keyword::Raid));
}

#[test]
fn ex13_011_clause_shape() {
    let r = start(&[], 5);
    let c = r.compiled_card(CARD_ID).expect("compiled");
    let t: Vec<_> = c
        .effects
        .iter()
        .filter_map(|e| match e {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(t.len(), 1);
    assert!(t[0].when.contains(&CompiledTiming::OnPlay));
    assert!(t[0].when.contains(&CompiledTiming::WhenDigivolving));
    assert!(t[0].condition.is_some() || t[0].active_when.is_some(), "≤1 Tamer gate");
}

// ─── Digivolve conditions ────────────────────────────────────────────────────

#[test]
fn ex13_011_digivolves_from_red_lv3_for_2() {
    let mut r = start(&[CARD_ID], 5);
    let base = r.place_on_field(0, "RED-L3", Some(0));
    digivolve_onto(&mut r, base);
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, base), CARD_ID);
    assert_eq!(r.memory(), 3);
}

#[test]
fn ex13_011_digivolves_from_huckmon_text_lv3_of_any_color() {
    let mut r = start(&[CARD_ID], 5);
    let base = r.place_on_field(0, "WHITE-HUCK-L3", Some(0));
    digivolve_onto(&mut r, base);
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, base), CARD_ID, "white Lv.3 [Huckmon] qualifies");
    assert_eq!(r.memory(), 3);
}

#[test]
fn ex13_011_cannot_digivolve_from_plain_white_lv3() {
    let mut r = start(&[CARD_ID], 5);
    let base = r.place_on_field(0, "WHITE-L3", Some(0));
    digivolve_onto(&mut r, base);
    assert_eq!(top_id(&r, base), "WHITE-L3");
}

// ─── Section 2/3 — [On Play][When Digivolving] ───────────────────────────────

#[test]
fn ex13_011_when_digivolving_with_no_tamers_plays_mon_free() {
    let mut r = start(&[CARD_ID, "MON"], 5);
    let base = r.place_on_field(0, "RED-L3", Some(0));
    digivolve_onto(&mut r, base);
    assert!(r.pending_is_optional(), "'you may play'");
    assert_eq!(offered_hand_ids(&r), vec!["MON".to_string()]);
    pick_hand(&mut r, "MON");
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&"MON".to_string()), "[Mon] played");
    assert_eq!(r.memory(), 3, "only the digivolve cost was paid");
}

#[test]
fn ex13_011_on_play_with_one_tamer_still_offers_mon() {
    let mut r = start(&[CARD_ID, "MON"], 8);
    r.place_on_field(0, "T1", Some(0));
    play_card(&mut r, 0, CARD_ID);
    assert_eq!(offered_hand_ids(&r), vec!["MON".to_string()]);
    pick_hand(&mut r, "MON");
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&"MON".to_string()));
    assert_eq!(r.memory(), 3, "paid BaoHuckmon's 5 only");
}

#[test]
fn ex13_011_two_tamers_block_the_free_play() {
    let mut r = start(&[CARD_ID, "MON"], 8);
    r.place_on_field(0, "T1", Some(0));
    r.place_on_field(0, "T2", Some(0));
    play_card(&mut r, 0, CARD_ID);
    assert!(r.pending_selection().is_none(), "2 Tamers > 1");
    assert_eq!(hand_ids(&r, 0), vec!["MON".to_string()]);
}

#[test]
fn ex13_011_only_exact_mon_name_is_eligible() {
    let mut r = start(&[CARD_ID, "MONZA"], 8);
    play_card(&mut r, 0, CARD_ID);
    assert!(r.pending_selection().is_none(), "[Monzaemon] is not [Mon]");
}

#[test]
fn ex13_011_declining_plays_nothing() {
    let mut r = start(&[CARD_ID, "MON"], 8);
    play_card(&mut r, 0, CARD_ID);
    decline(&mut r);
    let _ = r.auto_resolve();
    assert_eq!(hand_ids(&r, 0), vec!["MON".to_string()]);
}

// ─── Section 2 — inherited [Your Turn] +2000 ─────────────────────────────────

#[test]
fn ex13_011_inherited_plus_2000_on_your_turn() {
    let mut r = start(&[], 5);
    let h = r.place_stack(0, &[CARD_ID, "TOP"]);
    assert_eq!(r.effective_dp(h), Some(9000));
}

#[test]
fn ex13_011_inherited_no_bonus_on_opponents_turn() {
    let mut r = start(&[], 5);
    let h = r.place_stack(0, &[CARD_ID, "TOP"]);
    r.end_turn();
    let _ = r.auto_resolve();
    assert_eq!(r.turn_player(), 1);
    assert_eq!(r.effective_dp(h), Some(7000));
}
