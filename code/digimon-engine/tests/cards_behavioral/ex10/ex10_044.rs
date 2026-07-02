//! EX10-044 Damemon — Digimon, Lv.4, Purple, DP 3000, Cost 4.
//! Traits: [Mutant, Bagra Army].
//!
//! # Card text (card image — verified)
//!
//! [On Play] By placing 1 [Bagra Army] trait Digimon card from your hand or
//! trash under any of your Tamers, <Draw 1>.
//!
//! [On Deletion] You may play 1 [Tuwarmon] with a play cost of 7 or less from
//! under your Tamers without paying the cost. Then, <Save>.
//!
//! Inherited: When effects trash this card from a [Bagra Army] trait
//! Digimon's digivolution cards, <Draw 1>.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX10/Purple/EX10_044.cs

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::CardKind;
use digimon_engine::replacement::ReplacementCause;

fn make_tuwarmon() -> CardData {
    let mut c = make_test_card("TUWARMON-T", "Tuwarmon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(6000);
    c.play_cost = 7;
    c.traits = vec!["Bagra Army".to_string()];
    c
}

fn make_bagra_carrier(id: &str) -> CardData {
    let mut c = make_test_card(id, &format!("{id} Bagra Carrier"));
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.traits = vec!["Bagra Army".to_string()];
    c
}

#[test]
fn ex10_044_compiles_and_is_in_pack() {
    let r = DebugRunner::builder()
        .dsl_card("EX10-044")
        .expect("EX10-044 YAML parses and compiles")
        .build();
    assert!(r.compiled_card("EX10-044").is_some());
}

/// [On Play] (hand branch): placing a [Bagra Army] card from hand under a
/// Tamer pays the cost of <Draw 1>.
#[test]
fn ex10_044_on_play_places_bagra_card_under_tamer_then_draws_1() {
    let mut r = DebugRunner::builder()
        .dsl_card("EX10-044")
        .expect("EX10-044 loads")
        .dsl_card("BT10-093")
        .expect("BT10-093 Yuu Amano (Tamer) loads")
        .dsl_card("EX10-039")
        .expect("EX10-039 ChuuChuumon ([Bagra Army]) loads")
        .add_card({
            let mut c = make_test_card("DAME-FILL", "Filler");
            c.card_kind = CardKind::Digimon;
            c
        })
        .deck(0, &["DAME-FILL"; 5])
        .memory(10)
        .hand(0, &["EX10-039"])
        .start();
    r.skip_mulligan();
    r.set_first_player(0);

    let _yuu = r.place_on_field(0, "BT10-093", Some(0));
    let dame = r.place_on_field(0, "EX10-044", Some(0));
    let hand_before = r.hand_size(0); // 1 (ChuuChuumon)

    r.fire_on_play(0, dame.index as usize);

    // Optional clause: accept gate → zone choice "From hand" → pick
    // ChuuChuumon → pick the Tamer → draw 1.
    let view = r
        .pending_selection_view()
        .expect("the optional clause's accept prompt must surface");
    assert!(view.is_optional, "the 'By placing...' pick is declinable");
    if view.valid_action_ids.len() == 1 {
        let accept = view.valid_action_ids[0];
        r.execute_action(0, accept).expect("accept the clause");
    }
    let view = r
        .pending_selection_view()
        .expect("the zone-choice prompt must surface");
    let from_hand = view.valid_action_ids[0];
    r.execute_action(0, from_hand).expect("choose 'From hand'");
    r.auto_resolve().expect("card pick + Tamer pick resolve");

    // Net hand change: −1 (placed) +1 (drawn) = 0.
    assert_eq!(
        r.hand_size(0),
        hand_before,
        "place 1 from hand + Draw 1 must net to an unchanged hand size"
    );
    let yuu = r.game.players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&r.game.card_data) == "BT10-093")
        .expect("Yuu Amano on field");
    assert!(
        yuu.card_sources
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "EX10-039"),
        "ChuuChuumon must be placed under the Tamer"
    );
}

/// [On Deletion]: play a ≤7-cost [Tuwarmon] from under your Tamers for free;
/// then <Save> places the deleted Damemon under a Tamer.
#[test]
fn ex10_044_on_deletion_plays_tuwarmon_from_under_tamer_then_saves() {
    let mut r = DebugRunner::builder()
        .dsl_card("EX10-044")
        .expect("EX10-044 loads")
        .add_card({
            let mut c = make_test_card("DAME-TAMER", "Plain Tamer");
            c.card_kind = CardKind::Tamer;
            c
        })
        .add_card(make_tuwarmon())
        .memory(10)
        .start();
    r.skip_mulligan();
    r.set_first_player(0);

    let yuu = r.place_on_field(0, "DAME-TAMER", Some(0));
    r.push_source(yuu, "TUWARMON-T");
    let dame = r.place_on_field(0, "EX10-044", Some(0));
    let memory_before = r.memory();

    // Delete Damemon by an (opponent) effect.
    let _ = r
        .game
        .delete_permanents_batch(vec![dame], ReplacementCause::OpponentEffect);
    r.game.drain_effect_queue();

    // The [On Deletion] is optional; drive all of its picks (Tuwarmon from
    // under the Tamer, then the Save destination), always preferring a real
    // pick over PASS — the Save destination select is itself a "you may".
    let pass = digimon_engine::action::space::PASS;
    for _ in 0..8 {
        let Some(view) = r.pending_selection_view() else {
            break;
        };
        let action = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|&a| a != pass)
            .unwrap_or(pass);
        let player = view.selecting_player;
        if r.execute_action(player, action).is_err() {
            break;
        }
    }

    // Tuwarmon was played without paying the cost.
    assert!(
        r.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&r.game.card_data) == "TUWARMON-T"),
        "Tuwarmon must be played from under the Tamer"
    );
    assert_eq!(r.memory(), memory_before, "the play must cost no memory");

    // <Save>: the deleted Damemon sits under the Tamer, not in trash.
    let yuu_perm = r.game.players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&r.game.card_data) == "DAME-TAMER")
        .expect("the Tamer remains on field");
    assert!(
        yuu_perm
            .card_sources
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "EX10-044"),
        "Save must place the deleted Damemon under the Tamer"
    );
    assert!(
        !r.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "EX10-044"),
        "the saved Damemon must not remain in the trash"
    );
}

/// G-TRIGGER-CONTEXT-CLOBBERED-BY-COST-REDUCTION-INTERRUPT repro: the same
/// [On Deletion] flow, but the Tamer is Yuu Amano (BT10-093) — a would-play
/// cost-reduction observer. The free Tuwarmon play mid-tail fires Yuu's
/// BeforePayCost accept dialog; after it resolves, the trailing <Save>'s
/// `event_card` (the deleted-object snapshot) must STILL resolve so Damemon
/// is placed under the Tamer. Before the fix, the interrupt replaced
/// `current_trigger_context` and the Save silently no-op'd (Damemon stayed
/// in trash).
#[test]
fn ex10_044_save_survives_cost_reduction_interrupt() {
    let mut r = DebugRunner::builder()
        .dsl_card("EX10-044")
        .expect("EX10-044 loads")
        .dsl_card("BT10-093")
        .expect("BT10-093 Yuu Amano (would-play cost-reduction Tamer) loads")
        .add_card(make_tuwarmon())
        .memory(10)
        .start();
    r.skip_mulligan();
    r.set_first_player(0);

    let yuu = r.place_on_field(0, "BT10-093", Some(0));
    r.push_source(yuu, "TUWARMON-T");
    let dame = r.place_on_field(0, "EX10-044", Some(0));

    let _ = r
        .game
        .delete_permanents_batch(vec![dame], ReplacementCause::OpponentEffect);
    r.game.drain_effect_queue();

    // Drive: Tuwarmon pick → Yuu Amano's "Use Cost reduction?" accept dialog
    // (the interrupt under test) → the Save destination pick. Always prefer a
    // real pick over PASS.
    let pass = digimon_engine::action::space::PASS;
    for _ in 0..8 {
        let Some(view) = r.pending_selection_view() else {
            break;
        };
        let action = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|&a| a != pass)
            .unwrap_or(pass);
        let player = view.selecting_player;
        if r.execute_action(player, action).is_err() {
            break;
        }
    }

    // Tuwarmon was still played for free.
    assert!(
        r.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&r.game.card_data) == "TUWARMON-T"),
        "Tuwarmon must be played from under the Tamer"
    );

    // ── THE PIN ─────────────────────────────────────────────────────────────
    // The Save must survive the interleaved cost-reduction interrupt: the
    // deleted Damemon sits under Yuu Amano, not in the trash.
    let yuu_perm = r.game.players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&r.game.card_data) == "BT10-093")
        .expect("Yuu Amano remains on field");
    assert!(
        yuu_perm
            .card_sources
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "EX10-044"),
        "Save must place the deleted Damemon under the Tamer even when a \
         cost-reduction interrupt fired mid-tail \
         (G-TRIGGER-CONTEXT-CLOBBERED-BY-COST-REDUCTION-INTERRUPT)"
    );
    assert!(
        !r.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "EX10-044"),
        "the saved Damemon must not remain in the trash"
    );
}

/// Inherited: trashed by an effect from a [Bagra Army] Digimon's digivolution
/// cards → Draw 1.
#[test]
fn ex10_044_inherited_draws_1_when_effect_trashes_it_from_bagra_stack() {
    let mut r = DebugRunner::builder()
        .dsl_card("EX10-044")
        .expect("EX10-044 loads")
        .add_card(make_bagra_carrier("DAME-CARRIER"))
        .add_card({
            let mut c = make_test_card("DAME-FILL3", "Filler");
            c.card_kind = CardKind::Digimon;
            c
        })
        .deck(0, &["DAME-FILL3"; 5])
        .memory(10)
        .start();
    r.skip_mulligan();
    r.set_first_player(0);

    let carrier = r.place_on_field(0, "DAME-CARRIER", Some(0));
    r.push_source(carrier, "EX10-044");
    let hand_before = r.hand_size(0);

    r.trash_one_source(carrier);

    assert_eq!(
        r.hand_size(0),
        hand_before + 1,
        "the inherited on-trash effect must Draw 1"
    );
}
