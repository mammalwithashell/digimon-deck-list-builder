//! EX4-073 Omnimon Alter-B — Digimon, Lv.7, Black, DP 15000, Cost 15.
//! Traits: Holy Warrior. Attribute: Virus.
//! Evo: Lv6 Black / Cost 5. Special evo: Lv7 w/[Omnimon] in name / Cost 2.
//!
//! # Card text (cards.json)
//!
//! ```text
//! [When Digivolving] <De-Digivolve 3> 1 of your opponent's Digimon.
//!   (Trash up to 3 cards from the top. You can't trash past level 3 cards.)
//!   Then, delete up to 6 play cost's total worth of their Digimon.
//! [When Attacking] By trashing up to 3 level 6 or higher cards in this
//!   Digimon's digivolution cards, for each card trashed, activate the effect
//!   below. Then, if you trashed 3 cards, trash the top 2 cards of your
//!   opponent's security stack.
//!   - Delete 1 of your opponent's Digimon or Tamers with the lowest play cost.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX4/Black/EX4_073.cs
//!
//! # Patterns this test covers
//! - Two alt-paths (standard digivolve + Omnimon-name special digivolve at cost 2)
//! - [When Digivolving] arm 1: select_opponent_permanent + de_digivolve(3, stop_at:3)
//! - [When Digivolving] arm 2: BLOCKED — G-MULTI-SELECT-OPP-PLAY-COST-SUM
//!   (no play_cost-budget multi-select in DSL or engine; only DP-budget exists)
//! - [When Attacking] clause C: IMPLEMENTED — `select_own_sources` with
//!   `filter: { level_gte: 6 }` (G-DSL-SELECT-OWN-SOURCES-FILTER CLOSED),
//!   `per_selected` over the source_refs binding for the per-trash
//!   lowest-play-cost delete (`selector: lowest_play_cost`), and a
//!   `binding_count_eq` gate for the "if you trashed 3" security tail.

use digimon_dsl::compiled::{
    CompiledClause, CompiledScope, CompiledTiming, CompiledTriggeredClause,
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::EffectTiming;
use digimon_engine::selection::TriggerSource;

// ─── Helper ──────────────────────────────────────────────────────────────────

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX4-073")
        .expect("EX4-073 in embedded DSL pack")
        .memory(20)
        .start()
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Compilation smoke test
// ═══════════════════════════════════════════════════════════════════════════════

/// EX4-073 YAML must compile and appear in the embedded DSL pack.
#[test]
fn ex4_073_yaml_compiles_without_error() {
    let runner = runner();
    let card = runner.compiled_card("EX4-073");
    assert!(
        card.is_some(),
        "EX4-073 must be present in the embedded DSL pack (YAML must parse + compile)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

/// EX4-073 has TWO triggered clauses: clause B ([When Digivolving], both arms)
/// and clause C ([When Attacking]). All blocking gaps are closed.
#[test]
fn ex4_073_has_two_triggered_clauses_authored() {
    let runner = runner();
    let card = runner
        .compiled_card("EX4-073")
        .expect("EX4-073 in embedded pack");

    let triggered: Vec<&CompiledTriggeredClause> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(
        triggered.len(),
        2,
        "expected 2 triggered clauses: clause B (when_digivolving) + clause C (when_attacking)"
    );

    let has_when_attacking = triggered
        .iter()
        .any(|t| t.when.contains(&CompiledTiming::WhenAttacking));
    assert!(
        has_when_attacking,
        "clause C must fire on WhenAttacking"
    );
}

/// The single authored triggered clause fires on WhenDigivolving and is
/// mandatory (no "you may"), face-up scope, no [Once Per Turn].
#[test]
fn ex4_073_clause_b_fires_on_when_digivolving_mandatory_face_up() {
    let runner = runner();
    let card = runner
        .compiled_card("EX4-073")
        .expect("EX4-073 in embedded pack");

    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::WhenDigivolving))
        .expect("clause B must include WhenDigivolving");

    assert!(
        !clause.optional,
        "clause B is mandatory — printed text has no 'you may' on the outer trigger"
    );
    assert!(!clause.once_per_turn, "clause B has no [Once Per Turn]");
    assert_eq!(clause.scope, CompiledScope::FaceUp, "clause B is own-scope");
}

/// EX4-073 has two alt-paths: standard digivolve (Lv6 Black / 5) and
/// Omnimon-name special digivolve (Lv7 / 2).
#[test]
fn ex4_073_has_two_alt_paths() {
    let runner = runner();
    let card = runner
        .compiled_card("EX4-073")
        .expect("EX4-073 in embedded pack");

    assert_eq!(
        card.alt_paths.len(),
        2,
        "EX4-073 has standard digivolve (Lv6 Black) + Omnimon-name special (Lv7) = 2 alt-paths"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Clause B arm 1 behavioral: <De-Digivolve 3> on 1 opp Digimon
// ═══════════════════════════════════════════════════════════════════════════════

/// Clause B arm 1: opponent has 1 Lv5 Digimon with stacked sources →
/// fire WhenDigivolving → selection is installed for the de-digivolve target.
/// (No multi-pick budget — clause B arm 2 is BLOCKED.)
#[test]
fn ex4_073_clause_b_installs_selection_for_de_digivolve_target() {
    let mut opp_lv5 = make_test_card("OPP-LV5", "OppLv5");
    opp_lv5.level = Some(5);
    opp_lv5.dp = Some(7000);

    let mut runner = DebugRunner::builder()
        .dsl_card("EX4-073")
        .expect("EX4-073 in embedded DSL pack")
        .add_card(opp_lv5)
        .memory(20)
        .start();

    let handle = runner.place_on_field(0, "EX4-073", Some(0));
    runner.place_on_field(1, "OPP-LV5", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(handle),
    );
    runner.game.drain_effect_queue();

    // A pending selection must be installed for the de-digivolve pick.
    assert!(
        runner.game.pending_selection.is_some(),
        "clause B arm 1 must install a pending selection for the de-digivolve target"
    );
}

/// Clause B arm 1: opponent has no Digimon → outer condition gate prevents
/// firing; no selection installed.
#[test]
fn ex4_073_clause_b_no_fire_when_opponent_has_no_digimon() {
    let mut runner = runner();
    let handle = runner.place_on_field(0, "EX4-073", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(handle),
    );
    runner.game.drain_effect_queue();

    assert!(
        runner.game.pending_selection.is_none(),
        "no opponent Digimon → outer existential condition fails → no selection"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — Clause B arm 2 (BLOCKED): delete up to 6 play-cost worth
// ═══════════════════════════════════════════════════════════════════════════════

/// Build a Digimon test card with an explicit play cost.
fn opp_digimon(id: &str, name: &str, level: u8, play_cost: u16) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = digimon_engine::enums::CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(4000);
    card.play_cost = play_cost;
    card
}

/// Clause B arm 2 — "delete up to 6 play cost's total worth of their Digimon".
/// G-MULTI-SELECT-OPP-PLAY-COST-SUM CLOSED: after arm 1's de-digivolve target
/// select resolves, the play-cost-budget multi-select installs. The player
/// picks opponent Digimon within a running play-cost sum ≤ 6; each pick is
/// deleted.
#[test]
fn ex4_073_clause_b_arm2_deletes_opp_digimon_within_play_cost_sum_6() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX4-073")
        .expect("EX4-073 in embedded DSL pack")
        .add_card(opp_digimon("DD-TGT", "DeDigiTarget", 5, 5))
        .add_card(opp_digimon("CHEAP-A", "Cheap3", 4, 3))
        .add_card(opp_digimon("CHEAP-B", "Cheap2", 4, 2))
        .memory(20)
        .start();

    let handle = runner.place_on_field(0, "EX4-073", Some(0));
    runner.place_on_field(1, "DD-TGT", Some(0));
    let cheap_a = runner.place_on_field(1, "CHEAP-A", Some(0));
    let cheap_b = runner.place_on_field(1, "CHEAP-B", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(handle),
    );
    runner.game.drain_effect_queue();

    // Arm 1: resolve the de-digivolve target select (pick the only OppField
    // candidate).
    let view = runner
        .pending_selection_view()
        .expect("arm 1 de-digivolve target select installs");
    let sel_player = view.selecting_player;
    runner
        .game
        .resolve_selection(sel_player, view.valid_action_ids[0])
        .expect("arm 1 de-digivolve resolves");

    // Arm 2: the play-cost-budget multi-select installs. Pick CHEAP-A (cost 3)
    // then CHEAP-B (cost 2) — running sum 5 ≤ 6.
    use digimon_engine::action::space::{encode_attack, PASS};
    let v = runner
        .pending_selection_view()
        .expect("arm 2 play-cost-budget select installs");
    runner
        .game
        .resolve_selection(v.selecting_player, encode_attack(0, cheap_a.index as u16))
        .expect("pick CHEAP-A");
    let v = runner
        .pending_selection_view()
        .expect("budget select still open after first pick");
    runner
        .game
        .resolve_selection(v.selecting_player, encode_attack(0, cheap_b.index as u16))
        .expect("pick CHEAP-B");
    // Drain any remaining trampoline (PASS to finish if still open).
    while let Some(v) = runner.pending_selection_view() {
        if v.valid_action_ids.contains(&PASS) {
            runner
                .game
                .resolve_selection(v.selecting_player, PASS)
                .expect("decline");
        } else {
            runner
                .game
                .resolve_selection(v.selecting_player, v.valid_action_ids[0])
                .expect("advance");
        }
    }
    let _ = (cheap_a, cheap_b);

    // Both cheap Digimon were deleted (running sum 3 + 2 = 5 ≤ 6).
    let surviving: Vec<String> = runner
        .game
        .player(1)
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&runner.game.card_data).to_string())
        .collect();
    assert!(
        !surviving.contains(&"CHEAP-A".to_string())
            && !surviving.contains(&"CHEAP-B".to_string()),
        "both opponent Digimon within the play-cost budget must be deleted; \
         survivors={surviving:?}"
    );
}

/// Clause B arm 2: a single opp Digimon with play_cost > 6 must be ineligible
/// (individual cap, mirrors DCGO `permanent.TopCard.GetCostItself <= 6`).
#[test]
fn ex4_073_clause_b_arm2_excludes_opp_digimon_with_play_cost_above_6() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX4-073")
        .expect("EX4-073 in embedded DSL pack")
        .add_card(opp_digimon("DD-TGT", "DeDigiTarget", 5, 5))
        .add_card(opp_digimon("EXPENSIVE", "Expensive8", 6, 8))
        .memory(20)
        .start();

    let handle = runner.place_on_field(0, "EX4-073", Some(0));
    runner.place_on_field(1, "DD-TGT", Some(0));
    runner.place_on_field(1, "EXPENSIVE", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(handle),
    );
    runner.game.drain_effect_queue();

    // Arm 1: resolve the de-digivolve target select.
    let view = runner
        .pending_selection_view()
        .expect("arm 1 de-digivolve target select installs");
    runner
        .game
        .resolve_selection(view.selecting_player, view.valid_action_ids[0])
        .expect("arm 1 resolves");

    // Arm 2: only EXPENSIVE (cost 8 > 6) and DD-TGT (cost 5) remain. The
    // play-cost-budget multi-select must offer DD-TGT but never EXPENSIVE.
    use digimon_engine::action::space::{encode_attack, PASS};
    if let Some(v) = runner.pending_selection_view() {
        // EXPENSIVE is at field index 1 on player 1.
        let expensive_action = encode_attack(0, 1);
        assert!(
            !v.valid_action_ids.contains(&expensive_action),
            "opponent Digimon with play cost 8 (> budget 6) must NOT be selectable; \
             valid={:?}",
            v.valid_action_ids
        );
        // Decline the rest.
        runner
            .game
            .resolve_selection(v.selecting_player, PASS)
            .expect("decline arm 2");
    }

    // EXPENSIVE must survive — it was never selectable.
    let surviving: Vec<String> = runner
        .game
        .player(1)
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&runner.game.card_data).to_string())
        .collect();
    assert!(
        surviving.contains(&"EXPENSIVE".to_string()),
        "the over-budget opponent Digimon must survive; survivors={surviving:?}"
    );
}

/// Clause B arm 2: when no opp Digimon are eligible after arm 1 (none on field
/// at all), the arm silently no-ops and arm 1 still resolved.
#[test]
fn ex4_073_clause_b_arm2_silent_noop_when_no_eligible_targets() {
    // Single opponent Digimon: it is the de-digivolve target, and after arm 1
    // resolves it remains the only opponent permanent. Give it play cost > 6
    // so arm 2 has no eligible candidate.
    let mut runner = DebugRunner::builder()
        .dsl_card("EX4-073")
        .expect("EX4-073 in embedded DSL pack")
        .add_card(opp_digimon("LONE", "LoneExpensive", 6, 9))
        .memory(20)
        .start();

    let handle = runner.place_on_field(0, "EX4-073", Some(0));
    runner.place_on_field(1, "LONE", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(handle),
    );
    runner.game.drain_effect_queue();

    // Arm 1: resolve the de-digivolve target select.
    let view = runner
        .pending_selection_view()
        .expect("arm 1 de-digivolve target select installs");
    runner
        .game
        .resolve_selection(view.selecting_player, view.valid_action_ids[0])
        .expect("arm 1 resolves");

    // Arm 2: LONE costs 9 > 6 — no eligible candidate, so the budget step
    // silently no-ops. No further pending selection.
    assert!(
        runner.game.pending_selection.is_none(),
        "arm 2 must silently no-op when no opponent Digimon fits the play-cost budget"
    );
    // The clause completed — LONE is still on the field (de-digivolved, not
    // deleted, since its play cost exceeds the budget).
    assert_eq!(
        runner.game.player(1).battle_area.len(),
        1,
        "the lone over-budget opponent Digimon survives arm 2"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 5 — Clause C: [When Attacking] trash Lv6+ sources + lowest-cost delete
// ═══════════════════════════════════════════════════════════════════════════════
//
// G-DSL-SELECT-OWN-SOURCES-FILTER CLOSED — `select_own_sources` carries a
// per-source `filter:`; `per_selected` iterates a `source_refs` binding; and
// `binding_count_eq` gates the "if you trashed 3" tail.

/// A digivolution-source card at a given level.
fn source_card(id: &str, level: u8) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = digimon_engine::enums::CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(5000);
    card
}

/// Fire WhenAttacking for a permanent.
fn fire_when_attacking(runner: &mut DebugRunner, handle: digimon_engine::permanent::PermanentHandle) {
    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(handle),
    );
    runner.game.drain_effect_queue();
}

/// Clause C outer step — "trash up to 3 level 6 or higher cards in this
/// Digimon's digivolution cards". G-DSL-SELECT-OWN-SOURCES-FILTER CLOSED:
/// `select_own_sources` with `filter: { level_gte: 6 }` offers ONLY the Lv6+
/// digivolution sources — a Lv5 source must never be selectable.
#[test]
fn ex4_073_clause_c_trashes_up_to_3_lv6_or_higher_sources() {
    use digimon_engine::action::space::encode_source_select;

    let mut runner = DebugRunner::builder()
        .dsl_card("EX4-073")
        .expect("EX4-073 in embedded DSL pack")
        .add_card(source_card("LV6-SRC", 6))
        .add_card(source_card("LV7-SRC", 7))
        .add_card(source_card("LV5-SRC", 5))
        .add_card(opp_digimon("OPP-D", "OppDigimon", 4, 3))
        .memory(20)
        .start();

    // EX4-073 stack: [LV6-SRC, LV5-SRC, LV7-SRC, EX4-073] — two Lv6+ sources
    // (indices 0 and 2) and one Lv5 source (index 1).
    let handle = runner.place_stack(0, &["LV6-SRC", "LV5-SRC", "LV7-SRC", "EX4-073"]);
    runner.place_on_field(1, "OPP-D", Some(0));

    fire_when_attacking(&mut runner, handle);

    let view = runner
        .pending_selection_view()
        .expect("clause C must install a source-select when Lv6+ sources exist");
    let lv6_action =
        encode_source_select(handle.index as u16, 0).expect("Lv6 source action");
    let lv7_action =
        encode_source_select(handle.index as u16, 2).expect("Lv7 source action");
    let lv5_action =
        encode_source_select(handle.index as u16, 1).expect("Lv5 source action");
    assert!(
        view.valid_action_ids.contains(&lv6_action)
            && view.valid_action_ids.contains(&lv7_action),
        "both Lv6+ digivolution sources must be selectable; valid={:?}",
        view.valid_action_ids
    );
    assert!(
        !view.valid_action_ids.contains(&lv5_action),
        "the Lv5 digivolution source must NOT be selectable (level_gte: 6 filter); valid={:?}",
        view.valid_action_ids
    );

    // Pick both Lv6+ sources — they must move to trash.
    let trash_before = runner.trash_size(0);
    runner.execute_action(0, lv6_action).expect("pick Lv6 source");
    runner.execute_action(0, lv7_action).expect("pick Lv7 source");
    // Resolve the per-trash delete prompts + finish.
    while let Some(v) = runner.pending_selection_view() {
        let act = v
            .valid_action_ids
            .first()
            .copied()
            .expect("a resolvable action");
        runner.execute_action(v.selecting_player, act).ok();
    }
    assert_eq!(
        runner.trash_size(0),
        trash_before + 2,
        "the two trashed Lv6+ digivolution sources must land in trash"
    );
}

/// Clause C inner per-trash effect — "Delete 1 of your opponent's Digimon or
/// Tamers with the lowest play cost." For each of N trashed sources, one
/// opponent permanent is deleted; `selector: lowest_play_cost` offers only the
/// cheapest candidate(s).
#[test]
fn ex4_073_clause_c_inner_deletes_opp_lowest_play_cost_per_trash() {
    use digimon_engine::action::space::{encode_source_select, PASS};

    let mut runner = DebugRunner::builder()
        .dsl_card("EX4-073")
        .expect("EX4-073 in embedded DSL pack")
        .add_card(source_card("LV6-SRC", 6))
        .add_card(opp_digimon("OPP-CHEAP", "OppCheap", 4, 2))
        .add_card(opp_digimon("OPP-PRICEY", "OppPricey", 6, 9))
        .memory(20)
        .start();

    // EX4-073 with one Lv6 digivolution source.
    let handle = runner.place_stack(0, &["LV6-SRC", "EX4-073"]);
    let cheap = runner.place_on_field(1, "OPP-CHEAP", Some(0));
    runner.place_on_field(1, "OPP-PRICEY", Some(0));

    fire_when_attacking(&mut runner, handle);

    // Trash the single Lv6 source.
    let view = runner
        .pending_selection_view()
        .expect("source-select installs");
    let src_action =
        encode_source_select(handle.index as u16, 0).expect("Lv6 source action");
    runner.execute_action(0, src_action).expect("pick Lv6 source");

    // The per-trash delete must offer ONLY the lowest-play-cost opponent
    // permanent (OPP-CHEAP, cost 2). OPP-PRICEY (cost 9) must not be offered.
    let view = runner
        .pending_selection_view()
        .expect("per-trash lowest-cost delete select installs");
    let act = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|a| *a != PASS)
        .expect("a delete-target action");
    runner
        .execute_action(view.selecting_player, act)
        .expect("delete lowest-cost opp permanent");
    while let Some(v) = runner.pending_selection_view() {
        let a = v.valid_action_ids.first().copied().expect("action");
        runner.execute_action(v.selecting_player, a).ok();
    }

    let survivors: Vec<String> = runner
        .game
        .player(1)
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&runner.game.card_data).to_string())
        .collect();
    assert!(
        !survivors.contains(&"OPP-CHEAP".to_string()),
        "the lowest-play-cost opponent permanent must be deleted; survivors={survivors:?}"
    );
    assert!(
        survivors.contains(&"OPP-PRICEY".to_string()),
        "the higher-cost opponent permanent must survive (1 source trashed → 1 delete); \
         survivors={survivors:?}"
    );
    let _ = cheap;
}

/// Clause C tail — "if you trashed 3 cards, trash the top 2 cards of your
/// opponent's security stack". Trashing all 3 Lv6+ sources fires the tail.
#[test]
fn ex4_073_clause_c_trashes_2_opp_security_when_3_sources_trashed() {
    use digimon_engine::action::space::encode_source_select;

    let mut runner = DebugRunner::builder()
        .dsl_card("EX4-073")
        .expect("EX4-073 in embedded DSL pack")
        .add_card(source_card("LV6-A", 6))
        .add_card(source_card("LV6-B", 6))
        .add_card(source_card("LV6-C", 6))
        .add_card(opp_digimon("OPP-D", "OppDigimon", 4, 3))
        .memory(20)
        .start();

    let handle = runner.place_stack(0, &["LV6-A", "LV6-B", "LV6-C", "EX4-073"]);
    runner.place_on_field(1, "OPP-D", Some(0));
    // Seed 3 opponent security cards.
    for _ in 0..3 {
        let card = {
            let data_idx = runner
                .game
                .card_data
                .iter()
                .position(|c| c.card_id == "OPP-D")
                .expect("OPP-D card data");
            let next = runner.game.next_card_index();
            digimon_engine::card_source::CardSource::new(data_idx, 1, next)
        };
        runner.game.player_mut(1).security.push(card);
    }
    let security_before = runner.security_count(1);

    fire_when_attacking(&mut runner, handle);

    // Trash all 3 Lv6 sources.
    for src_index in 0..3u16 {
        let act = encode_source_select(handle.index as u16, src_index)
            .expect("Lv6 source action");
        runner.execute_action(0, act).expect("pick Lv6 source");
    }
    // Resolve the 3 per-trash delete prompts + tail.
    while let Some(v) = runner.pending_selection_view() {
        let a = v.valid_action_ids.first().copied().expect("action");
        runner.execute_action(v.selecting_player, a).ok();
    }

    assert_eq!(
        runner.security_count(1),
        security_before - 2,
        "trashing 3 Lv6+ sources must trash the top 2 cards of opponent security"
    );
}

/// Clause C is OPTIONAL — the player may decline the source-select entirely
/// (DCGO `isOptional: true` / `canNoSelect: true`; `min: 0`). Passing on the
/// source-select must trash no sources and trash no security.
#[test]
fn ex4_073_clause_c_player_may_decline_to_trash_any_sources() {
    use digimon_engine::action::space::PASS;

    let mut runner = DebugRunner::builder()
        .dsl_card("EX4-073")
        .expect("EX4-073 in embedded DSL pack")
        .add_card(source_card("LV6-SRC", 6))
        .add_card(opp_digimon("OPP-D", "OppDigimon", 4, 3))
        .memory(20)
        .start();

    let handle = runner.place_stack(0, &["LV6-SRC", "EX4-073"]);
    runner.place_on_field(1, "OPP-D", Some(0));
    let trash_before = runner.trash_size(0);
    let opp_battle_before = runner.battle_area_size(1);

    fire_when_attacking(&mut runner, handle);

    // Decline the source-select.
    let view = runner
        .pending_selection_view()
        .expect("source-select installs");
    assert!(
        view.valid_action_ids.contains(&PASS),
        "an optional (min: 0) source-select must offer PASS"
    );
    runner
        .execute_action(view.selecting_player, PASS)
        .expect("decline the source-select");

    assert!(
        runner.pending_selection().is_none(),
        "declining the source-select must end clause C — no per-trash prompts"
    );
    assert_eq!(
        runner.trash_size(0),
        trash_before,
        "declining must trash no digivolution sources"
    );
    assert_eq!(
        runner.battle_area_size(1),
        opp_battle_before,
        "declining must delete no opponent permanents"
    );
}
