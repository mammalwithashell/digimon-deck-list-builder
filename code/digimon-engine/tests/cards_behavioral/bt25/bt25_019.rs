//! BT25-019 UltimateBrachiomon — Digimon, Lv.6, DP 13000, Cost 13.
//! Traits: Cyborg, X Antibody, Titan, TS. Colors: White/Black (printed [0,5]).
//! Evo: Lv.5 colorless / cost 5 (printed; + DCGO alt on a [TS]/[Dinosaur] Lv.5
//! base, cost 4).
//!
//! # Card text (cards.json — verbatim)
//!
//! <Reboot> (This Digimon also unsuspends in your opponent's unsuspend phase.)
//! <Blocker> (This Digimon can block in the blocker timing.)
//! [On Play] [When Digivolving] Delete 1 of your opponent's highest DP Digimon.
//! [End of Your Turn] [Once Per Turn] If your opponent has 5 or more memory,
//! their Digimon effects don't affect this Digimon until their turn ends. Then,
//! if they have 5 or less, their Option effects don't affect this Digimon until
//! their turn ends.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Red/BT25_019.cs
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - A (keyword grants: Reboot, Blocker)
//! - C (OnPlay/WhenDigivolving mandatory highest-DP delete)
//! - F (memory-gated effect-immunity grants at EoT)
//! - H (alt digivolution path)

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledScope,
    CompiledStep, CompiledTiming,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, EffectSourceKind, EffectTiming, PlayerId};
use digimon_engine::selection::TriggerSource;
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT25-019";

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn opp_digimon(id: &str, dp: i32) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(5);
    card.dp = Some(dp);
    card.traits = vec!["Beast".to_string()];
    card
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-019 YAML parses and compiles")
        .add_card(make_test_card("PAD", "Filler"))
        .add_card(opp_digimon("OPP-HIGH", 11000))
        .add_card(opp_digimon("OPP-LOW", 3000))
        .deck(0, &["PAD"; 8])
        .deck(1, &["PAD"; 8])
}

fn place_brachio(runner: &mut DebugRunner) -> PermanentHandle {
    runner.place_on_field(0, CARD_ID, Some(0))
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn bt25_019_metadata() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    assert_eq!(card.name, "UltimateBrachiomon");
    assert_eq!(card.level, Some(6));
    assert_eq!(card.dp, Some(13000));
    assert_eq!(card.cost, Some(13));
}

#[test]
fn bt25_019_has_reboot_and_blocker() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    for kw in ["Reboot", "Blocker"] {
        let has = card.effects.iter().any(|c| matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { keyword, .. })
                if keyword == kw
        ));
        assert!(has, "BT25-019 must grant {kw}");
    }
}

#[test]
fn bt25_019_delete_clause_is_mandatory_on_play_and_digivolve() {
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
        .expect("must have OnPlay/WhenDigivolving delete clause");
    assert!(!clause.optional, "delete clause is mandatory (no 'may')");
    assert!(
        clause
            .process
            .iter()
            .any(|s| matches!(s, CompiledStep::DeletePermanent { .. })),
        "delete clause must contain a delete_permanent step"
    );
}

#[test]
fn bt25_019_eot_clause_is_opt() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::EndOfYourTurn) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("must have an EndOfYourTurn clause");
    assert!(clause.once_per_turn, "[Once Per Turn] → once_per_turn");
    let immunity_grants = clause
        .process
        .iter()
        .filter(|s| matches!(s, CompiledStep::If { .. }))
        .count();
    assert_eq!(
        immunity_grants, 2,
        "EoT clause must have two conditional immunity grants (digimon then option)"
    );
}

#[test]
fn bt25_019_has_ts_or_dinosaur_lv5_cost4_alt_path() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let path = card
        .alt_paths
        .iter()
        .find(|p| p.kind == CompiledAltPathKind::Digivolve)
        .expect("must have a digivolve alt-path");
    assert_eq!(path.cost, Some(CompiledCost::Literal(4)));
}

// ─── Section 2 — Behavior: highest-DP delete ─────────────────────────────────

/// On play, the highest-DP opponent Digimon is deleted (the lower-DP one is
/// not a legal target).
#[test]
fn bt25_019_deletes_highest_dp_opponent() {
    let mut runner = base().start();
    let brachio = place_brachio(&mut runner);
    let high = runner.place_on_field(1, "OPP-HIGH", Some(0));
    let low = runner.place_on_field(1, "OPP-LOW", Some(0));

    runner.fire_on_play(0, brachio.index as usize);
    if runner.pending_selection().is_some() {
        runner.auto_resolve().expect("resolve delete select");
    }

    assert!(
        !runner.game.players[1].battle_area.iter().any(|p| {
            p.top_card().card_id(&runner.game.card_data) == "OPP-HIGH"
        }),
        "highest-DP opponent Digimon must be deleted"
    );
    assert!(
        runner.game.players[1].battle_area.iter().any(|p| {
            p.top_card().card_id(&runner.game.card_data) == "OPP-LOW"
        }),
        "the lower-DP Digimon is not a legal highest-DP target and survives"
    );
}

/// Negative: opponent has no Digimon → delete clause is a no-op.
#[test]
fn bt25_019_delete_no_fire_without_opponent_digimon() {
    let mut runner = base().start();
    let brachio = place_brachio(&mut runner);
    runner.fire_on_play(0, brachio.index as usize);
    assert!(
        runner.pending_selection().is_none(),
        "no opponent Digimon → delete installs no prompt"
    );
}

// ─── Section 3 — Behavior: EoT memory-gated immunity ─────────────────────────

/// Fire the EndOfYourTurn trigger.
fn fire_eot(runner: &mut DebugRunner, brachio: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(EffectTiming::EndOfYourTurn, TriggerSource::Permanent(brachio));
    runner.game.drain_effect_queue();
    if runner.pending_selection().is_some() {
        runner.auto_resolve().expect("resolve EoT immunity");
    }
}

/// Opponent has >=5 memory (gauge at -7) → only Digimon-effect immunity.
#[test]
fn bt25_019_eot_opp_high_memory_grants_digimon_immunity_only() {
    let mut runner = base().memory(-7).start();
    let brachio = place_brachio(&mut runner);
    fire_eot(&mut runner, brachio);

    assert!(
        runner
            .game
            .permanent_is_unaffected_by_effect(brachio, 1, EffectSourceKind::Digimon),
        "opponent >=5 memory → immune to opponent Digimon effects"
    );
    assert!(
        !runner
            .game
            .permanent_is_unaffected_by_effect(brachio, 1, EffectSourceKind::Option),
        "opponent >5 memory → NOT immune to opponent Option effects"
    );
}

/// Opponent has <5 memory (gauge at -2 → opponent has 2) → only Option-effect
/// immunity.
#[test]
fn bt25_019_eot_opp_low_memory_grants_option_immunity_only() {
    let mut runner = base().memory(-2).start();
    let brachio = place_brachio(&mut runner);
    fire_eot(&mut runner, brachio);

    assert!(
        !runner
            .game
            .permanent_is_unaffected_by_effect(brachio, 1, EffectSourceKind::Digimon),
        "opponent <5 memory → NOT immune to opponent Digimon effects"
    );
    assert!(
        runner
            .game
            .permanent_is_unaffected_by_effect(brachio, 1, EffectSourceKind::Option),
        "opponent <=5 memory → immune to opponent Option effects"
    );
}

/// Opponent has exactly 5 memory (gauge at -5) → BOTH immunities fire.
#[test]
fn bt25_019_eot_opp_exactly_5_memory_grants_both() {
    let mut runner = base().memory(-5).start();
    let brachio = place_brachio(&mut runner);
    fire_eot(&mut runner, brachio);

    assert!(
        runner
            .game
            .permanent_is_unaffected_by_effect(brachio, 1, EffectSourceKind::Digimon),
        "opponent exactly 5 → Digimon immunity fires (>=5)"
    );
    assert!(
        runner
            .game
            .permanent_is_unaffected_by_effect(brachio, 1, EffectSourceKind::Option),
        "opponent exactly 5 → Option immunity fires (<=5)"
    );
}
