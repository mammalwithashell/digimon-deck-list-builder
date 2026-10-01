//! EX13-010 Growlmon — Digimon, Lv.4, Red, 6000 DP, cost 6.
//! Traits: Dark Dragon. Champion / Virus. Digivolve: Red Lv.3 cost 2.
//!
//! # Card text (official Bandai DB bundle `data/card_bundles/EX13-010.md`)
//!
//! [When Moving] [When Digivolving] Delete 1 of your opponent's Digimon with
//! 4000 DP or less. If this effect didn't delete, this Digimon gains <Raid> and
//! +3000 DP for the turn.
//!
//! Inherited Effect: [All Turns] Add 2000 to this Digimon's DP deletion
//! effects' maximums.
//!
//! Official Q&A: the delete is mandatory — with a 4000-DP-or-less opponent
//! Digimon you must select and delete it.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX13/Red/EX13_010.cs
//! - Shared WM/WD (mandatory): select opponent Digimon `DP <=
//!   MaxDP_DeleteEffect(4000)` (canNoSelect false) → delete; failure →
//!   +3000 DP and <Raid> until turn end.
//! - Inherited `ChangeDPDeleteEffectMaxDPClass` (+2000, own stack's effects).
//!
//! # Patterns
//! - F1 mandatory DP-capped delete with a "didn't delete" fallback.
//! - G-ENGINE-DP-DELETION-MAX-MODIFIER (inherited +2000; raises the cap of
//!   EX13-013 WarGrowlmon's 5000 delete when this Growlmon is a source).

#![allow(dead_code, unused_imports)]

use super::orphan_support::*;
use digimon_dsl::compiled::{
    CompiledClause, CompiledColor, CompiledDeclarativeClause, CompiledScope, CompiledStep,
    CompiledTiming,
};
use digimon_engine::action::space::encode_digivolve;
use digimon_engine::enums::{CardColor, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "EX13-010";
const GUILMON: &str = "EX13-007";
const WARGROWLMON: &str = "EX13-013";

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-010 YAML loads")
        .dsl_card(GUILMON)
        .expect("EX13-007 YAML loads")
        .dsl_card(WARGROWLMON)
        .expect("EX13-013 YAML loads")
        .add_card(named("RED-L3", "Redmon", CardColor::Red, 3, 3000))
        .add_card(named("OPP-4K", "OppFour", CardColor::Blue, 4, 4000))
        .add_card(named("OPP-5K", "OppFive", CardColor::Blue, 4, 5000))
        .add_card(named("OPP-6K", "OppSix", CardColor::Blue, 4, 6000))
        .add_card(named("OPP-7K", "OppSeven", CardColor::Blue, 4, 7000))
        .add_card(named("OPP-9K", "OppNine", CardColor::Blue, 5, 9000))
        .add_card(named("OPP-10K", "OppTen", CardColor::Blue, 5, 10000))
        .add_card(named("FILL", "Filler", CardColor::Red, 3, 3000))
        .add_card(named("SEC", "SecCard", CardColor::Red, 3, 1000))
}

fn start(hand: &[&str], mem: i16) -> DebugRunner {
    builder()
        .hand(0, hand)
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .security(1, &["SEC", "SEC", "SEC"])
        .memory(mem)
        .start()
}

fn digivolve_onto(r: &mut DebugRunner, id: &str, base: PermanentHandle) {
    let slot = hand_index(r, 0, id) as u16;
    r.game
        .decode_action(encode_digivolve(slot, base.index as u16), 0);
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn ex13_010_printed_metadata_and_clause_shape() {
    let r = start(&[], 5);
    let c = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!((c.level, c.dp, c.cost), (Some(4), Some(6000), Some(6)));
    assert_eq!(c.color, vec![CompiledColor::Red]);
    assert_eq!(c.traits, vec!["Dark Dragon".to_string()]);
    let CompiledClause::Triggered(t) = &c.effects[0] else {
        panic!("clause 0 triggered");
    };
    assert!(t.when.contains(&CompiledTiming::OnMove));
    assert!(t.when.contains(&CompiledTiming::WhenDigivolving));
    assert!(!t.optional, "the delete is mandatory (Official Q&A)");
    let CompiledStep::SelectOpponentPermanent {
        filter, optional, ..
    } = &t.process[0]
    else {
        panic!("select first");
    };
    assert!(!optional);
    assert!(
        filter
            .all_of
            .iter()
            .any(|p| p.dp_lte.is_some() && p.dp_lte_deletion_cap),
        "the 4000 cap is flagged as a DP deletion effect's maximum"
    );
    let CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
        scope,
        modifier,
        modifier_value,
        ..
    }) = &c.effects[1]
    else {
        panic!("clause 1 is the inherited aura");
    };
    assert_eq!(*scope, CompiledScope::Inherited);
    assert_eq!(modifier.as_deref(), Some("ChangeDPDeleteEffectMaxDP"));
    assert_eq!(*modifier_value, Some(2000));
}

// ─── Section 2 — [When Digivolving] delete-or-buff ───────────────────────────

#[test]
fn ex13_010_when_digivolving_must_delete_a_4000_or_less_digimon() {
    let mut r = start(&[CARD_ID], 5);
    let base = r.place_on_field(0, "RED-L3", Some(0));
    let small = r.place_on_field(1, "OPP-4K", Some(0));
    let big = r.place_on_field(1, "OPP-5K", Some(0));
    digivolve_onto(&mut r, CARD_ID, base);
    assert_eq!(r.pending_kind(), Some(SelectionKind::OppField));
    assert!(!r.pending_is_optional(), "mandatory delete");
    assert!(
        field_offered(&r, small) && !field_offered(&r, big),
        "4000 DP or less only"
    );
    pick_field(&mut r, small);
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 1), vec!["OPP-5K".to_string()]);
    assert_eq!(r.effective_dp(base), Some(6000), "deleted → no +3000");
    assert!(!r.game.has_keyword(base, Keyword::Raid));
}

#[test]
fn ex13_010_when_digivolving_without_target_gains_raid_and_3000() {
    let mut r = start(&[CARD_ID], 5);
    let base = r.place_on_field(0, "RED-L3", Some(0));
    r.place_on_field(1, "OPP-5K", Some(0));
    digivolve_onto(&mut r, CARD_ID, base);
    let _ = r.auto_resolve();
    assert_eq!(r.effective_dp(base), Some(9000));
    assert!(r.game.has_keyword(base, Keyword::Raid));
}

#[test]
fn ex13_010_buff_lasts_for_the_turn_only() {
    let mut r = start(&[CARD_ID], 5);
    let base = r.place_on_field(0, "RED-L3", Some(0));
    digivolve_onto(&mut r, CARD_ID, base);
    let _ = r.auto_resolve();
    assert_eq!(r.effective_dp(base), Some(9000));
    end_turn_declining(&mut r);
    assert_eq!(r.effective_dp(base), Some(6000));
    assert!(!r.game.has_keyword(base, Keyword::Raid));
}

// ─── Section 2 — [When Moving] ───────────────────────────────────────────────

#[test]
fn ex13_010_when_moving_deletes() {
    let mut r = start(&[], 5);
    let small = r.place_on_field(1, "OPP-4K", Some(0));
    r.place_in_breeding(0, CARD_ID);
    assert!(r.move_from_breeding(0), "Growlmon moves to the battle area");
    assert_eq!(r.pending_kind(), Some(SelectionKind::OppField));
    pick_field(&mut r, small);
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 1).is_empty());
}

#[test]
fn ex13_010_another_digimon_moving_does_not_fire() {
    let mut r = start(&[], 5);
    let me = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(1, "OPP-4K", Some(0));
    r.place_in_breeding(0, "RED-L3");
    assert!(r.move_from_breeding(0));
    assert!(
        r.pending_selection().is_none(),
        "[When Moving] is self-only"
    );
    assert_eq!(r.effective_dp(me), Some(6000));
}

// ─── Section 3 — Inherited: +2000 to DP deletion maximums ────────────────────

#[test]
fn ex13_010_inherited_raises_wargrowlmons_deletion_cap_to_7000() {
    let mut r = start(&[WARGROWLMON], 5);
    let base = r.place_on_field(0, CARD_ID, Some(0));
    let o7 = r.place_on_field(1, "OPP-7K", Some(0));
    let o9 = r.place_on_field(1, "OPP-9K", Some(0));
    digivolve_onto(&mut r, WARGROWLMON, base);
    assert_eq!(r.pending_kind(), Some(SelectionKind::OppField));
    assert!(field_offered(&r, o7), "5000 + 2000 (Growlmon inherited)");
    assert!(!field_offered(&r, o9));
    pick_field(&mut r, o7);
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 1), vec!["OPP-9K".to_string()]);
}

#[test]
fn ex13_010_guilmon_and_growlmon_sources_stack_to_9000() {
    let mut r = start(&[WARGROWLMON], 5);
    let base = r.place_stack(0, &[GUILMON, CARD_ID]);
    let o9 = r.place_on_field(1, "OPP-9K", Some(0));
    let o10 = r.place_on_field(1, "OPP-10K", Some(0));
    digivolve_onto(&mut r, WARGROWLMON, base);
    assert!(field_offered(&r, o9), "5000 + 2000 + 2000");
    assert!(!field_offered(&r, o10));
}

#[test]
fn ex13_010_face_up_growlmon_does_not_raise_its_own_cap() {
    // Growlmon's inherited only applies as a source; the face-up Growlmon on a
    // plain Lv.3 keeps the printed 4000 cap.
    let mut r = start(&[CARD_ID], 5);
    let base = r.place_on_field(0, "RED-L3", Some(0));
    let o4 = r.place_on_field(1, "OPP-4K", Some(0));
    let o6 = r.place_on_field(1, "OPP-6K", Some(0));
    digivolve_onto(&mut r, CARD_ID, base);
    assert!(field_offered(&r, o4));
    assert!(!field_offered(&r, o6));
}
