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
//! `DCGO/Assets/Scripts/CardEffect/EX13/Black/EX13_058.cs`. The hand pick
//! (`CanSelectCardCondition`, EX13_058.cs:32-35) offers `HasText("Knightmon")
//! && GetCostItself <= 4 && CardEffectCommons.CanPlayOrUse(card, activateClass)`;
//! `CanPlayOrUse` (PlayEffects.cs:286-292) = `(IsOption && !CanNotPlayThisOption)
//! || (HasPlayCost && CanPlayAsNewPermanent(card, payCost: false, ...))`, and
//! `CanNotPlayThisOption` includes an unmet colour requirement
//! (CardSource.cs:184-246). Idioms: ST24-06 (`play_or_use_cost_lte` +
//! `can_play_or_use_from_effect` + `play_or_use_from_hand { cost_delta: free }`),
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
        .add_card({
            // A red [Knightmon]-text Option: no red Digimon/Tamer of player 0
            // meets its colour requirement (CR 4.3 4-22-2).
            let mut o = option("K-OPT-RED", CardColor::Red, 2);
            o.effect_text = "Your [Knightmon] gets +1000 DP.".into();
            o
        })
        .add_card({
            // A [Knightmon]-text Digi-Egg: no play cost (CR 4.3 7-1-1).
            let mut e = named("K-EGG", "Knightmon Egg", CardColor::Black, 2, 0);
            e.card_kind = digimon_engine::enums::CardKind::DigiEgg;
            e.dp = None;
            e.play_cost = 0;
            e
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

// ─── Section 2/3 — candidate legality (G-DSL-PLAY-OR-USE-CANDIDATE-LEGALITY) ──
// The pick offers only cards the follow-up play/use can carry out (DCGO
// EX13_058.cs:32-35 `CanPlayOrUse`; CR 4.3 1-3-2: an impossible action is not
// carried out, so offering it would be an illegal choice in the action space).

fn when_attacking_offered(hand: &[&str], setup: impl FnOnce(&mut DebugRunner)) -> Vec<String> {
    let mut r = start(hand);
    let me = r.place_on_field(0, CARD_ID, Some(0));
    setup(&mut r);
    r.attack_player(me, 1, false);
    if r.pending_kind() != Some(SelectionKind::Hand) {
        return Vec::new();
    }
    offered_hand_ids(&r)
}

/// A [Knightmon]-text Digi-Egg has no play cost (CR 4.3 7-1-1; DCGO
/// `HasPlayCost` false) and is no Option: it is never offered.
#[test]
fn ex13_058_digi_egg_is_not_offered() {
    assert_eq!(
        when_attacking_offered(&["K-EGG", "K4"], |_| {}),
        vec!["K4".to_string()]
    );
}

/// A red [Knightmon]-text Option can't be used without a red Digimon/Tamer
/// (CR 4.3 4-22-1/4-22-2; an effect use still needs the colour requirement,
/// official rulings Q1500 / Q2785; DCGO `CanNotPlayThisOption`).
#[test]
fn ex13_058_option_whose_colour_requirement_is_unmet_is_not_offered() {
    assert_eq!(
        when_attacking_offered(&["K-OPT-RED", "K-OPT"], |_| {}),
        vec!["K-OPT".to_string()],
        "black K-OPT is usable via Knightmon (EX13-058); red K-OPT-RED is not"
    );
}

/// "You can't use Option cards": the Option is not offered.
#[test]
fn ex13_058_option_not_offered_when_options_cannot_be_used() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::PlayerModifierEntry;
    let offered = when_attacking_offered(&["K-OPT", "K4"], |r| {
        r.game.modifiers.add_player_modifier(
            0,
            PlayerModifierEntry::simple(ModifierType::CannotUseOptionCards, 0, Expiry::Permanent, None, 1),
        );
    });
    assert_eq!(offered, vec!["K4".to_string()]);
}

/// "You can't play Digimon by effects": the Digimon is not offered.
#[test]
fn ex13_058_digimon_not_offered_when_effect_play_is_prohibited() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::PlayerModifierEntry;
    let offered = when_attacking_offered(&["K-OPT", "K4"], |r| {
        r.game.modifiers.add_player_modifier(
            0,
            PlayerModifierEntry::simple(ModifierType::CannotPlayDigimonByEffect, 0, Expiry::Permanent, None, 1),
        );
    });
    assert_eq!(offered, vec!["K-OPT".to_string()]);
}

/// A full battle area (DCGO `CanPlayAsNewPermanent` needs an empty frame): the
/// Digimon is not offered, the Option still is.
#[test]
fn ex13_058_digimon_not_offered_when_battle_area_full() {
    let offered = when_attacking_offered(&["K-OPT", "K4"], |r| {
        r.game.rules.field_slots = r.game.players[0].battle_area.len() as u8;
    });
    assert_eq!(offered, vec!["K-OPT".to_string()]);
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
