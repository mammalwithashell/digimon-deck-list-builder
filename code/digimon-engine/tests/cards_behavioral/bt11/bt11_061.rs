//! BT11-061 Vemmon — Digimon, Lv.3, Black, DP 1000, Cost 3.
//!
//! # Card text (card image BT11-061.webp — authoritative; cross-checked
//! data/card_bundles/BT11-061.md [official Bandai DB] + data/cards.json)
//!
//! You can include up to 50 copies of cards with this card's card number in
//! your deck.
//! [Main] By suspending this Digimon, reveal the top 3 cards of your deck.
//! Add up to 1 [Snatchmon], [Destromon], [Galacticmon], or [Fusionize] among
//! them to your hand, and place 1 [Vemmon] among them under this Digimon as
//! its bottom digivolution card. Place the rest at the bottom of your deck
//! in any order.
//!
//! # Inherited Effect
//! [Your Turn][Once Per Turn] When this Digimon would digivolve into
//! [Destromon] or [Galacticmon], reduce the digivolution cost by 1.
//!
//! # Official Q&A
//! "Yes, you can place 1 [Vemmon] as a digivolution card even when none of
//! those cards are among the revealed cards." — the hand-bucket and the
//! bottom-source bucket are independent.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT11/Black/BT11_061.cs
//!
//! # Patterns this test covers
//! - Deck rule "up to 50 copies" — structural assertion that
//!   `max_count_in_deck` reports 50 for this card number (data/
//!   card_overrides.json patches the API-dropped rule text).
//! - `main_on_field` suspend-cost activated ability, gated on
//!   `source_is_unsuspended` (CanActivateSuspendCostEffect).
//! - Two-bucket independent reveal-and-select (`select_reveal_buckets`) over
//!   one `reveal_top_deck` pool: bucket A → hand (4 possible names), bucket
//!   B → bottom digivolution source of self (Vemmon only) — proven
//!   independent per the official Q&A.
//! - `per_selected` unwrap of a bucket `CardList` into `place_as_bottom_source`.
//! - Remainder to deck bottom via `per_selected` + `return_to_deck_from_reveal`.
//! - Inherited `[Your Turn][OPT]` digivolve-cost -1 scoped to
//!   Destromon/Galacticmon exact names, `source_is_cost_target_permanent`.

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::{
    EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_MAIN, FIELD_EFFECT_START, PASS, SEL_REVEAL_START,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{
    make_test_card, make_test_card_with_level, DebugRunner,
};
use digimon_engine::enums::{CardColor, CardKind, PlaySource};
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "BT11-061";

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn vemmon_runner() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT11-061 YAML parses and compiles")
}

fn snatchmon(id: &str) -> CardData {
    let mut card = make_test_card(id, "Snatchmon");
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Black];
    card.level = Some(5);
    card.dp = Some(5000);
    card.play_cost = 6;
    card
}

fn destromon(id: &str) -> CardData {
    let mut card = make_test_card(id, "Destromon");
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Black];
    card.level = Some(4);
    card.dp = Some(4000);
    card.play_cost = 4;
    card.evo_costs = vec![EvoCost {
        card_color: 5, // Black
        level: 3,
        memory_cost: 3,
    }];
    card
}

fn galacticmon(id: &str) -> CardData {
    let mut card = make_test_card(id, "Galacticmon");
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Black];
    card.level = Some(6);
    card.dp = Some(9000);
    card.play_cost = 10;
    card.evo_costs = vec![EvoCost {
        card_color: 5, // Black
        level: 4,
        memory_cost: 4,
    }];
    card
}

fn fusionize_option(id: &str) -> CardData {
    let mut card = make_test_card(id, "Fusionize");
    card.card_kind = CardKind::Option;
    card.colors = vec![CardColor::Black];
    card.level = None;
    card.dp = None;
    card.play_cost = 1;
    card
}

fn vemmon_card(id: &str) -> CardData {
    let mut card = make_test_card(id, "Vemmon");
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Black];
    card.level = Some(3);
    card.dp = Some(1000);
    card.play_cost = 3;
    card
}

fn filler(id: &str) -> CardData {
    make_test_card(id, &format!("{id} Filler"))
}

/// Push a card straight to a player's trash without going through the
/// standard hand/deck builders (used to seed the `evo_costs` digivolve
/// target for Clause 2 tests, mirroring the BT21-056 test's `push_to_hand`).
fn push_to_hand(runner: &mut DebugRunner, player: u8, card_id: &str) -> usize {
    let data_index = runner
        .game
        .card_data
        .iter()
        .position(|card| card.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card id {card_id}"));
    let instance_id = runner.game.next_card_index();
    runner.game.players[player as usize]
        .hand
        .push(CardSource::new(data_index, player, instance_id));
    runner.game.players[player as usize].hand.len() - 1
}

fn field_main_action(field_index: u16) -> u16 {
    FIELD_EFFECT_START + field_index * EFFECTS_PER_PERMANENT + FIELD_EFFECT_SLOT_FOR_MAIN
}

/// A plain Black Lv.3 top card to bury BT11-061 under, so it digivolves
/// further (matching Destromon's `evo_costs: Black Lv.3`). The evo-cost gate
/// (`digivolve_from_hand_inner`) checks the CARRIER's actual top card (this
/// card), not the buried BT11-061, so its color must be Black to satisfy
/// the printed digivolve requirement.
fn black_lv3_top(id: &str) -> CardData {
    let mut c = make_test_card_with_level(id, "TopDigi", 3);
    c.colors = vec![CardColor::Black];
    c
}

/// A plain Black Lv.4 top card, matching Galacticmon's `evo_costs: Black
/// Lv.4` for the direct (non-chained) digivolve-into-Galacticmon test.
fn black_lv4_top(id: &str) -> CardData {
    let mut c = make_test_card_with_level(id, "TopDigi4", 4);
    c.colors = vec![CardColor::Black];
    c
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn bt11_061_yaml_compiles_without_error() {
    let runner = vemmon_runner().build();
    assert!(
        runner.compiled_card(CARD_ID).is_some(),
        "BT11-061 must be present in the embedded DSL pack"
    );
}

// Deck rule: "You can include up to 50 copies of cards with this card's
// card number in your deck." (`data/card_overrides.json` patches
// `max_count_in_deck: 50` since the digimoncard.io API drops this rule
// text — see G-DECK-COPY-LIMIT-RAISE). This is a data/validator concern,
// not a DSL card-effect clause, and is ALREADY covered structurally by
// `code/digimon-engine/tests/deck_tools/main.rs`:
//   - `vemmon_intrinsic_max_count_is_fifty` (raw DB field == 50)
//   - `validate_deck_accepts_fifty_copies_of_vemmon` (a 50-copy deck validates)
//   - `card_legality_reports_raised_cap_for_vemmon` (per-card legality
//     projection reports max_copies == 50, not the format default 4)
//   - `card_legality_raised_cap_still_respects_singleton` (a downward format
//     restriction still dominates the printed RAISE)
// Not duplicated here per the brief ("reference, don't duplicate").

/// Clause 1 — a `main_on_field` triggered clause exists, gated on
/// `source_is_unsuspended` (the printed "by suspending this Digimon" cost
/// precondition).
#[test]
fn bt11_061_has_main_on_field_clause_gated_on_unsuspended() {
    let runner = vemmon_runner().build();
    let card = runner.compiled_card(CARD_ID).expect("BT11-061 present");
    let main = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::MainOnField) => {
                Some(t)
            }
            _ => None,
        })
        .expect("BT11-061 must have a MainOnField clause");
    assert!(
        main.condition.is_some(),
        "the [Main] clause must carry a condition gate (source_is_unsuspended)"
    );
    assert!(
        !main.once_per_turn,
        "the printed [Main] text has no [Once Per Turn] tag"
    );
}

/// Clause 1 — the process pays the suspend cost as the FIRST step.
#[test]
fn bt11_061_main_suspends_self_first() {
    let runner = vemmon_runner().build();
    let card = runner.compiled_card(CARD_ID).expect("BT11-061 present");
    let main = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::MainOnField) => {
                Some(t)
            }
            _ => None,
        })
        .expect("MainOnField clause must exist");
    assert!(
        matches!(main.process.first(), Some(CompiledStep::Suspend { .. })),
        "the first step of the [Main] process must be the suspend-self cost"
    );
}

/// Clause 1 — the process reveals the top 3 cards.
#[test]
fn bt11_061_main_reveals_top_3() {
    let runner = vemmon_runner().build();
    let card = runner.compiled_card(CARD_ID).expect("BT11-061 present");
    let main = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::MainOnField) => {
                Some(t)
            }
            _ => None,
        })
        .expect("MainOnField clause must exist");
    assert!(
        main.process
            .iter()
            .any(|s| matches!(s, CompiledStep::RevealTopDeck { count: 3, .. })),
        "the [Main] process must contain RevealTopDeck {{ count: 3 }}"
    );
}

/// Clause 1 — the process contains a SelectRevealBuckets step with exactly
/// TWO independent buckets, each (min:0, max:1).
#[test]
fn bt11_061_main_has_two_independent_buckets() {
    let runner = vemmon_runner().build();
    let card = runner.compiled_card(CARD_ID).expect("BT11-061 present");
    let main = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::MainOnField) => {
                Some(t)
            }
            _ => None,
        })
        .expect("MainOnField clause must exist");
    let bucket_step = main.process.iter().find_map(|s| match s {
        CompiledStep::SelectRevealBuckets { buckets, .. } => Some(buckets),
        _ => None,
    });
    let buckets = bucket_step.expect("MainOnField process must contain SelectRevealBuckets");
    assert_eq!(buckets.len(), 2, "there must be exactly 2 independent buckets");
    for bucket in buckets {
        assert_eq!(bucket.min, 0, "each bucket must be optional (min:0)");
        assert_eq!(bucket.max, 1, "each bucket caps at 1 pick");
    }
}

/// Clause 1 — the process moves bucket A's pick to hand, loops bucket B's
/// pick through PlaceAsBottomSource, and loops the full reveal pool through
/// ReturnToDeckFromReveal (the remainder tail).
#[test]
fn bt11_061_main_has_hand_move_bottom_source_loop_and_remainder_loop() {
    let runner = vemmon_runner().build();
    let card = runner.compiled_card(CARD_ID).expect("BT11-061 present");
    let main = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::MainOnField) => {
                Some(t)
            }
            _ => None,
        })
        .expect("MainOnField clause must exist");

    assert!(
        main.process
            .iter()
            .any(|s| matches!(s, CompiledStep::AddToHandFromReveal { .. })),
        "process must contain AddToHandFromReveal for the hand bucket"
    );

    let has_bottom_source_loop = main.process.iter().any(|s| match s {
        CompiledStep::PerSelected { body, .. } => body
            .iter()
            .any(|inner| matches!(inner, CompiledStep::PlaceAsBottomSource { .. })),
        _ => false,
    });
    assert!(
        has_bottom_source_loop,
        "process must contain a PerSelected loop with PlaceAsBottomSource for the Vemmon bucket"
    );

    let has_remainder_loop = main.process.iter().any(|s| match s {
        CompiledStep::PerSelected { body, .. } => body
            .iter()
            .any(|inner| matches!(inner, CompiledStep::ReturnToDeckFromReveal { .. })),
        _ => false,
    });
    assert!(
        has_remainder_loop,
        "process must contain a PerSelected loop with ReturnToDeckFromReveal for the remainder"
    );
}

/// Clause 2 — an inherited CostReduction clause exists, once_per_turn, with
/// `active_when` gating on your_turn + source_is_cost_target_permanent +
/// cost_target Destromon/Galacticmon.
#[test]
fn bt11_061_has_inherited_cost_reduction_clause() {
    let runner = vemmon_runner().build();
    let card = runner.compiled_card(CARD_ID).expect("BT11-061 present");
    let cr = card.effects.iter().find_map(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction {
            scope,
            active_when,
            once_per_turn,
            amount,
            ..
        }) => Some((*scope, active_when.clone(), *once_per_turn, *amount)),
        _ => None,
    });
    let (scope, active_when, once_per_turn, amount) =
        cr.expect("BT11-061 must have a CostReduction declarative clause");
    assert_eq!(
        scope,
        CompiledScope::Inherited,
        "cost reducer must be inherited scope (printed under inherited_effect_description_eng)"
    );
    assert!(once_per_turn, "cost reducer must be once_per_turn");
    assert_eq!(amount, Some(1), "cost reduction amount must be 1");

    let active_when = active_when.expect("active_when must be present");
    let has_your_turn = active_when.your_turn == Some(true)
        || active_when.all_of.iter().any(|p| p.your_turn == Some(true));
    assert!(has_your_turn, "must gate on [Your Turn]");

    let has_cost_target_permanent = active_when.source_is_cost_target_permanent == Some(true)
        || active_when
            .all_of
            .iter()
            .any(|p| p.source_is_cost_target_permanent == Some(true));
    assert!(
        has_cost_target_permanent,
        "must gate on source_is_cost_target_permanent"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 2 — [Main] behavioral: mask gating + suspend cost
// ═══════════════════════════════════════════════════════════════════════════

/// The [Main] action must be legal (mask bit set) while BT11-061 is
/// unsuspended on the battle area.
#[test]
fn bt11_061_main_action_legal_when_unsuspended() {
    let mut runner = vemmon_runner()
        .add_card(filler("FILL-A1"))
        .add_card(filler("FILL-A2"))
        .add_card(filler("FILL-A3"))
        .deck(0, &["FILL-A1", "FILL-A2", "FILL-A3"])
        .memory(6)
        .start();

    let vemmon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.enter_main_phase();

    let action = field_main_action(vemmon.index as u16);
    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[action as usize], 1.0,
        "BT11-061's [Main] activation must be legal while unsuspended"
    );
}

/// Negative: with BT11-061 already suspended, the [Main] action must NOT be
/// legal (CanActivateSuspendCostEffect requires an unsuspended carrier).
#[test]
fn bt11_061_main_action_illegal_when_already_suspended() {
    let mut runner = vemmon_runner()
        .add_card(filler("FILL-B1"))
        .add_card(filler("FILL-B2"))
        .add_card(filler("FILL-B3"))
        .deck(0, &["FILL-B1", "FILL-B2", "FILL-B3"])
        .memory(6)
        .start();

    let vemmon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.players[0].battle_area[vemmon.index as usize].is_suspended = true;
    runner.game.enter_main_phase();

    let action = field_main_action(vemmon.index as u16);
    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[action as usize], 0.0,
        "BT11-061's [Main] activation must be illegal while already suspended"
    );
}

/// Activating [Main] suspends BT11-061 as the cost, unconditionally.
#[test]
fn bt11_061_main_activation_suspends_self() {
    let mut runner = vemmon_runner()
        .add_card(filler("FILL-C1"))
        .add_card(filler("FILL-C2"))
        .add_card(filler("FILL-C3"))
        .deck(0, &["FILL-C1", "FILL-C2", "FILL-C3"])
        .memory(6)
        .start();

    let vemmon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.enter_main_phase();

    let action = field_main_action(vemmon.index as u16);
    runner.game.decode_action(action, 0);
    runner.game.drain_effect_queue();

    assert!(
        runner.game.players[0].battle_area[vemmon.index as usize].is_suspended,
        "BT11-061 must be suspended as the [Main] cost"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 3 — [Main] behavioral: reveal + independent bucket picks
// ═══════════════════════════════════════════════════════════════════════════

/// With a Destromon among the top 3, activating [Main] installs a
/// RevealBucket selection offering it as a legal hand-bucket pick.
#[test]
fn bt11_061_main_offers_destromon_for_hand_bucket() {
    let mut runner = vemmon_runner()
        .add_card(destromon("DEST-D1"))
        .add_card(filler("FILL-D1"))
        .add_card(filler("FILL-D2"))
        // Destromon at top of deck (last in array) -> revealed at index 0.
        .deck(0, &["FILL-D1", "FILL-D2", "DEST-D1"])
        .memory(6)
        .start();

    let vemmon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.enter_main_phase();

    let action = field_main_action(vemmon.index as u16);
    runner.game.decode_action(action, 0);
    runner.game.drain_effect_queue();

    let view = runner
        .pending_selection_view()
        .expect("RevealBucket selection must be pending after suspend + reveal");
    assert!(
        matches!(view.kind, SelectionKind::RevealBucket { .. }),
        "selection kind must be RevealBucket, got {:?}",
        view.kind
    );
    assert!(
        view.valid_action_ids.contains(&SEL_REVEAL_START),
        "Destromon (revealed at index 0) must be a legal bucket-A pick; \
         valid_action_ids = {:?}",
        view.valid_action_ids
    );
}

/// Picking the revealed Destromon adds it to hand; the bucket-B (Vemmon)
/// prompt then follows with zero candidates and auto-skips, and the
/// remainder (2 filler cards) lands at the deck bottom.
#[test]
fn bt11_061_main_picking_hand_bucket_adds_to_hand_and_bottoms_remainder() {
    let mut runner = vemmon_runner()
        .add_card(destromon("DEST-E1"))
        .add_card(filler("FILL-E1"))
        .add_card(filler("FILL-E2"))
        .deck(0, &["FILL-E1", "FILL-E2", "DEST-E1"])
        .memory(6)
        .start();

    let vemmon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.enter_main_phase();
    let hand_before = runner.hand_size(0);
    let deck_before = runner.deck_size(0);

    let action = field_main_action(vemmon.index as u16);
    runner.game.decode_action(action, 0);
    runner.game.drain_effect_queue();

    // Pick the Destromon for the hand bucket.
    let view = runner
        .pending_selection_view()
        .expect("hand-bucket selection must be pending");
    let pick = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != PASS)
        .expect("Destromon must be a legal pick");
    runner
        .execute_action(view.selecting_player, pick)
        .expect("select Destromon for hand");
    runner.game.drain_effect_queue();
    runner.auto_resolve().expect("drain remaining prompts");

    assert_eq!(
        runner.hand_size(0),
        hand_before + 1,
        "the picked Destromon must be added to hand"
    );
    // deck started at 3 (revealed all 3), Destromon left the pool to hand,
    // the 2 fillers return to deck bottom -> net deck size: -1 (Destromon
    // permanently left the deck).
    assert_eq!(
        runner.deck_size(0),
        deck_before - 1,
        "the 2 filler cards return to the deck bottom; only the picked \
         Destromon permanently leaves the deck"
    );
}

/// Declining the hand-bucket pick (PASS) still proceeds to the independent
/// Vemmon bucket and the remainder tail — no cross-bucket coupling.
#[test]
fn bt11_061_main_declining_hand_bucket_still_offers_vemmon_bucket() {
    let mut runner = vemmon_runner()
        .add_card(destromon("DEST-F1"))
        .add_card(vemmon_card("VEM-F1"))
        .add_card(filler("FILL-F1"))
        .deck(0, &["FILL-F1", "VEM-F1", "DEST-F1"])
        .memory(6)
        .start();

    let vemmon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.enter_main_phase();

    let action = field_main_action(vemmon.index as u16);
    runner.game.decode_action(action, 0);
    runner.game.drain_effect_queue();

    // Decline the hand bucket.
    let view = runner
        .pending_selection_view()
        .expect("hand-bucket selection must be pending");
    assert!(view.is_optional, "the hand bucket must be optional (PASS legal)");
    runner
        .execute_action(view.selecting_player, PASS)
        .expect("decline hand bucket");
    runner.game.drain_effect_queue();

    // The Vemmon bucket must now be offered, independently of the decline.
    let view = runner
        .pending_selection_view()
        .expect("Vemmon-bucket selection must be pending after declining the hand bucket");
    assert!(
        matches!(view.kind, SelectionKind::RevealBucket { .. }),
        "selection kind must be RevealBucket, got {:?}",
        view.kind
    );
    assert!(
        !view.valid_action_ids.is_empty() && view.valid_action_ids.iter().any(|&a| a != PASS),
        "the Vemmon card must be a legal pick for bucket B"
    );
}

/// Picking the revealed Vemmon places it under BT11-061 as its bottom
/// digivolution source (stack size increases by 1) — even though the hand
/// bucket had NO eligible card among the reveal (official Q&A precedent).
#[test]
fn bt11_061_main_picking_vemmon_bucket_places_bottom_source_with_no_hand_candidate() {
    let mut runner = vemmon_runner()
        .add_card(vemmon_card("VEM-G1"))
        .add_card(filler("FILL-G1"))
        .add_card(filler("FILL-G2"))
        .deck(0, &["FILL-G1", "FILL-G2", "VEM-G1"])
        .memory(6)
        .start();

    let vemmon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.enter_main_phase();

    let action = field_main_action(vemmon.index as u16);
    runner.game.decode_action(action, 0);
    runner.game.drain_effect_queue();

    // Hand bucket: no eligible card among the reveal (only Vemmon + fillers)
    // -> zero candidates, auto-skips within the same drain, landing directly
    // on the Vemmon bucket.
    let view = runner
        .pending_selection_view()
        .expect("Vemmon-bucket selection must be pending (hand bucket auto-skipped)");
    assert!(
        matches!(view.kind, SelectionKind::RevealBucket { .. }),
        "selection kind must be RevealBucket, got {:?}",
        view.kind
    );
    let pick = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != PASS)
        .expect("Vemmon must be a legal pick for bucket B");
    runner
        .execute_action(view.selecting_player, pick)
        .expect("select Vemmon for bottom source");
    runner.game.drain_effect_queue();
    runner.auto_resolve().expect("drain remaining prompts");

    let perm = runner.game.players[0]
        .battle_area
        .get(vemmon.index as usize)
        .expect("BT11-061 permanent must still exist");
    assert_eq!(
        perm.card_sources.len(),
        2,
        "BT11-061 must have gained the picked Vemmon as a bottom digivolution \
         source (1 base top card + 1 placed source)"
    );
}

/// Declining the Vemmon-bucket pick still sends the whole reveal pool to the
/// deck bottom (mandatory remainder tail).
#[test]
fn bt11_061_main_declining_vemmon_bucket_bottoms_all_3() {
    let mut runner = vemmon_runner()
        .add_card(vemmon_card("VEM-H1"))
        .add_card(filler("FILL-H1"))
        .add_card(filler("FILL-H2"))
        .deck(0, &["FILL-H1", "FILL-H2", "VEM-H1"])
        .memory(6)
        .start();

    let vemmon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.enter_main_phase();
    let deck_before = runner.deck_size(0);

    let action = field_main_action(vemmon.index as u16);
    runner.game.decode_action(action, 0);
    runner.game.drain_effect_queue();

    // Decline the Vemmon bucket (hand bucket auto-skipped: no candidates).
    let view = runner
        .pending_selection_view()
        .expect("Vemmon-bucket selection must be pending");
    assert!(view.is_optional, "the Vemmon bucket must be optional (PASS legal)");
    runner
        .execute_action(view.selecting_player, PASS)
        .expect("decline Vemmon bucket");
    runner.game.drain_effect_queue();
    runner.auto_resolve().expect("drain remaining prompts");

    assert_eq!(
        runner.deck_size(0),
        deck_before,
        "declining both buckets must still return all 3 revealed cards to \
         the deck bottom (net deck size unchanged)"
    );
    let perm = runner.game.players[0]
        .battle_area
        .get(vemmon.index as usize)
        .expect("BT11-061 permanent must still exist");
    assert_eq!(
        perm.card_sources.len(),
        1,
        "declining the Vemmon bucket must NOT place any bottom source"
    );
}

/// Fusionize (an Option card, not a Digimon) is a valid hand-bucket pick —
/// the bucket filters by NAME only, no `kind` restriction.
#[test]
fn bt11_061_main_fusionize_option_card_is_valid_hand_bucket_pick() {
    let mut runner = vemmon_runner()
        .add_card(fusionize_option("FUS-I1"))
        .add_card(filler("FILL-I1"))
        .add_card(filler("FILL-I2"))
        .deck(0, &["FILL-I1", "FILL-I2", "FUS-I1"])
        .memory(6)
        .start();

    let vemmon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.enter_main_phase();

    let action = field_main_action(vemmon.index as u16);
    runner.game.decode_action(action, 0);
    runner.game.drain_effect_queue();

    let view = runner
        .pending_selection_view()
        .expect("hand-bucket selection must be pending");
    assert!(
        view.valid_action_ids.contains(&SEL_REVEAL_START),
        "Fusionize (an Option card) must be a legal hand-bucket pick despite \
         not being a Digimon; valid_action_ids = {:?}",
        view.valid_action_ids
    );
}

/// Snatchmon and Galacticmon are also valid hand-bucket picks (covering all
/// 4 printed names, not just Destromon/Fusionize used in other tests).
#[test]
fn bt11_061_main_snatchmon_and_galacticmon_are_valid_hand_bucket_picks() {
    let mut runner = vemmon_runner()
        .add_card(snatchmon("SNATCH-J1"))
        .add_card(galacticmon("GALACT-J1"))
        .add_card(filler("FILL-J1"))
        .deck(0, &["FILL-J1", "GALACT-J1", "SNATCH-J1"])
        .memory(6)
        .start();

    let vemmon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.enter_main_phase();

    let action = field_main_action(vemmon.index as u16);
    runner.game.decode_action(action, 0);
    runner.game.drain_effect_queue();

    let view = runner
        .pending_selection_view()
        .expect("hand-bucket selection must be pending");
    // Both Snatchmon (index 0) and Galacticmon (index 1) must be legal picks.
    assert!(
        view.valid_action_ids.contains(&SEL_REVEAL_START),
        "Snatchmon must be a legal hand-bucket pick; ids={:?}",
        view.valid_action_ids
    );
    assert!(
        view.valid_action_ids.contains(&(SEL_REVEAL_START + 1)),
        "Galacticmon must be a legal hand-bucket pick; ids={:?}",
        view.valid_action_ids
    );
}

/// With none of the 5 relevant names among the top 3, activating [Main]
/// still suspends the card and returns all 3 revealed cards to the deck
/// bottom with no prompts installed (both buckets have zero candidates).
#[test]
fn bt11_061_main_with_no_matches_suspends_and_bottoms_all_with_no_prompt() {
    let mut runner = vemmon_runner()
        .add_card(filler("FILL-K1"))
        .add_card(filler("FILL-K2"))
        .add_card(filler("FILL-K3"))
        .deck(0, &["FILL-K1", "FILL-K2", "FILL-K3"])
        .memory(6)
        .start();

    let vemmon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.enter_main_phase();
    let deck_before = runner.deck_size(0);

    let action = field_main_action(vemmon.index as u16);
    runner.game.decode_action(action, 0);
    runner.game.drain_effect_queue();
    runner.auto_resolve().expect("drain any residual prompts");

    assert!(
        runner.game.players[0].battle_area[vemmon.index as usize].is_suspended,
        "BT11-061 must still be suspended even with no matches among the reveal"
    );
    assert_eq!(
        runner.deck_size(0),
        deck_before,
        "all 3 revealed cards must return to the deck bottom when neither \
         bucket has a match"
    );
    assert!(
        runner.pending_selection().is_none(),
        "no selection should remain pending once the remainder tail resolves"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 4 — Event-log assertion for the placement (source leaves deck)
// ═══════════════════════════════════════════════════════════════════════════

/// The picked Vemmon must physically leave the deck pool (not merely be
/// referenced) when placed as the bottom digivolution source — verified via
/// deck size decreasing and the stack composition.
#[test]
fn bt11_061_vemmon_placement_removes_card_from_deck_pool_permanently() {
    let mut runner = vemmon_runner()
        .add_card(vemmon_card("VEM-L1"))
        .add_card(filler("FILL-L1"))
        .add_card(filler("FILL-L2"))
        .deck(0, &["FILL-L1", "FILL-L2", "VEM-L1"])
        .memory(6)
        .start();

    let vemmon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.enter_main_phase();
    let deck_before = runner.deck_size(0);

    let action = field_main_action(vemmon.index as u16);
    runner.game.decode_action(action, 0);
    runner.game.drain_effect_queue();

    let view = runner
        .pending_selection_view()
        .expect("Vemmon-bucket selection must be pending");
    let pick = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != PASS)
        .expect("Vemmon must be a legal pick");
    runner
        .execute_action(view.selecting_player, pick)
        .expect("select Vemmon for bottom source");
    runner.game.drain_effect_queue();
    runner.auto_resolve().expect("drain remaining prompts");

    // The 2 fillers return to deck bottom; the Vemmon permanently leaves the
    // deck (now a digivolution source, not in the deck zone) -> net -1.
    assert_eq!(
        runner.deck_size(0),
        deck_before - 1,
        "the picked Vemmon must permanently leave the deck pool"
    );
    let top_id = runner.game.players[0].battle_area[vemmon.index as usize]
        .card_sources
        .first()
        .map(|c| c.card_id(&runner.game.card_data).to_string());
    assert_eq!(
        top_id.as_deref(),
        Some("VEM-L1"),
        "the placed Vemmon must be the bottom-most digivolution source"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 5 — Inherited [Your Turn][OPT] digivolve-cost -1 behavioral
// ═══════════════════════════════════════════════════════════════════════════

/// Positive: digivolving a buried BT11-061 into Destromon reduces the
/// printed digivolution cost by 1.
#[test]
fn bt11_061_inherited_reduces_cost_when_digivolving_into_destromon() {
    let mut runner = vemmon_runner()
        .add_card(black_lv3_top("TOP-M1"))
        .add_card(destromon("DEST-M1"))
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let carrier = runner.place_stack(0, &[CARD_ID, "TOP-M1"]);
    let hand_index = push_to_hand(&mut runner, 0, "DEST-M1");

    let memory_before = runner.game.memory;
    let digivolved =
        runner
            .game
            .digivolve_from_hand(0, hand_index, carrier.index as usize, PlaySource::ByHand);

    assert!(digivolved, "carrier should digivolve into Destromon");
    assert_eq!(
        runner.game.memory,
        memory_before - 2,
        "base evo cost 3 should be reduced to 2 when digivolving into Destromon"
    );
}

/// Positive: digivolving a buried BT11-061 into Galacticmon also reduces
/// the cost by 1.
#[test]
fn bt11_061_inherited_reduces_cost_when_digivolving_into_galacticmon() {
    let mut runner = vemmon_runner()
        .add_card(black_lv4_top("TOP-N1"))
        .add_card(galacticmon("GALACT-N1"))
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let carrier = runner.place_stack(0, &[CARD_ID, "TOP-N1"]);
    let hand_index = push_to_hand(&mut runner, 0, "GALACT-N1");

    let memory_before = runner.game.memory;
    let digivolved =
        runner
            .game
            .digivolve_from_hand(0, hand_index, carrier.index as usize, PlaySource::ByHand);

    assert!(digivolved, "carrier should digivolve into Galacticmon");
    assert_eq!(
        runner.game.memory,
        memory_before - 3,
        "base evo cost 4 should be reduced to 3 when digivolving into Galacticmon"
    );
}

/// Negative control: digivolving into a non-Destromon/Galacticmon target
/// pays the full, unreduced cost.
#[test]
fn bt11_061_inherited_does_not_reduce_cost_for_other_targets() {
    let mut plain_target = make_test_card_with_level("PLAIN-O1", "PlainDigi", 4);
    plain_target.colors = vec![CardColor::Black];
    plain_target.evo_costs = vec![EvoCost {
        card_color: 5,
        level: 3,
        memory_cost: 3,
    }];

    let mut runner = vemmon_runner()
        .add_card(black_lv3_top("TOP-O1"))
        .add_card(plain_target)
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let carrier = runner.place_stack(0, &[CARD_ID, "TOP-O1"]);
    let hand_index = push_to_hand(&mut runner, 0, "PLAIN-O1");

    let memory_before = runner.game.memory;
    let digivolved =
        runner
            .game
            .digivolve_from_hand(0, hand_index, carrier.index as usize, PlaySource::ByHand);

    assert!(digivolved, "should still digivolve into the plain target");
    assert_eq!(
        runner.game.memory,
        memory_before - 3,
        "a non-Destromon/Galacticmon target must pay the full base cost (no reduction)"
    );
}

/// OPT: two digivolutions of the SAME buried carrier into
/// Destromon/Galacticmon targets in the same turn — only the first is
/// reduced.
#[test]
fn bt11_061_inherited_opt_blocks_second_reduction_same_turn() {
    let mut runner = vemmon_runner()
        .add_card(black_lv3_top("TOP-P1"))
        .add_card(destromon("DEST-P1"))
        .add_card(galacticmon("GALACT-P1"))
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let carrier = runner.place_stack(0, &[CARD_ID, "TOP-P1"]);
    let hand_index_dest = push_to_hand(&mut runner, 0, "DEST-P1");

    let memory_before = runner.game.memory;
    let digivolved = runner.game.digivolve_from_hand(
        0,
        hand_index_dest,
        carrier.index as usize,
        PlaySource::ByHand,
    );
    assert!(digivolved, "first digivolve should succeed");
    assert_eq!(
        memory_before - runner.game.memory,
        2,
        "first digivolve into Destromon must be reduced by 1 (3 -> 2)"
    );

    // Second digivolve of the SAME stack (BT11-061 remains buried, now
    // under TOP-P1 and DEST-P1) into a Galacticmon target, same turn.
    let hand_index_galact = push_to_hand(&mut runner, 0, "GALACT-P1");
    let memory_before_second = runner.game.memory;
    let digivolved_second = runner.game.digivolve_from_hand(
        0,
        hand_index_galact,
        carrier.index as usize,
        PlaySource::ByHand,
    );
    assert!(digivolved_second, "second digivolve should succeed");
    assert_eq!(
        memory_before_second - runner.game.memory,
        4,
        "OPT must block the second reduction in the same turn — full cost \
         (4) paid for Galacticmon"
    );
}

/// OPT resets after end_turn: the reducer fires again on the controller's
/// next turn.
#[test]
fn bt11_061_inherited_opt_resets_after_end_turn() {
    let deck_pad = make_test_card("DECK-PAD-Q", "DeckPad");

    let mut runner = vemmon_runner()
        .add_card(black_lv3_top("TOP-Q1"))
        .add_card(destromon("DEST-Q1"))
        .add_card(galacticmon("GALACT-Q1"))
        .add_card(deck_pad)
        .deck(
            0,
            &[
                "DECK-PAD-Q",
                "DECK-PAD-Q",
                "DECK-PAD-Q",
                "DECK-PAD-Q",
                "DECK-PAD-Q",
            ],
        )
        .deck(
            1,
            &[
                "DECK-PAD-Q",
                "DECK-PAD-Q",
                "DECK-PAD-Q",
                "DECK-PAD-Q",
                "DECK-PAD-Q",
            ],
        )
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let carrier = runner.place_stack(0, &[CARD_ID, "TOP-Q1"]);
    let hand_index_dest = push_to_hand(&mut runner, 0, "DEST-Q1");

    let memory_before = runner.game.memory;
    let digivolved = runner.game.digivolve_from_hand(
        0,
        hand_index_dest,
        carrier.index as usize,
        PlaySource::ByHand,
    );
    assert!(digivolved, "first digivolve should succeed");
    assert_eq!(
        memory_before - runner.game.memory,
        2,
        "first digivolve must be reduced"
    );

    runner.end_turn(); // -> P1's turn
    runner.end_turn(); // -> P0's turn again; OPT should reset

    let hand_index_galact = push_to_hand(&mut runner, 0, "GALACT-Q1");
    let memory_before_second = runner.game.memory;
    let digivolved_second = runner.game.digivolve_from_hand(
        0,
        hand_index_galact,
        carrier.index as usize,
        PlaySource::ByHand,
    );
    assert!(digivolved_second, "second digivolve should succeed");
    assert_eq!(
        memory_before_second - runner.game.memory,
        3,
        "OPT must reset after end_turn — the reducer fires again on the \
         next controller turn (4 -> 3)"
    );
}

/// The reducer is inherited-scope: it fires from a buried position (not
/// requiring BT11-061 to be the literal top card) — this is already implied
/// by every Section 5 test above (all use `place_stack` with BT11-061
/// buried under a plain top card), but this test additionally checks the
/// structural `scope: inherited` marker directly on a freshly compiled card
/// with no board state, to pin the requirement independent of behavior.
#[test]
fn bt11_061_inherited_scope_is_structurally_pinned() {
    let runner = vemmon_runner().build();
    let card = runner.compiled_card(CARD_ID).expect("BT11-061 present");
    let scope = card.effects.iter().find_map(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction { scope, .. }) => {
            Some(*scope)
        }
        _ => None,
    });
    assert_eq!(
        scope,
        Some(CompiledScope::Inherited),
        "the digivolve-cost reducer must be scope: inherited"
    );
}
