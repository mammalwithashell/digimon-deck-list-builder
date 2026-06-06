//! BT25-011 Aquilamon — Digimon, Lv.4, Red/Green, DP 4000, Cost 4.
//! Traits: Giant Bird, Iliad, TS.
//!
//! # Card text (cards.json — verbatim)
//!
//! <Raid> (When this Digimon attacks, you may switch the target of attack to 1
//! of your opponent's unsuspended Digimon with the highest DP.)
//! [On Play] [When Digivolving] Suspend 1 of your opponent's Digimon. Then, if
//! it's your turn, 2 of your Digimon may DNA digivolve into [Silphymon] in the
//! hand.
//!
//! Inherited: [Your Turn] This Digimon gets +2000 DP.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Red/BT25_011.cs
//!
//! - Raid (OnAllyAttack): RaidSelfEffect.
//! - OP/WD Shared: MANDATORY suspend 1 opp Digimon; then if owner turn, DNA
//!   digivolve 2 own Digimon into [Silphymon] (in hand), optional.
//! - Inherit (None, inherited): +2000 DP, owner-turn gated.
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - H9 Raid keyword
//! - F7-adjacent suspend on play
//! - G2 effect-initiated DNA digivolve (into hand) — your-turn gated
//! - D1 inherited self +DP (your turn)

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep,
    CompiledTiming,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, Keyword, PlayerId};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT25-011";

fn make_digimon(id: &str, level: u8, dp: i32, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card.play_cost = Some(4);
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    card.evo_costs = vec![EvoCost {
        card_color: 0,
        level: level.saturating_sub(1),
        memory_cost: 3,
    }];
    card
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-011 YAML parses and compiles")
        .add_card(make_test_card("DECK-PAD", "Filler"))
        .add_card(make_digimon("OPP-A", 4, 5000, &["Beast"]))
        .add_card(make_digimon("OWN-A", 4, 4000, &["Beast"]))
        .add_card(make_digimon("CARRIER", 5, 7000, &["Beast"]))
        .deck(0, &["DECK-PAD"; 12])
        .deck(1, &["DECK-PAD"; 12])
}

fn is_suspended(runner: &DebugRunner, h: PermanentHandle) -> bool {
    runner.game.players[h.player as usize].battle_area[h.index as usize].is_suspended
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn bt25_011_yaml_has_printed_metadata() {
    let runner = base().start();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("BT25-011 present in pack");
    assert_eq!(card.name, "Aquilamon");
    assert_eq!(card.level, Some(4));
    assert_eq!(card.dp, Some(4000));
    for t in ["Giant Bird", "Iliad", "TS"] {
        assert!(card.traits.contains(&t.to_string()), "trait {t}");
    }
}

#[test]
fn bt25_011_grants_raid() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let grants_raid = card.effects.iter().any(|c| matches!(
        c,
        CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { keyword, .. })
            if keyword.eq_ignore_ascii_case("Raid")
    ));
    assert!(grants_raid, "BT25-011 must grant <Raid>");
}

#[test]
fn bt25_011_has_shared_on_play_when_digivolving_clause_with_dna_step() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnPlay)
                    && t.when.contains(&CompiledTiming::WhenDigivolving) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("shared clause present");
    // The clause must contain the DNA-digivolve step (gated under the your-turn if).
    fn has_dna(steps: &[CompiledStep]) -> bool {
        steps.iter().any(|s| match s {
            CompiledStep::MayDnaDigivolveNow { .. } => true,
            CompiledStep::If { then, otherwise, .. } => {
                has_dna(then) || otherwise.as_ref().map_or(false, |e| has_dna(e))
            }
            _ => false,
        })
    }
    assert!(
        has_dna(&clause.process),
        "shared clause must contain a may_dna_digivolve_now step"
    );
}

#[test]
fn bt25_011_has_inherited_dp_aura() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let aura = card.effects.iter().find_map(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
            dp_modifier,
            scope,
            ..
        }) => Some((*dp_modifier, *scope)),
        _ => None,
    });
    assert_eq!(
        aura,
        Some((Some(2000), CompiledScope::Inherited)),
        "BT25-011 must have an inherited +2000 DP aura"
    );
}

// ─── Section 2 — Behavior: suspend on play ───────────────────────────────────

/// Positive: playing Aquilamon with an opponent Digimon present installs a
/// prompt (the mandatory suspend pick).
#[test]
fn bt25_011_on_play_installs_suspend_prompt() {
    let mut runner = base().hand(0, &[CARD_ID]).memory(6).start();
    runner.place_on_field(1, "OPP-A", Some(0));

    let _a = runner.play(0, 0).expect("play Aquilamon");
    assert!(
        runner.pending_selection().is_some(),
        "OnPlay must install the suspend prompt over the opponent Digimon"
    );
}

/// Behavioral: resolving the OnPlay suspends an opponent Digimon.
#[test]
fn bt25_011_on_play_suspends_an_opponent_digimon() {
    let mut runner = base().hand(0, &[CARD_ID]).memory(6).start();
    let opp = runner.place_on_field(1, "OPP-A", Some(0));

    let _a = runner.play(0, 0).expect("play Aquilamon");
    runner.auto_resolve().ok();

    assert!(
        is_suspended(&runner, opp),
        "the chosen opponent Digimon must be suspended"
    );
}

/// Negative: with no opponent Digimon, the suspend is a no-op (no prompt for it).
#[test]
fn bt25_011_on_play_no_suspend_prompt_without_opponent_digimon() {
    let mut runner = base().hand(0, &[CARD_ID]).memory(6).start();

    let _a = runner.play(0, 0).expect("play Aquilamon");
    // No opponent Digimon to suspend; and no own Digimon other than Aquilamon to
    // DNA — the clause should be a no-op (no install) or resolve immediately.
    runner.auto_resolve().ok();
    assert!(
        runner.pending_selection().is_none(),
        "with no targets the OnPlay clause resolves without leaving a prompt"
    );
}

// ─── Section 3 — Behavior: inherited +2000 DP on your turn ───────────────────

#[test]
fn bt25_011_inherited_aura_buffs_carrier_on_your_turn() {
    let mut runner = base().memory(6).start();
    // Aquilamon as a digivolution source beneath a carrier (last id = top).
    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);

    let dp = runner.effective_dp(carrier).expect("carrier DP");
    assert_eq!(
        dp, 9000,
        "inherited [Your Turn] +2000 must apply (carrier base 7000)"
    );
}

#[test]
fn bt25_011_inherited_aura_inactive_on_opponents_turn() {
    let mut runner = base().memory(6).start();
    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    runner.end_turn();

    let dp = runner.effective_dp(carrier).expect("carrier DP");
    assert_eq!(
        dp, 7000,
        "on the opponent's turn the [Your Turn] aura is inactive (base 7000)"
    );
}
