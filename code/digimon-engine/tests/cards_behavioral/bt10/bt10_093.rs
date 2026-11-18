//! BT10-093 Yuu Amano — Tamer, Purple, Cost 3. Traits: [General, Bagra Army].
//!
//! # Card text (card image — verified)
//!
//! [All Turns][Once Per Turn] When a purple card is placed under this Tamer,
//! <Draw 1> and gain 1 memory. — `on_add_digivolution_cards` +
//! `event_host_permanent_is_source` + `event_added_card_any: purple`
//! (DCGO `CanTriggerOnAddDigivolutionCard`, `cardEffectCondition: null`).
//! Covered by the `clause1_*` tests at the bottom of this file.
//!
//! [Your Turn][Once Per Turn] When you would play 1 level 4 or higher Digimon
//! card with [Bagra Army] in its traits, by placing up to 3 purple Digimon
//! cards from under your Tamers in the played Digimon card's digivolution
//! cards, reduce the play cost of that Digimon by 2 for each card placed.
//!
//! [Security] Play this card without paying its memory cost.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT10/Purple/BT10_093.cs
//!
//! The full DigiXros interaction (pre-attach ordering + cost stacking with
//! DarknessBagramon's [DigiXros -3]) is pinned by judge-quiz Q29
//! (`tests/judge_quiz/e_partition_digixros.rs`). This file covers the hook on
//! a plain (non-DigiXros) ≥Lv4 [Bagra Army] play.

use digimon_engine::action::space::{PASS};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::selection::SelectionKind;

fn yuu_runner() -> DebugRunner {
    // EX10-031 DarkKnightmon (Lv5, [Bagra Army], cost 7) is the played card;
    // EX10-039 ChuuChuumon (purple Digimon) sits under Yuu Amano.
    let mut r = DebugRunner::builder()
        .dsl_card("BT10-093")
        .expect("BT10-093 Yuu Amano loads")
        .dsl_card("EX10-031")
        .expect("EX10-031 DarkKnightmon loads")
        .dsl_card("EX10-039")
        .expect("EX10-039 ChuuChuumon loads")
        .memory(10)
        .hand(0, &["EX10-031"])
        .start();
    r.skip_mulligan();
    r.set_first_player(0);
    let yuu = r.place_on_field(0, "BT10-093", Some(0));
    r.push_source(yuu, "EX10-039");
    r
}

#[test]
fn bt10_093_compiles_and_is_in_pack() {
    let r = DebugRunner::builder()
        .dsl_card("BT10-093")
        .expect("BT10-093 YAML parses and compiles")
        .build();
    assert!(r.compiled_card("BT10-093").is_some());
}

/// The would-play hook fires on a plain ≥Lv4 [Bagra Army] Digimon play:
/// accepting it and placing ChuuChuumon reduces the cost by 2 and puts the
/// card in the played Digimon's digivolution cards.
#[test]
fn bt10_093_hook_places_under_tamer_card_and_reduces_cost_by_2() {
    let mut r = yuu_runner();
    let memory_before = r.memory();

    let _ = r.play(0, 0);

    // Optional accept prompt for the [Your Turn][Once Per Turn] hook.
    let view = r
        .pending_selection_view()
        .expect("Yuu Amano's would-play hook must surface");
    assert!(
        view.is_optional,
        "the printed 'by placing...' hook is optional"
    );
    if view.card_choice.is_none() {
        let accept = view.valid_action_ids[0];
        r.execute_action(0, accept).expect("accept the hook");
    }

    // Pick ChuuChuumon from under Yuu Amano, then close the up-to-3 select.
    let view = r
        .pending_selection_view()
        .expect("the under-Tamer source pick must surface");
    let pick = physical_menu::card_action(&r.game, "EX10-039", digimon_engine::owned_card_choice::CardChoiceZone::BattleSource);
    assert!(view.valid_action_ids.contains(&pick));
    r.execute_action(0, pick).expect("pick ChuuChuumon");
    if r.pending_selection_view()
        .is_some_and(|v| matches!(v.kind, SelectionKind::SourceMulti { .. }))
    {
        r.execute_action(0, PASS).expect("close the multi-select");
    }
    // DarkKnightmon's own [On Play] (select a Digimon to shield) resolves after.
    let _ = r.auto_resolve();

    // Cost 7 − 2 = 5 paid.
    assert_eq!(
        r.memory(),
        memory_before - 5,
        "DarkKnightmon must cost 7 − 2 = 5 after one placed card"
    );

    // ChuuChuumon is now a digivolution card of the played DarkKnightmon...
    let dk = r.game.players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&r.game.card_data) == "EX10-031")
        .expect("DarkKnightmon must be on the field");
    assert!(
        dk.card_sources
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "EX10-039"),
        "ChuuChuumon must be placed in DarkKnightmon's digivolution cards"
    );
    // ...and no longer under Yuu Amano.
    let yuu = r.game.players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&r.game.card_data) == "BT10-093")
        .expect("Yuu Amano remains on field");
    assert!(
        !yuu.card_sources
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "EX10-039"),
        "ChuuChuumon must leave Yuu Amano's source stack"
    );
}

/// Declining the optional hook plays the Digimon at full printed cost and
/// leaves the under-Tamer card in place.
#[test]
fn bt10_093_declining_hook_plays_at_full_cost() {
    let mut r = yuu_runner();
    let memory_before = r.memory();

    let _ = r.play(0, 0);

    // Decline at the accept prompt (or PASS the source select if it surfaced
    // directly).
    if let Some(view) = r.pending_selection_view() {
        assert!(view.is_optional);
        r.decline_optional_trigger()
            .or_else(|_| r.execute_action(0, PASS))
            .expect("decline Yuu Amano's hook");
    }
    let _ = r.auto_resolve();

    assert_eq!(
        r.memory(),
        memory_before - 7,
        "declining the hook must pay DarkKnightmon's full printed cost 7"
    );
    let yuu = r.game.players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&r.game.card_data) == "BT10-093")
        .expect("Yuu Amano remains on field");
    assert!(
        yuu.card_sources
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "EX10-039"),
        "ChuuChuumon must stay under Yuu Amano when the hook is declined"
    );
}

// ─── Clause 1 — [All Turns][OPT] purple card placed under this Tamer ─────────

use std::sync::Arc;

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunnerBuilder};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind, CardSourceRef, PlayerId};
use digimon_engine::permanent::PermanentHandle;

fn colored(id: &str, color: CardColor) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![color];
    c.level = Some(3);
    c.dp = Some(3000);
    c.play_cost = 3;
    c
}

/// The CAUSE: an [On Play] effect that places the first `count` cards of its
/// controller's hand under `target` (the engine's per-host batch window
/// coalesces them into ONE `OnAddDigivolutionCards` firing, CR 4.3 15-5-2).
struct PlaceHandUnder {
    count: usize,
    target: PermanentHandle,
}
impl CardEffect for PlaceHandUnder {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let count = self.count;
        let target = self.target;
        vec![Effect::on_play(card)
            .name("test placer: tuck hand cards under a permanent")
            .process(move |ctx| {
                for _ in 0..count {
                    let ok = ctx.place_as_bottom_source(
                        CardSourceRef::Hand(ctx.player, 0),
                        target,
                        false,
                    );
                    assert!(ok, "placement must succeed");
                }
            })
            .build()]
    }
}

fn clause1_base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card("BT10-093")
        .expect("BT10-093 loads")
        .add_card(colored("PURPLE", CardColor::Purple))
        .add_card(colored("RED", CardColor::Red))
        .add_card(colored("HOST", CardColor::Purple))
        .add_card(colored("PLACER", CardColor::Red))
        .add_card(colored("PLACER2", CardColor::Red))
        .add_card(colored("OPP-PLACER", CardColor::Red))
        .add_card(colored("FILL", CardColor::Green))
        .deck(0, &["FILL"; 10])
        .deck(1, &["FILL"; 10])
        .memory(5)
}

fn tuck(
    r: &mut DebugRunner,
    player: PlayerId,
    placer_id: &str,
    count: usize,
    target: PermanentHandle,
) {
    r.register_effect(placer_id, Arc::new(PlaceHandUnder { count, target }));
    let placer = r.place_on_field(player, placer_id, Some(1));
    r.game.fire_on_play(player, placer.index as usize);
    r.auto_resolve()
        .expect("Yuu Amano's clause 1 is mandatory: no selection expected");
}

fn yuu_on_turn(r: &mut DebugRunner, turn_player: usize) -> PermanentHandle {
    r.game.turn_count = 1;
    r.game.turn_player_idx = turn_player;
    r.place_on_field(0, "BT10-093", Some(0))
}

/// POSITIVE: your effect places a purple card under this Tamer on your turn ->
/// draw 1 and gain 1 memory.
#[test]
fn bt10_093_clause1_purple_placed_under_tamer_draws_and_gains_memory() {
    let mut r = clause1_base().hand(0, &["PURPLE"]).start();
    let yuu = yuu_on_turn(&mut r, 0);
    let (mem, hand_len) = (r.memory(), r.game.players[0].hand.len());
    tuck(&mut r, 0, "PLACER", 1, yuu);
    assert_eq!(r.memory(), mem + 1, "+1 memory");
    assert_eq!(
        r.game.players[0].hand.len(),
        hand_len - 1 + 1,
        "the tucked card left the hand and 1 card was drawn"
    );
    assert_eq!(
        r.game.players[0].battle_area[yuu.index as usize]
            .card_sources
            .len(),
        2,
        "Yuu Amano itself + the purple card now under it"
    );
}

/// NEGATIVE (colour gate): a non-purple card placed under the Tamer does not
/// trigger.
#[test]
fn bt10_093_clause1_non_purple_card_does_not_trigger() {
    let mut r = clause1_base().hand(0, &["RED"]).start();
    let yuu = yuu_on_turn(&mut r, 0);
    let (mem, hand_len) = (r.memory(), r.game.players[0].hand.len());
    tuck(&mut r, 0, "PLACER", 1, yuu);
    assert_eq!(r.memory(), mem);
    assert_eq!(r.game.players[0].hand.len(), hand_len - 1, "no draw");
}

/// NEGATIVE (host gate): a purple card placed under a different permanent does
/// not trigger ("under THIS Tamer").
#[test]
fn bt10_093_clause1_purple_placed_under_another_permanent_does_not_trigger() {
    let mut r = clause1_base().hand(0, &["PURPLE"]).start();
    let _yuu = yuu_on_turn(&mut r, 0);
    let host = r.place_on_field(0, "HOST", Some(0));
    let (mem, hand_len) = (r.memory(), r.game.players[0].hand.len());
    tuck(&mut r, 0, "PLACER", 1, host);
    assert_eq!(r.memory(), mem);
    assert_eq!(r.game.players[0].hand.len(), hand_len - 1, "no draw");
}

/// [All Turns]: the clause also fires on the OPPONENT's turn, and an
/// opponent's effect placing the purple card counts too (DCGO
/// `cardEffectCondition: null`).
#[test]
fn bt10_093_clause1_fires_on_opponents_turn_and_for_opponent_effects() {
    let mut r = clause1_base().hand(0, &["PURPLE"]).hand(1, &["PURPLE"]).start();
    let yuu = yuu_on_turn(&mut r, 1);
    let (mem, hand_len) = (r.memory(), r.game.players[0].hand.len());
    // The opponent's effect places a card from the OPPONENT's hand under Yuu.
    tuck(&mut r, 1, "OPP-PLACER", 1, yuu);
    // The gauge is stored from the TURN player's perspective (here player 1),
    // so player 0's +1 on the opponent's turn reads as -1.
    assert_eq!(r.memory(), mem - 1, "player 0 gains 1 memory on the opponent's turn");
    assert_eq!(
        r.game.players[0].hand.len(),
        hand_len + 1,
        "the Tamer's controller draws"
    );
}

/// [Once Per Turn]: a second placement the same turn does not re-trigger; a
/// batch of several cards placed by one effect triggers once (CR 4.3 15-5-2).
#[test]
fn bt10_093_clause1_is_once_per_turn() {
    let mut r = clause1_base().hand(0, &["PURPLE", "PURPLE", "PURPLE"]).start();
    let yuu = yuu_on_turn(&mut r, 0);
    let mem = r.memory();
    tuck(&mut r, 0, "PLACER", 2, yuu);
    assert_eq!(r.memory(), mem + 1, "one batch -> one trigger");
    tuck(&mut r, 0, "PLACER2", 1, yuu);
    assert_eq!(r.memory(), mem + 1, "second placement this turn: locked out");
}

#[path = "../../support/owned_card_choice.rs"]
mod physical_menu;
