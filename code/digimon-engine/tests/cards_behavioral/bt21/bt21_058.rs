//! BT21-058 Snatchmon — Digimon, Lv.4, Black, DP 6000, Cost 6.
//! Traits: Unknown, LIBERATOR. Attribute: Unknown. Form: Champion.
//!
//! # Printed card text (official Bandai DB bundle data/card_bundles/BT21-058.md,
//! cross-checked against the card image Card/BT21-058.webp)
//!
//! ```text
//! [On Play] [When Digivolving] Reveal the top 3 cards of your deck. Add 1
//! card with [Vemmon] in its text among them to the hand. Trash the rest.
//! Then, you may place up to 2 [Vemmon] from your trash as 1 of your
//! Digimon's bottom digivolution cards.
//!
//! Inherited [All Turns] [Once Per Turn] When any [Vemmon] are returned to
//! the bottom of the deck from this Digimon's digivolution cards, delete 1
//! of your opponent's Digimon with a play cost of 4 or less.
//! ```
//!
//! Standard digivolve circle: Black Lv.3 / cost 3 (single circle; no
//! DigiXros box printed on this card — see YAML file header for the
//! contrast against the BT18-065 reprint of Snatchmon, which DOES have one).
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT21/Black/BT21_058.cs (present in the
//! working DCGO checkout). Cross-checked: `SimplifiedRevealDeckTopCardsAndSelect`
//! (mandatory hand bucket) matches the reveal-search shape; the trash-placement
//! select block is gated on `HasMatchConditionOwnersCardInTrash(card, IsVemmon)`
//! BEFORE any prompt surfaces (modeled via an `if: count_gte` existence gate
//! wrapping the whole placement tail); each trash pick is `canNoSelect: true`
//! (optional); the inherited clause matches DCGO's
//! `OnDigivolutionCardReturnToDeckBottom` + `SetIsInheritedEffect(true)` +
//! `SetHashString(...)` OPT key 1:1 via `scope: inherited` +
//! `once_per_turn: true`. See
//! `code/digimon-engine/tests/source_returned_to_deck_bottom_observer.rs`
//! and the `OnSourceReturnedToDeckBottom` timing doc-comment in
//! `code/digimon-dsl/src/clause.rs`, both of which name BT21-058 as the
//! consumer.
//!
//! Note: DCGO auto-targets the sole own Digimon (skips the target prompt)
//! when the owner has exactly 1 battle-area Digimon
//! (`Count(HasDigimonOnOwnerBattleArea) > 1` gate); per rule 17 (no
//! auto-selections) this DSL intentionally does NOT replicate that
//! UI shortcut — it always offers the optional target pick once >=1 Vemmon
//! is in trash, which is strictly more exposed to the RL action space and
//! never changes the outcome (only 1 legal target exists in that case).
//!
//! # Patterns covered
//! - Section 1: Structural — 2 compiled clauses, reveal_search shape,
//!   inherited-observer shape, if/count_gte existence-gate wrap.
//! - Section 2: Behavioral — [On Play]/[When Digivolving] reveal-3
//!   hand-1/trash-rest, then optional place-up-to-2-from-trash, INCLUDING
//!   the whole-tail existence gate (no Vemmon in trash -> no prompt at all,
//!   not even the target pick).
//! - Section 3: Behavioral — inherited delete-on-Vemmon-returned-to-deck-
//!   bottom observer: positive, name-gate negative, host-scope negative,
//!   OPT lockout + clear-next-turn, no-eligible-target fizzle.

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledPredicate, CompiledRevealSearchDest,
    CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::CardKind;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "BT21-058";

// ─── Card factories ─────────────────────────────────────────────────────────

fn make_filler(id: &str) -> CardData {
    make_test_card(id, &format!("{id}-Filler"))
}

/// A card with "[Vemmon]" in its EFFECT TEXT but NOT named "Vemmon" — only
/// eligible for the hand bucket (broad `in_text_contains` scan).
fn make_vemmon_text_card(id: &str) -> CardData {
    let mut c = make_test_card(id, &format!("{id}-VemmonText"));
    c.effect_text = "This card synergizes with [Vemmon].".to_string();
    c
}

/// An exact-name "Vemmon" card with NO text mention — eligible only via
/// exact-name matching (used for the trash-placement bucket, not the reveal
/// hand bucket, since `in_text_contains` also scans the NAME and would match
/// a bare "Vemmon"-named card too; kept distinct so trash-placement tests
/// don't accidentally interact with the reveal step).
fn make_exact_vemmon(id: &str) -> CardData {
    let mut c = make_test_card(id, "Vemmon");
    c.effect_text = String::new();
    let _ = id; // id only used for registry lookup distinction via card_id
    c
}

fn make_digimon(id: &str, level: u8, dp: i32, cost: u16) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = cost;
    c
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT21-058 YAML parses and compiles")
}

/// Push `ids` onto player `p`'s deck TOP, in order (last id ends up as the
/// topmost / first-revealed card).
fn stack_deck_top(runner: &mut DebugRunner, p: usize, ids: &[&str]) {
    for id in ids {
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == *id)
            .unwrap_or_else(|| panic!("card {id} registered"));
        let card_index = runner.game.next_card_index();
        runner.game.players[p]
            .deck
            .push(CardSource::new(data_idx, p as u8, card_index));
    }
}

fn push_to_trash(runner: &mut DebugRunner, p: usize, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("{card_id} in card_data"));
    let card_index = runner.game.next_card_index();
    runner.game.players[p]
        .trash
        .push(CardSource::new(data_idx, p as u8, card_index));
}

/// Drive every pending selection by picking the FIRST valid action id, up to
/// `max_steps` iterations.
fn drive_first_choice_n_times(runner: &mut DebugRunner, max_steps: usize) {
    for _ in 0..max_steps {
        let Some(sel) = runner.pending_selection() else {
            return;
        };
        let action = sel
            .valid_action_ids
            .first()
            .copied()
            .expect("selection has at least one valid action");
        let _ = runner.execute_action(0, action);
    }
}

/// Decline the current pending selection (PASS), up to `max_steps` times, IF
/// the current selection is optional and offers PASS. Stops early once no
/// selection is pending.
fn decline_n_times(runner: &mut DebugRunner, max_steps: usize) {
    for _ in 0..max_steps {
        let Some(sel) = runner.pending_selection() else {
            return;
        };
        assert!(sel.is_optional, "expected an optional (declinable) prompt");
        let player = sel.selecting_player;
        let _ = runner.execute_action(player, PASS);
    }
}

/// Build a permanent stack where Snatchmon (BT21-058) is BURIED as a
/// digivolution source (never the face-up top), with `sources_above` stacked
/// above it (the LAST id is the face-up top). This matches the engine's
/// `effect.inherited` gate (`code/digimon-engine/src/effect_queue.rs`: "The
/// card scanned here is the carrier's *top* card, never a source, so its own
/// inherited clauses stay dormant" — a `scope: inherited` clause on
/// Snatchmon's own printed card ONLY fires while Snatchmon itself is a
/// digivolution source beneath something else) and the
/// `source_returned_to_deck_bottom_observer.rs` proof's `seed_stack`
/// (observer card at the BOTTOM, something else on TOP). Returns the
/// permanent handle.
fn place_buried_snatchmon(r: &mut DebugRunner, sources_above: &[&str]) -> PermanentHandle {
    assert!(
        !sources_above.is_empty(),
        "need at least 1 card stacked above Snatchmon so it is buried, not face-up"
    );
    let mut ids: Vec<&str> = vec![CARD_ID];
    ids.extend_from_slice(sources_above);
    r.place_stack(0, &ids)
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn bt21_058_yaml_printed_metadata() {
    let r = base()
        .add_card(make_filler("PAD"))
        .deck(0, &["PAD"; 12])
        .start();
    let card = r.compiled_card(CARD_ID).expect("present in pack");
    assert_eq!(card.name, "Snatchmon");
    assert_eq!(card.level, Some(4));
    assert_eq!(card.dp, Some(6000));
    assert_eq!(card.cost, Some(6));
}

#[test]
fn bt21_058_traits_include_unknown_liberator() {
    let r = base()
        .add_card(make_filler("PAD"))
        .deck(0, &["PAD"; 12])
        .start();
    let card = r.compiled_card(CARD_ID).expect("present in pack");
    assert!(
        card.traits.iter().any(|t| t == "Unknown"),
        "must have Unknown trait"
    );
    assert!(
        card.traits.iter().any(|t| t == "LIBERATOR"),
        "must have LIBERATOR trait"
    );
}

#[test]
fn bt21_058_compiles_to_two_clauses() {
    let r = base()
        .add_card(make_filler("PAD"))
        .deck(0, &["PAD"; 12])
        .start();
    let card = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(
        card.effects.len(),
        2,
        "BT21-058 must compile to exactly 2 clauses: [On Play][When Digivolving] \
         triggered + inherited on_source_returned_to_deck_bottom triggered; got {}",
        card.effects.len()
    );
}

#[test]
fn bt21_058_clause1_fires_on_both_play_and_digivolve() {
    let r = base()
        .add_card(make_filler("PAD"))
        .deck(0, &["PAD"; 12])
        .start();
    let card = r.compiled_card(CARD_ID).expect("compiled");
    let clause1 = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnPlay) => Some(t),
            _ => None,
        })
        .expect("must have a Triggered clause containing OnPlay");
    assert!(
        clause1.when.contains(&CompiledTiming::OnDigivolve),
        "[On Play][When Digivolving] clause must ALSO carry OnDigivolve timing"
    );
    assert_eq!(
        clause1.scope,
        CompiledScope::FaceUp,
        "clause 1 must be FaceUp scope (not inherited)"
    );
}

#[test]
fn bt21_058_clause1_reveal_search_has_hand_bucket_and_trash_catchall() {
    let r = base()
        .add_card(make_filler("PAD"))
        .deck(0, &["PAD"; 12])
        .start();
    let card = r.compiled_card(CARD_ID).expect("compiled");
    let clause1 = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnPlay) => Some(t),
            _ => None,
        })
        .expect("clause 1 present");

    let reveal_search = clause1
        .process
        .iter()
        .find_map(|s| match s {
            CompiledStep::RevealSearch {
                count,
                buckets,
                remainder,
                ..
            } => Some((*count, buckets.clone(), *remainder)),
            _ => None,
        })
        .expect("clause 1 must contain a reveal_search step");

    let (count, buckets, _remainder) = reveal_search;
    assert_eq!(count, 3, "must reveal the top 3 cards");
    assert_eq!(
        buckets.len(),
        2,
        "must have exactly 2 buckets: hand (Vemmon-in-text) + trash (catch-all)"
    );
    assert_eq!(
        buckets[0].to,
        CompiledRevealSearchDest::Hand,
        "bucket 1 must route to hand"
    );
    assert_eq!(buckets[0].max, 1, "bucket 1 adds exactly 1 card to hand");
    assert!(
        !buckets[0].optional,
        "bucket 1 (add to hand) is mandatory per printed text (no \"may\")"
    );
    assert_eq!(
        buckets[1].to,
        CompiledRevealSearchDest::Trash,
        "bucket 2 (the rest) must route to trash"
    );
}

/// Recursively search a step list (descending into `If` then/else branches)
/// for any step matching `pred`. The trash-placement tail lives inside an
/// `if:` existence-gate wrapper (DCGO's `HasMatchConditionOwnersCardInTrash`
/// gate), so a flat top-level scan of `process` is not sufficient.
fn any_step_recursive(steps: &[CompiledStep], pred: &impl Fn(&CompiledStep) -> bool) -> bool {
    steps.iter().any(|s| {
        if pred(s) {
            return true;
        }
        match s {
            CompiledStep::If {
                then, else_branch, ..
            } => any_step_recursive(then, pred) || any_step_recursive(else_branch, pred),
            _ => false,
        }
    })
}

#[test]
fn bt21_058_clause1_has_optional_trash_placement_tail() {
    let r = base()
        .add_card(make_filler("PAD"))
        .deck(0, &["PAD"; 12])
        .start();
    let card = r.compiled_card(CARD_ID).expect("compiled");
    let clause1 = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnPlay) => Some(t),
            _ => None,
        })
        .expect("clause 1 present");

    let has_place_as_bottom_source = any_step_recursive(&clause1.process, &|s| {
        matches!(s, CompiledStep::PlaceAsBottomSource { .. })
    });
    assert!(
        has_place_as_bottom_source,
        "clause 1 must contain place_as_bottom_source steps for the trash placement tail"
    );

    let select_own_permanent_optional = any_step_recursive(&clause1.process, &|s| {
        matches!(
            s,
            CompiledStep::SelectOwnPermanent { optional, .. } if *optional
        )
    });
    assert!(
        select_own_permanent_optional,
        "the target-Digimon pick must be optional (\"you may place...\")"
    );
}

/// The trash-placement tail must be wrapped in an `if:` existence-gate on
/// "at least 1 [Vemmon] in your trash" — DCGO's whole-block
/// `HasMatchConditionOwnersCardInTrash` gate (skips even the target pick
/// when trash has zero eligible Vemmon).
#[test]
fn bt21_058_clause1_placement_tail_is_gated_on_vemmon_in_trash_existence() {
    let r = base()
        .add_card(make_filler("PAD"))
        .deck(0, &["PAD"; 12])
        .start();
    let card = r.compiled_card(CARD_ID).expect("compiled");
    let clause1 = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnPlay) => Some(t),
            _ => None,
        })
        .expect("clause 1 present");

    let has_gate = clause1.process.iter().any(|s| {
        let CompiledStep::If { condition, then, .. } = s else {
            return false;
        };
        let gates_on_trash_vemmon_count = condition
            .count_gte
            .as_ref()
            .map(|agg| {
                agg.filter.zone.contains(&digimon_dsl::compiled::CompiledZone::Trash)
                    && agg.filter.name_is.as_deref() == Some("Vemmon")
            })
            .unwrap_or(false);
        let wraps_the_placement_tail = then
            .iter()
            .any(|inner| matches!(inner, CompiledStep::PlaceAsBottomSource { .. }));
        gates_on_trash_vemmon_count && wraps_the_placement_tail
    });
    assert!(
        has_gate,
        "clause 1 must wrap the trash-placement tail in an if/count_gte \
         existence gate (filter: zone=trash, name_is=Vemmon, n=1), \
         matching DCGO's HasMatchConditionOwnersCardInTrash whole-block gate"
    );
}

#[test]
fn bt21_058_clause2_is_inherited_all_turns_once_per_turn() {
    let r = base()
        .add_card(make_filler("PAD"))
        .deck(0, &["PAD"; 12])
        .start();
    let card = r.compiled_card(CARD_ID).expect("compiled");
    let clause2 = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when
                    .contains(&CompiledTiming::OnDigivolutionCardReturnedToDeckBottom) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("must have a Triggered clause on OnDigivolutionCardReturnedToDeckBottom");

    assert_eq!(
        clause2.scope,
        CompiledScope::Inherited,
        "clause 2 must be Inherited scope"
    );
    assert!(
        clause2.once_per_turn,
        "[Once Per Turn] printed qualifier must compile to once_per_turn: true"
    );

    fn has_all_turns(p: &CompiledPredicate) -> bool {
        p.all_turns == Some(true)
            || p.all_of.iter().any(has_all_turns)
            || p.any_of.iter().any(has_all_turns)
    }
    let aw = clause2
        .active_when
        .as_ref()
        .expect("clause 2 must carry active_when for [All Turns]");
    assert!(has_all_turns(aw), "must carry all_turns: true gate");
}

#[test]
fn bt21_058_clause2_gates_on_host_scope_and_vemmon_name() {
    let r = base()
        .add_card(make_filler("PAD"))
        .deck(0, &["PAD"; 12])
        .start();
    let card = r.compiled_card(CARD_ID).expect("compiled");
    let clause2 = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when
                    .contains(&CompiledTiming::OnDigivolutionCardReturnedToDeckBottom) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("clause 2 present");

    fn has_host_scope(p: &CompiledPredicate) -> bool {
        p.event_host_permanent_is_source == Some(true)
            || p.all_of.iter().any(has_host_scope)
            || p.any_of.iter().any(has_host_scope)
    }
    fn has_vemmon_name_gate(p: &CompiledPredicate) -> bool {
        p.event_card_name_contains.as_deref() == Some("Vemmon")
            || p.all_of.iter().any(has_vemmon_name_gate)
            || p.any_of.iter().any(has_vemmon_name_gate)
    }

    let cond = clause2
        .condition
        .as_ref()
        .expect("clause 2 must carry a condition");
    assert!(
        has_host_scope(cond),
        "must gate on event_host_permanent_is_source: true (\"from THIS Digimon's \
         digivolution cards\")"
    );
    assert!(
        has_vemmon_name_gate(cond),
        "must gate on event_card_name_contains: Vemmon (\"any [Vemmon]\")"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 2 — Behavioral: [On Play][When Digivolving] reveal / hand / trash
// ═══════════════════════════════════════════════════════════════════════════

/// POSITIVE: playing Snatchmon reveals top 3; the sole [Vemmon]-text card
/// goes to hand; the other 2 are trashed (not returned to deck).
#[test]
fn bt21_058_on_play_adds_vemmon_text_card_to_hand_and_trashes_rest() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("parses")
        .add_card(make_vemmon_text_card("F058-TEXT"))
        .add_card(make_filler("F058-A"))
        .add_card(make_filler("F058-B"))
        .hand(0, &[CARD_ID])
        .deck(0, &["F058-B", "F058-A", "F058-TEXT"])
        .memory(10)
        .start();

    let hand_before = r.game.players[0].hand.len();
    let trash_before = r.trash_size(0);
    let deck_before = r.game.players[0].deck.len();

    let played = r.play(0, 0);
    assert!(played.is_some(), "BT21-058 must be playable");

    // Drive: hand-bucket pick, trash-bucket catch-all, target decline
    // (no trash Vemmon exists yet, so the placement tail must fizzle
    // cleanly whether accepted or declined) — drive first-choice
    // exhaustively; the target select_own_permanent has candidates (the
    // just-played Snatchmon itself), so accept it, then the trash picks
    // naturally fizzle (empty trash).
    drive_first_choice_n_times(&mut r, 20);
    let _ = r.auto_resolve();

    let hand_ids: Vec<&str> = r.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&r.game.card_data))
        .collect();
    assert!(
        hand_ids.contains(&"F058-TEXT"),
        "the [Vemmon]-in-text card must be added to hand: {hand_ids:?}"
    );
    // hand_before included BT21-058 itself which left the hand on play, then
    // +1 for F058-TEXT.
    assert_eq!(
        r.game.players[0].hand.len(),
        hand_before - 1 + 1,
        "BT21-058 leaves hand (played), F058-TEXT added"
    );
    assert_eq!(
        r.trash_size(0),
        trash_before + 2,
        "the other 2 revealed cards (F058-A, F058-B) must be trashed, not returned to deck"
    );
    assert_eq!(
        r.game.players[0].deck.len(),
        deck_before - 3,
        "all 3 revealed cards left the deck (1 to hand, 2 to trash), none returned"
    );
}

/// NEGATIVE: no [Vemmon]-in-text card among the 3 revealed — the hand
/// bucket fizzles (nothing added to hand) and ALL 3 revealed cards are
/// trashed by the catch-all bucket.
#[test]
fn bt21_058_on_play_no_vemmon_text_card_trashes_all_three() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("parses")
        .add_card(make_filler("F058-P1"))
        .add_card(make_filler("F058-P2"))
        .add_card(make_filler("F058-P3"))
        .hand(0, &[CARD_ID])
        .deck(0, &["F058-P3", "F058-P2", "F058-P1"])
        .memory(10)
        .start();

    let hand_before_play = r.game.players[0].hand.len();
    let trash_before = r.trash_size(0);
    let deck_before = r.game.players[0].deck.len();

    let played = r.play(0, 0);
    assert!(played.is_some());
    drive_first_choice_n_times(&mut r, 20);
    let _ = r.auto_resolve();

    assert_eq!(
        r.game.players[0].hand.len(),
        hand_before_play - 1,
        "hand bucket fizzled — only BT21-058 leaving the hand changes hand size"
    );
    assert_eq!(
        r.trash_size(0),
        trash_before + 3,
        "all 3 revealed cards must be trashed by the catch-all bucket"
    );
    assert_eq!(
        r.game.players[0].deck.len(),
        deck_before - 3,
        "all 3 revealed cards left the deck permanently (trashed, not returned)"
    );
}

/// POSITIVE trash-placement: after the reveal step, with 2 exact-name
/// "Vemmon" cards already sitting in trash, accepting the target pick and
/// both trash picks places BOTH as bottom digivolution sources of the
/// chosen Digimon.
#[test]
fn bt21_058_optional_tail_places_up_to_2_vemmon_from_trash() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("parses")
        .add_card(make_exact_vemmon("F058-V1"))
        .add_card(make_exact_vemmon("F058-V2"))
        .add_card(make_filler("F058-DECKPAD"))
        .hand(0, &[CARD_ID])
        .deck(0, &["F058-DECKPAD"; 3])
        .memory(10)
        .start();

    push_to_trash(&mut r, 0, "F058-V1");
    push_to_trash(&mut r, 0, "F058-V2");

    let played = r.play(0, 0);
    assert!(played.is_some());

    // Drive the mandatory reveal-search buckets (hand fizzles — no Vemmon
    // text among 3 filler cards; catch-all trashes all 3), then accept the
    // target pick + both trash picks (first-choice driving accepts, since
    // valid_action_ids is non-empty for each optional-but-available step).
    drive_first_choice_n_times(&mut r, 20);
    let _ = r.auto_resolve();

    let snatchmon = PermanentHandle {
        player: 0,
        index: 0,
    };
    let perm = &r.game.players[0].battle_area[snatchmon.index as usize];
    let source_ids: Vec<&str> = perm
        .card_sources
        .iter()
        .map(|c| c.card_id(&r.game.card_data))
        .collect();
    assert!(
        source_ids.contains(&"F058-V1") && source_ids.contains(&"F058-V2"),
        "both trash Vemmon cards must be placed as bottom digivolution sources: {source_ids:?}"
    );
    assert_eq!(
        r.trash_size(0),
        3,
        "both Vemmon left trash (placed as sources); the 3 revealed fillers were trashed \
         by the reveal step, net trash = 3 filler - 0 (Vemmon moved out, not counted as \
         ever having been added by reveal)"
    );
}

/// NEGATIVE trash-placement: declining the target pick skips the ENTIRE
/// placement tail — no Vemmon is moved from trash even though eligible
/// candidates exist.
#[test]
fn bt21_058_declining_target_pick_skips_entire_placement_tail() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("parses")
        .add_card(make_exact_vemmon("F058-VD"))
        .add_card(make_filler("F058-DECKPAD"))
        .hand(0, &[CARD_ID])
        .deck(0, &["F058-DECKPAD"; 3])
        .memory(10)
        .start();

    push_to_trash(&mut r, 0, "F058-VD");

    let played = r.play(0, 0);
    assert!(played.is_some());

    // Drive the mandatory reveal-search bucket picks first (first-choice
    // driving is safe here since both reveal buckets are mandatory), then
    // decline the target select_own_permanent (PASS) once reached. The deck
    // is all filler (no [Vemmon]-in-text card), so bucket 1 (max 1) has 0
    // candidates and auto-skips with no pick; bucket 2 (catch-all, max 3)
    // claims all 3 revealed fillers — 3 picks total.
    drive_first_choice_n_times(&mut r, 3);

    // Now the target-Digimon select_own_permanent should be pending.
    assert_eq!(
        r.pending_kind(),
        Some(SelectionKind::OwnField),
        "expected the optional target-Digimon pick to be pending"
    );
    let sel = r.pending_selection().expect("pending");
    assert!(sel.is_optional, "target pick must be optional (\"you may\")");
    let player = sel.selecting_player;
    r.execute_action(player, PASS).expect("decline succeeds");

    let _ = r.auto_resolve();

    let trash_ids: Vec<&str> = r.game.players[0]
        .trash
        .iter()
        .map(|c| c.card_id(&r.game.card_data))
        .collect();
    assert!(
        trash_ids.contains(&"F058-VD"),
        "the Vemmon must remain in trash — declining the target pick must skip the \
         entire placement tail: {trash_ids:?}"
    );
}

/// Own-battle-area Digimon target pick must offer Snatchmon itself as a
/// valid target (single own Digimon on the field after playing).
#[test]
fn bt21_058_target_pick_offers_own_snatchmon_permanent() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("parses")
        .add_card(make_exact_vemmon("F058-VT"))
        .add_card(make_filler("F058-DECKPAD"))
        .hand(0, &[CARD_ID])
        .deck(0, &["F058-DECKPAD"; 3])
        .memory(10)
        .start();

    push_to_trash(&mut r, 0, "F058-VT");

    let played = r.play(0, 0);
    assert!(played.is_some());
    // Deck is all filler: bucket 1 (Vemmon-in-text, max 1) has 0 candidates
    // and auto-skips; bucket 2 (catch-all, max 3) claims all 3 fillers.
    drive_first_choice_n_times(&mut r, 3);

    assert_eq!(r.pending_kind(), Some(SelectionKind::OwnField));
    let sel = r.pending_selection().expect("pending");
    assert!(
        !sel.valid_action_ids.is_empty(),
        "Snatchmon itself must be a legal target for the bottom-source placement"
    );
}

/// NEGATIVE (whole-tail existence gate): with ZERO [Vemmon] anywhere in
/// trash, DCGO's `HasMatchConditionOwnersCardInTrash(card, IsVemmon)` gate
/// skips the ENTIRE placement block — not even the target-Digimon pick
/// should surface. After the mandatory reveal-search resolves (no eligible
/// trash Vemmon to place), there must be no pending selection left at all.
#[test]
fn bt21_058_no_vemmon_in_trash_skips_entire_placement_tail_including_target_pick() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("parses")
        .add_card(make_filler("F058-NOVEM-A"))
        .add_card(make_filler("F058-NOVEM-B"))
        .add_card(make_filler("F058-NOVEM-C"))
        .hand(0, &[CARD_ID])
        .deck(0, &["F058-NOVEM-C", "F058-NOVEM-B", "F058-NOVEM-A"])
        .memory(10)
        .start();

    // Trash starts EMPTY of any [Vemmon] card (no push_to_trash call).
    let played = r.play(0, 0);
    assert!(played.is_some());

    // Drive the mandatory reveal-search buckets: bucket 1 (Vemmon-in-text,
    // max 1) has 0 candidates among the 3 filler cards and auto-skips;
    // bucket 2 (catch-all, max 3) claims all 3 fillers to trash.
    drive_first_choice_n_times(&mut r, 3);

    assert!(
        r.pending_selection().is_none(),
        "with no [Vemmon] anywhere in trash, the entire \"place up to 2 from \
         trash\" tail (including the target-Digimon pick) must be skipped \
         entirely — DCGO gates the whole select block on \
         HasMatchConditionOwnersCardInTrash before offering any prompt"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 3 — Behavioral: inherited delete-on-Vemmon-returned observer
// ═══════════════════════════════════════════════════════════════════════════

/// POSITIVE: a [Vemmon] digivolution source is returned to the bottom of
/// the deck FROM the SAME permanent Snatchmon is buried in (Snatchmon
/// itself buried, per the engine's `effect.inherited` gate — its own
/// `scope: inherited` clause is dormant while face-up) → the inherited
/// observer fires, surfacing a delete-target selection over eligible
/// (cost <= 4) opponent Digimon.
#[test]
fn bt21_058_inherited_observer_fires_on_own_vemmon_returned_to_deck_bottom() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("parses")
        .add_card(make_exact_vemmon("F058-SRC"))
        .add_card(make_digimon("HOST-TOP", 5, 5000, 5))
        .add_card(make_digimon("OPP-CHEAP", 3, 3000, 4))
        .add_card(make_filler("F058-DECKPAD"))
        .deck(0, &["F058-DECKPAD"; 5])
        .memory(10)
        .start();
    r.game.enter_main_phase();

    // Stack (bottom -> top): BT21-058 (buried) / F058-SRC (Vemmon) / HOST-TOP.
    let host = place_buried_snatchmon(&mut r, &["F058-SRC", "HOST-TOP"]);
    let vemmon_source = {
        let perm = &r.game.players[0].battle_area[host.index as usize];
        perm.card_sources[1].handle() // index 0 = BT21-058, index 1 = F058-SRC
    };
    r.place_on_field(1, "OPP-CHEAP", None);
    let opp_before = r.game.players[1].battle_area.len();

    {
        let top = r.top_card(host);
        let mut ctx = digimon_engine::effect_context::EffectContext::new(&mut r.game, top, Some(host), 0);
        assert!(
            ctx.return_card_source_to_deck(host, vemmon_source, true),
            "the Vemmon source must be found in the stack and returned to deck bottom"
        );
    }

    assert_eq!(
        r.pending_kind(),
        Some(SelectionKind::OppField),
        "returning a [Vemmon] source to deck bottom from Snatchmon's own permanent \
         must fire the inherited delete observer"
    );
    r.auto_resolve().expect("delete resolves");
    assert_eq!(
        r.game.players[1].battle_area.len(),
        opp_before - 1,
        "the eligible opponent Digimon must be deleted"
    );
}

/// NEGATIVE (name gate): returning a NON-[Vemmon] source to deck bottom
/// from Snatchmon's own permanent must NOT fire the observer.
#[test]
fn bt21_058_inherited_observer_does_not_fire_for_non_vemmon_source() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("parses")
        .add_card(make_digimon("AGUMON-SRC", 3, 2000, 3))
        .add_card(make_digimon("HOST-TOP2", 5, 5000, 5))
        .add_card(make_digimon("OPP-CHEAP", 3, 3000, 4))
        .add_card(make_filler("F058-DECKPAD"))
        .deck(0, &["F058-DECKPAD"; 5])
        .start();
    r.game.enter_main_phase();

    let host = place_buried_snatchmon(&mut r, &["AGUMON-SRC", "HOST-TOP2"]);
    let agumon_source = {
        let perm = &r.game.players[0].battle_area[host.index as usize];
        perm.card_sources[1].handle()
    };
    r.place_on_field(1, "OPP-CHEAP", None);
    let opp_before = r.game.players[1].battle_area.len();

    {
        let top = r.top_card(host);
        let mut ctx = digimon_engine::effect_context::EffectContext::new(&mut r.game, top, Some(host), 0);
        assert!(ctx.return_card_source_to_deck(host, agumon_source, true));
    }

    assert!(
        r.pending_selection().is_none(),
        "a non-[Vemmon] source return must NOT fire the name-gated observer"
    );
    assert_eq!(r.game.players[1].battle_area.len(), opp_before);
}

/// NEGATIVE (host scope): a [Vemmon] source returned to deck bottom from a
/// DIFFERENT permanent (one that does NOT carry Snatchmon anywhere in its
/// stack) must NOT fire Snatchmon's inherited observer.
#[test]
fn bt21_058_inherited_observer_does_not_fire_for_other_permanent_source() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("parses")
        .add_card(make_exact_vemmon("F058-OTHER-SRC"))
        .add_card(make_digimon("HOST-TOP3", 5, 5000, 5))
        .add_card(make_digimon("OTHER-HOST", 4, 4000, 4))
        .add_card(make_digimon("OPP-CHEAP", 3, 3000, 4))
        .add_card(make_filler("F058-DECKPAD"))
        .deck(0, &["F058-DECKPAD"; 5])
        .start();
    r.game.enter_main_phase();

    // Snatchmon buried under its own unrelated stack (present on the field,
    // but this test's return happens on a DIFFERENT permanent entirely).
    let _snatchmon_host = place_buried_snatchmon(&mut r, &["HOST-TOP3"]);
    // A DIFFERENT permanent (OTHER-HOST, no Snatchmon anywhere in it) carries
    // the Vemmon source.
    let other_host = r.place_stack(0, &["F058-OTHER-SRC", "OTHER-HOST"]);
    let vemmon_source = {
        let perm = &r.game.players[0].battle_area[other_host.index as usize];
        perm.card_sources[0].handle()
    };
    r.place_on_field(1, "OPP-CHEAP", None);
    let opp_before = r.game.players[1].battle_area.len();

    {
        let top = r.top_card(other_host);
        let mut ctx = digimon_engine::effect_context::EffectContext::new(
            &mut r.game,
            top,
            Some(other_host),
            0,
        );
        assert!(ctx.return_card_source_to_deck(other_host, vemmon_source, true));
    }

    assert!(
        r.pending_selection().is_none(),
        "a [Vemmon] source returned from a DIFFERENT permanent's stack (no Snatchmon \
         anywhere in it) must NOT fire Snatchmon's host-scoped inherited observer"
    );
    assert_eq!(r.game.players[1].battle_area.len(), opp_before);
}

/// NEGATIVE (deck-top return): the printed observer is scoped to deck-
/// BOTTOM returns; a deck-TOP return must not fire it.
#[test]
fn bt21_058_inherited_observer_does_not_fire_on_deck_top_return() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("parses")
        .add_card(make_exact_vemmon("F058-TOP-SRC"))
        .add_card(make_digimon("HOST-TOP4", 5, 5000, 5))
        .add_card(make_digimon("OPP-CHEAP", 3, 3000, 4))
        .add_card(make_filler("F058-DECKPAD"))
        .deck(0, &["F058-DECKPAD"; 5])
        .start();
    r.game.enter_main_phase();

    let host = place_buried_snatchmon(&mut r, &["F058-TOP-SRC", "HOST-TOP4"]);
    let vemmon_source = {
        let perm = &r.game.players[0].battle_area[host.index as usize];
        perm.card_sources[1].handle()
    };
    r.place_on_field(1, "OPP-CHEAP", None);
    let opp_before = r.game.players[1].battle_area.len();

    {
        let top = r.top_card(host);
        let mut ctx = digimon_engine::effect_context::EffectContext::new(&mut r.game, top, Some(host), 0);
        assert!(ctx.return_card_source_to_deck(host, vemmon_source, /* to_bottom */ false));
    }

    assert!(
        r.pending_selection().is_none(),
        "a deck-TOP return must NOT fire the bottom-scoped observer"
    );
    assert_eq!(r.game.players[1].battle_area.len(), opp_before);
}

/// NEGATIVE (no eligible target): with no opponent Digimon at play cost <=
/// 4 on the field, the observer's condition gate fizzles — no selection
/// surfaces.
#[test]
fn bt21_058_inherited_observer_fizzles_with_no_eligible_opponent_target() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("parses")
        .add_card(make_exact_vemmon("F058-NOOPP-SRC"))
        .add_card(make_digimon("HOST-TOP5", 5, 5000, 5))
        .add_card(make_digimon("OPP-EXPENSIVE", 5, 8000, 8))
        .add_card(make_filler("F058-DECKPAD"))
        .deck(0, &["F058-DECKPAD"; 5])
        .start();
    r.game.enter_main_phase();

    let host = place_buried_snatchmon(&mut r, &["F058-NOOPP-SRC", "HOST-TOP5"]);
    let vemmon_source = {
        let perm = &r.game.players[0].battle_area[host.index as usize];
        perm.card_sources[1].handle()
    };
    // Opponent only has a play-cost-8 Digimon — not eligible (cost > 4).
    r.place_on_field(1, "OPP-EXPENSIVE", None);

    {
        let top = r.top_card(host);
        let mut ctx = digimon_engine::effect_context::EffectContext::new(&mut r.game, top, Some(host), 0);
        assert!(ctx.return_card_source_to_deck(host, vemmon_source, true));
    }

    assert!(
        r.pending_selection().is_none(),
        "no eligible (cost <= 4) opponent Digimon exists — the observer's condition \
         gate must fizzle without surfacing a selection"
    );
}

/// OPT lockout: a second qualifying return in the SAME turn must not fire
/// again.
#[test]
fn bt21_058_inherited_observer_opt_lockout_same_turn() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("parses")
        .add_card(make_exact_vemmon("F058-OPT-A"))
        .add_card(make_exact_vemmon("F058-OPT-B"))
        .add_card(make_digimon("HOST-TOP6", 5, 5000, 5))
        .add_card(make_digimon("OPP-A", 3, 3000, 4))
        .add_card(make_digimon("OPP-B", 3, 3000, 4))
        .add_card(make_filler("F058-DECKPAD"))
        .deck(0, &["F058-DECKPAD"; 5])
        .start();
    r.game.enter_main_phase();

    let host = place_buried_snatchmon(&mut r, &["F058-OPT-A", "F058-OPT-B", "HOST-TOP6"]);
    let (src_a, src_b) = {
        let perm = &r.game.players[0].battle_area[host.index as usize];
        // index 0 = BT21-058, index 1 = F058-OPT-A, index 2 = F058-OPT-B.
        (perm.card_sources[1].handle(), perm.card_sources[2].handle())
    };
    r.place_on_field(1, "OPP-A", None);
    r.place_on_field(1, "OPP-B", None);

    // First return fires and must be resolvable.
    {
        let top = r.top_card(host);
        let mut ctx = digimon_engine::effect_context::EffectContext::new(&mut r.game, top, Some(host), 0);
        assert!(ctx.return_card_source_to_deck(host, src_a, true));
    }
    assert_eq!(
        r.pending_kind(),
        Some(SelectionKind::OppField),
        "first return fires the OPT observer"
    );
    r.auto_resolve().expect("first delete resolves");
    assert_eq!(r.game.players[1].battle_area.len(), 1);

    // Second return, SAME turn, must NOT fire again.
    {
        let top = r.top_card(host);
        let mut ctx = digimon_engine::effect_context::EffectContext::new(&mut r.game, top, Some(host), 0);
        assert!(ctx.return_card_source_to_deck(host, src_b, true));
    }
    assert!(
        r.pending_selection().is_none(),
        "OPT lockout: second same-turn return must not surface a selection"
    );
    assert_eq!(
        r.game.players[1].battle_area.len(),
        1,
        "no second deletion — OPT consumed its single use"
    );
}

/// OPT clears next turn: after end_turn cycles back to P0, the lockout
/// resets and the observer can fire again.
#[test]
fn bt21_058_inherited_observer_opt_clears_next_turn() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("parses")
        .add_card(make_exact_vemmon("F058-CLR-A"))
        .add_card(make_exact_vemmon("F058-CLR-B"))
        .add_card(make_digimon("HOST-TOP7", 5, 5000, 5))
        .add_card(make_digimon("OPP-A2", 3, 3000, 4))
        .add_card(make_digimon("OPP-B2", 3, 3000, 4))
        .add_card(make_filler("F058-DECKPAD"))
        .deck(0, &["F058-DECKPAD"; 5])
        .deck(1, &["F058-DECKPAD"; 5])
        .start();
    r.game.enter_main_phase();

    let host = place_buried_snatchmon(&mut r, &["F058-CLR-A", "F058-CLR-B", "HOST-TOP7"]);
    let (src_a, src_b) = {
        let perm = &r.game.players[0].battle_area[host.index as usize];
        (perm.card_sources[1].handle(), perm.card_sources[2].handle())
    };
    r.place_on_field(1, "OPP-A2", None);
    r.place_on_field(1, "OPP-B2", None);

    {
        let top = r.top_card(host);
        let mut ctx = digimon_engine::effect_context::EffectContext::new(&mut r.game, top, Some(host), 0);
        assert!(ctx.return_card_source_to_deck(host, src_a, true));
    }
    r.auto_resolve().expect("first delete resolves");
    assert_eq!(r.game.players[1].battle_area.len(), 1);

    // Cycle through P1's turn back to P0's turn.
    r.game.end_turn(); // P0 -> P1
    r.game.end_turn(); // P1 -> P0

    {
        let top = r.top_card(host);
        let mut ctx = digimon_engine::effect_context::EffectContext::new(&mut r.game, top, Some(host), 0);
        assert!(ctx.return_card_source_to_deck(host, src_b, true));
    }
    assert_eq!(
        r.pending_kind(),
        Some(SelectionKind::OppField),
        "OPT lockout must clear after end_turn — the observer fires again next turn"
    );
}

/// [All Turns]: the observer must ALSO fire on the OPPONENT's turn (not
/// restricted to your own turn), matching the printed [All Turns] qualifier.
#[test]
fn bt21_058_inherited_observer_fires_on_opponents_turn() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("parses")
        .add_card(make_exact_vemmon("F058-AT-SRC"))
        .add_card(make_digimon("HOST-TOP8", 5, 5000, 5))
        .add_card(make_digimon("OPP-AT", 3, 3000, 4))
        .add_card(make_filler("F058-DECKPAD"))
        .deck(0, &["F058-DECKPAD"; 5])
        .deck(1, &["F058-DECKPAD"; 5])
        .start();
    r.game.enter_main_phase();

    let host = place_buried_snatchmon(&mut r, &["F058-AT-SRC", "HOST-TOP8"]);
    let vemmon_source = {
        let perm = &r.game.players[0].battle_area[host.index as usize];
        perm.card_sources[1].handle()
    };
    r.place_on_field(1, "OPP-AT", None);

    // Advance to P1's turn.
    r.game.end_turn();

    {
        let top = r.top_card(host);
        let mut ctx = digimon_engine::effect_context::EffectContext::new(&mut r.game, top, Some(host), 0);
        assert!(ctx.return_card_source_to_deck(host, vemmon_source, true));
    }

    assert_eq!(
        r.pending_kind(),
        Some(SelectionKind::OppField),
        "[All Turns] qualifier means the observer must fire on the opponent's turn too"
    );
}

/// Integrated event-log check: resolving the delete via the inherited
/// observer emits a Trash event (deleted permanents route through
/// `trash_permanent_stack`'s emission helper).
#[test]
fn bt21_058_inherited_observer_delete_emits_event() {
    use digimon_engine::events::GameEvent;

    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("parses")
        .add_card(make_exact_vemmon("F058-EVT-SRC"))
        .add_card(make_digimon("HOST-TOP9", 5, 5000, 5))
        .add_card(make_digimon("OPP-EVT", 3, 3000, 4))
        .add_card(make_filler("F058-DECKPAD"))
        .deck(0, &["F058-DECKPAD"; 5])
        .start();
    r.game.enter_main_phase();

    let host = place_buried_snatchmon(&mut r, &["F058-EVT-SRC", "HOST-TOP9"]);
    let vemmon_source = {
        let perm = &r.game.players[0].battle_area[host.index as usize];
        perm.card_sources[1].handle()
    };
    r.place_on_field(1, "OPP-EVT", None);

    let cp = r.event_checkpoint();
    {
        let top = r.top_card(host);
        let mut ctx = digimon_engine::effect_context::EffectContext::new(&mut r.game, top, Some(host), 0);
        assert!(ctx.return_card_source_to_deck(host, vemmon_source, true));
    }
    r.auto_resolve().expect("delete resolves");

    let events = r.events_since(cp);
    let trash_count = events
        .iter()
        .filter(|e| matches!(e, GameEvent::Trash { .. }))
        .count();
    assert!(
        trash_count >= 1,
        "a Trash event must fire for the inherited observer's deletion (deleted \
         permanents route through trash_permanent_stack's emission helper); \
         events: {events:?}"
    );
}
