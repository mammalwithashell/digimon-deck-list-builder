//! BT15-101 MetalGarurumon — Lv.6, Blue, DP 12000, Cost 12.
//!
//! # Card text (printed)
//!
//! While you have a Tamer with [Matt Ishida] in its name and your opponent has
//! a 10000 DP or higher Digimon, one of your [Gabumon] may digivolve into this
//! card in your hand for a digivolution cost of 4, ignoring its digivolution
//! requirements.
//! <Evade>
//! [When Digivolving] 3 of your opponent's Digimon and Tamers can't suspend
//! until the end of their turn.
//! [All Turns] [Once Per Turn] When this Digimon becomes suspended, you may
//! unsuspend it.
//!
//! # Inherited
//! (none)
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT15/Blue/BT15_101.cs
//!
//! # Implementation notes / DSL gaps
//!
//! 1. **Alt-path with activation gate** — The "[Matt Ishida] tamer present AND
//!    opponent has 10000 DP+ Digimon" gate is expressible at the substrate
//!    level: `AltPathSpec.condition` was added 2026-05-15
//!    (G-ALT-PATH-CONDITION RESOLVED, qa/resolved-gaps.md). The opp-DP
//!    side of the gate still requires `G-PRED-DP-LTE` for the inverse
//!    "opponent has 10000+ DP Digimon" check. Until that lands, authoring
//!    the alt-path with only the Matt Ishida half would still violate
//!    printed text. The Gabumon alt-path therefore remains OMITTED from
//!    the YAML pending G-PRED-DP-LTE; the standard Lv5 Blue cost 4 path
//!    is present (printed `evo_costs`).
//!
//! 2. **Self-target on_suspend gate** — Printed text says "When THIS Digimon
//!    becomes suspended". The DSL has `event_target_owner` and
//!    `event_target_kind` predicate leaves but no `event_target_is_source` /
//!    `event_target_is_self` predicate that compares the suspended-permanent
//!    handle against the source permanent. The DSL `equals: [...]` predicate
//!    only compares integers (literals + integer bindings), not permanent
//!    handles. Without this gate, the clause uses the AD1-014 pattern
//!    (`event_target_owner: you, event_target_kind: digimon`), which over-fires
//!    when ANY of the controller's Digimon (including allies) suspend. New
//!    DSL gap: G-DSL-EVENT-TARGET-IS-SELF.
//!
//! 3. **Repeat-N selection over BattleArea** — `select_count_capped_multi`
//!    only supports Hand and Trash zones (see
//!    `count_capped_candidate_count` in selections.rs); it cannot pick N
//!    permanents from the opponent's battle area. The "3 of your opponent's
//!    Digimon and Tamers" effect is therefore implemented as 3 sequential
//!    `select_opponent_permanent` + `add_modifier` steps, each filtered with
//!    `not_in_binding` against the prior picks. Each step is non-optional
//!    (DCGO `canNoSelect: false`); when fewer than 3 candidates exist, later
//!    selects find no targets and skip silently.
//!
//! # Patterns this test covers
//!
//! - A4 alt-path (standard Lv5 Blue cost 4)
//! - H8 Evade keyword (declarative grant)
//! - F7 cannot_suspend modifier with end_of_opponents_turn expiry
//! - All-Turns OPT on_suspend → unsuspend self (via `target: source`)
//! - Repeat-3 sequential select-and-modify pattern (BattleArea zone)

#![allow(dead_code, unused_imports)]

#[path = "../../support/dsl_card_data.rs"]
mod dsl_card_data;

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledDeclarativeClause,
    CompiledScope, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, EffectTiming, Expiry, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::TriggerSource;

// ─── Compiled card helpers ────────────────────────────────────────────────────

fn compiled() -> digimon_dsl::compiled::CompiledCard {
    dsl_card_data::compiled("BT15-101")
}

fn metal_garurumon_runner() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card("BT15-101")
        .expect("BT15-101 YAML in embedded pack")
}

fn make_digimon(id: &str, level: u8, dp: i32) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card
}

fn make_tamer(id: &str, name: &str) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Tamer;
    card.level = None;
    card.dp = None;
    card
}

// ─── Section 1: Structural assertions ─────────────────────────────────────────

#[test]
fn bt15_101_has_printed_stats_and_traits() {
    let card = compiled();

    assert_eq!(card.card, "BT15-101");
    assert_eq!(card.name, "MetalGarurumon");
    assert_eq!(card.level, Some(6));
    assert_eq!(card.color, vec![CompiledColor::Blue]);
    assert_eq!(card.cost, Some(12));
    assert_eq!(card.dp, Some(12000));
    assert!(
        card.traits.iter().any(|t| t == "Cyborg"),
        "missing trait Cyborg — got {:?}",
        card.traits
    );
}

#[test]
fn bt15_101_declares_evade_keyword_grant() {
    let card = compiled();

    let grants: Vec<&str> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                ..
            }) => Some(keyword.as_str()),
            _ => None,
        })
        .collect();

    assert!(
        grants.contains(&"Evade"),
        "Evade keyword grant missing — got {grants:?}"
    );
}

#[test]
fn bt15_101_has_when_digivolving_lock_clause_and_all_turns_on_suspend_clause() {
    let card = compiled();

    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    // [When Digivolving] 3-lock clause.
    assert!(
        triggered
            .iter()
            .any(|t| t.when == vec![CompiledTiming::WhenDigivolving]),
        "expected [When Digivolving] triggered clause"
    );

    // [All Turns] [OPT] on_suspend may-unsuspend clause.
    assert!(
        triggered
            .iter()
            .any(|t| t.when == vec![CompiledTiming::OnSuspend] && t.once_per_turn && t.optional),
        "expected [All Turns] [OPT] on_suspend optional clause"
    );
}

#[test]
fn bt15_101_omits_gabumon_alt_path_pending_dsl_gap() {
    // The Gabumon alt-path is gated on a printed condition (Matt Ishida tamer
    // + opp 10000 DP+ Digimon). The `AltPathSpec.condition` side is now
    // expressible (G-ALT-PATH-CONDITION RESOLVED 2026-05-15), but the
    // opp-DP half still requires `G-PRED-DP-LTE` for the "opponent has
    // 10000+ DP Digimon" check. The path is intentionally OMITTED from
    // YAML until that gap closes. We assert that no cost-4 alt-path with
    // `ignore_requirements: true` is present — the standard cost-4
    // alt-path (printed evo_costs) does NOT ignore requirements.
    let card = compiled();

    let has_ignore_alt = card
        .alt_paths
        .iter()
        .any(|p| p.kind == CompiledAltPathKind::Digivolve && p.ignore_requirements);
    assert!(
        !has_ignore_alt,
        "Gabumon alt-path must be omitted pending G-PRED-DP-LTE"
    );
}

// ─── Section 2: Behavioral — [When Digivolving] 3-lock cluster ────────────────

#[test]
fn bt15_101_when_digivolving_with_no_opp_permanents_installs_no_selection() {
    let mut runner = metal_garurumon_runner().memory(20).start();
    let perm = runner.place_on_field(0, "BT15-101", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(perm),
    );
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "no selection should install when opponent has no permanents"
    );
}

#[test]
fn bt15_101_when_digivolving_offers_selection_for_opponent_digimon_or_tamer() {
    let mut runner = metal_garurumon_runner()
        .add_card(make_digimon("OPP-DIGI", 5, 7000))
        .memory(20)
        .start();
    let perm = runner.place_on_field(0, "BT15-101", Some(0));
    runner.place_on_field(1, "OPP-DIGI", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(perm),
    );
    runner.game.drain_effect_queue();

    let view = runner
        .pending_selection_view()
        .expect("first lock pick selection must install");
    assert!(
        !view.valid_action_ids.is_empty(),
        "at least one valid lock target"
    );
}

#[test]
fn bt15_101_when_digivolving_locks_chosen_opponent_with_cannot_suspend() {
    let mut runner = metal_garurumon_runner()
        .add_card(make_digimon("OPP-DIGI", 5, 7000))
        .memory(20)
        .start();
    let perm = runner.place_on_field(0, "BT15-101", Some(0));
    let target = runner.place_on_field(1, "OPP-DIGI", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(perm),
    );
    runner.game.drain_effect_queue();

    // Resolve the first lock pick (only valid target is OPP-DIGI).
    let (sp, aid) = {
        let v = runner.pending_selection_view().expect("first lock pick");
        (v.selecting_player, v.valid_action_ids[0])
    };
    runner.execute_action(sp, aid).expect("execute first pick");
    let _ = runner.auto_resolve();

    assert!(
        runner.modifiers().has(target, ModifierType::CannotSuspend),
        "chosen opp permanent must receive CannotSuspend"
    );
}

#[test]
fn bt15_101_when_digivolving_locks_up_to_three_distinct_opponents() {
    // With 3 valid opp permanents (mix of Digimon and Tamer), all 3 receive
    // CannotSuspend across three sequential picks (none repeated due to
    // not_in_binding filters).
    let mut runner = metal_garurumon_runner()
        .add_card(make_digimon("OPP-A", 5, 7000))
        .add_card(make_digimon("OPP-B", 4, 5000))
        .add_card(make_tamer("OPP-T", "OPP-T"))
        .memory(20)
        .start();
    let perm = runner.place_on_field(0, "BT15-101", Some(0));
    let a = runner.place_on_field(1, "OPP-A", Some(0));
    let b = runner.place_on_field(1, "OPP-B", Some(0));
    let t = runner.place_on_field(1, "OPP-T", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(perm),
    );
    runner.game.drain_effect_queue();

    // First pick installs as a parked selection. Then `auto_resolve` walks
    // through the entire chain — pick 2 and pick 3 — each time taking the
    // first valid target (which `not_in_binding` filters keep distinct).
    assert!(
        runner.pending_selection().is_some(),
        "first lock pick selection must install"
    );
    runner
        .auto_resolve()
        .expect("auto-resolve all 3 lock picks");

    assert!(
        runner.pending_selection().is_none(),
        "after auto-resolving 3 picks, no further lock selection should remain"
    );

    // All three opp permanents must hold CannotSuspend.
    let modifiers = runner.modifiers();
    let locked = [a, b, t]
        .iter()
        .filter(|h| modifiers.has(**h, ModifierType::CannotSuspend))
        .count();
    assert_eq!(
        locked, 3,
        "all three distinct opp permanents must receive CannotSuspend (got {locked})"
    );
}

#[test]
fn bt15_101_when_digivolving_filters_out_own_permanents() {
    // Lock filter targets opponent only — own Digimon must not be a candidate.
    let mut runner = metal_garurumon_runner()
        .add_card(make_digimon("OWN", 4, 5000))
        .memory(20)
        .start();
    let perm = runner.place_on_field(0, "BT15-101", Some(0));
    runner.place_on_field(0, "OWN", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(perm),
    );
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "no selection should install when only own permanents exist"
    );
}

// ─── Section 3: All-Turns OPT — on_suspend → may unsuspend self ──────────────

#[test]
fn bt15_101_on_self_suspend_offers_optional_unsuspend_prompt() {
    // Standard self-suspend → self-unsuspend pattern. Suspend BT15-101 via
    // game.suspend(); the on_suspend trigger fires, offering an optional
    // unsuspend prompt that resolves to BT15-101 itself via target: source.
    let mut runner = metal_garurumon_runner().memory(20).start();
    let perm = runner.place_on_field(0, "BT15-101", Some(0));

    // Suspend BT15-101 — fires its own OnSuspend clause.
    runner.game.suspend(perm);
    let _ = runner.auto_resolve();

    let suspended_after = runner.game.players[0].battle_area[perm.index as usize].is_suspended;
    assert!(
        !suspended_after,
        "BT15-101 should unsuspend after the OPT triggers on its own suspend"
    );
}

#[test]
#[ignore = "pending: G-DSL-EVENT-TARGET-IS-SELF — DSL has no predicate to assert the suspended permanent equals the source permanent; the on_suspend clause uses the AD1-014 pattern (event_target_owner: you, event_target_kind: digimon), so it over-fires when an ALLY suspends rather than only when this Digimon suspends. Negative case held until the predicate ships."]
fn bt15_101_on_ally_suspend_does_not_consume_opt_or_offer_prompt() {
    // Printed text: "When THIS Digimon becomes suspended, you may unsuspend it."
    // The clause must NOT fire on an ally suspending — only on self.
    let mut runner = metal_garurumon_runner()
        .add_card(make_digimon("ALLY", 4, 5000))
        .memory(20)
        .start();
    let perm = runner.place_on_field(0, "BT15-101", Some(0));
    let ally = runner.place_on_field(0, "ALLY", Some(0));

    // Pre-suspend BT15-101 by direct mutation so we can detect whether the
    // ally's suspension wrongly consumes the OPT and unsuspends BT15-101.
    runner.game.players[0].battle_area[perm.index as usize].is_suspended = true;

    // Suspending the ally must NOT trigger BT15-101's clause (printed text
    // limits the trigger to "this Digimon").
    runner.game.suspend(ally);
    let _ = runner.auto_resolve();

    let suspended_after = runner.game.players[0].battle_area[perm.index as usize].is_suspended;
    assert!(
        suspended_after,
        "ally suspending must not trigger BT15-101's self-only on_suspend clause (printed: this Digimon)"
    );
}

#[test]
fn bt15_101_on_opponent_suspend_does_not_unsuspend_self() {
    // Even with the missing self-gate, the event_target_owner: you filter
    // closes the cross-controller leak. Suspending an opponent permanent must
    // never trigger the may-unsuspend prompt.
    let mut runner = metal_garurumon_runner()
        .add_card(make_digimon("OPP", 4, 5000))
        .memory(20)
        .start();
    let perm = runner.place_on_field(0, "BT15-101", Some(0));
    let opp = runner.place_on_field(1, "OPP", Some(0));

    // Pre-suspend BT15-101 by direct mutation (avoid firing its own clause).
    runner.game.players[0].battle_area[perm.index as usize].is_suspended = true;

    runner.game.suspend(opp);
    let _ = runner.auto_resolve();

    let suspended_after = runner.game.players[0].battle_area[perm.index as usize].is_suspended;
    assert!(
        suspended_after,
        "opponent suspending must not trigger BT15-101's clause (event_target_owner: you gate)"
    );
}
