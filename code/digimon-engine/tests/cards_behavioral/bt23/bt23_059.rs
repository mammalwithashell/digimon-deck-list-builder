//! BT23-059 Justimon: Blitz Arm — Digimon, Lv.6, Black, DP 11000, Cost 11.
//! Traits: Cyborg, Hudie, CS. Attribute: Vaccine.
//! Evo: Lv.5 w/[CS] trait / Cost 3.
//!
//! # Card text (cards.json — printed)
//!
//! ```text
//! <Blocker>
//! [On Play] [When Digivolving] [When Attacking] [Once Per Turn]
//!   By trashing 1 Option card in the battle area, delete 1 of your
//!   opponent's Digimon with the lowest play cost.
//! [All Turns] [Once Per Turn]
//!   When Option cards in the battle area are trashed, this Digimon
//!   unsuspends. Then, your opponent's Digimon's effects don't affect
//!   this Digimon for the turn.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT23/Black/BT23_059.cs
//!
//! # Patterns this test covers
//! - H5 Blocker (declarative grant_keyword)
//! - Clause A: multi-timing OPT (On Play / When Digivolving / When Attacking),
//!   pay-cost option trash, lowest-play-cost delete selector
//! - Clause B: `on_option_trashed` OPT unsuspend plus opponent-Digimon
//!   effect immunity
//!
//! # Gap summary
//! - Clause A: IMPLEMENTED.
//! - Clause B: IMPLEMENTED (G-ON-OPTION-TRASHED-DSL resolved 2026-05-23).

use digimon_dsl::compiled::{
    CompiledClause, CompiledScope, CompiledTiming, CompiledTriggeredClause,
};
use digimon_engine::action::space::{encode_attack, REPLACEMENT_ACCEPT};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectSourceKind, EffectTiming, Keyword, PlayerId};
use digimon_engine::option_lifecycle::OptionTrashCause;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT23-059")
        .expect("BT23-059 YAML parses and compiles")
        .memory(10)
        .start()
}

fn make_option_card(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Option;
    c
}

fn make_digimon(id: &str, play_cost: u16) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = play_cost;
    c
}

/// Fire a given EffectTiming for `handle`.
fn fire_timing(runner: &mut DebugRunner, timing: EffectTiming, handle: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(timing, TriggerSource::Permanent(handle));
    runner.game.drain_effect_queue();
}

fn field_handle_by_card_id(
    runner: &DebugRunner,
    player: PlayerId,
    card_id: &str,
) -> PermanentHandle {
    let index = runner
        .game
        .player(player)
        .battle_area
        .iter()
        .position(|permanent| permanent.top_card().card_id(&runner.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} must be in player {player}'s battle area"));
    PermanentHandle {
        player,
        index: index as u8,
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// SECTION 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn bt23_059_yaml_compiles() {
    let r = runner();
    assert!(
        r.compiled_card("BT23-059").is_some(),
        "BT23-059 must be present in the embedded DSL pack"
    );
}

#[test]
fn bt23_059_on_field_has_blocker() {
    let mut runner = runner();
    let handle = runner.place_on_field(0, "BT23-059", None);
    runner.game.tick_declarative_effects();

    assert!(runner.game.has_keyword(handle, Keyword::Blocker));
}

/// Clause A must fire on three timings: OnPlay, WhenDigivolving, WhenAttacking.
#[test]
fn bt23_059_clause_a_fires_on_three_timings() {
    let r = runner();
    let card = r.compiled_card("BT23-059").expect("compiles");

    let clause_a: Vec<&CompiledTriggeredClause> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .filter(|t| {
            t.when.contains(&CompiledTiming::OnPlay)
                || t.when.contains(&CompiledTiming::WhenDigivolving)
                || t.when.contains(&CompiledTiming::WhenAttacking)
        })
        .collect();

    assert!(
        !clause_a.is_empty(),
        "BT23-059 must have a triggered clause covering On Play / When Digivolving / When Attacking"
    );
    let clause = clause_a[0];
    assert!(clause.once_per_turn, "clause A has [Once Per Turn]");
    assert!(
        clause.when.contains(&CompiledTiming::OnPlay),
        "must include OnPlay"
    );
    assert!(
        clause.when.contains(&CompiledTiming::WhenDigivolving),
        "must include WhenDigivolving"
    );
    assert!(
        clause.when.contains(&CompiledTiming::WhenAttacking),
        "must include WhenAttacking"
    );
    assert_eq!(clause.scope, CompiledScope::FaceUp, "face-up scope");
}

// ═══════════════════════════════════════════════════════════════════════════
// SECTION 2 — Clause A: cost selection (option trash prompt)
// ═══════════════════════════════════════════════════════════════════════════

/// Clause A: when an Option is in the battle area, firing On Play installs an
/// outer optional-trigger prompt. Accepting it installs the option-pick selection.
#[test]
fn bt23_059_clause_a_on_play_installs_option_pick_when_option_exists() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT23-059")
        .expect("BT23-059 compiles")
        .add_card(make_option_card("OWN-OPT"))
        .add_card(make_digimon("OPP-DIG", 3))
        .memory(10)
        .start();

    let handle = r.place_on_field(0, "BT23-059", Some(0));
    r.place_on_field(0, "OWN-OPT", Some(0));
    r.place_on_field(1, "OPP-DIG", Some(0));

    fire_timing(&mut r, EffectTiming::OnPlay, handle);

    // The outer optional trigger prompt installs (SelectionKind::Replacement,
    // accept=REPLACEMENT_ACCEPT, decline=PASS).
    assert!(
        r.game.pending_selection.is_some(),
        "clause A must install an outer optional-trigger prompt when condition passes"
    );
    let v = r.pending_selection_view().expect("outer optional prompt");
    assert_eq!(
        v.kind,
        SelectionKind::Replacement,
        "outer optional-trigger prompt must be SelectionKind::Replacement"
    );
    assert!(
        v.valid_action_ids.contains(&REPLACEMENT_ACCEPT),
        "accept action must be available"
    );

    // Accepting installs the inner option-pick selection.
    r.accept_optional_trigger()
        .expect("accept outer optional prompt");

    assert!(
        r.game.pending_selection.is_some(),
        "after accepting, option-pick selection must install"
    );
}

/// Clause A: no option in own battle area → clause A silently no-ops (optional/gated).
#[test]
fn bt23_059_clause_a_on_play_no_selection_when_no_own_option() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT23-059")
        .expect("BT23-059 compiles")
        .add_card(make_digimon("OPP-DIG", 3))
        .memory(10)
        .start();

    let handle = r.place_on_field(0, "BT23-059", Some(0));
    r.place_on_field(1, "OPP-DIG", Some(0));

    fire_timing(&mut r, EffectTiming::OnPlay, handle);

    assert!(
        r.game.pending_selection.is_none(),
        "no own Option in battle area → clause A must not install a selection"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// SECTION 3 — Clause A: effect (lowest-play-cost delete)
// ═══════════════════════════════════════════════════════════════════════════

/// Clause A full flow: outer optional accept, trash the own option, then select
/// the lowest-play-cost opponent Digimon. Only the cheapest candidate is offered.
#[test]
fn bt23_059_clause_a_on_play_offers_only_lowest_cost_opp_digimon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT23-059")
        .expect("BT23-059 compiles")
        .add_card(make_option_card("OWN-OPT"))
        .add_card(make_digimon("CHEAP", 2))
        .add_card(make_digimon("PRICEY", 8))
        .memory(10)
        .start();

    let handle = r.place_on_field(0, "BT23-059", Some(0));
    let opt_handle = r.place_on_field(0, "OWN-OPT", Some(0));
    let cheap = r.place_on_field(1, "CHEAP", Some(0));
    r.place_on_field(1, "PRICEY", Some(0));

    fire_timing(&mut r, EffectTiming::OnPlay, handle);

    // Outer optional prompt installs first (clause is optional: true).
    r.accept_optional_trigger()
        .expect("accept outer optional-trigger prompt");

    // Step 1: pick the option to trash.
    let v = r
        .pending_selection_view()
        .expect("option-pick selection must be installed after accepting");
    let opt_action = encode_attack(0, opt_handle.index as u16);
    assert!(
        v.valid_action_ids.contains(&opt_action),
        "the own Option card must be selectable as the cost; valid={:?}",
        v.valid_action_ids
    );
    r.game
        .resolve_selection(v.selecting_player, opt_action)
        .expect("pick option");

    // Step 2: lowest-play-cost delete. Only CHEAP (cost 2) offered, not PRICEY (8).
    let v = r
        .pending_selection_view()
        .expect("lowest-cost delete selection must follow after option is trashed");
    let cheap_action = encode_attack(0, cheap.index as u16);
    let pricey_action = encode_attack(0, 1_u16); // PRICEY is at index 1 on opp side
    assert!(
        v.valid_action_ids.contains(&cheap_action),
        "CHEAP (cost 2, lowest) must be selectable; valid={:?}",
        v.valid_action_ids
    );
    assert!(
        !v.valid_action_ids.contains(&pricey_action),
        "PRICEY (cost 8) must NOT be selectable (not the lowest); valid={:?}",
        v.valid_action_ids
    );

    r.game
        .resolve_selection(v.selecting_player, cheap_action)
        .expect("delete CHEAP");

    // CHEAP must be gone.
    assert!(
        !r.game
            .player(1)
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&r.game.card_data) == "CHEAP"),
        "CHEAP must be deleted"
    );
    // Own option must be in trash.
    assert!(
        r.game
            .player(0)
            .trash
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "OWN-OPT"),
        "OWN-OPT must be in controller's trash after being trashed as the cost"
    );
}

/// When all opponent Digimon have the same play cost, all are offered.
#[test]
fn bt23_059_clause_a_on_play_offers_all_tied_lowest_cost_digimon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT23-059")
        .expect("BT23-059 compiles")
        .add_card(make_option_card("OWN-OPT"))
        .add_card(make_digimon("OPP-A", 3))
        .add_card(make_digimon("OPP-B", 3))
        .memory(10)
        .start();

    let handle = r.place_on_field(0, "BT23-059", Some(0));
    let opt_handle = r.place_on_field(0, "OWN-OPT", Some(0));
    let opp_a = r.place_on_field(1, "OPP-A", Some(0));
    let opp_b = r.place_on_field(1, "OPP-B", Some(0));

    fire_timing(&mut r, EffectTiming::OnPlay, handle);

    // Accept the outer optional trigger prompt.
    r.accept_optional_trigger()
        .expect("accept outer optional-trigger prompt");

    // Resolve option trash step.
    let v = r
        .pending_selection_view()
        .expect("option-pick selection after accepting");
    r.game
        .resolve_selection(
            v.selecting_player,
            encode_attack(0, opt_handle.index as u16),
        )
        .expect("trash option");

    // Both tied-cheapest Digimon must be offered.
    let v = r
        .pending_selection_view()
        .expect("lowest-cost delete select");
    let a_action = encode_attack(0, opp_a.index as u16);
    let b_action = encode_attack(0, opp_b.index as u16);
    assert!(
        v.valid_action_ids.contains(&a_action),
        "OPP-A (tied cheapest) must be selectable"
    );
    assert!(
        v.valid_action_ids.contains(&b_action),
        "OPP-B (tied cheapest) must be selectable"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// SECTION 4 — Clause A: multi-timing coverage
// ═══════════════════════════════════════════════════════════════════════════

/// Clause A fires on WhenDigivolving: outer optional prompt installs.
#[test]
fn bt23_059_clause_a_fires_on_when_digivolving() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT23-059")
        .expect("BT23-059 compiles")
        .add_card(make_option_card("OWN-OPT"))
        .add_card(make_digimon("OPP-DIG", 3))
        .memory(10)
        .start();

    let handle = r.place_on_field(0, "BT23-059", Some(0));
    r.place_on_field(0, "OWN-OPT", Some(0));
    r.place_on_field(1, "OPP-DIG", Some(0));

    fire_timing(&mut r, EffectTiming::WhenDigivolving, handle);

    assert!(
        r.game.pending_selection.is_some(),
        "clause A must fire on WhenDigivolving — outer optional prompt must install"
    );
    assert_eq!(
        r.pending_kind(),
        Some(SelectionKind::Replacement),
        "the outer prompt is SelectionKind::Replacement (optional trigger accept/decline)"
    );
}

/// Clause A fires on WhenAttacking: outer optional prompt installs.
#[test]
fn bt23_059_clause_a_fires_on_when_attacking() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT23-059")
        .expect("BT23-059 compiles")
        .add_card(make_option_card("OWN-OPT"))
        .add_card(make_digimon("OPP-DIG", 3))
        .memory(10)
        .start();

    let handle = r.place_on_field(0, "BT23-059", Some(0));
    r.place_on_field(0, "OWN-OPT", Some(0));
    r.place_on_field(1, "OPP-DIG", Some(0));

    fire_timing(&mut r, EffectTiming::WhenAttacking, handle);

    assert!(
        r.game.pending_selection.is_some(),
        "clause A must fire on WhenAttacking — outer optional prompt must install"
    );
    assert_eq!(
        r.pending_kind(),
        Some(SelectionKind::Replacement),
        "the outer prompt is SelectionKind::Replacement (optional trigger accept/decline)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// SECTION 5 — Clause A: OPT lockout
// ═══════════════════════════════════════════════════════════════════════════

/// Clause A is [Once Per Turn]: a second fire in the same turn must not
/// install another selection.
#[test]
fn bt23_059_clause_a_opt_blocks_second_activation_same_turn() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT23-059")
        .expect("BT23-059 compiles")
        .add_card(make_option_card("OWN-OPT-1"))
        .add_card(make_option_card("OWN-OPT-2"))
        .add_card(make_digimon("OPP-DIG-1", 3))
        .add_card(make_digimon("OPP-DIG-2", 3))
        .memory(10)
        .start();

    let handle = r.place_on_field(0, "BT23-059", Some(0));
    r.place_on_field(0, "OWN-OPT-1", Some(0));
    r.place_on_field(0, "OWN-OPT-2", Some(0));
    r.place_on_field(1, "OPP-DIG-1", Some(0));
    r.place_on_field(1, "OPP-DIG-2", Some(0));

    // First activation on OnPlay: accept the outer optional prompt, then auto-resolve.
    fire_timing(&mut r, EffectTiming::OnPlay, handle);
    r.accept_optional_trigger()
        .expect("accept outer optional-trigger prompt for first activation");
    r.auto_resolve()
        .expect("auto-resolve first activation body");

    // Second activation on WhenDigivolving same turn — OPT must block.
    fire_timing(&mut r, EffectTiming::WhenDigivolving, handle);
    assert!(
        r.game.pending_selection.is_none(),
        "OPT lockout must prevent second activation in the same turn"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// SECTION 6 — Clause B: on_option_trashed unsuspend + immunity
// ═══════════════════════════════════════════════════════════════════════════
//
// Printed text: "[All Turns] [Once Per Turn] When Option cards in the battle
// area are trashed, this Digimon unsuspends. Then, your opponent's Digimon's
// effects don't affect this Digimon for the turn."
//
// Uses the DSL `when: on_option_trashed` timing, which lowers to the engine's
// existing `EffectTiming::OnOptionTrashed` dispatch from `trash_field_option`.

#[test]
fn bt23_059_clause_b_unsuspend_on_option_trashed() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT23-059")
        .expect("BT23-059 compiles")
        .add_card(make_option_card("TRASHED-OPT"))
        .memory(10)
        .start();

    let blitz = r.place_on_field(0, "BT23-059", Some(0));
    r.place_on_field(0, "TRASHED-OPT", Some(0));
    r.game.players[0].battle_area[blitz.index as usize].is_suspended = true;

    let option = field_handle_by_card_id(&r, 0, "TRASHED-OPT");
    assert!(r.game.trash_field_option(option, OptionTrashCause::Effect));

    assert!(
        !r.game.players[0].battle_area[blitz.index as usize].is_suspended,
        "BT23-059 must unsuspend when a battle-area Option is trashed"
    );
}

#[test]
fn bt23_059_clause_b_opt_blocks_second_option_trash_same_turn() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT23-059")
        .expect("BT23-059 compiles")
        .add_card(make_option_card("TRASHED-OPT-1"))
        .add_card(make_option_card("TRASHED-OPT-2"))
        .memory(10)
        .start();

    let blitz = r.place_on_field(0, "BT23-059", Some(0));
    r.place_on_field(0, "TRASHED-OPT-1", Some(0));
    r.place_on_field(0, "TRASHED-OPT-2", Some(0));
    r.game.players[0].battle_area[blitz.index as usize].is_suspended = true;

    let first = field_handle_by_card_id(&r, 0, "TRASHED-OPT-1");
    assert!(r.game.trash_field_option(first, OptionTrashCause::Effect));
    assert!(
        !r.game.players[0].battle_area[blitz.index as usize].is_suspended,
        "first Option trash should unsuspend BT23-059"
    );

    r.game.players[0].battle_area[blitz.index as usize].is_suspended = true;
    let second = field_handle_by_card_id(&r, 0, "TRASHED-OPT-2");
    assert!(r.game.trash_field_option(second, OptionTrashCause::Effect));

    assert!(
        r.game.players[0].battle_area[blitz.index as usize].is_suspended,
        "OPT lockout must prevent a second unsuspend in the same turn"
    );
}

#[test]
fn bt23_059_clause_b_grants_effect_immunity_from_opponent_digimon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT23-059")
        .expect("BT23-059 compiles")
        .add_card(make_option_card("TRASHED-OPT"))
        .memory(10)
        .start();

    let blitz = r.place_on_field(0, "BT23-059", Some(0));
    r.place_on_field(0, "TRASHED-OPT", Some(0));

    let option = field_handle_by_card_id(&r, 0, "TRASHED-OPT");
    assert!(r.game.trash_field_option(option, OptionTrashCause::Effect));

    assert!(
        r.game
            .permanent_is_unaffected_by_effect(blitz, 1, EffectSourceKind::Digimon),
        "BT23-059 must be unaffected by opponent Digimon effects for the turn"
    );
    assert!(
        !r.game
            .permanent_is_unaffected_by_effect(blitz, 1, EffectSourceKind::Option),
        "BT23-059 should not gain immunity from opponent Option effects"
    );
}
