//! BT16-084 Yolei Inoue & Kari Kamiya — Tamer, Red + Yellow, cost 4.
//!
//! Printed text (data/card_bundles/BT16-084.md, official Bandai DB):
//!
//! ```text
//! [Start of Your Main Phase] You may play 1 [Hawkmon] or [Salamon] from your
//!   hand without paying the cost. At the next end of your opponent's turn,
//!   return it to the hand.
//! [Your Turn] When one of your Digimon digivolves into a red or yellow
//!   Digimon, by suspending this Tamer, gain 1 memory. If DNA digivolving, 1 of
//!   your opponent's Digimon gets -3000 DP for the turn.
//! [Security] Play this card without paying the cost.
//! ```
//!
//! Official Q&A: "Yes, it's returned. In such cases, only the top card is
//! returned to the hand and all digivolution cards are trashed." (the played
//! Digimon is still returned after it digivolved).
//!
//! DCGO: DCGO/Assets/Scripts/CardEffect/BT16/Red/BT16_084.cs. Synthetic boards.

#![allow(dead_code, unused_imports)]

use digimon_engine::action::space::PASS;
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

fn digimon(id: &str, name: &str, level: u8, dp: i32, color: CardColor) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = 3;
    c.colors = vec![color];
    c
}

/// A Lv.4 digivolution target (cost 0 from a Lv.3).
fn evolved(id: &str, color: CardColor) -> CardData {
    let mut c = digimon(id, id, 4, 5000, color);
    c.evo_costs = vec![EvoCost {
        card_color: 0,
        level: 3,
        memory_cost: 0,
    }];
    c
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card("BT16-084")
        .expect("BT16-084 compiles into the embedded pack")
        .add_card(digimon("T-HAWK", "Hawkmon", 3, 1000, CardColor::Red))
        .add_card(digimon("T-SAL", "Salamon", 3, 1000, CardColor::Yellow))
        .add_card(digimon("T-HAWKX", "Hawkmon X", 3, 1000, CardColor::Red))
        .add_card(digimon("T-OPP", "Opp Digimon", 4, 5000, CardColor::Blue))
        .add_card(evolved("T-EVO-RED", CardColor::Red))
        .add_card(evolved("T-EVO-GREEN", CardColor::Green))
        .add_card(digimon("T-MAT-A", "Mat A", 3, 1000, CardColor::Red))
        .add_card(digimon("T-MAT-B", "Mat B", 3, 1000, CardColor::Yellow))
        .add_card(digimon("T-DNA", "Dna Result", 5, 7000, CardColor::Red))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
}

fn on_field(runner: &DebugRunner, p: usize, id: &str) -> bool {
    runner.game.players[p]
        .battle_area
        .iter()
        .any(|perm| perm.top_card().card_id(&runner.game.card_data) == id)
}

fn in_hand(runner: &DebugRunner, p: usize, id: &str) -> bool {
    runner.game.players[p]
        .hand
        .iter()
        .any(|c| c.card_id(&runner.game.card_data) == id)
}

fn in_trash(runner: &DebugRunner, p: usize, id: &str) -> bool {
    runner.game.players[p]
        .trash
        .iter()
        .any(|c| c.card_id(&runner.game.card_data) == id)
}

/// Accept the Start-of-Main play of `pick_id` (the only offered Hawkmon/Salamon).
fn play_start_of_main(runner: &mut DebugRunner) {
    runner.game.enter_main_phase();
    let view = runner
        .pending_selection_view()
        .expect("the optional [Hawkmon]/[Salamon] pick is parked");
    assert_eq!(view.kind, SelectionKind::Hand);
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("pick the card to play");
    runner.auto_resolve().expect("finish the free play");
}

/// Resolve whatever is pending, DECLINING optional prompts (the Tamer's
/// digivolve observer is not under test in the delayed-return scenarios).
fn drain_declining(runner: &mut DebugRunner) {
    let mut guard = 0;
    while runner.pending_selection().is_some() && guard < 30 {
        if runner.pending_is_optional() {
            runner.decline_optional_trigger().expect("decline");
        } else {
            runner.auto_resolve().expect("resolve");
        }
        runner.game.drain_effect_queue();
        guard += 1;
    }
}

// ─── [Start of Your Main Phase] ──────────────────────────────────────────────

/// Only the EXACT names [Hawkmon] / [Salamon] qualify; the play is free.
#[test]
fn bt16_084_start_of_main_offers_only_exact_hawkmon_or_salamon() {
    let mut runner = base()
        .hand(0, &["T-HAWK", "T-SAL", "T-HAWKX"])
        .memory(5)
        .start();
    runner.place_on_field(0, "BT16-084", Some(0));
    runner.game.enter_main_phase();
    let view = runner.pending_selection_view().expect("hand pick parked");
    assert_eq!(view.kind, SelectionKind::Hand);
    let picks: Vec<u16> = view
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != PASS)
        .collect();
    assert_eq!(picks.len(), 2, "[Hawkmon] and [Salamon], not [Hawkmon X]: {picks:?}");
}

#[test]
fn bt16_084_start_of_main_plays_free_and_may_be_declined() {
    let mut runner = base().hand(0, &["T-HAWK"]).memory(5).start();
    runner.place_on_field(0, "BT16-084", Some(0));
    let before = runner.game.memory;
    play_start_of_main(&mut runner);
    assert!(on_field(&runner, 0, "T-HAWK"), "Hawkmon was played");
    assert_eq!(runner.game.memory, before, "played without paying the cost");

    let mut runner = base().hand(0, &["T-HAWK"]).memory(5).start();
    runner.place_on_field(0, "BT16-084", Some(0));
    runner.game.enter_main_phase();
    runner.decline_optional_trigger().expect("decline");
    runner.auto_resolve().expect("finish");
    assert!(!on_field(&runner, 0, "T-HAWK"), "declined: nothing played");
}

#[test]
fn bt16_084_start_of_main_offers_nothing_without_hawkmon_or_salamon_in_hand() {
    let mut runner = base().hand(0, &["T-HAWKX"]).memory(5).start();
    runner.place_on_field(0, "BT16-084", Some(0));
    runner.game.enter_main_phase();
    assert!(runner.pending_selection().is_none(), "no [Hawkmon]/[Salamon]: no prompt");
}

/// "At the next end of your opponent's turn, return it to the hand."
#[test]
fn bt16_084_played_digimon_returns_to_hand_at_the_next_end_of_the_opponents_turn() {
    let mut runner = base().hand(0, &["T-HAWK"]).memory(5).start();
    runner.place_on_field(0, "BT16-084", Some(0));
    play_start_of_main(&mut runner);
    assert!(on_field(&runner, 0, "T-HAWK"));

    // End P0's own turn: it stays (the rider waits for the OPPONENT's turn end).
    runner.end_turn();
    drain_declining(&mut runner);
    assert!(on_field(&runner, 0, "T-HAWK"), "still in play during the opponent's turn");
    assert!(!in_hand(&runner, 0, "T-HAWK"));

    // End P1's turn: it returns.
    runner.end_turn();
    drain_declining(&mut runner);
    assert!(!on_field(&runner, 0, "T-HAWK"), "returned at the opponent's turn end");
    assert!(in_hand(&runner, 0, "T-HAWK"), "back in hand");
}

/// Official Q&A: after the played Digimon digivolved it is STILL returned; only
/// the top card goes to the hand and the digivolution cards are trashed.
#[test]
fn bt16_084_return_follows_the_digimon_after_it_digivolved() {
    let mut runner = base()
        .hand(0, &["T-HAWK", "T-EVO-GREEN"])
        .memory(10)
        .start();
    runner.place_on_field(0, "BT16-084", Some(0));
    runner.game.enter_main_phase();
    // Pick Hawkmon (the only candidate), play it free.
    let view = runner.pending_selection_view().expect("pick parked");
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("pick Hawkmon");
    runner.auto_resolve().expect("free play");
    assert!(on_field(&runner, 0, "T-HAWK"));

    // Digivolve it (green: not a red/yellow trigger for the Tamer).
    let host_idx = runner.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&runner.game.card_data) == "T-HAWK")
        .expect("Hawkmon on field");
    let hand_idx = runner.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "T-EVO-GREEN")
        .expect("evolved card in hand");
    let ok = runner
        .game
        .digivolve_from_hand(0, hand_idx, host_idx, PlaySource::ByDigivolve);
    assert!(ok, "digivolve must succeed");
    runner.game.drain_effect_queue();
    drain_declining(&mut runner);
    assert!(on_field(&runner, 0, "T-EVO-GREEN"), "digivolved Digimon in play");

    runner.end_turn();
    drain_declining(&mut runner);
    runner.end_turn();
    drain_declining(&mut runner);

    assert!(!on_field(&runner, 0, "T-EVO-GREEN"), "the Digimon left the battle area");
    assert!(
        in_hand(&runner, 0, "T-EVO-GREEN"),
        "only the TOP card returns to the hand"
    );
    assert!(
        !in_hand(&runner, 0, "T-HAWK") && in_trash(&runner, 0, "T-HAWK"),
        "the digivolution card is trashed, not returned"
    );
}

/// If the played Digimon already left play, the delayed return does nothing.
#[test]
fn bt16_084_return_does_nothing_if_the_digimon_already_left_play() {
    let mut runner = base().hand(0, &["T-HAWK"]).memory(5).start();
    runner.place_on_field(0, "BT16-084", Some(0));
    play_start_of_main(&mut runner);
    let idx = runner.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&runner.game.card_data) == "T-HAWK")
        .expect("Hawkmon on field");
    let handle = PermanentHandle {
        player: 0,
        index: idx as u8,
    };
    let src = runner.game.player(0).deck[0].handle();
    {
        let mut ctx = EffectContext::new(&mut runner.game, src, None, 0);
        ctx.delete_permanent(handle);
    }
    runner.game.drain_effect_queue();
    assert!(in_trash(&runner, 0, "T-HAWK"));
    runner.end_turn();
    drain_declining(&mut runner);
    runner.end_turn();
    drain_declining(&mut runner);
    assert!(!in_hand(&runner, 0, "T-HAWK"), "a deleted Digimon is not returned from the trash");
}

// ─── [Your Turn] digivolve observer ──────────────────────────────────────────

fn digivolve_into(runner: &mut DebugRunner, evo_id: &str, base_id: &str) -> PermanentHandle {
    let host_idx = runner.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&runner.game.card_data) == base_id)
        .expect("base on field");
    let hand_idx = runner.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == evo_id)
        .expect("evolved card in hand");
    let ok = runner
        .game
        .digivolve_from_hand(0, hand_idx, host_idx, PlaySource::ByDigivolve);
    assert!(ok, "digivolve must succeed");
    runner.game.drain_effect_queue();
    PermanentHandle {
        player: 0,
        index: host_idx as u8,
    }
}

#[test]
fn bt16_084_digivolving_into_red_suspends_the_tamer_for_one_memory() {
    let mut runner = base().hand(0, &["T-EVO-RED"]).memory(3).start();
    let tamer = runner.place_on_field(0, "BT16-084", Some(0));
    runner.place_on_field(0, "T-MAT-A", Some(0));
    runner.game.enter_main_phase();
    let before = runner.game.memory;
    digivolve_into(&mut runner, "T-EVO-RED", "T-MAT-A");
    let view = runner.pending_selection_view().expect("optional prompt parked");
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("accept");
    runner.auto_resolve().expect("finish");
    assert!(
        runner.game.players[0].battle_area[tamer.index as usize].is_suspended,
        "the Tamer was suspended as the cost"
    );
    assert_eq!(runner.game.memory, before + 1, "+1 memory");
}

#[test]
fn bt16_084_digivolving_into_green_does_not_trigger() {
    let mut runner = base().hand(0, &["T-EVO-GREEN"]).memory(3).start();
    runner.place_on_field(0, "BT16-084", Some(0));
    runner.place_on_field(0, "T-MAT-A", Some(0));
    runner.game.enter_main_phase();
    digivolve_into(&mut runner, "T-EVO-GREEN", "T-MAT-A");
    assert!(
        runner.pending_selection().is_none(),
        "a green top card is neither red nor yellow: no prompt"
    );
}

/// DNA digivolving into a red/yellow Digimon: +1 memory and 1 opposing Digimon
/// gets -3000 DP for the turn.
#[test]
fn bt16_084_dna_digivolve_gives_an_opposing_digimon_minus_3000_dp() {
    let mut runner = base().hand(0, &["T-DNA"]).memory(3).start();
    let tamer = runner.place_on_field(0, "BT16-084", Some(0));
    let mat_a = runner.place_on_field(0, "T-MAT-A", None);
    let mat_b = runner.place_on_field(0, "T-MAT-B", None);
    let opp = runner.place_on_field(1, "T-OPP", None);
    let before = runner.game.memory;
    let hand_card = runner.game.player(0).hand[0].handle();
    {
        let mut ctx = EffectContext::new(&mut runner.game, hand_card, None, 0);
        ctx.effect_initiated_dna_digivolve(mat_a, mat_b, hand_card, 0, true);
    }
    runner.game.drain_effect_queue();
    let mut guard = 0;
    while let Some(view) = runner.pending_selection_view() {
        assert!(guard < 12, "selection loop did not terminate");
        runner
            .execute_action(view.selecting_player, view.valid_action_ids[0])
            .expect("drive selection");
        runner.game.drain_effect_queue();
        guard += 1;
    }
    assert!(runner.game.players[0].battle_area[tamer.index as usize].is_suspended);
    assert_eq!(runner.game.memory, before + 1, "+1 memory");
    assert_eq!(
        runner.effective_dp(opp),
        Some(2000),
        "the opposing Digimon (5000) got -3000 DP for the turn"
    );
}

/// A non-DNA digivolve into red gives the memory but no DP penalty.
#[test]
fn bt16_084_non_dna_digivolve_has_no_dp_penalty() {
    let mut runner = base().hand(0, &["T-EVO-RED"]).memory(3).start();
    runner.place_on_field(0, "BT16-084", Some(0));
    runner.place_on_field(0, "T-MAT-A", Some(0));
    let opp = runner.place_on_field(1, "T-OPP", None);
    runner.game.enter_main_phase();
    digivolve_into(&mut runner, "T-EVO-RED", "T-MAT-A");
    let view = runner.pending_selection_view().expect("optional prompt parked");
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("accept");
    runner.auto_resolve().expect("finish");
    assert_eq!(runner.effective_dp(opp), Some(5000), "no DNA: no -3000 DP");
}

// ─── [Security] ──────────────────────────────────────────────────────────────

#[test]
fn bt16_084_security_plays_itself_free() {
    let mut runner = base().security(1, &["BT16-084"]).memory(5).start();
    let attacker = runner.place_on_field(0, "T-OPP", Some(0));
    let _ = runner.attack_player(attacker, 1, false);
    runner.auto_resolve().expect("finish");
    assert!(on_field(&runner, 1, "BT16-084"), "the Tamer played itself from security");
}
