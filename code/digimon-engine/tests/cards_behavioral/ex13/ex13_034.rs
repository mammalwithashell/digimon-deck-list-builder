//! EX13-034 Wisemon — Digimon, Lv.5, Yellow/Black, 6000 DP, cost 6.
//! Traits: Wizard. Ultimate / Virus.
//! Digivolve: Yellow Lv.4 cost 4; Black Lv.4 cost 4;
//! [Digivolve] Lv.4 w/[Witchelny] in text: Cost 3.
//!
//! # Card text (official Bandai DB bundle `data/card_bundles/EX13-034.md`)
//!
//! <Barrier> [On Play] [When Digivolving] [When Attacking] [Once Per Turn]
//! Until your opponent's turn ends, 1 of your Digimon gains <Reboot> and
//! <Blocker>, and their <De-Digivolve> effects don't affect it. [All Turns]
//! [Once Per Turn] When your security stack is removed from, <De-Digivolve 1>
//! 1 of your opponent's Digimon. Then, if you have 3 or fewer security cards,
//! 1 of their Digimon can't digivolve until their turn ends.
//!
//! Inherited Effect: [All Turns] [Once Per Turn] When your security stack is
//! removed from, this Digimon may unsuspend.
//!
//! # DCGO C# reference
//! None at b9a0638cd (no `EX13_034.cs`); printed text + general_rule.pdf govern.
//! Idioms: BT21-074 / EX10-031 (`CannotBeDeDigivolved` opponent-scoped
//! protection), AD1-017 / EX13-003 (`on_own_security_removed`).
//!
//! # Patterns
//! - H: printed <Barrier>.  - C: text-gated special digivolve.
//! - D4: timed keyword grants + De-Digivolve immunity on a chosen ally.
//! - F2: security-removed observer → De-Digivolve + conditional can't-digivolve.
//! - G4/E2: inherited OPT optional self-unsuspend.

#![allow(dead_code, unused_imports)]

use super::orphan_support::*;
use digimon_dsl::compiled::{CompiledClause, CompiledColor, CompiledScope, CompiledTiming};
use digimon_engine::action::space::encode_digivolve;
use digimon_engine::enums::{CardColor, Keyword};
use digimon_engine::enums::ModifierType;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "EX13-034";

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-034 YAML loads")
        .add_card(named("YEL-L4", "Yellowmon", CardColor::Yellow, 4, 5000))
        .add_card(named("BLK-L4", "Blackmon", CardColor::Black, 4, 5000))
        .add_card(named("RED-WITCH-L4", "Witchelny Mage", CardColor::Red, 4, 5000))
        .add_card(named("RED-L4", "Redmon", CardColor::Red, 4, 5000))
        .add_card(named("ALLY", "Allymon", CardColor::Yellow, 4, 5000))
        .add_card(named("OPP-LV3", "OppThree", CardColor::Red, 3, 3000))
        .add_card(named("OPP-LV4", "OppFour", CardColor::Red, 4, 5000))
        .add_card(named("TOP", "Topmon", CardColor::Yellow, 6, 10000))
        .add_card(named("FILL", "Filler", CardColor::Yellow, 3, 3000))
        .add_card(named("SEC", "SecCard", CardColor::Yellow, 3, 1000))
}

fn start(hand: &[&str], my_sec: usize) -> DebugRunner {
    builder()
        .hand(0, hand)
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .security(0, &vec!["SEC"; my_sec])
        .security(1, &["SEC", "SEC", "SEC"])
        .memory(8)
        .start()
}

fn digivolve_onto(r: &mut DebugRunner, base: PermanentHandle) {
    let slot = hand_index(r, 0, CARD_ID) as u16;
    r.game.decode_action(encode_digivolve(slot, base.index as u16), 0);
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn ex13_034_printed_metadata_paths_barrier_and_clauses() {
    let mut r = start(&[], 5);
    let c = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!((c.level, c.dp, c.cost), (Some(5), Some(6000), Some(6)));
    assert_eq!(c.color, vec![CompiledColor::Yellow, CompiledColor::Black]);
    assert_eq!(c.alt_paths.len(), 3);
    let t: Vec<_> = c
        .effects
        .iter()
        .filter_map(|e| match e {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(t.len(), 3);
    for w in [CompiledTiming::OnPlay, CompiledTiming::WhenDigivolving, CompiledTiming::WhenAttacking] {
        assert!(t[0].when.contains(&w));
    }
    assert!(t[0].once_per_turn && t[1].once_per_turn && t[2].once_per_turn);
    assert_eq!(t[1].scope, CompiledScope::FaceUp);
    assert_eq!(t[2].scope, CompiledScope::Inherited);
    assert!(t[2].optional, "'may unsuspend'");
    let h = r.place_on_field(0, CARD_ID, Some(0));
    assert!(r.game.has_keyword(h, Keyword::Barrier));
}

// ─── Digivolve conditions ────────────────────────────────────────────────────

#[test]
fn ex13_034_digivolves_from_witchelny_text_lv4_for_3() {
    let mut r = start(&[CARD_ID], 5);
    let base = r.place_on_field(0, "RED-WITCH-L4", Some(0));
    digivolve_onto(&mut r, base);
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, base), CARD_ID);
    assert_eq!(r.memory(), 5, "special condition cost 3");
}

#[test]
fn ex13_034_digivolves_from_black_lv4_for_4() {
    let mut r = start(&[CARD_ID], 5);
    let base = r.place_on_field(0, "BLK-L4", Some(0));
    digivolve_onto(&mut r, base);
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, base), CARD_ID);
    assert_eq!(r.memory(), 4);
}

#[test]
fn ex13_034_cannot_digivolve_from_plain_red_lv4() {
    let mut r = start(&[CARD_ID], 5);
    let base = r.place_on_field(0, "RED-L4", Some(0));
    digivolve_onto(&mut r, base);
    assert_eq!(top_id(&r, base), "RED-L4");
}

// ─── Section 3/5 — OP/WD/WA protection grant ─────────────────────────────────

#[test]
fn ex13_034_on_play_grants_reboot_blocker_and_de_digivolve_immunity_to_chosen_ally() {
    let mut r = start(&[CARD_ID], 5);
    let ally = r.place_on_field(0, "ALLY", Some(0));
    play_card(&mut r, 0, CARD_ID);
    assert_eq!(r.pending_kind(), Some(SelectionKind::OwnField));
    assert!(!r.pending_is_optional(), "'1 of your Digimon' is mandatory");
    pick_field(&mut r, ally);
    let _ = r.auto_resolve();
    assert!(r.game.has_keyword(ally, Keyword::Reboot));
    assert!(r.game.has_keyword(ally, Keyword::Blocker));
    assert!(r.game.modifiers.has(ally, ModifierType::CannotBeDeDigivolved));
    let me = handle_of(&r, 0, CARD_ID);
    assert!(!r.game.has_keyword(me, Keyword::Blocker), "only the chosen Digimon");
}

#[test]
fn ex13_034_grant_lasts_through_the_opponents_turn_then_expires() {
    let mut r = start(&[CARD_ID], 5);
    let ally = r.place_on_field(0, "ALLY", Some(0));
    play_card(&mut r, 0, CARD_ID);
    pick_field(&mut r, ally);
    let _ = r.auto_resolve();
    end_turn_declining(&mut r);
    assert!(r.game.has_keyword(ally, Keyword::Blocker), "still on the opponent's turn");
    r.game.memory = 3;
    end_turn_declining(&mut r);
    assert!(!r.game.has_keyword(ally, Keyword::Blocker), "expired when their turn ended");
    assert!(!r.game.has_keyword(ally, Keyword::Reboot));
    assert!(!r.game.modifiers.has(ally, ModifierType::CannotBeDeDigivolved));
}

#[test]
fn ex13_034_protection_clause_is_once_per_turn_across_timings() {
    let mut r = start(&[CARD_ID], 5);
    play_card(&mut r, 0, CARD_ID);
    let me = handle_of(&r, 0, CARD_ID);
    pick_field(&mut r, me);
    let _ = r.auto_resolve();
    r.game.players[0].battle_area[me.index as usize].turn_played = 0;
    r.attack_player(me, 1, false);
    assert_ne!(
        r.pending_kind(),
        Some(SelectionKind::OwnField),
        "[When Attacking] shares the once-per-turn with [On Play]"
    );
}

// ─── Section 2/3/5 — security-removed De-Digivolve ───────────────────────────

#[test]
fn ex13_034_security_removed_de_digivolves_and_locks_digivolve_at_3_or_fewer() {
    let mut r = start(&[], 4);
    r.place_on_field(0, CARD_ID, Some(0));
    let opp = r.place_stack(1, &["OPP-LV3", "OPP-LV4"]);
    let other = r.place_on_field(1, "OPP-LV4", Some(0));
    trash_security_by(&mut r, 0, 1);
    assert_eq!(r.pending_kind(), Some(SelectionKind::OppField), "De-Digivolve target");
    pick_field(&mut r, opp);
    // Now 3 security → can't-digivolve target.
    assert_eq!(r.pending_kind(), Some(SelectionKind::OppField), "can't-digivolve target");
    pick_field(&mut r, other);
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, opp), "OPP-LV3");
    assert!(r.game.modifiers.has(other, ModifierType::CannotDigivolve));
    assert!(!r.game.modifiers.has(opp, ModifierType::CannotDigivolve));
}

#[test]
fn ex13_034_security_removed_with_4_left_skips_the_digivolve_lock() {
    let mut r = start(&[], 5);
    r.place_on_field(0, CARD_ID, Some(0));
    let opp = r.place_stack(1, &["OPP-LV3", "OPP-LV4"]);
    trash_security_by(&mut r, 0, 1);
    pick_field(&mut r, opp);
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, opp), "OPP-LV3");
    assert!(!r.game.modifiers.has(opp, ModifierType::CannotDigivolve), "4 security > 3");
}

#[test]
fn ex13_034_security_clause_is_once_per_turn() {
    let mut r = start(&[], 5);
    r.place_on_field(0, CARD_ID, Some(0));
    let opp = r.place_stack(1, &["OPP-LV3", "OPP-LV4"]);
    trash_security_by(&mut r, 0, 1);
    pick_field(&mut r, opp);
    let _ = r.auto_resolve();
    r.place_stack(1, &["OPP-LV3", "OPP-LV4"]);
    trash_security_by(&mut r, 0, 1);
    assert!(r.pending_selection().is_none(), "OPT spent");
}

#[test]
fn ex13_034_opponent_security_removal_does_not_trigger() {
    let mut r = start(&[], 5);
    r.place_on_field(0, CARD_ID, Some(0));
    r.place_stack(1, &["OPP-LV3", "OPP-LV4"]);
    trash_security_by(&mut r, 1, 0);
    assert!(r.pending_selection().is_none(), "'your security stack' only");
}

// ─── Section 2/3/5 — inherited unsuspend ─────────────────────────────────────

#[test]
fn ex13_034_inherited_security_removed_may_unsuspend_carrier() {
    let mut r = start(&[], 5);
    let carrier = r.place_stack(0, &[CARD_ID, "TOP"]);
    set_suspended(&mut r, carrier, true);
    trash_security_by(&mut r, 0, 1);
    assert!(r.pending_is_optional());
    accept(&mut r);
    let _ = r.auto_resolve();
    assert!(!is_suspended(&r, carrier));
}

#[test]
fn ex13_034_inherited_decline_keeps_carrier_suspended() {
    let mut r = start(&[], 5);
    let carrier = r.place_stack(0, &[CARD_ID, "TOP"]);
    set_suspended(&mut r, carrier, true);
    trash_security_by(&mut r, 0, 1);
    decline(&mut r);
    let _ = r.auto_resolve();
    assert!(is_suspended(&r, carrier));
}

#[test]
fn ex13_034_inherited_once_per_turn() {
    let mut r = start(&[], 5);
    let carrier = r.place_stack(0, &[CARD_ID, "TOP"]);
    set_suspended(&mut r, carrier, true);
    trash_security_by(&mut r, 0, 1);
    accept(&mut r);
    let _ = r.auto_resolve();
    set_suspended(&mut r, carrier, true);
    trash_security_by(&mut r, 0, 1);
    assert!(r.pending_selection().is_none(), "OPT spent");
}
