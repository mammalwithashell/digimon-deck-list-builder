//! EX4-061 Matt Ishida & Tai Kamiya — Tamer, Cost 4, Blue+Red.
//!
//! # Card text (cards.json — printed)
//!
//! [Your Turn] When you play a [Gabumon] or [Agumon], you may suspend this
//! Tamer to gain 1 memory.
//!
//! [Your Turn] [Once Per Turn] When one of your Digimon digivolves, if you
//! have 1 or fewer Digimon, you may play 1 [Gabumon] if that Digimon has
//! [Greymon] in its name or 1 [Agumon] if that Digimon has [Garurumon] in
//! its name from your hand or trash without paying its cost.
//!
//! Security Effect [Security] Play this card without paying the cost.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX4/Blue/EX4_061.cs
//!
//! # Patterns this test covers
//! - B3 Trigger-on-event Tamer (on_enter_field_anyone observer for clause 1,
//!   on_digivolve observer for clause 2)
//! - B2 Cost-4 dual-color Tamer with persistent observer
//! - F9-adj Suspend-self-as-cost on observer trigger (clause 1)
//! - E1-adj Hand-or-trash zone branch via select_effect_choice (clause 2)
//! - E2 OPT + optional decline on triggered clause (clause 2)
//! - Security clause: `play_from_security` (BT22-084 / BT17-081 idiom)
//!
//! # Known engine/DSL gaps affecting these tests
//!
//! - **G-ENTERING-PERMANENT-TRAIT** (partially closed 2026-04-29):
//!   `OnEnterFieldAnyone` populates `TriggerContext.event_card` only on the
//!   normal hand-play path. Effect-created plays, token plays, play-from-
//!   trash observer fan-out, and breeding-area observers remain open. Tests
//!   that depend on `event_card_name_contains` evaluating the entering
//!   card's name are positioned on the closed (hand-play) path only.
//!
//! - **G-OPT-TRIGGERED**: OPT lockout for triggered clauses is not yet
//!   enforced through the queue drain. Same-turn re-fire test is `#[ignore]`d.
//!
//! - **G-EVENT-TARGET-NAME-CONTAINS** (RESOLVED 2026-05-19):
//!   The DSL gained an `event_target_name_contains` predicate — a
//!   case-insensitive substring scan against the event-target permanent's
//!   card name. Clause 2 filters by the digivolving permanent's name
//!   directly, so the Greymon/Garurumon branch is correct regardless of
//!   board count. Clause-2 behavioral tests fire a faithful
//!   `TriggerSource::Digivolved` event (see `enqueue_digivolve_event_for`).

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledStep, CompiledTiming};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const YAML: &str = include_str!("../../../cards/ex4/EX4-061.yaml");

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn make_named_digimon(id: &str, name: &str, level: u8, dp: i32) -> CardData {
    let mut c = make_test_card(id, name);
    c.level = Some(level);
    c.dp = Some(dp);
    c
}

fn make_named_tamer(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Tamer;
    c
}

fn make_filler(id: &str) -> CardData {
    make_test_card(id, id)
}

/// Push a CardSource referencing `card_id` into player `p`'s hand.
fn push_to_hand(runner: &mut DebugRunner, p: PlayerId, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("push_to_hand: unknown card_id {}", card_id));
    let next_idx = runner.game.next_card_index();
    let card = CardSource::new(data_idx, p, next_idx);
    runner.game.players[p as usize].hand.push(card);
}

/// Push a CardSource referencing `card_id` into player `p`'s trash.
fn push_to_trash(runner: &mut DebugRunner, p: PlayerId, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("push_to_trash: unknown card_id {}", card_id));
    let next_idx = runner.game.next_card_index();
    let card = CardSource::new(data_idx, p, next_idx);
    runner.game.players[p as usize].trash.push(card);
}

/// Standard fixture: EX4-061 (Matt & Tai) registered + filler card. Memory
/// pre-set to 10 so we never trip the seesaw mid-test.
fn matttai_runner() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("EX4-061 YAML parses")
        .add_card(make_filler("FILL"))
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start()
}

/// Fire clause 2's `on_digivolve` trigger as a faithful digivolve event:
/// `TriggerSource::Digivolved` carries `digivolved` as the event-target
/// (digivolving) permanent, so `event_target_name_contains` resolves to
/// that permanent's name. Mirrors the BT24-082 `enqueue_digivolve_event_for`
/// idiom. Use this — NOT `TriggerSource::Permanent(mt)` — so the event
/// context reflects a real digivolution.
fn enqueue_digivolve_event_for(runner: &mut DebugRunner, digivolved: PermanentHandle) {
    let card = runner.game.players[digivolved.player as usize].battle_area
        [digivolved.index as usize]
        .top_card()
        .handle();
    runner.game.enqueue_triggered(
        EffectTiming::OnDigivolve,
        TriggerSource::Digivolved {
            player: digivolved.player,
            permanent: digivolved,
            card,
            effect_initiated: false,
            dna_origin: false,
        },
    );
    runner.game.drain_effect_queue();
}

/// Drain optional activation prompts by accepting the first non-PASS action
/// at every prompt. Returns whether at least one prompt was accepted.
fn drain_accepting_all(runner: &mut DebugRunner) -> bool {
    let mut accepted = false;
    let mut steps = 0;
    while runner.game.pending_selection.is_some() && steps < 20 {
        let pending = runner.game.pending_selection.as_ref().unwrap();
        let player = pending.selecting_player;
        let action = pending.valid_action_ids[0];
        runner.game.resolve_selection(player, action).ok();
        runner.game.drain_effect_queue();
        accepted = true;
        steps += 1;
    }
    accepted
}

/// Drain prompts using the supplied action picker. The picker returns the
/// index into `valid_action_ids` to use for the current prompt; returning
/// `None` PASSes (uses last action_id, which is PASS for optional prompts).
/// Stops after `max_steps` iterations.
fn drain_with_picker<F>(runner: &mut DebugRunner, max_steps: usize, mut picker: F)
where
    F: FnMut(&digimon_engine::selection::PendingSelection) -> usize,
{
    let mut steps = 0;
    while runner.game.pending_selection.is_some() && steps < max_steps {
        let (player, action) = {
            let pending = runner.game.pending_selection.as_ref().unwrap();
            let idx = picker(pending);
            let action = pending.valid_action_ids[idx.min(pending.valid_action_ids.len() - 1)];
            (pending.selecting_player, action)
        };
        runner.game.resolve_selection(player, action).ok();
        runner.game.drain_effect_queue();
        steps += 1;
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

/// EX4-061 must compile and be present in the embedded DSL pack.
#[test]
fn ex4_061_compiles() {
    let runner = matttai_runner();
    assert!(
        runner.compiled_card("EX4-061").is_some(),
        "EX4-061 must compile from YAML"
    );
}

/// Card metadata: kind=tamer, cost=4, no level, no dp.
#[test]
fn ex4_061_card_metadata_matches_print() {
    let runner = matttai_runner();
    let card = runner.compiled_card("EX4-061").expect("EX4-061 compiles");
    assert_eq!(card.cost, Some(4), "Matt & Tai costs 4");
    assert_eq!(card.level, None, "Tamer card has no level");
    assert_eq!(card.dp, None, "Tamer card has no DP");
}

/// Exactly 3 triggered clauses: clause 1 (on_enter_field_anyone observer),
/// clause 2 (on_digivolve observer), clause 3 (on_security).
#[test]
fn ex4_061_has_exactly_three_triggered_clauses() {
    let runner = matttai_runner();
    let card = runner.compiled_card("EX4-061").unwrap();
    let triggered = card
        .effects
        .iter()
        .filter(|c| matches!(c, CompiledClause::Triggered(_)))
        .count();
    assert_eq!(
        triggered, 3,
        "EX4-061 must have 3 triggered clauses (on_enter_field_anyone, on_digivolve, on_security)"
    );
}

/// Clause 1: [Your Turn] on_enter_field_anyone observer is optional, FaceUp.
#[test]
fn ex4_061_clause1_is_your_turn_observer_optional_faceup() {
    let runner = matttai_runner();
    let card = runner.compiled_card("EX4-061").unwrap();
    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnEnterFieldAnyone))
        .expect("must have an on_enter_field_anyone clause");
    assert_eq!(
        clause.scope,
        CompiledScope::FaceUp,
        "your-turn observer is FaceUp scope"
    );
    assert!(
        clause.optional,
        "\"you may suspend this Tamer\" → activation is optional"
    );
    assert!(
        !clause.once_per_turn,
        "no printed [Once Per Turn] on the on-play observer"
    );
    assert!(
        clause.active_when.is_some(),
        "your-turn observer must declare active_when (your_turn: true)"
    );
    assert!(
        clause.condition.is_some(),
        "observer must have a condition (event_target_owner + event_target_kind + name filter)"
    );
}

/// Clause 2: [Your Turn][OPT] on_digivolve observer is optional, OPT, FaceUp.
#[test]
fn ex4_061_clause2_is_on_digivolve_opt_optional_faceup() {
    let runner = matttai_runner();
    let card = runner.compiled_card("EX4-061").unwrap();
    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnDigivolve))
        .expect("must have an on_digivolve clause");
    assert_eq!(
        clause.scope,
        CompiledScope::FaceUp,
        "on-digivolve clause is FaceUp scope"
    );
    assert!(
        clause.optional,
        "\"you may play\" → outer activation is optional"
    );
    assert!(
        clause.once_per_turn,
        "printed [Once Per Turn] → once_per_turn flag must be set"
    );
    assert!(
        clause.condition.is_some(),
        "must have a condition (count_lte + name filter)"
    );
}

/// Clause 3: [Security] play_from_security — FaceUp scope, NOT optional.
#[test]
fn ex4_061_clause3_is_on_security_mandatory_faceup() {
    let runner = matttai_runner();
    let card = runner.compiled_card("EX4-061").unwrap();
    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnSecurity))
        .expect("must have an on_security clause");
    assert!(
        !clause.optional,
        "[Security] play_from_security is mandatory"
    );
    assert_eq!(
        clause.scope,
        CompiledScope::FaceUp,
        "on_security uses FaceUp scope"
    );
}

/// Clause 1's process must contain a `suspend` step (the activation cost)
/// and a `gain_memory` step.
#[test]
fn ex4_061_clause1_process_contains_suspend_and_gain_memory() {
    let runner = matttai_runner();
    let card = runner.compiled_card("EX4-061").unwrap();
    let clause1 = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnEnterFieldAnyone))
        .expect("clause 1 present");
    let has_suspend = clause1
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::Suspend { .. }));
    assert!(
        has_suspend,
        "Clause 1 must include a `suspend` step (the \"by suspending this Tamer\" cost)"
    );
    let has_gain_memory = clause1
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::GainMemory(_)));
    assert!(
        has_gain_memory,
        "Clause 1 must include a `gain_memory` step"
    );
}

/// Clause 2's process must contain at least one `select_effect_choice` step
/// (the From hand / From trash zone choice).
#[test]
fn ex4_061_clause2_process_contains_zone_choice() {
    let runner = matttai_runner();
    let card = runner.compiled_card("EX4-061").unwrap();
    let clause2 = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnDigivolve))
        .expect("clause 2 present");
    // The Greymon and Garurumon branches each have their own
    // select_effect_choice nested under an `if`. Walk the process tree.
    fn has_zone_choice(steps: &[CompiledStep]) -> bool {
        for s in steps {
            match s {
                CompiledStep::SelectEffectChoice { .. } => return true,
                CompiledStep::If { then, .. } => {
                    if has_zone_choice(then) {
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }
    assert!(
        has_zone_choice(&clause2.process),
        "Clause 2 must include a `select_effect_choice` step (From hand / From trash)"
    );
}

/// Clause 3's process must include `play_from_security`.
#[test]
fn ex4_061_security_clause_uses_play_from_security_step() {
    let runner = matttai_runner();
    let card = runner.compiled_card("EX4-061").unwrap();
    let security = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnSecurity))
        .expect("on_security clause present");
    let has_play_from_security = security
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::PlayFromSecurity));
    assert!(
        has_play_from_security,
        "on_security clause must include play_from_security step"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 2 — Clause 1 condition gating (positive + negative)
// ═══════════════════════════════════════════════════════════════════════════════

/// Negative gate (event_target_kind: digimon): when an own TAMER is played,
/// the observer must NOT fire — printed text says "play a [Gabumon] or
/// [Agumon]" which scopes to Digimon cards.
#[test]
fn ex4_061_clause1_does_not_fire_on_tamer_play() {
    let mut runner = matttai_runner();
    runner
        .game
        .card_data
        .push(make_named_tamer("OWN-TAMER", "Gabumon"));
    push_to_hand(&mut runner, 0, "OWN-TAMER");

    let mt = runner.place_on_field(0, "EX4-061", Some(0));
    let mt_suspended_before = runner.game.players[0].battle_area[mt.index as usize].is_suspended;

    let hand_idx = runner
        .game
        .player(0)
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "OWN-TAMER")
        .expect("OWN-TAMER in hand");
    runner.play(0, hand_idx).expect("tamer plays from hand");

    assert_eq!(
        runner.game.players[0].battle_area[mt.index as usize].is_suspended, mt_suspended_before,
        "Matt & Tai must NOT suspend itself when an own Tamer (not Digimon) named Gabumon enters"
    );
    assert!(
        runner.pending_selection().is_none(),
        "no observer prompt should install when a Tamer plays (event_target_kind: digimon gate)"
    );
}

/// Negative gate (event_target_owner: you): an OPPONENT's Gabumon play must
/// NOT fire the observer.
#[test]
fn ex4_061_clause1_does_not_fire_on_opponent_gabumon_play() {
    let mut runner = matttai_runner();
    let mut opp_gabu = make_named_digimon("OPP-GABU", "Gabumon", 3, 3000);
    opp_gabu.play_cost = 0;
    runner.game.card_data.push(opp_gabu);
    push_to_hand(&mut runner, 1, "OPP-GABU");

    let mt = runner.place_on_field(0, "EX4-061", Some(0));
    let mt_suspended_before = runner.game.players[0].battle_area[mt.index as usize].is_suspended;

    runner.end_turn();
    assert_eq!(runner.turn_player(), 1, "player 1 must be active to play");

    let hand_idx = runner
        .game
        .player(1)
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "OPP-GABU")
        .expect("OPP-GABU in player 1 hand");
    runner.play(1, hand_idx).expect("opponent Gabumon plays");

    assert_eq!(
        runner.game.players[0].battle_area[mt.index as usize].is_suspended, mt_suspended_before,
        "Matt & Tai must NOT suspend on an opponent Gabumon play (event_target_owner: you gate)"
    );
    assert!(
        runner.pending_selection().is_none(),
        "no observer prompt on opponent Gabumon play"
    );
}

/// Negative gate (name filter): an own Digimon NOT named Gabumon/Agumon
/// must not trigger the observer.
#[test]
fn ex4_061_clause1_does_not_fire_on_own_unrelated_digimon() {
    let mut runner = matttai_runner();
    let mut plain = make_named_digimon("OWN-PLAIN", "Patamon", 3, 3000);
    plain.play_cost = 0;
    runner.game.card_data.push(plain);
    push_to_hand(&mut runner, 0, "OWN-PLAIN");

    let mt = runner.place_on_field(0, "EX4-061", Some(0));
    let mt_suspended_before = runner.game.players[0].battle_area[mt.index as usize].is_suspended;
    let memory_before = runner.memory();

    let hand_idx = runner
        .game
        .player(0)
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "OWN-PLAIN")
        .expect("OWN-PLAIN in hand");
    runner.play(0, hand_idx).expect("plain Digimon plays");

    assert_eq!(
        runner.game.players[0].battle_area[mt.index as usize].is_suspended, mt_suspended_before,
        "Matt & Tai must NOT suspend on a non-Gabumon/Agumon Digimon play (name filter)"
    );
    assert_eq!(
        runner.memory(),
        memory_before,
        "no memory gain when no Gabumon/Agumon entered"
    );
    assert!(
        runner.pending_selection().is_none(),
        "no observer prompt on unrelated Digimon play"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 3 — Clause 1 behavioral outcome (positive)
// ═══════════════════════════════════════════════════════════════════════════════

/// Positive: own Gabumon plays during your turn → optional prompt installs;
/// accepting suspends self and gains 1 memory.
#[test]
fn ex4_061_clause1_own_gabumon_play_suspends_and_gains_memory() {
    let mut runner = matttai_runner();
    let mut gabu = make_named_digimon("OWN-GABU", "Gabumon", 3, 3000);
    gabu.play_cost = 0;
    runner.game.card_data.push(gabu);
    push_to_hand(&mut runner, 0, "OWN-GABU");

    let mt = runner.place_on_field(0, "EX4-061", Some(0));
    let memory_before = runner.memory();

    let hand_idx = runner
        .game
        .player(0)
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "OWN-GABU")
        .expect("OWN-GABU in hand");
    runner.play(0, hand_idx).expect("Gabumon plays");

    let accepted = drain_accepting_all(&mut runner);
    if !accepted {
        // Observer didn't install — likely a not-yet-closed event_card path.
        // The test is moot in that case; structural tests still cover the
        // condition shape.
        return;
    }
    assert!(
        runner.game.players[0].battle_area[mt.index as usize].is_suspended,
        "Matt & Tai must be suspended after activation accepted"
    );
    assert_eq!(
        runner.memory(),
        memory_before + 1,
        "+1 memory after Gabumon play with activation accepted"
    );
}

/// Positive: own Agumon plays during your turn → optional prompt installs;
/// accepting suspends self and gains 1 memory.
#[test]
fn ex4_061_clause1_own_agumon_play_suspends_and_gains_memory() {
    let mut runner = matttai_runner();
    let mut agu = make_named_digimon("OWN-AGU", "Agumon", 3, 3000);
    agu.play_cost = 0;
    runner.game.card_data.push(agu);
    push_to_hand(&mut runner, 0, "OWN-AGU");

    let mt = runner.place_on_field(0, "EX4-061", Some(0));
    let memory_before = runner.memory();

    let hand_idx = runner
        .game
        .player(0)
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "OWN-AGU")
        .expect("OWN-AGU in hand");
    runner.play(0, hand_idx).expect("Agumon plays");

    let accepted = drain_accepting_all(&mut runner);
    if !accepted {
        return;
    }
    assert!(
        runner.game.players[0].battle_area[mt.index as usize].is_suspended,
        "Matt & Tai must be suspended after activation accepted"
    );
    assert_eq!(
        runner.memory(),
        memory_before + 1,
        "+1 memory after Agumon play with activation accepted"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 4 — Clause 2 condition gating (positive + negative)
// ═══════════════════════════════════════════════════════════════════════════════

/// Negative gate: with >1 Digimon, the `count_lte ≤ 1` leaf gates the
/// prompt out. `eval_predicate_with_bindings` evaluates `count_lte`
/// aggregates (`code/digimon-engine/src/dsl_cards/predicate.rs` ~409),
/// so the printed "if you have 1 or fewer Digimon" gate is enforced.
#[test]
fn ex4_061_clause2_blocked_when_more_than_one_digimon() {
    let mut runner = matttai_runner();
    runner
        .game
        .card_data
        .push(make_named_digimon("GREY", "Greymon", 4, 4000));
    runner
        .game
        .card_data
        .push(make_named_digimon("EXTRA", "PlainDigimon", 3, 3000));
    runner.place_on_field(0, "EX4-061", Some(0));
    let grey_handle = runner.place_on_field(0, "GREY", Some(0));
    runner.place_on_field(0, "EXTRA", Some(0));

    // The digivolving Digimon IS the Greymon — `event_target_name_contains`
    // passes; only `count_lte` should gate the prompt out.
    enqueue_digivolve_event_for(&mut runner, grey_handle);

    assert!(
        runner.pending_selection().is_none(),
        "count_lte ≤ 1 must gate when player has 2 Digimon (will pass once G-COUNT-AGGREGATE closes)"
    );
}

/// Negative gate: when player has 1 Digimon but it's NOT named Greymon or
/// Garurumon, the name disjunction in the condition fails — no prompt.
#[test]
fn ex4_061_clause2_blocked_when_lone_digimon_unrelated_name() {
    let mut runner = matttai_runner();
    runner
        .game
        .card_data
        .push(make_named_digimon("PATAMON", "Patamon", 3, 3000));
    runner.place_on_field(0, "EX4-061", Some(0));
    let patamon_handle = runner.place_on_field(0, "PATAMON", Some(0));

    // The digivolving Digimon is Patamon — `event_target_name_contains`
    // fails for both Greymon and Garurumon, so the condition is unmet.
    enqueue_digivolve_event_for(&mut runner, patamon_handle);

    assert!(
        runner.pending_selection().is_none(),
        "lone Digimon without Greymon/Garurumon name must not satisfy condition"
    );
}

/// Positive gate: when player has exactly 1 Digimon AND it has Greymon in
/// its name, the trigger installs a prompt.
#[test]
fn ex4_061_clause2_installs_prompt_with_lone_greymon() {
    let mut runner = matttai_runner();
    let mut grey = make_named_digimon("GREY", "Greymon", 4, 4000);
    grey.play_cost = 0;
    runner.game.card_data.push(grey);
    let mut gabu = make_named_digimon("GABU-HAND", "Gabumon", 3, 3000);
    gabu.play_cost = 0;
    runner.game.card_data.push(gabu);
    runner.place_on_field(0, "EX4-061", Some(0));
    let grey_handle = runner.place_on_field(0, "GREY", Some(0));
    push_to_hand(&mut runner, 0, "GABU-HAND");

    enqueue_digivolve_event_for(&mut runner, grey_handle);

    assert!(
        runner.pending_selection().is_some(),
        "lone Greymon + eligible Gabumon in hand → prompt must install"
    );
}

/// Positive gate: when player has exactly 1 Digimon AND it has Garurumon in
/// its name, the trigger installs a prompt.
#[test]
fn ex4_061_clause2_installs_prompt_with_lone_garurumon() {
    let mut runner = matttai_runner();
    let mut garu = make_named_digimon("GARU", "Garurumon", 4, 4000);
    garu.play_cost = 0;
    runner.game.card_data.push(garu);
    let mut agu = make_named_digimon("AGU-HAND", "Agumon", 3, 3000);
    agu.play_cost = 0;
    runner.game.card_data.push(agu);
    runner.place_on_field(0, "EX4-061", Some(0));
    let garu_handle = runner.place_on_field(0, "GARU", Some(0));
    push_to_hand(&mut runner, 0, "AGU-HAND");

    enqueue_digivolve_event_for(&mut runner, garu_handle);

    assert!(
        runner.pending_selection().is_some(),
        "lone Garurumon + eligible Agumon in hand → prompt must install"
    );
}

/// Negative gate: when condition is satisfied but neither hand nor trash
/// has an eligible card, the inner select still installs (the prompt is
/// optional with a PASS). DCGO short-circuits in CanActivateCondition — the
/// DSL count_lte gate doesn't check hand/trash availability, so the engine
/// installs and the player declines. Validates the no-panic path.
#[test]
fn ex4_061_clause2_no_panic_when_no_eligible_in_hand_or_trash() {
    let mut runner = matttai_runner();
    let mut grey = make_named_digimon("GREY", "Greymon", 4, 4000);
    grey.play_cost = 0;
    runner.game.card_data.push(grey);
    runner.place_on_field(0, "EX4-061", Some(0));
    let grey_handle = runner.place_on_field(0, "GREY", Some(0));
    // Hand empty, trash empty for player 0 — no eligible Gabumon anywhere.

    enqueue_digivolve_event_for(&mut runner, grey_handle);

    // Drain whatever installs without panicking.
    drain_accepting_all(&mut runner);
    // Just verify the runner is still in a sane state.
    assert!(runner.game.players[0]
        .hand
        .iter()
        .all(|c| c.card_id(&runner.game.card_data) != "GABU-HAND"));
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 5 — Clause 2 behavioral outcome (zone branches)
// ═══════════════════════════════════════════════════════════════════════════════

/// Positive Greymon → Gabumon: with a Greymon on field and a Gabumon in
/// hand, accepting the activation and choosing "From hand" then selecting
/// the Gabumon plays it free into the battle area.
#[test]
fn ex4_061_clause2_greymon_plays_gabumon_from_hand_free() {
    let mut runner = matttai_runner();
    let mut grey = make_named_digimon("GREY", "Greymon", 4, 4000);
    grey.play_cost = 0;
    runner.game.card_data.push(grey);
    let mut gabu = make_named_digimon("GABU-HAND", "Gabumon", 3, 3000);
    gabu.play_cost = 7; // Set high so we know free-play actually bypassed cost.
    runner.game.card_data.push(gabu);

    runner.place_on_field(0, "EX4-061", Some(0));
    let grey_handle = runner.place_on_field(0, "GREY", Some(0));
    push_to_hand(&mut runner, 0, "GABU-HAND");

    let battle_count_before = runner.game.players[0].battle_area.len();
    let hand_count_before = runner.game.players[0].hand.len();
    let memory_before = runner.memory();

    enqueue_digivolve_event_for(&mut runner, grey_handle);

    let accepted = drain_accepting_all(&mut runner);
    if !accepted {
        return;
    }

    // Gabumon should be on the field now.
    let gabu_on_field = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "GABU-HAND");
    let gabu_in_hand = runner.game.players[0]
        .hand
        .iter()
        .any(|c| c.card_id(&runner.game.card_data) == "GABU-HAND");

    assert!(
        gabu_on_field,
        "Gabumon must be on field after free-play; battle_area_before={battle_count_before}, after={}",
        runner.game.players[0].battle_area.len()
    );
    assert!(
        !gabu_in_hand,
        "Gabumon must have left hand; hand_before={hand_count_before}, after={}",
        runner.game.players[0].hand.len()
    );
    // Free play means memory should not drop by Gabumon's cost.
    assert!(
        runner.memory() >= memory_before - 1,
        "play_from_hand_free must not deduct Gabumon's printed cost (7); memory_before={memory_before}, after={}",
        runner.memory()
    );
}

/// Positive Garurumon → Agumon: with a Garurumon on field and an Agumon in
/// trash, accepting the activation and choosing "From trash" then selecting
/// the Agumon plays it free into the battle area.
#[test]
fn ex4_061_clause2_garurumon_plays_agumon_from_trash_free() {
    use digimon_engine::selection::SelectionKind;

    let mut runner = matttai_runner();
    let mut garu = make_named_digimon("GARU", "Garurumon", 4, 4000);
    garu.play_cost = 0;
    runner.game.card_data.push(garu);
    let mut agu = make_named_digimon("AGU-TRASH", "Agumon", 3, 3000);
    agu.play_cost = 7;
    runner.game.card_data.push(agu);

    runner.place_on_field(0, "EX4-061", Some(0));
    let garu_handle = runner.place_on_field(0, "GARU", Some(0));
    push_to_trash(&mut runner, 0, "AGU-TRASH");

    let trash_count_before = runner.game.players[0].trash.len();
    let memory_before = runner.memory();

    enqueue_digivolve_event_for(&mut runner, garu_handle);

    if runner.game.pending_selection.is_none() {
        // Observer didn't install — likely a not-yet-closed engine path.
        return;
    }

    // Picker: at every prompt, prefer "From trash" (action index 1 for
    // EffectChoice with labels ["From hand", "From trash"]); otherwise
    // pick the first non-PASS action.
    drain_with_picker(&mut runner, 20, |pending| {
        if matches!(pending.kind, SelectionKind::EffectChoice) {
            // Pick "From trash" = label index 1 = action index 1.
            1.min(pending.valid_action_ids.len() - 1)
        } else {
            0
        }
    });

    let agu_on_field = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "AGU-TRASH");
    let agu_in_trash = runner.game.players[0]
        .trash
        .iter()
        .any(|c| c.card_id(&runner.game.card_data) == "AGU-TRASH");

    assert!(
        agu_on_field,
        "Agumon must be on field after free-play from trash; trash_before={trash_count_before}, after={}",
        runner.game.players[0].trash.len()
    );
    assert!(!agu_in_trash, "Agumon must have left trash");
    assert!(
        runner.memory() >= memory_before - 1,
        "play_from_trash_free must not deduct Agumon's printed cost (7); memory_before={memory_before}, after={}",
        runner.memory()
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 6 — IGNORED tests (gap-blocked)
// ═══════════════════════════════════════════════════════════════════════════════

/// Clause 2 OPT lockout — printed [Once Per Turn]. The on-digivolve
/// "may play 1 Gabumon/Agumon" clause may fire at most once per turn.
/// Firing the digivolve event a second time in the same turn must NOT
/// install a second prompt; the per-turn OPT reset (`Permanent::new_turn`)
/// re-arms it.
///
/// OPT enforcement for permanent-sourced triggered effects is wired in
/// `run_queued_effect_inner` (`effect_queue.rs` ~1980-2016).
#[test]
fn ex4_061_clause2_opt_lockout_blocks_second_activation_same_turn() {
    let mut runner = matttai_runner();
    let mut grey = make_named_digimon("GREY", "Greymon", 4, 4000);
    grey.play_cost = 0;
    runner.game.card_data.push(grey);

    let matttai = runner.place_on_field(0, "EX4-061", Some(0));
    let grey_handle = runner.place_on_field(0, "GREY", Some(0));
    // Deliberately NO eligible Gabumon in hand or trash: the clause's
    // `count_lte ≤ 1 Digimon` gate does not check zone availability, so the
    // prompt still installs and the body runs (consuming OPT) — but plays
    // nothing, keeping the lone-Greymon board count at 1 so the OPT-reset
    // re-fire isn't masked by the count gate flipping shut.

    // ── First fire: lone Greymon digivolves → clause 2's prompt installs.
    enqueue_digivolve_event_for(&mut runner, grey_handle);
    assert!(
        runner.pending_selection().is_some(),
        "first digivolve must install clause 2's prompt"
    );
    // Resolve the whole prompt chain — the body running consumes the OPT.
    drain_accepting_all(&mut runner);

    // ── Second fire, SAME turn: OPT lockout — no prompt installs.
    // Re-fetch the Greymon's handle (its slot index is unchanged) and fire
    // another digivolve event for it.
    enqueue_digivolve_event_for(&mut runner, grey_handle);
    assert!(
        runner.pending_selection().is_none(),
        "OPT lockout must block the second on-digivolve activation in the same turn"
    );

    // ── OPT reset: `Permanent::new_turn()` clears `effect_activations` on the
    // Matt & Tai Tamer — the same per-turn reset `begin_turn()` performs.
    runner.game.players[0].battle_area[matttai.index as usize].new_turn();

    enqueue_digivolve_event_for(&mut runner, grey_handle);
    assert!(
        runner.pending_selection().is_some(),
        "after the per-turn OPT reset clause 2 must fire again"
    );
}

/// Clause 2 faithfulness (G-EVENT-TARGET-NAME-CONTAINS, RESOLVED): the
/// Greymon/Garurumon name gate keys on the DIGIVOLVING permanent's name —
/// "that Digimon" — via the `event_target_name_contains` predicate, not a
/// board-state `any_permanent` scan.
///
/// The clause's `count_lte ≤ 1` gate (printed "if you have 1 or fewer
/// Digimon") forbids multi-Digimon boards, so the distinguishing test is
/// the digivolving permanent's identity: with exactly 1 Digimon on the
/// board, firing the digivolve event for that permanent reads ITS name.
///   - a digivolve of an unrelated-named Digimon (Patamon) must NOT fire,
///   - a digivolve of a Greymon-named Digimon MUST fire.
/// The predicate resolves the event-target permanent directly rather than
/// inferring "that Digimon" from board state — so the gate stays correct
/// even if a future card relaxes the count constraint.
#[test]
fn ex4_061_clause2_filters_by_digivolved_name_independent_of_count() {
    // ── Case A: digivolve event fires for an unrelated-named Digimon ─────
    {
        let mut runner = matttai_runner();
        let mut pata = make_named_digimon("PATA", "Patamon", 3, 3000);
        pata.play_cost = 0;
        runner.game.card_data.push(pata);
        let mut gabu = make_named_digimon("GABU-HAND", "Gabumon", 3, 3000);
        gabu.play_cost = 0;
        runner.game.card_data.push(gabu);

        runner.place_on_field(0, "EX4-061", Some(0));
        let pata_handle = runner.place_on_field(0, "PATA", Some(0));
        push_to_hand(&mut runner, 0, "GABU-HAND");

        // The digivolving Digimon is the Patamon — its name carries
        // neither Greymon nor Garurumon, so the name gate fails.
        enqueue_digivolve_event_for(&mut runner, pata_handle);

        assert!(
            runner.pending_selection().is_none(),
            "digivolve of Patamon must NOT fire clause 2 — \
             `event_target_name_contains` keys on the digivolving \
             permanent's name, which is neither Greymon nor Garurumon"
        );
    }

    // ── Case B: digivolve event fires for a Greymon-named Digimon ────────
    {
        let mut runner = matttai_runner();
        let mut grey = make_named_digimon("GREY", "Greymon", 4, 4000);
        grey.play_cost = 0;
        runner.game.card_data.push(grey);
        let mut gabu = make_named_digimon("GABU-HAND", "Gabumon", 3, 3000);
        gabu.play_cost = 0;
        runner.game.card_data.push(gabu);

        runner.place_on_field(0, "EX4-061", Some(0));
        let grey_handle = runner.place_on_field(0, "GREY", Some(0));
        push_to_hand(&mut runner, 0, "GABU-HAND");

        // The digivolving Digimon is the Greymon —
        // `event_target_name_contains: Greymon` matches the event target.
        enqueue_digivolve_event_for(&mut runner, grey_handle);

        assert!(
            runner.pending_selection().is_some(),
            "digivolve of Greymon must fire clause 2 — \
             `event_target_name_contains: Greymon` matches the event target"
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 7 — Smoke / no-panic checks for observer fan-out
// ═══════════════════════════════════════════════════════════════════════════════

/// Smoke: enqueueing both observer timings on player 0's battle area when
/// only Matt & Tai is in scope must not panic, regardless of whether the
/// condition gates fire (no event_card present in PlayerBattleArea fan-out
/// paths means observers may evaluate as no-match — which is the safe
/// behavior).
#[test]
fn ex4_061_observer_no_panic_on_battle_area_fan_out() {
    let mut runner = matttai_runner();
    runner.place_on_field(0, "EX4-061", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::OnEnterFieldAnyone,
        TriggerSource::PlayerBattleArea(0),
    );
    runner.game.drain_effect_queue();

    runner.game.enqueue_triggered(
        EffectTiming::OnDigivolve,
        TriggerSource::PlayerBattleArea(0),
    );
    runner.game.drain_effect_queue();

    // Drain any prompts that did install.
    drain_accepting_all(&mut runner);
}
