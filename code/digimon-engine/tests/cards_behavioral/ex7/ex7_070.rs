//! EX7-070 Der Blitz — Option, Black, Cost 6. Traits: [Three Musketeers].
//!
//! # Card text (official Bandai DB — data/card_bundles/EX7-070.md, verbatim)
//!
//! ```text
//! When effects trash this card from digivolution cards, <De-Digivolve 1>
//! 1 of your opponent's Digimon. While you have a [Three Musketeers] trait
//! Digimon, you may ignore this card's color requirements. [Main] Delete 1
//! of your opponent's Digimon with the lowest play cost. Then, place this
//! card as the bottom digivolution card of 1 of your [Three Musketeers]
//! trait Digimon.
//! ```
//!
//! Security Effect: "[Security] Delete 1 of your opponent's Digimon with the
//! lowest play cost."
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX7/Black/EX7_070.cs
//!
//! # Patterns this test covers
//! - Structural: card metadata + 4 clauses (2 triggered/inherited + 1 main +
//!   1 flood_gate declarative).
//! - Clause 1 (inherited, optional): `on_digivolution_card_trashed` self-scoped
//!   via `trashed_source_card_id_is` + `event_target_owner: you`, gated on an
//!   opponent Digimon existing. `de_digivolve { amount: 1, stop_at_level: 3 }`.
//!   Positive fire, negative (host-owner mismatch / different source card /
//!   no opponent Digimon), and optional-decline coverage.
//! - Clause 2 (flood_gate): positive-gate ignore-color while a [Three
//!   Musketeers] Digimon is on the field (vs BT19-093's negative-gate
//!   sibling), scoped to this card only via `card_number_is`.
//! - Clause 3 ([Main], main_from_hand): two INDEPENDENTLY-gated mandatory
//!   picks — delete opponent's lowest-play-cost Digimon, and place self as
//!   bottom digivolution source of 1 of the controller's [Three Musketeers]
//!   Digimon. Verifies each half fires alone when the other has no
//!   candidates (DCGO's two separate `if (HasMatchConditionPermanent(...))`
//!   blocks), and that `place_as_bottom_source { source: self }` actually
//!   relocates EX7-070 from hand to the target's stack bottom.
//! - Clause 4 (inherited security): same lowest-play-cost delete selector,
//!   no placement tail, no add-to-hand (non-Tamer security disposition).

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "EX7-070";

// ─── Fixture builders ─────────────────────────────────────────────────────────

/// An opponent Digimon with a parameterized play cost — for the
/// lowest-play-cost delete-target tests.
fn opp_digimon_cost(id: &str, cost: u16, dp: i32) -> CardData {
    let mut c = make_test_card(id, "OppDigimon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(dp);
    c.play_cost = cost;
    c
}

/// A generic Digimon usable as a plain battle-area body (no special traits).
fn plain_digimon(id: &str, level: u8, dp: i32) -> CardData {
    let mut c = make_test_card(id, "PlainDigimon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c
}

/// A Digimon carrying the [Three Musketeers] trait — the placement target
/// for Clause 3b / the color-bypass predicate for Clause 2.
fn tm_digimon(id: &str, level: u8, dp: i32) -> CardData {
    let mut c = make_test_card(id, "TMDigimon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c.traits = vec!["Three Musketeers".to_string()];
    c
}

/// A leveled Digimon for De-Digivolve stack tests.
fn lvl_digimon(id: &str, level: u8, dp: i32) -> CardData {
    let mut c = make_test_card(id, "LvlDigimon");
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Red];
    c.level = Some(level);
    c.dp = Some(dp);
    c
}

fn base_builder() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX7-070 YAML loads from the embedded pack")
}

/// Push a registered `card_id` onto `host`'s digivolution stack as its
/// bottom-most source (used to seed a self-trash scenario for Clause 1).
fn push_source_bottom(runner: &mut DebugRunner, host: PermanentHandle, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("{card_id} must be registered"));
    let next = runner.game.next_card_index();
    let cs = CardSource::new(data_idx, host.player, next);
    runner.game.players[host.player as usize].battle_area[host.index as usize]
        .card_sources
        .insert(0, cs);
}

/// Auto-resolve all remaining prompts by always taking the first legal
/// action — used once a specific branch decision is already locked in.
fn drain_prompts(runner: &mut DebugRunner) {
    while let Some(v) = runner.pending_selection_view() {
        let act = v
            .valid_action_ids
            .first()
            .copied()
            .expect("a resolvable action");
        runner.execute_action(v.selecting_player, act).ok();
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 1 — Structural assertions
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn ex7_070_compiles_with_printed_stats() {
    let runner = base_builder().start();
    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("EX7-070 in compiled cards");

    assert_eq!(compiled.card, CARD_ID);
    assert_eq!(compiled.name, "Der Blitz");
    assert_eq!(
        compiled.kind,
        digimon_dsl::compiled::CompiledCardKind::Option
    );
    assert_eq!(compiled.cost, Some(6));
    assert!(
        compiled.traits.iter().any(|t| t == "Three Musketeers"),
        "printed [Three Musketeers] trait missing"
    );
}

/// EX7-070 must have exactly 3 triggered clauses (inherited self-trash
/// De-Digivolve, [Main], inherited [Security]) plus 1 declarative flood_gate.
#[test]
fn ex7_070_has_expected_clause_shape() {
    let runner = base_builder().start();
    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("EX7-070 in compiled cards");

    let triggered_count = compiled
        .effects
        .iter()
        .filter(|c| matches!(c, CompiledClause::Triggered(_)))
        .count();
    let declarative_count = compiled
        .effects
        .iter()
        .filter(|c| matches!(c, CompiledClause::Declarative(_)))
        .count();

    assert_eq!(
        triggered_count, 3,
        "expected 3 triggered clauses (self-trash DeDigivolve, Main, Security)"
    );
    assert_eq!(
        declarative_count, 1,
        "expected 1 declarative clause (ignore-color flood_gate)"
    );
}

/// Clause 1 is inherited, optional, on_digivolution_card_trashed.
#[test]
fn ex7_070_clause1_is_inherited_optional_source_trash() {
    let runner = base_builder().start();
    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("EX7-070 in compiled cards");

    let found = compiled.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Triggered(t)
                if t.scope == CompiledScope::Inherited
                    && t.optional
                    && t.when == vec![CompiledTiming::OnDigivolutionCardTrashed]
        )
    });
    assert!(
        found,
        "Clause 1 must be inherited + optional + on_digivolution_card_trashed"
    );
}

/// Clause 2 is a flood_gate declarative granting IgnoreColorRequirement.
#[test]
fn ex7_070_clause2_is_ignore_color_flood_gate() {
    let runner = base_builder().start();
    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("EX7-070 in compiled cards");

    let found = compiled.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::FloodGate { modifier, .. })
                if modifier.contains("IgnoreColorRequirement")
        )
    });
    assert!(
        found,
        "Clause 2 must be a flood_gate granting IgnoreColorRequirement"
    );
}

/// Clause 3 fires on MainFromHand, is not scoped inherited, not optional at
/// clause level (both sub-picks are individually mandatory, gated by
/// candidate existence via nested `if`).
#[test]
fn ex7_070_clause3_is_main_from_hand() {
    let runner = base_builder().start();
    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("EX7-070 in compiled cards");

    let found = compiled.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Triggered(t)
                if t.when == vec![CompiledTiming::MainFromHand] && t.scope == CompiledScope::FaceUp
        )
    });
    assert!(found, "Clause 3 must be a face-up MainFromHand clause");
}

/// Clause 4 is inherited on_security.
#[test]
fn ex7_070_clause4_is_inherited_security() {
    let runner = base_builder().start();
    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("EX7-070 in compiled cards");

    let found = compiled.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Triggered(t)
                if t.scope == CompiledScope::Inherited
                    && t.when == vec![CompiledTiming::OnSecurity]
        )
    });
    assert!(found, "Clause 4 must be inherited on_security");
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 2 — Clause 1: inherited self-source-trash → De-Digivolve 1
// ─────────────────────────────────────────────────────────────────────────────

/// POSITIVE: when EX7-070 (as a digivolution source under the controller's
/// own Digimon) is trashed by an effect, and the opponent has a Digimon, the
/// optional De-Digivolve 1 prompt installs; accepting pops 1 source off the
/// opponent's stack.
#[test]
fn ex7_070_source_trash_offers_dedigivolve_and_resolves() {
    let host = plain_digimon("HOST", 5, 6000);
    let opp_base = lvl_digimon("OPP-BASE", 3, 2000);
    let opp_mid = lvl_digimon("OPP-MID", 4, 4000);

    let mut runner = base_builder()
        .add_card(host)
        .add_card(opp_base)
        .add_card(opp_mid)
        .build();

    let host_handle = runner.place_on_field(0, "HOST", None);
    let source = runner.push_source(host_handle, CARD_ID);
    let opponent = runner.place_stack(1, &["OPP-BASE", "OPP-MID"]);

    let top_card = runner.top_card(host_handle);
    {
        let mut ctx = EffectContext::new(&mut runner.game, top_card, Some(host_handle), 0);
        ctx.trash_card_source(host_handle, source);
    }

    // The optional trigger installs as an outer accept/decline Replacement
    // gate first (standard optional-triggered-clause shape); accepting it
    // then installs the actual De-Digivolve target selection.
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Replacement),
        "the optional inherited clause must first offer an accept/decline gate"
    );
    assert!(
        runner.pending_is_optional(),
        "the source-trash clause is optional (DCGO SetIsOptionEffect(true))"
    );

    runner
        .accept_optional_trigger()
        .expect("accept the optional De-Digivolve trigger");

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::OppField),
        "accepting must install the De-Digivolve target selection"
    );

    runner
        .auto_resolve()
        .expect("De-Digivolve target selection resolves");

    let opp_sources = runner.game.player(1).battle_area[opponent.index as usize]
        .card_sources
        .len();
    assert_eq!(
        opp_sources, 1,
        "opponent's Digimon must lose 1 stack level via De-Digivolve 1"
    );
}

/// NEGATIVE: declining the optional prompt leaves the opponent's stack
/// untouched.
#[test]
fn ex7_070_source_trash_decline_leaves_stack_untouched() {
    let host = plain_digimon("HOST-D", 5, 6000);
    let opp_base = lvl_digimon("OPP-BASE-D", 3, 2000);
    let opp_mid = lvl_digimon("OPP-MID-D", 4, 4000);

    let mut runner = base_builder()
        .add_card(host)
        .add_card(opp_base)
        .add_card(opp_mid)
        .build();

    let host_handle = runner.place_on_field(0, "HOST-D", None);
    let source = runner.push_source(host_handle, CARD_ID);
    let opponent = runner.place_stack(1, &["OPP-BASE-D", "OPP-MID-D"]);

    let top_card = runner.top_card(host_handle);
    {
        let mut ctx = EffectContext::new(&mut runner.game, top_card, Some(host_handle), 0);
        ctx.trash_card_source(host_handle, source);
    }

    // The optional trigger installs as an outer accept/decline Replacement
    // gate; PASS is accepted to decline even though it is not itself listed
    // in `valid_action_ids` (which holds only the single ACCEPT entry per
    // the `SelectionKind::Replacement` contract) — `decline_optional_trigger`
    // resolves PASS directly, matching the `is_optional` decline path.
    assert!(runner.pending_is_optional());
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Replacement),
        "De-Digivolve prompt installs as the outer optional accept/decline gate"
    );
    runner
        .decline_optional_trigger()
        .expect("decline the optional De-Digivolve trigger");

    let opp_sources = runner.game.player(1).battle_area[opponent.index as usize]
        .card_sources
        .len();
    assert_eq!(
        opp_sources, 2,
        "declining must leave the opponent's stack untouched"
    );
}

/// NEGATIVE: no De-Digivolve fires when the opponent has no Digimon at all.
#[test]
fn ex7_070_source_trash_no_fire_when_no_opp_digimon() {
    let host = plain_digimon("HOST-N", 5, 6000);

    let mut runner = base_builder().add_card(host).build();

    let host_handle = runner.place_on_field(0, "HOST-N", None);
    let source = runner.push_source(host_handle, CARD_ID);

    let top_card = runner.top_card(host_handle);
    {
        let mut ctx = EffectContext::new(&mut runner.game, top_card, Some(host_handle), 0);
        ctx.trash_card_source(host_handle, source);
    }

    assert!(
        runner.pending_selection().is_none(),
        "no De-Digivolve prompt when the opponent has no Digimon"
    );
}

/// NEGATIVE: trashing a DIFFERENT source card (not EX7-070 itself) from the
/// same host must never fire this clause — `trashed_source_card_id_is` must
/// scope strictly to this card's own identity.
#[test]
fn ex7_070_source_trash_no_fire_for_different_card() {
    let host = plain_digimon("HOST-DIFF", 5, 6000);
    let other_source = plain_digimon("OTHER-SRC", 3, 2000);
    let opp_top = lvl_digimon("OPP-TOP-DIFF", 4, 4000);

    let mut runner = base_builder()
        .add_card(host)
        .add_card(other_source)
        .add_card(opp_top)
        .build();

    let host_handle = runner.place_on_field(0, "HOST-DIFF", None);
    // EX7-070 sits under the host too, but we trash the OTHER source card.
    runner.push_source(host_handle, CARD_ID);
    let other = runner.push_source(host_handle, "OTHER-SRC");
    runner.place_on_field(1, "OPP-TOP-DIFF", Some(0));

    let top_card = runner.top_card(host_handle);
    {
        let mut ctx = EffectContext::new(&mut runner.game, top_card, Some(host_handle), 0);
        ctx.trash_card_source(host_handle, other);
    }

    assert!(
        runner.pending_selection().is_none(),
        "trashing a different source card must not fire EX7-070's self-scoped clause"
    );
}

/// NEGATIVE: the clause is scoped to the CONTROLLER's own host — trashing a
/// player-0-OWNED EX7-070 that is seated as a FOREIGN digivolution source
/// under a player-1-CONTROLLED host must not fire it. `event_target_owner:
/// you` reads the host's controller (player 1) relative to EX7-070's own
/// owner (player 0, via `push_source_owned`) — a mismatch ("opponent", not
/// "you") — so the clause must not fire. (A same-seat `push_source` variant
/// of this scenario is NOT a genuine mismatch: `push_source` always ties the
/// pushed source's owner to the host's controller, so relabeling both to
/// player 1 is a no-op re-seating, not a controller mismatch — hence the
/// explicit `push_source_owned` foreign-host construction here.)
#[test]
fn ex7_070_source_trash_no_fire_when_host_owned_by_opponent() {
    let opp_host = plain_digimon("OPP-HOST", 5, 6000);
    let opp_top = lvl_digimon("OPP-TOP-OWN", 4, 4000);

    let mut runner = base_builder().add_card(opp_host).add_card(opp_top).build();

    // Host is controlled by player 1; EX7-070 is seated under it as a
    // FOREIGN source still owned by player 0 (its actual controller).
    let host_handle = runner.place_on_field(1, "OPP-HOST", None);
    let source = runner.push_source_owned(host_handle, CARD_ID, 0);
    // Give player 1 (the host's controller / EX7-070-owner's opponent) a
    // Digimon so the clause's "opponent has a Digimon" gate is not what
    // blocks the fire — the assertion isolates the owner-mismatch gate.
    runner.place_on_field(1, "OPP-TOP-OWN", Some(0));

    let top_card = runner.top_card(host_handle);
    {
        let mut ctx = EffectContext::new(&mut runner.game, top_card, Some(host_handle), 0);
        ctx.trash_card_source(host_handle, source);
    }

    assert!(
        runner.pending_selection().is_none(),
        "EX7-070 owned by player 0 but attached under player 1's (foreign) \
         Digimon must not fire the 'your opponent's Digimon' de-digivolve \
         (event_target_owner mismatch: host's controller is not EX7-070's own owner)"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 3 — Clause 2: ignore-color flood_gate
// ─────────────────────────────────────────────────────────────────────────────

/// POSITIVE: with a [Three Musketeers] Digimon on the field, EX7-070 (a
/// Black card) becomes playable ignoring color — verified via the compiled
/// flood_gate's active_when predicate matching the board state indirectly
/// through a structural check (the runtime color-mask enforcement itself is
/// a separate, already-covered engine concern — G-IGNORE-COLOR-MASK).
#[test]
fn ex7_070_flood_gate_scoped_to_this_card_only() {
    let runner = base_builder().start();
    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("EX7-070 in compiled cards");

    let target = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::FloodGate {
                target, ..
            }) => Some(target),
            _ => None,
        })
        .expect("flood_gate clause must exist");

    // The target predicate must scope the bypass to this card's own number.
    let target_debug = format!("{target:?}");
    assert!(
        target_debug.contains("EX7-070"),
        "flood_gate target must scope to EX7-070 only; got {target_debug}"
    );
}

/// The flood_gate's active_when must require a [Three Musketeers] Digimon —
/// checked structurally since active_when is a compiled predicate tree.
#[test]
fn ex7_070_flood_gate_requires_three_musketeers_digimon() {
    let runner = base_builder().start();
    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("EX7-070 in compiled cards");

    let active_when = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::FloodGate {
                active_when,
                ..
            }) => Some(active_when),
            _ => None,
        })
        .expect("flood_gate clause must exist");

    let active_when_debug = format!("{active_when:?}");
    assert!(
        active_when_debug.contains("Three Musketeers"),
        "flood_gate active_when must require a [Three Musketeers] Digimon; got {active_when_debug}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 4 — Clause 3: [Main] delete lowest-cost + place self (independent halves)
// ─────────────────────────────────────────────────────────────────────────────

/// POSITIVE (both halves fire): opponent has Digimon at varying costs, and
/// the controller has a [Three Musketeers] Digimon. Playing EX7-070 from
/// hand deletes the lowest-play-cost opponent Digimon AND places EX7-070 as
/// the bottom source of the chosen TM Digimon.
#[test]
fn ex7_070_main_deletes_lowest_cost_and_places_self_under_tm_digimon() {
    let tm = tm_digimon("TM-1", 4, 5000);
    let opp_cheap = opp_digimon_cost("OPP-CHEAP", 2, 3000);
    let opp_expensive = opp_digimon_cost("OPP-EXP", 7, 9000);

    let mut runner = base_builder()
        .add_card(tm)
        .add_card(opp_cheap)
        .add_card(opp_expensive)
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();

    let tm_handle = runner.place_on_field(0, "TM-1", None);
    runner.place_on_field(1, "OPP-CHEAP", Some(0));
    runner.place_on_field(1, "OPP-EXP", Some(0));

    runner.play(0, 0).expect("EX7-070 plays from hand");

    // 3a: mandatory delete-target select over the lowest-play-cost opponent
    // Digimon (only OPP-CHEAP at cost 2 should be a candidate). Resolve ONLY
    // this one selection (not `auto_resolve`, which would drain BOTH 3a and
    // 3b in one loop and leave nothing pending for the 3b assertion below).
    let view = runner
        .pending_selection_view()
        .expect("delete-target selection must install (opponent has Digimon)");
    assert!(
        !view.is_optional,
        "the lowest-play-cost delete pick is mandatory"
    );
    let delete_action = view
        .valid_action_ids
        .first()
        .copied()
        .expect("a resolvable delete-target action");
    runner
        .execute_action(view.selecting_player, delete_action)
        .expect("resolve delete-target pick");

    assert_eq!(
        runner.battle_area_size(1),
        1,
        "the lowest-play-cost opponent Digimon (OPP-CHEAP, cost 2) must be deleted"
    );

    // 3b: mandatory placement-target select over the controller's TM Digimon.
    let place_view = runner
        .pending_selection_view()
        .expect("placement-target selection must install (a TM Digimon exists)");
    assert!(
        !place_view.is_optional,
        "the placement pick is mandatory once a TM Digimon exists"
    );
    runner
        .auto_resolve()
        .expect("resolve placement-target pick");

    let stack = &runner.game.player(0).battle_area[tm_handle.index as usize].card_sources;
    assert!(
        stack
            .iter()
            .any(|s| s.card_id(&runner.game.card_data) == CARD_ID),
        "EX7-070 must be placed as a digivolution source under the chosen TM Digimon"
    );
    assert_eq!(
        stack[0].card_id(&runner.game.card_data),
        CARD_ID,
        "EX7-070 must be the BOTTOM source of the stack"
    );
}

/// The lowest-play-cost selector must exclude higher-cost candidates —
/// verify only the cost-2 Digimon is offered as a target, not the cost-7 one.
#[test]
fn ex7_070_main_delete_target_is_only_lowest_cost() {
    let tm = tm_digimon("TM-LC", 4, 5000);
    let opp_cheap = opp_digimon_cost("OPP-LC-CHEAP", 2, 3000);
    let opp_expensive = opp_digimon_cost("OPP-LC-EXP", 7, 9000);

    let mut runner = base_builder()
        .add_card(tm)
        .add_card(opp_cheap)
        .add_card(opp_expensive)
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();

    runner.place_on_field(0, "TM-LC", None);
    runner.place_on_field(1, "OPP-LC-CHEAP", Some(0));
    runner.place_on_field(1, "OPP-LC-EXP", Some(0));

    runner.play(0, 0).expect("EX7-070 plays from hand");

    let view = runner
        .pending_selection_view()
        .expect("delete-target selection must install");
    let candidate_count = view.valid_action_ids.iter().filter(|&&a| a != PASS).count();
    assert_eq!(
        candidate_count, 1,
        "only the lowest-play-cost opponent Digimon (cost 2) may be a candidate"
    );
}

/// NEGATIVE (3a only): the controller has no [Three Musketeers] Digimon, so
/// only the delete half fires; the placement half is silently skipped with
/// no pending selection installed for it, and EX7-070 is NOT placed
/// anywhere (it stays wherever the play pipeline routes an Option with no
/// placement, i.e. does not appear as a source).
#[test]
fn ex7_070_main_delete_only_when_no_tm_digimon() {
    let opp_cheap = opp_digimon_cost("OPP-DO-CHEAP", 3, 4000);

    let mut runner = base_builder()
        .add_card(opp_cheap)
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();

    runner.place_on_field(1, "OPP-DO-CHEAP", Some(0));

    runner.play(0, 0).expect("EX7-070 plays from hand");

    let view = runner
        .pending_selection_view()
        .expect("delete-target selection must install");
    runner.auto_resolve().expect("resolve delete-target pick");

    assert_eq!(
        runner.battle_area_size(1),
        0,
        "the sole opponent Digimon must be deleted"
    );
    // No further selection should be pending — the placement half found no
    // TM Digimon candidate and silently skipped.
    assert!(
        runner.pending_selection().is_none(),
        "no placement selection should install when the controller has no [Three Musketeers] Digimon"
    );
}

/// NEGATIVE (3b only): the opponent has no Digimon, so only the placement
/// half fires; the delete half is silently skipped with no selection
/// installed for it.
#[test]
fn ex7_070_main_place_only_when_no_opp_digimon() {
    let tm = tm_digimon("TM-PO", 4, 5000);

    let mut runner = base_builder()
        .add_card(tm)
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();

    let tm_handle = runner.place_on_field(0, "TM-PO", None);

    runner.play(0, 0).expect("EX7-070 plays from hand");

    // No opponent Digimon exists, so the delete half's `if any_permanent`
    // guard must skip straight to the placement half.
    let view = runner
        .pending_selection_view()
        .expect("placement-target selection must install (no opponent Digimon exists)");
    runner
        .auto_resolve()
        .expect("resolve placement-target pick");

    let stack = &runner.game.player(0).battle_area[tm_handle.index as usize].card_sources;
    assert!(
        stack
            .iter()
            .any(|s| s.card_id(&runner.game.card_data) == CARD_ID),
        "EX7-070 must still be placed under the TM Digimon even with no opponent Digimon to delete"
    );
}

/// NEGATIVE (neither half fires): no opponent Digimon and no controller TM
/// Digimon — the whole clause is a silent no-op with no pending selection.
#[test]
fn ex7_070_main_no_selection_when_neither_condition_met() {
    let mut runner = base_builder().hand(0, &[CARD_ID]).memory(10).start();

    runner.play(0, 0).expect("EX7-070 plays from hand");

    assert!(
        runner.pending_selection().is_none(),
        "neither half has a candidate — the whole clause must be a silent no-op"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 5 — Clause 4: inherited [Security] delete lowest-cost
// ─────────────────────────────────────────────────────────────────────────────

/// POSITIVE: firing the security clause with opponent Digimon at varying
/// costs deletes only the lowest-play-cost one. EX7-070's [Security] clause
/// is `scope: inherited`, so it fires from a digivolution-source position —
/// mirrored on the AD1-017 `ex7_070_security_*` sibling idiom: seat EX7-070
/// as a buried source under a host the security owner (player 0) controls,
/// then fire `EffectTiming::SecuritySkill` via `TriggerSource::Permanent(host)`.
#[test]
fn ex7_070_security_deletes_lowest_cost_opp_digimon() {
    let host = plain_digimon("HOST-SEC", 5, 6000);
    let opp_cheap = opp_digimon_cost("OPP-SEC-CHEAP", 3, 4000);
    let opp_expensive = opp_digimon_cost("OPP-SEC-EXP", 8, 10000);

    let mut runner = base_builder()
        .add_card(host)
        .add_card(opp_cheap)
        .add_card(opp_expensive)
        .build();

    let host_handle = runner.place_on_field(0, "HOST-SEC", None);
    runner.push_source(host_handle, CARD_ID);
    runner.place_on_field(1, "OPP-SEC-CHEAP", Some(0));
    runner.place_on_field(1, "OPP-SEC-EXP", Some(0));

    runner.game.enqueue_triggered(
        digimon_engine::enums::EffectTiming::SecuritySkill,
        digimon_engine::selection::TriggerSource::Permanent(host_handle),
    );
    runner.game.drain_effect_queue();

    let view = runner
        .pending_selection_view()
        .expect("the inherited [Security] clause must offer a delete-target selection");
    let candidate_count = view.valid_action_ids.iter().filter(|&&a| a != PASS).count();
    assert_eq!(
        candidate_count, 1,
        "only the lowest-play-cost opponent Digimon may be a security-delete candidate"
    );
    runner.auto_resolve().expect("resolve security delete pick");

    assert_eq!(
        runner.battle_area_size(1),
        1,
        "only the lowest-play-cost opponent Digimon must be deleted"
    );
}

/// NEGATIVE: no opponent Digimon → the security clause is a silent no-op.
#[test]
fn ex7_070_security_no_op_when_no_opp_digimon() {
    let host = plain_digimon("HOST-SEC-N", 5, 6000);

    let mut runner = base_builder().add_card(host).build();

    let host_handle = runner.place_on_field(0, "HOST-SEC-N", None);
    runner.push_source(host_handle, CARD_ID);

    runner.game.enqueue_triggered(
        digimon_engine::enums::EffectTiming::SecuritySkill,
        digimon_engine::selection::TriggerSource::Permanent(host_handle),
    );
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "no opponent Digimon → the security clause must be a silent no-op"
    );
    assert_eq!(runner.battle_area_size(1), 0);
}
