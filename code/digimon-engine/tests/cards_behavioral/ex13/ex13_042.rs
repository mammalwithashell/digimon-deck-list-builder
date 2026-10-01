//! EX13-042 Bastemon — Digimon, Lv.5, Green, 7000 DP, cost 7.
//! Traits: Beastkin. Ultimate / Virus. Digivolve: Green Lv.4 cost 3.
//!
//! # Card text (official Bandai DB bundle `data/card_bundles/EX13-042.md`)
//!
//! <Alliance> [When Digivolving] [When Attacking] [Once Per Turn] You may play
//! 1 play cost 4 or lower Digimon card with [Beast], [Animal] or [Sovereign],
//! other than [Sea Animal], in any of its traits from your hand without paying
//! the cost.
//!
//! Inherited Effect: <Alliance>
//!
//! # DCGO C# reference
//! None at b9a0638cd (no `EX13_042.cs`); printed text governs. The trait test
//! follows BT11-089's shipped reading (substring `trait_contains` over every
//! trait, `[Sea Animal]` excluded — its Official Q&A: "Other than [Sea Animal],
//! all traits with … are applicable regardless of other words").
//!
//! # Patterns
//! - H: printed <Alliance> (face-up + inherited).
//! - A2: filtered free play from hand, shared OPT across WD/WA.

#![allow(dead_code, unused_imports)]

use super::orphan_support::*;
use digimon_dsl::compiled::{CompiledClause, CompiledColor, CompiledScope, CompiledTiming};
use digimon_engine::action::space::encode_digivolve;
use digimon_engine::enums::{CardColor, Keyword};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "EX13-042";

fn d(id: &str, cost: u16, traits: &[&str]) -> digimon_engine::card_data::CardData {
    with_cost(digimon(id, CardColor::Green, 4, 5000, traits), cost)
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-042 YAML loads")
        .add_card(d("BEASTKIN", 4, &["Beastkin"]))
        .add_card(d("ANIMAL", 3, &["Animal"]))
        .add_card(d("SOV", 4, &["Sovereign"]))
        .add_card(d("SEA", 3, &["Sea Animal"]))
        .add_card(d("BEAST-5", 5, &["Beast"]))
        .add_card(d("PLAIN", 3, &["Dragon"]))
        .add_card({
            let mut o = option("BEAST-OPT", CardColor::Green, 2);
            o.traits = vec!["Beast".into()];
            o
        })
        .add_card(named("GREEN-L4", "Greenmon", CardColor::Green, 4, 5000))
        .add_card(named("TOP", "Topmon", CardColor::Green, 6, 10000))
        .add_card(named("FILL", "Filler", CardColor::Green, 3, 3000))
        .add_card(named("SEC", "SecCard", CardColor::Green, 3, 1000))
}

fn start(hand: &[&str]) -> DebugRunner {
    builder()
        .hand(0, hand)
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
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
fn ex13_042_printed_metadata_alliance_and_clause_shape() {
    let mut r = start(&[]);
    let c = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!((c.level, c.dp, c.cost), (Some(5), Some(7000), Some(7)));
    assert_eq!(c.color, vec![CompiledColor::Green]);
    assert_eq!(c.traits, vec!["Beastkin".to_string()]);
    assert_eq!(c.alt_paths.len(), 1);
    let t: Vec<_> = c
        .effects
        .iter()
        .filter_map(|e| match e {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(t.len(), 1);
    assert!(t[0].when.contains(&CompiledTiming::WhenDigivolving));
    assert!(t[0].when.contains(&CompiledTiming::WhenAttacking));
    assert!(t[0].once_per_turn);
    let h = r.place_on_field(0, CARD_ID, Some(0));
    assert!(r.game.has_keyword(h, Keyword::Alliance));
    let carrier = r.place_stack(0, &[CARD_ID, "TOP"]);
    assert!(r.game.has_keyword(carrier, Keyword::Alliance), "inherited <Alliance>");
}

// ─── Section 2/3 — free play ─────────────────────────────────────────────────

#[test]
fn ex13_042_offers_only_cost_4_or_lower_beast_animal_sovereign_digimon() {
    let mut r = start(&[CARD_ID, "BEASTKIN", "ANIMAL", "SOV", "SEA", "BEAST-5", "PLAIN", "BEAST-OPT"]);
    let base = r.place_on_field(0, "GREEN-L4", Some(0));
    digivolve_onto(&mut r, base);
    assert!(r.pending_is_optional());
    assert_eq!(
        offered_hand_ids(&r),
        vec!["ANIMAL".to_string(), "BEASTKIN".to_string(), "SOV".to_string()],
        "Beastkin/Animal/Sovereign ≤4; not Sea Animal, cost 5, other traits, or non-Digimon"
    );
}

#[test]
fn ex13_042_when_digivolving_plays_the_pick_for_free() {
    let mut r = start(&[CARD_ID, "BEASTKIN"]);
    let base = r.place_on_field(0, "GREEN-L4", Some(0));
    digivolve_onto(&mut r, base);
    pick_hand(&mut r, "BEASTKIN");
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&"BEASTKIN".to_string()));
    assert_eq!(r.memory(), 5, "only the digivolve cost (3) was paid");
}

#[test]
fn ex13_042_when_attacking_plays_the_pick_for_free() {
    let mut r = start(&["ANIMAL"]);
    let me = r.place_on_field(0, CARD_ID, Some(0));
    r.attack_player(me, 1, false);
    pick_hand(&mut r, "ANIMAL");
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&"ANIMAL".to_string()));
    assert_eq!(r.memory(), 8);
}

#[test]
fn ex13_042_declining_refunds_the_once_per_turn() {
    let mut r = start(&[CARD_ID, "BEASTKIN"]);
    let base = r.place_on_field(0, "GREEN-L4", Some(0));
    digivolve_onto(&mut r, base);
    decline(&mut r);
    let _ = r.auto_resolve();
    r.attack_player(base, 1, false);
    assert_eq!(offered_hand_ids(&r), vec!["BEASTKIN".to_string()]);
}

#[test]
fn ex13_042_once_per_turn_shared_between_timings() {
    let mut r = start(&[CARD_ID, "BEASTKIN", "ANIMAL"]);
    let base = r.place_on_field(0, "GREEN-L4", Some(0));
    digivolve_onto(&mut r, base);
    pick_hand(&mut r, "BEASTKIN");
    let _ = r.auto_resolve();
    r.attack_player(base, 1, false);
    while r.pending_selection().is_some() {
        let v = r.pending_selection_view().unwrap();
        assert_ne!(v.kind, digimon_engine::selection::SelectionKind::Hand, "OPT spent: {v:?}");
        if v.is_optional {
            decline(&mut r);
        } else {
            let _ = r.auto_resolve();
        }
    }
    assert!(hand_ids(&r, 0).contains(&"ANIMAL".to_string()));
}
