//! EX10-025 KoDokugumon — Digimon, Lv.3, Black, DP 1000, Cost 3.
//! Traits: [Reptile, LIBERATOR]
//!
//! # Card text (cards.json — verbatim)
//!
//! **[On Play]** You may place 2 cards with the [Mineral] or [Rock] trait from
//! your trash as 1 of your [Mineral] or [Rock] trait Digimon's bottom
//! digivolution cards.
//!
//! **Inherited:** When effects trash this card from a [Mineral] or [Rock] trait
//! Digimon's digivolution cards, delete 1 of your opponent's Digimon with a
//! play cost of 4 or less.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX10/Black/EX10_025.cs
//!
//! # Patterns this test covers
//! - §1  Structural: 1 triggered OnPlay (optional, FaceUp) + 1 triggered
//!        Inherited OnDigivolutionCardTrashed
//! - §2  Condition gating (OnPlay): positive (Mineral/Rock Digimon on field,
//!        2 eligible trash cards) and negative variants (no Mineral/Rock Digimon;
//!        fewer than 2 eligible trash cards)
//! - §3  Behavioral (OnPlay): two sequential trash picks + placement under target
//!        Digimon; sources grow by 2, trash shrinks by 2
//! - §4  Optional decline (PASS at the first OwnField prompt leaves everything
//!        unchanged)
//! - §5  Non-Mineral/non-Rock trash cards must not appear as valid targets
//! - §6  Inherited: source-trash trigger deletes an opponent Digimon with play
//!        cost ≤ 4; cost-5 Digimon is excluded; negative (no valid opponent
//!        targets → no prompt)

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::selection::SelectionKind;

// ─── Card factories ───────────────────────────────────────────────────────────

fn make_mineral(id: &str) -> CardData {
    let mut c = make_test_card(id, &format!("{id}-Mineral"));
    c.traits = vec!["Mineral".to_string()];
    c
}

fn make_rock(id: &str) -> CardData {
    let mut c = make_test_card(id, &format!("{id}-Rock"));
    c.traits = vec!["Rock".to_string()];
    c
}

fn make_plain(id: &str) -> CardData {
    let mut c = make_test_card(id, &format!("{id}-Plain"));
    c.traits = vec![];
    c
}

/// A Digimon that can host on field, with Mineral trait.
fn make_mineral_host(id: &str) -> CardData {
    let mut c = make_test_card(id, &format!("{id}-Host"));
    c.traits = vec!["Mineral".to_string()];
    c
}

/// A Digimon that can host on field, with Rock trait.
fn make_rock_host(id: &str) -> CardData {
    let mut c = make_test_card(id, &format!("{id}-Host"));
    c.traits = vec!["Rock".to_string()];
    c
}

/// Push a card (already registered in card_data) into player 0's trash.
fn push_to_trash(runner: &mut DebugRunner, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("{card_id} not found in card_data"));
    let next = runner.game.next_card_index();
    runner
        .game
        .players[0]
        .trash
        .push(CardSource::new(data_idx, 0, next));
}

// ─── Base runner ──────────────────────────────────────────────────────────────

/// Minimal runner that registers EX10-025. No hand/deck pre-loaded.
fn base_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX10-025")
        .expect("EX10-025 YAML parses and compiles")
        .build()
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

/// EX10-025 must compile to exactly 2 clauses:
///   [0] Triggered OnPlay (FaceUp scope, optional)
///   [1] Triggered Inherited OnDigivolutionCardTrashed
#[test]
fn ex10_025_compiles_to_two_clauses() {
    let runner = base_runner();
    let card = runner
        .compiled_card("EX10-025")
        .expect("EX10-025 compiled card present");
    assert_eq!(
        card.effects.len(),
        2,
        "EX10-025 must have exactly 2 compiled clauses; got {}",
        card.effects.len()
    );
}

/// Clause 0 must be a triggered clause with OnPlay timing, FaceUp scope,
/// optional: true, once_per_turn: false.
#[test]
fn ex10_025_on_play_clause_is_optional_face_up() {
    let runner = base_runner();
    let card = runner
        .compiled_card("EX10-025")
        .expect("EX10-025 compiled card present");

    let on_play = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnPlay) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("EX10-025 must have a Triggered OnPlay clause");

    assert_eq!(
        on_play.scope,
        CompiledScope::FaceUp,
        "OnPlay clause must be FaceUp scope"
    );
    assert!(
        on_play.optional,
        "OnPlay clause must be optional (\"You may\")"
    );
    assert!(
        !on_play.once_per_turn,
        "OnPlay clause must NOT be once_per_turn"
    );
}

/// Clause 1 must be a triggered Inherited clause with OnDigivolutionCardTrashed timing.
#[test]
fn ex10_025_has_inherited_source_trash_delete_clause() {
    let runner = base_runner();
    let card = runner
        .compiled_card("EX10-025")
        .expect("EX10-025 compiled card present");

    let inherited = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(triggered)
                if triggered.scope == CompiledScope::Inherited
                    && triggered
                        .when
                        .contains(&CompiledTiming::OnDigivolutionCardTrashed) =>
            {
                Some(triggered)
            }
            _ => None,
        });

    assert!(
        inherited.is_some(),
        "EX10-025 must compile an inherited OnDigivolutionCardTrashed clause"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — OnPlay condition gating
// ═══════════════════════════════════════════════════════════════════════════════

/// Negative: no Mineral or Rock Digimon on the field → no selection installs.
/// The condition `any_permanent … (trait_has: Mineral | trait_has: Rock)` fails.
#[test]
fn ex10_025_on_play_no_prompt_when_no_mineral_rock_digimon_on_field() {
    let min1 = make_mineral("MIN-COND-1");
    let min2 = make_mineral("MIN-COND-2");
    let filler = make_plain("FILL-DECK");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-025")
        .expect("EX10-025 parses")
        .add_card(min1)
        .add_card(min2)
        .add_card(filler)
        .hand(0, &["EX10-025"])
        .deck(0, &["FILL-DECK"])
        .memory(10)
        .start();

    // Add 2 Mineral trash cards but put NO Mineral/Rock host on the field.
    push_to_trash(&mut runner, "MIN-COND-1");
    push_to_trash(&mut runner, "MIN-COND-2");

    runner.play(0, 0);
    let _ = runner.auto_resolve();

    assert!(
        runner.pending_selection().is_none(),
        "no selection must install: condition fails because there is no Mineral/Rock Digimon on field"
    );
}

/// Negative: a Mineral/Rock Digimon is on field but fewer than 2 eligible
/// trash cards exist → the on_play condition passes but the process stalls
/// at the first or second trash pick because there are 0 or 1 eligible targets.
/// In the DSL this means the select_trash prompt either skips (0 targets) or
/// is exhausted after 1 pick. Either way, no second trash selection fires.
#[test]
fn ex10_025_on_play_no_second_trash_pick_when_only_one_eligible_trash_card() {
    let host = make_mineral_host("HOST-1TRASH");
    let min1 = make_mineral("MIN-ONE");
    let filler = make_plain("FILL-DECK");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-025")
        .expect("EX10-025 parses")
        .add_card(host)
        .add_card(min1)
        .add_card(filler)
        .hand(0, &["EX10-025"])
        .deck(0, &["FILL-DECK"])
        .memory(10)
        .start();

    runner.place_on_field(0, "HOST-1TRASH", None);
    // Only 1 eligible trash card instead of the required 2.
    push_to_trash(&mut runner, "MIN-ONE");

    let sources_before = runner.game.players[0].battle_area[0].card_sources.len();
    let trash_before = runner.trash_size(0);

    runner.play(0, 0);
    // Drive: OwnField pick (dest), then Trash pick 1, then attempt trash pick 2.
    // After the first trash card is placed the second select_trash should find
    // 0 eligible targets and auto-skip or complete without selecting.
    let _ = runner.auto_resolve();

    let sources_after = runner.game.players[0].battle_area[0].card_sources.len();
    let trash_after = runner.trash_size(0);

    // At most 1 source card can have been placed (only 1 was available).
    assert!(
        sources_after <= sources_before + 1,
        "at most 1 source may be placed when only 1 eligible trash card exists"
    );
    assert!(
        trash_after >= trash_before - 1,
        "at most 1 card should leave trash when only 1 was eligible"
    );
}

/// Positive: Mineral/Rock Digimon on field + 2 eligible trash cards → OwnField
/// selection installs for the destination pick.
#[test]
fn ex10_025_on_play_installs_own_field_selection_when_condition_met() {
    let host = make_mineral_host("HOST-COND-POS");
    let min1 = make_mineral("MIN-POS-1");
    let min2 = make_mineral("MIN-POS-2");
    let filler = make_plain("FILL-DECK");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-025")
        .expect("EX10-025 parses")
        .add_card(host)
        .add_card(min1)
        .add_card(min2)
        .add_card(filler)
        .hand(0, &["EX10-025"])
        .deck(0, &["FILL-DECK"])
        .memory(10)
        .start();

    runner.place_on_field(0, "HOST-COND-POS", None);
    push_to_trash(&mut runner, "MIN-POS-1");
    push_to_trash(&mut runner, "MIN-POS-2");

    runner.play(0, 0);

    let kind = runner.pending_kind();
    assert_eq!(
        kind,
        Some(SelectionKind::OwnField),
        "OwnField selection must install for the destination Digimon pick"
    );
    assert!(
        runner.pending_is_optional(),
        "destination pick must be optional (\"you may\" at clause level)"
    );
}

/// Positive: Rock trait Digimon is also accepted as a valid destination.
#[test]
fn ex10_025_on_play_rock_host_is_valid_destination() {
    let host = make_rock_host("ROCK-HOST-DEST");
    let min1 = make_mineral("MIN-RH-1");
    let min2 = make_mineral("MIN-RH-2");
    let filler = make_plain("FILL-DECK");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-025")
        .expect("EX10-025 parses")
        .add_card(host)
        .add_card(min1)
        .add_card(min2)
        .add_card(filler)
        .hand(0, &["EX10-025"])
        .deck(0, &["FILL-DECK"])
        .memory(10)
        .start();

    runner.place_on_field(0, "ROCK-HOST-DEST", None);
    push_to_trash(&mut runner, "MIN-RH-1");
    push_to_trash(&mut runner, "MIN-RH-2");

    runner.play(0, 0);

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::OwnField),
        "OwnField selection must install when a Rock trait Digimon is on field"
    );
    assert!(
        runner.pending_action_count() >= 1,
        "the Rock host must appear as a valid destination"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — OnPlay behavioral: two sequential trash placements
// ═══════════════════════════════════════════════════════════════════════════════

/// Full positive path: place 2 Mineral trash cards under a Mineral host.
/// After resolving, the host gains +2 bottom sources and trash shrinks by 2.
#[test]
fn ex10_025_on_play_places_two_trash_cards_under_target_digimon() {
    let host = make_mineral_host("HOST-PLACE2");
    let min1 = make_mineral("MIN-PLACE-A");
    let min2 = make_mineral("MIN-PLACE-B");
    let filler = make_plain("FILL-DECK");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-025")
        .expect("EX10-025 parses")
        .add_card(host)
        .add_card(min1)
        .add_card(min2)
        .add_card(filler)
        .hand(0, &["EX10-025"])
        .deck(0, &["FILL-DECK"])
        .memory(10)
        .start();

    let host_handle = runner.place_on_field(0, "HOST-PLACE2", None);
    push_to_trash(&mut runner, "MIN-PLACE-A");
    push_to_trash(&mut runner, "MIN-PLACE-B");

    let sources_before = runner.game.players[0].battle_area[host_handle.index as usize]
        .card_sources
        .len();
    let trash_before = runner.trash_size(0);

    runner.play(0, 0);

    // Step 1: OwnField → pick the Mineral host
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::OwnField),
        "step 1 must prompt OwnField (destination pick)"
    );
    {
        let view = runner.pending_selection_view().unwrap();
        runner
            .execute_action(view.selecting_player, view.valid_action_ids[0])
            .expect("pick Mineral host as destination");
    }

    // Step 2: Trash → pick first Mineral card
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Trash),
        "step 2 must prompt Trash (first source pick)"
    );
    {
        let view = runner.pending_selection_view().unwrap();
        runner
            .execute_action(view.selecting_player, view.valid_action_ids[0])
            .expect("pick first Mineral card from trash");
    }

    // Step 3: Trash → pick second Mineral card
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Trash),
        "step 3 must prompt Trash (second source pick)"
    );
    {
        let view = runner.pending_selection_view().unwrap();
        runner
            .execute_action(view.selecting_player, view.valid_action_ids[0])
            .expect("pick second Mineral card from trash");
    }

    let _ = runner.auto_resolve();

    let sources_after = runner.game.players[0].battle_area[host_handle.index as usize]
        .card_sources
        .len();
    let trash_after = runner.trash_size(0);

    assert_eq!(
        sources_after,
        sources_before + 2,
        "host must gain exactly 2 bottom digivolution cards (before: {sources_before}, after: {sources_after})"
    );
    assert_eq!(
        trash_after,
        trash_before - 2,
        "trash must shrink by exactly 2 (before: {trash_before}, after: {trash_after})"
    );
}

/// Mixed Mineral and Rock trash cards are both eligible; placing one of each works.
#[test]
fn ex10_025_on_play_accepts_both_mineral_and_rock_trash_cards() {
    let host = make_mineral_host("HOST-MIXEDTR");
    let min1 = make_mineral("MIN-MIX");
    let rock1 = make_rock("ROCK-MIX");
    let filler = make_plain("FILL-DECK");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-025")
        .expect("EX10-025 parses")
        .add_card(host)
        .add_card(min1)
        .add_card(rock1)
        .add_card(filler)
        .hand(0, &["EX10-025"])
        .deck(0, &["FILL-DECK"])
        .memory(10)
        .start();

    let host_handle = runner.place_on_field(0, "HOST-MIXEDTR", None);
    push_to_trash(&mut runner, "MIN-MIX");
    push_to_trash(&mut runner, "ROCK-MIX");

    let sources_before = runner.game.players[0].battle_area[host_handle.index as usize]
        .card_sources
        .len();
    let trash_before = runner.trash_size(0);

    runner.play(0, 0);
    let _ = runner.auto_resolve();

    let sources_after = runner.game.players[0].battle_area[host_handle.index as usize]
        .card_sources
        .len();
    let trash_after = runner.trash_size(0);

    assert_eq!(
        sources_after,
        sources_before + 2,
        "host must gain 2 sources after placing 1 Mineral + 1 Rock trash card"
    );
    assert_eq!(
        trash_after,
        trash_before - 2,
        "trash must shrink by 2 after placing 1 Mineral + 1 Rock"
    );
}

/// The first trash card placed is no longer in trash when the second pick fires,
/// so it cannot be re-selected. The second Trash selection must offer 1 fewer option.
#[test]
fn ex10_025_on_play_first_pick_leaves_trash_before_second_pick() {
    let host = make_mineral_host("HOST-SEQTR");
    let min1 = make_mineral("MIN-SEQ-1");
    let min2 = make_mineral("MIN-SEQ-2");
    let min3 = make_mineral("MIN-SEQ-3"); // third card so second pick still has >1 choice
    let filler = make_plain("FILL-DECK");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-025")
        .expect("EX10-025 parses")
        .add_card(host)
        .add_card(min1)
        .add_card(min2)
        .add_card(min3)
        .add_card(filler)
        .hand(0, &["EX10-025"])
        .deck(0, &["FILL-DECK"])
        .memory(10)
        .start();

    runner.place_on_field(0, "HOST-SEQTR", None);
    push_to_trash(&mut runner, "MIN-SEQ-1");
    push_to_trash(&mut runner, "MIN-SEQ-2");
    push_to_trash(&mut runner, "MIN-SEQ-3");

    runner.play(0, 0);

    // Pick destination.
    {
        let view = runner.pending_selection_view().unwrap();
        runner
            .execute_action(view.selecting_player, view.valid_action_ids[0])
            .expect("pick dest");
    }

    // Trash pick 1: all 3 eligible cards available.
    let count_at_first_pick = {
        let view = runner.pending_selection_view().unwrap();
        let count = view.valid_action_ids.len();
        runner
            .execute_action(view.selecting_player, view.valid_action_ids[0])
            .expect("pick first trash card");
        count
    };

    // Trash pick 2: the first card is no longer in trash → 1 fewer option.
    let count_at_second_pick = runner.pending_action_count();

    assert_eq!(
        count_at_second_pick,
        count_at_first_pick - 1,
        "second trash pick must have 1 fewer option because the first pick moved to sources"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — Optional decline
// ═══════════════════════════════════════════════════════════════════════════════

/// Declining the OnPlay (passing at the OwnField prompt) leaves sources and
/// trash completely unchanged.
#[test]
fn ex10_025_on_play_decline_leaves_sources_and_trash_unchanged() {
    let host = make_mineral_host("HOST-DECLINE");
    let min1 = make_mineral("MIN-DEC-1");
    let min2 = make_mineral("MIN-DEC-2");
    let filler = make_plain("FILL-DECK");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-025")
        .expect("EX10-025 parses")
        .add_card(host)
        .add_card(min1)
        .add_card(min2)
        .add_card(filler)
        .hand(0, &["EX10-025"])
        .deck(0, &["FILL-DECK"])
        .memory(10)
        .start();

    let host_handle = runner.place_on_field(0, "HOST-DECLINE", None);
    push_to_trash(&mut runner, "MIN-DEC-1");
    push_to_trash(&mut runner, "MIN-DEC-2");

    let sources_before = runner.game.players[0].battle_area[host_handle.index as usize]
        .card_sources
        .len();
    let trash_before = runner.trash_size(0);

    runner.play(0, 0);

    // Confirm OwnField prompt is pending (optional).
    assert_eq!(runner.pending_kind(), Some(SelectionKind::OwnField));
    assert!(runner.pending_is_optional(), "must be declinable with PASS");

    // Decline.
    use digimon_engine::action::space::PASS;
    runner
        .execute_action(0, PASS)
        .expect("PASS declines the optional OnPlay");
    let _ = runner.auto_resolve();

    let sources_after = runner.game.players[0].battle_area[host_handle.index as usize]
        .card_sources
        .len();
    let trash_after = runner.trash_size(0);

    assert_eq!(
        sources_after, sources_before,
        "declining must leave source count unchanged"
    );
    assert_eq!(
        trash_after, trash_before,
        "declining must leave trash count unchanged"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 5 — Filter: non-Mineral/non-Rock trash cards are ineligible
// ═══════════════════════════════════════════════════════════════════════════════

/// Trash contains both eligible (Mineral) and ineligible (plain) cards.
/// The Trash selection must offer only the Mineral/Rock cards.
#[test]
fn ex10_025_on_play_trash_selection_excludes_non_mineral_rock_cards() {
    let host = make_mineral_host("HOST-FILTER");
    let min1 = make_mineral("MIN-FILT-A");
    let min2 = make_mineral("MIN-FILT-B");
    let plain1 = make_plain("PLAIN-FILT-1");
    let plain2 = make_plain("PLAIN-FILT-2");
    let filler = make_plain("FILL-DECK");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-025")
        .expect("EX10-025 parses")
        .add_card(host)
        .add_card(min1)
        .add_card(min2)
        .add_card(plain1)
        .add_card(plain2)
        .add_card(filler)
        .hand(0, &["EX10-025"])
        .deck(0, &["FILL-DECK"])
        .memory(10)
        .start();

    runner.place_on_field(0, "HOST-FILTER", None);
    // 2 eligible + 2 ineligible in trash
    push_to_trash(&mut runner, "MIN-FILT-A");
    push_to_trash(&mut runner, "MIN-FILT-B");
    push_to_trash(&mut runner, "PLAIN-FILT-1");
    push_to_trash(&mut runner, "PLAIN-FILT-2");

    runner.play(0, 0);

    // Accept the OwnField destination pick.
    {
        let view = runner.pending_selection_view().unwrap();
        runner
            .execute_action(view.selecting_player, view.valid_action_ids[0])
            .expect("pick dest");
    }

    // First Trash pick: only the 2 Mineral cards must be offered.
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Trash),
        "Trash selection must install for first source pick"
    );
    assert_eq!(
        runner.pending_action_count(),
        2,
        "only the 2 Mineral trash cards must be valid targets; plain cards must be excluded"
    );
}

/// OwnField destination selection excludes Digimon with no Mineral/Rock trait.
#[test]
fn ex10_025_on_play_own_field_selection_excludes_non_mineral_rock_digimon() {
    let mineral_host = make_mineral_host("HOST-MINCLUDE");
    let plain_host = make_plain("HOST-EXCLUDED");
    let min1 = make_mineral("MIN-EXC-1");
    let min2 = make_mineral("MIN-EXC-2");
    let filler = make_plain("FILL-DECK");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-025")
        .expect("EX10-025 parses")
        .add_card(mineral_host)
        .add_card(plain_host)
        .add_card(min1)
        .add_card(min2)
        .add_card(filler)
        .hand(0, &["EX10-025"])
        .deck(0, &["FILL-DECK"])
        .memory(10)
        .start();

    runner.place_on_field(0, "HOST-MINCLUDE", None);
    runner.place_on_field(0, "HOST-EXCLUDED", None);
    push_to_trash(&mut runner, "MIN-EXC-1");
    push_to_trash(&mut runner, "MIN-EXC-2");

    runner.play(0, 0);

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::OwnField),
        "OwnField selection must install"
    );
    assert_eq!(
        runner.pending_action_count(),
        1,
        "only the 1 Mineral/Rock Digimon must be offered; plain Digimon must be excluded"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 6 — Inherited: source-trash delete clause
// ═══════════════════════════════════════════════════════════════════════════════

/// Positive: trashing EX10-025 from a Mineral host installs an OppField
/// selection that only offers opponent Digimon with play cost ≤ 4.
/// A play-cost-5 Digimon must be excluded.
#[test]
fn ex10_025_source_trash_deletes_only_opponent_digimon_cost_four_or_less() {
    let mut host = make_mineral_host("MINERAL-HOST");
    host.traits.push("Mineral".to_string());
    let mut cheap = make_test_card("CHEAP4", "Cheap Cost 4");
    cheap.play_cost = 4;
    let mut expensive = make_test_card("EXPENSIVE5", "Expensive Cost 5");
    expensive.play_cost = 5;

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-025")
        .expect("EX10-025 YAML parses and compiles")
        .add_card(host)
        .add_card(cheap)
        .add_card(expensive)
        .build();

    let host = runner.place_on_field(0, "MINERAL-HOST", None);
    let source = runner.push_source(host, "EX10-025");
    runner.place_on_field(1, "CHEAP4", None);
    runner.place_on_field(1, "EXPENSIVE5", None);

    let top = runner.top_card(host);
    {
        let mut ctx = EffectContext::new(&mut runner.game, top, Some(host), 0);
        ctx.trash_card_source(host, source);
    }

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::OppField),
        "OppField selection must install after source is trashed"
    );
    assert_eq!(
        runner.pending_action_count(),
        1,
        "only the play-cost-4 opponent Digimon must be selectable; cost-5 must be excluded"
    );

    runner
        .auto_resolve()
        .expect("EX10-025 delete target resolves");

    let remaining: Vec<&str> = runner
        .game
        .player(1)
        .battle_area
        .iter()
        .map(|perm| perm.top_card().card_id(&runner.game.card_data))
        .collect();
    assert_eq!(
        remaining,
        vec!["EXPENSIVE5"],
        "the cost-4 Digimon must be deleted; the cost-5 Digimon must survive"
    );
}

/// Boundary: play cost exactly 4 is included (≤ 4).
#[test]
fn ex10_025_source_trash_includes_play_cost_exactly_four() {
    let mut host = make_mineral_host("MINERAL-HOST2");
    host.traits.push("Mineral".to_string());
    let mut cost4 = make_test_card("COST4", "Cost-4 Opp");
    cost4.play_cost = 4;

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-025")
        .expect("EX10-025 YAML parses and compiles")
        .add_card(host)
        .add_card(cost4)
        .build();

    let host = runner.place_on_field(0, "MINERAL-HOST2", None);
    let source = runner.push_source(host, "EX10-025");
    runner.place_on_field(1, "COST4", None);

    let top = runner.top_card(host);
    {
        let mut ctx = EffectContext::new(&mut runner.game, top, Some(host), 0);
        ctx.trash_card_source(host, source);
    }

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::OppField),
        "cost-4 target must be offered"
    );
    assert_eq!(runner.pending_action_count(), 1);

    runner.auto_resolve().expect("resolves");

    assert_eq!(
        runner.game.player(1).battle_area.len(),
        0,
        "cost-4 Digimon must be deleted"
    );
}

/// Boundary: play cost exactly 5 is excluded (> 4).
#[test]
fn ex10_025_source_trash_excludes_play_cost_exactly_five() {
    let mut host = make_mineral_host("MINERAL-HOST3");
    host.traits.push("Mineral".to_string());
    let mut cost5 = make_test_card("COST5", "Cost-5 Opp");
    cost5.play_cost = 5;

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-025")
        .expect("EX10-025 YAML parses and compiles")
        .add_card(host)
        .add_card(cost5)
        .build();

    let host = runner.place_on_field(0, "MINERAL-HOST3", None);
    let source = runner.push_source(host, "EX10-025");
    runner.place_on_field(1, "COST5", None);

    let top = runner.top_card(host);
    {
        let mut ctx = EffectContext::new(&mut runner.game, top, Some(host), 0);
        ctx.trash_card_source(host, source);
    }

    assert!(
        runner.pending_selection().is_none(),
        "no selection must install when only a cost-5 Digimon is present (condition fails)"
    );
    assert_eq!(
        runner.game.player(1).battle_area.len(),
        1,
        "cost-5 Digimon must not be deleted"
    );
}

/// Negative: no eligible opponent Digimon (≤ 4 cost) → condition fails, no
/// selection installs.
#[test]
fn ex10_025_source_trash_no_prompt_when_no_valid_opp_targets() {
    let mut host = make_mineral_host("MINERAL-HOST4");
    host.traits.push("Mineral".to_string());
    let mut costly = make_test_card("COSTLY", "High Cost Opp");
    costly.play_cost = 7;

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-025")
        .expect("EX10-025 YAML parses and compiles")
        .add_card(host)
        .add_card(costly)
        .build();

    let host = runner.place_on_field(0, "MINERAL-HOST4", None);
    let source = runner.push_source(host, "EX10-025");
    runner.place_on_field(1, "COSTLY", None);

    let top = runner.top_card(host);
    {
        let mut ctx = EffectContext::new(&mut runner.game, top, Some(host), 0);
        ctx.trash_card_source(host, source);
    }

    assert!(
        runner.pending_selection().is_none(),
        "no selection must install when the only opponent Digimon costs more than 4"
    );
}

/// Negative: trashing EX10-025 from a non-Mineral/non-Rock host must NOT
/// trigger the delete effect (condition `host_permanent_trait_has: Mineral/Rock`
/// fails for a plain host).
#[test]
fn ex10_025_source_trash_no_effect_from_non_mineral_rock_host() {
    let plain_host = make_plain("PLAIN-HOST");
    let mut cheap = make_test_card("CHEAP4-NH", "Cheap Cost 4 NH");
    cheap.play_cost = 4;

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-025")
        .expect("EX10-025 YAML parses and compiles")
        .add_card(plain_host)
        .add_card(cheap)
        .build();

    let host = runner.place_on_field(0, "PLAIN-HOST", None);
    let source = runner.push_source(host, "EX10-025");
    runner.place_on_field(1, "CHEAP4-NH", None);

    let top = runner.top_card(host);
    {
        let mut ctx = EffectContext::new(&mut runner.game, top, Some(host), 0);
        ctx.trash_card_source(host, source);
    }

    assert!(
        runner.pending_selection().is_none(),
        "delete effect must NOT fire when the host has no Mineral/Rock trait"
    );
    assert_eq!(
        runner.game.player(1).battle_area.len(),
        1,
        "opponent Digimon must survive when host is not Mineral/Rock"
    );
}

/// Rock trait host also satisfies the inherited condition.
#[test]
fn ex10_025_source_trash_fires_from_rock_host() {
    let mut host = make_rock_host("ROCK-HOST-INH");
    host.traits.push("Rock".to_string());
    let mut cheap = make_test_card("CHEAP4-RH", "Cheap Cost 4 RH");
    cheap.play_cost = 3;

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-025")
        .expect("EX10-025 YAML parses and compiles")
        .add_card(host)
        .add_card(cheap)
        .build();

    let host = runner.place_on_field(0, "ROCK-HOST-INH", None);
    let source = runner.push_source(host, "EX10-025");
    runner.place_on_field(1, "CHEAP4-RH", None);

    let top = runner.top_card(host);
    {
        let mut ctx = EffectContext::new(&mut runner.game, top, Some(host), 0);
        ctx.trash_card_source(host, source);
    }

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::OppField),
        "OppField selection must install when EX10-025 is trashed from a Rock host"
    );

    runner.auto_resolve().expect("resolves");

    assert_eq!(
        runner.game.player(1).battle_area.len(),
        0,
        "opponent Digimon must be deleted when trashed from a Rock host"
    );
}

/// Multiple eligible opponent Digimon (all cost ≤ 4): the selection offers
/// exactly all of them; after resolving one is deleted and one remains.
#[test]
fn ex10_025_source_trash_with_multiple_eligible_opp_digimon_offers_all() {
    let mut host = make_mineral_host("MINERAL-HOST5");
    host.traits.push("Mineral".to_string());
    let mut opp1 = make_test_card("OPP-A", "Opp A");
    opp1.play_cost = 2;
    let mut opp2 = make_test_card("OPP-B", "Opp B");
    opp2.play_cost = 4;

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-025")
        .expect("EX10-025 YAML parses and compiles")
        .add_card(host)
        .add_card(opp1)
        .add_card(opp2)
        .build();

    let host = runner.place_on_field(0, "MINERAL-HOST5", None);
    let source = runner.push_source(host, "EX10-025");
    runner.place_on_field(1, "OPP-A", None);
    runner.place_on_field(1, "OPP-B", None);

    let top = runner.top_card(host);
    {
        let mut ctx = EffectContext::new(&mut runner.game, top, Some(host), 0);
        ctx.trash_card_source(host, source);
    }

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::OppField),
        "OppField selection must install"
    );
    assert_eq!(
        runner.pending_action_count(),
        2,
        "both cost-≤4 opponent Digimon must be offered"
    );

    runner.auto_resolve().expect("delete first valid target");

    assert_eq!(
        runner.game.player(1).battle_area.len(),
        1,
        "exactly one opponent Digimon must be deleted; one must survive"
    );
}
