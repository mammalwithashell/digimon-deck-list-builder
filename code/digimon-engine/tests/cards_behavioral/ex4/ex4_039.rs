//! EX4-039 Gabumon — Lv3, Black, Cost 3, DP 1000, Reptile.
//!
//! # Card text (cards.json — printed)
//!
//! [On Play] Reveal the top 3 cards of your deck. Add 1 Digimon card with
//! [Garurumon] in its name and 1 Digimon card with [Agumon], [Greymon] or
//! [Omnimon] in its name among them to your hand. Place the rest on top of
//! your deck in any order.
//!
//! [Your Turn] [Once Per Turn] When one of your other Digimon digivolves,
//! gain 1 memory.
//!
//! No printed security effect.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX4/Black/EX4_039.cs
//!
//! # Sister card
//! EX4-038 Agumon — same shape with bucket halves swapped.
//!
//! # Patterns this test covers
//! - On-Play reveal-3 with two name-based mutual buckets + remainder on top
//!   of deck (mirror of BT24-031 / BT12-059 trait/name bucket idiom).
//! - Inherited [Your Turn][OPT] on_digivolve observer that gains memory when
//!   a friendly Digimon digivolves.
//!
//! # Known engine/DSL gaps affecting these tests
//!
//! - **G-EVENT-TARGET-NOT-SOURCE** (NEW, this card surfaces it):
//!   No predicate excludes the OnDigivolve event_permanent when it equals
//!   the inherited clause's source_permanent (the carrier). Printed text
//!   "your other Digimon" cannot be faithfully expressed today. A test that
//!   asserts the carrier-self-digivolve does NOT trigger the inherited
//!   memory gain is `#[ignore]`'d pending the gap closure. Logged in
//!   qa/dsl-vocab-gaps.md.
//!
//! - **G-OPT-TRIGGERED**: `once_per_turn: true` on a triggered clause is
//!   authored but engine queue-drain does not yet enforce same-turn lockout.
//!   The same-turn-second-fire test is `#[ignore]`'d.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledColor, CompiledCost, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_dsl::{compile::compile, spec::CardSpec};
use digimon_engine::action::space::{PASS, SEL_REVEAL_START};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const YAML: &str = include_str!("../../../cards/ex4/EX4-039.yaml");

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn make_named_digimon(id: &str, name: &str, level: u8, dp: i32) -> CardData {
    let mut c = make_test_card(id, name);
    c.level = Some(level);
    c.dp = Some(dp);
    c
}

fn make_filler(id: &str) -> CardData {
    make_test_card(id, id)
}

/// Drain optional activation prompts by accepting the first non-PASS action.
/// Used to walk through reveal-bucket sub-prompts deterministically.
fn drain_accepting_all(runner: &mut DebugRunner) -> bool {
    let mut accepted = false;
    let mut steps = 0;
    while runner.game.pending_selection.is_some() && steps < 30 {
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

fn revealed_action_for_id(runner: &DebugRunner, id: &str) -> u16 {
    runner
        .game
        .revealed_cards
        .iter()
        .enumerate()
        .find_map(|(idx, card)| {
            (card.card_id(&runner.game.card_data) == id).then_some(SEL_REVEAL_START + idx as u16)
        })
        .expect("revealed card exists")
}

fn pick_revealed_by_id(runner: &mut DebugRunner, id: &str, label: &str) {
    let view = runner.pending_selection_view().expect(label);
    let action = revealed_action_for_id(runner, id);
    assert!(view.valid_action_ids.contains(&action), "{label}");
    runner.execute_action(0, action).expect(label);
}

fn zone_ids(cards: &[CardSource], data: &[CardData]) -> Vec<String> {
    cards.iter().map(|c| c.card_id(data).to_string()).collect()
}

/// Standard fixture for EX4-039 without a deck preloaded; tests that need
/// reveal cards add to deck explicitly.
fn gabu_runner_empty() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("EX4-039 YAML parses")
        .add_card(make_filler("FILL"))
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start()
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn ex4_039_yaml_compiles() {
    let spec: CardSpec = serde_yml::from_str(YAML).expect("EX4-039 YAML parses");
    let _compiled = compile(&spec).expect("EX4-039 YAML compiles");
}

#[test]
fn ex4_039_metadata_matches_print() {
    let spec: CardSpec = serde_yml::from_str(YAML).unwrap();
    let compiled = compile(&spec).unwrap();
    assert_eq!(compiled.cost, Some(3));
    assert_eq!(compiled.level, Some(3));
    assert_eq!(compiled.dp, Some(1000));
    assert!(compiled
        .color
        .iter()
        .any(|c| matches!(c, CompiledColor::Black)));
}

#[test]
fn ex4_039_has_on_play_and_inherited_on_digivolve_clauses() {
    let spec: CardSpec = serde_yml::from_str(YAML).unwrap();
    let compiled = compile(&spec).unwrap();
    let on_play = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnPlay))
        .expect("EX4-039 must have an on_play clause");
    assert_eq!(on_play.scope, CompiledScope::FaceUp);

    let inherited = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| {
            t.scope == CompiledScope::Inherited && t.when.contains(&CompiledTiming::OnDigivolve)
        })
        .expect("EX4-039 must have an inherited on_digivolve clause");
    assert!(
        inherited.once_per_turn,
        "printed [Once Per Turn] → once_per_turn must be set"
    );
    assert!(
        inherited.active_when.is_some(),
        "[Your Turn] requires active_when"
    );
    assert!(
        inherited.condition.is_some(),
        "inherited clause needs an event_target_owner condition"
    );
}

#[test]
fn ex4_039_on_play_uses_select_reveal_buckets_and_top_remainder() {
    let spec: CardSpec = serde_yml::from_str(YAML).unwrap();
    let compiled = compile(&spec).unwrap();
    let on_play = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnPlay))
        .expect("on_play clause");
    let has_buckets = on_play
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::SelectRevealBuckets { .. }));
    assert!(
        has_buckets,
        "on_play clause must call select_reveal_buckets"
    );
    let remainder_top = on_play.process.iter().any(|s| match s {
        CompiledStep::PlaceRemainderOnDeck { position, .. } => {
            matches!(position, digimon_dsl::compiled::CompiledStackPosition::Top)
        }
        _ => false,
    });
    assert!(
        remainder_top,
        "remainder must be placed on TOP of deck (not bottom — printed text)"
    );
}

#[test]
fn ex4_039_inherited_clause_body_gains_memory_one() {
    let spec: CardSpec = serde_yml::from_str(YAML).unwrap();
    let compiled = compile(&spec).unwrap();
    let inherited = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| {
            t.scope == CompiledScope::Inherited && t.when.contains(&CompiledTiming::OnDigivolve)
        })
        .expect("inherited on_digivolve clause");
    let gain = inherited
        .process
        .iter()
        .find_map(|s| match s {
            CompiledStep::GainMemory(n) => Some(*n),
            _ => None,
        })
        .expect("inherited clause must include gain_memory step");
    assert_eq!(gain, 1, "printed text says +1 memory");
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 2 — On-Play reveal-bucket behavior
// ═══════════════════════════════════════════════════════════════════════════════

/// All three revealed cards are eligible (one Garurumon, one Greymon-named,
/// one Omnimon-named). Both buckets resolve; both go to hand; the remaining
/// card goes back on top of the deck.
#[test]
fn ex4_039_on_play_reveals_three_picks_one_garurumon_and_one_agu_grey_omni() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("EX4-039 YAML parses")
        .add_card(make_named_digimon("GARU", "Garurumon", 4, 4000))
        .add_card(make_named_digimon("GREY", "Greymon", 4, 4000))
        .add_card(make_named_digimon("OMNI", "Omnimon", 6, 13000))
        .deck(0, &["OMNI", "GREY", "GARU"])
        .hand(0, &["EX4-039"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Gabumon");

    // First bucket: Garurumon. PASS must NOT be legal yet (min: 1).
    let view1 = runner.pending_selection_view().expect("Garurumon prompt");
    assert!(
        !view1.valid_action_ids.contains(&PASS),
        "first bucket is mandatory (min: 1) when a Garurumon-named card is in the reveal"
    );
    pick_revealed_by_id(&mut runner, "GARU", "pick GARU");

    // Second bucket: Agumon/Greymon/Omnimon — both GREY and OMNI are eligible.
    let view2 = runner
        .pending_selection_view()
        .expect("Greymon/Omnimon prompt");
    let grey_action = revealed_action_for_id(&runner, "GREY");
    let omni_action = revealed_action_for_id(&runner, "OMNI");
    assert!(view2.valid_action_ids.contains(&grey_action));
    assert!(view2.valid_action_ids.contains(&omni_action));
    // Pick GREY for the second bucket.
    pick_revealed_by_id(&mut runner, "GREY", "pick GREY");

    // Drain remainder-permutation prompts (single card → trivial).
    drain_accepting_all(&mut runner);

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(
        hand_ids.contains(&"GARU".to_string()),
        "GARU must be in hand"
    );
    assert!(
        hand_ids.contains(&"GREY".to_string()),
        "GREY must be in hand"
    );
    assert!(
        !hand_ids.contains(&"OMNI".to_string()),
        "OMNI must remain on deck"
    );

    // Remainder must end up on top of deck (printed: "Place the rest on top").
    let deck_top = runner.game.players[0]
        .deck
        .last()
        .map(|c| c.card_id(&runner.game.card_data).to_string());
    assert_eq!(
        deck_top.as_deref(),
        Some("OMNI"),
        "the unpicked OMNI card must sit on top of the deck"
    );
}

/// With only ONE Garurumon-named card and NO Agumon/Greymon/Omnimon-named
/// card in the reveal, the first bucket consumes Garurumon, the second
/// bucket has no eligible candidate, and the engine skips it (auto-resolves
/// the bucket without installing a prompt). Only Garurumon ends up in hand.
#[test]
fn ex4_039_on_play_second_bucket_auto_skips_when_no_candidate() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("EX4-039 YAML parses")
        .add_card(make_named_digimon("GARU", "Garurumon", 4, 4000))
        .add_card(make_named_digimon("PATA", "Patamon", 3, 3000))
        .add_card(make_named_digimon("BIYO", "Biyomon", 3, 3000))
        .deck(0, &["BIYO", "PATA", "GARU"])
        .hand(0, &["EX4-039"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Gabumon");

    // First bucket: pick the only Garurumon.
    pick_revealed_by_id(&mut runner, "GARU", "pick GARU");

    // Second bucket auto-skips (no candidate). The next prompt — if any —
    // is the remainder permutation. Drain to end.
    drain_accepting_all(&mut runner);

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(hand_ids.contains(&"GARU".to_string()), "GARU goes to hand");
    assert!(
        !hand_ids.contains(&"PATA".to_string()),
        "PATA stays on deck"
    );
    assert!(
        !hand_ids.contains(&"BIYO".to_string()),
        "BIYO stays on deck"
    );
}

/// Negative shape: when neither bucket has a candidate, both buckets
/// auto-skip and no card ends up in hand. (The reveal still happens; the
/// remainder ordering prompt may install for the 3 cards.)
#[test]
fn ex4_039_on_play_both_buckets_auto_skip_when_no_candidate() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("EX4-039 YAML parses")
        .add_card(make_named_digimon("PATA", "Patamon", 3, 3000))
        .add_card(make_named_digimon("BIYO", "Biyomon", 3, 3000))
        .add_card(make_named_digimon("PALM", "Palmon", 3, 3000))
        .deck(0, &["PALM", "BIYO", "PATA"])
        .hand(0, &["EX4-039"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Gabumon");

    // Both buckets auto-skip (no candidates). Drain remainder ordering prompt.
    drain_accepting_all(&mut runner);

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(!hand_ids.contains(&"PATA".to_string()));
    assert!(!hand_ids.contains(&"BIYO".to_string()));
    assert!(!hand_ids.contains(&"PALM".to_string()));
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 3 — Inherited on-digivolve memory gain
// ═══════════════════════════════════════════════════════════════════════════════

/// Positive: when EX4-039 is a digivolution source under another permanent
/// AND a separate friendly Digimon digivolves on player 0's turn, +1 memory.
#[test]
fn ex4_039_inherited_gains_memory_when_other_friendly_digivolves() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("EX4-039 YAML parses")
        .add_card(make_named_digimon("CARRIER", "Carrier", 4, 4000))
        .add_card(make_named_digimon("OTHER-LV3", "OtherLv3", 3, 3000))
        .add_card(make_named_digimon("OTHER-LV4", "OtherLv4", 4, 5000))
        .add_card(make_filler("FILL"))
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(0)
        .start();

    // EX4-039 sits as a source under CARRIER on player 0's field.
    let _carrier_handle = runner.place_stack(0, &["EX4-039", "CARRIER"]);
    // Place a separate Digimon (OTHER-LV3) on the field — it will digivolve
    // into OTHER-LV4 to fire OnDigivolve on a DIFFERENT permanent.
    let other_handle = runner.place_on_field(0, "OTHER-LV3", Some(0));

    let memory_before = runner.memory();

    // Synthesize an OnDigivolve event for OTHER-LV3 → OTHER-LV4. We use the
    // public TriggerSource::Digivolved variant which is exactly what the
    // engine fires after a real digivolve. The test exercises observer
    // wiring + condition + body, not the digivolve action machinery.
    let other_lv4_card_data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "OTHER-LV4")
        .unwrap();
    let next_idx = runner.game.next_card_index();
    let new_top = CardSource::new(other_lv4_card_data_idx, 0, next_idx);
    // Simulate the digivolve: push the new top onto OTHER-LV3's stack.
    let perm = &mut runner.game.players[0].battle_area[other_handle.index as usize];
    perm.digivolve(new_top.clone(), runner.game.turn_count);
    let event_card = perm.top_card().handle();

    runner.game.enqueue_triggered(
        EffectTiming::OnDigivolve,
        TriggerSource::Digivolved {
            player: 0,
            permanent: other_handle,
            card: event_card,
            effect_initiated: false,
            dna_origin: false,
        },
    );
    runner.game.drain_effect_queue();

    // Drain any optional prompts (mandatory clause shouldn't install a
    // selection — it just gains memory directly).
    drain_accepting_all(&mut runner);

    assert_eq!(
        runner.memory(),
        memory_before + 1,
        "inherited clause must grant +1 memory when another friendly Digimon digivolves"
    );
}

/// Negative gate: opponent's Digimon digivolving must NOT trigger the
/// inherited clause (event_target_owner: you gate).
#[test]
fn ex4_039_inherited_does_not_fire_on_opponent_digivolve() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("EX4-039 YAML parses")
        .add_card(make_named_digimon("CARRIER", "Carrier", 4, 4000))
        .add_card(make_named_digimon("OPP-LV3", "OppLv3", 3, 3000))
        .add_card(make_named_digimon("OPP-LV4", "OppLv4", 4, 5000))
        .add_card(make_filler("FILL"))
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(0)
        .start();

    let _carrier = runner.place_stack(0, &["EX4-039", "CARRIER"]);
    let opp_handle = runner.place_on_field(1, "OPP-LV3", Some(0));

    let memory_before = runner.memory();

    let opp_lv4_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "OPP-LV4")
        .unwrap();
    let next_idx = runner.game.next_card_index();
    let new_top = CardSource::new(opp_lv4_idx, 1, next_idx);
    let perm = &mut runner.game.players[1].battle_area[opp_handle.index as usize];
    perm.digivolve(new_top.clone(), runner.game.turn_count);
    let event_card = perm.top_card().handle();

    runner.game.enqueue_triggered(
        EffectTiming::OnDigivolve,
        TriggerSource::Digivolved {
            player: 1,
            permanent: opp_handle,
            card: event_card,
            effect_initiated: false,
            dna_origin: false,
        },
    );
    runner.game.drain_effect_queue();
    drain_accepting_all(&mut runner);

    assert_eq!(
        runner.memory(),
        memory_before,
        "opponent's Digimon digivolving must NOT trigger the inherited clause"
    );
}

/// Engine behavior: the `scope: inherited` flag on a TRIGGERED clause makes
/// the clause active while the card is a digivolution SOURCE sitting beneath
/// another permanent (RULES_CONTEXT.md 15-3-1). Printed text "When one of your
/// other Digimon digivolves" then fires EX4-039's inherited clause when ANOTHER
/// friendly permanent digivolves. Verify that path: EX4-039 is a digivolution
/// source under a generic top card, and a separate friendly Digimon digivolves.
#[test]
fn ex4_039_inherited_fires_when_source_and_other_friendly_digivolves() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("EX4-039 YAML parses")
        .add_card(make_named_digimon("CARRIER", "Carrier", 4, 4000))
        .add_card(make_named_digimon("OTHER-LV3", "OtherLv3", 3, 3000))
        .add_card(make_named_digimon("OTHER-LV4", "OtherLv4", 4, 5000))
        .add_card(make_filler("FILL"))
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(0)
        .start();

    // EX4-039 sits as a digivolution source under a generic CARRIER top card so
    // its inherited clause is active from its legitimate source position.
    let _carrier_handle = runner.place_stack(0, &["EX4-039", "CARRIER"]);
    let other_handle = runner.place_on_field(0, "OTHER-LV3", Some(0));
    let memory_before = runner.memory();

    let lv4_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "OTHER-LV4")
        .unwrap();
    let next_idx = runner.game.next_card_index();
    let new_top = CardSource::new(lv4_idx, 0, next_idx);
    let perm = &mut runner.game.players[0].battle_area[other_handle.index as usize];
    perm.digivolve(new_top, runner.game.turn_count);
    let event_card = perm.top_card().handle();

    runner.game.enqueue_triggered(
        EffectTiming::OnDigivolve,
        TriggerSource::Digivolved {
            player: 0,
            permanent: other_handle,
            card: event_card,
            effect_initiated: false,
            dna_origin: false,
        },
    );
    runner.game.drain_effect_queue();
    drain_accepting_all(&mut runner);

    // The inherited clause is active because EX4-039 is a digivolution source;
    // it grants +1 memory because OTHER-LV3 (a different friendly permanent)
    // digivolved.
    assert_eq!(
        runner.memory(),
        memory_before + 1,
        "inherited clause fires when EX4-039 is a digivolution source and ANOTHER friendly Digimon digivolves"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 4 — IGNORED tests (gap-blocked)
// ═══════════════════════════════════════════════════════════════════════════════

/// Printed text says "your other Digimon", which excludes the carrier
/// (the permanent EX4-039 sits under) from triggering the clause when IT
/// digivolves further. The YAML condition uses `event_permanent_is_source:
/// false` — the digivolving (event) permanent must NOT be this inherited
/// clause's source-permanent (the carrier). When the carrier itself
/// digivolves, the condition fails and no memory is gained.
#[test]
fn ex4_039_inherited_does_not_fire_when_carrier_itself_digivolves() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("EX4-039 YAML parses")
        .add_card(make_named_digimon("CARRIER", "Carrier", 4, 4000))
        .add_card(make_named_digimon("CARRIER-LV5", "CarrierLv5", 5, 7000))
        .add_card(make_filler("FILL"))
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(0)
        .start();

    let carrier = runner.place_stack(0, &["EX4-039", "CARRIER"]);
    let memory_before = runner.memory();

    let lv5_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "CARRIER-LV5")
        .unwrap();
    let next_idx = runner.game.next_card_index();
    let new_top = CardSource::new(lv5_idx, 0, next_idx);
    let perm = &mut runner.game.players[0].battle_area[carrier.index as usize];
    perm.digivolve(new_top, runner.game.turn_count);
    let event_card = perm.top_card().handle();

    runner.game.enqueue_triggered(
        EffectTiming::OnDigivolve,
        TriggerSource::Digivolved {
            player: 0,
            permanent: carrier,
            card: event_card,
            effect_initiated: false,
            dna_origin: false,
        },
    );
    runner.game.drain_effect_queue();
    drain_accepting_all(&mut runner);

    assert_eq!(
        runner.memory(),
        memory_before,
        "carrier-self-digivolve must NOT trigger (printed: \"your OTHER Digimon\"); will pass once G-EVENT-TARGET-NOT-SOURCE closes"
    );
}

/// Inherited clause OPT lockout — printed [Once Per Turn]. The inherited
/// "[Your Turn][Once Per Turn] When one of your other Digimon digivolves,
/// gain 1 memory" clause may fire at most once per turn. Firing a second
/// other-friendly digivolve in the same turn must NOT grant a second
/// memory; the per-turn OPT reset (`Permanent::new_turn`) re-arms it.
///
/// OPT enforcement for permanent-sourced triggered effects is wired in
/// `run_queued_effect_inner` (`effect_queue.rs` ~1980-2016); the per-permanent
/// `effect_activations` counter is keyed to the carrier hosting EX4-039.
#[test]
fn ex4_039_inherited_opt_lockout_blocks_second_fire_same_turn() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("EX4-039 YAML parses")
        .add_card(make_named_digimon("CARRIER", "Carrier", 4, 4000))
        .add_card(make_named_digimon("OTHER-LV3", "OtherLv3", 3, 3000))
        .add_card(make_named_digimon("OTHER-LV4", "OtherLv4", 4, 5000))
        .add_card(make_named_digimon("OTHER-LV5", "OtherLv5", 5, 7000))
        .add_card(make_filler("FILL"))
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(0)
        .start();

    // EX4-039 sits as a source under CARRIER on player 0's field.
    let carrier_handle = runner.place_stack(0, &["EX4-039", "CARRIER"]);
    // A separate Digimon (OTHER-LV3) that will digivolve twice this turn.
    let other_handle = runner.place_on_field(0, "OTHER-LV3", Some(0));

    /// Push a new top onto `other_handle` and fire its OnDigivolve event.
    fn digivolve_other(runner: &mut DebugRunner, other: PermanentHandle, into_id: &str) {
        let idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == into_id)
            .unwrap();
        let next_idx = runner.game.next_card_index();
        let new_top = CardSource::new(idx, 0, next_idx);
        let perm = &mut runner.game.players[0].battle_area[other.index as usize];
        perm.digivolve(new_top, runner.game.turn_count);
        let event_card = perm.top_card().handle();
        runner.game.enqueue_triggered(
            EffectTiming::OnDigivolve,
            TriggerSource::Digivolved {
                player: 0,
                permanent: other,
                card: event_card,
                effect_initiated: false,
                dna_origin: false,
            },
        );
        runner.game.drain_effect_queue();
        drain_accepting_all(runner);
    }

    let memory_before = runner.memory();

    // ── First fire: OTHER-LV3 → OTHER-LV4. The inherited clause grants +1.
    digivolve_other(&mut runner, other_handle, "OTHER-LV4");
    assert_eq!(
        runner.memory(),
        memory_before + 1,
        "first other-friendly digivolve must grant +1 memory"
    );

    // ── Second fire, SAME turn: OTHER-LV4 → OTHER-LV5. OPT lockout — no gain.
    digivolve_other(&mut runner, other_handle, "OTHER-LV5");
    assert_eq!(
        runner.memory(),
        memory_before + 1,
        "OPT lockout must block the second memory gain in the same turn"
    );

    // ── OPT reset: `Permanent::new_turn()` clears `effect_activations` on the
    // carrier hosting EX4-039 — the same per-turn reset `begin_turn()` does.
    runner.game.players[0].battle_area[carrier_handle.index as usize].new_turn();

    // Place a fresh separate Digimon and digivolve it — the inherited clause
    // must fire again after the OPT reset.
    let third_handle = {
        let idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "OTHER-LV3")
            .unwrap();
        let next_idx = runner.game.next_card_index();
        let card = CardSource::new(idx, 0, next_idx);
        runner.game.players[0]
            .battle_area
            .push(digimon_engine::permanent::Permanent::new(
                card,
                runner.game.turn_count,
            ));
        PermanentHandle {
            player: 0,
            index: (runner.game.players[0].battle_area.len() - 1) as u8,
        }
    };
    digivolve_other(&mut runner, third_handle, "OTHER-LV4");
    assert_eq!(
        runner.memory(),
        memory_before + 2,
        "after the per-turn OPT reset the inherited clause must grant memory again"
    );
}
