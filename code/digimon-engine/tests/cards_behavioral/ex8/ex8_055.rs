//! EX8-055 Pyramidimon — Digimon, Black, Level 6, Cost 12, 12000 DP.
//! Traits: [Mineral, LIBERATOR]
//!
//! # Card text (EX8-055.json / data/cards.json — verbatim)
//!
//! ＜Fragment (3)＞
//!
//! [When Digivolving] [When Attacking]
//! By trashing any 3 digivolution cards with the [Mineral] or [Rock] trait
//! from your Digimon, this Digimon unsuspends and gains ＜Security A. +1＞
//! for the turn.
//!
//! [End of Your Turn] [Once Per Turn]
//! You may place up to 3 cards with the [Mineral] or [Rock] trait from your
//! trash as this Digimon's bottom digivolution cards.
//!
//! # Patterns this test covers
//!
//! | Clause | Pattern | Section |
//! |--------|---------|---------|
//! | Fragment(3) keyword | declarative grant_keyword | § 1 |
//! | Clause A structural: [WD][WA] FaceUp, not OPT | compiled struct | § 2 |
//! | Clause A positive: SourceMulti(3,3) with ≥3 Mineral/Rock sources | select_own_sources | § 3 |
//! | Clause A positive: Rock-trait sources accepted | filter any_of | § 3 |
//! | Clause A negative: silent-skip when <3 Mineral/Rock sources | silent-skip | § 3 |
//! | Clause A negative: plain sources excluded | filter | § 3 |
//! | Clause A effect: this Digimon unsuspends | unsuspend target:source | § 4 |
//! | Clause A effect: SecurityAttackChange +1 for the turn | add_modifier end_of_turn | § 5 |
//! | Clause A effect: SecurityAttackChange expires after turn | expiry | § 5 |
//! | Clause A effect: 3 sources trashed from stack | trash_selected_sources | § 6 |
//! | Clause A multi-timing: WhenAttacking also fires | enqueue WhenAttacking | § 7 |
//! | Clause B structural: [EOT][OPT][OncPerTurn] FaceUp | compiled struct | § 8 |
//! | Clause B positive: EOT fires outer Replacement + SelectTrash | select_trash | § 9 |
//! | Clause B positive: 1 card placed as bottom source | place_as_bottom_source | § 9 |
//! | Clause B positive: PASS on first pick → no placement | optional | § 9 |
//! | Clause B positive: up to 3 Rock cards placed | place_as_bottom_source ×3 | § 9 |
//! | Clause B negative: plain-trait trash excluded from selection | filter | § 9 |
//! | Clause B OPT lockout: second EOT same turn no prompt | once_per_turn | § 10 |

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::action::space::PASS;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{EffectTiming, Keyword, ModifierType};
use digimon_engine::selection::{SelectionKind, TriggerSource};

// ─── Helper factories ─────────────────────────────────────────────────────────

fn make_mineral_source(id: &str) -> digimon_engine::card_data::CardData {
    let mut c = make_test_card(id, &format!("{id} Mineral Src"));
    c.traits.push("Mineral".to_string());
    c.level = Some(3);
    c
}

fn make_rock_source(id: &str) -> digimon_engine::card_data::CardData {
    let mut c = make_test_card(id, &format!("{id} Rock Src"));
    c.traits.push("Rock".to_string());
    c.level = Some(3);
    c
}

fn make_plain_source(id: &str) -> digimon_engine::card_data::CardData {
    let mut c = make_test_card(id, &format!("{id} Plain Src"));
    c.level = Some(3);
    c
}

/// Build a carrier Digimon (level 5) that can hold digivolution sources.
fn make_carrier(id: &str) -> digimon_engine::card_data::CardData {
    let mut c = make_test_card(id, &format!("{id} Carrier"));
    c.level = Some(5);
    c.dp = Some(7000);
    c
}

/// Push a card into a player's trash.
fn push_to_trash(runner: &mut DebugRunner, player: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .expect("card data registered");
    let card = CardSource::new(data_idx, player, runner.game.next_card_index());
    runner.game.players[player as usize].trash.push(card);
}

fn pyramidimon_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX8-055")
        .expect("EX8-055 YAML parses and compiles")
        .build()
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Fragment(3) keyword (existing test retained)
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn ex8_055_on_field_has_fragment_three() {
    let mut runner = pyramidimon_runner();
    let handle = runner.place_on_field(0, "EX8-055", None);
    runner.game.tick_declarative_effects();

    assert!(runner.game.has_keyword(handle, Keyword::Fragment(3)));
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Structural: Clause A compiled shape
// ═══════════════════════════════════════════════════════════════════════════════

/// EX8-055 must compile and be present in the embedded DSL pack.
#[test]
fn ex8_055_yaml_compiles_without_error() {
    let runner = pyramidimon_runner();
    assert!(
        runner.compiled_card("EX8-055").is_some(),
        "EX8-055 must be present in the embedded DSL pack (YAML must parse + compile)"
    );
}

/// Clause A must be a [WhenDigivolving, WhenAttacking] FaceUp clause, not OPT.
#[test]
fn ex8_055_clause_a_has_wd_wa_not_opt() {
    let runner = pyramidimon_runner();
    let card = runner
        .compiled_card("EX8-055")
        .expect("EX8-055 compiled card present");

    let has_clause_a = card.effects.iter().any(|clause| {
        matches!(
            clause,
            CompiledClause::Triggered(t)
                if t.scope == CompiledScope::FaceUp
                    && t.when.contains(&CompiledTiming::WhenDigivolving)
                    && t.when.contains(&CompiledTiming::WhenAttacking)
                    && !t.once_per_turn
                    && !t.optional
        )
    });

    assert!(
        has_clause_a,
        "EX8-055 must have a [WD][WA] FaceUp non-optional clause (Clause A)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Clause A: source-trash cost (positive and negative)
// ═══════════════════════════════════════════════════════════════════════════════

/// Positive: with 3 Mineral sources on a carrier Digimon, Clause A prompts
/// SourceMulti(3,3) when WhenDigivolving is enqueued at EX8-055.
#[test]
fn ex8_055_clause_a_when_digivolving_prompts_source_multi_3_of_3() {
    let src_a = make_mineral_source("MA-SRC1");
    let src_b = make_mineral_source("MA-SRC2");
    let src_c = make_mineral_source("MA-SRC3");
    let carrier = make_carrier("MA-CARRIER");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-055")
        .expect("EX8-055 YAML parses and compiles")
        .add_card(src_a)
        .add_card(src_b)
        .add_card(src_c)
        .add_card(carrier)
        .memory(10)
        .start();

    let carrier_h = runner.place_on_field(0, "MA-CARRIER", None);
    runner.push_source(carrier_h, "MA-SRC1");
    runner.push_source(carrier_h, "MA-SRC2");
    runner.push_source(carrier_h, "MA-SRC3");

    let pyra = runner.place_on_field(0, "EX8-055", None);

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(pyra),
    );
    runner.game.drain_effect_queue();

    assert!(
        matches!(
            runner.pending_kind(),
            Some(SelectionKind::SourceMulti { min: 3, max: 3, .. })
        ),
        "Clause A must prompt SourceMulti(3,3) when WhenDigivolving fires and 3 Mineral sources are available; got {:?}",
        runner.pending_kind()
    );
}

/// Positive: Rock-trait sources also satisfy the Clause A cost.
#[test]
fn ex8_055_clause_a_rock_sources_are_valid_cost() {
    let src_a = make_rock_source("RA-SRC1");
    let src_b = make_rock_source("RA-SRC2");
    let src_c = make_rock_source("RA-SRC3");
    let carrier = make_carrier("RA-CARRIER");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-055")
        .expect("EX8-055 YAML parses and compiles")
        .add_card(src_a)
        .add_card(src_b)
        .add_card(src_c)
        .add_card(carrier)
        .memory(10)
        .start();

    let carrier_h = runner.place_on_field(0, "RA-CARRIER", None);
    runner.push_source(carrier_h, "RA-SRC1");
    runner.push_source(carrier_h, "RA-SRC2");
    runner.push_source(carrier_h, "RA-SRC3");

    let pyra = runner.place_on_field(0, "EX8-055", None);

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(pyra),
    );
    runner.game.drain_effect_queue();

    assert!(
        matches!(
            runner.pending_kind(),
            Some(SelectionKind::SourceMulti { min: 3, max: 3, .. })
        ),
        "Clause A must accept Rock-trait sources as valid cost; got {:?}",
        runner.pending_kind()
    );
}

/// Negative: with only 2 Mineral/Rock sources, Clause A silently skips —
/// no effect fires and EX8-055 stays suspended.
#[test]
fn ex8_055_clause_a_silently_skips_when_fewer_than_3_sources() {
    let src_a = make_mineral_source("FEW-SRC1");
    let src_b = make_mineral_source("FEW-SRC2");
    let carrier = make_carrier("FEW-CARRIER");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-055")
        .expect("EX8-055 YAML parses and compiles")
        .add_card(src_a)
        .add_card(src_b)
        .add_card(carrier)
        .memory(10)
        .start();

    let carrier_h = runner.place_on_field(0, "FEW-CARRIER", None);
    runner.push_source(carrier_h, "FEW-SRC1");
    runner.push_source(carrier_h, "FEW-SRC2");

    let pyra = runner.place_on_field(0, "EX8-055", None);
    runner.game.players[0].battle_area[pyra.index as usize].is_suspended = true;

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(pyra),
    );
    runner.game.drain_effect_queue();

    // Drain any partial SourceMulti prompts that may appear.
    for _ in 0..4 {
        if runner.pending_selection().is_some() {
            runner.auto_resolve().ok();
        }
    }

    // EX8-055 must still be suspended (effect never fired).
    assert!(
        runner.game.players[0].battle_area[pyra.index as usize].is_suspended,
        "Clause A must not unsuspend EX8-055 when fewer than 3 Mineral/Rock sources exist"
    );
    assert_eq!(
        runner
            .game
            .modifiers
            .sum(pyra, ModifierType::SecurityAttackChange),
        0,
        "Clause A must not grant SecurityAttackChange when cost cannot be paid"
    );
}

/// Negative: plain sources (no Mineral/Rock trait) do not count toward Clause A cost.
#[test]
fn ex8_055_clause_a_plain_sources_do_not_count() {
    let src_a = make_plain_source("PL-SRC1");
    let src_b = make_plain_source("PL-SRC2");
    let src_c = make_plain_source("PL-SRC3");
    let carrier = make_carrier("PL-CARRIER");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-055")
        .expect("EX8-055 YAML parses and compiles")
        .add_card(src_a)
        .add_card(src_b)
        .add_card(src_c)
        .add_card(carrier)
        .memory(10)
        .start();

    let carrier_h = runner.place_on_field(0, "PL-CARRIER", None);
    runner.push_source(carrier_h, "PL-SRC1");
    runner.push_source(carrier_h, "PL-SRC2");
    runner.push_source(carrier_h, "PL-SRC3");

    let pyra = runner.place_on_field(0, "EX8-055", None);
    runner.game.players[0].battle_area[pyra.index as usize].is_suspended = true;

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(pyra),
    );
    runner.game.drain_effect_queue();

    for _ in 0..4 {
        if runner.pending_selection().is_some() {
            runner.auto_resolve().ok();
        }
    }

    assert!(
        runner.game.players[0].battle_area[pyra.index as usize].is_suspended,
        "Plain sources must not satisfy Clause A cost — EX8-055 must remain suspended"
    );
    assert_eq!(
        runner
            .game
            .modifiers
            .sum(pyra, ModifierType::SecurityAttackChange),
        0,
        "Plain sources must not grant SecurityAttackChange"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — Clause A effect: unsuspend this Digimon
// ═══════════════════════════════════════════════════════════════════════════════

/// After paying the 3-source cost, EX8-055 unsuspends.
#[test]
fn ex8_055_clause_a_unsuspends_this_digimon_after_cost() {
    let src_a = make_mineral_source("US-SRC1");
    let src_b = make_mineral_source("US-SRC2");
    let src_c = make_mineral_source("US-SRC3");
    let carrier = make_carrier("US-CARRIER");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-055")
        .expect("EX8-055 YAML parses and compiles")
        .add_card(src_a)
        .add_card(src_b)
        .add_card(src_c)
        .add_card(carrier)
        .memory(10)
        .start();

    let carrier_h = runner.place_on_field(0, "US-CARRIER", None);
    runner.push_source(carrier_h, "US-SRC1");
    runner.push_source(carrier_h, "US-SRC2");
    runner.push_source(carrier_h, "US-SRC3");

    let pyra = runner.place_on_field(0, "EX8-055", None);
    runner.game.players[0].battle_area[pyra.index as usize].is_suspended = true;

    assert!(
        runner.game.players[0].battle_area[pyra.index as usize].is_suspended,
        "EX8-055 must be suspended before Clause A fires"
    );

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(pyra),
    );
    runner.game.drain_effect_queue();

    // Pay the cost: auto-resolve picks all 3 sources.
    runner.auto_resolve().expect("source multi resolves");

    assert!(
        !runner.game.players[0].battle_area[pyra.index as usize].is_suspended,
        "EX8-055 must be unsuspended after Clause A cost is paid"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 5 — Clause A effect: SecurityAttackChange +1 for the turn
// ═══════════════════════════════════════════════════════════════════════════════

/// After paying the 3-source cost, EX8-055 gains SecurityAttackChange +1.
#[test]
fn ex8_055_clause_a_grants_security_attack_plus_1() {
    let src_a = make_mineral_source("SA-SRC1");
    let src_b = make_mineral_source("SA-SRC2");
    let src_c = make_mineral_source("SA-SRC3");
    let carrier = make_carrier("SA-CARRIER");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-055")
        .expect("EX8-055 YAML parses and compiles")
        .add_card(src_a)
        .add_card(src_b)
        .add_card(src_c)
        .add_card(carrier)
        .memory(10)
        .start();

    let carrier_h = runner.place_on_field(0, "SA-CARRIER", None);
    runner.push_source(carrier_h, "SA-SRC1");
    runner.push_source(carrier_h, "SA-SRC2");
    runner.push_source(carrier_h, "SA-SRC3");

    let pyra = runner.place_on_field(0, "EX8-055", None);

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(pyra),
    );
    runner.game.drain_effect_queue();

    runner.auto_resolve().expect("source multi resolves");

    assert_eq!(
        runner
            .game
            .modifiers
            .sum(pyra, ModifierType::SecurityAttackChange),
        1,
        "EX8-055 must gain SecurityAttackChange +1 after paying the 3-source cost"
    );
}

/// SecurityAttackChange expires at the end of the turn it was granted (expiry: end_of_turn).
#[test]
fn ex8_055_clause_a_security_attack_change_expires_after_turn() {
    let src_a = make_mineral_source("EXP-SRC1");
    let src_b = make_mineral_source("EXP-SRC2");
    let src_c = make_mineral_source("EXP-SRC3");
    let carrier = make_carrier("EXP-CARRIER");
    let p1_filler = make_test_card("EXP-PAD", "Filler P1");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-055")
        .expect("EX8-055 YAML parses and compiles")
        .add_card(src_a)
        .add_card(src_b)
        .add_card(src_c)
        .add_card(carrier)
        .add_card(p1_filler)
        .deck(1, &["EXP-PAD", "EXP-PAD", "EXP-PAD"])
        .memory(10)
        .start();

    let carrier_h = runner.place_on_field(0, "EXP-CARRIER", None);
    runner.push_source(carrier_h, "EXP-SRC1");
    runner.push_source(carrier_h, "EXP-SRC2");
    runner.push_source(carrier_h, "EXP-SRC3");

    let pyra = runner.place_on_field(0, "EX8-055", None);

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(pyra),
    );
    runner.game.drain_effect_queue();

    runner.auto_resolve().expect("source multi resolves");

    assert_eq!(
        runner
            .game
            .modifiers
            .sum(pyra, ModifierType::SecurityAttackChange),
        1,
        "SecurityAttackChange must be +1 immediately after cost payment"
    );

    // End player 0's turn. Pyramidimon's EOT source-placement clause can
    // park an optional prompt before turn rotation; decline it so cleanup
    // reaches the modifier expiry pass.
    runner.end_turn();
    while runner.game.pending_selection.is_some() {
        runner
            .game
            .resolve_selection(0, digimon_engine::action::space::PASS)
            .expect("decline optional EOT source placement");
    }

    assert_eq!(
        runner
            .game
            .modifiers
            .sum(pyra, ModifierType::SecurityAttackChange),
        0,
        "SecurityAttackChange must expire at the end of the turn it was granted"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 6 — Clause A effect: 3 sources trashed from stack
// ═══════════════════════════════════════════════════════════════════════════════

/// After paying the cost, 3 Mineral sources are removed from the carrier's
/// stack and land in trash.
#[test]
fn ex8_055_clause_a_trashes_3_sources_from_carrier() {
    let src_a = make_mineral_source("TR-SRC1");
    let src_b = make_mineral_source("TR-SRC2");
    let src_c = make_mineral_source("TR-SRC3");
    let carrier = make_carrier("TR-CARRIER");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-055")
        .expect("EX8-055 YAML parses and compiles")
        .add_card(src_a)
        .add_card(src_b)
        .add_card(src_c)
        .add_card(carrier)
        .memory(10)
        .start();

    let carrier_h = runner.place_on_field(0, "TR-CARRIER", None);
    runner.push_source(carrier_h, "TR-SRC1");
    runner.push_source(carrier_h, "TR-SRC2");
    runner.push_source(carrier_h, "TR-SRC3");

    let sources_before = runner.game.players[0].battle_area[carrier_h.index as usize]
        .card_sources
        .len();
    // carrier has 1 card (itself) + 3 pushed mineral sources = 4 total
    assert_eq!(
        sources_before, 4,
        "carrier must start with 4 sources (1 top card + 3 mineral)"
    );

    let trash_before = runner.trash_size(0);

    let pyra = runner.place_on_field(0, "EX8-055", None);

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(pyra),
    );
    runner.game.drain_effect_queue();

    runner.auto_resolve().expect("source multi resolves");

    let sources_after = runner.game.players[0].battle_area[carrier_h.index as usize]
        .card_sources
        .len();
    let trash_after = runner.trash_size(0);

    assert_eq!(
        sources_after, 1,
        "only the carrier's own top card must remain after trashing the 3 Mineral sources"
    );
    assert!(
        trash_after >= trash_before + 3,
        "3 sources must land in trash (before: {trash_before}, after: {trash_after})"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 7 — Clause A multi-timing: WhenAttacking also fires
// ═══════════════════════════════════════════════════════════════════════════════

/// Clause A fires on WhenAttacking timing with the same unsuspend + SecurityA+1 effect.
#[test]
fn ex8_055_clause_a_when_attacking_also_fires() {
    let src_a = make_mineral_source("WA-SRC1");
    let src_b = make_mineral_source("WA-SRC2");
    let src_c = make_mineral_source("WA-SRC3");
    let carrier = make_carrier("WA-CARRIER");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-055")
        .expect("EX8-055 YAML parses and compiles")
        .add_card(src_a)
        .add_card(src_b)
        .add_card(src_c)
        .add_card(carrier)
        .memory(10)
        .start();

    let carrier_h = runner.place_on_field(0, "WA-CARRIER", None);
    runner.push_source(carrier_h, "WA-SRC1");
    runner.push_source(carrier_h, "WA-SRC2");
    runner.push_source(carrier_h, "WA-SRC3");

    let pyra = runner.place_on_field(0, "EX8-055", None);
    runner.game.players[0].battle_area[pyra.index as usize].is_suspended = true;

    runner
        .game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(pyra));
    runner.game.drain_effect_queue();

    assert!(
        matches!(
            runner.pending_kind(),
            Some(SelectionKind::SourceMulti { min: 3, max: 3, .. })
        ),
        "Clause A must also fire on WhenAttacking timing; got {:?}",
        runner.pending_kind()
    );

    runner.auto_resolve().expect("source multi resolves");

    assert!(
        !runner.game.players[0].battle_area[pyra.index as usize].is_suspended,
        "EX8-055 must unsuspend when Clause A fires via WhenAttacking"
    );
    assert_eq!(
        runner
            .game
            .modifiers
            .sum(pyra, ModifierType::SecurityAttackChange),
        1,
        "SecurityAttackChange +1 must be granted when Clause A fires via WhenAttacking"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 8 — Structural: Clause B compiled shape
// ═══════════════════════════════════════════════════════════════════════════════

/// Clause B must be a [EndOfYourTurn] once_per_turn optional FaceUp clause.
#[test]
fn ex8_055_clause_b_has_eot_opt_once_per_turn_shape() {
    let runner = pyramidimon_runner();
    let card = runner
        .compiled_card("EX8-055")
        .expect("EX8-055 compiled card present");

    let has_clause_b = card.effects.iter().any(|clause| {
        matches!(
            clause,
            CompiledClause::Triggered(t)
                if t.scope == CompiledScope::FaceUp
                    && t.when.contains(&CompiledTiming::EndOfYourTurn)
                    && t.once_per_turn
                    && t.optional
        )
    });

    assert!(
        has_clause_b,
        "EX8-055 must have a [EOT][OPT][Once Per Turn] FaceUp clause (Clause B)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 9 — Clause B behavioral: trash-to-source placement
// ═══════════════════════════════════════════════════════════════════════════════

/// Positive: with a Mineral card in trash, EOT fires an outer Replacement
/// (accept/decline) then a SelectTrash prompt.
#[test]
fn ex8_055_clause_b_eot_prompts_replacement_then_select_trash() {
    let tc1 = make_mineral_source("EOT-TC1");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-055")
        .expect("EX8-055 YAML parses and compiles")
        .add_card(tc1)
        .memory(10)
        .start();

    let pyra = runner.place_on_field(0, "EX8-055", None);
    push_to_trash(&mut runner, 0, "EOT-TC1");

    runner
        .game
        .enqueue_triggered(EffectTiming::EndOfYourTurn, TriggerSource::Permanent(pyra));
    runner.game.drain_effect_queue();

    // Clause B's first body step is `select_trash { optional: true }`, which is
    // itself a declinable (PASS-able) selection — so no outer Replacement prompt
    // is installed (G-OUTER-OPTIONAL-NOT-INSTALLED). The engine goes directly to
    // the SelectTrash prompt; the player's PASS on that prompt is the decline path.
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Trash),
        "Clause B [EOT][OPT] must install a SelectTrash prompt directly (no outer Replacement); got {:?}",
        runner.pending_kind()
    );
    assert!(
        runner.pending_is_optional(),
        "EOT SelectTrash must be optional — PASS must be a valid action"
    );
}

/// Positive: 1 Mineral card is placed as bottom source after accept + pick.
#[test]
fn ex8_055_clause_b_places_one_mineral_card_as_bottom_source() {
    let tc1 = make_mineral_source("P1-TC1");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-055")
        .expect("EX8-055 YAML parses and compiles")
        .add_card(tc1)
        .memory(10)
        .start();

    let pyra = runner.place_on_field(0, "EX8-055", None);
    push_to_trash(&mut runner, 0, "P1-TC1");

    let trash_before = runner.trash_size(0);
    let sources_before = runner.game.players[0].battle_area[pyra.index as usize]
        .card_sources
        .len();

    runner
        .game
        .enqueue_triggered(EffectTiming::EndOfYourTurn, TriggerSource::Permanent(pyra));
    runner.game.drain_effect_queue();

    // Accept outer optional.
    runner
        .accept_optional_trigger()
        .expect("accept outer optional");
    // Pick 1: the Mineral card from trash.
    runner.auto_resolve().expect("first trash pick resolves");
    // Picks 2 and 3 are optional — drain follow-up (player may pass automatically).
    runner.auto_resolve().ok();
    runner.auto_resolve().ok();

    let trash_after = runner.trash_size(0);
    let sources_after = runner.game.players[0].battle_area[pyra.index as usize]
        .card_sources
        .len();

    assert!(
        trash_after < trash_before,
        "the picked Mineral card must leave trash (before: {trash_before}, after: {trash_after})"
    );
    assert!(
        sources_after > sources_before,
        "EX8-055's source stack must grow by at least 1 \
         (before: {sources_before}, after: {sources_after})"
    );
}

/// Positive: player may PASS the first SelectTrash pick (0 placements).
#[test]
fn ex8_055_clause_b_no_placement_when_player_passes_first_pick() {
    let tc1 = make_mineral_source("PASS-TC1");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-055")
        .expect("EX8-055 YAML parses and compiles")
        .add_card(tc1)
        .memory(10)
        .start();

    let pyra = runner.place_on_field(0, "EX8-055", None);
    push_to_trash(&mut runner, 0, "PASS-TC1");

    let sources_before = runner.game.players[0].battle_area[pyra.index as usize]
        .card_sources
        .len();

    runner
        .game
        .enqueue_triggered(EffectTiming::EndOfYourTurn, TriggerSource::Permanent(pyra));
    runner.game.drain_effect_queue();

    // No outer Replacement prompt — Clause B's body starts with an optional
    // select_trash, which itself is the decline path (PASS-able directly).
    // PASS the first SelectTrash pick (decline all placements).
    let player = runner
        .pending_selection()
        .expect("SelectTrash must be pending after EOT drain")
        .selecting_player;
    assert!(
        runner.pending_is_optional(),
        "first SelectTrash pick must be optional (PASS available)"
    );
    runner
        .game
        .resolve_selection(player, PASS)
        .expect("PASS on first pick resolves");

    let sources_after = runner.game.players[0].battle_area[pyra.index as usize]
        .card_sources
        .len();
    assert_eq!(
        sources_after, sources_before,
        "source stack must not change after PASS on first pick \
         (before: {sources_before}, after: {sources_after})"
    );
}

/// Positive: up to 3 Rock cards placed from trash as bottom sources.
#[test]
fn ex8_055_clause_b_places_up_to_3_rock_cards() {
    let tc1 = make_rock_source("B3-TC1");
    let tc2 = make_rock_source("B3-TC2");
    let tc3 = make_rock_source("B3-TC3");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-055")
        .expect("EX8-055 YAML parses and compiles")
        .add_card(tc1)
        .add_card(tc2)
        .add_card(tc3)
        .memory(10)
        .start();

    let pyra = runner.place_on_field(0, "EX8-055", None);
    push_to_trash(&mut runner, 0, "B3-TC1");
    push_to_trash(&mut runner, 0, "B3-TC2");
    push_to_trash(&mut runner, 0, "B3-TC3");

    let sources_before = runner.game.players[0].battle_area[pyra.index as usize]
        .card_sources
        .len();

    runner
        .game
        .enqueue_triggered(EffectTiming::EndOfYourTurn, TriggerSource::Permanent(pyra));
    runner.game.drain_effect_queue();

    // Accept outer optional.
    runner
        .accept_optional_trigger()
        .expect("accept outer optional");
    // Pick all 3 Rock cards.
    runner.auto_resolve().expect("pick 1 resolves");
    runner.auto_resolve().expect("pick 2 resolves");
    runner.auto_resolve().expect("pick 3 resolves");
    runner.auto_resolve().ok(); // drain any follow-up

    let sources_after = runner.game.players[0].battle_area[pyra.index as usize]
        .card_sources
        .len();

    assert_eq!(
        sources_after,
        sources_before + 3,
        "EX8-055 must gain exactly 3 bottom sources after picking all 3 Rock cards \
         (before: {sources_before}, after: {sources_after})"
    );
    assert_eq!(
        runner.trash_size(0),
        0,
        "all 3 Rock cards must leave trash after placement"
    );
}

/// Negative: plain-trait trash cards are excluded from the SelectTrash filter.
/// With only plain-trait cards in trash, the inner picks silently skip.
#[test]
fn ex8_055_clause_b_plain_trash_cards_excluded() {
    let plain = make_plain_source("BPLAIN-SRC");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-055")
        .expect("EX8-055 YAML parses and compiles")
        .add_card(plain)
        .memory(10)
        .start();

    let pyra = runner.place_on_field(0, "EX8-055", None);
    push_to_trash(&mut runner, 0, "BPLAIN-SRC");

    let sources_before = runner.game.players[0].battle_area[pyra.index as usize]
        .card_sources
        .len();

    runner
        .game
        .enqueue_triggered(EffectTiming::EndOfYourTurn, TriggerSource::Permanent(pyra));
    runner.game.drain_effect_queue();

    // No outer Replacement prompt — the first body step is optional select_trash.
    // With only plain-trait cards in trash (no Mineral/Rock), the filter matches
    // nothing. The optional select_trash with no candidates silently skips,
    // leaving no pending selection.
    runner.auto_resolve().ok();
    runner.auto_resolve().ok();
    runner.auto_resolve().ok();

    let sources_after = runner.game.players[0].battle_area[pyra.index as usize]
        .card_sources
        .len();

    assert_eq!(
        sources_after, sources_before,
        "plain trash cards must not be placed as sources (sources unchanged)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 10 — Clause B OPT lockout
// ═══════════════════════════════════════════════════════════════════════════════

/// [Once Per Turn] lockout: firing EOT a second time on the same turn does not
/// install a second Replacement prompt after the first activation.
#[test]
fn ex8_055_clause_b_once_per_turn_lockout() {
    let tc1 = make_mineral_source("OPT-TC1");
    let tc2 = make_mineral_source("OPT-TC2");
    let p1_pad = make_test_card("OPT-PAD", "OPT Filler");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-055")
        .expect("EX8-055 YAML parses and compiles")
        .add_card(tc1)
        .add_card(tc2)
        .add_card(p1_pad)
        .deck(1, &["OPT-PAD", "OPT-PAD", "OPT-PAD"])
        .memory(10)
        .start();

    let pyra = runner.place_on_field(0, "EX8-055", None);
    push_to_trash(&mut runner, 0, "OPT-TC1");
    push_to_trash(&mut runner, 0, "OPT-TC2");

    // First EOT activation.
    runner
        .game
        .enqueue_triggered(EffectTiming::EndOfYourTurn, TriggerSource::Permanent(pyra));
    runner.game.drain_effect_queue();

    // No outer Replacement prompt — body starts with optional select_trash.
    // auto_resolve picks the first valid trash card (OPT-TC1).
    runner.auto_resolve().expect("first activation: pick 1");
    runner.auto_resolve().ok();
    runner.auto_resolve().ok();

    // Second EOT activation on the same turn — OPT lockout must suppress it.
    runner
        .game
        .enqueue_triggered(EffectTiming::EndOfYourTurn, TriggerSource::Permanent(pyra));
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "second EOT on the same turn must not install a selection (once_per_turn lockout)"
    );

    // After turn rotation, the OPT resets — next turn the clause fires again.
    runner.end_turn(); // → player 1's turn
    runner.end_turn(); // → back to player 0's turn

    push_to_trash(&mut runner, 0, "OPT-TC2");

    runner
        .game
        .enqueue_triggered(EffectTiming::EndOfYourTurn, TriggerSource::Permanent(pyra));
    runner.game.drain_effect_queue();

    // After turn rotation OPT resets. No outer Replacement prompt — body starts
    // with optional select_trash directly (G-OUTER-OPTIONAL-NOT-INSTALLED pattern).
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Trash),
        "after turn rotation, Clause B OPT must reset — SelectTrash must be installed directly"
    );
}
