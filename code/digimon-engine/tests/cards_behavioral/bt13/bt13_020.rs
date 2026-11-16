//! BT13-020 ShineGreymon: Burst Mode — Digimon, Lv.7, Red/Yellow, DP 15000,
//! Cost 15, Light Dragon.
//!
//! # Card text (cards.json / BT13-020.json)
//!
//! `[When Digivolving] You may play 1 [Marcus Damon] from your hand without`
//! `paying its cost. For the turn, treat the Tamer played with this effect as`
//! `a 12000 DP Digimon with ＜Rush＞ (This Digimon can attack the turn it comes`
//! `into play.), and it cannot digivolve.`
//! `[Your Turn] [Once Per Turn] When one of your Tamers becomes suspended,`
//! `Trash 1 card from the top of your opponent's security stack.`
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT13/Red/BT13_020.cs
//!   - OnEnterFieldAnyone (When Digivolving): optional SelectHandEffect for a
//!     Marcus Damon (canNoSelect: true), play free, then
//!     BecomeDigimonThatCantDigivolve(DP: 12000, UntilEachTurnEnd) + GainRush.
//!   - OnTappedAnyone (own Tamer suspend, owner's turn, maxUsableCount: 1):
//!     IDestroySecurity(card.Owner.Enemy, count: 1, fromTop: true).
//!
//! # Patterns this test file covers
//! - C4/H12-adjacent: [Burst Digivolve] alt-path (kind:burst_digivolve, cost 0)
//!   with extra_cost (return 1 [Marcus Damon] to hand) + on_burst_turn_end
//!   (trash this Digimon's top card), plus the named-prior-form digivolve path.
//! - When-Digivolving optional free-play of a named Tamer from hand
//!   (`select_hand` optional + `play_from_hand_free` with `bind_as`).
//! - TreatAsDigimon synth-identity (12000 DP) + CannotDigivolve + granted Rush,
//!   all expiry end_of_turn, installed on the just-played Tamer.
//! - B3-adjacent: triggered observer of own-Tamer suspend events, [Your Turn]
//!   gated, [Once Per Turn] (G-OPT-TRIGGERED resolved — enforcement is live).
//! - Cost-firing: opponent security trash via event log.

#![allow(dead_code, unused_imports)]

#[path = "../../support/dsl_card_data.rs"]
mod dsl_card_data;

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, Expiry, Keyword, ModifierType, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::PendingSelection;
use digimon_engine::{EffectTiming, TriggerSource};

// ─── helpers ──────────────────────────────────────────────────────────────────

fn burst() -> digimon_dsl::compiled::CompiledCard {
    dsl_card_data::compiled("BT13-020")
}

fn make_tamer(card_id: &str, name: &str) -> CardData {
    let mut card = make_test_card(card_id, name);
    card.card_kind = CardKind::Tamer;
    card.colors = vec![CardColor::Red];
    card.dp = None;
    card.level = None;
    card
}

fn make_digimon(card_id: &str, dp: i32, color: CardColor) -> CardData {
    let mut card = make_test_card(card_id, card_id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![color];
    card.dp = Some(dp);
    card.level = Some(4);
    card
}

fn marcus_handle(runner: &DebugRunner) -> Option<PermanentHandle> {
    runner.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&runner.game.card_data) == "MARCUS")
        .map(|i| PermanentHandle {
            player: 0,
            index: i as u8,
        })
}

fn tamer_on_field(
    runner: &DebugRunner,
    player: PlayerId,
    card_id: &str,
) -> Option<PermanentHandle> {
    runner.game.players[player as usize]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&runner.game.card_data) == card_id)
        .map(|i| PermanentHandle {
            player,
            index: i as u8,
        })
}

fn player_of(p: &PendingSelection) -> PlayerId {
    p.selecting_player
}

fn first_action(p: &PendingSelection) -> u16 {
    *p.valid_action_ids
        .first()
        .expect("at least one legal action")
}

/// Place BT13-020 on the field (top card) and fire its own [When Digivolving]
/// clause, then drain the queue.
fn fire_when_digivolving(runner: &mut DebugRunner) -> PermanentHandle {
    let carrier = runner.place_on_field(0, "BT13-020", Some(0));
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(carrier),
    );
    runner.game.drain_effect_queue();
    carrier
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 1 — Structural assertions
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn bt13_020_metadata_matches_printed() {
    let card = burst();
    assert_eq!(card.card, "BT13-020");
    assert_eq!(card.name, "ShineGreymon: Burst Mode");
    assert_eq!(card.level, Some(7));
    assert_eq!(card.dp, Some(15000));
    assert_eq!(card.cost, Some(15));

    let colors: Vec<_> = card.color.iter().copied().collect();
    assert!(
        colors.contains(&digimon_dsl::compiled::CompiledColor::Red),
        "must be Red"
    );
    assert!(
        colors.contains(&digimon_dsl::compiled::CompiledColor::Yellow),
        "must be Yellow"
    );
    assert!(
        card.traits
            .iter()
            .any(|t| t.eq_ignore_ascii_case("Light Dragon")),
        "must have Light Dragon trait; got {:?}",
        card.traits
    );
}

#[test]
fn bt13_020_has_two_triggered_clauses() {
    let card = burst();
    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(
        triggered.len(),
        2,
        "[When Digivolving] + [Your Turn][OPT] on_suspend; got {:?}",
        triggered
            .iter()
            .map(|t| (t.scope, t.when.clone()))
            .collect::<Vec<_>>()
    );
}

#[test]
fn bt13_020_clause1_is_when_digivolving() {
    let card = burst();
    let has_wd = card.effects.iter().any(|c| match c {
        CompiledClause::Triggered(t) => t.when.contains(&CompiledTiming::WhenDigivolving),
        _ => false,
    });
    assert!(has_wd, "clause 1 must be [When Digivolving]");
}

#[test]
fn bt13_020_clause2_on_suspend_is_opt() {
    let card = burst();
    let suspend = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSuspend) => Some(t),
            _ => None,
        })
        .expect("OnSuspend clause must exist");
    assert!(suspend.once_per_turn, "[Once Per Turn] flag must be set");
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 1b — Alt-paths (named-prior-form digivolve + Burst Digivolve)
// ─────────────────────────────────────────────────────────────────────────────
//
// The compiled alt-path shape is pinned here; the burst digivolve itself (the
// Tamer return, cost 0 + draw, the turn-end top-card trash, the route choice
// and the 8-3-2-2 effect exclusion) is driven through the real action surface
// in `mod burst_digivolve` at the bottom of this file
// (`crate::burst_digivolve_common`). DCGO BT13_020.cs
// AddBurstDigivolutionConditionClass.

/// The printed cost-5 circles (Red / Yellow Lv.6) are authored as digivolve paths.
#[test]
fn bt13_020_has_named_lv6_digivolve_path_cost5() {
    let card = burst();
    let found = card.alt_paths.iter().any(|p| {
        p.kind == CompiledAltPathKind::Digivolve && p.cost == Some(CompiledCost::Literal(5))
    });
    assert!(
        found,
        "must declare a cost-5 Digivolve alt-path from the named Lv.6 ShineGreymon; \
         alt_paths={:?}",
        card.alt_paths
    );
}

/// The [Burst Digivolve] path: kind:burst_digivolve, cost 0, with both the
/// return-Marcus extra_cost and the trash-top-card on_burst_turn_end populated.
#[test]
fn bt13_020_has_burst_digivolve_path_cost0_with_extra_cost_and_teardown() {
    let card = burst();
    let burst_path = card
        .alt_paths
        .iter()
        .find(|p| p.kind == CompiledAltPathKind::BurstDigivolve)
        .unwrap_or_else(|| {
            panic!(
                "must declare a BurstDigivolve alt-path; alt_paths={:?}",
                card.alt_paths
            )
        });

    assert_eq!(
        burst_path.cost,
        Some(CompiledCost::Literal(0)),
        "[Burst Digivolve] cost must be 0 (DCGO BurstDigivolutionCondition cost: 0)"
    );
    assert!(
        !burst_path.extra_cost.is_empty(),
        "burst path must carry the 'By returning 1 [Marcus Damon] to hand' extra_cost"
    );
    assert!(
        !burst_path.on_burst_turn_end.is_empty(),
        "burst path must carry the 'trash this Digimon's top card at the end of the burst \
         digivolution turn' on_burst_turn_end teardown"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 2 — Clause 1 [When Digivolving]: optional free-play of Marcus Damon
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn bt13_020_when_digivolving_installs_optional_marcus_play() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT13-020")
        .expect("BT13-020 in pack")
        .add_card(make_tamer("MARCUS", "Marcus Damon"))
        .hand(0, &["MARCUS"])
        .memory(10)
        .start();

    fire_when_digivolving(&mut runner);

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("the [When Digivolving] play prompt must install");
    assert!(
        pending.is_optional,
        "'You may play' is optional — PASS must be legal"
    );
    assert_eq!(
        pending.valid_action_ids.len(),
        1,
        "exactly the Marcus Damon in hand is selectable; got {:?}",
        pending.valid_action_ids
    );
}

#[test]
fn bt13_020_when_digivolving_no_marcus_in_hand_installs_no_prompt() {
    // A non-Marcus Tamer in hand must NOT be offered (name filter).
    let mut runner = DebugRunner::builder()
        .dsl_card("BT13-020")
        .expect("BT13-020 in pack")
        .add_card(make_tamer("OTHER-TAMER", "Some Other Tamer"))
        .hand(0, &["OTHER-TAMER"])
        .memory(10)
        .start();

    fire_when_digivolving(&mut runner);

    assert!(
        runner.game.pending_selection.is_none(),
        "no Marcus Damon → no play prompt installs; got {:?}",
        runner.game.pending_selection
    );
}

// ─── Candidate legality: only a Marcus Damon the free play would accept ─────
// DCGO BT13_020.cs:111-122 `CanSelectCardCondition`: CardNames ∋ "Marcus
// Damon" && `CanPlayAsNewPermanent(cardSource, payCost:false, activateClass)`
// — no free battle-area frame or a "can't play Tamers by effects" lock removes
// the card from the pick. CR 4.3 1-3-11-1: only a card that can be played may
// be declared for play.

fn fire_with_marcus_after(setup: impl FnOnce(&mut DebugRunner)) -> DebugRunner {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT13-020")
        .expect("BT13-020 in pack")
        .add_card(make_tamer("MARCUS", "Marcus Damon"))
        .hand(0, &["MARCUS"])
        .memory(10)
        .start();
    let carrier = runner.place_on_field(0, "BT13-020", Some(0));
    setup(&mut runner);
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(carrier),
    );
    runner.game.drain_effect_queue();
    runner
}

fn non_pass_candidates(runner: &DebugRunner) -> Vec<u16> {
    runner
        .game
        .pending_selection
        .as_ref()
        .map(|p| p.valid_action_ids.iter().copied().filter(|&a| a != PASS).collect())
        .unwrap_or_default()
}

#[test]
fn bt13_020_marcus_not_offered_when_battle_area_full() {
    let runner = fire_with_marcus_after(|r| {
        r.game.rules.field_slots = r.game.players[0].battle_area.len() as u8;
    });
    assert!(
        non_pass_candidates(&runner).is_empty(),
        "no free slot → Marcus Damon can't be played, so not offered; got {:?}",
        runner.game.pending_selection
    );
    assert!(marcus_handle(&runner).is_none());
    assert_eq!(runner.game.players[0].hand.len(), 1, "Marcus stays in hand");
}

#[test]
fn bt13_020_marcus_not_offered_under_tamer_play_prohibition() {
    let runner = fire_with_marcus_after(|r| {
        r.game.modifiers.add_player_modifier(
            0,
            digimon_engine::modifiers::PlayerModifierEntry::simple(
                ModifierType::CannotPlayTamerByEffect,
                0,
                Expiry::Permanent,
                None,
                1,
            ),
        );
    });
    assert!(
        non_pass_candidates(&runner).is_empty(),
        "\"can't play Tamers by effects\" → not offered; got {:?}",
        runner.game.pending_selection
    );
    assert!(marcus_handle(&runner).is_none());
    assert_eq!(runner.game.players[0].hand.len(), 1, "Marcus stays in hand");
}

#[test]
fn bt13_020_when_digivolving_plays_marcus_and_grants_synth_identity() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT13-020")
        .expect("BT13-020 in pack")
        .add_card(make_tamer("MARCUS", "Marcus Damon"))
        .hand(0, &["MARCUS"])
        .memory(10)
        .start();

    let battle_before = runner.battle_area_size(0);
    fire_when_digivolving(&mut runner);

    let (player, action_id) = {
        let pending = runner
            .game
            .pending_selection
            .as_ref()
            .expect("play prompt installed");
        (player_of(pending), first_action(pending))
    };
    runner
        .execute_action(player, action_id)
        .expect("Marcus play resolves");
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.battle_area_size(0),
        battle_before + 2,
        "carrier + played Marcus on field"
    );
    let marcus = marcus_handle(&runner).expect("Marcus Damon must be on the field");

    assert_eq!(
        runner.effective_dp(marcus),
        Some(12000),
        "Marcus must be treated as a 12000 DP Digimon"
    );
    assert!(
        runner
            .game
            .modifiers
            .has(marcus, ModifierType::TreatAsDigimon),
        "TreatAsDigimon must be installed"
    );
    assert!(
        runner.game.has_keyword(marcus, Keyword::Rush),
        "Marcus must gain Rush"
    );
    assert!(
        runner
            .game
            .modifiers
            .has(marcus, ModifierType::CannotDigivolve),
        "Marcus must be marked CannotDigivolve"
    );
}

#[test]
fn bt13_020_when_digivolving_modifiers_revert_at_end_of_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT13-020")
        .expect("BT13-020 in pack")
        .add_card(make_tamer("MARCUS", "Marcus Damon"))
        .hand(0, &["MARCUS"])
        .memory(10)
        .start();

    fire_when_digivolving(&mut runner);
    let (player, action_id) = {
        let pending = runner
            .game
            .pending_selection
            .as_ref()
            .expect("play prompt installed");
        (player_of(pending), first_action(pending))
    };
    runner
        .execute_action(player, action_id)
        .expect("Marcus play resolves");
    let _ = runner.auto_resolve();

    let marcus = marcus_handle(&runner).expect("Marcus on field");
    assert_eq!(runner.effective_dp(marcus), Some(12000));

    // End P0's turn → all "for the turn" modifiers expire.
    runner.end_turn();
    let _ = runner.auto_resolve();

    let marcus = marcus_handle(&runner).expect("Marcus still on field");
    assert!(
        !runner
            .game
            .modifiers
            .has(marcus, ModifierType::TreatAsDigimon),
        "TreatAsDigimon must expire at end of turn"
    );
    assert!(
        !runner.game.has_keyword(marcus, Keyword::Rush),
        "Rush must expire at end of turn"
    );
    assert!(
        !runner
            .game
            .modifiers
            .has(marcus, ModifierType::CannotDigivolve),
        "CannotDigivolve must expire at end of turn"
    );
}

#[test]
fn bt13_020_when_digivolving_decline_plays_no_tamer_and_no_modifiers() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT13-020")
        .expect("BT13-020 in pack")
        .add_card(make_tamer("MARCUS", "Marcus Damon"))
        .hand(0, &["MARCUS"])
        .memory(10)
        .start();

    let battle_before = runner.battle_area_size(0);
    let hand_before = runner.game.players[0].hand.len();
    fire_when_digivolving(&mut runner);

    let player = player_of(
        runner
            .game
            .pending_selection
            .as_ref()
            .expect("play prompt installed"),
    );
    runner
        .execute_action(player, PASS)
        .expect("decline resolves");
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.battle_area_size(0),
        battle_before + 1,
        "declining must not play Marcus (only the carrier is on field)"
    );
    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before,
        "Marcus stays in hand on decline"
    );
    assert!(
        marcus_handle(&runner).is_none(),
        "no Marcus on field when the play was declined"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 3 — Clause 2 [Your Turn][OPT] on own Tamer suspend → trash opp sec top
// ─────────────────────────────────────────────────────────────────────────────

/// Runner with BT13-020 as a top-card host (its main clauses are active),
/// a Tamer to suspend, and a 2-card opponent security stack. Both players get
/// a filler deck so turn rotation does not deck-out (needed by OPT-clears).
fn suspend_runner() -> DebugRunner {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT13-020")
        .expect("BT13-020 in pack")
        .add_card(make_tamer("TAMER", "Tamer A"))
        .add_card(make_digimon("SEC1", 3000, CardColor::Blue))
        .add_card(make_digimon("SEC2", 3000, CardColor::Blue))
        .add_card(make_digimon("FILLER", 1000, CardColor::Blue))
        .security(1, &["SEC1", "SEC2"])
        .deck(0, &["FILLER"; 20])
        .deck(1, &["FILLER"; 20])
        .memory(10)
        .start();
    runner.place_on_field(0, "BT13-020", Some(0));
    runner
}

#[test]
fn bt13_020_own_tamer_suspend_trashes_opponent_security_top() {
    let mut runner = suspend_runner();
    let tamer = runner.place_on_field(0, "TAMER", Some(0));

    assert_eq!(runner.game.turn_player(), 0, "must be P0's turn");
    let sec_before = runner.security_count(1);
    assert_eq!(sec_before, 2);

    runner.game.suspend(tamer);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.security_count(1),
        sec_before - 1,
        "exactly one opponent security card trashed"
    );
}

/// The trashed security card must land in the OPPONENT's trash (proving it
/// was trashed from the top of security, not merely removed). The engine does
/// not emit a dedicated security-trash `GameEvent` variant today, so the
/// observable signal is the destination zone delta.
#[test]
fn bt13_020_own_tamer_suspend_moves_top_security_into_opponent_trash() {
    let mut runner = suspend_runner();
    let tamer = runner.place_on_field(0, "TAMER", Some(0));

    // The top of security (the trash target) is the LAST element of the vec
    // (`security_top` push convention).
    let top_card_index = runner.game.players[1]
        .security
        .last()
        .expect("opponent has security")
        .card_index;
    let trash_before = runner.game.players[1].trash.len();

    runner.game.suspend(tamer);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.game.players[1].trash.len(),
        trash_before + 1,
        "exactly one card moved into the opponent's trash"
    );
    assert!(
        runner.game.players[1]
            .trash
            .iter()
            .any(|c| c.card_index == top_card_index),
        "the trashed card must be the former top of the opponent's security"
    );
}

#[test]
fn bt13_020_does_not_fire_when_own_digimon_suspends() {
    let mut runner = suspend_runner();
    let ally = runner.place_on_field(0, "SEC1", Some(0)); // a Digimon, not a Tamer

    let sec_before = runner.security_count(1);
    runner.game.suspend(ally);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.security_count(1),
        sec_before,
        "a Digimon suspending must NOT trigger the Tamer-suspend clause"
    );
}

#[test]
fn bt13_020_does_not_fire_when_opponent_tamer_suspends() {
    let mut runner = suspend_runner();
    let opp_tamer = runner.place_on_field(1, "TAMER", Some(0));

    let sec_before = runner.security_count(1);
    runner.game.suspend(opp_tamer);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.security_count(1),
        sec_before,
        "the OPPONENT's Tamer suspending must NOT trigger 'your Tamers'"
    );
}

#[test]
fn bt13_020_does_not_fire_on_opponents_turn() {
    let mut runner = suspend_runner();
    let tamer = runner.place_on_field(0, "TAMER", Some(0));

    // Switch to the opponent's turn — [Your Turn] gate must block.
    runner.game.turn_player_idx = 1;

    let sec_before = runner.security_count(1);
    runner.game.suspend(tamer);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.security_count(1),
        sec_before,
        "[Your Turn] must gate the clause off on the opponent's turn"
    );
}

#[test]
fn bt13_020_opt_blocks_second_own_tamer_suspend_same_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT13-020")
        .expect("BT13-020 in pack")
        .add_card(make_tamer("TAMER", "Tamer A"))
        .add_card(make_digimon("SEC1", 3000, CardColor::Blue))
        .add_card(make_digimon("SEC2", 3000, CardColor::Blue))
        .add_card(make_digimon("SEC3", 3000, CardColor::Blue))
        .security(1, &["SEC1", "SEC2", "SEC3"])
        .memory(10)
        .start();
    runner.place_on_field(0, "BT13-020", Some(0));
    let tamer_a = runner.place_on_field(0, "TAMER", Some(0));
    let tamer_b = runner.place_on_field(0, "TAMER", Some(0));

    let sec_before = runner.security_count(1);

    // First own-Tamer suspend → trashes one.
    runner.game.suspend(tamer_a);
    let _ = runner.auto_resolve();
    assert_eq!(
        runner.security_count(1),
        sec_before - 1,
        "first suspend trashes one security card"
    );

    // Second own-Tamer suspend same turn → OPT lockout, no further trash.
    runner.game.suspend(tamer_b);
    let _ = runner.auto_resolve();
    assert_eq!(
        runner.security_count(1),
        sec_before - 1,
        "[Once Per Turn] must block the second own-Tamer suspend in the same turn"
    );
}

#[test]
fn bt13_020_opt_clears_next_turn() {
    let mut runner = suspend_runner();
    let tamer = runner.place_on_field(0, "TAMER", Some(0));

    let sec_before = runner.security_count(1);

    // Fire once this turn.
    runner.game.suspend(tamer);
    let _ = runner.auto_resolve();
    assert_eq!(runner.security_count(1), sec_before - 1);

    // Cycle turns until it is P0's turn again (the next own turn).
    for _ in 0..6 {
        runner.end_turn();
        let _ = runner.auto_resolve();
        if runner.game.turn_player() == 0 {
            break;
        }
    }
    assert_eq!(runner.game.turn_player(), 0, "back to P0's turn");

    // Re-suspend (it may have unsuspended); fire again — OPT cleared.
    let tamer = tamer_on_field(&runner, 0, "TAMER").expect("Tamer still on field");
    let sec_mid = runner.security_count(1);
    runner.game.suspend(tamer);
    let _ = runner.auto_resolve();
    assert_eq!(
        runner.security_count(1),
        sec_mid - 1,
        "OPT lockout must clear on the new turn"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// [Burst Digivolve] (CR 6-5-1-2-3, 8-3) through the real action surface.
// Shared scenario: `crate::burst_digivolve_common`.
// ─────────────────────────────────────────────────────────────────────────────

mod burst_digivolve {
    use crate::burst_digivolve_common::*;
    use digimon_engine::enums::CardColor;

    const SPEC: BurstSpec = BurstSpec {
        card: "BT13-020",
        host_name: "ShineGreymon",
        host_color: CardColor::Yellow,
        tamer_name: "Marcus Damon",
    };

    /// 8-3-3: offered via the mask with [Marcus Damon] on the field; returns the
    /// Tamer, pays 0, draws 1, stacks on the [ShineGreymon].
    #[test]
    fn bt13_020_burst_offered_returns_tamer_costs_zero_draws_one() {
        burst_and_check(SPEC, Host::BurstOnly);
    }

    /// The burst cost can't be paid without a [Marcus Damon].
    #[test]
    fn bt13_020_burst_not_offered_without_tamer() {
        assert_not_offered_without_tamer(SPEC);
    }

    /// 8-3-2-1: the top card is trashed at the end of the burst turn,
    /// tracked by permanent identity (the host shifted slots).
    #[test]
    fn bt13_020_burst_top_card_trashed_at_turn_end() {
        assert_turn_end_trash(SPEC, Host::BurstOnly);
    }

    /// Normal (cost 5) and burst routes both legal: a real player choice.
    #[test]
    fn bt13_020_burst_vs_normal_route_is_a_player_choice() {
        assert_route_choice(SPEC);
    }

    /// Choosing the burst route from the route prompt, then the turn-end trash.
    #[test]
    fn bt13_020_burst_chosen_from_route_prompt_then_trashed() {
        assert_turn_end_trash(SPEC, Host::AlsoNormal);
    }

    /// 8-3-2-2: an effect that digivolves can't burst digivolve.
    #[test]
    fn bt13_020_effect_digivolve_cannot_burst() {
        assert_effect_cannot_burst(SPEC);
    }

    /// Two qualifying Tamers: the player picks which returns.
    #[test]
    fn bt13_020_burst_two_tamers_is_a_player_choice() {
        assert_two_tamers_choice(SPEC);
    }
}
