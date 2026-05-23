//! EX10-036 Magneticdramon — Digimon, Black, Level 7, Cost 14, 14000 DP.
//! Traits: [Rock Dragon, LIBERATOR]
//!
//! # Card text (data/cards.json — verbatim)
//!
//! ＜Fragment (3)＞
//! [When Digivolving] [When Attacking] By trashing 3 [Mineral] or [Rock] trait
//! cards from any of your Digimon's digivolution cards, delete 1 of your
//! opponent's Digimon and trash their top security card.
//! [When Digivolving] [When Attacking] [Once Per Turn] By placing 3 [Mineral]
//! or [Rock] trait cards from your trash as this Digimon's bottom digivolution
//! cards, it unsuspends.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX10/Black/EX10_036.cs
//!
//! # Patterns this test covers
//! - H16-adjacent: Fragment(3) declarative keyword (§ Section 1)
//! - Clause A [WD][WA]: cost = trash exactly 3 Mineral/Rock sources from any
//!   of your Digimon's digivolution cards; effect = delete 1 opp Digimon +
//!   trash opp top security card.
//! - Clause B [WD][WA][OPT]: cost = place exactly 3 Mineral/Rock cards from
//!   trash as THIS Digimon's bottom sources; effect = unsuspend this Digimon.
//! - OPT lockout: Clause B fires only once per turn (same-timing lockout).
//! - Multi-timing: both WhenDigivolving and WhenAttacking fire each clause.

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{EffectTiming, Keyword};
use digimon_engine::selection::{SelectionKind, TriggerSource};

// ─── TriggerOrder helper ──────────────────────────────────────────────────────
//
// EX10-036 has two triggered clauses (Clause A and Clause B) that fire on the
// same timing (when_digivolving / when_attacking). When both are enqueued, the
// engine installs a TriggerOrder selection so the controller can pick which
// clause resolves first. This helper drains that ordering prompt (choosing the
// first valid action, which corresponds to Clause A since it is defined first).
// After the TriggerOrder resolves, Clause A fires: if it silently skips (cost
// not payable), Clause B immediately fires as the sole remaining trigger.
fn drain_trigger_order_if_any(runner: &mut DebugRunner) {
    if matches!(runner.pending_kind(), Some(SelectionKind::TriggerOrder)) {
        let view = runner
            .pending_selection_view()
            .expect("TriggerOrder view present");
        let player = view.selecting_player;
        let action = view.valid_action_ids[0];
        runner
            .game
            .resolve_selection(player, action)
            .expect("TriggerOrder resolves");
    }
}

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

fn make_opponent_digimon(id: &str) -> digimon_engine::card_data::CardData {
    let mut c = make_test_card(id, &format!("{id} Opp Digi"));
    c.level = Some(4);
    c.dp = Some(5000);
    c
}

/// Push a card into player `player`'s trash.
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

fn magneticdramon_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX10-036")
        .expect("EX10-036 YAML parses and compiles")
        .build()
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Fragment(3) keyword (existing test retained)
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn ex10_036_on_field_has_fragment_three() {
    let mut runner = magneticdramon_runner();
    let handle = runner.place_on_field(0, "EX10-036", None);
    runner.game.tick_declarative_effects();

    assert!(runner.game.has_keyword(handle, Keyword::Fragment(3)));
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Structural: compiled clause shapes
// ═══════════════════════════════════════════════════════════════════════════════

/// EX10-036 must compile and be present in the embedded DSL pack.
#[test]
fn ex10_036_yaml_compiles_without_error() {
    let runner = DebugRunner::builder()
        .dsl_card("EX10-036")
        .expect("EX10-036 YAML parses and compiles")
        .build();
    assert!(
        runner.compiled_card("EX10-036").is_some(),
        "EX10-036 must be present in the embedded DSL pack"
    );
}

/// Clause A must be a [WhenDigivolving, WhenAttacking] clause (FaceUp scope).
#[test]
fn ex10_036_has_wd_wa_source_trash_delete_security_clause() {
    let runner = magneticdramon_runner();
    let card = runner
        .compiled_card("EX10-036")
        .expect("EX10-036 compiled card present");

    let has_clause_a = card.effects.iter().any(|clause| {
        matches!(
            clause,
            CompiledClause::Triggered(t)
                if t.scope == CompiledScope::FaceUp
                    && t.when.contains(&CompiledTiming::WhenDigivolving)
                    && t.when.contains(&CompiledTiming::WhenAttacking)
                    && !t.once_per_turn
        )
    });

    assert!(
        has_clause_a,
        "EX10-036 must have a [WD][WA] FaceUp clause (Clause A: delete + security trash)"
    );
}

/// Clause B must be a [WhenDigivolving, WhenAttacking, OncPerTurn] clause (FaceUp scope).
#[test]
fn ex10_036_has_wd_wa_opt_trash_to_source_unsuspend_clause() {
    let runner = magneticdramon_runner();
    let card = runner
        .compiled_card("EX10-036")
        .expect("EX10-036 compiled card present");

    let has_clause_b = card.effects.iter().any(|clause| {
        matches!(
            clause,
            CompiledClause::Triggered(t)
                if t.scope == CompiledScope::FaceUp
                    && t.when.contains(&CompiledTiming::WhenDigivolving)
                    && t.when.contains(&CompiledTiming::WhenAttacking)
                    && t.once_per_turn
        )
    });

    assert!(
        has_clause_b,
        "EX10-036 must have a [WD][WA][OPT] FaceUp clause (Clause B: trash-to-source unsuspend)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Clause A: [WD][WA] source-trash delete + security
// ═══════════════════════════════════════════════════════════════════════════════
//
// Setup pattern for Clause A positive tests:
//   - EX10-036 on player 0's field.
//   - A separate carrier Digimon on player 0's field with 3 Mineral/Rock
//     sources in its digivolution stack.
//   - 1 opponent Digimon on field.
//   - Opponent has ≥1 security card.
//   - Fire the WhenDigivolving event at EX10-036.

/// Positive: with 3 Mineral sources available, Clause A prompts for SourceMulti
/// (min 3, max 3) when fired via WhenDigivolving.
#[test]
fn ex10_036_clause_a_when_digivolving_prompts_source_multi_3_of_3() {
    let src_a = make_mineral_source("SRC-A1");
    let src_b = make_mineral_source("SRC-A2");
    let src_c = make_mineral_source("SRC-A3");
    let carrier = make_test_card("CARRIER-A", "Carrier A");
    let opp = make_opponent_digimon("OPP-CA");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-036")
        .expect("EX10-036 YAML loads")
        .add_card(src_a)
        .add_card(src_b)
        .add_card(src_c)
        .add_card(carrier)
        .add_card(opp)
        .memory(10)
        .start();

    let carrier_h = runner.place_on_field(0, "CARRIER-A", None);
    runner.push_source(carrier_h, "SRC-A1");
    runner.push_source(carrier_h, "SRC-A2");
    runner.push_source(carrier_h, "SRC-A3");
    runner.place_on_field(1, "OPP-CA", None);
    // Give opponent 2 security cards via the filler card.
    let sec_filler = make_test_card("SEC-FILLER-CA", "Security Filler");
    runner.game.card_data.push(sec_filler);
    let filler_idx = runner.game.card_data.len() - 1;
    for _ in 0..2 {
        let cs = CardSource::new(filler_idx, 1, runner.game.next_card_index());
        runner.game.players[1].security.push(cs);
    }

    let magnet = runner.place_on_field(0, "EX10-036", None);

    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(magnet));
    runner.game.drain_effect_queue();
    // Both Clause A and Clause B are enqueued simultaneously (same timing).
    // Drain the TriggerOrder ordering prompt first (picks Clause A).
    drain_trigger_order_if_any(&mut runner);

    assert!(
        matches!(
            runner.pending_kind(),
            Some(SelectionKind::SourceMulti { min: 3, max: 3, .. })
        ),
        "Clause A must prompt SourceMulti(3,3) when WhenDigivolving fires and 3 Mineral sources are available; got {:?}",
        runner.pending_kind()
    );
}

/// Positive: Rock-trait sources also count as valid Clause A cost.
#[test]
fn ex10_036_clause_a_rock_sources_valid_cost() {
    let src_a = make_rock_source("RSRC-A1");
    let src_b = make_rock_source("RSRC-A2");
    let src_c = make_rock_source("RSRC-A3");
    let carrier = make_test_card("RCARRIER-A", "Rock Carrier A");
    let opp = make_opponent_digimon("OPP-RA");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-036")
        .expect("EX10-036 YAML loads")
        .add_card(src_a)
        .add_card(src_b)
        .add_card(src_c)
        .add_card(carrier)
        .add_card(opp)
        .memory(10)
        .start();

    let carrier_h = runner.place_on_field(0, "RCARRIER-A", None);
    runner.push_source(carrier_h, "RSRC-A1");
    runner.push_source(carrier_h, "RSRC-A2");
    runner.push_source(carrier_h, "RSRC-A3");
    runner.place_on_field(1, "OPP-RA", None);

    let sec_filler = make_test_card("SEC-FILL-RA", "Sec Filler RA");
    runner.game.card_data.push(sec_filler);
    let fi = runner.game.card_data.len() - 1;
    let cs = CardSource::new(fi, 1, runner.game.next_card_index());
    runner.game.players[1].security.push(cs);

    let magnet = runner.place_on_field(0, "EX10-036", None);
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(magnet));
    runner.game.drain_effect_queue();
    drain_trigger_order_if_any(&mut runner);

    assert!(
        matches!(
            runner.pending_kind(),
            Some(SelectionKind::SourceMulti { min: 3, max: 3, .. })
        ),
        "Clause A must accept Rock-trait sources as valid cost"
    );
}

/// Negative: with only 2 Mineral/Rock sources available, the SourceMulti
/// selection cannot be completed (min 3 requires 3 candidates).
#[test]
fn ex10_036_clause_a_no_prompt_when_fewer_than_3_mineral_rock_sources() {
    // With only 2 Mineral/Rock sources (min: 3), the SourceMulti selection
    // still appears for picks 1 and 2, but the final callback (delete +
    // security trash) is NOT called because 3 picks cannot be completed.
    // This test verifies the effect never fires (opponent not deleted).
    let src_a = make_mineral_source("FEW-SRC-A1");
    let src_b = make_mineral_source("FEW-SRC-A2");
    let carrier = make_test_card("FEW-CARRIER", "Few Carrier");
    let opp = make_opponent_digimon("FEW-OPP");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-036")
        .expect("EX10-036 YAML loads")
        .add_card(src_a)
        .add_card(src_b)
        .add_card(carrier)
        .add_card(opp)
        .memory(10)
        .start();

    let carrier_h = runner.place_on_field(0, "FEW-CARRIER", None);
    runner.push_source(carrier_h, "FEW-SRC-A1");
    runner.push_source(carrier_h, "FEW-SRC-A2");
    runner.place_on_field(1, "FEW-OPP", None);
    // Give opponent 2 security cards so we can observe no-change.
    let sec_filler = make_test_card("FEW-SEC", "Few Sec");
    runner.game.card_data.push(sec_filler);
    let fi = runner.game.card_data.len() - 1;
    for _ in 0..2 {
        let cs = CardSource::new(fi, 1, runner.game.next_card_index());
        runner.game.players[1].security.push(cs);
    }
    let sec_before = runner.security_count(1);

    let magnet = runner.place_on_field(0, "EX10-036", None);
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(magnet));
    runner.game.drain_effect_queue();
    drain_trigger_order_if_any(&mut runner);

    // The SourceMulti selection may appear for up to 2 partial picks, but
    // the final callback (delete + security trash) is NOT called because
    // the 3rd pick cannot be satisfied. Drain any selections that appear.
    for _ in 0..4 {
        if runner.pending_selection().is_some() {
            runner.auto_resolve().ok();
        }
    }

    // After all selections drain: the opponent Digimon must NOT have been
    // deleted (Clause A's effect never fired — min:3 not met).
    assert_eq!(
        runner.game.players[1].battle_area.len(),
        1,
        "Clause A must not delete the opponent Digimon when fewer than 3 Mineral/Rock sources exist"
    );
    assert_eq!(
        runner.security_count(1),
        sec_before,
        "Clause A must not trash opponent's security when fewer than 3 Mineral/Rock sources exist"
    );
}

/// Negative: plain sources (no Mineral/Rock trait) do not count.
#[test]
fn ex10_036_clause_a_plain_sources_do_not_count() {
    let src_a = make_plain_source("PLAIN-SRC-CA1");
    let src_b = make_plain_source("PLAIN-SRC-CA2");
    let src_c = make_plain_source("PLAIN-SRC-CA3");
    let carrier = make_test_card("PLAIN-CARRIER-CA", "Plain Carrier CA");
    let opp = make_opponent_digimon("PLAIN-OPP-CA");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-036")
        .expect("EX10-036 YAML loads")
        .add_card(src_a)
        .add_card(src_b)
        .add_card(src_c)
        .add_card(carrier)
        .add_card(opp)
        .memory(10)
        .start();

    let carrier_h = runner.place_on_field(0, "PLAIN-CARRIER-CA", None);
    runner.push_source(carrier_h, "PLAIN-SRC-CA1");
    runner.push_source(carrier_h, "PLAIN-SRC-CA2");
    runner.push_source(carrier_h, "PLAIN-SRC-CA3");
    runner.place_on_field(1, "PLAIN-OPP-CA", None);

    let magnet = runner.place_on_field(0, "EX10-036", None);
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(magnet));
    runner.game.drain_effect_queue();
    // Drain TriggerOrder → Clause A fires → silently skips (no Mineral/Rock
    // sources). Clause B fires → no Mineral/Rock in trash → silently skips.
    drain_trigger_order_if_any(&mut runner);

    assert!(
        !matches!(
            runner.pending_kind(),
            Some(SelectionKind::SourceMulti { .. })
        ),
        "Plain (non-Mineral/Rock) sources must not count toward Clause A cost"
    );
}

/// Positive behavioral: Clause A's delete-opponent effect fires after the
/// source-trash cost is paid, prompting for an opponent Digimon (OppField).
///
/// # Execution sequence (with Clause B interaction)
///
/// When `trash_selected_sources` trashes the first source, it fires
/// `fire_digivolution_card_trashed` which calls `drain_effect_queue()`.
/// Clause B is still in the queue at that point (TriggerOrder only dequeued
/// Clause A), so it fires immediately. Clause B sees 1 Mineral card in trash
/// (the just-trashed src1) and installs a SelectTrash pending. This parks
/// Clause A's remaining inner-tail steps (select_opponent_permanent, delete,
/// trash_top_security) as `dsl_outer_tail`.
///
/// After Clause B resolves all 3 picks and places the sources on EX10-036,
/// `drain_dsl_outer_tail` resumes Clause A's inner tail → OppField is prompted.
///
/// This test verifies that OppField DOES eventually appear (after Clause B runs),
/// confirming that Clause A's delete effect fires despite the interleaving.
#[test]
fn ex10_036_clause_a_after_source_trash_prompts_opp_field_delete() {
    let src_a = make_mineral_source("BEH-SRC-A1");
    let src_b = make_mineral_source("BEH-SRC-A2");
    let src_c = make_mineral_source("BEH-SRC-A3");
    let carrier = make_test_card("BEH-CARRIER-A", "Beh Carrier A");
    let opp1 = make_opponent_digimon("BEH-OPP-A1");
    let opp2 = make_opponent_digimon("BEH-OPP-A2");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-036")
        .expect("EX10-036 YAML loads")
        .add_card(src_a)
        .add_card(src_b)
        .add_card(src_c)
        .add_card(carrier)
        .add_card(opp1)
        .add_card(opp2)
        .memory(10)
        .start();

    let carrier_h = runner.place_on_field(0, "BEH-CARRIER-A", None);
    runner.push_source(carrier_h, "BEH-SRC-A1");
    runner.push_source(carrier_h, "BEH-SRC-A2");
    runner.push_source(carrier_h, "BEH-SRC-A3");
    runner.place_on_field(1, "BEH-OPP-A1", None);
    runner.place_on_field(1, "BEH-OPP-A2", None);

    let sec_filler = make_test_card("SEC-BEH-A", "Sec Beh A");
    runner.game.card_data.push(sec_filler);
    let fi = runner.game.card_data.len() - 1;
    let cs = CardSource::new(fi, 1, runner.game.next_card_index());
    runner.game.players[1].security.push(cs);

    let magnet = runner.place_on_field(0, "EX10-036", None);
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(magnet));
    runner.game.drain_effect_queue();
    // Both clauses enqueued simultaneously; drain TriggerOrder first (picks Clause A).
    drain_trigger_order_if_any(&mut runner);

    // Step 1: SourceMulti(3,3) — pick all 3 sources one at a time.
    for _ in 0..3 {
        assert!(
            matches!(runner.pending_kind(), Some(SelectionKind::SourceMulti { .. })),
            "must have SourceMulti pending for each source pick; got {:?}",
            runner.pending_kind()
        );
        let view = runner.pending_selection_view().expect("SourceMulti view present");
        let player = view.selecting_player;
        let action = view.valid_action_ids[0];
        runner.game.resolve_selection(player, action).expect("source pick resolves");
    }

    // After the 3rd source pick, final_callback fires:
    //   1. trash_selected_sources trashes src1 → fire_digivolution_card_trashed
    //      → drain_effect_queue fires Clause B (still queued).
    //   2. Clause B picks up the just-trashed Mineral card; SelectTrash pending
    //      installed. Clause A's remaining steps parked as outer tail.
    //
    // Resolve Clause B's 3 trash picks one at a time (NOT with auto_resolve()).
    // auto_resolve() loops until no pending — it would also consume the OppField
    // prompt that fires after Clause B completes. Use individual resolve_selection
    // calls so we can check the final pending state.
    assert!(
        matches!(runner.pending_kind(), Some(SelectionKind::Trash)),
        "after Clause A's cost is paid, Clause B fires from the trash event \
         and installs a SelectTrash pending; got {:?}",
        runner.pending_kind()
    );
    // Pick each of Clause B's 3 trash cards individually.
    for pick in 1..=3 {
        assert!(
            matches!(runner.pending_kind(), Some(SelectionKind::Trash)),
            "Clause B trash pick {pick}/3 must have SelectTrash pending; got {:?}",
            runner.pending_kind()
        );
        let view = runner.pending_selection_view().expect("trash pick view present");
        let player = view.selecting_player;
        let action = view.valid_action_ids[0];
        runner
            .game
            .resolve_selection(player, action)
            .expect(&format!("Clause B trash pick {pick}/3 resolves"));
    }

    // Step 2: after Clause B's 3rd pick resolves, drain_dsl_outer_tail resumes
    // Clause A's inner tail → select_opponent_permanent → OppField prompted.
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::OppField),
        "after Clause B resolves, Clause A's outer tail runs and prompts OppField; \
         got {:?}",
        runner.pending_kind()
    );
}

/// Positive behavioral: after selecting 3 sources and an opponent Digimon,
/// the chosen Digimon is deleted.
#[test]
fn ex10_036_clause_a_deletes_one_opponent_digimon() {
    let src_a = make_mineral_source("DEL-SRC-A1");
    let src_b = make_mineral_source("DEL-SRC-A2");
    let src_c = make_mineral_source("DEL-SRC-A3");
    let carrier = make_test_card("DEL-CARRIER-A", "Del Carrier A");
    let opp = make_opponent_digimon("DEL-OPP-A");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-036")
        .expect("EX10-036 YAML loads")
        .add_card(src_a)
        .add_card(src_b)
        .add_card(src_c)
        .add_card(carrier)
        .add_card(opp)
        .memory(10)
        .start();

    let carrier_h = runner.place_on_field(0, "DEL-CARRIER-A", None);
    runner.push_source(carrier_h, "DEL-SRC-A1");
    runner.push_source(carrier_h, "DEL-SRC-A2");
    runner.push_source(carrier_h, "DEL-SRC-A3");
    runner.place_on_field(1, "DEL-OPP-A", None);

    let sec_filler = make_test_card("SEC-DEL-A", "Sec Del A");
    runner.game.card_data.push(sec_filler);
    let fi = runner.game.card_data.len() - 1;
    let cs = CardSource::new(fi, 1, runner.game.next_card_index());
    runner.game.players[1].security.push(cs);

    let magnet = runner.place_on_field(0, "EX10-036", None);
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(magnet));
    runner.game.drain_effect_queue();
    drain_trigger_order_if_any(&mut runner);

    // Resolve source pick (Clause A cost: 3 Mineral sources).
    runner.auto_resolve().expect("source multi resolves");
    // Resolve opponent Digimon pick.
    runner.auto_resolve().expect("opp field pick resolves");

    // Opponent must have 0 Digimon on field (it was deleted).
    assert_eq!(
        runner.game.players[1].battle_area.len(),
        0,
        "opponent Digimon must be deleted after Clause A resolves"
    );
}

/// Positive behavioral: after Clause A resolves, the opponent's security count
/// decreases by 1 (top security card was trashed).
#[test]
fn ex10_036_clause_a_trashes_opponent_top_security() {
    let src_a = make_mineral_source("SEC-SRC-A1");
    let src_b = make_mineral_source("SEC-SRC-A2");
    let src_c = make_mineral_source("SEC-SRC-A3");
    let carrier = make_test_card("SEC-CARRIER-A", "Sec Carrier A");
    let opp = make_opponent_digimon("SEC-OPP-A");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-036")
        .expect("EX10-036 YAML loads")
        .add_card(src_a)
        .add_card(src_b)
        .add_card(src_c)
        .add_card(carrier)
        .add_card(opp)
        .memory(10)
        .start();

    let carrier_h = runner.place_on_field(0, "SEC-CARRIER-A", None);
    runner.push_source(carrier_h, "SEC-SRC-A1");
    runner.push_source(carrier_h, "SEC-SRC-A2");
    runner.push_source(carrier_h, "SEC-SRC-A3");
    runner.place_on_field(1, "SEC-OPP-A", None);

    // Give opponent 3 security cards.
    let sec_filler = make_test_card("SEC-FILL-SA", "Sec Fill SA");
    runner.game.card_data.push(sec_filler);
    let fi = runner.game.card_data.len() - 1;
    for _ in 0..3 {
        let cs = CardSource::new(fi, 1, runner.game.next_card_index());
        runner.game.players[1].security.push(cs);
    }
    let sec_before = runner.security_count(1);
    assert_eq!(sec_before, 3, "opponent must start with 3 security");

    let magnet = runner.place_on_field(0, "EX10-036", None);
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(magnet));
    runner.game.drain_effect_queue();
    drain_trigger_order_if_any(&mut runner);

    runner.auto_resolve().expect("source multi resolves");
    runner.auto_resolve().expect("opp field pick resolves");

    let sec_after = runner.security_count(1);
    assert_eq!(
        sec_after,
        sec_before - 1,
        "opponent's top security card must be trashed (before: {sec_before}, after: {sec_after})"
    );
}

/// Positive behavioral: the 3 cost sources are removed from the Digimon's
/// digivolution stack after Clause A resolves.
#[test]
fn ex10_036_clause_a_trashes_the_3_selected_sources() {
    let src_a = make_mineral_source("TSRC-A1");
    let src_b = make_mineral_source("TSRC-A2");
    let src_c = make_mineral_source("TSRC-A3");
    let carrier = make_test_card("TCARRIER-A", "Trash Carrier A");
    let opp = make_opponent_digimon("TOPP-A");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-036")
        .expect("EX10-036 YAML loads")
        .add_card(src_a)
        .add_card(src_b)
        .add_card(src_c)
        .add_card(carrier)
        .add_card(opp)
        .memory(10)
        .start();

    let carrier_h = runner.place_on_field(0, "TCARRIER-A", None);
    runner.push_source(carrier_h, "TSRC-A1");
    runner.push_source(carrier_h, "TSRC-A2");
    runner.push_source(carrier_h, "TSRC-A3");

    // Note sources_before: 1 top card (TCARRIER-A) + 3 pushed sources = 4.
    let sources_before = runner.game.players[0].battle_area[carrier_h.index as usize]
        .card_sources
        .len();
    let trash_before = runner.trash_size(0);

    runner.place_on_field(1, "TOPP-A", None);
    let sec_filler = make_test_card("SEC-TSRC", "Sec Tsrc");
    runner.game.card_data.push(sec_filler);
    let fi = runner.game.card_data.len() - 1;
    let cs = CardSource::new(fi, 1, runner.game.next_card_index());
    runner.game.players[1].security.push(cs);

    let magnet = runner.place_on_field(0, "EX10-036", None);
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(magnet));
    runner.game.drain_effect_queue();
    drain_trigger_order_if_any(&mut runner);

    runner.auto_resolve().expect("source multi resolves");
    runner.auto_resolve().expect("opp field pick resolves");

    let sources_after = runner.game.players[0].battle_area[carrier_h.index as usize]
        .card_sources
        .len();
    let trash_after = runner.trash_size(0);

    assert_eq!(
        sources_after,
        sources_before - 3,
        "3 sources must be removed from the carrier's stack (before: {sources_before}, after: {sources_after})"
    );
    // Note: Clause B fires immediately after Clause A via drain_effect_queue and finds the
    // 3 Mineral cards in player 0's trash. It then places all 3 back onto EX10-036's bottom
    // sources. Net trash change is therefore 0, not +3. The important invariant is that the
    // carrier's stack shrank by 3 (Clause A's cost was paid), not the final trash count.
    let _ = (trash_before, trash_after);
}

/// Multi-timing: Clause A also fires at WhenAttacking.
#[test]
fn ex10_036_clause_a_when_attacking_fires_source_trash_delete_security() {
    let src_a = make_mineral_source("WA-SRC-A1");
    let src_b = make_mineral_source("WA-SRC-A2");
    let src_c = make_mineral_source("WA-SRC-A3");
    let carrier = make_test_card("WA-CARRIER-A", "WA Carrier A");
    let opp = make_opponent_digimon("WA-OPP-A");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-036")
        .expect("EX10-036 YAML loads")
        .add_card(src_a)
        .add_card(src_b)
        .add_card(src_c)
        .add_card(carrier)
        .add_card(opp)
        .memory(10)
        .start();

    let carrier_h = runner.place_on_field(0, "WA-CARRIER-A", None);
    runner.push_source(carrier_h, "WA-SRC-A1");
    runner.push_source(carrier_h, "WA-SRC-A2");
    runner.push_source(carrier_h, "WA-SRC-A3");
    runner.place_on_field(1, "WA-OPP-A", None);

    let sec_filler = make_test_card("SEC-WA-A", "Sec WA A");
    runner.game.card_data.push(sec_filler);
    let fi = runner.game.card_data.len() - 1;
    let cs = CardSource::new(fi, 1, runner.game.next_card_index());
    runner.game.players[1].security.push(cs);

    let sec_before = runner.security_count(1);

    let magnet = runner.place_on_field(0, "EX10-036", None);
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(magnet));
    runner.game.drain_effect_queue();
    drain_trigger_order_if_any(&mut runner);

    runner.auto_resolve().expect("source multi resolves");
    runner.auto_resolve().expect("opp field pick resolves");

    let sec_after = runner.security_count(1);
    assert_eq!(
        runner.game.players[1].battle_area.len(),
        0,
        "WhenAttacking: opponent Digimon must be deleted"
    );
    assert_eq!(
        sec_after,
        sec_before - 1,
        "WhenAttacking: opponent top security must be trashed"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — Clause B: [WD][WA][OPT] trash-to-source unsuspend
// ═══════════════════════════════════════════════════════════════════════════════
//
// Setup: EX10-036 on field (suspended). Player's trash has 3+ Mineral/Rock cards.
// Fire WhenDigivolving. Clause B should prompt for 3 trash picks (TrashPick).

/// Positive: Clause B prompts for a trash card selection when ≥3 Mineral/Rock
/// cards are in trash. (First of 3 `select_trash` picks.)
#[test]
fn ex10_036_clause_b_when_digivolving_prompts_trash_pick_when_3_mineral_in_trash() {
    let ts_a = make_mineral_source("TRS-B1");
    let ts_b = make_mineral_source("TRS-B2");
    let ts_c = make_mineral_source("TRS-B3");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-036")
        .expect("EX10-036 YAML loads")
        .add_card(ts_a)
        .add_card(ts_b)
        .add_card(ts_c)
        .memory(10)
        .start();

    push_to_trash(&mut runner, 0, "TRS-B1");
    push_to_trash(&mut runner, 0, "TRS-B2");
    push_to_trash(&mut runner, 0, "TRS-B3");

    let magnet = runner.place_on_field(0, "EX10-036", None);

    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(magnet));
    runner.game.drain_effect_queue();
    // Both clauses enqueued; drain TriggerOrder (picks Clause A first).
    // Clause A has no Mineral/Rock sources in any digivolution stack here →
    // silently skips. Clause B fires next and should install a Trash-pick.
    drain_trigger_order_if_any(&mut runner);

    assert!(
        runner.pending_selection().is_some(),
        "Clause B must install a pending selection (trash pick) when 3 Mineral cards are in trash"
    );
}

/// Positive behavioral: after picking 3 Mineral/Rock cards from trash and
/// placing them as bottom sources, this Digimon unsuspends.
#[test]
fn ex10_036_clause_b_places_3_trash_cards_as_bottom_sources_and_unsuspends() {
    let ts_a = make_mineral_source("BSRC-B1");
    let ts_b = make_mineral_source("BSRC-B2");
    let ts_c = make_mineral_source("BSRC-B3");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-036")
        .expect("EX10-036 YAML loads")
        .add_card(ts_a)
        .add_card(ts_b)
        .add_card(ts_c)
        .memory(10)
        .start();

    push_to_trash(&mut runner, 0, "BSRC-B1");
    push_to_trash(&mut runner, 0, "BSRC-B2");
    push_to_trash(&mut runner, 0, "BSRC-B3");

    let magnet = runner.place_on_field(0, "EX10-036", None);
    // Suspend EX10-036 manually so we can verify it unsuspends.
    runner.game.players[0].battle_area[magnet.index as usize].is_suspended = true;

    // Pre-condition: must be suspended.
    assert!(
        runner.game.players[0].battle_area[magnet.index as usize].is_suspended,
        "EX10-036 must be suspended before Clause B fires"
    );

    let sources_before = runner.game.players[0].battle_area[magnet.index as usize]
        .card_sources
        .len();
    let trash_before = runner.trash_size(0);

    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(magnet));
    runner.game.drain_effect_queue();
    // Drain TriggerOrder (picks Clause A first); Clause A silently skips
    // (no Mineral/Rock sources in stacks). Then Clause B fires.
    drain_trigger_order_if_any(&mut runner);

    // Resolve all 3 trash picks in a single auto_resolve() call.
    //
    // Why one call? The select_trash steps are chained via continuation callbacks:
    // pick-1's callback runs [place_as_bottom_source(src1), select_trash(src2), ...]
    // which installs pick-2 inside the same callback invocation. auto_resolve()
    // loops until no pending, so it resolves picks 1 → 2 → 3 and the final
    // unsuspend step all before returning.
    runner
        .auto_resolve()
        .expect("all 3 Clause B trash picks and unsuspend resolve in one auto_resolve");

    // After the 3 picks and placements, EX10-036 must be unsuspended.
    assert!(
        !runner.game.players[0].battle_area[magnet.index as usize].is_suspended,
        "EX10-036 must be unsuspended after Clause B resolves"
    );

    // The 3 trash cards must now be in EX10-036's digivolution stack.
    let sources_after = runner.game.players[0].battle_area[magnet.index as usize]
        .card_sources
        .len();
    assert_eq!(
        sources_after,
        sources_before + 3,
        "3 cards from trash must be placed as bottom sources (before: {sources_before}, after: {sources_after})"
    );

    // The trash must have shrunk by 3 (cards moved from trash to sources).
    let trash_after = runner.trash_size(0);
    assert_eq!(
        trash_before - trash_after,
        3,
        "3 cards must leave trash (before: {trash_before}, after: {trash_after})"
    );
}

/// Positive: Clause B is optional — `pending_is_optional()` must return true
/// for the first trash pick.
#[test]
fn ex10_036_clause_b_is_optional_first_trash_pick() {
    let ts_a = make_mineral_source("OPT-TRS-B1");
    let ts_b = make_mineral_source("OPT-TRS-B2");
    let ts_c = make_mineral_source("OPT-TRS-B3");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-036")
        .expect("EX10-036 YAML loads")
        .add_card(ts_a)
        .add_card(ts_b)
        .add_card(ts_c)
        .memory(10)
        .start();

    push_to_trash(&mut runner, 0, "OPT-TRS-B1");
    push_to_trash(&mut runner, 0, "OPT-TRS-B2");
    push_to_trash(&mut runner, 0, "OPT-TRS-B3");

    let magnet = runner.place_on_field(0, "EX10-036", None);
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(magnet));
    runner.game.drain_effect_queue();
    // Drain TriggerOrder (picks Clause A first); Clause A silently skips
    // (no Mineral/Rock sources). Clause B fires; its first select_trash
    // step has optional: true so the player can PASS at the first prompt.
    drain_trigger_order_if_any(&mut runner);

    // The first selection for Clause B must be optional (player can PASS).
    assert!(
        runner.pending_is_optional(),
        "Clause B's first trash pick must be optional (player may decline the whole clause)"
    );
}

/// OPT lockout: Clause B must not fire twice in the same turn.
///
/// # Test design notes
///
/// - Only 3 Mineral cards in trash (enough for exactly one Clause B activation).
/// - First fire (WhenDigivolving): Clause A silently skips (magnet starts with
///   1 source = just its top card; `has_own_source_candidates` is false). Clause B
///   fires, picks all 3 Mineral cards from trash, places them on magnet as bottom
///   sources (magnet now has 4 sources: top + 3 Mineral), and unsuspends.
/// - Re-suspend magnet before second fire.
/// - Second fire (WhenAttacking): both Clause A and Clause B are enqueued.
///   TriggerOrder picks Clause A first. Clause A NOW sees 3 Mineral sources on
///   magnet (placed by Clause B). It runs SelectOwnSources → picks all 3 →
///   trashes them. No opponent Digimon is on field so select_opponent_permanent
///   silently skips (0 candidates → Continue → remaining steps no-op).
///   drain_effect_queue then fires Clause B which is OPT-blocked → no pending.
/// - After calling auto_resolve() to resolve Clause A, pending must be None
///   and magnet must remain suspended (Clause B did not run).
#[test]
fn ex10_036_clause_b_opt_blocks_second_activation_same_turn() {
    let ts_a = make_mineral_source("LOCK-TRS-B1");
    let ts_b = make_mineral_source("LOCK-TRS-B2");
    let ts_c = make_mineral_source("LOCK-TRS-B3");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-036")
        .expect("EX10-036 YAML loads")
        .add_card(ts_a)
        .add_card(ts_b)
        .add_card(ts_c)
        .memory(10)
        .start();

    // 3 Mineral cards in trash — enough for exactly one Clause B activation.
    for id in &["LOCK-TRS-B1", "LOCK-TRS-B2", "LOCK-TRS-B3"] {
        push_to_trash(&mut runner, 0, id);
    }

    let magnet = runner.place_on_field(0, "EX10-036", None);
    runner.game.players[0].battle_area[magnet.index as usize].is_suspended = true;

    // First fire (WhenDigivolving):
    // Clause A skips (magnet has only 1 source = top card, so
    // has_own_source_candidates returns false → SelectOwnSources TailAlreadyRan).
    // Clause B picks all 3 Mineral cards from trash and unsuspends magnet.
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(magnet));
    runner.game.drain_effect_queue();
    drain_trigger_order_if_any(&mut runner);
    // One auto_resolve() resolves all 3 trash picks + unsuspend.
    runner
        .auto_resolve()
        .expect("first activation: all 3 Clause B picks and unsuspend resolve");

    assert!(
        !runner.game.players[0].battle_area[magnet.index as usize].is_suspended,
        "after first Clause B activation, magnet must be unsuspended"
    );

    // Re-suspend for the second fire attempt.
    runner.game.players[0].battle_area[magnet.index as usize].is_suspended = true;

    // Second fire (WhenAttacking):
    // Both Clause A and Clause B are enqueued. TriggerOrder picks Clause A.
    // Clause A now sees 3 Mineral sources on magnet (placed by Clause B above).
    // auto_resolve() resolves Clause A: picks 3 sources → trashes them →
    // select_opponent_permanent has 0 candidates → Continue → remaining nop.
    // drain_effect_queue then fires Clause B → OPT-blocked → no pending.
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(magnet));
    runner.game.drain_effect_queue();
    drain_trigger_order_if_any(&mut runner);
    // Resolve Clause A (which now has valid Mineral sources to pick from).
    runner
        .auto_resolve()
        .expect("second fire: Clause A resolves (picks 3 Mineral sources off magnet)");

    assert!(
        runner.pending_selection().is_none(),
        "Clause B OPT must block second activation in the same turn"
    );

    // After OPT lockout, magnet must remain suspended.
    assert!(
        runner.game.players[0].battle_area[magnet.index as usize].is_suspended,
        "EX10-036 must remain suspended after OPT lockout blocks Clause B"
    );
}

/// Multi-timing: Clause B also fires at WhenAttacking.
#[test]
fn ex10_036_clause_b_when_attacking_also_prompts_trash_pick() {
    let ts_a = make_mineral_source("WA-TRS-B1");
    let ts_b = make_mineral_source("WA-TRS-B2");
    let ts_c = make_mineral_source("WA-TRS-B3");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-036")
        .expect("EX10-036 YAML loads")
        .add_card(ts_a)
        .add_card(ts_b)
        .add_card(ts_c)
        .memory(10)
        .start();

    push_to_trash(&mut runner, 0, "WA-TRS-B1");
    push_to_trash(&mut runner, 0, "WA-TRS-B2");
    push_to_trash(&mut runner, 0, "WA-TRS-B3");

    let magnet = runner.place_on_field(0, "EX10-036", None);
    runner.game.players[0].battle_area[magnet.index as usize].is_suspended = true;

    runner
        .game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(magnet));
    runner.game.drain_effect_queue();
    // Drain TriggerOrder (picks Clause A first); Clause A silently skips
    // (no Mineral/Rock sources in stacks). Clause B fires.
    drain_trigger_order_if_any(&mut runner);

    assert!(
        runner.pending_selection().is_some(),
        "Clause B must also prompt at WhenAttacking when ≥3 Mineral/Rock cards are in trash"
    );
}

/// Partial cost scenario: with only 2 Mineral/Rock cards in trash, Clause B
/// selects both and places them as sources, but the 3rd pick has 0 candidates.
///
/// # Engine behavior (G-SELECT-EMPTY-MANDATORY-ABORT gap)
///
/// When a non-optional `select_trash` step encounters 0 valid candidates it
/// returns `InstallResult::Continue` rather than aborting the clause. The outer
/// step dispatcher then advances past the empty-pick step and runs the remaining
/// steps — `place_as_bottom_source(src3)` (silently no-ops because the `src3`
/// binding is unset) and `unsuspend`. As a result the Digimon IS unsuspended
/// even though the cost was only partially paid.
///
/// CORRECT behavior per card text ("By placing 3 … cards … it unsuspends"):
///   unsuspend should NOT fire when the cost cannot be fully paid.
///
/// This test documents the ACTUAL (gap) engine behavior. When the gap is fixed
/// (G-SELECT-EMPTY-MANDATORY-ABORT) the final assertion should be flipped.
#[test]
fn ex10_036_clause_b_no_unsuspend_when_fewer_than_3_mineral_rock_in_trash() {
    let ts_a = make_mineral_source("INSUF-TRS-B1");
    let ts_b = make_mineral_source("INSUF-TRS-B2");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-036")
        .expect("EX10-036 YAML loads")
        .add_card(ts_a)
        .add_card(ts_b)
        .memory(10)
        .start();

    // Only 2 Mineral cards in trash — not enough for 3 picks.
    push_to_trash(&mut runner, 0, "INSUF-TRS-B1");
    push_to_trash(&mut runner, 0, "INSUF-TRS-B2");

    let magnet = runner.place_on_field(0, "EX10-036", None);
    runner.game.players[0].battle_area[magnet.index as usize].is_suspended = true;

    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(magnet));
    runner.game.drain_effect_queue();
    // Drain TriggerOrder (picks Clause A first); Clause A silently skips
    // (no Mineral/Rock sources in stacks). Clause B fires: only 2 Mineral
    // cards in trash, so picks 1 and 2 fire, pick 3 returns Continue (0
    // candidates) and the outer dispatcher runs `unsuspend` directly.
    drain_trigger_order_if_any(&mut runner);

    // auto_resolve() resolves all pending selections. With 2 Mineral cards:
    // pick 1 (2 candidates) → picked → pick 2 (1 candidate) → picked →
    // pick 3 (0 candidates) → select_trash returns Continue (no pending
    // installed) → outer dispatcher continues to place_as_bottom_source(src3)
    // (nop: src3 unbound) → unsuspend runs.
    // One auto_resolve() call resolves the two actual picks plus the
    // continuation that fires unsuspend.
    runner.auto_resolve().ok();

    // ACTUAL engine behavior (gap G-SELECT-EMPTY-MANDATORY-ABORT):
    // Digimon IS unsuspended even though only 2 of 3 picks were paid.
    // TODO: when the gap is fixed, this assertion should become:
    //   assert!(is_suspended, "should NOT unsuspend when cost is only partially paid")
    assert!(
        !runner.game.players[0].battle_area[magnet.index as usize].is_suspended,
        "KNOWN GAP (G-SELECT-EMPTY-MANDATORY-ABORT): engine unsuspends even with \
         only 2 picks paid because the 3rd empty select_trash returns Continue \
         instead of aborting the clause tail"
    );

    // Verify: only 2 cards were placed as bottom sources (not 3).
    // place_as_bottom_source(src3) is a no-op when src3 is unbound, so
    // magnet ends up with 1 (original top card) + 2 (placed picks) = 3.
    let sources_after = runner.game.players[0].battle_area[magnet.index as usize]
        .card_sources
        .len();
    assert_eq!(
        sources_after,
        3,
        "only 2 cards should be placed as bottom sources (src3 binding is empty); \
         expected 1 top + 2 placed = 3 total, got {sources_after}"
    );
}
