//! EX13-024 Slayerdramon — Digimon, Lv.6, Blue/Red, 12000 DP, cost 12.
//! Traits: Dragonkin. Mega / Vaccine.
//! Digivolve: Blue Lv.5 cost 4; Red Lv.5 cost 4;
//! [Digivolve] [Wingdramon]/[Groundramon]: Cost 3.
//! Assembly -5: Lv.5 × Lv.4 × Lv.3, all w/[Dracomon]/[Examon] in text.
//!
//! # Card text (official Bandai DB bundle `data/card_bundles/EX13-024.md`)
//!
//! <Raid> <Blocker> [On Play] [When Digivolving] For each of this Digimon's
//! digivolution cards, trash any 1 digivolution card from your opponent's
//! Digimon. Then, you may return all of their Digimon with the fewest
//! digivolution cards to the bottom of the deck. [All Turns] [Once Per Turn]
//! When any of your [Dracomon] or [Examon] text Digimon would leave the battle
//! area, by suspending 1 of your such Digimon, they don't leave.
//!
//! Inherited Effect: [All Turns] [Once Per Turn] When any of your [Dracomon] or
//! [Examon] text Digimon would leave the battle area, by suspending 1 of your
//! such Digimon, they don't leave.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX13/Blue/EX13_024.cs
//! - Alt digivolve `EqualsCardName("Wingdramon"|"Groundramon")` cost 3; Raid;
//!   Blocker; Assembly elements Lv.5/Lv.4/Lv.3 `HasText("Dracomon"|"Examon")`,
//!   reduce 5.
//! - Shared OP/WD: `SelectTrashDigivolutionCards(maxCount = own source count,
//!   canNoTrash: false, isFromOnly1Permanent: false)`, then a Yes/No to deck-
//!   bottom the fewest-source opponent Digimon. (DCGO restricts that set to
//!   opponent Digimon that still have a digivolution card; printed text —
//!   "all of their Digimon with the fewest digivolution cards" — governs, so a
//!   0-source Digimon counts.)
//! - `WhenRemoveField` × {face-up, inherited}, each its own OPT hash: any own
//!   [Dracomon]/[Examon]-text Digimon leaving (any cause) → optional pick of an
//!   unsuspended such Digimon to suspend (`RemoveUse()` on no pick) → all the
//!   leaving matching Digimon stay.
//!
//! # Patterns
//! - H: printed <Raid>, <Blocker>.  - C: Assembly + named special digivolve.
//! - F3: count-scaled digivolution-card trash + fewest-sources deck-bottom.
//! - F4: OPT replacement (face-up + inherited) with a suspend cost.

#![allow(dead_code, unused_imports)]

use super::orphan_support::*;
use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledDeclarativeClause,
    CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::{encode_digivolve, PASS, PLAY_HAND_START, TRASH_EFFECT_START};
use digimon_engine::enums::{CardColor, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "EX13-024";

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-024 YAML loads")
        .add_card(with_text(
            named("MAT-L5", "Coredramon", CardColor::Blue, 5, 7000),
            "Treated as an [Examon] material.",
        ))
        .add_card(named("MAT-L4", "Dracomon Four", CardColor::Blue, 4, 5000))
        .add_card(named("MAT-L3", "Dracomon", CardColor::Blue, 3, 3000))
        .add_card(named("PLAIN-L4", "Plain Four", CardColor::Blue, 4, 5000))
        .add_card(named("WING", "Wingdramon", CardColor::Green, 5, 7000))
        .add_card(named("BLUE-L5", "Bluemon", CardColor::Blue, 5, 7000))
        .add_card(named("SRC", "Sourcemon", CardColor::Blue, 3, 3000))
        .add_card(named("DRACO", "Dracomon Ally", CardColor::Blue, 4, 5000))
        .add_card(named("DRACO2", "Dracomon Pal", CardColor::Blue, 4, 5000))
        .add_card(named("PLAIN", "Plainmon", CardColor::Blue, 4, 5000))
        .add_card(named("OPP-A", "OppA", CardColor::Red, 5, 7000))
        .add_card(named("OPP-B", "OppB", CardColor::Red, 4, 5000))
        .add_card(named("OPP-S", "OppSrc", CardColor::Red, 3, 3000))
        .add_card(named("FILL", "Filler", CardColor::Blue, 3, 3000))
}

fn start(hand: &[&str], mem: i16) -> DebugRunner {
    builder()
        .hand(0, hand)
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .memory(mem)
        .start()
}

fn digivolve_onto(r: &mut DebugRunner, base: PermanentHandle) {
    let slot = hand_index(r, 0, CARD_ID) as u16;
    r.game.decode_action(encode_digivolve(slot, base.index as u16), 0);
}

/// Pick the first legal action `n` times at the pending source prompt.
fn pick_any_sources(r: &mut DebugRunner, n: usize) {
    for _ in 0..n {
        let v = r.pending_selection_view().expect("source prompt");
        let a = v.valid_action_ids.iter().copied().find(|&a| a != PASS).expect("a source pick");
        r.execute_action(v.selecting_player, a).expect("pick source");
    }
}

fn opp_source_total(r: &DebugRunner) -> usize {
    r.game.players[1]
        .battle_area
        .iter()
        .map(|p| p.card_sources.len().saturating_sub(1))
        .sum()
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn ex13_024_printed_metadata_paths_and_keywords() {
    let mut r = start(&[], 5);
    let c = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!((c.level, c.dp, c.cost), (Some(6), Some(12000), Some(12)));
    assert_eq!(c.color, vec![CompiledColor::Blue, CompiledColor::Red]);
    let digi = c.alt_paths.iter().filter(|p| p.kind == CompiledAltPathKind::Digivolve).count();
    assert_eq!(digi, 3, "Blue Lv.5, Red Lv.5, [Wingdramon]/[Groundramon]");
    let asm = c
        .alt_paths
        .iter()
        .find(|p| p.kind == CompiledAltPathKind::Assembly)
        .expect("Assembly path");
    assert_eq!(asm.cost, Some(CompiledCost::Literal(5)));
    assert_eq!(asm.materials.len(), 3);
    let h = r.place_on_field(0, CARD_ID, Some(0));
    assert!(r.game.has_keyword(h, Keyword::Raid));
    assert!(r.game.has_keyword(h, Keyword::Blocker));
}

#[test]
fn ex13_024_clause_shape() {
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
    let mut reps: Vec<_> = c
        .effects
        .iter()
        .filter_map(|e| match e {
            CompiledClause::Declarative(CompiledDeclarativeClause::Replacement {
                scope,
                optional,
                once_per_turn,
                ..
            }) => Some((*scope == CompiledScope::Inherited, *optional, *once_per_turn)),
            _ => None,
        })
        .collect();
    reps.sort();
    assert_eq!(reps, vec![(false, true, true), (true, true, true)]);
}

// ─── Assembly / digivolve conditions ─────────────────────────────────────────

#[test]
fn ex13_024_assembly_places_three_materials_then_trashes_three_opponent_sources() {
    let mut r = start(&[CARD_ID], 10);
    r.inject_trash(0, "MAT-L5");
    r.inject_trash(0, "MAT-L4");
    r.inject_trash(0, "MAT-L3");
    r.place_stack(1, &["OPP-S", "OPP-S", "OPP-S", "OPP-S", "OPP-A"]);
    let slot = hand_index(&r, 0, CARD_ID) as u16;
    r.game.decode_action(PLAY_HAND_START + slot, 0);
    for _ in 0..3 {
        let v = r.pending_selection_view().expect("Assembly material prompt");
        let a = v
            .valid_action_ids
            .iter()
            .copied()
            .find(|&a| a >= TRASH_EFFECT_START && a != PASS)
            .expect("a trash material");
        r.execute_action(v.selecting_player, a).expect("pick material");
    }
    let me = handle_of(&r, 0, CARD_ID);
    assert_eq!(source_count(&r, me), 3);
    pick_any_sources(&mut r, 3);
    assert_eq!(opp_source_total(&r), 1, "4 − 3 trashed");
    // Then: may return the fewest-source Digimon — decline.
    assert_eq!(r.pending_kind(), Some(SelectionKind::EffectChoice));
    r.execute_branch(1).unwrap();
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 1), vec!["OPP-A".to_string()]);
}

#[test]
fn ex13_024_digivolves_from_wingdramon_for_3() {
    let mut r = start(&[CARD_ID], 5);
    let base = r.place_on_field(0, "WING", Some(0));
    digivolve_onto(&mut r, base);
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, base), CARD_ID);
}

// ─── Section 3 — [When Digivolving] trash + fewest bottom ────────────────────

#[test]
fn ex13_024_when_digivolving_trashes_one_per_own_source_and_may_bottom_fewest() {
    let mut r = start(&[CARD_ID], 8);
    let base = r.place_stack(0, &["SRC", "BLUE-L5"]); // after digivolving: 2 sources
    let a = r.place_stack(1, &["OPP-S", "OPP-S", "OPP-S", "OPP-A"]); // 3 sources
    r.place_stack(1, &["OPP-S", "OPP-B"]); // 1 source
    digivolve_onto(&mut r, base);
    assert!(
        matches!(r.pending_kind(), Some(SelectionKind::SourceMulti { min: 2, max: 2, .. })),
        "one pick per own digivolution card (2): {:?}",
        r.pending_kind()
    );
    assert!(!r.pending_is_optional(), "trash is mandatory");
    pick_any_sources(&mut r, 2);
    assert_eq!(opp_source_total(&r), 2, "two trashed");
    assert_eq!(r.pending_kind(), Some(SelectionKind::EffectChoice), "'you may return'");
    r.execute_branch(0).unwrap();
    let _ = r.auto_resolve();
    let _ = a;
    // Whatever was trashed, every opponent Digimon tied for fewest left the field
    // and the survivors (if any) have strictly more digivolution cards.
    let remaining: Vec<usize> = r.game.players[1]
        .battle_area
        .iter()
        .map(|p| p.card_sources.len() - 1)
        .collect();
    assert!(remaining.len() < 2, "at least one fewest Digimon returned: {remaining:?}");
    assert!(r.deck_size(1) > 8 - 1, "returned to the bottom of the deck");
}

#[test]
fn ex13_024_trash_clamps_to_available_opponent_sources() {
    let mut r = start(&[CARD_ID], 8);
    let base = r.place_stack(0, &["SRC", "SRC", "BLUE-L5"]); // 3 sources after digivolve
    r.place_stack(1, &["OPP-S", "OPP-A"]); // only 1 source
    digivolve_onto(&mut r, base);
    pick_any_sources(&mut r, 1);
    assert_eq!(opp_source_total(&r), 0);
    assert_eq!(r.pending_kind(), Some(SelectionKind::EffectChoice), "continues to the 'then'");
}

#[test]
fn ex13_024_zero_source_opponent_digimon_count_as_fewest() {
    let mut r = start(&[CARD_ID], 8);
    let base = r.place_on_field(0, "WING", Some(0)); // 1 source after digivolve
    r.place_stack(1, &["OPP-S", "OPP-S", "OPP-A"]);
    r.place_on_field(1, "OPP-B", Some(0)); // 0 sources
    digivolve_onto(&mut r, base);
    pick_any_sources(&mut r, 1);
    r.execute_branch(0).unwrap();
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 1), vec!["OPP-A".to_string()], "OPP-B (0 sources) returned");
}

// ─── Section 2/3/5 — replacement ─────────────────────────────────────────────

#[test]
fn ex13_024_suspending_a_text_digimon_saves_a_leaving_text_digimon() {
    let mut r = start(&[], 5);
    r.place_on_field(0, CARD_ID, Some(0));
    let ally = r.place_on_field(0, "DRACO", Some(0));
    let payer = r.place_on_field(0, "DRACO2", Some(0));
    delete_with(&mut r, ally, ReplacementCause::OpponentEffect);
    assert_replacement_prompt(&r);
    accept(&mut r);
    assert_eq!(r.pending_kind(), Some(SelectionKind::OwnField));
    pick_field(&mut r, payer);
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&"DRACO".to_string()), "doesn't leave");
    assert!(is_suspended(&r, payer), "cost: suspended");
}

#[test]
fn ex13_024_own_effect_removal_is_also_covered() {
    let mut r = start(&[], 5);
    r.place_on_field(0, CARD_ID, Some(0));
    let ally = r.place_on_field(0, "DRACO", Some(0));
    delete_with(&mut r, ally, ReplacementCause::OwnEffect);
    assert_replacement_prompt(&r);
}

#[test]
fn ex13_024_non_text_digimon_is_not_protected() {
    let mut r = start(&[], 5);
    r.place_on_field(0, CARD_ID, Some(0));
    let plain = r.place_on_field(0, "PLAIN", Some(0));
    delete_with(&mut r, plain, ReplacementCause::OpponentEffect);
    assert!(r.pending_selection().is_none());
}

#[test]
fn ex13_024_no_unsuspended_text_digimon_means_no_offer() {
    let mut r = start(&[], 5);
    let me = r.place_on_field(0, CARD_ID, Some(0));
    set_suspended(&mut r, me, true);
    let ally = r.place_on_field(0, "DRACO", Some(0));
    set_suspended(&mut r, ally, true);
    delete_with(&mut r, ally, ReplacementCause::OpponentEffect);
    assert!(r.pending_selection().is_none(), "no payable suspend cost");
}

#[test]
fn ex13_024_face_up_replacement_is_once_per_turn() {
    let mut r = start(&[], 5);
    r.place_on_field(0, CARD_ID, Some(0));
    let a = r.place_on_field(0, "DRACO", Some(0));
    let payer = r.place_on_field(0, "DRACO2", Some(0));
    delete_with(&mut r, a, ReplacementCause::OpponentEffect);
    accept(&mut r);
    pick_field(&mut r, payer);
    let _ = r.auto_resolve();
    let a = handle_of(&r, 0, "DRACO");
    delete_with(&mut r, a, ReplacementCause::OpponentEffect);
    let _ = r.auto_resolve();
    assert!(!field_ids(&r, 0).contains(&"DRACO".to_string()), "OPT spent → leaves");
}

#[test]
fn ex13_024_inherited_replacement_works_from_a_source() {
    let mut r = start(&[], 5);
    r.place_stack(0, &[CARD_ID, "DRACO2"]);
    let ally = r.place_on_field(0, "DRACO", Some(0));
    delete_with(&mut r, ally, ReplacementCause::OpponentEffect);
    assert_replacement_prompt(&r);
    accept(&mut r);
    // Either Dracomon-text Digimon may pay; pick the leaving one itself.
    pick_field(&mut r, ally);
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&"DRACO".to_string()));
    assert!(is_suspended(&r, handle_of(&r, 0, "DRACO")));
}
