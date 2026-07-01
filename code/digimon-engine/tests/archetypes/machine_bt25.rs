//! Machine (BT25 slice) — archetype interaction tests.
//!
//! Model: `qa/archetype-qa/machine-bt25-model.md`. Slice cards: BT25-062
//! Kokuwamon (IMPLEMENTED), BT25-066 Guardromon (BLOCKED, dsl gap), BT25-074
//! Tankdramon (BLOCKED, dsl gap). Per Phase-4 precondition gating, only combos
//! whose pieces are all implemented are authored here — so the two combos that
//! name Guardromon / Tankdramon are dropped (logged in the model doc), and the
//! single authored combo pairs the one implemented slice card with an already-
//! implemented cross-set TS-trait partner.
//!
//! # Combo M1 — "Kokuwamon free-digivolve fires a [When Digivolving] payoff"
//!
//! Cards:
//! - **BT25-062 Kokuwamon** (enabler): `[Start of Your Main Phase] If you have 4
//!   or less memory, this Digimon may digivolve into a [Machine], [Cyborg] or
//!   [TS] trait Digimon card in the hand without paying the cost.`
//! - **BT24-035 Gatomon** (payoff, cross-set, traits Holy Beast/Iliad/**TS**):
//!   `[On Play] [When Digivolving] 1 of your opponent's Digimon gets -3000 DP for
//!   the turn. ...`
//!
//! Why this is a *system-level* interaction the two per-card tests cannot see:
//! Kokuwamon's clause performs an **effect-initiated digivolution** (cost-0,
//! `ignore_requirements`), and the engine fires `[When Digivolving]` off an
//! effect-initiated digivolve exactly as off a manual one
//! (`game_actions.rs::effect_initiated_digivolve_from_source_inner`, step 5 —
//! `enqueue_triggered(WhenDigivolving)` + drain). So Kokuwamon's *free tempo*
//! clause is also a *trigger platform*: digivolving into Gatomon makes
//! Gatomon's `[When Digivolving]` −3000-DP debuff appear, even though the
//! controller never manually digivolved and paid nothing. Kokuwamon's per-card
//! test (`cards_behavioral/bt25/bt25_062.rs`) only digivolves into trait-only
//! synthetic targets with no `[When Digivolving]` of their own; Gatomon's
//! per-card test (`cards_behavioral/bt24/bt24_035.rs`) fires its trigger via a
//! direct `enqueue_triggered(OnPlay)`, never through another card's clause.
//!
//! Sources:
//! - Card text: cards.json BT25-062, BT24-035.
//! - DCGO C#: `$BASE_DCGO/Assets/Scripts/CardEffect/BT25/Black/BT25_062.cs`
//!   (`DigivolveIntoHandOrTrashCard(payCost:false)`),
//!   `.../BT24/Yellow/BT24_035.cs` (−3000 DP for the turn).
//! - Rules: an effect-driven digivolution is a digivolution and triggers
//!   `[When Digivolving]` (`general_rule.pdf` digivolution timing).
//!
//! The trait gate is load-bearing: Gatomon is an eligible Kokuwamon evolution
//! target *only* because it carries the **TS** trait (it is not Machine/Cyborg).

#![allow(dead_code)]

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const KOKUWAMON: &str = "BT25-062";
const GATOMON: &str = "BT24-035";

// ─── Combo-piece fixtures ────────────────────────────────────────────────────

/// A neutral opponent Digimon target with a chosen DP, high enough that −3000
/// does not delete it (so the assertion reads the surviving DP rather than a
/// removal — keeps the combo's *claimed* outcome, the debuff, isolated).
fn make_opp_digimon(id: &str, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Purple];
    c.level = Some(4);
    c.dp = Some(dp);
    c.play_cost = 4;
    c
}

/// A Lv.4 Beast Digimon — a legal hand card but NOT a [Machine]/[Cyborg]/[TS]
/// target, so Kokuwamon's `select_hand` picker finds no eligible evolution and
/// the SOMP clause never fires (the enabler-absent / unhappy path).
fn make_lv4_beast(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Green];
    c.level = Some(4);
    c.dp = Some(5000);
    c.play_cost = 4;
    c.traits = vec!["Beast".to_string()];
    c
}

/// Board: Kokuwamon (BT25-062) + Gatomon (BT24-035) loaded from the embedded
/// DSL pack, plus a deck pad so the digivolve's draw has a card. `hand_card` is
/// placed into P0's hand; `memory` and an opponent Digimon complete the combo
/// setup. Returns the runner with Kokuwamon standing on P0's field and the
/// opponent Digimon on P1's field, P0's turn.
fn combo_board(
    hand_card: &str,
    memory: i16,
    opp_dp: i32,
) -> (DebugRunner, PermanentHandle, PermanentHandle) {
    let mut runner = DebugRunner::builder()
        .dsl_card(KOKUWAMON)
        .expect("BT25-062 (Kokuwamon) in embedded DSL pack")
        .dsl_card(GATOMON)
        .expect("BT24-035 (Gatomon) in embedded DSL pack")
        .add_card(make_test_card("DECK-PAD", "Filler"))
        .add_card(make_opp_digimon("OPP", opp_dp))
        .add_card(make_lv4_beast("BEAST-LV4"))
        .deck(0, &["DECK-PAD"; 6])
        .deck(1, &["DECK-PAD"; 6])
        .hand(0, &[hand_card])
        .memory(memory)
        .start();
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 0;

    let koku = runner.place_on_field(0, KOKUWAMON, Some(0));
    let opp = runner.place_on_field(1, "OPP", Some(0));
    (runner, koku, opp)
}

// ─── Combo M1: happy path ────────────────────────────────────────────────────

/// With ≤4 memory and Gatomon (a TS-trait Lv.4) in hand, Kokuwamon's
/// `[Start of Your Main Phase]` clause free-digivolves itself into Gatomon, and
/// the effect-initiated digivolve fires Gatomon's `[When Digivolving]`, debuffing
/// the opponent Digimon by −3000 DP for the turn.
#[test]
fn kokuwamon_free_digivolve_into_gatomon_fires_when_digivolving_debuff() {
    let (mut runner, koku, opp) = combo_board(GATOMON, 3, 9000);

    let memory_before = runner.memory();
    let hand_before = runner.hand_size(0);
    let opp_dp_before = runner.effective_dp(opp);
    assert_eq!(opp_dp_before, Some(9000), "opponent starts at 9000 DP");

    // Enter main phase → Kokuwamon's optional SOMP prompt installs ("may").
    runner.game.enter_main_phase();
    assert!(
        runner.pending_selection().is_some(),
        "Kokuwamon's [Start of Your Main Phase] clause must install the printed 'may' prompt"
    );

    // Accept the optional trigger, then pick Gatomon from hand.
    runner
        .accept_optional_trigger()
        .expect("accept the optional SOMP trigger");
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Hand),
        "after accepting, the select_hand evolution picker must be pending"
    );
    // Gatomon is the only [Machine]/[Cyborg]/[TS] Digimon in hand → resolve it.
    let pick = runner.pending_selection_view().expect("hand picker view");
    let pick_action = pick
        .valid_action_ids
        .iter()
        .copied()
        .find(|a| *a != digimon_engine::action::space::PASS)
        .expect("Gatomon must be an eligible evolution target");
    runner
        .execute_action(pick.selecting_player, pick_action)
        .expect("pick Gatomon as the free-digivolve evolution");

    // The effect-initiated digivolve fired Gatomon's [When Digivolving], which
    // installs the opponent-Digimon −3000 DP selection.
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::OppField),
        "Gatomon's [When Digivolving] must fire off the effect-initiated digivolve and \
         install the opponent-Digimon debuff selection"
    );
    let debuff = runner
        .pending_selection_view()
        .expect("opponent-Digimon debuff selection");
    let debuff_action = debuff
        .valid_action_ids
        .iter()
        .copied()
        .find(|a| *a != digimon_engine::action::space::PASS)
        .expect("the opponent Digimon must be offered for the −3000 DP debuff");
    runner
        .execute_action(debuff.selecting_player, debuff_action)
        .expect("select the opponent Digimon for −3000 DP");
    let _ = runner.auto_resolve();

    // ── Claimed mechanical outcome ───────────────────────────────────────────
    // 1. Kokuwamon became Gatomon (stack [Kokuwamon, Gatomon]).
    let perm = &runner.game.players[0].battle_area[koku.index as usize];
    assert_eq!(
        perm.top_card().card_id(&runner.game.card_data),
        GATOMON,
        "Kokuwamon must have digivolved into Gatomon"
    );
    assert!(
        perm.card_sources.len() >= 2,
        "the stack must hold Kokuwamon under the Gatomon result"
    );
    // 2. It was free: no memory was paid for the digivolve (the only memory
    //    delta would come from elsewhere; the SOMP clause is cost-0).
    assert_eq!(
        runner.memory(),
        memory_before,
        "the free-digivolve must not pay memory (printed 'without paying the cost')"
    );
    // 3. Gatomon left hand to become the new top card.
    assert_eq!(
        runner.hand_size(0),
        hand_before - 1,
        "Gatomon left hand to become Kokuwamon's evolution"
    );
    // 4. The payoff: the opponent Digimon is −3000 DP for the turn (9000 → 6000).
    assert_eq!(
        runner.effective_dp(opp),
        Some(6000),
        "Gatomon's [When Digivolving] debuff must reduce the opponent Digimon by 3000 DP"
    );
}

// ─── Combo M1: unhappy path — enabler reaches no valid partner ────────────────

/// The SAME board but the only hand card is a Lv.4 **Beast** (not
/// [Machine]/[Cyborg]/[TS]): Kokuwamon's `select_hand` picker has no eligible
/// evolution, so the SOMP clause never fires — no digivolve, no `[When
/// Digivolving]`, and the opponent Digimon keeps its full DP. The payoff is
/// gated on the enabler reaching a *valid partner*, which a per-card test of
/// either card cannot express.
#[test]
fn kokuwamon_without_eligible_partner_produces_no_debuff() {
    let (mut runner, koku, opp) = combo_board("BEAST-LV4", 3, 9000);

    let opp_dp_before = runner.effective_dp(opp);

    runner.game.enter_main_phase();
    assert!(
        runner.pending_selection().is_none(),
        "no [Machine]/[Cyborg]/[TS] Digimon in hand → Kokuwamon's SOMP clause must not fire"
    );

    // Kokuwamon is untouched and the opponent Digimon is undamaged: the whole
    // combo is gated out when the enabler cannot reach a valid partner.
    let perm = &runner.game.players[0].battle_area[koku.index as usize];
    assert_eq!(
        perm.top_card().card_id(&runner.game.card_data),
        KOKUWAMON,
        "with no eligible hand partner Kokuwamon stays Kokuwamon"
    );
    assert_eq!(
        runner.effective_dp(opp),
        opp_dp_before,
        "no Gatomon digivolve → no [When Digivolving] → opponent Digimon keeps full DP"
    );
}

// ─── Combo M1: memory-gate guard ─────────────────────────────────────────────

/// Even with Gatomon in hand, 5+ memory gates Kokuwamon's clause out entirely —
/// so the combo never starts and the opponent Digimon is never debuffed. This
/// pins that the combo's *entry condition* (≤4 memory) governs the whole chain,
/// not just the digivolve.
#[test]
fn kokuwamon_combo_gated_out_above_four_memory() {
    let (mut runner, _koku, opp) = combo_board(GATOMON, 5, 9000);

    runner.game.enter_main_phase();
    assert!(
        runner.pending_selection().is_none(),
        "memory > 4 → Kokuwamon's SOMP clause is gated out, so the combo never begins"
    );
    assert_eq!(
        runner.effective_dp(opp),
        Some(9000),
        "no digivolve at 5 memory → no Gatomon debuff"
    );
}
