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
        // Candidate-legality fixtures (all white with [Huckmon] in their text).
        .add_card({
            let mut e = named("W-HUCK-EGG", "Huckmon Egg", CardColor::White, 2, 0);
            e.card_kind = digimon_engine::enums::CardKind::DigiEgg;
            e.dp = None;
            e.play_cost = 0;
            e
        })
        .add_card({
            let mut o = option("WB-HUCK-OPT", CardColor::White, 4);
            o.colors = vec![CardColor::White, CardColor::Black];
            o.effect_text = "Search for 1 [Huckmon].".into();
            o
        })
        .add_card(with_cost(named("W-HUCK-18", "Huckmon Eighteen", CardColor::White, 6, 12000), 18))
        .add_card(with_cost(named("W-HUCK-19", "Huckmon Nineteen", CardColor::White, 6, 12000), 19))
        .add_card(named("BLACK-ALLY", "Blackmon", CardColor::Black, 3, 3000))
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

// ─── Candidate legality — only cards the play/use at cost − 3 can carry out ──
// DCGO EX13_012.cs:46-52 `HasCardColor(White) && HasText("Huckmon") &&
// CanPlayOrUse(cardSource, activateClass, fixedCost: max(0, GetCostItself - 3))`;
// `CanPlayOrUse` (PlayEffects.cs:286-294) = `(IsOption && !CanNotPlayThisOption
// && Owner.MaxMemoryCost >= fixedCost) || (HasPlayCost &&
// CanPlayAsNewPermanent(payCost, fixedCost))`, and `CanNotPlayThisOption`
// includes the colour requirement (CardSource.cs:238-243). Rules: a Digi-Egg
// has no play cost so it is never "played" (CR 4.3 7-1-1); an Option needs
// every colour's requirement met to be used (CR 4.3 4-22-2/4-22-3); a card whose
// cost can't be paid can't be played (CR 4.3 7-1-2-3); an impossible action is
// not carried out (CR 4.3 1-3-2).

fn add_player_lock(r: &mut DebugRunner, modifier: digimon_engine::enums::ModifierType) {
    use digimon_engine::enums::Expiry;
    use digimon_engine::modifiers::PlayerModifierEntry;
    r.game
        .modifiers
        .add_player_modifier(0, PlayerModifierEntry::simple(modifier, 0, Expiry::Permanent, None, 1));
}

/// SaviorHuckmon (EX13-012) alone on the field (red/white), memory 5, attacking
/// → the [When Attacking] hand pick's offered cards.
fn offered_when_attacking(hand: &[&str], setup: impl FnOnce(&mut DebugRunner)) -> Vec<String> {
    let mut r = start(hand, 5);
    let h = r.place_on_field(0, CARD_ID, Some(0));
    setup(&mut r);
    r.attack_player(h, 1, false);
    offered_hand_ids(&r)
}

#[test]
fn ex13_012_digi_egg_with_huckmon_text_is_not_offered() {
    assert_eq!(
        offered_when_attacking(&["W-HUCK-EGG", "W-HUCK"], |_| {}),
        vec!["W-HUCK".to_string()],
        "a Digi-Egg has no play cost (CR 4.3 7-1-1; DCGO HasPlayCost)"
    );
}

#[test]
fn ex13_012_option_with_unmet_colour_requirement_is_not_offered() {
    assert_eq!(
        offered_when_attacking(&["WB-HUCK-OPT", "W-HUCK-OPT"], |_| {}),
        vec!["W-HUCK-OPT".to_string()],
        "white/black Option needs a black Digimon or Tamer too (CR 4.3 4-22-3)"
    );
}

/// Regression guard: with a black Digimon on the field the white/black Option
/// meets both colour requirements and is offered.
#[test]
fn ex13_012_option_with_met_multicolour_requirement_is_offered() {
    assert_eq!(
        offered_when_attacking(&["WB-HUCK-OPT", "W-HUCK-OPT"], |r| {
            r.place_on_field(0, "BLACK-ALLY", Some(0));
        }),
        vec!["W-HUCK-OPT".to_string(), "WB-HUCK-OPT".to_string()]
    );
}

#[test]
fn ex13_012_option_not_offered_when_options_cannot_be_used() {
    assert_eq!(
        offered_when_attacking(&["W-HUCK-OPT", "W-HUCK"], |r| {
            add_player_lock(r, digimon_engine::enums::ModifierType::CannotUseOptionCards);
        }),
        vec!["W-HUCK".to_string()]
    );
}

#[test]
fn ex13_012_digimon_not_offered_under_effect_play_prohibition() {
    assert_eq!(
        offered_when_attacking(&["W-HUCK-OPT", "W-HUCK"], |r| {
            add_player_lock(r, digimon_engine::enums::ModifierType::CannotPlayDigimonByEffect);
        }),
        vec!["W-HUCK-OPT".to_string()]
    );
}

#[test]
fn ex13_012_digimon_not_offered_when_battle_area_full() {
    assert_eq!(
        offered_when_attacking(&["W-HUCK-OPT", "W-HUCK"], |r| {
            r.game.rules.field_slots = r.game.players[0].battle_area.len() as u8;
        }),
        vec!["W-HUCK-OPT".to_string()]
    );
}

/// Memory 5: cost 18 − 3 = 15 is payable (5 − 15 = −10), cost 19 − 3 = 16 is
/// not (DCGO `CanPlayAsNewPermanent(payCost: true, fixedCost)`).
#[test]
fn ex13_012_digimon_whose_reduced_cost_cannot_be_paid_is_not_offered() {
    assert_eq!(
        offered_when_attacking(&["W-HUCK-18", "W-HUCK-19"], |_| {}),
        vec!["W-HUCK-18".to_string()],
        "only the card whose cost − 3 is payable"
    );
}
