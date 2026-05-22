//! BT13-012 GeoGreymon — Digimon, Lv.4, Red+Yellow, DP 5000, Cost 5, Dinosaur.
//!
//! # Card text (cards.json)
//!
//! `[When Digivolving] Search your security stack, and you may play 1 red or`
//! `yellow Tamer card among it without paying its cost. If you did,`
//! `＜Recovery +1 (Deck)＞. Then, shuffle your security stack.`
//!
//! Inherited Effect:
//! `[Your Turn] [Once Per Turn] When one of your red or yellow Tamers becomes`
//! `suspended, you may delete 1 of your opponent's Digimon with 3000 DP or less.`
//!
//! Alt-path: `[Digivolve] Lv.3 w/[Agumon] in name and [Dinosaur] trait: Cost 2`
//! (per `xros_req` column / DCGO `AddSelfDigivolutionRequirementStaticEffect`).
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT13/Red/BT13_012.cs
//!
//! # Patterns this test file covers
//! - C5-adjacent: When-Digivolving security search + conditional Recovery +
//!   shuffle (clause 1). G-PLAY-SELECTED-SECURITY-CARD resolved 2026-05-20:
//!   `select_security` honours its `filter`, and the `play_security_card`
//!   step verb plays a bound security card free.
//! - B3-adjacent: Inherited triggered observer of own-Tamer suspend events
//!   (clause 2). G-EVENT-TARGET-COLOR resolved 2026-05-20 — the red/yellow
//!   color filter is enforced via `event_target_color_any_of`.
//! - F-OPT: [Once Per Turn] on a triggered observer.
//! - F-Predicate: dp_lte filter on opponent permanent select.

#![allow(dead_code, unused_imports)]

#[path = "../../support/dsl_card_data.rs"]
mod dsl_card_data;

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::{EffectTiming, TriggerSource};

use crate::dsl_card_data::{card_data_from_compiled, compiled};

// ─── helpers ──────────────────────────────────────────────────────────────────

fn geogreymon() -> digimon_dsl::compiled::CompiledCard {
    compiled("BT13-012")
}

fn make_tamer(card_id: &str, colors: Vec<CardColor>) -> CardData {
    let mut card = make_test_card(card_id, card_id);
    card.card_kind = CardKind::Tamer;
    card.colors = colors;
    card.dp = None;
    card.level = None;
    card
}

fn make_digimon(card_id: &str, dp: i32, color: CardColor) -> CardData {
    let mut card = make_test_card(card_id, card_id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![color];
    card.dp = Some(dp);
    card
}

/// Drop GeoGreymon (and any companion fixtures) into card_data and start a
/// minimal runner. The shared `TOP-DGM` fixture is the placeholder top card
/// for the stacked carrier — every test below uses
/// `place_stack(0, &["BT13-012", "TOP-DGM"])` so the inherited triggered
/// clause is active (only `card_sources` cards' inherited fires; the TOP
/// card's own triggered clauses fire from its main effect text, not its
/// inherited slot).
fn fresh_runner(extra: Vec<CardData>) -> DebugRunner {
    let mut builder = DebugRunner::builder()
        .dsl_card("BT13-012")
        .expect("BT13-012 in embedded pack")
        .add_card(make_test_card("TOP-DGM", "TopDgm"));
    for card in extra {
        builder = builder.add_card(card);
    }
    builder.start()
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 1 — Structural assertions
// ─────────────────────────────────────────────────────────────────────────────

/// GeoGreymon is a Digimon, Lv.4, Red+Yellow, DP 5000, cost 5, with the
/// Dinosaur trait. Sanity check the metadata in the compiled spec — these are
/// stable contract bits that must match the printed card.
#[test]
fn bt13_012_metadata_matches_printed() {
    let card = geogreymon();
    assert_eq!(card.card, "BT13-012");
    assert_eq!(card.name, "GeoGreymon");
    assert_eq!(card.level, Some(4));
    assert_eq!(card.dp, Some(5000));
    assert_eq!(card.cost, Some(5));

    // Color set: Red + Yellow.
    let colors: Vec<_> = card.color.iter().copied().collect();
    assert!(
        colors.contains(&digimon_dsl::compiled::CompiledColor::Red),
        "GeoGreymon must be Red"
    );
    assert!(
        colors.contains(&digimon_dsl::compiled::CompiledColor::Yellow),
        "GeoGreymon must be Yellow"
    );

    assert!(
        card.traits
            .iter()
            .any(|t| t.eq_ignore_ascii_case("Dinosaur")),
        "GeoGreymon must have the Dinosaur trait; got {:?}",
        card.traits
    );
}

/// Alt-path: Lv.3 w/[Agumon] in name and [Dinosaur] trait, cost 2.
/// Mirrors DCGO `BT13_012.AddSelfDigivolutionRequirementStaticEffect` — the
/// reduced-cost digivolve is a static rule, not a triggered effect.
#[test]
fn bt13_012_alt_path_agumon_dinosaur_lv3_cost2() {
    let card = geogreymon();
    let agumon_alt = card
        .alt_paths
        .iter()
        .find(|alt| matches!(alt.kind, CompiledAltPathKind::Digivolve));
    let alt = agumon_alt
        .expect("BT13-012 must declare a Digivolve alt-path for the Agumon/Dinosaur Lv.3");
    // Cost of 2 per the printed alt-path text. Lowered cost is encoded as a
    // CompiledCostSpec literal — this is the canonical shape used by
    // BT17-018, BT20-102, etc.
    let cost_repr = format!("{:?}", alt.cost);
    assert!(
        cost_repr.contains('2'),
        "Agumon/Dinosaur alt-path cost must be 2; got {}",
        cost_repr
    );
}

/// The shipping spec has exactly two Triggered clauses — the [When
/// Digivolving] security-search clause (clause 1) and the inherited
/// on_suspend observer (clause 2).
#[test]
fn bt13_012_has_exactly_one_triggered_clause() {
    let card = geogreymon();
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
        "Two triggered clauses ship: [When Digivolving] + inherited on_suspend. Got: {:?}",
        triggered
            .iter()
            .map(|t| (t.scope, t.when.clone()))
            .collect::<Vec<_>>()
    );
}

/// The inherited clause is on `OnSuspend`, optional, OPT, scope = Inherited.
#[test]
fn bt13_012_inherited_on_suspend_clause_shape() {
    let card = geogreymon();
    let triggered = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSuspend) => Some(t),
            _ => None,
        })
        .expect("OnSuspend triggered clause must exist");
    assert_eq!(
        triggered.scope,
        CompiledScope::Inherited,
        "clause is printed as 'Inherited Effect' so scope must be Inherited"
    );
    assert!(
        triggered.optional,
        "card text 'you may delete' makes the prompt optional"
    );
    assert!(triggered.once_per_turn, "[Once Per Turn] flag must be set");
}

/// Clause 1 ([When Digivolving] security search) must be present in the
/// shipping spec. G-PLAY-SELECTED-SECURITY-CARD is resolved — the clause is
/// authored with `select_security` + the `play_security_card` step verb.
#[test]
fn bt13_012_when_digivolving_clause_intentionally_omitted() {
    let card = geogreymon();
    let when_digivolving = card.effects.iter().any(|c| match c {
        CompiledClause::Triggered(t) => t.when.contains(&CompiledTiming::WhenDigivolving),
        _ => false,
    });
    assert!(
        when_digivolving,
        "Clause 1 ([When Digivolving] security search) must be authored — \
         G-PLAY-SELECTED-SECURITY-CARD is resolved via the `play_security_card` step verb."
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 2 — Clause 1 [When Digivolving]: security search + play + Recovery
//
// G-PLAY-SELECTED-SECURITY-CARD resolved 2026-05-20: `select_security` now
// honours its `filter` field (was previously ignored by
// `install_select_security`), and the new `play_security_card: { of, card }`
// step verb plays a bound security card free (engine
// `EffectContext::play_from_security_card`). The conditional Recovery is
// gated with `binding_present: sec_tamer`.
// ─────────────────────────────────────────────────────────────────────────────

/// Clause 1 — Pick branch: digivolving BT13-012 installs the optional
/// security-search prompt; picking a red Tamer plays it free onto the field,
/// fires Recovery +1, and shuffles security.
#[test]
fn bt13_012_clause1_when_digivolving_is_blocked() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT13-012")
        .expect("BT13-012 in embedded pack")
        .add_card(make_tamer("SEC-RED-TAMER", vec![CardColor::Red]))
        .add_card(make_tamer("SEC-YEL-TAMER", vec![CardColor::Yellow]))
        .add_card(make_digimon("SEC-FILLER", 3000, CardColor::Blue))
        .add_card(make_digimon("DECK-FILLER", 3000, CardColor::Blue))
        .security(0, &["SEC-RED-TAMER", "SEC-YEL-TAMER", "SEC-FILLER"])
        .deck(0, &["DECK-FILLER"])
        .memory(10)
        .start();

    // BT13-012 as the carrier (top card) on P0's field — its own [When
    // Digivolving] clause fires from the top-card effect slot.
    let carrier = runner.place_on_field(0, "BT13-012", Some(0));

    let battle_before = runner.battle_area_size(0);
    let sec_before = runner.security_count(0);
    let deck_before = runner.deck_size(0);

    // Fire [When Digivolving] for the carrier.
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(carrier),
    );
    runner.game.drain_effect_queue();

    // The optional security-search prompt installs — only the two Tamers
    // (red + yellow) are legal picks; the Blue filler Digimon is filtered out.
    let (player, action_id) = {
        let pending = runner
            .game
            .pending_selection
            .as_ref()
            .expect("select_security prompt must install on [When Digivolving]");
        assert!(
            pending.is_optional,
            "the security-search play is 'you may', so optional"
        );
        // The `filter` admits exactly the 2 red/yellow Tamers — the Blue
        // filler Digimon in security is excluded. (PASS is surfaced via the
        // `is_optional` flag, not as a `valid_action_ids` entry.)
        assert_eq!(
            pending.valid_action_ids.len(),
            2,
            "filter must admit exactly the 2 red/yellow Tamers; got {:?}",
            pending.valid_action_ids
        );
        (
            pending.selecting_player,
            *pending
                .valid_action_ids
                .first()
                .expect("a Tamer pick"),
        )
    };
    runner
        .execute_action(player, action_id)
        .expect("security-card pick resolves");
    let _ = runner.auto_resolve();

    // A Tamer was played free onto P0's battle area.
    assert_eq!(
        runner.battle_area_size(0),
        battle_before + 1,
        "the chosen red/yellow Tamer must be played free onto the field"
    );
    let played_tamer = runner.game.players[0].battle_area.iter().any(|p| {
        let id = p.top_card().card_id(&runner.game.card_data);
        id == "SEC-RED-TAMER" || id == "SEC-YEL-TAMER"
    });
    assert!(played_tamer, "a red/yellow Tamer must now be on P0's field");

    // Recovery +1: one deck card moved to security. Net security delta:
    // -1 (Tamer played out) +1 (Recovery) = 0.
    assert_eq!(
        runner.security_count(0),
        sec_before,
        "security count net-unchanged: -1 Tamer played, +1 Recovery"
    );
    assert_eq!(
        deck_before - runner.deck_size(0),
        1,
        "Recovery +1 must move exactly 1 card from deck to security"
    );
}

/// Clause 1 — Decline branch: declining the optional security-search prompt
/// plays no Tamer and fires no Recovery, but the security stack is still
/// shuffled (the "Then, shuffle" clause is unconditional per printed text).
#[test]
fn bt13_012_clause1_decline_skips_play_and_recovery() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT13-012")
        .expect("BT13-012 in embedded pack")
        .add_card(make_tamer("SEC-RED-TAMER", vec![CardColor::Red]))
        .add_card(make_digimon("SEC-FILLER", 3000, CardColor::Blue))
        .add_card(make_digimon("DECK-FILLER", 3000, CardColor::Blue))
        .security(0, &["SEC-RED-TAMER", "SEC-FILLER"])
        .deck(0, &["DECK-FILLER"])
        .memory(10)
        .start();

    let carrier = runner.place_on_field(0, "BT13-012", Some(0));

    let battle_before = runner.battle_area_size(0);
    let sec_before = runner.security_count(0);
    let deck_before = runner.deck_size(0);

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(carrier),
    );
    runner.game.drain_effect_queue();

    let player = runner
        .game
        .pending_selection
        .as_ref()
        .expect("select_security prompt installs")
        .selecting_player;
    runner
        .execute_action(player, PASS)
        .expect("decline resolves");
    let _ = runner.auto_resolve();

    // Declining: no Tamer played, no Recovery — battle area, security count,
    // and deck size all unchanged.
    assert_eq!(
        runner.battle_area_size(0),
        battle_before,
        "declining must not play a Tamer"
    );
    assert_eq!(
        runner.security_count(0),
        sec_before,
        "declining must not change security count (no play, no Recovery)"
    );
    assert_eq!(
        runner.deck_size(0),
        deck_before,
        "Recovery must NOT fire when the play was declined (binding_present gate)"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 3 — Clause 2 (Inherited [OPT][Your Turn] on red/yellow Tamer
// suspends → may delete opp Digimon DP≤3000): condition gating
// ─────────────────────────────────────────────────────────────────────────────

/// Positive condition gate: an own Tamer suspending on the controller's turn
/// installs the optional delete prompt.
///
/// Setup:
///   - GeoGreymon on P0's field (acts as the inherited-effect host).
///   - A red Tamer on P0's field.
///   - A 3000 DP Digimon on P1's field.
///   - It is P0's turn (default after start()).
///
/// Action:
///   - Enqueue OnSuspend triggered with TriggerSource::Permanent(red_tamer).
///   - drain_effect_queue.
///
/// Expected:
///   - A pending optional selection is installed (delete prompt).
#[test]
fn bt13_012_on_own_red_tamer_suspend_installs_delete_prompt() {
    let mut runner = fresh_runner(vec![
        make_tamer("RED-TAMER", vec![CardColor::Red]),
        make_digimon("OPP-DGM-3000", 3000, CardColor::Blue),
    ]);

    // Place a stack with GeoGreymon as a SOURCE under TOP-DGM. Inherited
    // effects fire only from `card_sources` cards (excluding the top); they
    // are NOT active when the carrier itself is the top of the stack.
    let _carrier = runner.place_stack(0, &["BT13-012", "TOP-DGM"]);
    // Place the red Tamer ally on P0's field, opp Digimon on P1's field.
    let red_tamer = runner.place_on_field(0, "RED-TAMER", Some(0));
    let _opp = runner.place_on_field(1, "OPP-DGM-3000", Some(0));

    // Sanity — start of P0's turn.
    assert_eq!(runner.game.turn_player(), 0, "test assumes P0's turn");

    // Fire OnSuspend for the red Tamer.
    runner.game.suspend(red_tamer);

    // Optional delete prompt must install.
    let pending = runner.game.pending_selection.as_ref().expect(
        "BT13-012 inherited clause must install a pending selection \
                 when an own Tamer suspends on the controller's turn",
    );
    assert!(
        pending.is_optional,
        "delete prompt is printed as 'you may delete', so optional"
    );
}

/// Negative condition gate (kind): a Digimon suspending must NOT install the
/// delete prompt. The printed text reads "When one of your red or yellow
/// **Tamers** becomes suspended" — Digimon suspends are not in scope.
#[test]
fn bt13_012_does_not_fire_when_own_digimon_suspends() {
    let mut runner = fresh_runner(vec![
        make_digimon("ALLY-DGM", 4000, CardColor::Red),
        make_digimon("OPP-DGM-3000", 3000, CardColor::Blue),
    ]);

    let _carrier = runner.place_stack(0, &["BT13-012", "TOP-DGM"]);
    let ally_dgm = runner.place_on_field(0, "ALLY-DGM", Some(0));
    let _opp = runner.place_on_field(1, "OPP-DGM-3000", Some(0));

    runner.game.suspend(ally_dgm);

    assert!(
        runner.game.pending_selection.is_none(),
        "Digimon suspending must NOT trigger the inherited Tamer-suspend clause; \
         got pending selection: {:?}",
        runner.game.pending_selection
    );
}

/// Negative condition gate (owner): the OPPONENT'S Tamer suspending must NOT
/// install the delete prompt. The printed text reads "When one of *your* red
/// or yellow Tamers becomes suspended".
#[test]
fn bt13_012_does_not_fire_when_opponent_tamer_suspends() {
    let mut runner = fresh_runner(vec![
        make_tamer("OPP-TAMER", vec![CardColor::Red]),
        make_digimon("OPP-DGM-3000", 3000, CardColor::Blue),
    ]);

    let _carrier = runner.place_stack(0, &["BT13-012", "TOP-DGM"]);
    let opp_tamer = runner.place_on_field(1, "OPP-TAMER", Some(0));
    let _opp = runner.place_on_field(1, "OPP-DGM-3000", Some(0));

    runner.game.suspend(opp_tamer);

    assert!(
        runner.game.pending_selection.is_none(),
        "Opponent's Tamer suspending must NOT trigger the inherited 'your Tamer' \
         clause; got pending selection: {:?}",
        runner.game.pending_selection
    );
}

/// Active-when gate: it must be the CONTROLLER'S turn ([Your Turn] scoping).
/// On the opponent's turn, an own Tamer suspending must NOT install the
/// prompt.
#[test]
fn bt13_012_does_not_fire_on_opponents_turn() {
    let mut runner = fresh_runner(vec![
        make_tamer("RED-TAMER", vec![CardColor::Red]),
        make_digimon("OPP-DGM-3000", 3000, CardColor::Blue),
    ]);

    let _carrier = runner.place_stack(0, &["BT13-012", "TOP-DGM"]);
    let red_tamer = runner.place_on_field(0, "RED-TAMER", Some(0));
    let _opp = runner.place_on_field(1, "OPP-DGM-3000", Some(0));

    // Switch to the opponent's turn.
    runner.game.turn_player_idx = 1;

    runner.game.suspend(red_tamer);

    assert!(
        runner.game.pending_selection.is_none(),
        "[Your Turn] active_when must gate the clause off on the opponent's turn; \
         got pending selection: {:?}",
        runner.game.pending_selection
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 4 — Clause 2 behavioral outcomes (per branch)
// ─────────────────────────────────────────────────────────────────────────────

/// Behavioral: accepting the prompt + picking the legal target deletes the
/// opponent's Digimon.
#[test]
fn bt13_012_accept_delete_removes_opponent_digimon() {
    let mut runner = fresh_runner(vec![
        make_tamer("RED-TAMER", vec![CardColor::Red]),
        make_digimon("OPP-DGM-3000", 3000, CardColor::Blue),
    ]);

    let _carrier = runner.place_stack(0, &["BT13-012", "TOP-DGM"]);
    let red_tamer = runner.place_on_field(0, "RED-TAMER", Some(0));
    let _opp = runner.place_on_field(1, "OPP-DGM-3000", Some(0));

    let opp_battle_before = runner.battle_area_size(1);
    assert_eq!(opp_battle_before, 1);

    runner.game.suspend(red_tamer);

    // Resolve by picking the first legal action (the only opp Digimon).
    let (player, action_id) = {
        let pending = runner
            .game
            .pending_selection
            .as_ref()
            .expect("delete prompt must be installed");
        (
            pending.selecting_player,
            *pending
                .valid_action_ids
                .first()
                .expect("at least one legal target"),
        )
    };
    runner
        .execute_action(player, action_id)
        .expect("delete selection resolves");

    assert_eq!(
        runner.battle_area_size(1),
        0,
        "the opponent Digimon must be deleted"
    );
}

/// Behavioral: declining the prompt leaves the opponent's Digimon in play.
#[test]
fn bt13_012_decline_leaves_opponent_digimon() {
    let mut runner = fresh_runner(vec![
        make_tamer("RED-TAMER", vec![CardColor::Red]),
        make_digimon("OPP-DGM-3000", 3000, CardColor::Blue),
    ]);

    let _carrier = runner.place_stack(0, &["BT13-012", "TOP-DGM"]);
    let red_tamer = runner.place_on_field(0, "RED-TAMER", Some(0));
    let _opp = runner.place_on_field(1, "OPP-DGM-3000", Some(0));

    runner.game.suspend(red_tamer);

    let player = runner
        .game
        .pending_selection
        .as_ref()
        .expect("optional prompt installed")
        .selecting_player;
    runner
        .execute_action(player, PASS)
        .expect("decline resolves");

    assert_eq!(
        runner.battle_area_size(1),
        1,
        "declining the optional prompt must leave the opp Digimon in play"
    );
}

/// Activation feasibility gate: when the opponent has NO Digimon, the
/// optional clause must not install a useless prompt.
///
/// Mirrors BT24-018 clause (g) "no cost target" gating — selection install
/// helpers check `has_*_candidates` first and short-circuit.
#[test]
fn bt13_012_does_not_install_prompt_when_opponent_has_no_digimon() {
    let mut runner = fresh_runner(vec![make_tamer("RED-TAMER", vec![CardColor::Red])]);

    let _carrier = runner.place_stack(0, &["BT13-012", "TOP-DGM"]);
    let red_tamer = runner.place_on_field(0, "RED-TAMER", Some(0));
    // No opponent Digimon.

    runner.game.suspend(red_tamer);

    assert!(
        runner.game.pending_selection.is_none(),
        "with no eligible targets the optional delete prompt should not install; \
         got pending selection: {:?}",
        runner.game.pending_selection
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 5 — Known gaps that block faithfulness assertions
// ─────────────────────────────────────────────────────────────────────────────

/// The printed condition is "When one of your **red or yellow** Tamers
/// becomes suspended" — a green or purple own Tamer suspending should NOT
/// trigger the clause.
///
/// G-EVENT-TARGET-COLOR resolved 2026-05-20: the `event_target_color_any_of`
/// predicate leaf compares the event-target permanent's printed color set
/// against the requested list. BT13-012's clause condition gates on
/// `event_target_color_any_of: [red, yellow]`, so a green own Tamer
/// suspending does not install the delete prompt.
#[test]
fn bt13_012_does_not_fire_for_own_green_or_purple_tamer() {
    let mut runner = fresh_runner(vec![
        make_tamer("GREEN-TAMER", vec![CardColor::Green]),
        make_digimon("OPP-DGM-3000", 3000, CardColor::Blue),
    ]);

    let _carrier = runner.place_stack(0, &["BT13-012", "TOP-DGM"]);
    let green_tamer = runner.place_on_field(0, "GREEN-TAMER", Some(0));
    let _opp = runner.place_on_field(1, "OPP-DGM-3000", Some(0));

    runner.game.suspend(green_tamer);

    assert!(
        runner.game.pending_selection.is_none(),
        "color-restricted clause must not fire for a green own Tamer suspend"
    );
}

/// Once-per-turn enforcement: two own red/yellow Tamer suspends in the same
/// turn must produce only ONE optional delete prompt.
///
/// BLOCKED on [G-OPT-TRIGGERED]: `once_per_turn: true` compiles to
/// `Effect::max_per_turn = 1` but triggered-effect OPT enforcement is not yet
/// wired in `run_queued_effect_inner`. Same caveat as BT24-018 clause (f) and
/// BT17-018 clause(s) marked in their YAML files.
#[test]
fn bt13_012_opt_lockout_blocks_second_activation_in_same_turn() {
    let mut runner = fresh_runner(vec![
        make_tamer("RED-TAMER-A", vec![CardColor::Red]),
        make_tamer("YELLOW-TAMER-B", vec![CardColor::Yellow]),
        make_digimon("OPP-DGM-A", 3000, CardColor::Blue),
        make_digimon("OPP-DGM-B", 3000, CardColor::Blue),
    ]);

    let _carrier = runner.place_stack(0, &["BT13-012", "TOP-DGM"]);
    let red = runner.place_on_field(0, "RED-TAMER-A", Some(0));
    let yel = runner.place_on_field(0, "YELLOW-TAMER-B", Some(0));
    let _o1 = runner.place_on_field(1, "OPP-DGM-A", Some(0));
    let _o2 = runner.place_on_field(1, "OPP-DGM-B", Some(0));

    // First suspend → prompt installs.
    runner.game.suspend(red);
    assert!(
        runner.game.pending_selection.is_some(),
        "first own-Tamer suspend must install the prompt"
    );
    let player = runner
        .game
        .pending_selection
        .as_ref()
        .unwrap()
        .selecting_player;
    runner
        .execute_action(player, PASS)
        .expect("decline resolves");

    // Second suspend same turn → prompt must NOT install (OPT lockout).
    runner.game.suspend(yel);
    assert!(
        runner.game.pending_selection.is_none(),
        "OPT must lock the second activation in the same turn"
    );
}

/// DP filter: the printed text limits targets to "your opponent's Digimon
/// with 3000 DP or less". A 4000 DP opp Digimon must NOT be selectable.
///
/// BLOCKED on [G-PRED-DP-LTE]: `dp_lte: 3000` is parsed and lowered but the
/// predicate evaluator does not yet enforce dp_lte on permanents in
/// non-security zones (same caveat as BT18-087 clause and BT9-112 / EX8-074
/// formula DP caps documented in qa/dsl-vocab-gaps.md).
#[test]
fn bt13_012_dp_lte_filter_excludes_high_dp_targets() {
    let mut runner = fresh_runner(vec![
        make_tamer("RED-TAMER", vec![CardColor::Red]),
        make_digimon("OPP-DGM-LOW", 3000, CardColor::Blue),
        make_digimon("OPP-DGM-HIGH", 4000, CardColor::Blue),
    ]);

    let _carrier = runner.place_stack(0, &["BT13-012", "TOP-DGM"]);
    let red_tamer = runner.place_on_field(0, "RED-TAMER", Some(0));
    let _low = runner.place_on_field(1, "OPP-DGM-LOW", Some(0));
    let _high = runner.place_on_field(1, "OPP-DGM-HIGH", Some(0));

    runner.game.suspend(red_tamer);

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("delete prompt installed");
    // Only the 3000 DP target should be selectable; the 4000 DP one must be
    // filtered out.
    assert_eq!(
        pending.valid_action_ids.len(),
        1,
        "dp_lte: 3000 must filter out the 4000 DP opp Digimon; got {:?}",
        pending.valid_action_ids
    );
}
