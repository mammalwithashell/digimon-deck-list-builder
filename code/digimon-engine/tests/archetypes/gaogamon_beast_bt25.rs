//! Gaogamon / Beast (BT25 slice) — archetype interaction tests.
//!
//! Model: `qa/archetype-qa/gaogamon-beast-bt25-model.md`. Slice cards (all
//! IMPLEMENTED — YAML in `code/digimon-engine/cards/bt25/` + per-card tests in
//! `cards_behavioral/bt25/` green):
//!   BT25-008 Coronamon, BT25-009 Bearmon (Red), BT25-048 Bearmon (Green),
//!   BT25-012 Grizzlymon (Red/Green), BT25-013 Firamon (Red),
//!   BT25-051 Grizzlymon (Green/Yellow), BT25-021 Gaomon, BT25-023 Gaogamon.
//!
//! Two co-resident beast engines: the Red/off-colour [Iliad]/[TS] line (free /
//! discounted digivolution gated on the **[TS]** trait, +3000 DP / Raid combat
//! pumps, Iliad recursion) and the Blue [DATA SQUAD] "Gaogamon" line (reveal-3
//! dig, free Thomas H. Norstein tamer ramp). These tests pin the **cross-card /
//! cross-clause chains** those engines form — chains the per-card tests
//! (`cards_behavioral/bt25/bt25_0{08,09,12,13,21,23,48,51}.rs`) deliberately
//! exercise in isolation and therefore never see.
//!
//! Per Phase-4 precondition gating, every authored combo names only implemented
//! slice cards. Cross-set evolution prerequisites are supplied as **synthesized
//! DebugRunner fixtures** (neutral cards carrying only the level / colour / trait
//! / evo-cost a combo needs). A cross-set card's *real* implementation is pulled
//! in ONLY if a combo fires its actual printed effect — none of these combos
//! does (the Thomas H. Norstein free-played in C9 and the Flaremon digivolved
//! into in C6 are synthetic name-only targets whose own printed effects never
//! resolve), so no cross-set implementation is pulled (lazy closure).
//!
//! Source priority (CLAUDE.md): printed card face + DCGO C#
//! (`$BASE_DCGO/Assets/Scripts/CardEffect/BT25/<Colour>/BT25_0xx.cs`) outrank
//! cards.json; each `code/digimon-engine/cards/bt25/BT25-0xx.yaml` header carries
//! the per-card DCGO crosscheck each combo below relies on. `general_rule.pdf`
//! §16 governs digivolution / Raid / cost timing.
//!
//! Combos authored (ranked by play-frequency × payoff-centrality):
//!   C1 — Bearmon (BT25-009) SOMP free-digivolve INTO the real Grizzlymon
//!        (BT25-012) payoff: one trigger (SOMP) chains into the payoff's
//!        [When Digivolving] Raid+3000 grant. The Red line's engine→payoff bridge.
//!   C2 — Bearmon SOMP negative gating (>4 memory / no eligible hand target).
//!   C3 — BT25-048 Bearmon's [Your Turn] −1 cost is scoped to ITSELF as the
//!        digivolve base: a [TS] result off a *different* base pays full cost.
//!   C4 — BT25-012 Grizzlymon pump grants Raid + a +3000 DP window that expires
//!        end-of-turn.
//!   C5 — BT25-051 Grizzlymon pump grants +3000 DP that PERSISTS into the
//!        opponent's turn (the defensive mirror of C4, distinguished by expiry).
//!   C6 — BT25-013 Firamon's [Your Turn] blue-entry observer arms a free(-1)
//!        digivolve into [Flaremon]; a non-blue entry does NOT (colour gate).
//!   C7 — BT25-013 Firamon's [On Play] trash-1 recovers a red/blue [Iliad]
//!        Digimon from trash; a green/non-Iliad trash card is not a legal target.
//!   C9 — BT25-023 Gaogamon free-plays a Thomas H. Norstein when the controller
//!        holds ≤1 Tamers; with ≥2 Tamers already down the ramp does NOT offer.

#![allow(dead_code)]

use digimon_dsl::compiled::{CompiledClause, CompiledDeclarativeClause, CompiledTiming};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, Keyword, PlaySource};
use digimon_engine::permanent::PermanentHandle;

use super::support::snapshot;

const CORONAMON: &str = "BT25-008";
const BEARMON_RED: &str = "BT25-009";
const BEARMON_GREEN: &str = "BT25-048";
const GRIZZLYMON_RG: &str = "BT25-012";
const FIRAMON: &str = "BT25-013";
const GRIZZLYMON_GY: &str = "BT25-051";
const GAOMON: &str = "BT25-021";
const GAOGAMON: &str = "BT25-023";

// ─── Synthesized combo-piece fixtures ────────────────────────────────────────

/// A neutral [Beast] Lv.4 own Digimon — a legal Grizzlymon-grant target / a
/// digivolve target for the SOMP/cost-reduction combos. `traits`/`colors`/`evo`
/// are tuned per call site; only its level/trait/colour/cost matter (no printed
/// effect).
fn beast_lv4(id: &str, name: &str, color: CardColor, dp: i32) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(dp);
    c.colors = vec![color];
    c.traits = vec!["Beast".to_string()];
    c
}

/// A Lv.4 Digimon carrying the given traits, digivolvable from a Lv.3 base for
/// the given printed memory cost — the cost-reduction / digivolve target.
fn lv4_target(id: &str, name: &str, color: CardColor, traits: &[&str], cost: u8) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(5000);
    c.colors = vec![color];
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c.evo_costs = vec![EvoCost {
        card_color: color as u8,
        level: 3,
        memory_cost: cost as u16,
    }];
    c
}

/// A neutral Lv.3 Digimon with chosen colour/traits — a digivolve *base* for the
/// C3 cross-base control (a non-BT25-048 base that should NOT get the −1).
fn lv3_base(id: &str, name: &str, color: CardColor, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(3000);
    c.colors = vec![color];
    c.play_cost = 3;
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

/// A neutral [Iliad] Digimon in the given colour — the C7 trash-recovery target.
fn iliad_digimon(id: &str, name: &str, color: CardColor) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(5000);
    c.colors = vec![color];
    c.traits = vec!["Iliad".to_string(), "Beast".to_string()];
    c
}

/// A synthetic [Flaremon]-named Lv.5 — the C6 digivolve target. NAME-ONLY: its
/// own printed effect is never fired (the combo only needs Firamon's observer to
/// OFFER the digivolve into a hand card named "Flaremon"), so no cross-set
/// implementation is pulled.
fn flaremon_stub(id: &str) -> CardData {
    let mut c = make_test_card(id, "Flaremon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(9000);
    c.colors = vec![CardColor::Red];
    c.evo_costs = vec![EvoCost {
        card_color: 0,
        level: 4,
        memory_cost: 3,
    }];
    c
}

/// A synthetic Thomas H. Norstein Tamer — the C9 free-play target. NAME-ONLY:
/// BT25-023's clause only matches `name_is: "Thomas H. Norstein"` and plays it
/// for free; Thomas's own printed effect never resolves, so the real card
/// (BT25-087 / BT4-093) is NOT pulled (lazy closure).
fn thomas_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, "Thomas H. Norstein");
    c.card_kind = CardKind::Tamer;
    c.colors = vec![CardColor::Blue];
    c.play_cost = 3;
    c.traits = vec!["DATA SQUAD".to_string()];
    c
}

/// A neutral non-Thomas Tamer to pre-load the field for C9's "≥2 Tamers" gate.
fn neutral_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.colors = vec![CardColor::Blue];
    c.play_cost = 3;
    c
}

/// A neutral blue Lv.4 own Digimon — the C6 blue-entry trigger.
fn blue_lv4(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.colors = vec![CardColor::Blue];
    c.traits = vec!["Beast".to_string()];
    c
}

/// A neutral red Lv.4 own Digimon — the C6 non-blue control.
fn red_lv4(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.colors = vec![CardColor::Red];
    c.traits = vec!["Beast".to_string()];
    c
}

fn push_to_hand(runner: &mut DebugRunner, player: u8, card_id: &str) -> usize {
    let data_index = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card id {card_id}"));
    let iid = runner.game.next_card_index();
    runner.game.players[player as usize]
        .hand
        .push(CardSource::new(data_index, player, iid));
    runner.game.players[player as usize].hand.len() - 1
}

fn push_to_trash(runner: &mut DebugRunner, player: u8, card_id: &str) {
    let data_index = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card id {card_id}"));
    let iid = runner.game.next_card_index();
    runner.game.players[player as usize]
        .trash
        .push(CardSource::new(data_index, player, iid));
}

fn hand_pos(runner: &DebugRunner, player: u8, card_id: &str) -> usize {
    runner.game.players[player as usize]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} in P{player} hand"))
}

/// Count own-field Tamers for player 0 (the C9 gate observable).
fn tamer_count(runner: &DebugRunner, player: u8) -> usize {
    runner.game.players[player as usize]
        .battle_area
        .iter()
        .filter(|p| p.top_card().card_kind(&runner.game.card_data) == CardKind::Tamer)
        .count()
}

// ══════════════════════════════════════════════════════════════════════════
// Combo C1 — Bearmon (BT25-009) SOMP free-digivolve INTO the real Grizzlymon
//            (BT25-012) payoff → the payoff's [When Digivolving] grant chains
// ══════════════════════════════════════════════════════════════════════════
//
// BT25-009 [Start of Your Main Phase] (≤4 memory): may free-digivolve self into
// a [Beast]/[Animal]/[Sovereign] (not [Sea Animal]) or [TS] Digimon from hand.
// BT25-012 Grizzlymon is a real [Beast] Lv.4 → a legal SOMP target. Because the
// SOMP performs a *digivolution*, Grizzlymon's [On Play][When Digivolving] grant
// fires off the SAME event — installing a Raid+3000 target selection. One
// trigger (SOMP) chains directly into the payoff's trigger: the engine→payoff
// bridge no per-card test stages (bt25_009 evolves into a synthetic BEAST-LV4
// with no [When Digivolving]; bt25_012 is played from hand, never reached via a
// SOMP free-digivolve).
//
// Sources: BT25-009.yaml (SOMP optional, memory_lte:4, effect_initiated_digivolve
// cost 0) + DCGO BT25_009.cs; BT25-012.yaml (when:[on_play, when_digivolving]
// grant Raid+3000) + DCGO BT25_012.cs. general_rule.pdf §16 digivolve event.

/// Happy path: SOMP free-digivolves Bearmon into Grizzlymon (memory unchanged),
/// and Grizzlymon's [When Digivolving] grant immediately installs its
/// Raid+3000 selection — the chained payoff trigger.
#[test]
fn c1_bearmon_somp_into_grizzlymon_chains_the_raid_grant() {
    let mut runner = DebugRunner::builder()
        .dsl_card(BEARMON_RED)
        .expect("BT25-009 (Bearmon) in embedded DSL pack")
        .dsl_card(GRIZZLYMON_RG)
        .expect("BT25-012 (Grizzlymon) in embedded DSL pack")
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .memory(3) // ≤4 → SOMP gate open
        .start();
    runner.game.turn_player_idx = 0;

    // Bearmon on field; Grizzlymon (the real payoff) waiting in hand as the
    // free-digivolve target.
    let bear = runner.place_on_field(0, BEARMON_RED, Some(0));
    let _griz_in_hand = push_to_hand(&mut runner, 0, GRIZZLYMON_RG);

    let memory_before = runner.memory();
    let stack_before = runner.game.players[0].battle_area[bear.index as usize]
        .card_sources
        .len();

    runner.game.enter_main_phase();
    assert!(
        runner.pending_selection().is_some(),
        "the SOMP 'may' free-digivolve must install a prompt (≤4 memory, Grizzlymon eligible in hand)"
    );
    runner
        .accept_optional_trigger()
        .expect("accept the optional SOMP free-digivolve");
    // Resolve the hand-card pick → the digivolution completes; Grizzlymon's
    // [When Digivolving] grant then parks its OWN Raid+3000 target selection.
    runner.auto_resolve().ok();

    // Bearmon is now buried under Grizzlymon (the SOMP digivolved into it).
    let perm = &runner.game.players[0].battle_area[bear.index as usize];
    assert_eq!(
        perm.top_card().card_id(&runner.game.card_data),
        GRIZZLYMON_RG,
        "Bearmon's SOMP must digivolve it into Grizzlymon from hand"
    );
    assert!(
        perm.card_sources.len() > stack_before,
        "the stack grows — Bearmon is now a source beneath Grizzlymon"
    );
    // The free-digivolve paid no memory (printed 'without paying the cost').
    assert_eq!(
        runner.memory(),
        memory_before,
        "BT25-009's SOMP digivolve is free — memory unchanged by the chain"
    );
    // Grizzlymon's grant fired off the WhenDigivolving event: the buffed beast
    // (Grizzlymon itself is a legal [Beast] target) carries Raid.
    assert!(
        runner.game.has_keyword(bear, Keyword::Raid),
        "Grizzlymon's [When Digivolving] grant must have granted Raid via the SOMP-driven digivolve"
    );
}

// ══════════════════════════════════════════════════════════════════════════
// Combo C2 — Bearmon SOMP negative gating (the unhappy path proving C1's gate)
// ══════════════════════════════════════════════════════════════════════════
//
// The SOMP engine only arms when BOTH gates hold: ≤4 memory AND a legal hand
// target. A per-card test pins each gate from a bare board; this combo proves
// the system fact that drives C1 — with the real Grizzlymon in hand but 5
// memory, the engine→payoff bridge stays closed (no chain at all).

/// With 5 memory the SOMP must not fire even though the real Grizzlymon payoff
/// is in hand — the memory gate alone closes the C1 bridge.
#[test]
fn c2_bearmon_somp_gated_out_above_four_memory() {
    let mut runner = DebugRunner::builder()
        .dsl_card(BEARMON_RED)
        .expect("BT25-009 (Bearmon) in embedded DSL pack")
        .dsl_card(GRIZZLYMON_RG)
        .expect("BT25-012 (Grizzlymon) in embedded DSL pack")
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .memory(5) // >4 → SOMP gate closed
        .start();
    runner.game.turn_player_idx = 0;

    let _bear = runner.place_on_field(0, BEARMON_RED, Some(0));
    let _griz_in_hand = push_to_hand(&mut runner, 0, GRIZZLYMON_RG);

    runner.game.enter_main_phase();
    assert!(
        runner.pending_selection().is_none(),
        "memory 5 (>4) must close the SOMP gate — no free-digivolve into the payoff is offered"
    );
}

// ══════════════════════════════════════════════════════════════════════════
// Combo C3 — BT25-048 Bearmon's [Your Turn] −1 cost is scoped to ITSELF as base
// ══════════════════════════════════════════════════════════════════════════
//
// BT25-048 [Your Turn]: "When THIS Digimon would digivolve into a [TS] trait
// Digimon card, reduce the cost by 1." The reduction is gated on
// `source_is_cost_target_permanent` — it fires only when BT25-048 itself is the
// digivolve base. The per-card test pins the −1 (TS) vs full-cost (non-TS) split
// from the SAME base; the cross-card system fact is the *base scoping*: a [TS]
// result digivolved off a DIFFERENT base (with BT25-048 idle elsewhere on field)
// must pay FULL cost. This proves the reducer doesn't leak across permanents.
//
// Sources: BT25-048.yaml (cost_reduction, when_any_ally_digivolves_into:{TS},
// condition:{source_is_cost_target_permanent}) + DCGO BT25_048.cs
// ChangeDigivolutionCostStaticEffect(-1, permanentCondition: targetPermanent ==
// this). general_rule.pdf §16 digivolve cost timing.

/// Scoped: BT25-048 as the base → a [TS] result costs printed−1; the SAME [TS]
/// result off a different base (BT25-048 idle on field) costs FULL printed cost.
#[test]
fn c3_bt25048_cost_reduction_only_when_it_is_the_base() {
    let ts_target = lv4_target("TS-LV4", "TsTarget", CardColor::Green, &["TS", "Beast"], 2);
    let other_base = lv3_base("OTHER-BASE", "OtherBase", CardColor::Green, &["Beast"]);

    // ── Leg A: BT25-048 IS the base → reduction applies (printed 2 → 1). ──────
    let mut runner = DebugRunner::builder()
        .dsl_card(BEARMON_GREEN)
        .expect("BT25-048 (Bearmon) in embedded DSL pack")
        .add_card(ts_target.clone())
        .add_card(other_base.clone())
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .hand(0, &["TS-LV4"])
        .memory(10)
        .start();
    runner.game.turn_player_idx = 0;
    let bear = runner.place_on_field(0, BEARMON_GREEN, Some(0));

    let mem_before = runner.memory();
    let h = hand_pos(&runner, 0, "TS-LV4");
    let ok = runner
        .game
        .digivolve_from_hand(0, h, bear.index as usize, PlaySource::ByDigivolve);
    assert!(ok, "digivolve off BT25-048 must succeed");
    assert_eq!(
        runner.memory(),
        mem_before - 1,
        "BT25-048 as the base reduces the [TS] digivolve from 2 to 1"
    );

    // ── Leg B: a DIFFERENT base, BT25-048 idle elsewhere → NO reduction. ──────
    let mut runner = DebugRunner::builder()
        .dsl_card(BEARMON_GREEN)
        .expect("BT25-048 (Bearmon) in embedded DSL pack")
        .add_card(ts_target)
        .add_card(other_base)
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .hand(0, &["TS-LV4"])
        .memory(10)
        .start();
    runner.game.turn_player_idx = 0;
    // BT25-048 sits idle on the field; OTHER-BASE is the digivolve base.
    let _idle_bear = runner.place_on_field(0, BEARMON_GREEN, Some(0));
    let other = runner.place_on_field(0, "OTHER-BASE", Some(0));

    let mem_before = runner.memory();
    let h = hand_pos(&runner, 0, "TS-LV4");
    let ok = runner
        .game
        .digivolve_from_hand(0, h, other.index as usize, PlaySource::ByDigivolve);
    assert!(ok, "digivolve off OTHER-BASE must succeed");
    assert_eq!(
        runner.memory(),
        mem_before - 2,
        "with BT25-048 NOT the base, the [TS] digivolve pays full cost 2 — the reducer is base-scoped"
    );
}

// ══════════════════════════════════════════════════════════════════════════
// Combo C4 — BT25-012 Grizzlymon pump: Raid + a +3000 DP window (end-of-turn)
// ══════════════════════════════════════════════════════════════════════════
//
// BT25-012 [On Play][When Digivolving] grants a chosen beast ally <Raid> and
// +3000 DP *for the turn*. The cross-card combo: the pump lands on a separate
// beast ally and lifts its DP by exactly 3000 for the turn, then expires at end
// of turn. (Per-card bt25_012 asserts the grant lands on *some* eligible target;
// this stages a dedicated ally so the +DP swing + expiry is unambiguous, and
// contrasts with C5's cross-turn expiry.)
//
// Sources: BT25-012.yaml (grant_keyword Raid end_of_turn + add_dp_modifier 3000
// end_of_turn) + DCGO BT25_012.cs. general_rule.pdf §16 turn-scoped modifiers.

/// The pump grants Raid + exactly +3000 DP to a chosen beast ally, and the +3000
/// expires when the turn ends.
#[test]
fn c4_grizzlymon_rg_pump_grants_raid_and_3000_until_end_of_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card(GRIZZLYMON_RG)
        .expect("BT25-012 (Grizzlymon) in embedded DSL pack")
        .add_card(beast_lv4("BEAST-ALLY", "BeastAlly", CardColor::Red, 5000))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .memory(10)
        .start();
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 0;

    // A dedicated beast ally to receive the pump, plus Grizzlymon played from hand.
    let ally = runner.place_on_field(0, "BEAST-ALLY", Some(0));
    let dp_before = runner.effective_dp(ally).expect("ally dp");

    let hand_index = push_to_hand(&mut runner, 0, GRIZZLYMON_RG);
    runner.play(0, hand_index);
    assert!(
        runner.pending_selection().is_some(),
        "the [On Play] grant must install a mandatory beast-target selection"
    );
    // Steer the grant onto the dedicated ally.
    drive_grant_onto(&mut runner, ally);
    runner.auto_resolve().ok();

    assert!(
        runner.game.has_keyword(ally, Keyword::Raid),
        "the selected beast ally must gain <Raid>"
    );
    assert_eq!(
        runner.effective_dp(ally),
        Some(dp_before + 3000),
        "the selected beast ally must gain exactly +3000 DP"
    );

    // 'for the turn' → expires when this turn ends.
    runner.end_turn();
    assert_eq!(
        runner.effective_dp(ally),
        Some(dp_before),
        "BT25-012's +3000 DP is turn-scoped — it expires at end of turn"
    );
}

// ══════════════════════════════════════════════════════════════════════════
// Combo C5 — BT25-051 Grizzlymon pump: +3000 DP PERSISTS into the opponent turn
// ══════════════════════════════════════════════════════════════════════════
//
// BT25-051 (the other Grizzlymon printing) grants +3000 DP *until your
// opponent's turn ends* — strictly longer than C4's end-of-turn window. The
// system fact distinguishing the two printings: 051's buff SURVIVES the turn
// boundary (still +3000 on the opponent's turn), where 012's is already gone.
// 051 also self-grants <Blocker>. This is the defensive mirror of C4's offensive
// Raid pump.
//
// Sources: BT25-051.yaml (add_dp_modifier 3000 expiry: end_of_opponents_turn;
// self grant_keyword Blocker) + DCGO BT25_051.cs ChangeDigimonDP
// UntilOpponentTurnEnd. general_rule.pdf §16 cross-turn modifier expiry.

/// 051's pump persists into the opponent's turn (unlike 012's), and 051 enters
/// with <Blocker>.
#[test]
fn c5_grizzlymon_gy_pump_persists_into_opponent_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card(GRIZZLYMON_GY)
        .expect("BT25-051 (Grizzlymon) in embedded DSL pack")
        .add_card(beast_lv4("BEAST-ALLY", "BeastAlly", CardColor::Green, 5000))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .memory(10)
        .start();
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 0;

    let ally = runner.place_on_field(0, "BEAST-ALLY", Some(0));
    let dp_before = runner.effective_dp(ally).expect("ally dp");

    let hand_index = push_to_hand(&mut runner, 0, GRIZZLYMON_GY);
    runner.play(0, hand_index);
    assert!(
        runner.pending_selection().is_some(),
        "BT25-051's [On Play] grant must install a mandatory beast-target selection"
    );
    drive_grant_onto(&mut runner, ally);
    runner.auto_resolve().ok();

    // BT25-051 self-grants <Blocker>.
    let grizzly_idx = runner.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&runner.game.card_data) == GRIZZLYMON_GY)
        .expect("BT25-051 on field");
    let grizzly = PermanentHandle {
        player: 0,
        index: grizzly_idx as u8,
    };
    assert!(
        runner.game.has_keyword(grizzly, Keyword::Blocker),
        "BT25-051 must self-grant <Blocker>"
    );

    assert_eq!(
        runner.effective_dp(ally),
        Some(dp_before + 3000),
        "the selected beast ally gains +3000 DP"
    );

    // Cross-turn persistence: still +3000 on the opponent's turn (where C4's
    // identical-looking pump would already have expired).
    runner.end_turn();
    assert_eq!(runner.turn_player(), 1, "it is now the opponent's turn");
    assert_eq!(
        runner.effective_dp(ally),
        Some(dp_before + 3000),
        "BT25-051's +3000 DP persists 'until your opponent's turn ends' — still active on the opponent's turn"
    );
}

// ══════════════════════════════════════════════════════════════════════════
// Combo C6 — BT25-013 Firamon's [Your Turn] blue-entry observer → free(-1)
//            digivolve into [Flaremon]
// ══════════════════════════════════════════════════════════════════════════
//
// BT25-013 [Your Turn]: "When your Digimon are played or digivolve, if any of
// them are blue, this Digimon may digivolve into [Flaremon] in the hand with the
// cost reduced by 1." Firamon watches OTHER cards entering: a BLUE entry arms
// the offer, a non-blue entry does not. Cross-card — one card's entry fires
// Firamon's digivolve observer (the Beastkin model's F2 from the Flaremon side;
// here Firamon is the observer). bt25_013 plays a synthetic blue Lv.4 in
// isolation; it never contrasts the colour gate as the deciding cross-card fact.
//
// Sources: BT25-013.yaml clause-1 (when:[on_enter_field_anyone,on_digivolve],
// condition:{event_target_owner:you, event_card_color_has:[blue]}; select_hand
// name "Flaremon" → effect_initiated_digivolve cost:{reduce:1}) + DCGO
// BT25_013.cs OnEnterFieldAnyone blue gate. general_rule.pdf §16 play-vs-digivolve event.

/// Happy path: playing a BLUE Digimon on your turn arms Firamon's observer,
/// which offers the digivolve into the hand [Flaremon].
#[test]
fn c6_firamon_blue_entry_arms_the_flaremon_offer() {
    let mut runner = DebugRunner::builder()
        .dsl_card(FIRAMON)
        .expect("BT25-013 (Firamon) in embedded DSL pack")
        .add_card(blue_lv4("BLUE-LV4"))
        .add_card(flaremon_stub("FLAREMON-STUB"))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .memory(10)
        .start();
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 0;

    let _firamon = runner.place_on_field(0, FIRAMON, Some(0));
    let _flaremon_in_hand = push_to_hand(&mut runner, 0, "FLAREMON-STUB");

    // Play a BLUE Digimon → a 'your Digimon are played, any of them blue' event.
    let blue_idx = push_to_hand(&mut runner, 0, "BLUE-LV4");
    runner.play(0, blue_idx);

    assert!(
        runner.pending_kind().is_some(),
        "playing a blue Digimon on your turn must arm Firamon's [Your Turn] digivolve into [Flaremon]"
    );
    runner.auto_resolve().ok();
}

/// Unhappy path: the SAME board but a RED entry — Firamon's gate is 'if any of
/// them are blue', so a non-blue entry must NOT arm the offer, even with the
/// [Flaremon] live in hand. Proves the blue colour gate (not hand-availability)
/// governs the chain.
#[test]
fn c6_firamon_red_entry_does_not_arm_the_offer() {
    let mut runner = DebugRunner::builder()
        .dsl_card(FIRAMON)
        .expect("BT25-013 (Firamon) in embedded DSL pack")
        .add_card(red_lv4("RED-LV4"))
        .add_card(flaremon_stub("FLAREMON-STUB"))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .memory(10)
        .start();
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 0;

    let _firamon = runner.place_on_field(0, FIRAMON, Some(0));
    let _flaremon_in_hand = push_to_hand(&mut runner, 0, "FLAREMON-STUB");

    let red_idx = push_to_hand(&mut runner, 0, "RED-LV4");
    runner.play(0, red_idx);

    assert!(
        runner.pending_selection().is_none(),
        "a non-blue entry must NOT arm Firamon's blue-gated [Your Turn] digivolve into [Flaremon]"
    );
    runner.auto_resolve().ok();
}

// ══════════════════════════════════════════════════════════════════════════
// Combo C7 — BT25-013 Firamon's [On Play] trash-1 → recover a red/blue [Iliad]
// ══════════════════════════════════════════════════════════════════════════
//
// BT25-013 [On Play][When Digivolving]: "By trashing 1 card in your hand, you
// may return 1 red or blue [Iliad] trait Digimon card from your trash to the
// hand." The recursion engine. The recovery target is gated on BOTH a colour
// (red/blue) AND the [Iliad] trait — a green [Iliad] or a red non-Iliad in trash
// is NOT recoverable. We seed exactly one legal target and assert the
// hand/trash net (trash the cost card, gain the recovered card → both net 0),
// then prove the gate by seeding only an ineligible (green) [Iliad] and showing
// the clause cannot complete a recovery.
//
// Sources: BT25-013.yaml clause-0 (select_hand cost:true → trash → select_trash
// {Iliad, red|blue} → add_to_hand_from_trash) + DCGO BT25_013.cs. general_rule.pdf
// §16 trash-recovery timing.

/// Happy path: a red [Iliad] sits in trash; play Firamon, trash a hand card,
/// recover the red [Iliad]. Hand net 0, trash net 0 (swap cost-card ⇄ recovery).
#[test]
fn c7_firamon_trash_one_recovers_red_iliad() {
    let mut runner = DebugRunner::builder()
        .dsl_card(FIRAMON)
        .expect("BT25-013 (Firamon) in embedded DSL pack")
        .add_card(iliad_digimon("RED-ILIAD", "RedIliad", CardColor::Red))
        .add_card(make_test_card("PITCH", "Pitch")) // the hand card to trash
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .memory(10)
        .start();
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 0;

    // One legal recovery target seeded in trash; a pitchable card in hand.
    push_to_trash(&mut runner, 0, "RED-ILIAD");
    let _pitch = push_to_hand(&mut runner, 0, "PITCH");

    let before = snapshot(&runner);
    let hand_index = push_to_hand(&mut runner, 0, FIRAMON);
    runner.play(0, hand_index);
    // The optional clause parks; accept and resolve the trash → recover steps.
    if runner.pending_selection().is_some() {
        let _ = runner.accept_optional_trigger();
    }
    runner.auto_resolve().ok();

    let after = snapshot(&runner);
    // Firamon left hand (played); the pitch card left hand (trashed) and the
    // recovered RED-ILIAD entered hand → relative to BEFORE (which excludes the
    // not-yet-pushed Firamon), hand size is unchanged (−PITCH +RED-ILIAD).
    assert_eq!(
        after.hand[0], before.hand[0],
        "trash 1 from hand (−1) + recover 1 to hand (+1) → hand net 0 (Firamon itself left hand on play)"
    );
    // RED-ILIAD is now in hand, no longer in trash.
    let hand_ids: Vec<String> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    assert!(
        hand_ids.iter().any(|id| id == "RED-ILIAD"),
        "the red [Iliad] Digimon must be recovered to hand"
    );
}

/// Unhappy path: only a GREEN [Iliad] sits in trash (wrong colour). After
/// trashing the cost card the optional trash-recovery has no legal target, so
/// the green [Iliad] stays in trash — the colour gate (red/blue only) holds.
#[test]
fn c7_firamon_cannot_recover_green_iliad() {
    let mut runner = DebugRunner::builder()
        .dsl_card(FIRAMON)
        .expect("BT25-013 (Firamon) in embedded DSL pack")
        .add_card(iliad_digimon("GREEN-ILIAD", "GreenIliad", CardColor::Green))
        .add_card(make_test_card("PITCH", "Pitch"))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .memory(10)
        .start();
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 0;

    push_to_trash(&mut runner, 0, "GREEN-ILIAD");
    let _pitch = push_to_hand(&mut runner, 0, "PITCH");

    let hand_index = push_to_hand(&mut runner, 0, FIRAMON);
    runner.play(0, hand_index);
    if runner.pending_selection().is_some() {
        let _ = runner.accept_optional_trigger();
    }
    runner.auto_resolve().ok();

    // GREEN-ILIAD must NOT have been recovered (wrong colour).
    let hand_ids: Vec<String> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    assert!(
        !hand_ids.iter().any(|id| id == "GREEN-ILIAD"),
        "a green [Iliad] is not a legal recovery target — only red/blue [Iliad] qualify"
    );
}

// ══════════════════════════════════════════════════════════════════════════
// Combo C9 — BT25-023 Gaogamon free-plays Thomas H. Norstein when Tamer-light
// ══════════════════════════════════════════════════════════════════════════
//
// BT25-023 [On Play][When Digivolving]: "If you have 1 or fewer Tamers, you may
// play 1 [Thomas H. Norstein] from your hand without paying the cost." The Blue
// line's signature ramp. The gate is the CONTROLLER's own Tamer count, not hand
// contents: with ≤1 Tamers the free play is offered (Tamer count rises, no
// memory paid); with ≥2 Tamers already down it is NOT offered even with Thomas
// in hand. Thomas's own printed effect never fires here, so a NAME-ONLY synthetic
// Tamer suffices (lazy closure — no cross-set pull).
//
// Sources: BT25-023.yaml (active_when:{count_lte:{tamer, owner:you}, n:1},
// optional:true → select_hand(name "Thomas H. Norstein") → play_from_hand_free)
// + DCGO BT25_023.cs PlayForFree gated on Tamer count ≤1. general_rule.pdf §16
// play-for-free timing.

/// Happy path: 0 Tamers on field, Thomas in hand → Gaogamon's [On Play] offers
/// the free play; accepting puts Thomas on the field with no memory paid.
#[test]
fn c9_gaogamon_free_plays_thomas_when_tamer_light() {
    let mut runner = DebugRunner::builder()
        .dsl_card(GAOGAMON)
        .expect("BT25-023 (Gaogamon) in embedded DSL pack")
        .add_card(thomas_tamer("THOMAS"))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .memory(10)
        .start();
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 0;

    let _thomas_in_hand = push_to_hand(&mut runner, 0, "THOMAS");
    assert_eq!(tamer_count(&runner, 0), 0, "no Tamers on field — gate open");

    let hand_index = push_to_hand(&mut runner, 0, GAOGAMON);
    runner.play(0, hand_index);

    assert!(
        runner.pending_selection().is_some(),
        "≤1 Tamers + Thomas in hand → Gaogamon must offer the free Thomas play"
    );
    // Isolate the Thomas play's cost: capture memory with the offer pending (the
    // Gaogamon play has already resolved), so the only memory delta across the
    // accept+resolve below is whatever the Thomas play itself costs.
    let mem_before_thomas = runner.memory();
    let _ = runner.accept_optional_trigger();
    runner.auto_resolve().ok();

    assert_eq!(
        tamer_count(&runner, 0),
        1,
        "Thomas H. Norstein must be free-played onto the field (Tamer count 0 → 1)"
    );
    // 'without paying the cost': accepting and resolving the Thomas play spends
    // NO memory — the delta across just the free-play step is exactly zero.
    assert_eq!(
        runner.memory(),
        mem_before_thomas,
        "the Thomas H. Norstein play is free — its printed cost (3) is not paid"
    );
}

/// Unhappy path: ≥2 Tamers already on field → the gate ('1 or fewer Tamers')
/// is closed, so the free Thomas play is NOT offered even with Thomas in hand.
/// Proves the ramp is gated on the controller's Tamer count, not hand contents.
#[test]
fn c9_gaogamon_does_not_offer_when_two_tamers_down() {
    let mut runner = DebugRunner::builder()
        .dsl_card(GAOGAMON)
        .expect("BT25-023 (Gaogamon) in embedded DSL pack")
        .add_card(thomas_tamer("THOMAS"))
        .add_card(neutral_tamer("TAMER-A"))
        .add_card(neutral_tamer("TAMER-B"))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .memory(10)
        .start();
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 0;

    // Two Tamers already down → gate closed.
    let _t1 = runner.place_on_field(0, "TAMER-A", None);
    let _t2 = runner.place_on_field(0, "TAMER-B", None);
    let _thomas_in_hand = push_to_hand(&mut runner, 0, "THOMAS");
    assert_eq!(
        tamer_count(&runner, 0),
        2,
        "two Tamers on field — gate closed"
    );

    let hand_index = push_to_hand(&mut runner, 0, GAOGAMON);
    runner.play(0, hand_index);

    assert!(
        runner.pending_selection().is_none(),
        ">1 Tamers must close the gate — no free Thomas play is offered despite Thomas in hand"
    );
}

// ─── Selection-driving helpers ───────────────────────────────────────────────

/// Drive a mandatory own-field beast-grant selection (BT25-012 / BT25-051) onto
/// the own-field permanent at `wanted.index`. Own-field target selections encode
/// each slot as `ATTACK_START + slot` (the `100 + slot` field-target encoding the
/// action mask uses); pick that candidate, falling back to the first valid action
/// if the engine pre-targeted.
fn drive_grant_onto(runner: &mut DebugRunner, wanted: PermanentHandle) {
    use digimon_engine::action::space::ATTACK_START;
    let Some(view) = runner.pending_selection_view() else {
        return;
    };
    let want = ATTACK_START + wanted.index as u16;
    let chosen = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a == want)
        .or_else(|| view.valid_action_ids.first().copied());
    if let Some(action) = chosen {
        let _ = runner.execute_action(view.selecting_player, action);
    }
}

// ─── Compile-time model anchors (cheap structural cross-checks) ───────────────
//
// These keep the model doc honest about the clause shapes the behavioral combos
// above rely on, mirroring the per-card structural sections without re-testing
// their behavior.

#[test]
fn anchor_bt25048_has_cost_reduction_clause() {
    let runner = DebugRunner::builder()
        .dsl_card(BEARMON_GREEN)
        .expect("BT25-048 in pack")
        .start();
    let card = runner.compiled_card(BEARMON_GREEN).expect("present");
    assert!(
        card.effects.iter().any(|c| matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction { .. })
        )),
        "C3 relies on BT25-048's CostReduction declarative"
    );
}

#[test]
fn anchor_bt25013_has_your_turn_digivolve_observer() {
    let runner = DebugRunner::builder()
        .dsl_card(FIRAMON)
        .expect("BT25-013 in pack")
        .start();
    let card = runner.compiled_card(FIRAMON).expect("present");
    assert!(
        card.effects.iter().any(|c| matches!(
            c,
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnEnterFieldAnyone)
                    || t.when.contains(&CompiledTiming::OnDigivolve)
        )),
        "C6 relies on BT25-013's [Your Turn] play/digivolve observer clause"
    );
}
