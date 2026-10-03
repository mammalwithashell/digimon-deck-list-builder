//! EX13-052 Gladimon — Digimon, Lv.4, Black, 4000 DP, cost 4.
//! Traits: Warrior. Champion / Vaccine. Digivolve: Black Lv.3 cost 2.
//!
//! # Card text (official Bandai DB bundle `data/card_bundles/EX13-052.md`)
//!
//! <Guard> (When any of your other Digimon would leave the battle area by your
//! opponent's effects, by deleting this Digimon, they don't leave.) [On Play]
//! [On Deletion] <De-Digivolve 1> 1 of your opponent's Digimon.
//!
//! Inherited Effect: [All Turns] [Once Per Turn] When this Digimon would leave
//! the battle area other than by your effects, by deleting 1 of your other
//! Digimon with [Knightmon] in its text, it doesn't leave.
//!
//! # DCGO C# reference
//! None at b9a0638cd (no `EX13_052.cs`); printed text + general_rule.pdf govern.
//! <Guard> is the engine's printed keyword (`cards/keyword_effects.rs`); the
//! inherited replacement mirrors EX13-027 (shipped) with a [Knightmon]-text
//! cost filter.
//!
//! # Patterns
//! - H: printed <Guard>.  - F2: [On Play]/[On Deletion] <De-Digivolve 1>.
//! - F4 + G4: inherited OPT would-leave replacement with a delete cost.

#![allow(dead_code, unused_imports)]

use super::orphan_support::*;
use digimon_dsl::compiled::{
    CompiledClause, CompiledColor, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::enums::{CardColor, Keyword};
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "EX13-052";

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-052 YAML loads")
        .add_card(named("KNIGHT", "Knightmon", CardColor::Black, 5, 7000))
        .add_card(with_text(
            named("KNIGHT-TEXT", "Pawnmon", CardColor::Black, 3, 3000),
            "Digivolve into a card with [Knightmon] in its text.",
        ))
        .add_card(named("PLAIN", "Plainmon", CardColor::Black, 4, 5000))
        .add_card(named("TOP", "Topmon", CardColor::Black, 5, 7000))
        .add_card(named("OPP-LV3", "OppThree", CardColor::Red, 3, 3000))
        .add_card(named("OPP-LV4", "OppFour", CardColor::Red, 4, 5000))
        .add_card(named("FILL", "Filler", CardColor::Black, 3, 3000))
}

fn start(hand: &[&str]) -> DebugRunner {
    builder()
        .hand(0, hand)
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start()
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn ex13_052_printed_metadata_guard_and_clause_shape() {
    let mut r = start(&[]);
    let c = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!((c.level, c.dp, c.cost), (Some(4), Some(4000), Some(4)));
    assert_eq!(c.color, vec![CompiledColor::Black]);
    assert_eq!(c.traits, vec!["Warrior".to_string()]);
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
    assert!(t[0].when.contains(&CompiledTiming::OnDeletion));
    let reps: Vec<_> = c
        .effects
        .iter()
        .filter_map(|e| match e {
            CompiledClause::Declarative(CompiledDeclarativeClause::Replacement {
                scope,
                optional,
                once_per_turn,
                ..
            }) => Some((*scope, *optional, *once_per_turn)),
            _ => None,
        })
        .collect();
    assert_eq!(reps, vec![(CompiledScope::Inherited, true, true)]);
    let h = r.place_on_field(0, CARD_ID, Some(0));
    assert!(r.game.has_keyword(h, Keyword::Guard), "<Guard>");
}

// ─── Section 3 — [On Play] / [On Deletion] ───────────────────────────────────

#[test]
fn ex13_052_on_play_de_digivolves_1() {
    let mut r = start(&[CARD_ID]);
    let opp = r.place_stack(1, &["OPP-LV3", "OPP-LV4"]);
    play_card(&mut r, 0, CARD_ID);
    assert_eq!(r.pending_kind(), Some(SelectionKind::OppField));
    pick_field(&mut r, opp);
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, opp), "OPP-LV3");
}

#[test]
fn ex13_052_on_deletion_de_digivolves_1() {
    let mut r = start(&[]);
    let me = r.place_on_field(0, CARD_ID, Some(0));
    let opp = r.place_stack(1, &["OPP-LV3", "OPP-LV4"]);
    delete_by_opponent_effect(&mut r, me);
    assert_eq!(r.pending_kind(), Some(SelectionKind::OppField));
    pick_field(&mut r, opp);
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, opp), "OPP-LV3");
}

#[test]
fn ex13_052_guard_deletes_gladimon_to_save_an_ally_then_on_deletion_fires() {
    let mut r = start(&[]);
    r.place_on_field(0, CARD_ID, Some(0));
    let ally = r.place_on_field(0, "PLAIN", Some(0));
    let opp = r.place_stack(1, &["OPP-LV3", "OPP-LV4"]);
    delete_with(&mut r, ally, ReplacementCause::OpponentEffect);
    assert_replacement_prompt(&r);
    accept(&mut r);
    // Gladimon is deleted by <Guard> → its [On Deletion] De-Digivolve.
    if r.pending_kind() == Some(SelectionKind::OppField) {
        pick_field(&mut r, opp);
    }
    let _ = r.auto_resolve();
    let field = field_ids(&r, 0);
    assert!(field.contains(&"PLAIN".to_string()), "ally doesn't leave");
    assert!(!field.contains(&CARD_ID.to_string()), "Gladimon deleted as the <Guard> cost");
    assert_eq!(top_id(&r, opp), "OPP-LV3", "[On Deletion] <De-Digivolve 1>");
}

// ─── Section 2/3/5 — inherited replacement ───────────────────────────────────

fn inherited_setup() -> (DebugRunner, digimon_engine::permanent::PermanentHandle) {
    let mut r = start(&[]);
    let carrier = r.place_stack(0, &[CARD_ID, "TOP"]);
    (r, carrier)
}

#[test]
fn ex13_052_inherited_deletes_other_knightmon_text_digimon_and_carrier_stays() {
    let (mut r, carrier) = inherited_setup();
    let knight = r.place_on_field(0, "KNIGHT-TEXT", Some(0));
    r.place_on_field(0, "PLAIN", Some(0));
    delete_with(&mut r, carrier, ReplacementCause::OpponentEffect);
    assert_replacement_prompt(&r);
    accept(&mut r);
    assert_eq!(r.pending_kind(), Some(SelectionKind::OwnField));
    assert_eq!(legal(&r).len(), 1, "only the [Knightmon]-text Digimon");
    pick_field(&mut r, knight);
    let _ = r.auto_resolve();
    let field = field_ids(&r, 0);
    assert!(field.contains(&"TOP".to_string()), "carrier stays");
    assert!(!field.contains(&"KNIGHT-TEXT".to_string()), "cost Digimon deleted");
    assert!(field.contains(&"PLAIN".to_string()));
}

#[test]
fn ex13_052_inherited_battle_removal_is_covered() {
    let (mut r, carrier) = inherited_setup();
    r.place_on_field(0, "KNIGHT", Some(0));
    delete_with(&mut r, carrier, ReplacementCause::Battle);
    assert_replacement_prompt(&r);
}

#[test]
fn ex13_052_inherited_not_offered_without_other_knightmon() {
    let (mut r, carrier) = inherited_setup();
    r.place_on_field(0, "PLAIN", Some(0));
    delete_with(&mut r, carrier, ReplacementCause::OpponentEffect);
    let _ = r.auto_resolve();
    assert!(!field_ids(&r, 0).contains(&"TOP".to_string()), "carrier leaves");
    assert!(field_ids(&r, 0).contains(&"PLAIN".to_string()));
}

#[test]
fn ex13_052_inherited_not_offered_for_own_effect_removal() {
    let (mut r, carrier) = inherited_setup();
    r.place_on_field(0, "KNIGHT", Some(0));
    delete_with(&mut r, carrier, ReplacementCause::OwnEffect);
    assert!(r.pending_selection().is_none(), "'other than by your effects'");
    assert!(!field_ids(&r, 0).contains(&"TOP".to_string()));
}

#[test]
fn ex13_052_inherited_decline_lets_carrier_leave() {
    let (mut r, carrier) = inherited_setup();
    r.place_on_field(0, "KNIGHT", Some(0));
    delete_with(&mut r, carrier, ReplacementCause::OpponentEffect);
    assert_replacement_prompt(&r);
    decline(&mut r);
    let _ = r.auto_resolve();
    assert!(!field_ids(&r, 0).contains(&"TOP".to_string()));
    assert!(field_ids(&r, 0).contains(&"KNIGHT".to_string()));
}

#[test]
fn ex13_052_inherited_protects_only_the_carrier() {
    let (mut r, _carrier) = inherited_setup();
    r.place_on_field(0, "KNIGHT", Some(0));
    let other = r.place_on_field(0, "PLAIN", Some(0));
    delete_with(&mut r, other, ReplacementCause::OpponentEffect);
    assert!(r.pending_selection().is_none(), "'this Digimon' only");
}

#[test]
fn ex13_052_inherited_once_per_turn() {
    let (mut r, carrier) = inherited_setup();
    let k1 = r.place_on_field(0, "KNIGHT", Some(0));
    r.place_on_field(0, "KNIGHT-TEXT", Some(0));
    delete_with(&mut r, carrier, ReplacementCause::OpponentEffect);
    accept(&mut r);
    pick_field(&mut r, k1);
    let _ = r.auto_resolve();
    let carrier = handle_of(&r, 0, "TOP");
    delete_with(&mut r, carrier, ReplacementCause::OpponentEffect);
    let _ = r.auto_resolve();
    assert!(!field_ids(&r, 0).contains(&"TOP".to_string()), "OPT spent → carrier leaves");
}
