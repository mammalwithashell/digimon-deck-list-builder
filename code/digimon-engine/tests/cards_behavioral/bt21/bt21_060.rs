//! BT21-060 Destromon — Digimon, Lv.5, Black, DP 10000, Cost 10.
//! Traits: [Unknown, LIBERATOR]. Form: Ultimate.
//! Standard digivolve: Lv.4 Black / Cost 5 (printed evo_costs).
//! Alt-source (printed xros_req): "[Digivolve] [Vemmon]: Cost 6".
//!
//! # Card text (data/card_bundles/BT21-060.md — official Bandai DB, verbatim)
//!
//! [When Digivolving] Until your opponent's turn ends, their effects can't
//! trash this Digimon's stacked cards. Then, to 1 of your opponent's Digimon,
//! ＜De-Digivolve 1＞ for every 2 [Vemmon] in this Digimon's digivolution cards.
//! [All Turns] When this Digimon would leave the battle area, you may play 1
//! [Vemmon] from its digivolution cards without paying the cost.
//!
//! Inherited: [Opponent's Turn] [Once Per Turn] When one of your opponent's
//! Digimon attacks, by returning 2 [Vemmon] from this Digimon's digivolution
//! cards to the bottom of the deck, end that attack.
//!
//! Official Q&A: the stack-trashing immunity covers BOTH top-peel and
//! bottom-source trashing (the whole stack), not just one mechanic.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT21/Black/BT21_060.cs
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - D3-adjacent: name-gated alt-path digivolve (`from: { name_is: "Vemmon" }`)
//! - F6 Effect immunity / IMMUNE_FROM_X (ImmuneFromStackTrashing, When Digivolving)
//! - Count-scaled De-Digivolve via `floor_div` / `source_stack_count` formula
//!   (BT20-021 idiom)
//! - F3 Replacement effect (non-cancelling would-leave, BT25-025 <Decode> idiom)
//! - Mandatory target selection that degrades gracefully with 0/1 Vemmon sources
//!
//! # Known gap — BLOCKED clause (see BT21-060.yaml header for full derivation)
//!
//! The inherited [Opponent's Turn][OPT] "return 2 [Vemmon] sources to deck
//! bottom, end attack" clause is OMITTED from the YAML. Empirically verified
//! (via a throwaway probe fixture, since removed): `select_own_sources` with
//! `min: 2, max: 2` and exactly 1 matching candidate silently drops the
//! player's pick with no resulting effect once candidates run out before
//! `min` is reached (`install_source_multi_selection`, `effect_context/
//! selections.rs`) — an illusory-choice / no-approximations violation if
//! shipped. No DSL predicate exists to pre-gate the clause on "count of own
//! digivolution sources named X >= 2" the way DCGO's `CanActivateCondition`
//! does, and no CardList-compatible single-card "return this digivolution
//! source to deck" verb exists to combine with the safe `select_materials`
//! (max-only) alternative. Gap-id: G-DSL-SELECT-OWN-SOURCES-EXACT-N-GATE.
//! No tests target this clause — there is nothing implemented to test.

#![allow(dead_code)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledScope,
    CompiledTiming, CompiledTriggeredClause,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, ModifierType, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "BT21-060";

// ─── Fixture helpers ─────────────────────────────────────────────────────────

/// A card named "Vemmon" (matches the printed xros_req alt-path AND the
/// `name_is: "Vemmon"` source-count / material filters used by every clause).
/// Deliberately a synthetic `TEST-*`-style id (no real Vemmon printing is
/// implemented as DSL yet) — only the NAME matters to BT21-060's own clauses.
fn vemmon(id: &str) -> CardData {
    let mut c = make_test_card(id, "Vemmon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(2000);
    c.colors = vec![digimon_engine::enums::CardColor::Black];
    c
}

/// A same-shape Digimon card that is NOT named Vemmon (used to prove the
/// name filter excludes non-Vemmon sources from the count/material pool).
fn plain_source(id: &str) -> CardData {
    let mut c = make_test_card(id, "Not Vemmon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(2000);
    c
}

/// A generic opponent Digimon target for the De-Digivolve clause.
fn opp_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(5000);
    c
}

fn filler(id: &str) -> CardData {
    make_test_card(id, "Filler")
}

/// Base runner: Destromon registered via the embedded DSL pack, in hand, plus
/// a generous filler deck so `enter_main_phase` / turn machinery never decks
/// out mid-test.
fn base() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT21-060 YAML parses and compiles")
        .add_card(vemmon("VEM-1"))
        .add_card(vemmon("VEM-2"))
        .add_card(vemmon("VEM-3"))
        .add_card(vemmon("VEM-4"))
        .add_card(plain_source("PLAIN-1"))
        .add_card(opp_digimon("OPP-A"))
        .add_card(filler("DECK-PAD"))
        .hand(0, &[CARD_ID])
        .deck(0, &["DECK-PAD"; 20])
        .deck(1, &["DECK-PAD"; 20])
        .memory(20)
        .start()
}

/// Digivolve Destromon (hand index 0, since `.hand(0, &[CARD_ID])` puts it
/// alone in hand) from `base_field_index` onto the field, driving the REAL
/// `digivolve_from_hand` action path so `[When Digivolving]` fires naturally.
/// The alt-path is purely name-gated (`from: { name_is: "Vemmon" }`), so the
/// base permanent's CURRENT TOP CARD must be named "Vemmon" — no synthetic
/// `evo_costs` are needed (confirmed via `all_digivolve_routes_for_card`,
/// which consults DSL alt_paths independently of printed evo-cost circles).
fn digivolve_destromon_onto(runner: &mut DebugRunner, base_field_index: usize) {
    let hand_idx = 0;
    assert!(
        runner
            .game
            .digivolve_from_hand(0, hand_idx, base_field_index, PlaySource::ByHand),
        "Destromon must be able to digivolve onto a Vemmon-topped base via the \
         printed xros_req alt-path"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn bt21_060_loads_from_embedded_pack() {
    let runner = base();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("BT21-060 must be present in the embedded DSL pack");
    assert_eq!(card.card, CARD_ID);
    assert_eq!(card.level, Some(5));
    assert_eq!(card.cost, Some(10));
    assert_eq!(card.dp, Some(10000));
    assert!(
        card.traits.iter().any(|t| t.eq_ignore_ascii_case("LIBERATOR")),
        "BT21-060 must carry the LIBERATOR trait"
    );
}

#[test]
fn bt21_060_has_vemmon_named_alt_path_cost_6() {
    let runner = base();
    let card = runner.compiled_card(CARD_ID).expect("compiled card present");

    let has_vemmon_alt = card.alt_paths.iter().any(|p| {
        matches!(p.kind, CompiledAltPathKind::Digivolve)
            && matches!(p.cost, Some(CompiledCost::Literal(6)))
    });
    assert!(
        has_vemmon_alt,
        "BT21-060 must carry a Vemmon-named digivolve alt-path at cost 6 \
         (printed xros_req: \"[Digivolve] [Vemmon]: Cost 6\")"
    );
}

#[test]
fn bt21_060_has_two_when_digivolving_triggered_clauses() {
    let runner = base();
    let card = runner.compiled_card(CARD_ID).expect("compiled card present");

    let when_digivolving: Vec<&CompiledTriggeredClause> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::WhenDigivolving) => {
                Some(t)
            }
            _ => None,
        })
        .collect();

    assert_eq!(
        when_digivolving.len(),
        2,
        "BT21-060 must carry exactly two [When Digivolving] triggered clauses \
         (stack-immunity grant + De-Digivolve), got {}",
        when_digivolving.len()
    );
    for clause in &when_digivolving {
        assert_eq!(clause.scope, CompiledScope::FaceUp);
        assert!(
            !clause.optional,
            "neither [When Digivolving] clause is optional (DCGO isOptional: false)"
        );
        assert!(
            !clause.once_per_turn,
            "neither [When Digivolving] clause is [Once Per Turn] (DCGO maxCountPerTurn: -1)"
        );
    }
}

#[test]
fn bt21_060_has_would_leave_replacement_clause() {
    let runner = base();
    let card = runner.compiled_card(CARD_ID).expect("compiled card present");

    let has_replacement = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::Replacement { .. })
        )
    });
    assert!(
        has_replacement,
        "BT21-060 must carry a would-leave replacement clause for the \
         [All Turns] free-Vemmon-play text"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 2 — Condition gating: stack-trash immunity + De-Digivolve fire
//              unconditionally on ANY digivolve (no Vemmon-count gate on the
//              clause itself — only the DE-DIGIVOLVE DEGREE scales)
// ═══════════════════════════════════════════════════════════════════════════

/// Positive: digivolving Destromon onto a Vemmon-topped base with an
/// opponent Digimon present installs the mandatory opponent-target selection
/// (the [When Digivolving] clauses fire regardless of Vemmon-source count).
#[test]
fn bt21_060_when_digivolving_installs_mandatory_target_selection_with_opponent_present() {
    let mut runner = base();
    let base_idx = runner.place_on_field(0, "VEM-1", Some(0));
    runner.place_on_field(1, "OPP-A", None);

    digivolve_destromon_onto(&mut runner, base_idx.index as usize);

    assert!(
        runner.pending_selection().is_some(),
        "the mandatory opponent-Digimon target selection must install after \
         digivolving, even with only 0 additional Vemmon sources beneath \
         (DCGO CanActivateCondition has no Vemmon-count gate)"
    );
}

/// Negative: with NO opponent Digimon on the field, the mandatory target
/// selection has zero legal candidates and auto-skips (no pending selection,
/// no crash) — matching DCGO's `canNoSelect: false, maxCount: 1` with an
/// empty candidate pool.
#[test]
fn bt21_060_when_digivolving_no_opponent_digimon_auto_skips_target_selection() {
    let mut runner = base();
    let base_idx = runner.place_on_field(0, "VEM-1", Some(0));
    // No opponent Digimon placed.

    digivolve_destromon_onto(&mut runner, base_idx.index as usize);
    let _ = runner.auto_resolve();

    assert!(
        runner.pending_selection().is_none(),
        "with no opponent Digimon, the mandatory target selection must \
         auto-skip rather than leave a dangling / stuck prompt"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 3 — Behavioral: stack-trash immunity modifier
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn bt21_060_when_digivolving_grants_immune_from_stack_trashing() {
    let mut runner = base();
    let base_idx = runner.place_on_field(0, "VEM-1", Some(0));
    runner.place_on_field(1, "OPP-A", None);

    digivolve_destromon_onto(&mut runner, base_idx.index as usize);
    let _ = runner.auto_resolve();

    let destromon = PermanentHandle {
        player: 0,
        index: base_idx.index,
    };
    assert!(
        runner
            .modifiers()
            .has(destromon, ModifierType::ImmuneFromStackTrashing),
        "Destromon must gain ImmuneFromStackTrashing immediately on digivolving"
    );
}

#[test]
fn bt21_060_immune_from_stack_trashing_blocks_top_source_trash() {
    let mut runner = base();
    let base_idx = runner.place_on_field(0, "VEM-1", Some(0));
    runner.place_on_field(1, "OPP-A", None);

    digivolve_destromon_onto(&mut runner, base_idx.index as usize);
    let _ = runner.auto_resolve();

    let destromon = PermanentHandle {
        player: 0,
        index: base_idx.index,
    };
    let stack_len_before = runner.game.players[0].battle_area[destromon.index as usize]
        .card_sources
        .len();

    // Directly exercise the engine's stack-peel primitive (the mutation
    // the modifier is designed to block), mirroring `track_c_gain_gates.rs`'s
    // `immune_from_stack_trashing_blocks_trash_top_source` idiom.
    let trashed = {
        let mut ctx = digimon_engine::effect_context::EffectContext::new(
            &mut runner.game,
            digimon_engine::card_source::CardHandle(0),
            None,
            1,
        );
        ctx.trash_top_source(destromon)
    };

    assert!(
        !trashed,
        "ImmuneFromStackTrashing must block trash_top_source on Destromon"
    );
    assert_eq!(
        runner.game.players[0].battle_area[destromon.index as usize]
            .card_sources
            .len(),
        stack_len_before,
        "the stack must be unchanged after the blocked trash attempt"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 4 — Behavioral: count-scaled De-Digivolve
// ═══════════════════════════════════════════════════════════════════════════

/// Drive past a `TriggerOrder` prompt (installed because BT21-060 has two
/// simultaneous [When Digivolving] clauses) by picking the first legal
/// action, however many times one installs. Order between the two
/// [When Digivolving] clauses does not affect either clause's own outcome
/// (the immunity grant and the De-Digivolve target/degree are independent),
/// so picking the first legal action at each TriggerOrder prompt is safe.
fn drive_past_trigger_order(runner: &mut DebugRunner) {
    let mut guard = 0;
    while let Some(kind) = runner.pending_kind() {
        if kind != SelectionKind::TriggerOrder {
            break;
        }
        guard += 1;
        assert!(guard < 8, "TriggerOrder loop did not terminate");
        let action = runner
            .pending_selection_view()
            .expect("TriggerOrder view present")
            .valid_action_ids[0];
        runner
            .execute_action(0, action)
            .expect("resolve TriggerOrder prompt");
    }
}

/// 0 additional Vemmon sources beneath the Vemmon-named top (the digivolve
/// base itself becomes 1 source once Destromon stacks on top — but the
/// filter counts ALL sources, so 1 Vemmon source → floor(1/2) = 0).
#[test]
fn bt21_060_one_vemmon_source_de_digivolves_zero() {
    let mut runner = base();
    let base_idx = runner.place_on_field(0, "VEM-1", Some(0));
    // Give the opp target 1 source to observe for de-digivolve.
    let opp = runner.place_stack(1, &["PLAIN-1", "OPP-A"]);

    digivolve_destromon_onto(&mut runner, base_idx.index as usize);
    drive_past_trigger_order(&mut runner);

    let target_sources_before = runner.game.players[1].battle_area[opp.index as usize]
        .card_sources
        .len();

    let view = runner
        .pending_selection_view()
        .expect("mandatory opponent target selection installs");
    assert_eq!(view.kind, SelectionKind::OppField);
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("select the opponent target");
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.game.players[1].battle_area[opp.index as usize]
            .card_sources
            .len(),
        target_sources_before,
        "floor(1 Vemmon source / 2) = 0 — no De-Digivolve should occur"
    );
}

/// 4 Vemmon sources beneath the top (after digivolving Destromon onto a
/// 4-Vemmon stack) → floor(4/2) = 2 De-Digivolve applications on the target.
#[test]
fn bt21_060_four_vemmon_sources_de_digivolves_two() {
    let mut runner = base();
    let base_idx = runner.place_stack(0, &["VEM-1", "VEM-2", "VEM-3", "VEM-4"]);
    let opp = runner.place_stack(1, &["PLAIN-1", "PLAIN-1", "OPP-A"]);

    digivolve_destromon_onto(&mut runner, base_idx.index as usize);
    drive_past_trigger_order(&mut runner);

    let destromon = PermanentHandle {
        player: 0,
        index: base_idx.index,
    };
    let vemmon_sources = runner.game.players[0].battle_area[destromon.index as usize]
        .card_sources
        .iter()
        .filter(|c| c.card_id(&runner.game.card_data) != CARD_ID)
        .count();
    assert_eq!(
        vemmon_sources, 4,
        "sanity: exactly 4 Vemmon-named cards must sit beneath Destromon's top"
    );

    let target_sources_before = runner.game.players[1].battle_area[opp.index as usize]
        .card_sources
        .len();
    assert_eq!(
        target_sources_before, 3,
        "sanity: the target must carry 3 card_sources (2 PLAIN sources + its own top) \
         before De-Digivolve"
    );

    let view = runner
        .pending_selection_view()
        .expect("mandatory opponent target selection installs");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("select the opponent target");
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.game.players[1].battle_area[opp.index as usize]
            .card_sources
            .len(),
        target_sources_before - 2,
        "floor(4 Vemmon sources / 2) = 2 — exactly 2 De-Digivolve pops must occur"
    );
}

/// Non-Vemmon sources are NOT counted: 2 Vemmon + 2 plain sources beneath
/// the top → still floor(2/2) = 1, not floor(4/2) = 2.
#[test]
fn bt21_060_non_vemmon_sources_not_counted() {
    let mut runner = base();
    // Last element is the base's CURRENT TOP CARD (must be Vemmon-named to
    // satisfy the alt-path); the rest become sources beneath it.
    let base_idx = runner.place_stack(0, &["PLAIN-1", "VEM-1", "PLAIN-1", "VEM-2"]);
    let opp = runner.place_stack(1, &["PLAIN-1", "OPP-A"]);

    digivolve_destromon_onto(&mut runner, base_idx.index as usize);
    drive_past_trigger_order(&mut runner);

    let target_sources_before = runner.game.players[1].battle_area[opp.index as usize]
        .card_sources
        .len();

    let view = runner
        .pending_selection_view()
        .expect("mandatory opponent target selection installs");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("select the opponent target");
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.game.players[1].battle_area[opp.index as usize]
            .card_sources
            .len(),
        target_sources_before - 1,
        "only the 2 Vemmon-named sources count → floor(2/2)=1 De-Digivolve, \
         not floor(4/2)=2"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 5 — Behavioral: [All Turns] would-leave free Vemmon play
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn bt21_060_would_leave_offers_optional_replacement_with_vemmon_source() {
    let mut runner = base();
    let base_idx = runner.place_stack(0, &["VEM-1", "VEM-2"]);
    runner.place_on_field(1, "OPP-A", None);

    digivolve_destromon_onto(&mut runner, base_idx.index as usize);
    let _ = runner.auto_resolve();

    let destromon = PermanentHandle {
        player: 0,
        index: base_idx.index,
    };

    runner
        .game
        .delete_permanent_with_cause(destromon, digimon_engine::replacement::ReplacementCause::OpponentEffect);

    let view = runner
        .pending_selection_view()
        .expect("the would-leave replacement must offer an accept/decline prompt");
    assert_eq!(view.kind, SelectionKind::Replacement);
    assert!(
        runner.pending_is_optional(),
        "the [All Turns] free-play clause is a MAY (optional replacement)"
    );
}

#[test]
fn bt21_060_would_leave_accepting_plays_vemmon_source_free_and_leave_still_proceeds() {
    let mut runner = base();
    let base_idx = runner.place_stack(0, &["VEM-1", "VEM-2"]);
    runner.place_on_field(1, "OPP-A", None);

    digivolve_destromon_onto(&mut runner, base_idx.index as usize);
    let _ = runner.auto_resolve();

    let destromon = PermanentHandle {
        player: 0,
        index: base_idx.index,
    };
    let field_size_before = runner.battle_area_size(0);
    let memory_before = runner.memory();

    runner
        .game
        .delete_permanent_with_cause(destromon, digimon_engine::replacement::ReplacementCause::OpponentEffect);

    let view = runner
        .pending_selection_view()
        .expect("replacement prompt installs");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("accept the free-play replacement");
    let _ = runner.auto_resolve();

    // Destromon still leaves (this replacement never cancels the removal) —
    // battle area count nets to the same size: -1 (Destromon leaves) +1
    // (a Vemmon source is played as a new permanent) = unchanged.
    assert_eq!(
        runner.battle_area_size(0),
        field_size_before,
        "Destromon leaves AND a [Vemmon] source is played free — net field size unchanged"
    );
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_name(&runner.game.card_data) == "Vemmon"),
        "a [Vemmon] permanent must now be on the field"
    );
    assert_eq!(
        runner.memory(),
        memory_before,
        "the [Vemmon] source is played WITHOUT paying its cost — memory unchanged"
    );
    assert!(
        !runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID),
        "Destromon itself must have actually left the battle area"
    );
}

#[test]
fn bt21_060_would_leave_declining_lets_it_leave_with_no_free_play() {
    let mut runner = base();
    let base_idx = runner.place_stack(0, &["VEM-1", "VEM-2"]);
    runner.place_on_field(1, "OPP-A", None);

    digivolve_destromon_onto(&mut runner, base_idx.index as usize);
    let _ = runner.auto_resolve();

    let destromon = PermanentHandle {
        player: 0,
        index: base_idx.index,
    };
    let memory_before = runner.memory();

    runner
        .game
        .delete_permanent_with_cause(destromon, digimon_engine::replacement::ReplacementCause::OpponentEffect);

    let view = runner
        .pending_selection_view()
        .expect("replacement prompt installs");
    assert!(runner.pending_is_optional());
    runner
        .execute_action(0, digimon_engine::action::space::PASS)
        .expect("decline the free-play replacement");
    let _ = runner.auto_resolve();

    assert!(
        !runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID),
        "Destromon must still leave the battle area after declining"
    );
    assert!(
        !runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_name(&runner.game.card_data) == "Vemmon"),
        "declining must NOT play any [Vemmon] source"
    );
    assert_eq!(
        runner.memory(),
        memory_before,
        "declining spends no memory"
    );
}

#[test]
fn bt21_060_would_leave_no_vemmon_source_offers_no_replacement() {
    // Digivolve via the STANDARD (printed evo-cost) path onto a plain Lv.4
    // Black base — no Vemmon-named card anywhere in the stack — so after
    // digivolving there is genuinely no legal material for the free-play
    // clause to offer.
    let mut c = make_test_card("BLACK4-BASE", "Not Vemmon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(3000);
    c.colors = vec![digimon_engine::enums::CardColor::Black];

    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT21-060 compiles")
        .add_card(c)
        .add_card(opp_digimon("OPP-A"))
        .add_card(filler("DECK-PAD"))
        .hand(0, &["BT21-060"])
        .deck(0, &["DECK-PAD"; 20])
        .deck(1, &["DECK-PAD"; 20])
        .memory(20)
        .start();

    let base_idx = runner.place_on_field(0, "BLACK4-BASE", Some(0));
    runner.place_on_field(1, "OPP-A", None);

    digivolve_destromon_onto(&mut runner, base_idx.index as usize);
    let _ = runner.auto_resolve();

    let destromon = PermanentHandle {
        player: 0,
        index: base_idx.index,
    };

    runner
        .game
        .delete_permanent_with_cause(destromon, digimon_engine::replacement::ReplacementCause::OpponentEffect);
    let _ = runner.auto_resolve();

    // With no Vemmon source to play, `select_material` (optional: implicit
    // via the replacement's own `optional: true`) has nothing to offer;
    // the leave proceeds with no material played and no dangling prompt.
    assert!(
        runner.pending_selection().is_none(),
        "with no [Vemmon] source available, no selection should be left pending"
    );
    assert!(
        !runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID),
        "Destromon must still leave (no Vemmon source available to play)"
    );
}

/// The would-leave clause is [All Turns] — it must also fire when Destromon
/// would leave during the OPPONENT's turn (not scoped to `your_turn`).
#[test]
fn bt21_060_would_leave_fires_on_opponents_turn_too() {
    let mut runner = base();
    let base_idx = runner.place_stack(0, &["VEM-1"]);
    runner.place_on_field(1, "OPP-A", None);

    digivolve_destromon_onto(&mut runner, base_idx.index as usize);
    let _ = runner.auto_resolve();

    runner.end_turn(); // now player 1's turn

    let destromon = PermanentHandle {
        player: 0,
        index: base_idx.index,
    };
    runner
        .game
        .delete_permanent_with_cause(destromon, digimon_engine::replacement::ReplacementCause::OpponentEffect);

    let view = runner.pending_selection_view();
    assert!(
        view.is_some(),
        "[All Turns] means the would-leave replacement must offer even on \
         the opponent's turn"
    );
    assert_eq!(view.unwrap().kind, SelectionKind::Replacement);
}
