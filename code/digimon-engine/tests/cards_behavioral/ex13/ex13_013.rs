//! EX13-013 WarGrowlmon — Digimon, Lv.5, Red, 8000 DP, cost 8.
//! Traits: Cyborg. Ultimate / Virus. Digivolve: Red Lv.4 cost 3.
//!
//! # Card text (official Bandai DB bundle `data/card_bundles/EX13-013.md`)
//!
//! <Engage> (At the end of your turn, this Digimon may attack.) [When
//! Digivolving] [When Attacking] Delete 1 of your opponent's Digimon with 5000
//! DP or less. If this effect didn't delete, this Digimon gains <Piercing> and
//! +3000 DP for the turn. [End of Attack] [On Deletion] You may play 1 Tamer
//! card with [Guilmon] in its text from your hand or trash without paying the
//! cost.
//!
//! Inherited Effect: [All Turns] [Once Per Turn] When any of your opponent's
//! Digimon are deleted, if this Digimon has [Gallantmon] in its name, trash
//! their top security card.
//!
//! Official Q&A: the delete is mandatory — with a 5000-DP-or-less opponent
//! Digimon you must select and delete it.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX13/Red/EX13_013.cs
//! - `EngageSelfStaticEffect`.
//! - Shared WD/WA (mandatory): select 1 opponent Digimon `DP <= MaxDP_DeleteEffect(5000)`,
//!   delete; if nothing was deleted → `GainPierce` + `ChangeDigimonDP(+3000)` until turn end.
//! - Shared EoA/OD: Tamer `HasText("Guilmon")` from hand or trash (choice of
//!   zone or "don't play"), `PlayByEffect(payCost: false)`.
//! - Inherited `OnDestroyedAnyone` OPT: an opponent Digimon deleted, carrier
//!   top card `ContainsCardName("Gallantmon")` → `IDestroySecurity(enemy, 1, top)`.
//!
//! # Patterns
//! - H: printed <Engage>.  - F1: mandatory DP-capped delete with a
//!   "didn't delete" fallback.  - A4: union-zone (hand/trash) free Tamer play.
//! - G4: inherited OPT name-gated security trash.

#![allow(dead_code, unused_imports)]

use super::orphan_support::*;
use digimon_dsl::compiled::{CompiledClause, CompiledColor, CompiledScope, CompiledTiming};
use digimon_engine::action::space::{encode_digivolve, PASS};
use digimon_engine::enums::{CardColor, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "EX13-013";

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-013 YAML loads")
        .add_card(named("RED-L4", "Growlmon", CardColor::Red, 4, 6000))
        .add_card(named("OPP-5K", "OppFive", CardColor::Blue, 4, 5000))
        .add_card(named("OPP-6K", "OppSix", CardColor::Blue, 4, 6000))
        .add_card({
            let mut t = tamer("GUIL-T", CardColor::Red, 4);
            t.card_name = "Takato".into();
            t.effect_text = "Play 1 [Guilmon] from your hand.".into();
            t
        })
        .add_card({
            let mut t = tamer("GUIL-T2", CardColor::Red, 4);
            t.card_name = "Guilmon Tamer".into();
            t
        })
        .add_card(tamer("PLAIN-T", CardColor::Red, 4))
        .add_card(named("GALLANT", "Gallantmon", CardColor::Red, 6, 12000))
        .add_card(named("TOP", "Topmon", CardColor::Red, 6, 11000))
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

fn digivolve_onto(r: &mut DebugRunner, base: PermanentHandle) {
    let slot = hand_index(r, 0, CARD_ID) as u16;
    r.game.decode_action(encode_digivolve(slot, base.index as u16), 0);
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn ex13_013_printed_metadata_engage_and_clause_shape() {
    let mut r = start(&[], 5);
    let c = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!((c.level, c.dp, c.cost), (Some(5), Some(8000), Some(8)));
    assert_eq!(c.color, vec![CompiledColor::Red]);
    assert_eq!(c.traits, vec!["Cyborg".to_string()]);
    let t: Vec<_> = c
        .effects
        .iter()
        .filter_map(|e| match e {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(t.len(), 3);
    assert!(t[0].when.contains(&CompiledTiming::WhenDigivolving));
    assert!(t[0].when.contains(&CompiledTiming::WhenAttacking));
    assert!(!t[0].optional, "the delete is mandatory (Official Q&A)");
    assert!(t[1].when.contains(&CompiledTiming::EndOfAttack));
    assert!(t[1].when.contains(&CompiledTiming::OnDeletion));
    assert_eq!(t[2].scope, CompiledScope::Inherited);
    assert!(t[2].once_per_turn);
    let h = r.place_on_field(0, CARD_ID, Some(0));
    assert!(r.game.has_keyword(h, Keyword::Engage), "<Engage>");
}

// ─── Section 2/3 — WD/WA delete-or-buff ──────────────────────────────────────

#[test]
fn ex13_013_when_digivolving_must_delete_a_5000_or_less_digimon() {
    let mut r = start(&[CARD_ID], 5);
    let base = r.place_on_field(0, "RED-L4", Some(0));
    let small = r.place_on_field(1, "OPP-5K", Some(0));
    let big = r.place_on_field(1, "OPP-6K", Some(0));
    digivolve_onto(&mut r, base);
    assert_eq!(r.pending_kind(), Some(SelectionKind::OppField));
    assert!(!r.pending_is_optional(), "mandatory delete");
    assert!(field_offered(&r, small) && !field_offered(&r, big), "5000 DP or less only");
    pick_field(&mut r, small);
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 1), vec!["OPP-6K".to_string()]);
    assert_eq!(r.effective_dp(base), Some(8000), "deleted → no +3000");
    assert!(!r.game.has_keyword(base, Keyword::Piercing));
}

#[test]
fn ex13_013_when_digivolving_without_target_gains_piercing_and_3000() {
    let mut r = start(&[CARD_ID], 5);
    let base = r.place_on_field(0, "RED-L4", Some(0));
    r.place_on_field(1, "OPP-6K", Some(0));
    digivolve_onto(&mut r, base);
    let _ = r.auto_resolve();
    assert_eq!(r.effective_dp(base), Some(11000));
    assert!(r.game.has_keyword(base, Keyword::Piercing));
}

#[test]
fn ex13_013_buff_lasts_for_the_turn_only() {
    let mut r = start(&[CARD_ID], 5);
    let base = r.place_on_field(0, "RED-L4", Some(0));
    digivolve_onto(&mut r, base);
    let _ = r.auto_resolve();
    assert_eq!(r.effective_dp(base), Some(11000));
    end_turn_declining(&mut r);
    assert_eq!(r.effective_dp(base), Some(8000));
    assert!(!r.game.has_keyword(base, Keyword::Piercing));
}

#[test]
fn ex13_013_when_attacking_also_deletes() {
    let mut r = start(&[], 5);
    let me = r.place_on_field(0, CARD_ID, Some(0));
    let small = r.place_on_field(1, "OPP-5K", Some(0));
    r.attack_player(me, 1, false);
    assert_eq!(r.pending_kind(), Some(SelectionKind::OppField));
    pick_field(&mut r, small);
    // Remaining prompts (End of Attack offer etc.) are declined/settled.
    while r.pending_selection().is_some() {
        if r.pending_is_optional() {
            decline(&mut r);
        } else {
            let _ = r.auto_resolve();
        }
    }
    assert!(field_ids(&r, 1).is_empty(), "OPP-5K deleted on attack");
}

// ─── Section 2/3 — [End of Attack] [On Deletion] Tamer play ──────────────────

#[test]
fn ex13_013_on_deletion_plays_guilmon_text_tamer_from_trash_free() {
    let mut r = start(&[], 5);
    let me = r.place_on_field(0, CARD_ID, Some(0));
    r.game.players[0].trash.clear();
    r.inject_trash(0, "GUIL-T");
    r.inject_trash(0, "PLAIN-T");
    let mem0 = r.memory();
    delete_by_opponent_effect(&mut r, me);
    assert!(r.pending_is_optional(), "'You may play'");
    pick_trash(&mut r, "GUIL-T");
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&"GUIL-T".to_string()), "Tamer played");
    assert_eq!(r.memory(), mem0, "without paying the cost");
}

#[test]
fn ex13_013_on_deletion_plays_guilmon_named_tamer_from_hand() {
    let mut r = start(&["GUIL-T2", "PLAIN-T"], 5);
    let me = r.place_on_field(0, CARD_ID, Some(0));
    delete_by_opponent_effect(&mut r, me);
    assert_eq!(offered_hand_ids(&r), vec!["GUIL-T2".to_string()], "plain Tamer excluded");
    pick_hand(&mut r, "GUIL-T2");
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&"GUIL-T2".to_string()));
}

#[test]
fn ex13_013_on_deletion_decline_plays_nothing() {
    let mut r = start(&["GUIL-T2"], 5);
    let me = r.place_on_field(0, CARD_ID, Some(0));
    delete_by_opponent_effect(&mut r, me);
    decline(&mut r);
    let _ = r.auto_resolve();
    assert_eq!(hand_ids(&r, 0), vec!["GUIL-T2".to_string()]);
}

#[test]
fn ex13_013_end_of_attack_offers_the_tamer_play() {
    let mut r = start(&["GUIL-T2"], 5);
    let me = r.place_on_field(0, CARD_ID, Some(0));
    r.attack_player(me, 1, false);
    // No ≤5000 target → WA buff resolves without a prompt; security check runs;
    // then [End of Attack] offers the Tamer.
    let mut offered = false;
    while r.pending_selection().is_some() {
        if matches!(
            r.pending_kind(),
            Some(SelectionKind::Hand) | Some(SelectionKind::UnionZone { .. })
        ) {
            offered = true;
            pick_hand(&mut r, "GUIL-T2");
        } else {
            let _ = r.auto_resolve();
        }
    }
    assert!(offered, "[End of Attack] hand pick");
    assert!(field_ids(&r, 0).contains(&"GUIL-T2".to_string()));
}

// ─── Section 2/3/5 — inherited security trash ────────────────────────────────

#[test]
fn ex13_013_inherited_gallantmon_carrier_trashes_opponent_top_security() {
    let mut r = start(&[], 5);
    r.place_stack(0, &[CARD_ID, "GALLANT"]);
    let opp = r.place_on_field(1, "OPP-5K", Some(0));
    delete_by_opponent_effect(&mut r, opp);
    let _ = r.auto_resolve();
    assert_eq!(r.security_count(1), 2);
}

#[test]
fn ex13_013_inherited_non_gallantmon_carrier_does_nothing() {
    let mut r = start(&[], 5);
    r.place_stack(0, &[CARD_ID, "TOP"]);
    let opp = r.place_on_field(1, "OPP-5K", Some(0));
    delete_by_opponent_effect(&mut r, opp);
    let _ = r.auto_resolve();
    assert_eq!(r.security_count(1), 3);
}

#[test]
fn ex13_013_inherited_own_digimon_deletion_does_not_trigger() {
    let mut r = start(&[], 5);
    r.place_stack(0, &[CARD_ID, "GALLANT"]);
    let mine = r.place_on_field(0, "OPP-5K", Some(0));
    delete_by_opponent_effect(&mut r, mine);
    let _ = r.auto_resolve();
    assert_eq!(r.security_count(1), 3);
}

#[test]
fn ex13_013_inherited_once_per_turn() {
    let mut r = start(&[], 5);
    r.place_stack(0, &[CARD_ID, "GALLANT"]);
    let a = r.place_on_field(1, "OPP-5K", Some(0));
    r.place_on_field(1, "OPP-6K", Some(0));
    delete_by_opponent_effect(&mut r, a);
    let _ = r.auto_resolve();
    let b = handle_of(&r, 1, "OPP-6K");
    delete_by_opponent_effect(&mut r, b);
    let _ = r.auto_resolve();
    assert_eq!(r.security_count(1), 2, "only once per turn");
}

#[test]
fn ex13_013_engage_opens_an_end_of_turn_attack_window() {
    let mut r = start(&[], 5);
    r.place_on_field(0, CARD_ID, Some(0));
    r.end_turn();
    assert_eq!(
        r.game.current_phase,
        digimon_engine::enums::GamePhase::EndOfTurnAction,
        "<Engage>: this Digimon may attack at the end of your turn"
    );
}
