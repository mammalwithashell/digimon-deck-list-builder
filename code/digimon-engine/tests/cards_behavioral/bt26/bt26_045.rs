//! BT26-045 GranKuwagamon — Lv.6 Green, Insectoid/Titan/TS.
//!
//! When this card would be played, if your hand has fewer cards than your
//! opponent's, reduce the cost by 4.
//! [On Play] [When Digivolving] [When Attacking] [Once Per Turn] You may play 1
//! level 4 or lower [Insectoid] or [Titan] trait Digimon card from your hand
//! without paying the cost.
//! [Your Turn] All of your [Insectoid] or [Titan] trait Digimon gain
//! <Alliance>, <Piercing> and <Vortex>.
//!
//! DCGO: BT26/Green/BT26_045.cs.

use super::support::*;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming, Keyword};

const CARD_ID: &str = "BT26-045";

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-045 in embedded DSL pack")
        .add_card(filler("FILLER"))
        .add_card(digimon(
            "BUG4",
            "Bug Four",
            CardColor::Green,
            4,
            5,
            &["Insectoid"],
        ))
        .add_card(digimon(
            "TITAN3",
            "Titan Three",
            CardColor::Purple,
            3,
            3,
            &["Titan"],
        ))
        .add_card(digimon(
            "BUG5",
            "Bug Five",
            CardColor::Green,
            5,
            7,
            &["Insectoid"],
        ))
        .add_card(digimon(
            "BEAST4",
            "Beast Four",
            CardColor::Green,
            4,
            5,
            &["Beast"],
        ))
        // A level-2 [Insectoid] Digi-Egg (synthetic): no play cost.
        .add_card({
            let mut e = digimon_engine::debug_runner::make_test_egg("BUG-EGG", "Bug Egg");
            e.colors = vec![CardColor::Green];
            e.traits = vec!["Insectoid".to_string()];
            e
        })
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(5)
        .start();
    r.set_first_player(0);
    r.game.turn_player_idx = 0;
    r.game.turn_count = 1;
    r
}

#[test]
fn bt26_045_play_cost_reduced_with_smaller_hand() {
    let mut r = setup();
    let idx = r.add_to_hand(0, CARD_ID);
    push_hand(&mut r, 1, "FILLER");
    push_hand(&mut r, 1, "FILLER");
    r.play(0, idx).expect("played");
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), 5 - (11 - 4));
}

#[test]
fn bt26_045_play_cost_full_with_equal_hands() {
    let mut r = setup();
    let idx = r.add_to_hand(0, CARD_ID);
    push_hand(&mut r, 1, "FILLER");
    r.play(0, idx).expect("played");
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), 5 - 11);
}

#[test]
fn bt26_045_on_play_plays_level4_insectoid_free() {
    let mut r = setup();
    let h = r.place_on_field(0, CARD_ID, Some(0));
    push_hand(&mut r, 0, "BUG4");
    push_hand(&mut r, 0, "BUG5");
    push_hand(&mut r, 0, "BEAST4");
    push_hand(&mut r, 0, "TITAN3");
    fire(&mut r, EffectTiming::OnPlay, h);
    let v = r.pending_selection_view().expect("optional hand pick");
    assert!(v.is_optional);
    assert_eq!(non_pass(&r).len(), 2, "BUG4 + TITAN3 only");
    pick_hand(&mut r, 0, "BUG4");
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&"BUG4".to_string()));
    assert_eq!(r.memory(), 5, "without paying the cost");
    // [Once Per Turn] across the shared timings.
    fire(&mut r, EffectTiming::WhenAttacking, h);
    assert!(r.pending_selection_view().is_none());
}

#[test]
fn bt26_045_decline_refunds_once_per_turn() {
    let mut r = setup();
    let h = r.place_on_field(0, CARD_ID, Some(0));
    push_hand(&mut r, 0, "BUG4");
    fire(&mut r, EffectTiming::WhenDigivolving, h);
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    fire(&mut r, EffectTiming::WhenAttacking, h);
    assert!(r.pending_selection_view().is_some(), "re-offered");
}

// ─── Hand-pick candidate legality (select-hand playability audit) ────────────
// DCGO BT26_045.cs:41-45 offers only `IsDigimon && HasLevel && Level <= 4 &&
// (Insectoid || Titan) && CanPlayAsNewPermanent(card, payCost: false,
// activateClass)`, and the effect activates only when such a card is in hand
// (`SharedAdditionalActivateCondition`, :47-48) — so an unplayable card neither
// is offered nor spends the [Once Per Turn] (CR 4.3 1-3-2).

/// "You can't play Digimon by effects": nothing is offered and the [Once Per
/// Turn] is still available once the lock is gone.
#[test]
fn bt26_045_not_offered_when_effect_play_is_prohibited() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::PlayerModifierEntry;
    let mut r = setup();
    let h = r.place_on_field(0, CARD_ID, Some(0));
    push_hand(&mut r, 0, "BUG4");
    let unlocked = r.game.modifiers.clone();
    r.game.modifiers.add_player_modifier(
        0,
        PlayerModifierEntry::simple(ModifierType::CannotPlayDigimonByEffect, 0, Expiry::EndOfTurn, None, 1),
    );
    fire(&mut r, EffectTiming::OnPlay, h);
    assert!(r.pending_selection_view().is_none(), "no playable candidate");
    let _ = r.auto_resolve();
    assert_eq!(hand_ids(&r, 0), vec!["BUG4".to_string()]);
    r.game.modifiers = unlocked;
    fire(&mut r, EffectTiming::WhenAttacking, h);
    assert!(r.pending_selection_view().is_some(), "OPT not spent");
}

/// A full battle area (DCGO needs an empty frame): nothing is offered.
#[test]
fn bt26_045_not_offered_when_battle_area_full() {
    let mut r = setup();
    let h = r.place_on_field(0, CARD_ID, Some(0));
    push_hand(&mut r, 0, "BUG4");
    r.game.rules.field_slots = r.game.players[0].battle_area.len() as u8;
    fire(&mut r, EffectTiming::WhenDigivolving, h);
    assert!(r.pending_selection_view().is_none(), "no free slot");
    let _ = r.auto_resolve();
    assert_eq!(hand_ids(&r, 0), vec!["BUG4".to_string()]);
}

/// A level-2 [Insectoid] Digi-Egg is never a "Digimon card" play candidate
/// (no play cost, CR 4.3 7-1-1). Regression guard.
#[test]
fn bt26_045_digi_egg_is_not_offered() {
    let mut r = setup();
    let h = r.place_on_field(0, CARD_ID, Some(0));
    push_hand(&mut r, 0, "BUG-EGG");
    push_hand(&mut r, 0, "BUG4");
    fire(&mut r, EffectTiming::OnPlay, h);
    let ids = hand_ids(&r, 0);
    let offered: Vec<String> = non_pass(&r)
        .into_iter()
        .map(|a| ids[(a - digimon_engine::action::space::PLAY_HAND_START) as usize].clone())
        .collect();
    assert_eq!(offered, vec!["BUG4".to_string()]);
}

#[test]
fn bt26_045_your_turn_grants_alliance_piercing_vortex() {
    let mut r = setup();
    let h = r.place_on_field(0, CARD_ID, Some(0));
    let bug = r.place_on_field(0, "BUG4", Some(0));
    let beast = r.place_on_field(0, "BEAST4", Some(0));
    r.game.tick_declarative_effects();
    for k in [Keyword::Alliance, Keyword::Piercing, Keyword::Vortex] {
        assert!(r.game.has_keyword(h, k), "{k:?} on self");
        assert!(r.game.has_keyword(bug, k), "{k:?} on Insectoid");
        assert!(!r.game.has_keyword(beast, k), "{k:?} not on Beast");
    }
    // Opponent's turn: the [Your Turn] grants drop.
    r.game.turn_player_idx = 1;
    r.game.tick_declarative_effects();
    assert!(
        !r.game.has_keyword(bug, Keyword::Piercing),
        "[Your Turn] only"
    );
    assert!(
        !r.game.has_keyword(h, Keyword::Alliance),
        "[Your Turn] only"
    );
}

#[test]
fn bt26_045_granted_vortex_offers_end_of_turn_attack() {
    let mut r = setup();
    r.place_on_field(0, CARD_ID, Some(0));
    let bug = r.place_on_field(0, "BUG4", Some(0));
    let opp = r.place_on_field(1, "BEAST4", Some(0));
    r.game.players[1].battle_area[opp.index as usize].is_suspended = true;
    r.game.tick_declarative_effects();
    assert!(r.game.has_keyword(bug, Keyword::Vortex));
    // §16-33-2: the granted <Vortex> triggers at the end of the turn and
    // offers its attack as its own decision inside the end-of-turn pool.
    r.end_turn();
    let attack = digimon_engine::action::space::encode_attack(bug.index as u16, opp.index as u16);
    let offered = |r: &DebugRunner| {
        r.pending_selection()
            .is_some_and(|sel| sel.valid_action_ids.contains(&attack))
    };
    // Other end-of-turn entries (this card's own granted <Vortex>) may pend
    // with it: order them, and decline any other keyword's offer first.
    for _ in 0..4 {
        if offered(&r) {
            break;
        }
        let Some(sel) = r.pending_selection() else {
            break;
        };
        if sel.kind == digimon_engine::selection::SelectionKind::TriggerOrder {
            let pick = sel.effect_choices.as_ref().expect("branches")[0].action_id;
            r.execute_action(0, pick).unwrap();
        } else {
            r.decline_optional_trigger().unwrap();
        }
    }
    assert!(offered(&r), "<Vortex> end-of-turn attack offered");
}
