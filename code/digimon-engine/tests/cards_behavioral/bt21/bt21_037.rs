//! BT21-037 Lighdramon — Digimon, Lv.4, Blue/Green, DP 6000, Cost 6.
//! Traits: Beast
//! Evo: Lv.3 / cost 3
//!
//! # Card text (cards.json)
//!
//! ```text
//! ＜Piercing＞ (When this Digimon attacks and deletes an opponent's Digimon
//! and survives the battle, it performs any security checks it normally would.)
//! ＜Armor Purge＞ (When this Digimon would be deleted, you may trash the top
//! card of this Digimon to prevent that deletion.)
//! [When Digivolving] Suspend 1 of your opponent's Digimon. Then, this Digimon
//! gets +2000 DP until your opponent's turn ends.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT21/Blue/BT21_037.cs
//!
//! # Patterns this test covers
//! - H3: Piercing printed-keyword grant via `kind: grant_keyword`
//! - Armor Purge: grant_keyword ArmorPurge (same as BT24-018, P-137)
//! - [When Digivolving] mandatory select + suspend opp Digimon + +2000 DP self
//!   with end_of_opponents_turn expiry (BT13-060 / BT24-101 pattern)
//!
//! # Build note
//! `dsl_card("BT21-037")` requires build.rs to scan `cards/bt21/` (Phase 1c+).
//! Tests below use `from_dsl_yaml(YAML)` with the YAML loaded via `include_str!`.

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledScope,
    CompiledTiming,
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::{SelectionKind, TriggerSource};

// ─── Inlined production YAML ──────────────────────────────────────────────────

const YAML: &str = include_str!("../../../cards/bt21/BT21-037.yaml");

// ─── Card fixtures ────────────────────────────────────────────────────────────

/// A plain Digimon for the opponent's field (suspendable).
fn opp_digimon(id: &str) -> digimon_engine::card_data::CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.colors = vec![CardColor::Red];
    c
}

/// Build a runner with Lighdramon available from the embedded YAML.
fn lighdramon_runner() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT21-037 YAML must parse and compile")
        .memory(10)
        .build()
}

// ─── §1 Structural assertions ─────────────────────────────────────────────────

/// BT21-037 must compile successfully from production YAML.
#[test]
fn bt21_037_yaml_parses_and_compiles() {
    let runner = lighdramon_runner();
    let compiled = runner
        .compiled_card("BT21-037")
        .expect("BT21-037 must compile");
    assert_eq!(compiled.card, "BT21-037");
}

/// BT21-037's `xros_req` is "[Digivolve] [Veemon]: Cost 2".
/// It must be preserved as an alternate digivolve path in addition to the
/// standard Lv.3 cost-3 path from `evo_costs`.
#[test]
fn bt21_037_has_veemon_cost_2_alt_path() {
    let runner = lighdramon_runner();
    let compiled = runner
        .compiled_card("BT21-037")
        .expect("BT21-037 must compile");

    let veemon_path = compiled.alt_paths.iter().find(|path| {
        path.kind == CompiledAltPathKind::Digivolve
            && path.cost == Some(CompiledCost::Literal(2))
            && path
                .from
                .as_ref()
                .is_some_and(|from| from.name_contains.as_deref() == Some("Veemon"))
    });

    assert!(
        veemon_path.is_some(),
        "BT21-037 must include the printed [Digivolve] [Veemon]: Cost 2 alt path; alt_paths={:?}",
        compiled.alt_paths
    );
}

/// Clause 1: a GrantKeyword(Piercing) declarative clause must exist with FaceUp scope.
#[test]
fn bt21_037_has_piercing_grant_keyword_clause() {
    let runner = lighdramon_runner();
    let compiled = runner
        .compiled_card("BT21-037")
        .expect("BT21-037 must compile");

    let found = compiled.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                scope,
                ..
            }) if keyword.eq_ignore_ascii_case("Piercing") && *scope == CompiledScope::FaceUp
        )
    });
    assert!(
        found,
        "BT21-037 must have a FaceUp-scope GrantKeyword(Piercing) clause"
    );
}

/// Clause 2: a GrantKeyword(ArmorPurge) declarative clause must exist with FaceUp scope.
#[test]
fn bt21_037_has_armor_purge_grant_keyword_clause() {
    let runner = lighdramon_runner();
    let compiled = runner
        .compiled_card("BT21-037")
        .expect("BT21-037 must compile");

    let found = compiled.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                scope,
                ..
            }) if keyword.eq_ignore_ascii_case("ArmorPurge") && *scope == CompiledScope::FaceUp
        )
    });
    assert!(
        found,
        "BT21-037 must have a FaceUp-scope GrantKeyword(ArmorPurge) clause"
    );
}

/// Clause 3: exactly one WhenDigivolving triggered clause, not optional, no OPT.
#[test]
fn bt21_037_has_when_digivolving_triggered_clause() {
    let runner = lighdramon_runner();
    let compiled = runner
        .compiled_card("BT21-037")
        .expect("BT21-037 must compile");

    let wd = compiled
        .effects
        .iter()
        .find_map(|c| {
            if let CompiledClause::Triggered(t) = c {
                if t.when.contains(&CompiledTiming::WhenDigivolving)
                    && t.scope == CompiledScope::FaceUp
                {
                    return Some(t);
                }
            }
            None
        })
        .expect("BT21-037 must have a WhenDigivolving triggered clause");

    assert!(
        !wd.optional,
        "WhenDigivolving clause must not be optional (no 'you may')"
    );
    assert!(
        !wd.once_per_turn,
        "WhenDigivolving clause has no [Once Per Turn]"
    );
}

/// Total clause count: 2 declarative (Piercing, ArmorPurge) + 1 triggered (WhenDigivolving).
#[test]
fn bt21_037_has_three_clauses() {
    let runner = lighdramon_runner();
    let compiled = runner
        .compiled_card("BT21-037")
        .expect("BT21-037 must compile");

    assert_eq!(
        compiled.effects.len(),
        3,
        "BT21-037 must have exactly 3 clauses: Piercing, ArmorPurge, WhenDigivolving"
    );
}

// ─── §2 Behavioral — [When Digivolving] suspend + DP buff ────────────────────
//
// NOTE: WhenDigivolving effects are not auto-dispatched from the digivolution
// stack in the Rust engine (G-WHEN-DIGIVOLVING-DISPATCH). These tests manually
// enqueue the WhenDigivolving trigger on the Lighdramon permanent, matching the
// test pattern in bt20_016.rs and bt21_013.rs.

/// Positive: when WhenDigivolving fires and opponent has a Digimon,
/// a pending OppField selection is installed.
#[test]
fn bt21_037_when_digivolving_installs_opp_selection_with_opp_digimon() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT21-037 YAML must parse")
        .add_card(opp_digimon("OPP-A"))
        .memory(10)
        .build();

    let lighdramon = runner.place_on_field(0, "BT21-037", None);
    let _opp = runner.place_on_field(1, "OPP-A", None);

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(lighdramon),
    );
    runner.game.drain_effect_queue();

    let view = runner
        .pending_selection_view()
        .expect("a selection must be pending after WhenDigivolving fires with opp Digimon");

    use digimon_engine::selection::SelectionKind;
    assert_eq!(
        view.kind,
        SelectionKind::OppField,
        "selection must be OppField to suspend 1 of opponent's Digimon"
    );
}

/// After selecting the opponent Digimon, it must be suspended.
#[test]
fn bt21_037_when_digivolving_suspends_selected_opp_digimon() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT21-037 YAML must parse")
        .add_card(opp_digimon("OPP-A"))
        .memory(10)
        .build();

    let lighdramon = runner.place_on_field(0, "BT21-037", None);
    let opp_h = runner.place_on_field(1, "OPP-A", None);

    // Pre-condition: opp Digimon is NOT suspended.
    assert!(
        !runner.game.players[opp_h.player as usize].battle_area[opp_h.index as usize].is_suspended,
        "opponent Digimon must start unsuspended"
    );

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(lighdramon),
    );
    runner.game.drain_effect_queue();

    // Accept the selection.
    runner.auto_resolve().expect("auto_resolve must succeed");

    assert!(
        runner.game.players[opp_h.player as usize].battle_area[opp_h.index as usize].is_suspended,
        "selected opponent Digimon must be suspended after WhenDigivolving effect resolves"
    );
}

/// After the effect resolves, Lighdramon itself gets +2000 DP.
#[test]
fn bt21_037_when_digivolving_grants_2000_dp_to_self() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT21-037 YAML must parse")
        .add_card(opp_digimon("OPP-A"))
        .memory(10)
        .build();

    let lighdramon = runner.place_on_field(0, "BT21-037", None);
    let _opp = runner.place_on_field(1, "OPP-A", None);

    let base_dp = runner
        .effective_dp(lighdramon)
        .expect("Lighdramon must have DP");

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(lighdramon),
    );
    runner.game.drain_effect_queue();
    runner.auto_resolve().expect("auto_resolve must succeed");

    let after_dp = runner
        .effective_dp(lighdramon)
        .expect("Lighdramon must still be on field");
    assert_eq!(
        after_dp,
        base_dp + 2000,
        "Lighdramon must have +2000 DP after WhenDigivolving effect resolves; base={base_dp}"
    );
}

/// The +2000 DP buff expires at end of opponent's turn (end_of_opponents_turn expiry).
/// Cycle: P0 fires WhenDigivolving → buff active → P0 ends turn (P1's turn starts) →
/// P1 ends turn → buff must be gone.
#[test]
fn bt21_037_dp_buff_expires_at_end_of_opponents_turn() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT21-037 YAML must parse")
        .add_card(opp_digimon("OPP-A"))
        .memory(10)
        .build();

    let lighdramon = runner.place_on_field(0, "BT21-037", None);
    let _opp = runner.place_on_field(1, "OPP-A", None);

    let base_dp = runner
        .effective_dp(lighdramon)
        .expect("Lighdramon must have DP");

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(lighdramon),
    );
    runner.game.drain_effect_queue();
    runner.auto_resolve().expect("auto_resolve must succeed");

    // Buff is active on P0's turn.
    let buffed_dp = runner
        .effective_dp(lighdramon)
        .expect("Lighdramon on field");
    assert_eq!(
        buffed_dp,
        base_dp + 2000,
        "+2000 DP must be active during P0's turn; base={base_dp}"
    );

    // End P0's turn → P1's turn starts.
    runner.end_turn();

    // Buff must still be active during P1's turn (expiry is END of opponent's turn).
    let during_opp_turn_dp = runner
        .effective_dp(lighdramon)
        .expect("Lighdramon persists during P1 turn");
    assert_eq!(
        during_opp_turn_dp,
        base_dp + 2000,
        "+2000 DP must persist during opponent's turn (expiry is END of opponent's turn)"
    );

    // End P1's turn → buff expires.
    runner.end_turn();

    let after_opp_turn_dp = runner
        .effective_dp(lighdramon)
        .expect("Lighdramon persists after P1 turn ends");
    assert_eq!(
        after_opp_turn_dp, base_dp,
        "+2000 DP must expire at end of opponent's turn"
    );
}

/// Negative: if opponent has NO Digimon, no selection is installed,
/// but the DP buff still fires (DCGO: selection block is gated by HasMatchConditionPermanent
/// but DP change runs unconditionally in the same coroutine).
#[test]
fn bt21_037_when_digivolving_no_opp_digimon_no_selection_but_dp_fires() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT21-037 YAML must parse")
        .memory(10)
        .build();

    let lighdramon = runner.place_on_field(0, "BT21-037", None);
    // No opponent Digimon on field.

    let base_dp = runner
        .effective_dp(lighdramon)
        .expect("Lighdramon must have DP");

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(lighdramon),
    );
    runner.game.drain_effect_queue();

    // No selection should be pending (no valid targets).
    let sel = runner.pending_selection();
    // If no selection, auto_resolve is safe (no-op).
    if sel.is_some() {
        // The select is optional: player may pass. No targets means it should auto-skip or
        // expose an optional empty selection; either way, the DP buff must still fire.
        runner
            .auto_resolve()
            .expect("auto_resolve with no valid targets");
    }

    let after_dp = runner
        .effective_dp(lighdramon)
        .expect("Lighdramon on field");
    assert_eq!(
        after_dp,
        base_dp + 2000,
        "+2000 DP must fire even when no opp Digimon is available to suspend"
    );
}

// ─── §3 Armor Purge keyword runtime behavior ────────────────────────────────

#[test]
fn bt21_037_armor_purge_prevents_deletion_when_top_source_trashed() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-037")
        .expect("BT21-037 YAML loads")
        .add_card(make_test_card("BASE", "Base"))
        .start();
    let lighdramon = runner.place_stack(0, &["BASE", "BT21-037"]);

    runner
        .game
        .delete_permanent_with_cause(lighdramon, ReplacementCause::OpponentEffect);

    let view = runner
        .pending_selection_view()
        .expect("Armor Purge accept prompt");
    assert_eq!(view.kind, SelectionKind::Replacement);
    assert!(view.is_optional);
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("accept Armor Purge");
    runner.auto_resolve().expect("finish Armor Purge");

    assert_eq!(runner.battle_area_size(0), 1);
    assert_eq!(
        runner.game.players[0].battle_area[0]
            .top_card()
            .card_id(&runner.game.card_data),
        "BASE"
    );
    assert!(runner.game.players[0]
        .trash
        .iter()
        .any(|card| card.card_id(&runner.game.card_data) == "BT21-037"));
}
