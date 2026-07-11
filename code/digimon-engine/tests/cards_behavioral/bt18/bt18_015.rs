//! BT18-015 Kimeramon — Digimon, Lv.5, Red/Purple, DP 8000, Cost 8.
//! Traits: Composite. Attribute: Data. Form: Ultimate.
//!
//! # Card text (official Bandai DB / data/card_bundles/BT18-015.md;
//! card image confirmed)
//!
//! [When Digivolving] [When Attacking] By deleting 1 of your Digimon, delete
//! 1 of your opponent's Digimon with the lowest DP.
//! [On Deletion] 1 of your [Machinedramon] and 1 [Kimeramon] in the trash may
//! DNA digivolve into [Millenniummon] in the hand.
//!
//! Inherited Effect: ＜Security A. +1＞ (This Digimon checks 1 additional
//! security card.)
//!
//! Digivolution: standard Red Lv.4/Cost 4 and Purple Lv.4/Cost 4 circles;
//! special condition (xros_req) "[Digivolve] Lv.4 w/[Composite] trait: Cost 3".
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT18/Red/BT18_015.cs
//!
//! # Role resolution (card-face wording is ambiguous in English; DCGO
//! disambiguates)
//!
//! DCGO's `HasMachinedramon` scans the OWNER's BATTLE AREA
//! (`IsPermanentExistsOnOwnerBattleAreaDigimon`) for a Digimon named
//! "Machinedramon"; `HasKimeramon` scans the OWNER's TRASH
//! (`IsExistOnTrash`) for a card named "Kimeramon" (no self-exclusion — the
//! just-deleted Kimeramon itself, already moved to trash per the batched
//! deletion flow (CLAUDE.md rule 25), is itself a legal material). So:
//! Machinedramon = FIELD partner, Kimeramon = TRASH partner. Recipe target:
//! BT18-019 Millenniummon (`dna_costs`: requirement1 name_contains
//! "Kimeramon", requirement2 name_contains "Machinedramon", memory_cost 0).
//!
//! # Patterns this test file covers
//! - Structural: 2 triggered clauses (WD+WA shared, on_deletion), 3
//!   digivolve alt-paths.
//! - WD/WA clause: cost-then-effect pairing (`select_own_permanent` optional,
//!   no `other:true`, default `continue_on_decline: false`), mandatory
//!   `select_opponent_permanent { selector: lowest_dp }`, no [OPT] (each
//!   timing independently re-fires).
//! - on_deletion clause: the `effect_initiated_dna_digivolve_trash_partner`
//!   DSL step (G-DSL-DNA-TRASH-PARTNER, resolved 2026-07-03), lowering to
//!   the engine primitive
//!   `EffectContext::effect_initiated_dna_digivolve_trash_partner`
//!   (G-ENGINE-DNA-TRASH-MATERIAL). Condition gating on all 3 candidate
//!   pools (own field Machinedramon, own trash Kimeramon, own hand
//!   Millenniummon); outer "may"; order of picks (hand result → field
//!   partner → trash partner).
//! - Inherited ＜Security A. +1＞: `scope: inherited` + `grant_keyword`
//!   (ST1-07 / AD1-009 idiom) — carrier-only via the has_keyword()
//!   stack-walk; a standalone face-up BT18-015 does NOT self-grant.
//!
//! # Known gaps
//! (none)

#![allow(unused_imports, dead_code)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
    CompiledTriggeredClause,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const BT18_015_YAML: &str = include_str!("../../../cards/bt18/BT18-015.yaml");

// ─── Runner builders ──────────────────────────────────────────────────────────

/// BT18-015 loaded from inline YAML (this card is brand-new — using
/// `from_dsl_yaml` avoids depending on the embedded pack having been rebuilt
/// since this file was authored) plus its real DSL siblings: BT18-019
/// Millenniummon (the DNA result) and ST5-12 Machinedramon (the field
/// material). Per §11 anti-patterns: prefer real shipped cards over
/// synthetic fixtures when they exist.
fn kimeramon_runner() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .from_dsl_yaml(BT18_015_YAML)
        .expect("BT18-015 YAML must parse")
        .dsl_card("BT18-019")
        .expect("BT18-019 Millenniummon must be in the embedded DSL pack")
        .dsl_card("ST5-12")
        .expect("ST5-12 Machinedramon must be in the embedded DSL pack")
}

fn make_digimon(id: &str, level: u8, dp: i32) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card
}

/// A named Digimon used to satisfy filters that key on `name_contains`
/// (Kimeramon trash copies, non-matching filler, etc).
fn make_named_digimon(id: &str, name: &str, level: u8, dp: i32) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card
}

fn push_to_hand(runner: &mut DebugRunner, player: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|card| card.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card {card_id}"));
    let next = runner.game.next_card_index();
    runner.game.players[player as usize]
        .hand
        .push(CardSource::new(data_idx, player, next));
}

fn push_to_trash(runner: &mut DebugRunner, player: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|card| card.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card {card_id}"));
    let next = runner.game.next_card_index();
    runner.game.players[player as usize]
        .trash
        .push(CardSource::new(data_idx, player, next));
}

fn has_named_on_field(runner: &DebugRunner, player: u8, name: &str) -> bool {
    runner.game.players[player as usize]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_name(&runner.game.card_data) == name)
}

/// Drive every installed selection by picking the first non-PASS action,
/// falling back to PASS if that's the only legal action. Used for tests that
/// only care about the eventual post-state, not which branch is exercised.
fn resolve_all_picking_first_non_pass(runner: &mut DebugRunner) {
    let mut steps = 0;
    while runner.game.pending_selection.is_some() && steps < 30 {
        let (player, action) = {
            let sel = runner.game.pending_selection.as_ref().unwrap();
            let action = sel
                .valid_action_ids
                .iter()
                .copied()
                .find(|&a| a != PASS)
                .unwrap_or(sel.valid_action_ids[0]);
            (sel.selecting_player, action)
        };
        let _ = runner.game.resolve_selection(player, action);
        runner.game.drain_effect_queue();
        steps += 1;
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn bt18_015_yaml_parses_without_error() {
    let _runner = kimeramon_runner();
}

#[test]
fn bt18_015_has_two_triggered_clauses_and_inherited_security_attack_grant() {
    let runner = kimeramon_runner().start();
    let compiled = runner
        .compiled_card("BT18-015")
        .expect("BT18-015 must be in compiled_cards");

    let triggered: Vec<&CompiledTriggeredClause> = compiled
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
        "BT18-015 has exactly 2 triggered clauses (WD+WA shared, on_deletion); got {}",
        triggered.len()
    );

    // The printed Inherited Effect ＜Security A. +1＞ must be present as an
    // inherited-scope GrantKeyword declarative clause (ST1-07 / AD1-009 idiom).
    let inherited_grant = compiled.effects.iter().find_map(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
            keyword,
            value,
            scope,
            ..
        }) if keyword == "SecurityAttackPlus" => Some((*value, *scope)),
        _ => None,
    });
    let (value, scope) = inherited_grant
        .expect("BT18-015 must carry the printed inherited <Security A. +1> grant_keyword clause");
    assert_eq!(value, Some(1), "the printed value is +1");
    assert_eq!(
        scope,
        CompiledScope::Inherited,
        "the grant is on the INHERITED effect line, not the face-up line"
    );
}

#[test]
fn bt18_015_wd_wa_clause_has_both_timings_and_is_not_optional_at_clause_level() {
    let runner = kimeramon_runner().start();
    let compiled = runner
        .compiled_card("BT18-015")
        .expect("BT18-015 must be in compiled_cards");

    let clause = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::WhenDigivolving))
        .expect("WD/WA clause must exist");

    assert!(
        clause.when.contains(&CompiledTiming::WhenAttacking),
        "the WD clause must also carry WhenAttacking (shared body, printed text has no OPT)"
    );
    assert!(
        !clause.once_per_turn,
        "BT18-015's WD/WA clause has no [OPT] tag in the printed text"
    );
    assert_eq!(clause.scope, CompiledScope::FaceUp);
}

#[test]
fn bt18_015_on_deletion_clause_is_optional_face_up() {
    let runner = kimeramon_runner().start();
    let compiled = runner
        .compiled_card("BT18-015")
        .expect("BT18-015 must be in compiled_cards");

    let clause = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnDeletion))
        .expect("on_deletion clause must exist");

    assert!(
        clause.optional,
        "BT18-015's [On Deletion] DNA rider is 'may' — the outer gate must be optional"
    );
    assert_eq!(clause.scope, CompiledScope::FaceUp);
}

#[test]
fn bt18_015_declares_three_digivolve_alt_paths() {
    let runner = kimeramon_runner().start();
    let compiled = runner
        .compiled_card("BT18-015")
        .expect("BT18-015 must be in compiled_cards");

    let digivolve_paths = compiled
        .alt_paths
        .iter()
        .filter(|p| matches!(p.kind, CompiledAltPathKind::Digivolve))
        .count();

    assert_eq!(
        digivolve_paths, 3,
        "BT18-015 must declare 3 digivolve alt-paths (Red Lv4/4, Purple Lv4/4, \
         Composite-trait Lv4/3); got {digivolve_paths}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 2 — [When Digivolving][When Attacking]: condition gating
// ═══════════════════════════════════════════════════════════════════════════

/// With no own Digimon to delete as the cost (only Kimeramon itself on
/// field, which IS a legal target — so seed a board with ONLY Kimeramon and
/// verify the pick prompt still installs, since `select_own_permanent` has
/// no `other:true` exclusion). This test asserts the POSITIVE: the prompt
/// installs even when Kimeramon is the sole own Digimon.
#[test]
fn bt18_015_wd_installs_cost_prompt_even_when_kimeramon_is_the_only_own_digimon() {
    let mut runner = kimeramon_runner().memory(10).start();
    let kimeramon = runner.place_on_field(0, "BT18-015", Some(0));
    runner.place_on_field(1, "ST5-12", None); // opponent Digimon so pool 2 isn't vacuous

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(kimeramon),
    );
    runner.game.drain_effect_queue();

    let sel = runner
        .game
        .pending_selection
        .as_ref()
        .expect("cost pick must install even with only Kimeramon on own field");
    assert!(
        sel.is_optional,
        "the cost pick is optional (You may decline)"
    );
    // Must have a non-PASS action available (Kimeramon itself is legal).
    assert!(
        sel.valid_action_ids.iter().any(|&a| a != PASS),
        "Kimeramon itself must be a legal cost target (no other:true exclusion)"
    );
}

/// Negative: no own Digimon on field at all (impossible in practice since
/// Kimeramon itself is on field to fire WD, but exercise the deeper case:
/// the SELECT installs only when a controller-owned Digimon exists, and here
/// there always is one — Kimeramon itself). This test instead exercises the
/// case where declining the cost drops the entire clause (no opponent
/// deletion happens).
#[test]
fn bt18_015_wd_declining_cost_prevents_opponent_deletion() {
    let mut runner = kimeramon_runner().memory(10).start();
    let kimeramon = runner.place_on_field(0, "BT18-015", Some(0));
    let opp = runner.place_on_field(1, "ST5-12", None);

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(kimeramon),
    );
    runner.game.drain_effect_queue();

    let (player, _) = {
        let sel = runner.game.pending_selection.as_ref().expect("cost prompt");
        (sel.selecting_player, ())
    };
    runner
        .game
        .resolve_selection(player, PASS)
        .expect("decline the cost");
    runner.game.drain_effect_queue();

    assert!(
        runner.game.pending_selection.is_none(),
        "declining the cost must drop the rest of the clause (no further prompt)"
    );
    assert!(
        runner.game.players[1]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "ST5-12"),
        "opponent's Digimon must survive — the delete never ran because the cost was declined"
    );
    // Kimeramon itself also survives (it was not chosen as the cost).
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "BT18-015"),
        "Kimeramon survives when the player declines to pay the cost"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 3 — [When Digivolving][When Attacking]: behavioral outcome
// ═══════════════════════════════════════════════════════════════════════════

/// Positive: pay the cost (delete a filler own Digimon), then the mandatory
/// second pick deletes the opponent's LOWEST-DP Digimon among two.
#[test]
fn bt18_015_wd_deletes_opponent_lowest_dp_after_paying_cost() {
    let mut runner = kimeramon_runner()
        .add_card(make_digimon("FILLER-OWN", 3, 3000))
        .add_card(make_digimon("OPP-LOW2000", 3, 2000))
        .memory(10)
        .start();
    let kimeramon = runner.place_on_field(0, "BT18-015", Some(0));
    let _filler_own = runner.place_on_field(0, "FILLER-OWN", None);
    runner.place_on_field(1, "ST5-12", None); // 11000 DP — must survive
    runner.place_on_field(1, "OPP-LOW2000", None); // 2000 DP — must be deleted

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(kimeramon),
    );
    runner.game.drain_effect_queue();

    // Pick the cost: delete the filler own Digimon (not Kimeramon).
    {
        let sel = runner.game.pending_selection.as_ref().expect("cost prompt");
        let action = sel
            .valid_action_ids
            .iter()
            .copied()
            .find(|&a| a != PASS)
            .expect("at least one own Digimon selectable");
        let player = sel.selecting_player;
        runner.game.resolve_selection(player, action).ok();
    }
    runner.game.drain_effect_queue();

    // filler_own should now be gone from the field (or Kimeramon, depending
    // on which slot the engine offered first — assert generically that
    // EXACTLY one own Digimon was deleted).
    let own_remaining = runner.game.players[0].battle_area.len();
    assert_eq!(
        own_remaining, 1,
        "exactly 1 own Digimon must have been deleted as the cost"
    );

    // Mandatory second pick: delete opponent's lowest DP Digimon (2000 DP).
    let sel = runner
        .game
        .pending_selection
        .as_ref()
        .expect("mandatory opponent lowest-DP delete prompt must install");
    assert!(
        !sel.is_optional,
        "the opponent delete step is mandatory once the cost is paid"
    );
    let action = sel
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != PASS)
        .expect("opponent target available");
    let player = sel.selecting_player;
    runner.game.resolve_selection(player, action).ok();
    runner.game.drain_effect_queue();

    assert!(
        !runner.game.players[1]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "OPP-LOW2000"),
        "the opponent's lowest-DP Digimon (2000 DP) must have been deleted"
    );
    assert!(
        runner.game.players[1]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "ST5-12"),
        "the higher-DP opponent Digimon (11000 DP) must survive"
    );
}

/// [When Attacking] independently fires the same body — no shared OPT with
/// [When Digivolving] (no [OPT] tag on the printed clause at all).
#[test]
fn bt18_015_wa_fires_independently_of_prior_wd_activation_same_turn() {
    let mut runner = kimeramon_runner()
        .add_card(make_digimon("FILLER-OWN2", 3, 3000))
        .memory(10)
        .start();
    let kimeramon = runner.place_on_field(0, "BT18-015", Some(0));
    let _filler = runner.place_on_field(0, "FILLER-OWN2", None);
    runner.place_on_field(1, "ST5-12", None);

    // Fire WD once and fully resolve (decline immediately to keep the board
    // simple — we only care that WA can still fire afterward).
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(kimeramon),
    );
    runner.game.drain_effect_queue();
    if let Some(sel) = runner.game.pending_selection.clone() {
        runner
            .game
            .resolve_selection(sel.selecting_player, PASS)
            .ok();
        runner.game.drain_effect_queue();
    }

    // Now fire WA (When Attacking) — must ALSO install a fresh cost prompt
    // (no shared lockout blocks it).
    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(kimeramon),
    );
    runner.game.drain_effect_queue();

    assert!(
        runner.game.pending_selection.is_some(),
        "[When Attacking] must independently install its own cost prompt \
         (no [OPT] tag shared with [When Digivolving])"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 4 — [On Deletion]: condition gating (all 3 pools required)
// ═══════════════════════════════════════════════════════════════════════════

/// Negative: Kimeramon deleted with NO Machinedramon on field → the rider
/// must not fire (no selection installs).
#[test]
fn bt18_015_on_deletion_does_not_fire_without_machinedramon_on_field() {
    let mut runner = kimeramon_runner().memory(10).start();
    let kimeramon = runner.place_on_field(0, "BT18-015", Some(0));
    push_to_trash(&mut runner, 0, "BT18-015"); // a spare Kimeramon-named copy in trash
    push_to_hand(&mut runner, 0, "BT18-019"); // Millenniummon in hand
                                              // NO Machinedramon on field.

    runner.game.delete_permanent_with_cause(
        kimeramon,
        digimon_engine::replacement::ReplacementCause::OwnEffect,
    );
    runner.game.drain_effect_queue();

    assert!(
        runner.game.pending_selection.is_none(),
        "on_deletion rider must NOT fire without a Machinedramon on the controller's field"
    );
}

/// Negative: Machinedramon on field + Millenniummon in hand, but NO
/// Kimeramon-named card anywhere in the trash (impossible in real play since
/// the deleted Kimeramon lands in trash, but exercise the condition guard by
/// removing the just-deleted card from trash before the observer fires is
/// not feasible — instead test the case where the deleted Kimeramon is the
/// ONLY possible trash material, which IS present, by asserting the POSITIVE
/// path in section 5). This test instead asserts the hand-side gate: with
/// Machinedramon on field and a Kimeramon in trash but NO Millenniummon in
/// hand, the rider must not fire.
#[test]
fn bt18_015_on_deletion_does_not_fire_without_millenniummon_in_hand() {
    let mut runner = kimeramon_runner().memory(10).start();
    let kimeramon = runner.place_on_field(0, "BT18-015", Some(0));
    runner.place_on_field(0, "ST5-12", None); // Machinedramon on field
                                              // No Millenniummon in hand.

    runner.game.delete_permanent_with_cause(
        kimeramon,
        digimon_engine::replacement::ReplacementCause::OwnEffect,
    );
    runner.game.drain_effect_queue();

    assert!(
        runner.game.pending_selection.is_none(),
        "on_deletion rider must NOT fire without a Millenniummon in the controller's hand"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 5 — [On Deletion]: behavioral outcome (the DNA merge)
// ═══════════════════════════════════════════════════════════════════════════

/// Positive: Kimeramon dies with a Machinedramon on field and a Millenniummon
/// in hand. Per rule 25 (OnDeletion fires post-trash), the deleted Kimeramon
/// itself is now IN the trash and satisfies the "1 [Kimeramon] in the trash"
/// material — no separate trash seed needed. Driving the full flow (accept →
/// pick Millenniummon → pick Machinedramon → pick the trashed Kimeramon)
/// must merge Machinedramon + Kimeramon into a single permanent topped by
/// Millenniummon.
#[test]
fn bt18_015_on_deletion_dna_digivolves_into_millenniummon_using_self_as_trash_material() {
    let mut runner = kimeramon_runner().memory(10).start();
    let kimeramon = runner.place_on_field(0, "BT18-015", Some(0));
    let kimeramon_card = runner.top_card(kimeramon);
    runner.place_on_field(0, "ST5-12", None); // Machinedramon on field
    push_to_hand(&mut runner, 0, "BT18-019"); // Millenniummon in hand

    runner.game.delete_permanent_with_cause(
        kimeramon,
        digimon_engine::replacement::ReplacementCause::OwnEffect,
    );
    runner.game.drain_effect_queue();

    // Kimeramon must now be in the trash (post-trash OnDeletion timing).
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.handle() == kimeramon_card),
        "precondition: the deleted Kimeramon must be in the trash when on_deletion fires"
    );

    let sel = runner
        .game
        .pending_selection
        .as_ref()
        .expect("on_deletion outer accept/decline prompt must install");
    assert!(
        sel.is_optional,
        "the [On Deletion] rider is 'may' — PASS must be legal"
    );

    resolve_all_picking_first_non_pass(&mut runner);

    let merged = runner.game.players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&runner.game.card_data) == "BT18-019")
        .expect("merged Millenniummon permanent must exist after DNA digivolve");

    assert!(
        merged
            .card_sources
            .iter()
            .any(|s| s.card_id(&runner.game.card_data) == "ST5-12"),
        "Machinedramon must be a digivolution source under the merged Millenniummon"
    );
    assert!(
        merged
            .card_sources
            .iter()
            .any(|s| s.handle() == kimeramon_card),
        "the deleted Kimeramon (now in trash) must be the trash material consumed into the merge"
    );
    assert_eq!(
        runner.game.players[0].hand.len(),
        0,
        "the Millenniummon result must leave the hand"
    );
    assert!(
        !runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.handle() == kimeramon_card),
        "the trash Kimeramon material must be consumed out of the trash by the merge"
    );
}

/// Positive, alternate material: with TWO Kimeramon-named cards available
/// (the just-deleted one AND a separately-seeded trash copy), the trash
/// pick prompt must expose a real choice (both are legal trash materials).
#[test]
fn bt18_015_on_deletion_offers_trash_pick_among_multiple_kimeramon_copies() {
    let mut runner = kimeramon_runner().memory(10).start();
    let kimeramon = runner.place_on_field(0, "BT18-015", Some(0));
    runner.place_on_field(0, "ST5-12", None);
    push_to_hand(&mut runner, 0, "BT18-019");
    push_to_trash(&mut runner, 0, "BT18-015"); // a second Kimeramon-named copy already in trash

    runner.game.delete_permanent_with_cause(
        kimeramon,
        digimon_engine::replacement::ReplacementCause::OwnEffect,
    );
    runner.game.drain_effect_queue();

    // Accept the outer "may".
    {
        let sel = runner
            .game
            .pending_selection
            .as_ref()
            .expect("outer accept prompt");
        let action = sel
            .valid_action_ids
            .iter()
            .copied()
            .find(|&a| a != PASS)
            .expect("accept action available");
        let player = sel.selecting_player;
        runner.game.resolve_selection(player, action).ok();
        runner.game.drain_effect_queue();
    }
    // Pick the Millenniummon result (only one in hand).
    {
        let sel = runner
            .game
            .pending_selection
            .as_ref()
            .expect("hand result prompt");
        let action = sel
            .valid_action_ids
            .iter()
            .copied()
            .find(|&a| a != PASS)
            .expect("Millenniummon selectable");
        let player = sel.selecting_player;
        runner.game.resolve_selection(player, action).ok();
        runner.game.drain_effect_queue();
    }
    // Pick the Machinedramon field partner (only one on field).
    {
        let sel = runner
            .game
            .pending_selection
            .as_ref()
            .expect("field partner prompt");
        let action = sel
            .valid_action_ids
            .iter()
            .copied()
            .find(|&a| a != PASS)
            .expect("Machinedramon selectable");
        let player = sel.selecting_player;
        runner.game.resolve_selection(player, action).ok();
        runner.game.drain_effect_queue();
    }

    // Now the trash-partner prompt must expose 2 distinct Kimeramon-named
    // choices (the just-deleted one + the seeded copy).
    let sel = runner
        .game
        .pending_selection
        .as_ref()
        .expect("trash partner prompt must install");
    let non_pass_count = sel.valid_action_ids.iter().filter(|&&a| a != PASS).count();
    assert_eq!(
        non_pass_count, 2,
        "both Kimeramon-named trash cards must be selectable as the DNA material; got {non_pass_count}"
    );
}

/// Declining the outer [On Deletion] "may" leaves the board untouched
/// (no merge; Kimeramon stays in trash, Machinedramon stays on field,
/// Millenniummon stays in hand).
#[test]
fn bt18_015_on_deletion_declining_leaves_board_untouched() {
    let mut runner = kimeramon_runner().memory(10).start();
    let kimeramon = runner.place_on_field(0, "BT18-015", Some(0));
    runner.place_on_field(0, "ST5-12", None);
    push_to_hand(&mut runner, 0, "BT18-019");

    runner.game.delete_permanent_with_cause(
        kimeramon,
        digimon_engine::replacement::ReplacementCause::OwnEffect,
    );
    runner.game.drain_effect_queue();

    let sel = runner
        .game
        .pending_selection
        .as_ref()
        .expect("outer accept prompt");
    let player = sel.selecting_player;
    runner.game.resolve_selection(player, PASS).ok();
    runner.game.drain_effect_queue();

    assert!(
        runner.game.pending_selection.is_none(),
        "declining the outer 'may' must end the clause with no further prompts"
    );
    assert_eq!(
        runner.game.players[0].hand.len(),
        1,
        "Millenniummon must remain in hand when the rider is declined"
    );
    assert!(
        has_named_on_field(&runner, 0, "Machinedramon"),
        "Machinedramon must remain on the field when the rider is declined"
    );
    assert!(
        !runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "BT18-019"),
        "no Millenniummon permanent must appear on the field when declined"
    );
}

/// Cost firing (event log): the on_deletion rider's DNA merge pays the
/// result's printed DNA cost (0 for BT18-019) — memory must be unchanged by
/// the merge itself (only the earlier WD/WA clause's own delete costs
/// nothing in memory either; this isolates the on_deletion path).
#[test]
fn bt18_015_on_deletion_merge_pays_printed_dna_cost_of_zero() {
    let mut runner = kimeramon_runner().memory(10).start();
    let kimeramon = runner.place_on_field(0, "BT18-015", Some(0));
    runner.place_on_field(0, "ST5-12", None);
    push_to_hand(&mut runner, 0, "BT18-019");

    let memory_before = runner.memory();

    runner.game.delete_permanent_with_cause(
        kimeramon,
        digimon_engine::replacement::ReplacementCause::OwnEffect,
    );
    runner.game.drain_effect_queue();
    resolve_all_picking_first_non_pass(&mut runner);

    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "BT18-019"),
        "merge must have completed"
    );
    assert_eq!(
        runner.memory(),
        memory_before,
        "BT18-019's printed DNA cost is 0 — memory must be unchanged by the merge"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 6 — Inherited ＜Security A. +1＞
// ═══════════════════════════════════════════════════════════════════════════

/// Positive: a Digimon carrying BT18-015 as a digivolution source inherits
/// SecurityAttackPlus(1) via the stack-walk and checks 1 additional security
/// card (base 1 + 1 = 2).
#[test]
fn bt18_015_inherited_security_attack_plus_grants_to_carrier_via_stack() {
    let mut runner = kimeramon_runner()
        .add_card(make_digimon("PLAIN-CARRIER", 6, 11000))
        .memory(10)
        .start();

    // Stack: bottom = BT18-015 (the inherited-effect source), top = carrier.
    let carrier = runner.place_stack(0, &["BT18-015", "PLAIN-CARRIER"]);
    // Inherited declarative grant_keyword grants materialize into the keyword
    // registry on a tick (combat's current_security_strike ticks before every
    // real attack; direct queries must tick explicitly).
    runner.game_mut().tick_declarative_effects();

    assert!(
        runner
            .game
            .has_keyword(carrier, Keyword::SecurityAttackPlus(1)),
        "a carrier with BT18-015 in its digivolution stack must inherit SecurityAttackPlus(1)"
    );
    assert_eq!(
        runner.game.effective_security_strike(carrier),
        2,
        "the carrier checks 1 (base) + 1 (inherited <Security A. +1>) = 2 security cards"
    );
}

/// Negative: a standalone face-up BT18-015 permanent does NOT self-grant the
/// inherited keyword — the grant lives on the Inherited Effect line only.
#[test]
fn bt18_015_standalone_does_not_self_grant_security_attack_plus() {
    let mut runner = kimeramon_runner().memory(10).start();
    let kimeramon = runner.place_on_field(0, "BT18-015", Some(0));
    runner.game_mut().tick_declarative_effects();

    assert!(
        !runner
            .game
            .has_keyword(kimeramon, Keyword::SecurityAttackPlus(1)),
        "a standalone face-up BT18-015 must NOT have SecurityAttackPlus(1) — it is inherited-only"
    );
    assert_eq!(
        runner.game.effective_security_strike(kimeramon),
        1,
        "a standalone BT18-015 checks the base 1 security card"
    );
}
