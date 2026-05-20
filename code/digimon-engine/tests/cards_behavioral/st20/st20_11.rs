//! ST20-11 WarGreymon — Lv.6, Black/Red, DP 12000, Cost 7.
//! Traits: Dragonkin, ADVENTURE, Hero. Attribute: Vaccine.
//!
//! # Card text (cards.json — printed)
//!
//! [Hand] [Counter] ＜Blast Digivolve＞ (Your Digimon may digivolve into this
//!   card without paying the cost.)
//! [On Play] [When Digivolving] Until your opponent's turn ends, for every 2
//!   colors your Tamers have, their Digimon's effects don't affect 1 of your
//!   Digimon.
//! [When Digivolving] [When Attacking] Delete 1 of your opponent's lowest DP
//!   Digimon.
//!
//! # Inherited
//! Ace Overflow ＜-4＞
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/ST20/Black/ST20_11.cs
//!
//! # Source priority
//! Per CLAUDE.md: printed text > docs/RULES_CONTEXT.md > fandom > DCGO. The
//! immunity clause's body diverges between [On Play] and [When Digivolving]
//! in DCGO (the [On Play] SkillCondition treats `IsDigimonEffect &&
//! IsSecurityEffect` as immune-target-eligible, while [When Digivolving]
//! checks only `IsDigimonEffect`). Printed text only specifies "Digimon's
//! effects" so we model both timings identically per the printed text — but
//! the entire clause is BLOCKED from authoring (see below).
//!
//! # Patterns this card exercises (test API §4.3)
//! - H12 Blast Digivolve (alt-path `kind: burst_digivolve, marker: true`)
//! - H13 ACE Overflow (-4 metadata via `ace_overflow:`)
//! - F8  multi-timing triggered cluster ([When Digivolving]+[When Attacking])
//! - F2  mandatory delete-1-opponent (lowest-DP filter)
//! - Alt-path xros_req: Lv.5 ADVENTURE/Hero alt-digivolve cost 3
//!
//! # Implementation notes
//!
//! - **G-DSL-DISTINCT-TAMER-COLORS-FORMULA (CLOSED)**: the [On Play]
//!   [When Digivolving] tamer-color immunity clause is authored with
//!   `select_count_capped_multi` over the controller's own battle area,
//!   bounded by a formula-valued `max` =
//!   `floor_div(distinct_colors_count(your Tamers), 2)`. `per_selected` +
//!   `grant_effect_immunity` grants each picked Digimon opponent-Digimon-
//!   effect immunity until end of opponent's turn.
//!
//! - The [OP][WD] immunity clause and the [WD][WA] delete clause both fire
//!   at WhenDigivolving, so the engine raises a `SelectionKind::TriggerOrder`
//!   prompt to order them; tests drain it via `skip_trigger_order_prompts`
//!   before reaching the real selection.

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

#[path = "../../support/dsl_card_data.rs"]
mod dsl_card_data;

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledDeclarativeClause,
    CompiledScope, CompiledTiming, CompiledTriggeredClause,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Expiry, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;
use digimon_engine::TriggerSource;

/// Drain leading `SelectionKind::TriggerOrder` prompts by submitting the
/// first legal action for each. ST20-11's [On Play][When Digivolving]
/// immunity clause and its [When Digivolving][When Attacking] delete clause
/// both fire at WhenDigivolving, so a TriggerOrder prompt asks the
/// controller which resolves first. Tests downstream of the order pick care
/// only about the real selection (immunity / delete) that follows.
fn skip_trigger_order_prompts(runner: &mut DebugRunner) {
    while runner.pending_kind() == Some(SelectionKind::TriggerOrder) {
        let view = runner
            .pending_selection_view()
            .expect("trigger-order prompt installed");
        let first_action = view.valid_action_ids[0];
        runner
            .execute_action(view.selecting_player, first_action)
            .expect("submit trigger-order pick");
    }
}

// ─── Compiled card helpers ────────────────────────────────────────────────────

fn compiled() -> digimon_dsl::compiled::CompiledCard {
    dsl_card_data::compiled("ST20-11")
}

fn wargreymon_runner() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card("ST20-11")
        .expect("ST20-11 YAML in embedded pack")
}

fn make_digimon(id: &str, level: u8, dp: i32) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card
}

fn make_digimon_named(id: &str, name: &str, level: u8, dp: i32) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card
}

fn make_tamer_with_colors(id: &str, colors: &[CardColor]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Tamer;
    card.level = None;
    card.dp = None;
    card.colors = colors.to_vec();
    card
}

// ─── Section 1: Structural assertions ─────────────────────────────────────────

/// ST20-11 must compile with the printed Lv.6 / DP 12000 / Cost 7 stats and
/// the dual-color (Black + Red) printed identity.
#[test]
fn st20_11_has_printed_stats() {
    let card = compiled();

    assert_eq!(card.card, "ST20-11");
    assert_eq!(card.name, "WarGreymon");
    assert_eq!(card.level, Some(6));
    assert_eq!(card.cost, Some(7));
    assert_eq!(card.dp, Some(12000));

    // Black + Red — order in YAML is [black, red] per cards.json card_colors [5, 0].
    assert!(
        card.color.contains(&CompiledColor::Black),
        "Black color missing — got {:?}",
        card.color
    );
    assert!(
        card.color.contains(&CompiledColor::Red),
        "Red color missing — got {:?}",
        card.color
    );

    for trait_name in ["Dragonkin", "ADVENTURE", "Hero"] {
        assert!(
            card.traits.iter().any(|t| t == trait_name),
            "missing trait {trait_name} — got {:?}",
            card.traits
        );
    }
}

/// Inherited Ace Overflow ＜-4＞ — the `ace_overflow:` field must be -4.
#[test]
fn st20_11_has_ace_overflow_minus_four() {
    let card = compiled();
    assert_eq!(
        card.ace_overflow,
        Some(-4),
        "Ace Overflow must be -4 (printed text)"
    );
}

/// Three alt-paths are expected:
///   1. standard digivolve Lv.5 Black / Cost 4 (printed evo_costs)
///   2. xros_req alt-digivolve Lv.5 ADVENTURE/Hero / Cost 3
///   3. ＜Blast Digivolve＞ via burst_digivolve marker:true cost 0
#[test]
fn st20_11_declares_three_alt_paths_including_blast_digivolve() {
    let card = compiled();

    let burst = card
        .alt_paths
        .iter()
        .find(|p| p.kind == CompiledAltPathKind::BurstDigivolve);
    assert!(
        burst.is_some(),
        "expected a BurstDigivolve alt-path (＜Blast Digivolve＞ marker) — got {:?}",
        card.alt_paths.iter().map(|p| p.kind).collect::<Vec<_>>()
    );
    assert_eq!(
        burst.unwrap().cost,
        Some(CompiledCost::Literal(0)),
        "Blast Digivolve cost must be 0"
    );

    // Standard cost-4 digivolve.
    assert!(
        card.alt_paths.iter().any(|p| {
            p.kind == CompiledAltPathKind::Digivolve && p.cost == Some(CompiledCost::Literal(4))
        }),
        "expected a cost-4 standard digivolve alt-path"
    );

    // xros_req cost-3 digivolve.
    assert!(
        card.alt_paths.iter().any(|p| {
            p.kind == CompiledAltPathKind::Digivolve && p.cost == Some(CompiledCost::Literal(3))
        }),
        "expected a cost-3 alt-digivolve path (ADVENTURE/Hero source)"
    );
}

/// The own-side triggered clauses authored in YAML are the [WD]+[WA] delete
/// cluster. The [OP][WD] tamer-color immunity clause is OMITTED pending
/// G-DSL-DISTINCT-TAMER-COLORS-FORMULA (see header). This test locks the
/// shape of what IS in the YAML — the immunity clause's presence is asserted
/// in the gap-tracked test below.
#[test]
fn st20_11_has_wd_wa_delete_cluster() {
    let card = compiled();

    let triggered: Vec<&CompiledTriggeredClause> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    let own: Vec<&CompiledTriggeredClause> = triggered
        .iter()
        .copied()
        .filter(|t| matches!(t.scope, CompiledScope::FaceUp))
        .collect();

    assert!(
        own.iter().any(|t| t
            .when
            .iter()
            .any(|w| matches!(w, CompiledTiming::WhenDigivolving))
            && t.when
                .iter()
                .any(|w| matches!(w, CompiledTiming::WhenAttacking))),
        "expected own [When Digivolving]+[When Attacking] clause covering the lowest-DP delete"
    );
}

/// The [On Play][When Digivolving] tamer-color immunity clause is authored
/// via `select_count_capped_multi` with a formula-valued `max` =
/// floor_div(distinct_colors_count(your Tamers), 2).
#[test]
fn st20_11_has_op_wd_tamer_color_immunity_clause() {
    let card = compiled();
    let triggered: Vec<&CompiledTriggeredClause> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    assert!(
        triggered
            .iter()
            .any(|t| matches!(t.scope, CompiledScope::FaceUp)
                && t.when.iter().any(|w| matches!(w, CompiledTiming::OnPlay))
                && t.when
                    .iter()
                    .any(|w| matches!(w, CompiledTiming::WhenDigivolving))),
        "expected [On Play][When Digivolving] immunity clause once gaps close"
    );
}

// ─── Section 2: Behavioral — [When Digivolving] delete cluster ──────────────

/// With no opponent permanents and no Tamers, neither the [WD] delete (gated
/// by `condition: any_permanent { of: opponent kind: digimon }`) nor the
/// [OP][WD] immunity grant (0 tamer colors / 2 = 0 picks) installs a real
/// selection. The two clauses still queue together at WhenDigivolving, so a
/// TriggerOrder prompt is raised first; after resolving it no real
/// selection remains.
#[test]
fn st20_11_when_digivolving_with_no_opponent_digimon_installs_no_selection() {
    let mut runner = wargreymon_runner().memory(20).start();
    let perm = runner.place_on_field(0, "ST20-11", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(perm),
    );
    runner.game.drain_effect_queue();
    skip_trigger_order_prompts(&mut runner);

    assert!(
        runner.pending_selection().is_none(),
        "no real selection should install when opponent has no Digimon and there are no Tamers"
    );
}

/// With at least one opponent Digimon, the [WD] delete prompt installs.
/// The mandatory selection covers the printed text "Delete 1 of your
/// opponent's lowest DP Digimon". The [OP][WD] immunity clause also fires
/// at WhenDigivolving; the leading TriggerOrder prompt is drained first.
#[test]
fn st20_11_when_digivolving_offers_delete_selection_with_opponent_digimon() {
    let mut runner = wargreymon_runner()
        .add_card(make_digimon("OPP-A", 4, 5000))
        .memory(20)
        .start();
    let perm = runner.place_on_field(0, "ST20-11", Some(0));
    runner.place_on_field(1, "OPP-A", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(perm),
    );
    runner.game.drain_effect_queue();
    skip_trigger_order_prompts(&mut runner);

    let view = runner
        .pending_selection_view()
        .expect("delete selection installs");
    assert!(
        !view.valid_action_ids.is_empty(),
        "at least one valid delete target for opp Digimon"
    );
}

/// [When Attacking] timing fires the same delete body as [When Digivolving]
/// (the YAML clause uses `when: [when_digivolving, when_attacking]`).
#[test]
fn st20_11_when_attacking_offers_same_delete_selection() {
    let mut runner = wargreymon_runner()
        .add_card(make_digimon("OPP-A", 4, 5000))
        .memory(20)
        .start();
    let perm = runner.place_on_field(0, "ST20-11", Some(0));
    runner.place_on_field(1, "OPP-A", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(perm));
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_some(),
        "[When Attacking] must fire the same delete cluster body"
    );
}

/// Resolve the [WD] delete: the chosen opponent Digimon must be removed
/// from the opponent's battle area. This locks the `delete_permanent`
/// step plumbing.
#[test]
fn st20_11_when_digivolving_deletes_chosen_opponent_digimon() {
    let mut runner = wargreymon_runner()
        .add_card(make_digimon("OPP-A", 4, 5000))
        .memory(20)
        .start();
    let perm = runner.place_on_field(0, "ST20-11", Some(0));
    let _target = runner.place_on_field(1, "OPP-A", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(perm),
    );
    runner.game.drain_effect_queue();
    skip_trigger_order_prompts(&mut runner);

    let (sp, aid) = {
        let view = runner.pending_selection_view().expect("delete selection");
        assert_eq!(view.valid_action_ids.len(), 1);
        (view.selecting_player, view.valid_action_ids[0])
    };
    runner.execute_action(sp, aid).expect("execute delete pick");
    let _ = runner.auto_resolve();

    assert!(
        !runner.game.players[1]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "OPP-A"),
        "chosen opponent permanent must be deleted"
    );
}

/// G-PRED-DP-LTE: the lowest-DP filter is authored as
/// `dp_lte: { aggregate: { selector: lowest_dp, scope: opponent } }` but the
/// permanent-side `dp_lte` predicate is not evaluated. With two opponent
/// Digimon at different DPs, only the lower-DP one should be a valid target —
/// when the gap closes this test should pass; until then both are offered.
#[test]
fn st20_11_when_digivolving_only_offers_lowest_dp_opponent() {
    let mut runner = wargreymon_runner()
        .add_card(make_digimon("OPP-LOW", 4, 4000))
        .add_card(make_digimon("OPP-HIGH", 6, 12000))
        .memory(20)
        .start();
    let perm = runner.place_on_field(0, "ST20-11", Some(0));
    runner.place_on_field(1, "OPP-LOW", Some(0));
    runner.place_on_field(1, "OPP-HIGH", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(perm),
    );
    runner.game.drain_effect_queue();
    skip_trigger_order_prompts(&mut runner);

    let view = runner
        .pending_selection_view()
        .expect("delete selection installs");
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only the lowest-DP opponent Digimon should be a valid target"
    );
}

/// G-PRED-DP-LTE companion: with two opponent Digimon TIED at the lowest DP,
/// both must be valid targets (player picks). With higher-DP Digimon also on
/// the field, those higher-DP ones must NOT be eligible. Until the gap
/// closes, this test fails because all four Digimon are offered.
#[test]
fn st20_11_when_digivolving_offers_all_tied_lowest_dp_opponents() {
    let mut runner = wargreymon_runner()
        .add_card(make_digimon("OPP-TIE-A", 4, 4000))
        .add_card(make_digimon("OPP-TIE-B", 4, 4000))
        .add_card(make_digimon("OPP-HIGH", 6, 12000))
        .memory(20)
        .start();
    let perm = runner.place_on_field(0, "ST20-11", Some(0));
    runner.place_on_field(1, "OPP-TIE-A", Some(0));
    runner.place_on_field(1, "OPP-TIE-B", Some(0));
    runner.place_on_field(1, "OPP-HIGH", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(perm),
    );
    runner.game.drain_effect_queue();
    skip_trigger_order_prompts(&mut runner);

    let view = runner
        .pending_selection_view()
        .expect("delete selection installs");
    assert_eq!(
        view.valid_action_ids.len(),
        2,
        "exactly the 2 tied lowest-DP opponent Digimon should be valid targets"
    );
}

// ─── Section 3: Behavioral — [On Play][When Digivolving] tamer-color immunity ──
//
// All tests in this section depend on G-DSL-DISTINCT-TAMER-COLORS-FORMULA
// (formula primitive missing) PLUS a companion repeat_n / per_n step driver
// to install N selections. Until both ship, the entire clause is OMITTED
// from the YAML — see header. These tests are kept as gap markers so the
// orchestrator can flip them on once the gaps close.

/// G-DSL-DISTINCT-TAMER-COLORS-FORMULA: with 1 Tamer color (1 / 2 = 0
/// immunities), the [On Play] body should install no immunity selection.
/// This is the "below threshold" case — even when the formula ships, this
/// case should result in no prompt.
#[test]
fn st20_11_on_play_with_one_tamer_color_grants_no_immunity() {
    let mut runner = wargreymon_runner()
        .add_card(make_tamer_with_colors("TAMER-R", &[CardColor::Red]))
        .add_card(make_digimon("ALLY", 4, 5000))
        .memory(20)
        .start();
    let perm = runner.place_on_field(0, "ST20-11", Some(0));
    runner.place_on_field(0, "TAMER-R", Some(0));
    let _ally = runner.place_on_field(0, "ALLY", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(perm));
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "1 tamer color / 2 = 0 — no immunity selection should install"
    );
}

/// With 2 Tamer colors, the player gets to pick exactly 1 own Digimon for
/// immunity (2 / 2 = 1 pick). The picked Digimon receives CannotBeAffected
/// (opponent-Digimon-effect immunity) until end of opponent's turn.
#[test]
fn st20_11_on_play_with_two_tamer_colors_grants_one_immunity_selection() {
    let mut runner = wargreymon_runner()
        .add_card(make_tamer_with_colors("TAMER-R", &[CardColor::Red]))
        .add_card(make_tamer_with_colors("TAMER-B", &[CardColor::Blue]))
        .add_card(make_digimon("ALLY", 4, 5000))
        .memory(20)
        .start();
    let perm = runner.place_on_field(0, "ST20-11", Some(0));
    runner.place_on_field(0, "TAMER-R", Some(0));
    runner.place_on_field(0, "TAMER-B", Some(0));
    runner.place_on_field(0, "ALLY", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(perm));
    runner.game.drain_effect_queue();

    let view = runner
        .pending_selection_view()
        .expect("immunity selection installs (2 colors / 2 = 1 pick)");
    assert!(!view.valid_action_ids.is_empty());
    // The filter is "1 of your Digimon" — both ST20-11 and ALLY are valid
    // targets. Pick the first; it must receive CannotBeAffected.
    let sp = view.selecting_player;
    let aid = view.valid_action_ids[0];
    runner.execute_action(sp, aid).expect("immunity pick");
    let _ = runner.auto_resolve();

    let modifiers = runner.modifiers();
    let immune = runner.game.players[0]
        .battle_area
        .iter()
        .enumerate()
        .filter(|(idx, _)| {
            modifiers.has(
                PermanentHandle {
                    player: 0,
                    index: *idx as u8,
                },
                ModifierType::CannotBeAffected,
            )
        })
        .count();
    assert_eq!(
        immune, 1,
        "exactly 1 own Digimon must receive effect immunity (2 tamer colors / 2 = 1)"
    );
}

/// With 4 Tamer colors, the player gets to pick exactly 2 distinct own
/// Digimon for immunity (4 / 2 = 2). `select_count_capped_multi` installs
/// the picks sequentially (each pick removed from the candidate set), then
/// `per_selected` grants each picked Digimon effect immunity.
#[test]
fn st20_11_on_play_with_four_tamer_colors_grants_two_immunity_selections() {
    use digimon_engine::action::space::PASS;

    let mut runner = wargreymon_runner()
        .add_card(make_tamer_with_colors("TAMER-R", &[CardColor::Red]))
        .add_card(make_tamer_with_colors("TAMER-B", &[CardColor::Blue]))
        .add_card(make_tamer_with_colors("TAMER-Y", &[CardColor::Yellow]))
        .add_card(make_tamer_with_colors("TAMER-G", &[CardColor::Green]))
        .add_card(make_digimon("ALLY-A", 4, 5000))
        .add_card(make_digimon("ALLY-B", 4, 5000))
        .memory(20)
        .start();
    let perm = runner.place_on_field(0, "ST20-11", Some(0));
    runner.place_on_field(0, "TAMER-R", Some(0));
    runner.place_on_field(0, "TAMER-B", Some(0));
    runner.place_on_field(0, "TAMER-Y", Some(0));
    runner.place_on_field(0, "TAMER-G", Some(0));
    runner.place_on_field(0, "ALLY-A", Some(0));
    runner.place_on_field(0, "ALLY-B", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(perm));
    runner.game.drain_effect_queue();

    // Resolve the two sequential immunity picks, always taking the first
    // real (non-PASS) target.
    while let Some(view) = runner.pending_selection_view() {
        let accept = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|&id| id != PASS)
            .unwrap_or(view.valid_action_ids[0]);
        runner
            .execute_action(view.selecting_player, accept)
            .expect("resolve immunity pick");
    }
    let _ = runner.auto_resolve();

    let modifiers = runner.modifiers();
    let immune = runner.game.players[0]
        .battle_area
        .iter()
        .enumerate()
        .filter(|(idx, _)| {
            modifiers.has(
                PermanentHandle {
                    player: 0,
                    index: *idx as u8,
                },
                ModifierType::CannotBeAffected,
            )
        })
        .count();
    assert_eq!(
        immune, 2,
        "exactly 2 distinct own Digimon must receive effect immunity (4 tamer colors / 2 = 2)"
    );
}

/// [When Digivolving] fires BOTH the [OP][WD] tamer-color immunity clause
/// and the [WD][WA] lowest-DP delete clause. A TriggerOrder prompt orders
/// them; after resolving everything the immunity is granted (2 tamer colors
/// / 2 = 1) AND the opponent Digimon is deleted.
#[test]
fn st20_11_when_digivolving_offers_immunity_selection_in_addition_to_delete() {
    use digimon_engine::action::space::PASS;

    let mut runner = wargreymon_runner()
        .add_card(make_tamer_with_colors("TAMER-R", &[CardColor::Red]))
        .add_card(make_tamer_with_colors("TAMER-B", &[CardColor::Blue]))
        .add_card(make_digimon("ALLY", 4, 5000))
        .add_card(make_digimon("OPP", 4, 4000))
        .memory(20)
        .start();
    let perm = runner.place_on_field(0, "ST20-11", Some(0));
    runner.place_on_field(0, "TAMER-R", Some(0));
    runner.place_on_field(0, "TAMER-B", Some(0));
    runner.place_on_field(0, "ALLY", Some(0));
    runner.place_on_field(1, "OPP", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(perm),
    );
    runner.game.drain_effect_queue();

    // A TriggerOrder prompt installs first (two clauses fired together).
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::TriggerOrder),
        "two clauses firing at WhenDigivolving must raise a TriggerOrder prompt"
    );

    // Resolve every prompt (trigger-order, immunity pick, delete pick).
    while let Some(view) = runner.pending_selection_view() {
        let accept = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|&id| id != PASS)
            .unwrap_or(view.valid_action_ids[0]);
        runner
            .execute_action(view.selecting_player, accept)
            .expect("resolve pending selection");
    }
    let _ = runner.auto_resolve();

    // Immunity clause granted 1 effect immunity (2 tamer colors / 2 = 1).
    let modifiers = runner.modifiers();
    let immune = runner.game.players[0]
        .battle_area
        .iter()
        .enumerate()
        .filter(|(idx, _)| {
            modifiers.has(
                PermanentHandle {
                    player: 0,
                    index: *idx as u8,
                },
                ModifierType::CannotBeAffected,
            )
        })
        .count();
    assert_eq!(
        immune, 1,
        "immunity clause must grant 1 effect immunity at WhenDigivolving"
    );

    // Delete clause removed the opponent Digimon.
    assert!(
        !runner.game.players[1]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "OPP"),
        "delete clause must delete the opponent Digimon at WhenDigivolving"
    );
}
