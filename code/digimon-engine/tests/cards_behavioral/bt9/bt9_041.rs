//! BT9-041 RizeGreymon (X Antibody) — Digimon, Lv.5, Yellow/Red, DP 8000,
//! Cost 8, Cyborg/X Antibody, Attribute Vaccine.
//!
//! # Card text (official Bandai DB / data/card_bundles/BT9-041.md)
//!
//! `[When Digivolving] You may play 1 yellow or red Tamer card from your`
//! `hand without paying its memory cost. Then, if [RizeGreymon] or`
//! `[X Antibody] is in this Digimon's digivolution cards, 1 of your`
//! `opponent's Digimon gets -2000 DP for the turn for each yellow or red`
//! `Tamer you have in play.`
//! `[Your Turn] For each Tamer you have in play, this Digimon gets +1000 DP.`
//!
//! Special digivolution condition (xros_req / official DB):
//! `[Digivolve] 1 from [RizeGreymon]` — digivolve for cost 1 from any
//! source card named "RizeGreymon" (name match, any level).
//!
//! Official Q&A (data/card_bundles/BT9-041.md):
//! "No. This effect specifies a card name, so you must have a card whose
//! name is either [RizeGreymon] or [X Antibody] in this Digimon's
//! digivolution cards. Having a card with [X Antibody] in its traits
//! doesn't count." — NAME match, not the trait grant.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT9/Yellow/BT9_041.cs
//!   - `EffectTiming.None`: `AddSelfDigivolutionRequirementStaticEffect`
//!     (permanentCondition: TopCard.CardNames.Contains("RizeGreymon"),
//!     digivolutionCost: 1).
//!   - `EffectTiming.OnEnterFieldAnyone` (When Digivolving):
//!     1. `SelectHandEffect` (canNoSelect: true) over hand Tamers that are
//!        yellow/red → `PlayPermanentCards(payCost: false)`.
//!     2. If `card.PermanentOfThisCard().DigivolutionCards.Count(name ==
//!        "RizeGreymon" || "X Antibody" || "XAntibody") >= 1`:
//!        `minusDP = 2000 * count(own battle-area Tamers that are Red or
//!        Yellow)` (counted AFTER the free play above). If `minusDP > 0`
//!        and an opponent Digimon exists, mandatory `SelectPermanentEffect`
//!        (canNoSelect: false) → `ChangeDigimonDP(-minusDP,
//!        UntilEachTurnEnd)`.
//!   - `EffectTiming.None` (Your Turn aura):
//!     `ChangeSelfDPStaticEffect(+1000 * count(own battle-area Tamers,
//!     ANY color), condition: IsExistOnBattleArea && IsOwnerTurn)`.
//!
//! # Patterns this test file covers (RUST_DSL_TEST_API.md §4.3)
//! - C1/G1-adjacent: alt-path digivolve from a NAMED source ("RizeGreymon"),
//!   cost 1 (`self_digivolution_sources_contain_name`-style name match).
//! - When-Digivolving optional free-play of a color-filtered Tamer
//!   (`select_hand` optional + `play_from_hand_free`).
//! - Sequenced conditional body: `if:` step gating the DP-debuff half on
//!   `self_digivolution_sources_contain_name` (sources-only scan — NOT the
//!   carrier's own top-card name; Q&A-critical).
//! - D1: conditional DP debuff scaled by a `card_count_in_zone` formula
//!   (red/yellow Tamer count), mandatory target select, `end_of_turn` expiry.
//! - D4: declarative self-aura DP buff scaled by ALL-Tamer count, `your_turn`
//!   gated (`kind: aura`, `dp_modifier` formula).

#![allow(dead_code, unused_imports)]

#[path = "../../support/dsl_card_data.rs"]
mod dsl_card_data;

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledScope,
    CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::PendingSelection;
use digimon_engine::{EffectTiming, TriggerSource};

// ─── helpers ──────────────────────────────────────────────────────────────────

fn rizegreymon_x() -> digimon_dsl::compiled::CompiledCard {
    dsl_card_data::compiled("BT9-041")
}

fn make_tamer(card_id: &str, name: &str, color: CardColor) -> CardData {
    let mut card = make_test_card(card_id, name);
    card.card_kind = CardKind::Tamer;
    card.colors = vec![color];
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

fn tamer_on_field(
    runner: &DebugRunner,
    player: digimon_engine::enums::PlayerId,
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

fn player_of(p: &PendingSelection) -> digimon_engine::enums::PlayerId {
    p.selecting_player
}

fn first_action(p: &PendingSelection) -> u16 {
    *p.valid_action_ids
        .first()
        .expect("at least one legal action")
}

/// Place BT9-041 as a bare top card (no digivolution source) and fire its
/// [When Digivolving] clause.
fn fire_when_digivolving_bare(runner: &mut DebugRunner) -> PermanentHandle {
    let carrier = runner.place_on_field(0, "BT9-041", Some(0));
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(carrier));
    runner.game.drain_effect_queue();
    carrier
}

/// Place BT9-041 on top of a digivolution source stack, then fire
/// [When Digivolving].
fn fire_when_digivolving_stacked(runner: &mut DebugRunner, source_ids: &[&str]) -> PermanentHandle {
    let mut stack: Vec<&str> = source_ids.to_vec();
    stack.push("BT9-041");
    let carrier = runner.place_stack(0, &stack);
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(carrier));
    runner.game.drain_effect_queue();
    carrier
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 1 — Structural assertions
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn bt9_041_metadata_matches_printed() {
    let card = rizegreymon_x();
    assert_eq!(card.card, "BT9-041");
    assert_eq!(card.name, "RizeGreymon (X Antibody)");
    assert_eq!(card.level, Some(5));
    assert_eq!(card.dp, Some(8000));
    assert_eq!(card.cost, Some(8));

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
        card.traits.iter().any(|t| t.eq_ignore_ascii_case("Cyborg")),
        "must have Cyborg trait; got {:?}",
        card.traits
    );
    assert!(
        card.traits
            .iter()
            .any(|t| t.eq_ignore_ascii_case("X Antibody")),
        "must have X Antibody trait; got {:?}",
        card.traits
    );
}

#[test]
fn bt9_041_has_one_triggered_when_digivolving_clause() {
    let card = rizegreymon_x();
    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    let wd: Vec<_> = triggered
        .iter()
        .filter(|t| t.when.contains(&CompiledTiming::WhenDigivolving))
        .collect();
    assert_eq!(
        wd.len(),
        1,
        "exactly one [When Digivolving] triggered clause; got {:?}",
        triggered.iter().map(|t| t.when.clone()).collect::<Vec<_>>()
    );
    assert_eq!(wd[0].scope, CompiledScope::FaceUp);
    assert!(
        !wd[0].optional,
        "the [When Digivolving] clause overall is not optional — only the \
         inner Tamer-play half is 'you may'"
    );
}

#[test]
fn bt9_041_has_declarative_your_turn_aura_clause() {
    let card = rizegreymon_x();
    let aura = card.effects.iter().find_map(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
            dp_modifier,
            dp_modifier_fn,
            active_when,
            ..
        }) => Some((dp_modifier.clone(), dp_modifier_fn.is_some(), active_when.clone())),
        _ => None,
    });

    let (dp_modifier, has_formula, _active_when) =
        aura.expect("a declarative Aura clause for the [Your Turn] +DP buff must exist");

    assert!(
        dp_modifier.is_none() || has_formula,
        "the +1000-per-Tamer buff must be a runtime formula, not a flat literal"
    );
    assert!(
        has_formula,
        "[Your Turn] +1000 DP per Tamer must compile to dp_modifier_fn (formula), not a flat int"
    );
}

#[test]
fn bt9_041_has_digivolve_alt_path_from_named_rizegreymon_cost_1() {
    let card = rizegreymon_x();
    let found = card.alt_paths.iter().any(|p| {
        p.kind == CompiledAltPathKind::Digivolve && p.cost == Some(CompiledCost::Literal(1))
    });
    assert!(
        found,
        "must declare a cost-1 Digivolve alt-path from a named [RizeGreymon] source; \
         alt_paths={:?}",
        card.alt_paths
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 2 — Clause (a) [When Digivolving]: optional free Tamer play
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn bt9_041_when_digivolving_installs_optional_yellow_red_tamer_play() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT9-041")
        .expect("BT9-041 in pack")
        .add_card(make_tamer("TAMER-Y", "Yellow Tamer", CardColor::Yellow))
        .hand(0, &["TAMER-Y"])
        .memory(10)
        .start();

    fire_when_digivolving_bare(&mut runner);

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("the [When Digivolving] optional Tamer-play prompt must install");
    assert!(
        pending.is_optional,
        "'You may play' is optional — PASS must be legal"
    );
    assert_eq!(
        pending.valid_action_ids.len(),
        1,
        "exactly the yellow Tamer in hand is selectable; got {:?}",
        pending.valid_action_ids
    );
}

#[test]
fn bt9_041_when_digivolving_offers_red_tamer_too() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT9-041")
        .expect("BT9-041 in pack")
        .add_card(make_tamer("TAMER-R", "Red Tamer", CardColor::Red))
        .hand(0, &["TAMER-R"])
        .memory(10)
        .start();

    fire_when_digivolving_bare(&mut runner);

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("a red Tamer must also be offered");
    assert_eq!(pending.valid_action_ids.len(), 1);
}

/// NEGATIVE: a non-yellow/red (e.g. blue) Tamer in hand must NOT be offered.
#[test]
fn bt9_041_when_digivolving_excludes_off_color_tamer() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT9-041")
        .expect("BT9-041 in pack")
        .add_card(make_tamer("TAMER-B", "Blue Tamer", CardColor::Blue))
        .hand(0, &["TAMER-B"])
        .memory(10)
        .start();

    fire_when_digivolving_bare(&mut runner);

    assert!(
        runner.game.pending_selection.is_none(),
        "a blue Tamer must not satisfy the yellow/red filter — no prompt should install; \
         got {:?}",
        runner.game.pending_selection
    );
}

/// NEGATIVE: no Tamer in hand at all → no prompt installs (existential guard).
#[test]
fn bt9_041_when_digivolving_no_tamer_in_hand_installs_no_prompt() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT9-041")
        .expect("BT9-041 in pack")
        .memory(10)
        .start();

    fire_when_digivolving_bare(&mut runner);

    assert!(
        runner.game.pending_selection.is_none(),
        "no yellow/red Tamer in hand → no play prompt; got {:?}",
        runner.game.pending_selection
    );
}

#[test]
fn bt9_041_when_digivolving_playing_tamer_does_not_pay_memory_cost() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT9-041")
        .expect("BT9-041 in pack")
        .add_card(make_tamer("TAMER-Y", "Yellow Tamer", CardColor::Yellow))
        .hand(0, &["TAMER-Y"])
        .memory(10)
        .start();

    let memory_before = runner.memory();
    let battle_before = runner.battle_area_size(0);
    fire_when_digivolving_bare(&mut runner);

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
        .expect("Tamer play resolves");
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.battle_area_size(0),
        battle_before + 2,
        "carrier + played Tamer on field"
    );
    assert_eq!(
        runner.memory(),
        memory_before,
        "playing the Tamer via this effect must not change the memory gauge \
         (free play, no cost paid)"
    );
}

#[test]
fn bt9_041_when_digivolving_decline_plays_no_tamer() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT9-041")
        .expect("BT9-041 in pack")
        .add_card(make_tamer("TAMER-Y", "Yellow Tamer", CardColor::Yellow))
        .hand(0, &["TAMER-Y"])
        .memory(10)
        .start();

    let battle_before = runner.battle_area_size(0);
    let hand_before = runner.game.players[0].hand.len();
    fire_when_digivolving_bare(&mut runner);

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
        "declining must not play the Tamer (only the carrier is on field)"
    );
    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before,
        "Tamer stays in hand on decline"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 3 — Clause (a)-part-2: conditional DP debuff (source-name gate)
// ─────────────────────────────────────────────────────────────────────────────
//
// "Then, if [RizeGreymon] or [X Antibody] is in this Digimon's digivolution
// cards, 1 of opponent's Digimon gets -2000 DP for the turn for each yellow
// or red Tamer you have in play." — Q&A: NAME match on digivolution SOURCE
// cards only (not the carrier's own top-card name, not the trait grant).

/// NEGATIVE (Q&A-critical): BT9-041 placed BARE (no digivolution source) has
/// NO qualifying name among its sources — its own top-card name ("RizeGreymon
/// (X Antibody)") does not count per the sources-only scan. No debuff prompt
/// installs even though an opponent Digimon exists.
#[test]
fn bt9_041_no_debuff_prompt_when_no_source_stack() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT9-041")
        .expect("BT9-041 in pack")
        .add_card(make_tamer("TAMER-Y", "Yellow Tamer", CardColor::Yellow))
        .add_card(make_digimon("OPP-1", 5000, CardColor::Blue))
        .hand(0, &["TAMER-Y"])
        .memory(10)
        .start();
    runner.place_on_field(1, "OPP-1", Some(0));

    let carrier = fire_when_digivolving_bare(&mut runner);
    let _ = carrier;

    // Decline the Tamer-play prompt so we can inspect the follow-up directly.
    let player = player_of(
        runner
            .game
            .pending_selection
            .as_ref()
            .expect("play prompt installed"),
    );
    runner.execute_action(player, PASS).expect("decline resolves");
    let _ = runner.auto_resolve();

    assert!(
        runner.game.pending_selection.is_none(),
        "bare BT9-041 (no digivolution source) must NOT satisfy the \
         'RizeGreymon/X Antibody in digivolution cards' condition — no debuff \
         prompt should install; got {:?}",
        runner.game.pending_selection
    );
    assert_eq!(
        runner.effective_dp(PermanentHandle { player: 1, index: 0 }),
        Some(5000),
        "opponent Digimon DP must be untouched"
    );
}

/// NEGATIVE: an unrelated source name (e.g. "WarGreymon") also fails the
/// condition.
#[test]
fn bt9_041_no_debuff_prompt_when_unrelated_source_in_stack() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT9-041")
        .expect("BT9-041 in pack")
        .add_card(make_digimon("TEST-PLAIN-SOURCE", 6000, CardColor::Red))
        .add_card(make_digimon("OPP-1", 5000, CardColor::Blue))
        .memory(10)
        .start();
    runner.place_on_field(1, "OPP-1", Some(0));
    {
        let mut card = runner.game.card_data
            [runner.game.card_data.iter().position(|c| c.card_id == "TEST-PLAIN-SOURCE").unwrap()]
        .clone();
        card.card_name = "WarGreymon".to_string();
        let idx = runner.game.card_data.iter().position(|c| c.card_id == "TEST-PLAIN-SOURCE").unwrap();
        runner.game.card_data[idx] = card;
    }

    fire_when_digivolving_stacked(&mut runner, &["TEST-PLAIN-SOURCE"]);

    assert!(
        runner.game.pending_selection.is_none(),
        "no yellow/red Tamer in hand → the Tamer-play half installs nothing; \
         the debuff half must also stay silent because the source name is \
         unrelated; got {:?}",
        runner.game.pending_selection
    );
    assert_eq!(
        runner.effective_dp(PermanentHandle { player: 1, index: 0 }),
        Some(5000),
        "opponent Digimon DP must be untouched"
    );
}

/// POSITIVE: BT9-041 stacked on a "RizeGreymon"-named source satisfies the
/// condition — with a red/yellow Tamer already in play and an opponent
/// Digimon present, a mandatory debuff-target prompt installs.
#[test]
fn bt9_041_rizegreymon_source_installs_debuff_prompt() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT9-041")
        .expect("BT9-041 in pack")
        .add_card(make_digimon("TEST-RIZE-SOURCE", 7000, CardColor::Red))
        .add_card(make_tamer("TAMER-Y", "Yellow Tamer", CardColor::Yellow))
        .add_card(make_digimon("OPP-1", 5000, CardColor::Blue))
        .memory(10)
        .start();
    {
        let idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "TEST-RIZE-SOURCE")
            .unwrap();
        runner.game.card_data[idx].card_name = "RizeGreymon".to_string();
    }
    runner.place_on_field(0, "TAMER-Y", Some(0));
    runner.place_on_field(1, "OPP-1", Some(0));

    fire_when_digivolving_stacked(&mut runner, &["TEST-RIZE-SOURCE"]);

    // No Tamer in hand → the free-play half is a silent no-op; the debuff
    // half fires immediately since the condition (RizeGreymon in sources) holds.
    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("the debuff target-select prompt must install");
    assert!(
        !pending.is_optional,
        "the debuff target-select is mandatory once the condition holds and \
         minusDP > 0 with an eligible opponent Digimon"
    );
}

/// POSITIVE: an "X Antibody"-named source also satisfies the condition
/// (the printed OR).
#[test]
fn bt9_041_x_antibody_named_source_installs_debuff_prompt() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT9-041")
        .expect("BT9-041 in pack")
        .add_card(make_digimon("TEST-XANTI-SOURCE", 7000, CardColor::Red))
        .add_card(make_tamer("TAMER-R", "Red Tamer", CardColor::Red))
        .add_card(make_digimon("OPP-1", 5000, CardColor::Blue))
        .memory(10)
        .start();
    {
        let idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "TEST-XANTI-SOURCE")
            .unwrap();
        runner.game.card_data[idx].card_name = "X Antibody".to_string();
    }
    runner.place_on_field(0, "TAMER-R", Some(0));
    runner.place_on_field(1, "OPP-1", Some(0));

    fire_when_digivolving_stacked(&mut runner, &["TEST-XANTI-SOURCE"]);

    assert!(
        runner.game.pending_selection.is_some(),
        "an 'X Antibody'-named source must also satisfy the condition"
    );
}

/// POSITIVE: resolving the debuff target applies -2000 DP per red/yellow
/// Tamer in play (counted AFTER the free Tamer play resolves).
#[test]
fn bt9_041_debuff_scales_by_red_yellow_tamer_count_after_free_play() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT9-041")
        .expect("BT9-041 in pack")
        .add_card(make_digimon("TEST-RIZE-SOURCE", 7000, CardColor::Red))
        .add_card(make_tamer("TAMER-Y1", "Yellow Tamer 1", CardColor::Yellow))
        .add_card(make_tamer("TAMER-R2", "Red Tamer 2 (hand)", CardColor::Red))
        .add_card(make_digimon("OPP-1", 9000, CardColor::Blue))
        .hand(0, &["TAMER-R2"])
        .memory(10)
        .start();
    {
        let idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "TEST-RIZE-SOURCE")
            .unwrap();
        runner.game.card_data[idx].card_name = "RizeGreymon".to_string();
    }
    // One yellow Tamer already in play.
    runner.place_on_field(0, "TAMER-Y1", Some(0));
    runner.place_on_field(1, "OPP-1", Some(0));

    fire_when_digivolving_stacked(&mut runner, &["TEST-RIZE-SOURCE"]);

    // Step 1: accept the free-play offer for the red Tamer in hand — this
    // brings the in-play red/yellow Tamer count to 2 BEFORE the debuff
    // formula evaluates.
    let (player, action_id) = {
        let pending = runner
            .game
            .pending_selection
            .as_ref()
            .expect("Tamer-play prompt must install (a red Tamer is in hand)");
        (player_of(pending), first_action(pending))
    };
    runner
        .execute_action(player, action_id)
        .expect("free Tamer play resolves");
    let _ = runner.game.drain_effect_queue();

    // Step 2: the debuff target-select prompt should now be pending.
    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("debuff target-select prompt must install after the free play");
    let (dplayer, daction) = (player_of(pending), first_action(pending));
    runner
        .execute_action(dplayer, daction)
        .expect("debuff target resolves");
    let _ = runner.auto_resolve();

    // 2 red/yellow Tamers in play (TAMER-Y1 placed + TAMER-R2 freshly played)
    // → -2000 * 2 = -4000 DP.
    assert_eq!(
        runner.effective_dp(PermanentHandle { player: 1, index: 0 }),
        Some(5000),
        "opponent Digimon must lose 2000 DP per red/yellow Tamer in play \
         (2 Tamers → -4000 DP; 9000 - 4000 = 5000)"
    );
}

/// NEGATIVE: with the condition satisfied but NO red/yellow Tamer in play at
/// all, minusDP is 0 — DCGO gates the target-select on `minusDP > 0`, so no
/// prompt installs and no debuff applies.
#[test]
fn bt9_041_no_debuff_prompt_when_condition_met_but_no_tamer_in_play() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT9-041")
        .expect("BT9-041 in pack")
        .add_card(make_digimon("TEST-RIZE-SOURCE", 7000, CardColor::Red))
        .add_card(make_digimon("OPP-1", 5000, CardColor::Blue))
        .memory(10)
        .start();
    {
        let idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "TEST-RIZE-SOURCE")
            .unwrap();
        runner.game.card_data[idx].card_name = "RizeGreymon".to_string();
    }
    runner.place_on_field(1, "OPP-1", Some(0));

    fire_when_digivolving_stacked(&mut runner, &["TEST-RIZE-SOURCE"]);

    assert!(
        runner.game.pending_selection.is_none(),
        "with zero red/yellow Tamers in play, minusDP is 0 and no debuff \
         prompt should install even though the source-name condition holds; \
         got {:?}",
        runner.game.pending_selection
    );
    assert_eq!(
        runner.effective_dp(PermanentHandle { player: 1, index: 0 }),
        Some(5000),
        "opponent Digimon DP must be untouched when minusDP is 0"
    );
}

/// NEGATIVE: condition satisfied, a Tamer is in play, but the opponent has
/// NO Digimon — the mandatory select silently no-ops (existential guard),
/// matching DCGO's `HasMatchConditionPermanent` guard.
#[test]
fn bt9_041_no_debuff_prompt_when_opponent_has_no_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT9-041")
        .expect("BT9-041 in pack")
        .add_card(make_digimon("TEST-RIZE-SOURCE", 7000, CardColor::Red))
        .add_card(make_tamer("TAMER-Y1", "Yellow Tamer 1", CardColor::Yellow))
        .memory(10)
        .start();
    {
        let idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "TEST-RIZE-SOURCE")
            .unwrap();
        runner.game.card_data[idx].card_name = "RizeGreymon".to_string();
    }
    runner.place_on_field(0, "TAMER-Y1", Some(0));

    fire_when_digivolving_stacked(&mut runner, &["TEST-RIZE-SOURCE"]);

    assert!(
        runner.game.pending_selection.is_none(),
        "opponent has no Digimon → the mandatory debuff select must silently \
         no-op; got {:?}",
        runner.game.pending_selection
    );
}

/// The DP debuff expires at end of turn ("for the turn").
#[test]
fn bt9_041_debuff_expires_at_end_of_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT9-041")
        .expect("BT9-041 in pack")
        .add_card(make_digimon("TEST-RIZE-SOURCE", 7000, CardColor::Red))
        .add_card(make_tamer("TAMER-Y1", "Yellow Tamer 1", CardColor::Yellow))
        .add_card(make_digimon("OPP-1", 9000, CardColor::Blue))
        .memory(10)
        .start();
    {
        let idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "TEST-RIZE-SOURCE")
            .unwrap();
        runner.game.card_data[idx].card_name = "RizeGreymon".to_string();
    }
    runner.place_on_field(0, "TAMER-Y1", Some(0));
    runner.place_on_field(1, "OPP-1", Some(0));

    fire_when_digivolving_stacked(&mut runner, &["TEST-RIZE-SOURCE"]);

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("debuff target-select prompt must install");
    let (player, action_id) = (player_of(pending), first_action(pending));
    runner
        .execute_action(player, action_id)
        .expect("debuff target resolves");
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.effective_dp(PermanentHandle { player: 1, index: 0 }),
        Some(7000),
        "1 red/yellow Tamer in play → -2000 DP; 9000 - 2000 = 7000"
    );

    // End P0's turn — "for the turn" modifiers expire.
    runner.end_turn();
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.effective_dp(PermanentHandle { player: 1, index: 0 }),
        Some(9000),
        "the DP debuff must expire at end of turn"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 4 — Clause (b) [Your Turn]: +1000 DP per Tamer (ALL colors)
// ─────────────────────────────────────────────────────────────────────────────

/// POSITIVE: with 2 Tamers of ANY color in play (not just red/yellow), the
/// carrier gets +2000 DP on its controller's turn.
#[test]
fn bt9_041_your_turn_buff_scales_by_all_tamer_colors() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT9-041")
        .expect("BT9-041 in pack")
        .add_card(make_tamer("TAMER-GREEN", "Green Tamer", CardColor::Green))
        .add_card(make_tamer("TAMER-BLUE", "Blue Tamer", CardColor::Blue))
        .memory(15)
        .start();

    let carrier = runner.place_on_field(0, "BT9-041", Some(0));
    assert_eq!(
        runner.effective_dp(carrier),
        Some(8000),
        "base DP with no Tamers in play"
    );

    runner.place_on_field(0, "TAMER-GREEN", Some(0));
    runner.place_on_field(0, "TAMER-BLUE", Some(0));
    runner.game.tick_declarative_effects();

    assert_eq!(
        runner.effective_dp(carrier),
        Some(10000),
        "+1000 per Tamer regardless of color: 2 Tamers → +2000; 8000 + 2000 = 10000"
    );
}

/// NEGATIVE: on the OPPONENT's turn, the [Your Turn] buff does not apply
/// even with Tamers in play.
#[test]
fn bt9_041_your_turn_buff_does_not_apply_on_opponents_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT9-041")
        .expect("BT9-041 in pack")
        .add_card(make_tamer("TAMER-GREEN", "Green Tamer", CardColor::Green))
        .memory(15)
        .start();

    let carrier = runner.place_on_field(0, "BT9-041", Some(0));
    runner.place_on_field(0, "TAMER-GREEN", Some(0));
    runner.game.turn_player_idx = 1;
    runner.game.tick_declarative_effects();

    assert_eq!(
        runner.effective_dp(carrier),
        Some(8000),
        "[Your Turn] buff must not apply on the opponent's turn"
    );
}

/// NEGATIVE: with zero Tamers in play, the buff contributes 0 (base DP only).
#[test]
fn bt9_041_your_turn_buff_zero_with_no_tamers() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT9-041")
        .expect("BT9-041 in pack")
        .memory(10)
        .start();

    let carrier = runner.place_on_field(0, "BT9-041", Some(0));
    runner.game.tick_declarative_effects();

    assert_eq!(
        runner.effective_dp(carrier),
        Some(8000),
        "no Tamers in play → no DP buff"
    );
}
