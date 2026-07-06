//! BT21-087 Zenith — Tamer, Black, Cost 4.
//! Traits: LIBERATOR
//!
//! # Printed card text (card image / official Bandai DB bundle — authoritative)
//! [Start of Your Turn] If you have 2 or less memory, set it to 3.
//! [On Play] Reveal the top 3 cards of your deck. Among them, play 1
//!   [Vemmon] without paying the cost or add 1 card with [Vemmon] in its
//!   text to the hand. Trash the rest.
//! [Security] Play this card without paying the cost.
//!
//! # Verdict: IMPLEMENTED
//!
//! # G-DSL-BINDING-CARD-NAME-EQUALS — RESOLVED
//! Clause 2 [On Play] was previously BLOCKED because the DSL had no way to
//! read the NAME of an already-bound reveal-pick `Card` binding inside a
//! downstream `if:` (to branch the Play-vs-Add-to-hand choice on whether the
//! ONE card selected from the reveal pool happens to be exactly named
//! "Vemmon", vs. merely referencing "[Vemmon]" in its printed text). The
//! `binding_card_name_is` predicate leaf (compiled field
//! `condition.binding_card_name_is: Option<(String, String)>`, engine eval in
//! `code/digimon-engine/src/dsl_cards/predicate.rs`) now exists and is
//! covered by `code/digimon-engine/tests/dsl/predicate_leaves_ii.rs` (Leaf 2).
//! Clause 2 is authored below using it.
//!
//! # Clause 2 shape (DCGO crosscheck)
//! DCGO `BT21_087.cs` (`ActivateCoroutine`, `EffectTiming.OnEnterFieldAnyone`):
//!   1. `SimplifiedRevealDeckTopCardsAndSelect(revealCount: 3, ...,
//!      remainingCardsPlace: Trash, canNoSelect: true)` with ONE
//!      `SimplifiedSelectCardConditionClass` (`canTargetCondition:
//!      cardSource.HasText("Vemmon")`, `maxCount: 1`). DCGO's `HasText` scans
//!      name + traits + all printed text (effect/inherited/security) — this
//!      is exactly the DSL's `in_text_contains` leaf (`G-DSL-IN-TEXT-CONTAINS`,
//!      see the official Q&A on this very card's bundle), so a SINGLE
//!      `select_reveal_buckets` bucket with `filter: { in_text_contains:
//!      Vemmon }`, `min: 0, max: 1` faithfully reproduces the DCGO bucket
//!      (union of "named Vemmon" and "has [Vemmon] in its text", capped at 1
//!      pick total — no double-pick over-exposure).
//!   2. If a card was selected:
//!      a. If `selectedCard.EqualsCardName("Vemmon")` (exact name match) →
//!         present a binary Play/Add-to-hand choice
//!         (`select_effect_choice`, labels ["Play", "Add to your hand"]).
//!         Gated via `binding_card_name_is: { binding: vemmon_pick, name_is:
//!         Vemmon }`.
//!      b. Else (has "Vemmon" in text but not named exactly Vemmon — e.g.
//!         Snatchmon/Destromon/Galacticmon/Tsumemon/Ragnarok Cannon/Xeno) →
//!         no choice; unconditionally add to hand
//!         (`add_to_hand_from_reveal`).
//!      c. If Play was picked → `play_from_revealed_free`. DCGO additionally
//!         gates on `CanPlayAsNewPermanent` (battle-area room) and trashes
//!         the card instead if that check fails; the engine's
//!         `play_from_revealed_with_cost` already reproduces this for free —
//!         on a failed play it reinserts the card into `revealed_cards`
//!         rather than the hand, so the DSL's later "trash the rest"
//!         (`per_selected` over `revealed`) naturally catches it. No extra
//!         authoring needed for that edge.
//!      d. If Add-to-hand was picked (2a) or 2b applies → add to hand.
//!   3. Any revealed card never selected in step 1 is trashed
//!      (`per_selected { selection: revealed, ... trash_from_reveal }`,
//!      mirroring BT24-059's shipped "trash the rest" idiom).
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT21/Black/BT21_087.cs

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::{PASS, SEL_REVEAL_START};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "BT21-087";

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT21-087 YAML parses and compiles")
        .add_card(make_test_card("DECK-PAD", "Filler"))
        .deck(1, &["DECK-PAD"; 12])
}

fn fire_timing(runner: &mut DebugRunner, timing: EffectTiming, source: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(timing, TriggerSource::Permanent(source));
    runner.game.drain_effect_queue();
}

fn make_vemmon(id: &str) -> CardData {
    let mut card = make_test_card(id, "Vemmon");
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Blue];
    card.level = Some(3);
    card.dp = Some(2000);
    card.play_cost = 3;
    card
}

/// A card that references "[Vemmon]" in its printed text without being named
/// Vemmon (e.g. Snatchmon/Destromon-shaped) — eligible for the reveal bucket
/// via `in_text_contains`, but must NOT get the Play/Add choice.
fn make_vemmon_text_card(id: &str, name: &str) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Black];
    card.level = Some(5);
    card.dp = Some(4000);
    card.play_cost = 5;
    card.effect_text = "[On Play] Digivolve 1 of your [Vemmon] Digimon.".to_string();
    card
}

fn make_filler(id: &str) -> CardData {
    make_test_card(id, id)
}

fn battle_area_contains(runner: &DebugRunner, player: usize, card_id: &str) -> bool {
    runner.game.players[player]
        .battle_area
        .iter()
        .any(|perm| perm.top_card().card_id(&runner.game.card_data) == card_id)
}

fn hand_contains(runner: &DebugRunner, player: usize, card_id: &str) -> bool {
    runner.game.players[player]
        .hand
        .iter()
        .any(|c| c.card_id(&runner.game.card_data) == card_id)
}

// ─── Structural tests ─────────────────────────────────────────────────────────

#[test]
fn bt21_087_yaml_printed_metadata() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present in pack");
    assert_eq!(card.name, "Zenith");
    assert_eq!(card.kind, CompiledCardKind::Tamer);
}

#[test]
fn bt21_087_has_exactly_three_triggered_clauses() {
    // Clause 2 [On Play] is now IMPLEMENTED (G-DSL-BINDING-CARD-NAME-EQUALS
    // resolved) alongside Clause 1 (Start of Your Turn) and Clause 3
    // (Security).
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let compiled = runner.compiled_card(CARD_ID).expect("present in pack");

    let triggered: Vec<&digimon_dsl::compiled::CompiledTriggeredClause> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    assert_eq!(
        triggered.len(),
        3,
        "BT21-087 ships three triggered clauses (Start of Turn, On Play, Security)"
    );

    assert!(
        triggered
            .iter()
            .any(|t| t.when == vec![CompiledTiming::StartOfYourTurn]),
        "Start of Your Turn clause must be present"
    );
    assert!(
        triggered
            .iter()
            .any(|t| t.when == vec![CompiledTiming::OnPlay]),
        "On Play clause must be present"
    );
    assert!(
        triggered
            .iter()
            .any(|t| t.when == vec![CompiledTiming::OnSecurity]),
        "Security clause must be present"
    );
}

/// The On Play clause must reveal top 3, install exactly one reveal bucket
/// (min:0, max:1, `in_text_contains: Vemmon`), gate the Play/Add choice on
/// `binding_card_name_is`, and trash the remainder.
#[test]
fn bt21_087_on_play_clause_shape() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let compiled = runner.compiled_card(CARD_ID).expect("present");
    let on_play = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnPlay) => Some(t),
            _ => None,
        })
        .expect("On Play clause");

    assert!(!on_play.optional, "reveal+trash are mandatory at clause level");

    assert!(
        on_play
            .process
            .iter()
            .any(|s| matches!(s, CompiledStep::RevealTopDeck { count: 3, .. })),
        "must reveal top 3"
    );

    let bucket_ok = on_play.process.iter().any(|s| {
        matches!(s, CompiledStep::SelectRevealBuckets { buckets, .. }
            if buckets.len() == 1 && buckets[0].min == 0 && buckets[0].max == 1)
    });
    assert!(
        bucket_ok,
        "must offer exactly one optional single-pick reveal bucket (select_reveal_buckets)"
    );

    let bucket_filter_ok = on_play.process.iter().any(|s| {
        matches!(s, CompiledStep::SelectRevealBuckets { buckets, .. }
            if buckets[0].filter.as_ref().is_some_and(|f| f.in_text_contains.as_deref() == Some("Vemmon")))
    });
    assert!(
        bucket_filter_ok,
        "the bucket filter must be in_text_contains: Vemmon (DCGO HasText union)"
    );

    // Downstream branch gated on binding_card_name_is — recurse into If
    // branches AND PerSelected bodies (the bucket pick is unwrapped via
    // per_selected before the name gate).
    fn steps_flat(steps: &[CompiledStep]) -> Vec<&CompiledStep> {
        let mut out = Vec::new();
        for s in steps {
            out.push(s);
            match s {
                CompiledStep::If { then, else_branch, .. } => {
                    out.extend(steps_flat(then));
                    out.extend(steps_flat(else_branch));
                }
                CompiledStep::PerSelected { body, .. } => {
                    out.extend(steps_flat(body));
                }
                _ => {}
            }
        }
        out
    }
    let flat = steps_flat(&on_play.process);
    let has_name_gate = flat.iter().any(|s| match s {
        CompiledStep::If { condition, .. } => {
            condition.binding_card_name_is.as_ref().is_some_and(|(_, name)| name == "Vemmon")
        }
        _ => false,
    });
    assert!(
        has_name_gate,
        "must branch on binding_card_name_is (binding, name_is: Vemmon)"
    );

    let has_effect_choice = flat
        .iter()
        .any(|s| matches!(s, CompiledStep::SelectEffectChoice { .. }));
    assert!(has_effect_choice, "must present a Play/Add-to-hand choice");

    let has_play_from_reveal = flat
        .iter()
        .any(|s| matches!(s, CompiledStep::PlayFromRevealedFree { .. }));
    assert!(has_play_from_reveal, "must play the picked card free from the reveal pool");

    let has_add_to_hand = flat
        .iter()
        .any(|s| matches!(s, CompiledStep::AddToHandFromReveal { .. }));
    assert!(has_add_to_hand, "must add the picked card to hand on the non-Play path(s)");

    let has_trash_rest = on_play.process.iter().any(|s| match s {
        CompiledStep::PerSelected { body, .. } => body
            .iter()
            .any(|inner| matches!(inner, CompiledStep::TrashFromReveal { .. })),
        _ => false,
    });
    assert!(has_trash_rest, "must trash the remaining revealed cards");
}

// ─── Clause 1: memory ramp ────────────────────────────────────────────────────

#[test]
fn bt21_087_start_of_turn_sets_memory_to_3_when_at_0() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(0).start();
    let zenith = r.place_on_field(0, CARD_ID, Some(0));
    let handle = r.perm_handle(0, zenith.index as usize);
    r.game.enqueue_triggered(
        EffectTiming::StartOfYourTurn,
        TriggerSource::Permanent(handle),
    );
    r.game.drain_effect_queue();
    assert_eq!(r.game.memory, 3, "memory was 0 (<=2) → must be set to 3");
}

#[test]
fn bt21_087_start_of_turn_sets_memory_to_3_when_at_2() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(2).start();
    let zenith = r.place_on_field(0, CARD_ID, Some(0));
    let handle = r.perm_handle(0, zenith.index as usize);
    r.game.enqueue_triggered(
        EffectTiming::StartOfYourTurn,
        TriggerSource::Permanent(handle),
    );
    r.game.drain_effect_queue();
    assert_eq!(r.game.memory, 3, "memory was 2 (<=2) → must be set to 3");
}

#[test]
fn bt21_087_start_of_turn_does_not_change_memory_when_at_3() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(3).start();
    let zenith = r.place_on_field(0, CARD_ID, Some(0));
    let handle = r.perm_handle(0, zenith.index as usize);
    r.game.enqueue_triggered(
        EffectTiming::StartOfYourTurn,
        TriggerSource::Permanent(handle),
    );
    r.game.drain_effect_queue();
    assert_eq!(r.game.memory, 3, "memory was 3 (>2) → must remain 3");
}

#[test]
fn bt21_087_start_of_turn_does_not_change_memory_when_above_3() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(5).start();
    let zenith = r.place_on_field(0, CARD_ID, Some(0));
    let handle = r.perm_handle(0, zenith.index as usize);
    r.game.enqueue_triggered(
        EffectTiming::StartOfYourTurn,
        TriggerSource::Permanent(handle),
    );
    r.game.drain_effect_queue();
    assert_eq!(
        r.game.memory, 5,
        "memory was 5 (>2) → must remain unchanged"
    );
}

// ─── Clause 2: [On Play] reveal-3 / play-or-hand / trash-rest behavioral ──────

/// A Vemmon in the reveal pool installs an optional reveal-bucket selection.
#[test]
fn bt21_087_on_play_installs_optional_reveal_bucket_with_vemmon_eligible() {
    let mut runner = base()
        .add_card(make_vemmon("VEMMON"))
        .add_card(make_filler("FILL-1"))
        .add_card(make_filler("FILL-2"))
        // VEMMON on top of deck (last in array) → revealed index 0.
        .deck(0, &["FILL-2", "FILL-1", "VEMMON"])
        .memory(10)
        .start();

    let zenith = runner.place_on_field(0, CARD_ID, Some(0));
    fire_timing(&mut runner, EffectTiming::OnPlay, zenith);

    let view = runner
        .pending_selection_view()
        .expect("reveal bucket selection installs");
    assert!(matches!(view.kind, SelectionKind::RevealBucket { .. }));
    assert!(runner.pending_is_optional(), "'canNoSelect: true' → optional");
    assert!(
        view.valid_action_ids.contains(&SEL_REVEAL_START),
        "the Vemmon at reveal index 0 must be a legal pick"
    );
}

/// Picking the exact-name "Vemmon" installs a Play/Add-to-hand choice;
/// choosing Play plays it for free and trashes the other 2 revealed cards.
#[test]
fn bt21_087_picking_vemmon_then_play_plays_it_free_and_trashes_rest() {
    let mut runner = base()
        .add_card(make_vemmon("VEMMON"))
        .add_card(make_filler("FILL-1"))
        .add_card(make_filler("FILL-2"))
        .deck(0, &["FILL-2", "FILL-1", "VEMMON"])
        .memory(0) // cannot afford Vemmon's cost normally — proves it's free
        .start();

    let zenith = runner.place_on_field(0, CARD_ID, Some(0));
    fire_timing(&mut runner, EffectTiming::OnPlay, zenith);

    let field_before = runner.battle_area_size(0);
    let trash_before = runner.trash_size(0);
    let memory_before = runner.memory();

    runner
        .execute_action(0, SEL_REVEAL_START)
        .expect("pick the Vemmon");
    runner.game.drain_effect_queue();

    let branch = runner
        .pending_selection_view()
        .expect("Play/Add-to-hand choice installs for exact-name Vemmon");
    assert_eq!(branch.kind, SelectionKind::EffectChoice);
    let labels: Vec<&str> = branch
        .effect_choices
        .as_ref()
        .unwrap()
        .iter()
        .map(|c| c.label.as_str())
        .collect();
    assert!(labels.iter().any(|l| l.to_lowercase().contains("play")));
    assert!(labels.iter().any(|l| l.to_lowercase().contains("hand")));

    runner.execute_branch(0).expect("choose Play"); // Play is listed first
    runner.game.drain_effect_queue();
    let _ = runner.auto_resolve();

    assert!(
        battle_area_contains(&runner, 0, "VEMMON"),
        "Vemmon must be played onto the field"
    );
    assert_eq!(
        runner.battle_area_size(0),
        field_before + 1,
        "exactly one card played"
    );
    assert_eq!(
        runner.memory(),
        memory_before,
        "play_from_revealed_free must not spend memory"
    );
    assert_eq!(
        runner.trash_size(0),
        trash_before + 2,
        "the 2 remaining revealed cards are trashed"
    );
}

/// Picking the exact-name "Vemmon" then choosing Add-to-hand adds it to hand
/// instead of playing it.
#[test]
fn bt21_087_picking_vemmon_then_add_to_hand_adds_it_to_hand() {
    let mut runner = base()
        .add_card(make_vemmon("VEMMON"))
        .add_card(make_filler("FILL-1"))
        .add_card(make_filler("FILL-2"))
        .deck(0, &["FILL-2", "FILL-1", "VEMMON"])
        .memory(10)
        .start();

    let zenith = runner.place_on_field(0, CARD_ID, Some(0));
    fire_timing(&mut runner, EffectTiming::OnPlay, zenith);

    runner
        .execute_action(0, SEL_REVEAL_START)
        .expect("pick the Vemmon");
    runner.game.drain_effect_queue();

    let branch = runner
        .pending_selection_view()
        .expect("Play/Add-to-hand choice installs");
    let add_idx = branch
        .effect_choices
        .as_ref()
        .unwrap()
        .iter()
        .position(|c| c.label.to_lowercase().contains("hand"))
        .expect("an Add to hand label exists");

    runner.execute_branch(add_idx).expect("choose Add to hand");
    runner.game.drain_effect_queue();
    let _ = runner.auto_resolve();

    assert!(
        !battle_area_contains(&runner, 0, "VEMMON"),
        "Vemmon must NOT be played when Add-to-hand is chosen"
    );
    assert!(
        hand_contains(&runner, 0, "VEMMON"),
        "Vemmon must be added to hand"
    );
}

/// A card with "[Vemmon]" in its printed TEXT but NOT named exactly Vemmon is
/// still an eligible reveal pick (in_text_contains), but picking it must NOT
/// offer a Play/Add choice — it is unconditionally added to hand.
#[test]
fn bt21_087_picking_vemmon_text_card_adds_to_hand_with_no_choice() {
    let mut runner = base()
        .add_card(make_vemmon_text_card("SNATCHMON", "Snatchmon"))
        .add_card(make_filler("FILL-1"))
        .add_card(make_filler("FILL-2"))
        .deck(0, &["FILL-2", "FILL-1", "SNATCHMON"])
        .memory(10)
        .start();

    let zenith = runner.place_on_field(0, CARD_ID, Some(0));
    fire_timing(&mut runner, EffectTiming::OnPlay, zenith);

    let view = runner
        .pending_selection_view()
        .expect("reveal bucket selection installs for a Vemmon-text card");
    assert!(
        view.valid_action_ids.contains(&SEL_REVEAL_START),
        "the Vemmon-text card at reveal index 0 must be a legal pick"
    );

    runner
        .execute_action(0, SEL_REVEAL_START)
        .expect("pick the Vemmon-text card");
    runner.game.drain_effect_queue();

    // No Play/Add choice — must resolve straight through to hand.
    if let Some(view) = runner.pending_selection_view() {
        assert_ne!(
            view.kind,
            SelectionKind::EffectChoice,
            "a non-'Vemmon'-named card must not present a Play/Add-to-hand choice"
        );
    }
    let _ = runner.auto_resolve();

    assert!(
        !battle_area_contains(&runner, 0, "SNATCHMON"),
        "a non-exact-name Vemmon-text card must never be played for free"
    );
    assert!(
        hand_contains(&runner, 0, "SNATCHMON"),
        "the Vemmon-text card must be added to hand unconditionally"
    );
}

/// Negative: a card with no relation to Vemmon (not named Vemmon, no
/// [Vemmon] in its text) is NOT an eligible reveal pick.
#[test]
fn bt21_087_unrelated_card_not_eligible() {
    let mut runner = base()
        .add_card(make_filler("UNRELATED"))
        .add_card(make_filler("FILL-1"))
        .add_card(make_filler("FILL-2"))
        .deck(0, &["FILL-2", "FILL-1", "UNRELATED"])
        .memory(10)
        .start();

    let zenith = runner.place_on_field(0, CARD_ID, Some(0));
    fire_timing(&mut runner, EffectTiming::OnPlay, zenith);

    // No eligible card → bucket auto-skips (no selection), or, if installed,
    // the unrelated card is not a legal pick.
    if let Some(view) = runner.pending_selection_view() {
        assert!(
            !view.valid_action_ids.iter().any(|a| *a >= SEL_REVEAL_START),
            "the unrelated card must not be a legal pick: {:?}",
            view.valid_action_ids
        );
    }
}

/// Declining the optional reveal-bucket pick still trashes all 3 revealed
/// cards.
#[test]
fn bt21_087_decline_reveal_pick_trashes_all_three() {
    let mut runner = base()
        .add_card(make_vemmon("VEMMON"))
        .add_card(make_filler("FILL-1"))
        .add_card(make_filler("FILL-2"))
        .deck(0, &["FILL-2", "FILL-1", "VEMMON"])
        .memory(10)
        .start();

    let zenith = runner.place_on_field(0, CARD_ID, Some(0));
    let trash_before = runner.trash_size(0);
    fire_timing(&mut runner, EffectTiming::OnPlay, zenith);

    if runner.pending_selection_view().is_some() {
        let _ = runner.execute_action(0, PASS);
        runner.game.drain_effect_queue();
    }
    let _ = runner.auto_resolve();

    assert!(
        runner.trash_size(0) >= trash_before + 3,
        "declining the pick trashes all 3 revealed cards"
    );
    assert!(
        !battle_area_contains(&runner, 0, "VEMMON"),
        "nothing is played when the optional pick is declined"
    );
    assert!(
        !hand_contains(&runner, 0, "VEMMON"),
        "nothing is added to hand when the optional pick is declined"
    );
}

// ─── Clause 3: security play ──────────────────────────────────────────────────

/// Structural test: the `on_security` clause compiles with a `PlayFromSecurity`
/// step. The behavioral runtime for play-self-free security is well-covered by
/// BT18-087, BT21-084, BT22-084, BT21-015; here we only assert the clause is
/// present so security checks can route to it correctly.
#[test]
fn bt21_087_security_clause_has_play_from_security_step() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("BT21-087 in compiled_cards");

    let clause = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnSecurity))
        .expect("on_security clause must exist on BT21-087");

    let has_play_from_security = clause
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::PlayFromSecurity));
    assert!(
        has_play_from_security,
        "on_security clause must lower to a PlayFromSecurity step"
    );
}
