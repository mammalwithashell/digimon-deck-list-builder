//! P-217 Haru Shinkai — Tamer, Red, Cost 3.
//! Traits: App Driver, Appmon.
//!
//! # Card text (printed — authoritative)
//!
//! [On Play] Reveal the top 3 cards of your deck. Add 1 [Social] trait card
//! and 1 [Creation], [Navi] or [Tool] trait card among them to the hand.
//! Return the rest to the bottom of the deck.
//!
//! [Your Turn] When any of your Digimon get linked to a [Social], [Navi] or
//! [Tool] trait card, by suspending this Tamer, gain 1 memory.
//!
//! [Security] Play this card without paying the cost.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/P/Red/P_217.cs
//!
//! # [Creation] adjudication (re-verified 2026-07-10)
//! The [Your Turn] link observer prints ONLY [Social], [Navi], [Tool] —
//! [Creation] belongs only to the [On Play] clause's second bucket. Verified
//! against the card image (P-217.webp, zoomed) and the official Bandai DB
//! bundle (data/card_bundles/P-217.md); DCGO's LinkCardCondition matches.
//! The lossy cards.json entry wrongly adds [Creation] to the [Your Turn] line;
//! an earlier version of this suite treated that as a "DCGO bug" and asserted
//! the observer fires on [Creation] — that was drift and is now inverted:
//! [Creation] links must NOT fire the observer, but a [Creation] card DOES
//! qualify for On Play bucket 2.
//!
//! # Patterns this test covers
//! - `when: on_any_link` board-wide link observer (G-DSL-WHEN-ANY-OWN-DIGIMON-LINKED)
//! - `event_target_owner: you` filtering (host must be own Digimon)
//! - `event_card_trait_has` filtering on the linked card
//! - `activation_cost: { suspend_self: true }` as a first body step on a Tamer
//! - `your_turn: true` gating on the board-wide observer
//! - `select_reveal_buckets` two-bucket reveal (same shape as BT25-007)
//! - `play_from_security: {}` Tamer security play

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{CompiledClause, CompiledStep, CompiledTiming};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, EffectTiming, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "P-217";

// ─── Card-factory helpers ────────────────────────────────────────────────────

fn make_digimon(id: &str, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(4000);
    card.play_cost = 4;
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    card
}

/// A plain Digimon with no relevant traits (not a valid link-trigger).
fn make_plain_digimon(id: &str) -> CardData {
    make_digimon(id, &["Beast"])
}

/// A Digimon with the [Social] trait (valid link target — bucket 1 and observer).
fn make_social_card(id: &str) -> CardData {
    make_digimon(id, &["Social"])
}

/// A Digimon with the [Creation] trait.
/// NOTE: [Creation] qualifies for On Play bucket 2 but NOT for the [Your Turn]
/// link observer (printed observer list is Social/Navi/Tool only — verified
/// against the card image + official Bandai DB; cards.json is wrong here).
fn make_creation_card(id: &str) -> CardData {
    make_digimon(id, &["Creation"])
}

/// A Digimon with the [Navi] trait.
fn make_navi_card(id: &str) -> CardData {
    make_digimon(id, &["Navi"])
}

/// A Digimon with the [Tool] trait.
fn make_tool_card(id: &str) -> CardData {
    make_digimon(id, &["Tool"])
}

/// A Digimon with none of the card's relevant traits (neither On Play bucket
/// nor the Social/Navi/Tool observer list).
fn make_irrelevant_card(id: &str) -> CardData {
    make_digimon(id, &["Reptile"])
}

// ─── Runner builder ─────────────────────────────────────────────────────────

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("P-217 YAML parses and compiles")
        .add_card(make_test_card("DECK-PAD", "Filler"))
        .add_card(make_social_card("SOCIAL-X"))
        .add_card(make_creation_card("CREATION-X"))
        .add_card(make_navi_card("NAVI-X"))
        .add_card(make_tool_card("TOOL-X"))
        .add_card(make_plain_digimon("HOST-DIGI"))
        .add_card(make_irrelevant_card("IRREL-X"))
        .deck(1, &["DECK-PAD"; 8])
}

/// Helper: fire an OnLink trigger directly, without the full link-option UI.
/// Simulates the engine's `attach_linked_card` → `enqueue_triggered(OnLink, Linked{...})`
/// path but lets us supply the just-linked card handle without the full
/// standing-permanent absorb flow.
///
/// Steps:
/// 1. Push a card into the host's `linked_cards` via `push_linked_owned` (test helper).
/// 2. Enqueue `EffectTiming::OnLink` with `TriggerSource::Linked { player: pid,
///    host, card }` for BOTH player sides (matching the engine's broadcast).
/// 3. Drain the effect queue.
fn fire_link_event(r: &mut DebugRunner, host: PermanentHandle, linked_card_id: &str) {
    let card = r.push_linked_owned(host, linked_card_id, host.player);
    for pid in 0..2u8 {
        r.game.enqueue_triggered(
            EffectTiming::OnLink,
            TriggerSource::Linked {
                player: pid,
                host,
                card,
            },
        );
    }
    r.game.drain_effect_queue();
}

fn advance_to_main(r: &mut DebugRunner) {
    r.game.enter_main_phase();
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

/// P-217 YAML must compile and be present in the embedded DSL pack.
#[test]
fn p_217_yaml_compiles_without_error() {
    let runner = base().start();
    assert!(
        runner.compiled_card(CARD_ID).is_some(),
        "P-217 must be present in the embedded DSL pack (YAML must parse + compile)"
    );
}

/// Compiled metadata matches the authored card.
#[test]
fn p_217_yaml_printed_metadata() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("P-217 in pack");
    assert_eq!(card.name, "Haru Shinkai");
    assert_eq!(card.level, None, "Tamers have no level");
    assert_eq!(card.dp, None, "Tamers have no DP");
}

/// P-217 must have an [On Play] triggered clause.
#[test]
fn p_217_has_on_play_clause() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("P-217 in pack");
    let has = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnPlay)
        )
    });
    assert!(has, "P-217 must have an [On Play] triggered clause");
}

/// P-217 must have an [On Any Link] triggered clause (the board-wide observer).
#[test]
fn p_217_has_on_any_link_clause() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("P-217 in pack");
    let has = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnAnyLink)
        )
    });
    assert!(
        has,
        "P-217 must have a board-wide [On Any Link] observer clause"
    );
}

/// The [On Any Link] clause must be optional (DCGO is_skippable = true).
#[test]
fn p_217_on_any_link_clause_is_optional() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("P-217 in pack");
    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnAnyLink) => Some(t),
            _ => None,
        })
        .expect("on_any_link clause must exist");
    assert!(
        clause.optional,
        "P-217's on_any_link clause must be optional (DCGO is_skippable=true)"
    );
}

/// P-217 must have a [Security] triggered clause.
#[test]
fn p_217_has_on_security_clause() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("P-217 in pack");
    let has = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSecurity)
        )
    });
    assert!(has, "P-217 must have an [On Security] triggered clause");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — On Play: reveal-3 two-bucket add
// ═══════════════════════════════════════════════════════════════════════════════

/// [On Play] with both buckets matched: adds 1 Social + 1 Navi to hand,
/// returns the third card to deck bottom.
///
/// Top-of-deck order (last = top): [DECK-PAD, SOCIAL-X, NAVI-X]
/// Bucket 1: Social → SOCIAL-X. Bucket 2: Navi → NAVI-X. Remainder: DECK-PAD.
#[test]
fn p_217_on_play_both_buckets_matched_adds_two_returns_one() {
    let mut r = base()
        .hand(0, &[CARD_ID])
        .deck(0, &["DECK-PAD", "SOCIAL-X", "NAVI-X"]) // top = NAVI-X
        .memory(10)
        .start();

    let hand_before = r.hand_size(0);
    let deck_before = r.deck_size(0);

    // Play P-217 from hand index 0.
    r.play(0, 0).expect("P-217 must be playable");
    // Resolve the two mandatory bucket picks; engine clamps so we auto-resolve.
    r.auto_resolve().ok();

    // Net hand: -1 (played tamer) + 2 (Social + Navi) = +1.
    assert_eq!(
        r.hand_size(0),
        hand_before + 1,
        "On Play must add exactly 2 revealed cards net of the played Tamer"
    );
    // Deck: -3 revealed + 1 remainder returned to bottom = -2.
    assert_eq!(
        r.deck_size(0),
        deck_before - 2,
        "Deck must lose 3 revealed cards and gain 1 remainder at bottom"
    );
    // Trash is untouched.
    assert_eq!(r.trash_size(0), 0, "No card must go to trash");
}

/// [On Play] bucket 2 accepts a [Creation] card: adds 1 Social + 1 Creation,
/// returns the third card to deck bottom. ([Creation] qualifies for the On Play
/// clause even though it is NOT in the [Your Turn] observer list.)
#[test]
fn p_217_on_play_creation_qualifies_for_bucket_two() {
    let mut r = base()
        .hand(0, &[CARD_ID])
        .deck(0, &["DECK-PAD", "SOCIAL-X", "CREATION-X"]) // top = CREATION-X
        .memory(10)
        .start();

    let hand_before = r.hand_size(0);
    let deck_before = r.deck_size(0);

    r.play(0, 0).expect("P-217 must be playable");
    r.auto_resolve().ok();

    // Net hand: -1 (played tamer) + 2 (Social + Creation) = +1.
    assert_eq!(
        r.hand_size(0),
        hand_before + 1,
        "[Creation] must qualify for On Play bucket 2 (Social + Creation added)"
    );
    // Deck: -3 revealed + 1 remainder returned to bottom = -2.
    assert_eq!(
        r.deck_size(0),
        deck_before - 2,
        "Deck must lose 3 revealed cards and gain 1 remainder at bottom"
    );
    assert_eq!(r.trash_size(0), 0, "No card must go to trash");
}

/// [On Play] only-Social matched: Bucket 1 adds Social; Bucket 2 has no
/// candidates → clamped, adds nothing. Two cards go to deck bottom.
#[test]
fn p_217_on_play_only_social_matched_adds_one_returns_two() {
    let mut r = base()
        .add_card(make_plain_digimon("PLAIN-A"))
        .add_card(make_plain_digimon("PLAIN-B"))
        .hand(0, &[CARD_ID])
        .deck(0, &["PLAIN-A", "PLAIN-B", "SOCIAL-X"]) // top = SOCIAL-X
        .memory(10)
        .start();

    let hand_before = r.hand_size(0);

    r.play(0, 0).expect("P-217 must be playable");
    r.auto_resolve().ok();

    // Net: -1 played + 1 Social = 0 change.
    assert_eq!(
        r.hand_size(0),
        hand_before,
        "On Play with only Social match must add exactly 1 card net of play"
    );
    assert_eq!(r.trash_size(0), 0, "No card to trash");
}

/// [On Play] none matched: both buckets clamped. All 3 revealed cards go to
/// deck bottom. Hand has no new cards.
#[test]
fn p_217_on_play_none_matched_all_three_to_bottom() {
    let mut r = base()
        .add_card(make_plain_digimon("PLAIN-A"))
        .add_card(make_plain_digimon("PLAIN-B"))
        .add_card(make_plain_digimon("PLAIN-C"))
        .hand(0, &[CARD_ID])
        .deck(0, &["PLAIN-A", "PLAIN-B", "PLAIN-C"])
        .memory(10)
        .start();

    let hand_before = r.hand_size(0);
    let deck_before = r.deck_size(0);

    r.play(0, 0).expect("P-217 must be playable");
    r.auto_resolve().ok();

    // Net: -1 played. No cards added to hand.
    assert_eq!(
        r.hand_size(0),
        hand_before - 1,
        "On Play with no matches must add 0 cards to hand"
    );
    // Deck: -3 revealed + 3 returned = unchanged.
    assert_eq!(
        r.deck_size(0),
        deck_before,
        "All 3 unmatched revealed cards must return to deck bottom"
    );
    assert_eq!(r.trash_size(0), 0, "No card to trash");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Observer: own Digimon linked on your turn — Social trigger
// ═══════════════════════════════════════════════════════════════════════════════

/// [Your Turn] own Digimon linked to a [Social] card → optional prompt.
/// When accepted: Tamer suspends + memory +1.
#[test]
fn p_217_observer_social_link_on_your_turn_accept_suspends_and_gains_memory() {
    let mut r = base().deck(0, &["DECK-PAD"; 8]).memory(3).start();

    // Place P-217 (unsuspended) and a host Digimon on P0's field.
    let tamer = r.place_on_field(0, CARD_ID, None);
    let host = r.place_on_field(0, "HOST-DIGI", None);
    advance_to_main(&mut r);

    let memory_before = r.game.memory;

    // Fire the OnLink event: SOCIAL-X linked to HOST-DIGI.
    fire_link_event(&mut r, host, "SOCIAL-X");

    // A selection prompt must surface (optional clause accepted).
    // Since the observer is optional, the engine presents a Yes/No or the
    // activation_cost suspend_self is the first step — either way a selection
    // is pending. Resolve it (first valid action = accept).
    assert!(
        r.game.pending_selection.is_some(),
        "On-any-link observer (Social) must surface a pending selection"
    );
    let action_id = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    r.game.resolve_selection(0, action_id).ok();
    r.auto_resolve().ok();

    // Tamer must be suspended.
    let tamer_perm = &r.game.player(0).battle_area[tamer.index as usize];
    assert!(
        tamer_perm.is_suspended,
        "P-217 Tamer must be suspended after paying the activation cost"
    );

    // Memory must have increased by 1.
    assert_eq!(
        r.game.memory,
        memory_before + 1,
        "Player must gain 1 memory when the observer fires and the cost is paid"
    );
}

/// [Your Turn] observer fires and player declines: Tamer stays unsuspended,
/// memory unchanged.
#[test]
fn p_217_observer_social_link_on_your_turn_decline_no_effect() {
    let mut r = base().deck(0, &["DECK-PAD"; 8]).memory(3).start();

    let tamer = r.place_on_field(0, CARD_ID, None);
    let host = r.place_on_field(0, "HOST-DIGI", None);
    advance_to_main(&mut r);

    let memory_before = r.game.memory;

    fire_link_event(&mut r, host, "SOCIAL-X");

    if r.game.pending_selection.is_some() {
        // Decline by passing.
        r.game.resolve_selection(0, PASS).ok();
        r.auto_resolve().ok();
    }

    let tamer_perm = &r.game.player(0).battle_area[tamer.index as usize];
    assert!(
        !tamer_perm.is_suspended,
        "Tamer must NOT be suspended when the optional observer is declined"
    );
    assert_eq!(
        r.game.memory, memory_before,
        "Memory must be unchanged when the observer is declined"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — Observer: trait gating
// ═══════════════════════════════════════════════════════════════════════════════

/// [Your Turn] linked card with NONE of the observer traits (Social/Navi/Tool)
/// → observer does NOT fire. No selection, no memory change.
#[test]
fn p_217_observer_irrelevant_trait_does_not_fire() {
    let mut r = base().deck(0, &["DECK-PAD"; 8]).memory(3).start();

    let _tamer = r.place_on_field(0, CARD_ID, None);
    let host = r.place_on_field(0, "HOST-DIGI", None);
    advance_to_main(&mut r);

    let memory_before = r.game.memory;

    // Link a [Reptile] card — not Social/Navi/Tool (and not an On Play trait).
    fire_link_event(&mut r, host, "IRREL-X");

    assert!(
        r.game.pending_selection.is_none(),
        "Observer must NOT fire when the linked card has no matching trait"
    );
    assert_eq!(
        r.game.memory, memory_before,
        "Memory must be unchanged when linked card has no matching trait"
    );
}

/// [Your Turn] linked card has [Creation] trait → observer does NOT fire.
/// The printed observer list is [Social], [Navi] or [Tool] ONLY (verified on
/// the card image + official Bandai DB bundle; DCGO agrees). [Creation] is an
/// On Play-bucket-2 trait, not a link-observer trait. cards.json wrongly adds
/// it — this test guards against re-importing that drift.
#[test]
fn p_217_observer_creation_trait_does_not_fire() {
    let mut r = base().deck(0, &["DECK-PAD"; 8]).memory(3).start();

    let tamer = r.place_on_field(0, CARD_ID, None);
    let host = r.place_on_field(0, "HOST-DIGI", None);
    advance_to_main(&mut r);

    let memory_before = r.game.memory;

    // Link a [Creation] card — not in the printed observer trait list.
    fire_link_event(&mut r, host, "CREATION-X");

    assert!(
        r.game.pending_selection.is_none(),
        "Observer must NOT fire for [Creation] trait — printed [Your Turn] list \
         is Social/Navi/Tool only (cards.json wrongly adds Creation)"
    );
    let tamer_perm = &r.game.player(0).battle_area[tamer.index as usize];
    assert!(
        !tamer_perm.is_suspended,
        "Tamer must stay unsuspended when a [Creation] card is linked"
    );
    assert_eq!(
        r.game.memory, memory_before,
        "Memory must be unchanged when a [Creation] card is linked"
    );
}

/// [Your Turn] linked card has [Navi] trait → observer fires.
#[test]
fn p_217_observer_navi_trait_fires() {
    let mut r = base().deck(0, &["DECK-PAD"; 8]).memory(3).start();

    let tamer = r.place_on_field(0, CARD_ID, None);
    let host = r.place_on_field(0, "HOST-DIGI", None);
    advance_to_main(&mut r);

    fire_link_event(&mut r, host, "NAVI-X");

    assert!(
        r.game.pending_selection.is_some(),
        "Observer must fire when linked card has [Navi] trait"
    );
}

/// [Your Turn] linked card has [Tool] trait → observer fires.
#[test]
fn p_217_observer_tool_trait_fires() {
    let mut r = base().deck(0, &["DECK-PAD"; 8]).memory(3).start();

    let tamer = r.place_on_field(0, CARD_ID, None);
    let host = r.place_on_field(0, "HOST-DIGI", None);
    advance_to_main(&mut r);

    fire_link_event(&mut r, host, "TOOL-X");

    assert!(
        r.game.pending_selection.is_some(),
        "Observer must fire when linked card has [Tool] trait"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 5 — Observer: turn and ownership gating
// ═══════════════════════════════════════════════════════════════════════════════

/// Opponent's turn: even when P1's Digimon gets linked to a [Social] card, P0's
/// P-217 must NOT fire — `your_turn: true` blocks it.
#[test]
fn p_217_observer_does_not_fire_on_opponents_turn() {
    let mut r = base().deck(0, &["DECK-PAD"; 8]).memory(3).start();

    let _tamer = r.place_on_field(0, CARD_ID, None);
    // Place a Digimon on P0's field that will be the linked host.
    let p0_host = r.place_on_field(0, "HOST-DIGI", None);

    // Switch to opponent's turn.
    r.end_turn();
    r.game.enter_main_phase();

    let memory_before = r.game.memory;

    // Fire OnLink as if it's P0's host being linked — but it's P1's turn.
    // The on_any_link observer's `your_turn: true` must suppress P0's clause.
    let card = r.push_linked_owned(p0_host, "SOCIAL-X", 0);
    for pid in 0..2u8 {
        r.game.enqueue_triggered(
            EffectTiming::OnLink,
            TriggerSource::Linked {
                player: pid,
                host: p0_host,
                card,
            },
        );
    }
    r.game.drain_effect_queue();

    assert!(
        r.game.pending_selection.is_none(),
        "P-217 observer must NOT fire on opponent's turn"
    );
    assert_eq!(
        r.game.memory, memory_before,
        "Memory must be unchanged when observer suppressed by opponent's turn"
    );
}

/// Opponent's Digimon linked to a [Social] card on P0's turn: P0's P-217
/// must NOT fire — `event_target_owner: you` blocks it.
#[test]
fn p_217_observer_does_not_fire_for_opponent_digimon_link() {
    let mut r = base().deck(0, &["DECK-PAD"; 8]).memory(3).start();

    let _tamer = r.place_on_field(0, CARD_ID, None);
    // Opponent's Digimon.
    let opp_host = r.place_on_field(1, "HOST-DIGI", None);
    advance_to_main(&mut r);

    let memory_before = r.game.memory;

    // Fire OnLink with the host being P1's Digimon (owner = 1).
    // P0's observer must NOT fire because event_target_owner != you.
    let card = r.push_linked_owned(opp_host, "SOCIAL-X", 1);
    for pid in 0..2u8 {
        r.game.enqueue_triggered(
            EffectTiming::OnLink,
            TriggerSource::Linked {
                player: pid,
                host: opp_host,
                card,
            },
        );
    }
    r.game.drain_effect_queue();

    assert!(
        r.game.pending_selection.is_none(),
        "P-217 observer must NOT fire when the link host belongs to the opponent"
    );
    assert_eq!(
        r.game.memory, memory_before,
        "Memory must be unchanged when opponent's Digimon is the link host"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 6 — Observer: activation cost — already suspended Tamer
// ═══════════════════════════════════════════════════════════════════════════════

/// If P-217 is already suspended, the activation cost (suspend_self) cannot be
/// paid → the observer prompt must not surface, or must be unchoosable (PASS
/// only, cost unpayable).
#[test]
fn p_217_observer_already_suspended_tamer_cannot_pay_cost() {
    let mut r = base().deck(0, &["DECK-PAD"; 8]).memory(3).start();

    let tamer = r.place_on_field(0, CARD_ID, None);
    let host = r.place_on_field(0, "HOST-DIGI", None);
    advance_to_main(&mut r);

    // Pre-suspend the Tamer.
    r.game.players[0].battle_area[tamer.index as usize].is_suspended = true;

    let memory_before = r.game.memory;

    fire_link_event(&mut r, host, "SOCIAL-X");

    // Either no selection surfaces (cost gate suppressed), or the selection
    // has no valid non-PASS actions (cost unpayable → only PASS offered).
    let selection_ok = match r.game.pending_selection.as_ref() {
        None => true, // cost gated — no prompt at all
        Some(sel) => {
            // Only PASS is valid (no suspend action available)
            sel.valid_action_ids.iter().all(|&id| id == PASS)
        }
    };
    assert!(
        selection_ok,
        "A suspended P-217 Tamer must not be able to pay the suspend activation cost"
    );

    // Memory unchanged regardless.
    let tamer_perm = &r.game.player(0).battle_area[tamer.index as usize];
    assert!(
        tamer_perm.is_suspended,
        "Tamer must still be suspended (cost couldn't be paid)"
    );
    assert_eq!(
        r.game.memory, memory_before,
        "Memory must not change when cost cannot be paid"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 7 — Security: play this card without paying the cost
// ═══════════════════════════════════════════════════════════════════════════════

/// [Security] P-217 in security → resolving its security effect plays the Tamer
/// onto the controller's field without paying the cost.
///
/// Test pattern: put an Attacker on P0's field, P-217 in P1's security,
/// P0 attacks P1 → security check → P-217 plays onto P1's field for free.
#[test]
fn p_217_security_plays_tamer_free() {
    let mut r = base()
        .add_card(make_test_card("ATTACKER", "Attacker"))
        // P-217 in P1's security stack.
        .security(1, &[CARD_ID])
        .deck(0, &["DECK-PAD"; 4])
        .deck(1, &["DECK-PAD"; 4])
        .memory(10)
        .start();

    let attacker = r.place_on_field(0, "ATTACKER", Some(0));

    let field_before = r.battle_area_size(1);
    let security_before = r.security_count(1);
    assert_eq!(security_before, 1, "P1 starts with 1 security (P-217)");

    // P0 attacks P1's security — security check fires, P-217's [Security] effect runs.
    r.attack_player(attacker, 1, false);
    r.auto_resolve().ok();

    // Security card must have been consumed.
    assert_eq!(
        r.security_count(1),
        0,
        "Security stack must be empty after check"
    );

    // P-217 must be on P1's battle area (played free by [Security] effect).
    assert_eq!(
        r.battle_area_size(1),
        field_before + 1,
        "[Security] must play P-217 onto P1's field without paying its cost"
    );
}
