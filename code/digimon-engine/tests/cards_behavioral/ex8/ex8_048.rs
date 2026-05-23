//! EX8-048 Landramon — Digimon, Lv.4, Black, DP 4000, Cost 4.
//! Traits: Mineral, LIBERATOR
//! Evo: Lv3 Black / 2 memory
//!
//! # Card text (cards.json)
//!
//! **[When Digivolving]** If you have 1 or fewer Tamers, you may play 1
//! [Close] from your hand without paying the cost.
//!
//! **Inherited:** When effects trash this card from a [Mineral] or [Rock]
//! trait Digimon's digivolution cards, delete 1 of your opponent's Digimon
//! with a play cost of 4 or less.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX8/Black/EX8_048.cs
//!
//! # Patterns this test covers
//! - Clause (a): [When Digivolving] optional conditional play Tamer from hand free
//!   (count_lte Tamer condition + play_from_hand_free; same pattern as BT21-017 Dimetromon)
//! - Clause (b): Inherited on_digivolution_card_trashed → delete opp Digimon cost ≤4

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming};
use digimon_engine::selection::{SelectionKind, TriggerSource};

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn make_tamer(id: &str) -> digimon_engine::card_data::CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c
}

fn make_close_tamer(id: &str) -> digimon_engine::card_data::CardData {
    let mut c = make_test_card(id, "Close");
    c.card_kind = CardKind::Tamer;
    c.play_cost = 3;
    c
}

fn make_filler(id: &str) -> digimon_engine::card_data::CardData {
    make_test_card(id, id)
}

/// Build a base runner with only EX8-048 loaded (structural / compilation tests).
fn landramon_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX8-048")
        .expect("EX8-048 YAML parses and compiles")
        .memory(10)
        .build()
}

/// Fire WhenDigivolving for the given permanent handle.
fn fire_when_digivolving(
    runner: &mut DebugRunner,
    handle: digimon_engine::permanent::PermanentHandle,
) {
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(handle),
    );
    runner.game.drain_effect_queue();
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn ex8_048_compiles_with_two_clauses() {
    let runner = landramon_runner();
    let compiled = runner
        .compiled_card("EX8-048")
        .expect("EX8-048 in compiled_cards");

    assert_eq!(
        compiled.effects.len(),
        2,
        "EX8-048 must have exactly 2 clauses: (a) when_digivolving + (b) inherited trash-delete"
    );
}

#[test]
fn ex8_048_has_when_digivolving_triggered_clause() {
    let runner = landramon_runner();
    let compiled = runner
        .compiled_card("EX8-048")
        .expect("EX8-048 in compiled_cards");

    let has_wd = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .any(|t| t.when.contains(&CompiledTiming::WhenDigivolving));

    assert!(
        has_wd,
        "EX8-048 must have a triggered clause with when_digivolving"
    );
}

#[test]
fn ex8_048_when_digivolving_clause_is_optional() {
    let runner = landramon_runner();
    let compiled = runner
        .compiled_card("EX8-048")
        .expect("EX8-048 in compiled_cards");

    let wd_clause = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::WhenDigivolving))
        .expect("WhenDigivolving clause must exist");

    assert!(
        wd_clause.optional,
        "WhenDigivolving clause must be optional ('you may')"
    );
}

#[test]
fn ex8_048_when_digivolving_clause_is_face_up_scope() {
    let runner = landramon_runner();
    let compiled = runner
        .compiled_card("EX8-048")
        .expect("EX8-048 in compiled_cards");

    let wd_clause = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::WhenDigivolving))
        .expect("WhenDigivolving clause must exist");

    assert_eq!(
        wd_clause.scope,
        CompiledScope::FaceUp,
        "WhenDigivolving clause must have FaceUp (default own) scope"
    );
}

#[test]
fn ex8_048_when_digivolving_clause_is_not_once_per_turn() {
    let runner = landramon_runner();
    let compiled = runner
        .compiled_card("EX8-048")
        .expect("EX8-048 in compiled_cards");

    let wd_clause = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::WhenDigivolving))
        .expect("WhenDigivolving clause must exist");

    assert!(
        !wd_clause.once_per_turn,
        "WhenDigivolving clause must NOT be once_per_turn (no OPT printed)"
    );
}

#[test]
fn ex8_048_has_inherited_source_trash_delete_clause() {
    let runner = landramon_runner();
    let card = runner
        .compiled_card("EX8-048")
        .expect("EX8-048 compiled card present");

    assert!(card.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Triggered(triggered)
            if triggered.scope == CompiledScope::Inherited
                && triggered.when == vec![CompiledTiming::OnDigivolutionCardTrashed]
    )));
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Condition gating (WhenDigivolving clause)
// ═══════════════════════════════════════════════════════════════════════════════

/// Positive: P0 has 0 Tamers and [Close] in hand → WhenDigivolving fires and
/// offers hand selection prompt.
#[test]
fn ex8_048_condition_passes_with_zero_tamers_and_close_in_hand() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-048")
        .expect("EX8-048 YAML parses")
        .add_card(make_close_tamer("CLOSE-1"))
        .add_card(make_filler("FILLER-DECK"))
        .hand(0, &["CLOSE-1"])
        .deck(0, &["FILLER-DECK", "FILLER-DECK", "FILLER-DECK"])
        .memory(10)
        .start();

    // P0 has 0 tamers on field → condition count_lte ≤1 passes.
    let handle = runner.place_on_field(0, "EX8-048", Some(0));
    fire_when_digivolving(&mut runner, handle);

    let pending = runner.pending_selection();
    assert!(
        pending.is_some(),
        "WhenDigivolving should install selection when 0 tamers and [Close] in hand"
    );
}

/// Positive: P0 has exactly 1 Tamer → count ≤ 1 still passes (boundary).
#[test]
fn ex8_048_condition_passes_with_one_tamer_boundary() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-048")
        .expect("EX8-048 YAML parses")
        .add_card(make_tamer("TAMER-1"))
        .add_card(make_close_tamer("CLOSE-1"))
        .add_card(make_filler("FILLER-DECK"))
        .deck(0, &["FILLER-DECK", "FILLER-DECK", "FILLER-DECK"])
        .hand(0, &["CLOSE-1"])
        .memory(10)
        .start();

    // Place 1 tamer on field — still ≤1.
    let _ = runner.place_on_field(0, "TAMER-1", Some(0));
    let handle = runner.place_on_field(0, "EX8-048", Some(0));
    fire_when_digivolving(&mut runner, handle);

    let pending = runner.pending_selection();
    assert!(
        pending.is_some(),
        "WhenDigivolving should install selection when exactly 1 tamer present (≤1 boundary passes)"
    );
}

/// Negative: P0 has 2 Tamers → condition count > 1, effect does NOT fire.
#[test]
fn ex8_048_condition_blocked_with_two_tamers() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-048")
        .expect("EX8-048 YAML parses")
        .add_card(make_tamer("TAMER-1"))
        .add_card(make_tamer("TAMER-2"))
        .add_card(make_close_tamer("CLOSE-1"))
        .add_card(make_filler("FILLER-DECK"))
        .deck(0, &["FILLER-DECK", "FILLER-DECK", "FILLER-DECK"])
        .hand(0, &["CLOSE-1"])
        .memory(10)
        .start();

    // Place 2 tamers on field → condition fails.
    let _ = runner.place_on_field(0, "TAMER-1", Some(0));
    let _ = runner.place_on_field(0, "TAMER-2", Some(0));
    let handle = runner.place_on_field(0, "EX8-048", Some(0));
    fire_when_digivolving(&mut runner, handle);

    let pending = runner.pending_selection();
    assert!(
        pending.is_none(),
        "WhenDigivolving must NOT install selection when 2 tamers present (condition fails)"
    );
}

/// Negative: P0 has 0 Tamers but NO [Close] in hand →
/// the select_hand filter yields no candidates; no prompt.
#[test]
fn ex8_048_condition_blocked_when_no_close_in_hand() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-048")
        .expect("EX8-048 YAML parses")
        .add_card(make_filler("FILLER-DECK"))
        .deck(0, &["FILLER-DECK", "FILLER-DECK", "FILLER-DECK"])
        // Empty hand — no Close present.
        .memory(10)
        .start();

    let handle = runner.place_on_field(0, "EX8-048", Some(0));
    fire_when_digivolving(&mut runner, handle);

    let pending = runner.pending_selection();
    assert!(
        pending.is_none(),
        "WhenDigivolving must NOT install selection when hand has no [Close]"
    );
}

/// Name-filter precision: a card whose name is NOT exactly "Close" must not
/// appear in the selection (e.g. a card named "Closely" is not a match).
#[test]
fn ex8_048_name_filter_excludes_near_miss_card_names() {
    let mut near_miss = make_test_card("NEAR-MISS", "Closely");
    near_miss.card_kind = CardKind::Tamer;
    near_miss.play_cost = 3;

    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-048")
        .expect("EX8-048 YAML parses")
        .add_card(near_miss)
        .add_card(make_filler("FILLER-DECK"))
        .hand(0, &["NEAR-MISS"])
        .deck(0, &["FILLER-DECK", "FILLER-DECK", "FILLER-DECK"])
        .memory(10)
        .start();

    let handle = runner.place_on_field(0, "EX8-048", Some(0));
    fire_when_digivolving(&mut runner, handle);

    // "Closely" does not match the exact-name filter for "Close" → no prompt.
    let pending = runner.pending_selection();
    assert!(
        pending.is_none(),
        "WhenDigivolving must NOT match a card named 'Closely' (name_is exact filter)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Behavioral outcome (WhenDigivolving clause)
// ═══════════════════════════════════════════════════════════════════════════════

/// When player selects [Close], it is played for free onto the field.
#[test]
fn ex8_048_playing_close_moves_it_to_field_for_free() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-048")
        .expect("EX8-048 YAML parses")
        .add_card(make_close_tamer("CLOSE-1"))
        .add_card(make_filler("FILLER-DECK"))
        .hand(0, &["CLOSE-1"])
        .deck(0, &["FILLER-DECK", "FILLER-DECK", "FILLER-DECK"])
        .memory(10)
        .start();

    let handle = runner.place_on_field(0, "EX8-048", Some(0));
    let hand_before = runner.hand_size(0);
    let field_before = runner.battle_area_size(0); // includes Landramon itself

    fire_when_digivolving(&mut runner, handle);

    assert!(
        runner.pending_selection().is_some(),
        "expect hand selection prompt for [Close]"
    );
    {
        let view = runner.pending_selection_view().unwrap();
        runner
            .execute_action(0, view.valid_action_ids[0])
            .expect("select Close from hand");
    }
    let _ = runner.auto_resolve();

    // Close (a tamer) should now be on field.
    let field_after = runner.battle_area_size(0);
    assert!(
        field_after > field_before,
        "[Close] should be on field after playing free; field_before={field_before}, field_after={field_after}"
    );

    // Close is no longer in hand.
    let hand_after = runner.hand_size(0);
    assert!(
        hand_after < hand_before,
        "[Close] must no longer be in hand after being played; hand_before={hand_before}, hand_after={hand_after}"
    );
}

/// [Close] is played for free — memory must not be spent.
#[test]
fn ex8_048_playing_close_free_does_not_spend_memory() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-048")
        .expect("EX8-048 YAML parses")
        .add_card(make_close_tamer("CLOSE-1"))
        .add_card(make_filler("FILLER-DECK"))
        .hand(0, &["CLOSE-1"])
        .deck(0, &["FILLER-DECK", "FILLER-DECK", "FILLER-DECK"])
        .memory(5)
        .start();

    let handle = runner.place_on_field(0, "EX8-048", Some(0));
    fire_when_digivolving(&mut runner, handle);

    let memory_before = runner.memory();

    {
        let view = runner.pending_selection_view().unwrap();
        runner
            .execute_action(0, view.valid_action_ids[0])
            .expect("select Close");
    }
    let _ = runner.auto_resolve();

    let memory_after = runner.memory();
    assert_eq!(
        memory_after, memory_before,
        "play_from_hand_free must not spend memory for [Close]'s cost; memory_before={memory_before}, memory_after={memory_after}"
    );
}

/// Declining (PASS) when prompt is optional leaves [Close] in hand.
#[test]
fn ex8_048_declining_leaves_close_in_hand() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-048")
        .expect("EX8-048 YAML parses")
        .add_card(make_close_tamer("CLOSE-1"))
        .add_card(make_filler("FILLER-DECK"))
        .hand(0, &["CLOSE-1"])
        .deck(0, &["FILLER-DECK", "FILLER-DECK", "FILLER-DECK"])
        .memory(10)
        .start();

    let handle = runner.place_on_field(0, "EX8-048", Some(0));
    let hand_before = runner.hand_size(0);
    let field_before = runner.battle_area_size(0);

    fire_when_digivolving(&mut runner, handle);

    // Prompt is optional → PASS is valid.
    assert!(
        runner.pending_is_optional(),
        "WhenDigivolving selection must be optional (player may decline)"
    );

    runner
        .execute_action(0, digimon_engine::action::space::PASS)
        .expect("pass the optional selection");
    let _ = runner.auto_resolve();

    let hand_after = runner.hand_size(0);
    assert_eq!(
        hand_after, hand_before,
        "[Close] must remain in hand after declining; hand_before={hand_before}, hand_after={hand_after}"
    );

    let field_after = runner.battle_area_size(0);
    assert_eq!(
        field_after, field_before,
        "No new permanent from declining; field_before={field_before}, field_after={field_after}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — Inherited source-trash delete clause (existing, kept intact)
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn ex8_048_source_trash_deletes_only_opponent_digimon_cost_four_or_less() {
    use digimon_engine::effect_context::EffectContext;

    let mut host = make_test_card("MINERAL-HOST", "Mineral Host");
    host.traits.push("Mineral".to_string());
    let mut cheap = make_test_card("CHEAP4", "Cheap Cost 4");
    cheap.play_cost = 4;
    let mut expensive = make_test_card("EXPENSIVE5", "Expensive Cost 5");
    expensive.play_cost = 5;

    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-048")
        .expect("EX8-048 YAML parses and compiles")
        .add_card(host)
        .add_card(cheap)
        .add_card(expensive)
        .build();

    let host = runner.place_on_field(0, "MINERAL-HOST", None);
    let source = runner.push_source(host, "EX8-048");
    runner.place_on_field(1, "CHEAP4", None);
    runner.place_on_field(1, "EXPENSIVE5", None);

    let top = runner.top_card(host);
    {
        let mut ctx = EffectContext::new(&mut runner.game, top, Some(host), 0);
        ctx.trash_card_source(host, source);
    }

    assert_eq!(runner.pending_kind(), Some(SelectionKind::OppField));
    assert_eq!(runner.pending_action_count(), 1);
    runner
        .auto_resolve()
        .expect("EX8-048 delete target resolves");

    let remaining: Vec<&str> = runner
        .game
        .player(1)
        .battle_area
        .iter()
        .map(|perm| perm.top_card().card_id(&runner.game.card_data))
        .collect();
    assert_eq!(remaining, vec!["EXPENSIVE5"]);
}
