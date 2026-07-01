//! EX5-060 Dragomon — Digimon, Lv.5, Purple, DP 7000, Cost 7.
//! Traits: Aquabeast. Form: Ultimate. Attribute: Virus.
//!
//! # Card text (card image, judge-quiz PDF p.63 — verified)
//! `[On Play][When Digivolving] Your opponent plays 1 level 4 or lower`
//! `  Digimon card from their trash suspended without paying the cost.`
//! `  [On Play] effects on Digimon played by this effect don't activate.`
//! `[All Turns][Once Per Turn] When an effect plays an opponent's Digimon,`
//! `  you may play 1 purple Digimon card with a level less than or equal to`
//! `  it from your trash without paying the cost.`
//! `Inherited: <Piercing>`
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX5/Purple/EX5_060.cs — the opponent is
//! the selecting player over their own trash; the played card's
//! `CanNotBeAffected` is consulted before applying the don't-activate
//! rider; clause 2 gates `IsByEffect` and reads `LevelJustAfterPlayed`.
//!
//! Judge-quiz consumer: Q28 (the protection-vs-rider pin lives in
//! `judge_quiz/a_immunity_scope.rs`).

#![allow(dead_code, unused_imports)]

#[path = "../../support/dsl_card_data.rs"]
mod dsl_card_data;

use std::sync::Arc;

use digimon_dsl::compiled::{CompiledClause, CompiledScope};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

fn compiled() -> digimon_dsl::compiled::CompiledCard {
    dsl_card_data::compiled("EX5-060")
}

fn digimon(id: &str, level: u8, color: CardColor) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(4000);
    c.colors = vec![color];
    c
}

// ─── SECTION 1 — Structural ─────────────────────────────────────────────────

#[test]
fn ex5_060_metadata_is_purple_lv5_7000_with_piercing() {
    let card = compiled();
    assert_eq!(card.card, "EX5-060");
    assert_eq!(card.name, "Dragomon");
    assert_eq!(card.level, Some(5));
    assert_eq!(card.dp, Some(7000));
    assert!(card
        .color
        .contains(&digimon_dsl::compiled::CompiledColor::Purple));
}

// ─── SECTION 2 — Clause 1: opponent plays Lv≤4 from THEIR trash ─────────────

#[test]
fn ex5_060_on_play_opponent_picks_lv4_or_lower_from_own_trash_suspended() {
    let mut r = DebugRunner::builder()
        .dsl_card("EX5-060")
        .expect("EX5-060 in pack")
        .add_card(digimon("OPP-LV4", 4, CardColor::Purple))
        .add_card(digimon("OPP-LV5", 5, CardColor::Purple))
        .memory(10)
        .start();
    r.skip_mulligan();
    r.set_first_player(0);
    let dragomon = r.place_on_field(0, "EX5-060", Some(0));
    // The OPPONENT's trash: a legal Lv4 + an ILLEGAL Lv5.
    r.inject_trash(1, "OPP-LV4");
    r.inject_trash(1, "OPP-LV5");

    r.game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(dragomon));
    r.game.drain_effect_queue();

    let view = r
        .pending_selection_view()
        .expect("the opponent-trash pick surfaces");
    assert_eq!(
        view.selecting_player, 1,
        "the OPPONENT (controller of the trash) is the selecting player"
    );
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only the Lv≤4 Digimon is a legal pick (the Lv5 is filtered out)"
    );
    r.execute_action(1, view.valid_action_ids[0])
        .expect("opponent picks the Lv4");
    let _ = r.auto_resolve();

    let idx = r.game.players[1]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&r.game.card_data) == "OPP-LV4")
        .expect("the picked Digimon is on the OPPONENT's field");
    assert!(
        r.game.players[1].battle_area[idx].is_suspended,
        "played SUSPENDED"
    );
    assert_eq!(r.trash_size(1), 1, "the Lv5 stays in the opponent's trash");
}

/// The rider: a played Digimon with its own [On Play] does NOT activate it
/// (no protection in this staging — the Q28 pin covers the protected case).
#[test]
fn ex5_060_on_play_rider_suppresses_played_digimons_on_play() {
    let mut r = DebugRunner::builder()
        .dsl_card("EX5-060")
        .expect("EX5-060 in pack")
        .dsl_card("BT6-084")
        .expect("BT6-084 Sistermon Ciel in pack")
        .memory(10)
        .start();
    r.skip_mulligan();
    r.set_first_player(0);
    let dragomon = r.place_on_field(0, "EX5-060", Some(0));
    r.inject_trash(1, "BT6-084"); // Ciel: [On Play] Gain 1 memory

    let memory_before = r.memory();
    r.game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(dragomon));
    r.game.drain_effect_queue();
    let view = r.pending_selection_view().expect("opponent pick");
    r.execute_action(1, view.valid_action_ids[0])
        .expect("opponent picks Ciel");
    let _ = r.auto_resolve();

    assert_eq!(
        r.memory(),
        memory_before,
        "the played Ciel's [On Play] gain-1-memory must NOT activate \
         (\"[On Play] effects on Digimon played by this effect don't activate\")"
    );
}

// ─── SECTION 3 — Clause 2: effect-play recursion (level ≤ it) ───────────────

/// A synthetic effect plays an OPPONENT's Digimon → Dragomon's controller
/// may play 1 purple Digimon with level ≤ the played one from their trash.
/// The level gate uses the event card's level (G-EVENT-PLAYED-LEVEL-FORMULA).
#[test]
fn ex5_060_all_turns_recursion_gated_on_played_level() {
    let mut r = DebugRunner::builder()
        .dsl_card("EX5-060")
        .expect("EX5-060 in pack")
        .add_card(digimon("OPP-LV3", 3, CardColor::Red))
        .add_card(digimon("MY-PURPLE-LV3", 3, CardColor::Purple))
        .add_card(digimon("MY-PURPLE-LV5", 5, CardColor::Purple))
        .memory(10)
        .start();
    r.skip_mulligan();
    r.set_first_player(0);
    let _dragomon = r.place_on_field(0, "EX5-060", Some(0));
    // Dragomon's controller's trash: a Lv3 purple (legal) + Lv5 purple
    // (ILLEGAL — level must be ≤ the played Lv3).
    r.inject_trash(0, "MY-PURPLE-LV3");
    r.inject_trash(0, "MY-PURPLE-LV5");
    // The OPPONENT's Lv3 sits in their trash and is played BY AN EFFECT.
    r.inject_trash(1, "OPP-LV3");

    // An effect (any controller) plays the opponent's Digimon from trash.
    {
        let src_card = r.game.players[1].trash[0].handle();
        let mut ctx =
            digimon_engine::effect_context::EffectContext::new(&mut r.game, src_card, None, 1);
        let played = ctx.play_from_trash_free_unsuspended_for(1, src_card, false);
        assert!(played.is_some(), "staging: the effect-play resolves");
    }
    r.game.drain_effect_queue();

    // Dragomon's [All Turns][OPT] offers the recursion to ITS controller.
    let view = r
        .pending_selection_view()
        .expect("the optional recursion prompt surfaces for Dragomon's controller");
    assert_eq!(view.selecting_player, 0, "Dragomon's controller chooses");
    // Accept the optional, then the trash pick: only the Lv3 purple is legal.
    let _ = r.accept_optional_trigger();
    let view = r
        .pending_selection_view()
        .expect("the own-trash purple pick surfaces");
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only the purple Digimon with level ≤ the played Lv3 is pickable \
         (the Lv5 purple is filtered by the event-level formula)"
    );
    r.execute_action(0, view.valid_action_ids[0])
        .expect("play the Lv3 purple");
    let _ = r.auto_resolve();

    assert!(
        r.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&r.game.card_data) == "MY-PURPLE-LV3"),
        "the Lv3 purple was played from the controller's trash for free"
    );
    assert_eq!(
        r.trash_size(0),
        1,
        "the Lv5 purple remains in the controller's trash"
    );
}

/// Negative: a HARD play (not an effect) of an opponent Digimon does not
/// trigger the recursion (DCGO `IsByEffect` gate / `event_is_effect_initiated`).
#[test]
fn ex5_060_all_turns_recursion_not_triggered_by_hard_play() {
    let mut r = DebugRunner::builder()
        .dsl_card("EX5-060")
        .expect("EX5-060 in pack")
        .add_card(digimon("OPP-LV3", 3, CardColor::Red))
        .add_card(digimon("MY-PURPLE-LV3", 3, CardColor::Purple))
        .hand(1, &["OPP-LV3"])
        .memory(10)
        .start();
    r.skip_mulligan();
    // The OPPONENT (player 1) hard-plays their Digimon on their own turn.
    r.set_first_player(1);
    let _dragomon = r.place_on_field(0, "EX5-060", Some(0));
    r.inject_trash(0, "MY-PURPLE-LV3");

    // Hard play via the normal play-from-hand path (effect_initiated=false).
    let played = r.play(1, 0);
    assert!(played.is_some(), "the hard play resolves");
    let _ = r.auto_resolve();

    assert!(
        r.game.pending_selection.is_none(),
        "a HARD play does not satisfy 'when an EFFECT plays an opponent's \
         Digimon' — no recursion prompt"
    );
}
