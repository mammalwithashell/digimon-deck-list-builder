//! EX13-012 SaviorHuckmon — Digimon, Lv.5, Red/White, 7000 DP, cost 7.
//! Traits: Dragonkin. Ultimate / Data.
//! Digivolve: Red Lv.4 cost 3; [Digivolve] Lv.4 w/[Huckmon] in text: Cost 3.
//!
//! # Card text (official Bandai DB bundle `data/card_bundles/EX13-012.md`)
//!
//! <Alliance> [When Digivolving] [When Attacking] [Once Per Turn] You may play
//! or use 1 white card with [Huckmon] in its text from your hand with the cost
//! reduced by 3.
//!
//! Inherited Effect: <Alliance>
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX13/Red/EX13_012.cs
//! - `AllianceSelfEffect` (face-up + inherited); special digivolve condition
//!   `TopCard.HasText("Huckmon")` Lv.4 cost 3.
//! - Shared WD/WA clause (`hashValue "EX13_012_WD_WA"`, max 1 per turn,
//!   skippable): optional hand pick of a white `HasText("Huckmon")` card
//!   playable/usable at cost − 3; `RemoveUse()` when nothing is picked.
//!
//! # Patterns
//! - H: printed <Alliance> (face-up + inherited).  - C: text-gated digivolve.
//! - A3: play-or-use from hand, cost − 3, shared OPT across two timings.

#![allow(dead_code, unused_imports)]

use super::orphan_support::*;
use digimon_dsl::compiled::{CompiledClause, CompiledColor, CompiledScope, CompiledTiming};
use digimon_engine::action::space::encode_digivolve;
use digimon_engine::enums::{CardColor, Keyword};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "EX13-012";

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-012 YAML loads")
        .add_card(with_cost(
            named("W-HUCK", "Huckmon Knight", CardColor::White, 5, 7000),
            5,
        ))
        .add_card({
            let mut o = option("W-HUCK-OPT", CardColor::White, 4);
            o.effect_text = "Search for 1 [Huckmon].".into();
            o
        })
        .add_card(with_cost(named("R-HUCK", "Huckmon Red", CardColor::Red, 5, 7000), 5))
        .add_card(with_cost(named("W-PLAIN", "Whitemon", CardColor::White, 5, 7000), 5))
        .add_card(named("RED-L4", "Redmon", CardColor::Red, 4, 5000))
        .add_card(named("WHITE-HUCK-L4", "BaoHuckmon", CardColor::White, 4, 5000))
        .add_card(named("WHITE-L4", "Whitefour", CardColor::White, 4, 5000))
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
fn ex13_012_printed_metadata_paths_and_alliance() {
    let mut r = start(&[], 5);
    let c = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!((c.level, c.dp, c.cost), (Some(5), Some(7000), Some(7)));
    assert_eq!(c.color, vec![CompiledColor::Red, CompiledColor::White]);
    assert_eq!(c.alt_paths.len(), 2);
    let t: Vec<_> = c
        .effects
        .iter()
        .filter_map(|e| match e {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(t.len(), 1, "one shared WD/WA clause");
    assert!(t[0].when.contains(&CompiledTiming::WhenDigivolving));
    assert!(t[0].when.contains(&CompiledTiming::WhenAttacking));
    assert!(t[0].once_per_turn);
    let h = r.place_on_field(0, CARD_ID, Some(0));
    assert!(r.game.has_keyword(h, Keyword::Alliance));
}

#[test]
fn ex13_012_inherited_alliance_on_carrier() {
    let mut r = start(&[], 5);
    let h = r.place_stack(0, &[CARD_ID, "TOP"]);
    assert!(r.game.has_keyword(h, Keyword::Alliance), "inherited <Alliance>");
}

// ─── Digivolve conditions ────────────────────────────────────────────────────

#[test]
fn ex13_012_digivolves_from_huckmon_text_lv4_of_any_color() {
    let mut r = start(&[CARD_ID], 5);
    let base = r.place_on_field(0, "WHITE-HUCK-L4", Some(0));
    digivolve_onto(&mut r, base);
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, base), CARD_ID);
}

#[test]
fn ex13_012_cannot_digivolve_from_plain_white_lv4() {
    let mut r = start(&[CARD_ID], 5);
    let base = r.place_on_field(0, "WHITE-L4", Some(0));
    digivolve_onto(&mut r, base);
    assert_eq!(top_id(&r, base), "WHITE-L4");
}

// ─── Section 2/3 — WD / WA play-or-use ───────────────────────────────────────

#[test]
fn ex13_012_when_digivolving_offers_only_white_huckmon_text_cards() {
    let mut r = start(&[CARD_ID, "W-HUCK", "W-HUCK-OPT", "R-HUCK", "W-PLAIN"], 10);
    let base = r.place_on_field(0, "RED-L4", Some(0));
    digivolve_onto(&mut r, base);
    assert!(r.pending_is_optional());
    assert_eq!(
        offered_hand_ids(&r),
        vec!["W-HUCK".to_string(), "W-HUCK-OPT".to_string()],
        "white AND [Huckmon] in text"
    );
}

#[test]
fn ex13_012_when_digivolving_plays_white_huckmon_for_3_less() {
    let mut r = start(&[CARD_ID, "W-HUCK"], 10);
    let base = r.place_on_field(0, "RED-L4", Some(0));
    digivolve_onto(&mut r, base);
    pick_hand(&mut r, "W-HUCK");
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&"W-HUCK".to_string()));
    assert_eq!(r.memory(), 10 - 3 - 2, "digivolve 3 + play cost 5 reduced by 3");
}

#[test]
fn ex13_012_when_attacking_uses_white_huckmon_option_for_3_less() {
    let mut r = start(&["W-HUCK-OPT"], 5);
    let h = r.place_on_field(0, CARD_ID, Some(0));
    r.attack_player(h, 1, false);
    assert!(r.pending_is_optional());
    pick_hand(&mut r, "W-HUCK-OPT");
    let _ = r.auto_resolve();
    assert!(trash_ids(&r, 0).contains(&"W-HUCK-OPT".to_string()));
    assert_eq!(r.memory(), 4, "use cost 4 reduced by 3");
}

#[test]
fn ex13_012_once_per_turn_shared_between_digivolving_and_attacking() {
    let mut r = start(&[CARD_ID, "W-HUCK", "W-HUCK-OPT"], 10);
    let base = r.place_on_field(0, "RED-L4", Some(0));
    digivolve_onto(&mut r, base);
    pick_hand(&mut r, "W-HUCK");
    let _ = r.auto_resolve();
    r.attack_player(base, 1, false);
    // Decline every remaining prompt (e.g. the <Alliance> offer): the shared
    // OPT clause must not get a chance to use the Option.
    while r.pending_selection().is_some() {
        let v = r.pending_selection_view().unwrap();
        assert_ne!(
            v.kind,
            digimon_engine::selection::SelectionKind::Hand,
            "the shared OPT clause must not offer a hand pick again: {v:?}"
        );
        if v.is_optional {
            decline(&mut r);
        } else {
            let _ = r.auto_resolve();
        }
    }
    assert!(
        hand_ids(&r, 0).contains(&"W-HUCK-OPT".to_string()),
        "OPT already used by [When Digivolving]: the Option stays in hand"
    );
    assert!(!trash_ids(&r, 0).contains(&"W-HUCK-OPT".to_string()));
}

#[test]
fn ex13_012_declining_refunds_the_once_per_turn() {
    let mut r = start(&[CARD_ID, "W-HUCK-OPT"], 10);
    let base = r.place_on_field(0, "RED-L4", Some(0));
    digivolve_onto(&mut r, base);
    decline(&mut r);
    let _ = r.auto_resolve();
    r.attack_player(base, 1, false);
    assert_eq!(
        offered_hand_ids(&r),
        vec!["W-HUCK-OPT".to_string()],
        "nothing picked → DCGO RemoveUse(): the attack trigger is still available"
    );
}
