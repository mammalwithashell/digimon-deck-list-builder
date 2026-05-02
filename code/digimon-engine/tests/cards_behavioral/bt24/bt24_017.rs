//! BT24-017 Medusamon — Digimon, Lv.6, Red, DP 11000, Cost 11.
//!
//! # Card text (cards.json)
//!
//! `<Raid>`
//! `<Progress>`
//! `<Piercing>`
//! [When Digivolving] Delete 1 of your opponent's lowest DP Digimon.
//! Then, by returning 2 cards from their trash to the bottom of the deck,
//! they play 2 [Petrification] Tokens.
//! (Digimon/White/3000 DP/[Your Turn] This Digimon can't suspend.
//!  [On Deletion] Trash your top security card.)
//! After, this Digimon gets +2000 DP for each of your opponent's Digimon
//! until their turn ends.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT24/Red/BT24_017.cs  (submodule not initialised)
//!
//! # Known engine gaps that affect these tests
//!
//! G-PRED-DP-LTE: The `dp_lte` predicate is compiled into `CompiledPredicate`
//!   but `eval_permanent_fields` in `dsl_cards/predicate.rs` does NOT evaluate
//!   it for permanents.  The "lowest-DP" filter therefore behaves as an
//!   unfiltered "any Digimon" filter until the gap closes.
//!   Tests that assert only the lowest-DP target is offered are marked
//!   `#[ignore = "pending gap: G-PRED-DP-LTE"]`.
//!
//! G-ZONE-TRASH-TO-DECK: No `return_trash_to_deck` engine API or DSL verb.
//!   The YAML delegates to `raw_rust: { fn: bt24_017_return_selected_trash_to_deck_bottom }`,
//!   which is unregistered in the test binary.  Any test that exercises the
//!   full digivolve sequence past the "delete + select trash" steps depends
//!   on this gap.  Those tests are marked
//!   `#[ignore = "pending gap: G-ZONE-TRASH-TO-DECK"]`.
//!
//! # Patterns this test covers
//! - H1 (Raid), H3 (Piercing), H6-adjacent (Progress) — declarative keywords
//! - D1-adjacent — dynamic DP modifier (+2000 × opp-Digimon-count) until opp turn ends
//! - F2-adjacent — mandatory when-digivolving delete + chained selection
//! - Token play (Petrification) — extending tokens.rs coverage

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

/// The production YAML for BT24-017, inlined at compile time from the
/// canonical location under `cards/bt24/`.
const YAML: &str = include_str!("../../../cards/bt24/BT24-017.yaml");

/// Compile BT24-017 from the production YAML and return the CompiledCard.
fn compiled_bt24_017() -> digimon_dsl::compiled::CompiledCard {
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(YAML).expect("BT24-017.yaml parses");
    let registry =
        digimon_dsl::CardRegistry::from_specs("test", &[spec]).expect("BT24-017.yaml compiles");
    registry
        .lookup("BT24-017")
        .expect("BT24-017 in registry")
        .clone()
}

/// Place BT24-017 on `player`'s field using YAML-loaded DSL card data,
/// returning its PermanentHandle.  The caller is responsible for having
/// set up a runner with `from_dsl_yaml(YAML)`.
fn place_bt24_on_field(runner: &mut DebugRunner, player: u8) -> PermanentHandle {
    runner.place_on_field(player, "BT24-017", Some(0))
}

/// Enqueue and drain BT24-017's WhenDigivolving trigger for the given handle.
fn trigger_when_digivolving(runner: &mut DebugRunner, handle: PermanentHandle) {
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(handle),
    );
    runner.game.drain_effect_queue();
}

// ─────────────────────────────────────────────────────────────────────────────
// § 1  Structural assertions
// ─────────────────────────────────────────────────────────────────────────────

/// BT24-017 must compile with exactly 3 declarative (keyword-grant) clauses
/// and exactly 1 triggered (when_digivolving) clause.
#[test]
fn bt24_017_structural_three_keywords_one_triggered_clause() {
    let card = compiled_bt24_017();

    let declarative: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Declarative(d) => Some(d),
            _ => None,
        })
        .collect();

    assert_eq!(
        declarative.len(),
        3,
        "expected exactly 3 declarative (grant_keyword) clauses"
    );
    for d in &declarative {
        assert!(
            matches!(d, CompiledDeclarativeClause::GrantKeyword { .. }),
            "all declarative clauses must be GrantKeyword"
        );
    }

    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    assert_eq!(triggered.len(), 1, "expected exactly 1 triggered clause");
    let t = triggered[0];
    assert_eq!(
        t.when,
        vec![CompiledTiming::WhenDigivolving],
        "triggered clause must fire on WhenDigivolving"
    );
    // The [When Digivolving] clause is mandatory, not optional.
    assert!(
        !t.optional,
        "when-digivolving clause is mandatory (not optional)"
    );
    assert!(
        !t.once_per_turn,
        "when-digivolving clause has no once-per-turn restriction"
    );
    assert_eq!(
        t.scope,
        CompiledScope::FaceUp,
        "triggered clause scope must be FaceUp (not Inherited)"
    );
}

/// BT24-017 must have a Digivolve alt-path (from Lv.5 at cost 3).
#[test]
fn bt24_017_has_lv5_digivolve_alt_path() {
    let card = compiled_bt24_017();
    assert!(
        card.alt_paths
            .iter()
            .any(|p| p.kind == CompiledAltPathKind::Digivolve),
        "BT24-017 must have at least one Digivolve alt-path"
    );
}

/// BT24-017 must declare Raid as a native keyword grant.
/// `CompiledDeclarativeClause::GrantKeyword.keyword` holds the keyword as a
/// `String` (the DSL's `keyword:` field value, lowercased or as-authored).
#[test]
fn bt24_017_grants_raid_keyword() {
    let card = compiled_bt24_017();
    let has_raid = card.effects.iter().any(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
            keyword, ..
        }) => keyword.eq_ignore_ascii_case("Raid"),
        _ => false,
    });
    assert!(
        has_raid,
        "BT24-017 must declare a GrantKeyword(Raid) clause"
    );
}

/// BT24-017 must declare Progress as a native keyword grant.
#[test]
fn bt24_017_grants_progress_keyword() {
    let card = compiled_bt24_017();
    let has_progress = card.effects.iter().any(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
            keyword, ..
        }) => keyword.eq_ignore_ascii_case("Progress"),
        _ => false,
    });
    assert!(
        has_progress,
        "BT24-017 must declare a GrantKeyword(Progress) clause"
    );
}

/// BT24-017 must declare Piercing as a native keyword grant.
#[test]
fn bt24_017_grants_piercing_keyword() {
    let card = compiled_bt24_017();
    let has_piercing = card.effects.iter().any(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
            keyword, ..
        }) => keyword.eq_ignore_ascii_case("Piercing"),
        _ => false,
    });
    assert!(
        has_piercing,
        "BT24-017 must declare a GrantKeyword(Piercing) clause"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// § 2  [When Digivolving] — delete step installs OppField selection
// ─────────────────────────────────────────────────────────────────────────────

/// Positive path: after triggering BT24-017's WhenDigivolving effect, a
/// pending OppField selection must be installed (the "delete lowest-DP
/// Digimon" prompt).
///
/// Note: G-PRED-DP-LTE means the filter shows all opp Digimon, not only
/// lowest-DP.  The selection *count* is not asserted here; only that the
/// selection installs as OppField.
#[test]
fn bt24_017_when_digivolving_installs_opp_field_selection() {
    use digimon_engine::selection::SelectionKind;

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT24-017 YAML loads")
        .add_card(make_test_card("OPP-DIGIMON", "OppDigimon"))
        .memory(20)
        .start();

    let bt24_handle = place_bt24_on_field(&mut runner, 0);
    runner.place_on_field(1, "OPP-DIGIMON", None);

    trigger_when_digivolving(&mut runner, bt24_handle);

    let kind = runner
        .pending_kind()
        .expect("pending selection after WhenDigivolving");
    assert_eq!(
        kind,
        SelectionKind::OppField,
        "WhenDigivolving should install an OppField selection for the delete step"
    );
}

/// Negative path: if the opponent has NO Digimon, the delete step finds no
/// valid target.  The game must not panic.
#[test]
fn bt24_017_when_digivolving_no_opp_digimon_does_not_panic() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT24-017 YAML loads")
        .memory(20)
        .start();

    let bt24_handle = place_bt24_on_field(&mut runner, 0);

    // Should not panic even with no valid targets.
    trigger_when_digivolving(&mut runner, bt24_handle);
    // no assertion required — absence of panic is the acceptance criterion
}

// ─────────────────────────────────────────────────────────────────────────────
// § 3  Lowest-DP target filter (GAP-gated)
// ─────────────────────────────────────────────────────────────────────────────

/// When multiple opponent Digimon are on the field the delete prompt must offer
/// ONLY the lowest-DP Digimon.
/// BLOCKED by G-PRED-DP-LTE: dp_lte predicate not evaluated for permanents.
#[test]
#[ignore = "pending gap: G-PRED-DP-LTE — dp_lte predicate not evaluated for permanents"]
fn bt24_017_delete_targets_only_lowest_dp_digimon() {
    let mut low_dp = make_test_card("OPP-LOW", "OppLow");
    low_dp.dp = Some(3000);
    let mut high_dp = make_test_card("OPP-HIGH", "OppHigh");
    high_dp.dp = Some(9000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("YAML loads")
        .add_card(low_dp)
        .add_card(high_dp)
        .memory(20)
        .start();

    let bt24_handle = place_bt24_on_field(&mut runner, 0);
    runner.place_on_field(1, "OPP-LOW", None);
    runner.place_on_field(1, "OPP-HIGH", None);

    trigger_when_digivolving(&mut runner, bt24_handle);

    let view = runner
        .pending_selection_view()
        .expect("selection installed");
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only the lowest-DP (3000) Digimon should be a valid delete target"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// § 4  Full sequence — delete → trash return → 2 tokens → DP boost (GAP-gated)
// ─────────────────────────────────────────────────────────────────────────────

/// Full positive sequence.
/// BLOCKED by G-ZONE-TRASH-TO-DECK.
#[test]
#[ignore = "pending gap: G-ZONE-TRASH-TO-DECK — return_trash_to_deck not implemented"]
fn bt24_017_full_sequence_two_trash_two_tokens_dp_boost() {
    use digimon_engine::action::space::PASS;
    use digimon_engine::selection::SelectionKind;

    let mut opp_digimon = make_test_card("OPP-D", "OppD");
    opp_digimon.dp = Some(4000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("YAML loads")
        .add_card(opp_digimon)
        .add_card(make_test_card("TRASH-A", "TrashA"))
        .add_card(make_test_card("TRASH-B", "TrashB"))
        .memory(20)
        .start();

    let bt24_handle = place_bt24_on_field(&mut runner, 0);
    runner.place_on_field(1, "OPP-D", None);

    // Populate opp's trash.
    for card_id in ["TRASH-A", "TRASH-B"] {
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == card_id)
            .expect("card in card_data");
        let idx = runner.game.next_card_index();
        runner.game.players[1]
            .trash
            .push(digimon_engine::card_source::CardSource::new(
                data_idx, 1, idx,
            ));
    }

    trigger_when_digivolving(&mut runner, bt24_handle);

    // Resolve the OppField delete selection.
    {
        let (action, player) = {
            let pending = runner
                .game
                .pending_selection
                .as_ref()
                .expect("delete prompt");
            (pending.valid_action_ids[0], pending.selecting_player)
        };
        runner
            .game
            .resolve_selection(player, action)
            .expect("delete");
    }
    assert_eq!(runner.battle_area_size(1), 0, "opp Digimon deleted");

    // CountCappedMultiSelect for opp trash.
    assert!(
        matches!(
            runner.pending_kind(),
            Some(SelectionKind::CountCappedMultiSelect { .. })
        ),
        "trash selection must follow delete"
    );
    for _ in 0..2 {
        let (action, player) = {
            let pending = runner.game.pending_selection.as_ref().expect("trash pick");
            (pending.valid_action_ids[0], pending.selecting_player)
        };
        runner.game.resolve_selection(player, action).expect("pick");
    }
    if let Some(pending) = runner.game.pending_selection.as_ref() {
        let player = pending.selecting_player;
        runner.game.resolve_selection(player, PASS).ok();
    }

    // After raw_rust gap step: opp trash should be empty.
    assert_eq!(runner.trash_size(1), 0, "both trash cards returned to deck");

    // 2 Petrification Tokens on opp's field.
    let token_count = runner
        .game
        .player(1)
        .battle_area
        .iter()
        .filter(|p: &&digimon_engine::permanent::Permanent| {
            p.top_card().card_kind(&runner.game.card_data) == CardKind::Token
        })
        .count();
    assert_eq!(token_count, 2, "2 Petrification Tokens must spawn");

    // +2000 per opp Digimon (2 tokens) until opp turn ends.
    assert_eq!(
        runner.effective_dp(bt24_handle),
        Some(11000 + 4000),
        "+4000 boost"
    );

    // After opp's turn ends the modifier expires.
    runner.end_turn();
    runner.end_turn();
    assert_eq!(
        runner.effective_dp(bt24_handle),
        Some(11000),
        "boost expires"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// § 5  DP modifier formula — battle_area count (unit test, no digivolve chain)
// ─────────────────────────────────────────────────────────────────────────────

/// Verify the DP modifier formula: +2000 × (opponent's battle_area count).
/// This test exercises the `card_count_in_zone(opponent, battle_area)` formula
/// directly without running the full digivolve sequence.
/// Does NOT depend on G-ZONE-TRASH-TO-DECK.
#[test]
fn bt24_017_dp_modifier_formula_scales_with_opp_digimon_count() {
    use digimon_dsl::compiled::{
        CompiledFormula, CompiledPerSelector, CompiledPlayerRef, CompiledZone,
    };
    use digimon_engine::dsl_cards::formula_eval;
    use digimon_engine::effect_context::EffectContext;

    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "Src"))
        .add_card(make_test_card("OPP-A", "OppA"))
        .add_card(make_test_card("OPP-B", "OppB"))
        .add_card(make_test_card("OPP-C", "OppC"))
        .memory(20)
        .start();

    let target = runner.place_on_field(0, "SRC", None);
    runner.place_on_field(1, "OPP-A", None);
    runner.place_on_field(1, "OPP-B", None);
    runner.place_on_field(1, "OPP-C", None);

    // The YAML's DP-modifier formula: base=0, per=card_count_in_zone(opp, battle_area), delta=2000.
    let formula = CompiledFormula::BasePerDelta {
        base: 0,
        per: CompiledPerSelector::CardCountInZoneScoped {
            of: CompiledPlayerRef::Opponent,
            zone: CompiledZone::BattleArea,
        },
        delta: 2000,
    };

    let src_card = runner.game.players[0].battle_area[0].top_card().handle();
    let ctx = EffectContext::new(&mut runner.game, src_card, Some(target), 0);
    let value = formula_eval::evaluate(&formula, &ctx, target);

    assert_eq!(value, 6000, "+2000 × 3 opp Digimon must equal 6000");
}

/// Formula yields 0 when the opponent has no Digimon.
#[test]
fn bt24_017_dp_modifier_formula_zero_with_no_opp_digimon() {
    use digimon_dsl::compiled::{
        CompiledFormula, CompiledPerSelector, CompiledPlayerRef, CompiledZone,
    };
    use digimon_engine::dsl_cards::formula_eval;
    use digimon_engine::effect_context::EffectContext;

    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "Src"))
        .memory(20)
        .start();

    let target = runner.place_on_field(0, "SRC", None);

    let formula = CompiledFormula::BasePerDelta {
        base: 0,
        per: CompiledPerSelector::CardCountInZoneScoped {
            of: CompiledPlayerRef::Opponent,
            zone: CompiledZone::BattleArea,
        },
        delta: 2000,
    };

    let src_card = runner.game.players[0].battle_area[0].top_card().handle();
    let ctx = EffectContext::new(&mut runner.game, src_card, Some(target), 0);
    let value = formula_eval::evaluate(&formula, &ctx, target);

    assert_eq!(value, 0, "+2000 × 0 opp Digimon must equal 0");
}

// ─────────────────────────────────────────────────────────────────────────────
// § 6  Opp has <2 trash — token play is gated (GAP-gated)
// ─────────────────────────────────────────────────────────────────────────────

/// When the opponent has <2 trash cards the "by returning 2" cost cannot be
/// paid, so no tokens should spawn.
/// BLOCKED by G-ZONE-TRASH-TO-DECK.
#[test]
#[ignore = "pending gap: G-ZONE-TRASH-TO-DECK — cost gate not enforceable without trash-to-deck primitive"]
fn bt24_017_less_than_two_trash_skips_token_play() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("YAML loads")
        .add_card(make_test_card("OPP-D", "OppD"))
        .memory(20)
        .start();

    let bt24_handle = place_bt24_on_field(&mut runner, 0);
    runner.place_on_field(1, "OPP-D", None);
    // Opponent has 0 trash cards.

    trigger_when_digivolving(&mut runner, bt24_handle);

    {
        let (action, player) = {
            let pending = runner.game.pending_selection.as_ref().expect("delete");
            (pending.valid_action_ids[0], pending.selecting_player)
        };
        runner
            .game
            .resolve_selection(player, action)
            .expect("delete resolves");
    }

    let token_count = runner
        .game
        .player(1)
        .battle_area
        .iter()
        .filter(|p: &&digimon_engine::permanent::Permanent| {
            p.top_card().card_kind(&runner.game.card_data) == CardKind::Token
        })
        .count();
    assert_eq!(
        token_count, 0,
        "no tokens when opp cannot pay the return-2 cost"
    );
}
