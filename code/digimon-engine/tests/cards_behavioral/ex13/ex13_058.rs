//! EX13-058 Knightmon — Digimon, Lv.5, Black, 7000 DP, cost 7.
//! Traits: Warrior. Ultimate / Data.
//! Digivolve: Black Lv.4 cost 3; [Digivolve] Lv.4 w/[Knightmon] in text: Cost 3.
//!
//! # Card text (official Bandai DB bundle `data/card_bundles/EX13-058.md`)
//!
//! [When Attacking] [On Deletion] You may play or use 1 card with [Knightmon]
//! in its text and a play or use cost of 4 or less from your hand without
//! paying the cost. [Opponent's Turn] All of your Digimon with [Knightmon] in
//! their texts gain <Reboot> and <Blocker>.
//!
//! Inherited Effect: [All Turns] [Once Per Turn] When any of your Digimon with
//! [Knightmon] in their texts are played, <De-Digivolve 1> 1 of your
//! opponent's Digimon.
//!
//! # DCGO C# reference
//! None at b9a0638cd (no `EX13_058.cs`); printed text governs. Idioms:
//! ST24-06 (`play_or_use_cost_lte` + `play_or_use_from_hand { cost_delta: free }`),
//! EX13-062 (`[Knightmon]`-text keyword aura), EX13-053 (De-Digivolve 1).
//!
//! # Patterns
//! - C: text-gated special digivolve.  - A3: free play-or-use from hand.
//! - D4: turn-gated keyword aura over own Digimon.  - G4: inherited OPT
//!   ally-played trigger → De-Digivolve.

#![allow(dead_code, unused_imports)]

use super::orphan_support::*;
use digimon_dsl::compiled::{CompiledClause, CompiledColor, CompiledScope, CompiledTiming};
use digimon_engine::enums::{CardColor, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "EX13-058";

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-058 YAML loads")
        .add_card(with_cost(named("K4", "Pawnmon Knightmon", CardColor::Black, 4, 5000), 4))
        .add_card(with_cost(named("K5", "Knightmon Lord", CardColor::Black, 5, 7000), 5))
        .add_card({
            let mut o = option("K-OPT", CardColor::Black, 3);
            o.effect_text = "Your [Knightmon] gets +2000 DP.".into();
            o
        })
        .add_card(with_cost(named("PLAIN", "Plainmon", CardColor::Black, 4, 5000), 3))
        .add_card(named("ALLY-K", "Knightmon Ally", CardColor::Black, 4, 5000))
        .add_card(named("ALLY", "Allymon", CardColor::Black, 4, 5000))
        .add_card(named("TOP", "Topmon", CardColor::Black, 6, 10000))
        .add_card(named("OPP-LV3", "OppThree", CardColor::Red, 3, 3000))
        .add_card(named("OPP-LV4", "OppFour", CardColor::Red, 4, 5000))
        .add_card(named("FILL", "Filler", CardColor::Black, 3, 3000))
        .add_card(named("SEC", "SecCard", CardColor::Black, 3, 1000))
}

fn start(hand: &[&str]) -> DebugRunner {
    builder()
        .hand(0, hand)
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .security(1, &["SEC", "SEC", "SEC"])
        .memory(6)
        .start()
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn ex13_058_printed_metadata_and_clause_shape() {
    let r = start(&[]);
    let c = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!((c.level, c.dp, c.cost), (Some(5), Some(7000), Some(7)));
    assert_eq!(c.color, vec![CompiledColor::Black]);
    assert_eq!(c.alt_paths.len(), 2);
    let t: Vec<_> = c
        .effects
        .iter()
        .filter_map(|e| match e {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(t.len(), 2);
    assert!(t[0].when.contains(&CompiledTiming::WhenAttacking));
    assert!(t[0].when.contains(&CompiledTiming::OnDeletion));
    assert!(!t[0].once_per_turn);
    assert_eq!(t[1].scope, CompiledScope::Inherited);
    assert!(t[1].once_per_turn);
}

// ─── Section 2/3 — WA / On Deletion free play-or-use ─────────────────────────

#[test]
fn ex13_058_when_attacking_offers_knightmon_text_cards_cost_4_or_less() {
    let mut r = start(&["K4", "K5", "K-OPT", "PLAIN"]);
    let me = r.place_on_field(0, CARD_ID, Some(0));
    r.attack_player(me, 1, false);
    assert!(r.pending_is_optional());
    assert_eq!(offered_hand_ids(&r), vec!["K-OPT".to_string(), "K4".to_string()]);
}

#[test]
fn ex13_058_when_attacking_plays_digimon_free() {
    let mut r = start(&["K4"]);
    let me = r.place_on_field(0, CARD_ID, Some(0));
    r.attack_player(me, 1, false);
    pick_hand(&mut r, "K4");
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&"K4".to_string()));
    assert_eq!(r.memory(), 6);
}

#[test]
fn ex13_058_on_deletion_uses_option_free() {
    let mut r = start(&["K-OPT"]);
    let me = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(0, "ALLY", Some(0)); // a black Digimon for the Option's colour requirement
    delete_by_opponent_effect(&mut r, me);
    assert!(r.pending_is_optional());
    pick_hand(&mut r, "K-OPT");
    let _ = r.auto_resolve();
    assert!(trash_ids(&r, 0).contains(&"K-OPT".to_string()), "Option used");
    assert_eq!(r.memory(), 6, "free");
}

#[test]
fn ex13_058_declining_plays_nothing() {
    let mut r = start(&["K4"]);
    let me = r.place_on_field(0, CARD_ID, Some(0));
    delete_by_opponent_effect(&mut r, me);
    decline(&mut r);
    let _ = r.auto_resolve();
    assert_eq!(hand_ids(&r, 0), vec!["K4".to_string()]);
}

// ─── Section 2 — [Opponent's Turn] aura ──────────────────────────────────────

#[test]
fn ex13_058_aura_inactive_on_your_turn() {
    let mut r = start(&[]);
    let me = r.place_on_field(0, CARD_ID, Some(0));
    let k = r.place_on_field(0, "ALLY-K", Some(0));
    r.game.tick_declarative_effects();
    assert!(!r.game.has_keyword(k, Keyword::Blocker));
    assert!(!r.game.has_keyword(me, Keyword::Reboot));
}

#[test]
fn ex13_058_aura_grants_reboot_and_blocker_to_knightmon_text_digimon_on_opponents_turn() {
    let mut r = start(&[]);
    let me = r.place_on_field(0, CARD_ID, Some(0));
    let k = r.place_on_field(0, "ALLY-K", Some(0));
    let plain = r.place_on_field(0, "ALLY", Some(0));
    end_turn_declining(&mut r);
    r.game.tick_declarative_effects();
    for h in [k, me] {
        assert!(r.game.has_keyword(h, Keyword::Reboot), "{h:?} <Reboot>");
        assert!(r.game.has_keyword(h, Keyword::Blocker), "{h:?} <Blocker>");
    }
    assert!(!r.game.has_keyword(plain, Keyword::Blocker), "no [Knightmon] in text");
}

// ─── Section 2/3/5 — inherited De-Digivolve ──────────────────────────────────

#[test]
fn ex13_058_inherited_knightmon_text_digimon_played_de_digivolves() {
    let mut r = start(&["K4"]);
    r.place_stack(0, &[CARD_ID, "TOP"]);
    let opp = r.place_stack(1, &["OPP-LV3", "OPP-LV4"]);
    play_card(&mut r, 0, "K4");
    assert_eq!(r.pending_kind(), Some(SelectionKind::OppField));
    pick_field(&mut r, opp);
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, opp), "OPP-LV3");
}

#[test]
fn ex13_058_inherited_plain_digimon_played_does_not_trigger() {
    let mut r = start(&["PLAIN"]);
    r.place_stack(0, &[CARD_ID, "TOP"]);
    let opp = r.place_stack(1, &["OPP-LV3", "OPP-LV4"]);
    play_card(&mut r, 0, "PLAIN");
    assert!(r.pending_selection().is_none());
    assert_eq!(top_id(&r, opp), "OPP-LV4");
}

#[test]
fn ex13_058_inherited_once_per_turn() {
    let mut r = start(&["K4", "K-OPT"]);
    r.game.memory = 10;
    r.place_stack(0, &[CARD_ID, "TOP"]);
    let opp = r.place_stack(1, &["OPP-LV3", "OPP-LV4"]);
    play_card(&mut r, 0, "K4");
    pick_field(&mut r, opp);
    let _ = r.auto_resolve();
    r.add_to_hand(0, "K4");
    let opp2 = r.place_stack(1, &["OPP-LV3", "OPP-LV4"]);
    play_card(&mut r, 0, "K4");
    assert!(r.pending_selection().is_none(), "OPT spent");
    assert_eq!(top_id(&r, opp2), "OPP-LV4");
}
