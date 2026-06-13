//! BT25-015 Garudamon — Digimon, Lv.5, Red/Yellow, DP 7000, Cost 7.
//! Traits: Birdkin, Iliad, TS.  Attribute: Vaccine.
//! Evo: Lv.4 Red / cost 4 (printed evo_costs).
//!
//! # Card text (cards.json — verbatim)
//!
//! <Raid> (When this Digimon attacks, you may switch the target of attack to 1
//!   of your opponent's unsuspended Digimon with the highest DP.)
//! <Fortitude> (When this Digimon with digivolution cards is deleted, play this
//!   card without paying the cost.)
//! [On Play] [When Digivolving] Delete 1 of your opponent's Digimon with 6000
//!   DP or less.
//!
//! Inherited Effect:
//! [All Turns] [Once Per Turn] When this Digimon deletes your opponent's Digimon
//!   in battle, trash their top security card.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Red/BT25_015.cs
//!   - None: AddSelfDigivolutionRequirementStaticEffect(level 4,
//!     Giant Bird || HasTSTraits, cost 3).
//!   - OnAllyAttack: RaidSelfEffect.
//!   - OnDestroyedAnyone: FortitudeSelfEffect.
//!   - Shared OnPlay/WhenDigivolving (mandatory): SelectPermanentEffect over opp
//!     Digimon with DP <= 6000, Mode.Destroy.
//!   - OnEndBattle (inherited, OPT): when this Digimon deletes opp Digimon in
//!     battle, trash opp top security.
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - H9 <Raid> keyword grant
//! - <Fortitude> keyword grant (OnDeletion replay)
//! - F: [On Play][When Digivolving] mandatory delete with dp_lte 6000 filter
//! - F5/H4: inherited [All Turns][OPT] battle-delete trash-top-security
//! - alt-digivolution Lv.4 [TS] (or Giant Bird) → cost 3

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledScope, CompiledTiming,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card_with_level, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::CardColor;
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT25-015";

// ─── Fixture helpers ─────────────────────────────────────────────────────────

/// An opponent Digimon with a given DP, used as a delete target.
fn opp_digimon(card_id: &str, dp: i32) -> CardData {
    let mut card = make_test_card_with_level(card_id, card_id, 4);
    card.colors = vec![CardColor::Blue];
    card.dp = Some(dp);
    card.play_cost = 4;
    card
}

fn push_to_field(runner: &mut DebugRunner, player: u8, card_id: &str) -> PermanentHandle {
    runner.place_on_field(player, card_id, Some(0))
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-015 YAML parses and compiles")
        .add_card(opp_digimon("OPP-6000", 6000))
        .add_card(opp_digimon("OPP-7000", 7000))
        .add_card(make_test_card_with_level("FILL", "Filler", 3))
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .security(1, &["FILL", "FILL", "FILL"])
        .memory(10)
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn bt25_015_yaml_has_printed_metadata() {
    let runner = base().start();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("BT25-015 must be in the embedded DSL pack");

    assert_eq!(card.name, "Garudamon");
    assert_eq!(card.level, Some(5));
    assert_eq!(card.dp, Some(7000));
    for trait_name in ["Birdkin", "Iliad", "TS"] {
        assert!(
            card.traits.contains(&trait_name.to_string()),
            "BT25-015 metadata must include trait {trait_name}"
        );
    }
}

#[test]
fn bt25_015_has_ts_alt_digivolution_path() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let has_ts = card.alt_paths.iter().any(|p| {
        p.kind == CompiledAltPathKind::Digivolve
            && p.cost == Some(CompiledCost::Literal(3))
            && p.from
                .as_ref()
                .is_some_and(|f| f.level_eq == Some(4) && f.trait_has.as_deref() == Some("TS"))
    });
    assert!(has_ts, "BT25-015 must have a Lv.4 [TS] → cost 3 alt-path");
}

#[test]
fn bt25_015_grants_raid_and_fortitude() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let decl_dbg: Vec<String> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Declarative(d) => Some(format!("{d:?}")),
            _ => None,
        })
        .collect();
    let blob = decl_dbg.join(" | ");
    assert!(blob.contains("Raid"), "BT25-015 must grant <Raid>; decls = {blob}");
    assert!(
        blob.contains("Fortitude"),
        "BT25-015 must grant <Fortitude>; decls = {blob}"
    );
}

#[test]
fn bt25_015_has_on_play_delete_and_inherited_ess_clauses() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    let has_op_wd = triggered.iter().any(|t| {
        t.when.contains(&CompiledTiming::OnPlay) && t.when.contains(&CompiledTiming::WhenDigivolving)
    });
    let has_inherited_ess = triggered.iter().any(|t| {
        t.scope == CompiledScope::Inherited
            && t.when.contains(&CompiledTiming::OnAnyDeletion)
            && t.once_per_turn
    });
    assert!(has_op_wd, "must have [On Play][When Digivolving] delete clause");
    assert!(
        has_inherited_ess,
        "must have inherited [All Turns][OPT] battle-delete trash-top-security clause"
    );
}

// ─── Section 2 — [On Play][When Digivolving] delete DP <= 6000 ───────────────

#[test]
fn bt25_015_on_play_offers_delete_of_opp_digimon_at_6000_or_less() {
    let mut runner = base().start();
    runner.game.turn_count = 1;
    let opp = push_to_field(&mut runner, 1, "OPP-6000");
    let g = push_to_field(&mut runner, 0, CARD_ID);
    runner.fire_on_play(0, g.index as usize);

    assert!(
        runner.pending_kind().is_some(),
        "OnPlay must offer a delete of an opponent Digimon with DP <= 6000"
    );
    runner.auto_resolve().ok();
}

#[test]
fn bt25_015_on_play_does_not_target_opp_digimon_above_6000() {
    let mut runner = base().start();
    runner.game.turn_count = 1;
    let opp = push_to_field(&mut runner, 1, "OPP-7000");
    let g = push_to_field(&mut runner, 0, CARD_ID);
    runner.fire_on_play(0, g.index as usize);

    assert!(
        runner.pending_selection().is_none(),
        "a 7000-DP opponent Digimon is out of the DP <= 6000 range — no delete installs"
    );
    runner.auto_resolve().ok();
}

// ─── Section 3 — delete behavioral outcome ───────────────────────────────────

#[test]
fn bt25_015_on_play_delete_removes_the_6000_opp_digimon() {
    let mut runner = base().start();
    runner.game.turn_count = 1;
    let opp = push_to_field(&mut runner, 1, "OPP-6000");
    let opp_before = runner.battle_area_size(1);

    let g = push_to_field(&mut runner, 0, CARD_ID);
    runner.fire_on_play(0, g.index as usize);
    runner.auto_resolve().ok();

    assert_eq!(
        runner.battle_area_size(1),
        opp_before - 1,
        "the 6000-DP opponent Digimon must be deleted by Garudamon's OnPlay"
    );
}
