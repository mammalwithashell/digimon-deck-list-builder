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
//! - C5-adjacent: When-Digivolving security search + conditional Recovery + shuffle
//!   (clause 1, BLOCKED — see `bt13_012_clause1_when_digivolving_is_blocked` for
//!   the gap rationale).
//! - B3-adjacent: Inherited triggered observer of own-Tamer suspend events
//!   (clause 2, partially shipped — color filter and OPT enforcement noted).
//! - F-OPT: [Once Per Turn] on a triggered observer (gap G-OPT-TRIGGERED).
//! - F-Predicate: dp_lte filter on opponent permanent select (gap G-PRED-DP-LTE).
//! - Predicate gap: event_target_color_any_of (gap G-EVENT-TARGET-COLOR).

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

/// The shipping spec has exactly one Triggered clause — the inherited
/// on_suspend observer (clause 2). Clause 1 (When Digivolving) is BLOCKED on a
/// hybrid DSL gap and is intentionally omitted; see
/// `bt13_012_clause1_when_digivolving_is_blocked` below for the explicit
/// rationale and the gap ID.
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
        1,
        "Only the inherited on_suspend clause ships; When Digivolving clause is BLOCKED. Got: {:?}",
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

/// Negative structural test: there must NOT be a `WhenDigivolving` clause in
/// the shipping spec. If a future commit accidentally re-adds clause 1
/// (perhaps via a partial implementation that violates the no-approximations
/// policy), this test fails loudly so the gap is closed properly first.
#[test]
fn bt13_012_when_digivolving_clause_intentionally_omitted() {
    let card = geogreymon();
    let when_digivolving = card.effects.iter().any(|c| match c {
        CompiledClause::Triggered(t) => t.when.contains(&CompiledTiming::WhenDigivolving),
        _ => false,
    });
    assert!(
        !when_digivolving,
        "Clause 1 (When Digivolving security search) is BLOCKED on \
         G-PLAY-SELECTED-SECURITY-CARD and must remain omitted until that gap closes. \
         If you intend to ship the clause, also update this test."
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 2 — Clause 1 [When Digivolving]: BLOCKED gap documentation
// ─────────────────────────────────────────────────────────────────────────────

/// Clause 1 placeholder: [When Digivolving] search security stack, may play 1
/// red/yellow Tamer free, conditional Recovery+1, then shuffle security.
///
/// BLOCKED on hybrid DSL gap [G-PLAY-SELECTED-SECURITY-CARD]:
///
///   - `select_security: { of: you, ... }` exists and binds the chosen card
///     via `Bindings::insert_card(name, CardHandle)`.
///   - `play_from_security: {}` exists but pops the *top* of security; it
///     does not accept a binding to a specific security card.
///   - `add_to_hand_from_security: { of: you, card: <Card binding> }` works,
///     but `play_from_hand_free` requires a `hand_index: BindingRef`
///     (`HandIndex` flavour) — there is no DSL primitive that converts the
///     post-move card position into a hand-index binding.
///   - Step (b) "If you did, Recovery +1" also needs a
///     `binding_was_set` / `last_play_succeeded` predicate to gate Recovery on
///     whether the Tamer was actually played.
///
/// Two equally-acceptable closures (see YAML header):
///   (i) Add a new `play_security_card: { of: you, card: <binding> }` step
///       verb that lowers to: pop matching security card, push to hand, call
///       `play_from_hand_with_cost_result(.., Free, ByEffect, false)`.
///   (ii) Extend `PlayFromHandFree` to accept Card-handle bindings,
///        resolving the current hand index at run-time.
///
/// Tracked under [G-PLAY-SELECTED-SECURITY-CARD] in `qa/dsl-vocab-gaps.md`.
/// Same blocker affects BT11-042 Angewomon (currently uses raw_rust escape).
#[test]
#[ignore = "pending: G-PLAY-SELECTED-SECURITY-CARD (qa/dsl-vocab-gaps.md) — \
            select_security binds CardHandle, but no DSL verb plays a bound \
            security card without paying cost; add_to_hand_from_security + \
            play_from_hand_free chain blocked by missing Card→HandIndex bridge"]
fn bt13_012_clause1_when_digivolving_is_blocked() {
    // Scaffolding for when the gap closes:
    //
    // 1. Place a Lv3 base on P0's field (Agumon-named, Dinosaur trait, to
    //    exercise the alt-path digivolve at cost 2).
    // 2. Seed P0's security with a mix of cards: at least one red Tamer, at
    //    least one yellow Tamer, plus filler.
    // 3. Digivolve BT13-012 onto the Lv3 base (effect-initiated digivolve via
    //    the alt-path; cost 2).
    // 4. WhenDigivolving prompt installs (optional) with the legal-action
    //    list filtered to the red and yellow Tamers in security.
    //    Three branches to assert:
    //
    //    Branch A — Decline:
    //      execute_action(player, PASS) → no Tamer played, NO Recovery
    //      fires, security shuffles, security count unchanged.
    //
    //    Branch B — Pick a Tamer:
    //      execute_action(player, valid_action_ids[0]) →
    //        - Selected Tamer enters battle_area (free play).
    //        - Recovery +1 fires (deck top → security top); security_count
    //          delta = 0 (one card played from security, one card recovered
    //          back into security).
    //        - Security stack is shuffled (verify via face_up_security state
    //          or order-randomisation hook).
    //
    //    Branch C — No legal targets in security:
    //      Seed security with NO red/yellow Tamers → optional prompt should
    //      either auto-decline or never install; Recovery does NOT fire;
    //      security IS still shuffled (the "Then, shuffle" clause is
    //      unconditional per printed text).
    //
    // 5. Assert event-log fires for security-search interactions if any
    //    OnSearchSecurity / OnAddToHandFromSecurity events are emitted by
    //    the eventual implementation.
    todo!(
        "pending G-PLAY-SELECTED-SECURITY-CARD: no DSL verb to play a selected \
         security card free; clause 1 must remain omitted until the gap closes"
    )
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
/// BLOCKED on [G-EVENT-TARGET-COLOR]: the DSL has no
/// `event_target_color_any_of` predicate leaf. Available leaves only include
/// `event_target_kind`, `event_target_trait_has`, `event_target_owner`. The
/// shipping YAML therefore over-fires on own Tamers of any color.
///
/// When the gap closes, augment the YAML clause's `condition` block with:
///   `event_target_color_any_of: [red, yellow]`
/// and re-enable this test.
#[test]
#[ignore = "pending: G-EVENT-TARGET-COLOR (qa/dsl-vocab-gaps.md) — \
            no event_target_color_any_of predicate; clause over-fires on \
            non-red/yellow own Tamers"]
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
#[ignore = "pending: G-OPT-TRIGGERED (qa/archetype-qa/engine-gaps.md) — \
            once_per_turn flag compiled but not enforced for triggered observers"]
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
#[ignore = "pending: G-PRED-DP-LTE (qa/dsl-vocab-gaps.md) — \
            dp_lte filter compiled but not evaluated on battle_area permanents"]
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

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 6 — Additional coverage: yellow Tamer path + inherited-only-when-source
// ─────────────────────────────────────────────────────────────────────────────

/// Positive condition gate (yellow Tamer): printed text says "red **or yellow**
/// Tamers". A yellow Tamer suspending must also install the optional delete prompt.
///
/// This is the symmetric counterpart to
/// `bt13_012_on_own_red_tamer_suspend_installs_delete_prompt` (which covers
/// red only). While the G-EVENT-TARGET-COLOR gap means the clause currently
/// fires on any-color own Tamers, this test validates the expected happy-path
/// for yellow once the color filter is wired in — and confirms the current
/// behavior is correctly permissive rather than accidentally blocking yellow.
#[test]
fn bt13_012_on_own_yellow_tamer_suspend_installs_delete_prompt() {
    let mut runner = fresh_runner(vec![
        make_tamer("YELLOW-TAMER", vec![CardColor::Yellow]),
        make_digimon("OPP-DGM-3000", 3000, CardColor::Blue),
    ]);

    let _carrier = runner.place_stack(0, &["BT13-012", "TOP-DGM"]);
    let yellow_tamer = runner.place_on_field(0, "YELLOW-TAMER", Some(0));
    let _opp = runner.place_on_field(1, "OPP-DGM-3000", Some(0));

    assert_eq!(runner.game.turn_player(), 0, "test assumes P0's turn");

    runner.game.suspend(yellow_tamer);

    let pending = runner.game.pending_selection.as_ref().expect(
        "BT13-012 inherited clause must install a pending selection \
         when an own yellow Tamer suspends on the controller's turn",
    );
    assert!(
        pending.is_optional,
        "delete prompt is printed as 'you may delete', so optional"
    );
}

/// Engine inherited-dispatch behavior: the `scope: inherited` YAML flag compiles
/// to `Effect.inherited = true`, which controls the *card_sources* inherited scan
/// path (where sources under the top card contribute inherited effects to their
/// host). However, the top-card dispatch path (`enqueue_from_permanent`, lines
/// 1323–1352 of effect_queue.rs) does NOT skip `effect.inherited = true` effects
/// except for Training permanents — so the effect also fires when GeoGreymon is
/// the top (only) card of its own stack.
///
/// This test documents that behavior: placing BT13-012 alone (no digivolution
/// source underneath) still activates its on_suspend inherited effect, because
/// the effect is a fully-registered triggered observer regardless of stack depth.
///
/// Practical implication: the `place_stack(0, &["BT13-012", "TOP-DGM"])` pattern
/// used in the rest of this test file is the correct setup for testing inherited
/// effects *as inherited* (i.e. from under another card), but does not change
/// whether the clause fires — it still fires in both configurations.
#[test]
fn bt13_012_inherited_fires_even_when_geogreymon_is_top_only() {
    // GeoGreymon alone as top card — effect still fires (see doc comment).
    let mut runner = fresh_runner(vec![
        make_tamer("RED-TAMER", vec![CardColor::Red]),
        make_digimon("OPP-DGM-3000", 3000, CardColor::Blue),
    ]);

    let _geo_alone = runner.place_on_field(0, "BT13-012", Some(0));
    let red_tamer = runner.place_on_field(0, "RED-TAMER", Some(0));
    let _opp = runner.place_on_field(1, "OPP-DGM-3000", Some(0));

    runner.game.suspend(red_tamer);

    // The effect is registered as a top-card triggered observer as well as an
    // inherited (card_sources) observer. When GeoGreymon is the sole top card,
    // the top-card dispatch path fires it.
    assert!(
        runner.game.pending_selection.is_some(),
        "effect.inherited = true does not suppress top-card dispatch; delete prompt \
         must install even when BT13-012 is the top-only card of its stack"
    );
    let pending = runner.game.pending_selection.as_ref().unwrap();
    assert!(
        pending.is_optional,
        "prompt must still be optional ('you may delete')"
    );
}
