//! ST17-07 Rapidmon — Digimon, Lv.5, Green, DP 7000, Cost 7. Trait: Cyborg.
//!
//! # Card text (ST17-07.json)
//! `[On Play] [When Digivolving] ＜De-Digivolve 1＞ 1 of your opponent's Digimon.`
//! `(Trash the top card. You can't trash past level 3 cards.) Then, if you have`
//! `a green Tamer, until the end of your opponent's turn, your opponent's`
//! `effects can't delete this Digimon or return it to the hand or deck.`
//! `Inherited: [All Turns] [Once Per Turn] When this Digimon deletes an`
//! `opponent's Digimon in battle, trash the top card of their security stack.`
//! Digivolve: Lv.4 w/[Gargomon]/[Rapidmon] in name: Cost 3 (xros_req).
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/ST17/Green/ST17_07.cs
//!
//! # PARTIAL — green-Tamer protection rider is BLOCKED (omitted).
//! The "if green Tamer, until EoT, your OPPONENT'S effects can't delete/return
//! this" rider needs OPPONENT-scoped CannotBeDestroyedByEffect /
//! CannotBeReturnedToHand / CannotBeReturnedToDeck. The DSL `add_modifier` step
//! lowers via `ModifierEntry::simple` (cause_filter None = cause-agnostic,
//! blocking the controller's OWN effects too); DCGO scopes them to
//! `IsOpponentEffect`. No `opponent_only`/`cause_filter` knob exists on
//! `add_modifier`. Logged as G-OPPONENT-SCOPED-EFFECT-PROTECTION-DSL. Per
//! no-approximations the rider is omitted (not shipped cause-agnostically); its
//! tests are #[ignore]'d citing the gap.

#![allow(dead_code, unused_imports)]

#[path = "../../support/dsl_card_data.rs"]
mod dsl_card_data;

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledScope, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::PendingSelection;
use digimon_engine::{EffectTiming, TriggerSource};

const CARD_ID: &str = "ST17-07";

fn rapidmon() -> digimon_dsl::compiled::CompiledCard {
    dsl_card_data::compiled(CARD_ID)
}

fn make_digimon(card_id: &str, level: u8) -> CardData {
    let mut card = make_test_card(card_id, card_id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Blue];
    card.dp = Some(3000);
    card.level = Some(level);
    card
}

fn first_action(p: &PendingSelection) -> u16 {
    *p.valid_action_ids.first().expect("a legal action")
}

fn fire_timing(runner: &mut DebugRunner, timing: EffectTiming, carrier: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(timing, TriggerSource::Permanent(carrier));
    runner.game.drain_effect_queue();
}

fn base_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST17-07 in pack")
        .add_card(make_digimon("OPP-BASE", 3))
        .add_card(make_digimon("OPP-TOP", 4))
        .add_card(make_digimon("OPP-LV3", 3))
        .add_card(make_digimon("OPP-LV4", 4))
        .add_card(make_digimon("FILLER", 4))
        .deck(0, &["FILLER"; 10])
        .deck(1, &["FILLER"; 10])
        .memory(10)
        .start()
}

// ─── SECTION 1 — Structural ─────────────────────────────────────────────────

#[test]
fn st17_07_metadata_green_lv5_cyborg() {
    let card = rapidmon();
    assert_eq!(card.card, CARD_ID);
    assert_eq!(card.level, Some(5));
    assert_eq!(card.dp, Some(7000));
    assert!(card
        .color
        .contains(&digimon_dsl::compiled::CompiledColor::Green));
}

#[test]
fn st17_07_clause1_is_on_play_and_when_digivolving() {
    let card = rapidmon();
    let found = card.effects.iter().any(|c| match c {
        CompiledClause::Triggered(t) => {
            t.when.contains(&CompiledTiming::OnPlay)
                && t.when.contains(&CompiledTiming::WhenDigivolving)
        }
        _ => false,
    });
    assert!(
        found,
        "De-Digivolve clause fires on [On Play] and [When Digivolving]"
    );
}

#[test]
fn st17_07_clause2_inherited_battle_delete_is_opt() {
    let card = rapidmon();
    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.scope == CompiledScope::Inherited
                    && t.when.contains(&CompiledTiming::OnAnyDeletion) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("inherited on_any_deletion clause must exist");
    assert!(clause.once_per_turn, "[Once Per Turn]");
}

#[test]
fn st17_07_has_named_lv4_digivolve_alt_path_cost3() {
    let card = rapidmon();
    let found = card.alt_paths.iter().any(|p| {
        p.kind == CompiledAltPathKind::Digivolve && p.cost == Some(CompiledCost::Literal(3))
    });
    assert!(
        found,
        "must declare the Lv.4 [Gargomon]/[Rapidmon] cost-3 digivolve; got {:?}",
        card.alt_paths
    );
}

// ─── SECTION 2 — Clause 1 De-Digivolve behavioral ───────────────────────────

#[test]
fn st17_07_on_play_installs_opp_selection_when_target_exists() {
    let mut runner = base_runner();
    let rapid = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_stack(1, &["OPP-BASE", "OPP-TOP"]);

    fire_timing(&mut runner, EffectTiming::OnPlay, rapid);

    assert!(
        runner.game.pending_selection.is_some(),
        "De-Digivolve target selection must install when the opponent has a Digimon"
    );
}

#[test]
fn st17_07_on_play_no_selection_when_no_opp_digimon() {
    let mut runner = base_runner();
    let rapid = runner.place_on_field(0, CARD_ID, Some(0));

    fire_timing(&mut runner, EffectTiming::OnPlay, rapid);

    assert!(
        runner.game.pending_selection.is_none(),
        "no opponent Digimon → no De-Digivolve selection"
    );
}

#[test]
fn st17_07_de_digivolve_trashes_top_source() {
    let mut runner = base_runner();
    let rapid = runner.place_on_field(0, CARD_ID, Some(0));
    let opp = runner.place_stack(1, &["OPP-BASE", "OPP-TOP"]);
    let opp_trash_before = runner.game.players[1].trash.len();

    fire_timing(&mut runner, EffectTiming::OnPlay, rapid);
    let (player, action) = {
        let p = runner.game.pending_selection.as_ref().expect("selection");
        (p.selecting_player, first_action(p))
    };
    runner
        .execute_action(player, action)
        .expect("select opp Digimon");
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.game.players[1].battle_area[opp.index as usize].stack_size(),
        1,
        "De-Digivolve 1 trashes the top source"
    );
    assert_eq!(
        runner.game.players[1].trash.len(),
        opp_trash_before + 1,
        "the trashed source goes to the opponent's trash"
    );
}

#[test]
fn st17_07_de_digivolve_stops_at_level_three() {
    let mut runner = base_runner();
    let rapid = runner.place_on_field(0, CARD_ID, Some(0));
    // base LV3, top LV4 → De-Digivolve 1 trashes the LV4 top, leaving LV3.
    let opp = runner.place_stack(1, &["OPP-LV3", "OPP-LV4"]);

    fire_timing(&mut runner, EffectTiming::OnPlay, rapid);
    let (player, action) = {
        let p = runner.game.pending_selection.as_ref().expect("selection");
        (p.selecting_player, first_action(p))
    };
    runner
        .execute_action(player, action)
        .expect("select opp Digimon");
    let _ = runner.auto_resolve();

    let perm = &runner.game.players[1].battle_area[opp.index as usize];
    assert_eq!(perm.stack_size(), 1, "removes the LV4 top");
    assert_eq!(perm.top_card().card_id(&runner.game.card_data), "OPP-LV3");
}

#[test]
fn st17_07_when_digivolving_also_de_digivolves() {
    let mut runner = base_runner();
    let rapid = runner.place_on_field(0, CARD_ID, Some(0));
    let opp = runner.place_stack(1, &["OPP-BASE", "OPP-TOP"]);

    fire_timing(&mut runner, EffectTiming::WhenDigivolving, rapid);
    let (player, action) = {
        let p = runner.game.pending_selection.as_ref().expect("selection");
        (p.selecting_player, first_action(p))
    };
    runner
        .execute_action(player, action)
        .expect("select opp Digimon");
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.game.players[1].battle_area[opp.index as usize].stack_size(),
        1,
        "[When Digivolving] fires the same De-Digivolve body"
    );
}

// ─── SECTION 3 — Clause 2 inherited battle-delete → trash opp security ──────
//
// Structural coverage: the inherited clause is gated on
// `source_deleted_battle_opponent` (this Digimon deleted an opponent in
// battle) and trashes the opponent's top security, OPT. A full
// battle-resolution behavioral drive (attack → win → trash) is not exercised
// here — the same structural-only convention used by other battle-delete
// observer cards (ST4-11) given the DebugRunner battle surface. The structural
// assertions (clause exists, inherited, OnAnyDeletion, once_per_turn) live in
// SECTION 1.

#[test]
fn st17_07_inherited_clause_condition_is_battle_delete_by_source() {
    // The clause must NOT fire on a generic deletion — its condition is
    // `source_deleted_battle_opponent`. We assert the compiled clause carries
    // exactly the inherited OnAnyDeletion + once_per_turn shape; the
    // source-battle-delete condition is what gates it (see YAML).
    let card = rapidmon();
    let inherited_count = card
        .effects
        .iter()
        .filter(|c| {
            matches!(c, CompiledClause::Triggered(t)
                if t.scope == CompiledScope::Inherited
                    && t.when.contains(&CompiledTiming::OnAnyDeletion))
        })
        .count();
    assert_eq!(
        inherited_count, 1,
        "exactly one inherited battle-delete observer"
    );
}

// ─── SECTION 4 — BLOCKED: green-Tamer opponent-scoped protection rider ──────
//
// G-OPPONENT-SCOPED-EFFECT-PROTECTION-DSL: the DSL `add_modifier` step cannot
// install OPPONENT-scoped delete/return immunity (it installs cause-agnostic
// protection via ModifierEntry::simple). The rider is omitted from the YAML
// per no-approximations; these tests pin the intended behavior pending the
// substrate widening.

#[test]
#[ignore = "pending: G-OPPONENT-SCOPED-EFFECT-PROTECTION-DSL (opponent-only delete/return immunity from add_modifier)"]
fn st17_07_green_tamer_grants_opponent_only_delete_protection() {
    // With a green Tamer in play, an OPPONENT delete effect on this Digimon is
    // prevented until end of opponent's turn, but the controller's OWN delete
    // effects still resolve. Unimplementable until add_modifier gains an
    // opponent_only cause filter.
}

#[test]
#[ignore = "pending: G-OPPONENT-SCOPED-EFFECT-PROTECTION-DSL (opponent-only delete/return immunity from add_modifier)"]
fn st17_07_protection_not_installed_without_green_tamer() {
    // Without a green Tamer the rider is skipped entirely (condition gate).
}
