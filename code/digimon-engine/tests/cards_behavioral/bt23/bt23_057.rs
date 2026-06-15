//! BT23-057 Gankoomon — Digimon, Lv.6, Purple, DP 11000, Cost 11.
//!
//! # Card text (image + cards.json)
//!
//! When this card would be played, by returning 3 cards with [Huckmon],
//! [Sistermon] or [Jesmon] in their names from your trash to the top or
//! bottom of the deck, reduce the play cost by 5.
//!
//! [On Play] [When Digivolving] You may play 1 [Hinukamuy] Token.
//! (Digimon/White/6000 DP/<Alliance> <Reboot> <Blocker>) Then, delete 1 of
//! your opponent's Digimon with a play cost of 6 or less. For each of your
//! other Digimon, add 3 to this effect's play cost maximum.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT23/Black/BT23_057.cs
//!
//! # Patterns this test covers
//! - D2 cost reduction with a `pay_cost` ritual (return 3 named trash cards)
//! - Tokens: `play_token "hinukamuy"` (optional → PASS)
//! - Dynamic play-cost delete cap: 6 + 3 × (count of OTHER own Digimon)
//!
//! # Faithfulness note (top/bottom choice)
//! Printed text returns the 3 cards to "the top or bottom of the deck" — the
//! player chooses. DCGO simplifies to deck-top only; the printed choice is
//! exposed here via a `select_effect_choice` modal inside the cost ritual.

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::{encode_attack, PASS};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming};
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "BT23-057";

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn huckmon(id: &str) -> CardData {
    let mut card = make_test_card(id, "Huckmon");
    card.card_kind = CardKind::Digimon;
    card.level = Some(3);
    card.dp = Some(2000);
    card.play_cost = 3;
    card
}

fn vanilla(id: &str, name: &str) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Digimon;
    card.level = Some(3);
    card.dp = Some(2000);
    card.play_cost = 3;
    card
}

fn opponent_digimon(id: &str, cost: u8) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(4000);
    card.play_cost = cost;
    card
}

fn gankoomon_runner() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT23-057 must load from embedded DSL pack")
}

fn fire_on_play(runner: &mut DebugRunner, source: digimon_engine::PermanentHandle) {
    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(source));
    runner.game.drain_effect_queue();
}

/// Push `card_id` onto player `p`'s trash. The card must already exist in
/// `card_data` (added via `.add_card(...)`).
fn push_to_trash(runner: &mut DebugRunner, p: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .expect("card in card_data");
    let idx = runner.game.next_card_index();
    runner.game.players[p as usize]
        .trash
        .push(digimon_engine::card_source::CardSource::new(data_idx, p, idx));
}

// ─── Section 1: Structural assertions ────────────────────────────────────────

#[test]
fn bt23_057_loads_with_clauses() {
    let runner = gankoomon_runner().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled BT23-057");
    assert!(
        !card.effects.is_empty(),
        "BT23-057 must declare effects (no empty stub)"
    );
}

#[test]
fn bt23_057_has_cost_reduction_clause() {
    let runner = gankoomon_runner().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled BT23-057");
    assert!(
        card.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction { .. })
        )),
        "BT23-057 must have a CostReduction clause"
    );
}

#[test]
fn bt23_057_has_shared_on_play_when_digivolving_clause() {
    let runner = gankoomon_runner().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled BT23-057");
    let shared = card.effects.iter().any(|clause| match clause {
        CompiledClause::Triggered(t) => {
            t.when.contains(&CompiledTiming::OnPlay)
                && t.when.contains(&CompiledTiming::WhenDigivolving)
        }
        _ => false,
    });
    assert!(
        shared,
        "BT23-057 must have a shared [On Play][When Digivolving] clause"
    );
}

#[test]
fn bt23_057_shared_clause_plays_token_and_deletes() {
    let runner = gankoomon_runner().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled BT23-057");
    let shared = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnPlay)
                    && t.when.contains(&CompiledTiming::WhenDigivolving) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("shared clause");
    assert!(
        shared
            .process
            .iter()
            .any(|step| matches!(step, CompiledStep::PlayToken { .. })),
        "shared clause must play the Hinukamuy token"
    );
    assert!(
        shared
            .process
            .iter()
            .any(|step| matches!(step, CompiledStep::DeletePermanent { .. })),
        "shared clause must delete an opponent Digimon"
    );
}

// ─── Section 2: Cost reduction ritual ────────────────────────────────────────

/// With 3 eligible named cards in trash, playing Gankoomon installs the
/// cost-reduction pay-cost ritual: a count-capped multi-select over trash.
#[test]
fn bt23_057_cost_ritual_prompts_when_three_named_cards_in_trash() {
    let mut runner = gankoomon_runner()
        .add_card(huckmon("HUCK-A"))
        .add_card(huckmon("HUCK-B"))
        .add_card(huckmon("HUCK-C"))
        .hand(0, &[CARD_ID])
        .deck(0, &["HUCK-A", "HUCK-B"])
        .memory(11)
        .start();
    push_to_trash(&mut runner, 0, "HUCK-A");
    push_to_trash(&mut runner, 0, "HUCK-B");
    push_to_trash(&mut runner, 0, "HUCK-C");

    runner.play(0, 0);

    assert!(
        runner.pending_selection().is_some(),
        "cost-reduction ritual must surface a selection when 3 named cards are in trash"
    );
}

/// Negative: with only 2 eligible named cards the cost cannot be paid, so the
/// ritual installs no trash-return selection (the reduction simply does not apply).
#[test]
fn bt23_057_cost_ritual_absent_with_only_two_named_cards() {
    let mut runner = gankoomon_runner()
        .add_card(huckmon("HUCK-A"))
        .add_card(huckmon("HUCK-B"))
        .add_card(vanilla("PLAIN", "Random"))
        .hand(0, &[CARD_ID])
        .deck(0, &["HUCK-A"])
        .memory(11)
        .start();
    push_to_trash(&mut runner, 0, "HUCK-A");
    push_to_trash(&mut runner, 0, "HUCK-B");
    push_to_trash(&mut runner, 0, "PLAIN");

    runner.play(0, 0);

    // Only 2 eligible cards → the multi-select min=3 is unmet → no cost ritual.
    // No opponent Digimon present, so the On Play delete clause adds no prompt.
    assert!(
        runner.pending_selection().is_none(),
        "no cost ritual selection when fewer than 3 named cards are in trash"
    );
}

// ─── Section 3: [On Play] token + dynamic-cap delete ─────────────────────────

#[test]
fn bt23_057_on_play_token_is_optional_pass() {
    let mut runner = gankoomon_runner()
        .add_card(opponent_digimon("OPP-6", 6))
        .memory(11)
        .start();
    let gank = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "OPP-6", Some(0));
    runner.game.turn_count = 1;

    fire_on_play(&mut runner, gank);

    let view = runner
        .pending_selection_view()
        .expect("token may-play prompt must install");
    assert!(
        view.is_optional || view.valid_action_ids.iter().any(|a| *a == PASS),
        "playing the Hinukamuy token is optional (you may) — a decline must be exposed"
    );
}

/// Decline the token, then the mandatory delete fires on an opponent Digimon
/// with play cost ≤ 6 (no other own Digimon → cap is exactly 6).
#[test]
fn bt23_057_on_play_deletes_opp_cost6_after_declining_token() {
    let mut runner = gankoomon_runner()
        .add_card(opponent_digimon("OPP-6", 6))
        .memory(11)
        .start();
    let gank = runner.place_on_field(0, CARD_ID, Some(0));
    let opp = runner.place_on_field(1, "OPP-6", Some(0));
    runner.game.turn_count = 1;
    let opp_before = runner.game.players[1].battle_area.len();

    fire_on_play(&mut runner, gank);

    // Decline the token (PASS or the "Don't play token" branch action).
    decline_token(&mut runner);

    // Now the mandatory delete prompt should target the cost-6 opponent Digimon.
    let view = runner
        .pending_selection_view()
        .expect("delete prompt must install for a cost-6 opponent Digimon");
    assert_eq!(view.kind, SelectionKind::OppField);
    runner
        .execute_action(0, encode_attack(0, opp.index as u16))
        .expect("delete the cost-6 opponent Digimon");
    runner.auto_resolve().ok();

    assert!(
        runner.game.players[1].battle_area.len() < opp_before,
        "cost-6 opponent Digimon must be deleted"
    );
}

/// A play-cost-7 opponent Digimon is NOT eligible when Gankoomon has no other
/// own Digimon (cap = 6). Declining the token leaves no legal delete target.
#[test]
fn bt23_057_on_play_cost7_not_deletable_with_no_other_own_digimon() {
    let mut runner = gankoomon_runner()
        .add_card(opponent_digimon("OPP-7", 7))
        .memory(11)
        .start();
    let gank = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "OPP-7", Some(0));
    runner.game.turn_count = 1;
    let opp_before = runner.game.players[1].battle_area.len();

    fire_on_play(&mut runner, gank);
    decline_token(&mut runner);
    runner.auto_resolve().ok();

    assert_eq!(
        runner.game.players[1].battle_area.len(),
        opp_before,
        "a cost-7 opponent Digimon is above the 6 cap with no other own Digimon"
    );
}

/// With 2 OTHER own Digimon, the cap is 6 + 3×2 = 12, so a cost-12 opponent
/// Digimon becomes deletable.
#[test]
fn bt23_057_dynamic_cap_scales_with_other_own_digimon() {
    let mut runner = gankoomon_runner()
        .add_card(vanilla("ALLY-A", "AllyA"))
        .add_card(vanilla("ALLY-B", "AllyB"))
        .add_card(opponent_digimon("OPP-12", 12))
        .memory(11)
        .start();
    let gank = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "ALLY-A", Some(0));
    runner.place_on_field(0, "ALLY-B", Some(0));
    let opp = runner.place_on_field(1, "OPP-12", Some(0));
    runner.game.turn_count = 1;
    let opp_before = runner.game.players[1].battle_area.len();

    fire_on_play(&mut runner, gank);
    decline_token(&mut runner);

    let view = runner
        .pending_selection_view()
        .expect("cost-12 opponent Digimon is within cap 6+3*2=12");
    assert_eq!(view.kind, SelectionKind::OppField);
    runner
        .execute_action(0, encode_attack(0, opp.index as u16))
        .expect("delete the cost-12 opponent Digimon");
    runner.auto_resolve().ok();

    assert!(
        runner.game.players[1].battle_area.len() < opp_before,
        "cost-12 opponent Digimon must be deletable once the cap reaches 12"
    );
}

/// Decline the "you may play 1 Hinukamuy Token" choice — picks PASS if exposed,
/// otherwise the "Don't play token" effect-choice branch (action index 1).
fn decline_token(runner: &mut DebugRunner) {
    let view = runner
        .pending_selection_view()
        .expect("token may-play prompt");
    let player = view.selecting_player;
    if view.valid_action_ids.iter().any(|a| *a == PASS) {
        runner.execute_action(player, PASS).expect("decline token via PASS");
    } else {
        // EffectChoice "Don't play token" is the second label (index 1).
        let decline = view
            .valid_action_ids
            .last()
            .copied()
            .expect("token choice has a decline branch");
        runner
            .execute_action(player, decline)
            .expect("decline token via effect-choice branch");
    }
}
