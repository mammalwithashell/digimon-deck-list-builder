//! BT25-104 ShineGreymon: Burst Mode // Final Shining Burst — DUAL card.
//! Digimon face: Lv.7, Red+Yellow, DP 15000. Option face: Use 6, Yellow.
//! Traits: Light Dragon, DATA SQUAD.
//!
//! # Card text (card image BT25-104 — authoritative for printed text)
//!
//! Digimon face:
//!   <Raid> <Piercing> <Security A. +1> <Blocker> <Barrier>
//!   [When Digivolving] [When Attacking] [Once Per Turn] Activate 1 [Main]
//!     effect on this card's Option side.
//!   [Your Turn] All of your [Marcus Damon]s are also treated as 12000 DP
//!     Digimon and gain <Rush>.
//! Option face (Final Shining Burst):
//!   <Use Req. ([DATA SQUAD] trait)>
//!   [Main] 1 of your opponent's Digimon gets -15000 DP for the turn. Then, you
//!     may play 1 Tamer card from your hand without paying the cost.
//!   <Arts Digivolve>
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Red/BT25_104.cs
//!
//! # Status: PARTIAL — the [Your Turn] mass-Marcus aura is OMITTED.
//! GAP G-DSL-AURA-TREAT-AS-DIGIMON-SYNTH (qa/dsl-vocab-gaps.md): the DSL cannot
//! express a CONTINUOUS aura that treats ALL of your [Marcus Damon]s as 12000 DP
//! Digimon (the TreatAsDigimon + synth_identity substrate is one-shot/single-
//! target only; the `continuous` mass path drops the ModifierPayload). The
//! omitted clause is covered by an `#[ignore]`d documentation test.
//!
//! # Patterns this test covers
//! - DUAL card metadata (kind: dual, dual.digimon + dual.option faces)
//! - dual.option.keywords [ArtsDigivolve] + use_requirement
//! - Digimon-face static keyword grants (Raid/Piercing/SA+1/Blocker/Barrier)
//! - alt-paths: trait-filtered digivolve + burst_digivolve (ShineGreymon host)
//! - Option-face [Main]: -15000 DP + optional free Tamer play
//! - Digimon-face [WD][WA][OPT] inline of the Option [Main] body

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledScope,
    CompiledTiming, CompiledTriggeredClause,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

use super::super::dsl_card_data::compiled;

// ─── Card-data helpers ────────────────────────────────────────────────────────

/// An opponent Digimon (DP -15000 target). DP 16000 so a -15000 debuff leaves it
/// at a positive 1000 DP — it is NOT swept by the ≤0-DP state-based rule, so the
/// debuff magnitude is observable on a live permanent.
fn make_opp_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(7);
    c.dp = Some(16000);
    c.play_cost = 6;
    c.colors = vec![CardColor::Black];
    c
}

/// A Tamer in hand (free-play target).
fn make_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![CardColor::Yellow];
    c
}

fn make_filler(id: &str) -> CardData {
    make_test_card(id, id)
}

// ════════════════════════════════════════════════════════════════════════════
// Section 1 — DUAL metadata + structural assertions
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn bt25_104_compiles_from_dsl_pack() {
    let _ = compiled("BT25-104");
}

#[test]
fn bt25_104_is_a_dual_card() {
    let card = compiled("BT25-104");
    assert_eq!(card.kind, digimon_dsl::compiled::CompiledCardKind::Dual);
    let dual = card.dual.as_ref().expect("dual metadata must be present");
    assert_eq!(dual.digimon.level, 7);
    assert_eq!(dual.digimon.dp, 15000);
    assert_eq!(dual.option.use_cost, 6);
}

/// The Digimon face is Red+Yellow; the Option face is Yellow.
#[test]
fn bt25_104_face_colors() {
    let card = compiled("BT25-104");
    let dual = card.dual.as_ref().expect("dual metadata");
    use digimon_dsl::compiled::CompiledColor;
    assert!(dual.digimon.colors.contains(&CompiledColor::Red));
    assert!(dual.digimon.colors.contains(&CompiledColor::Yellow));
    assert_eq!(dual.option.colors, vec![CompiledColor::Yellow]);
}

/// The Option face carries <Arts Digivolve>.
#[test]
fn bt25_104_option_has_arts_digivolve() {
    let card = compiled("BT25-104");
    let dual = card.dual.as_ref().expect("dual metadata");
    assert!(
        dual.option.keywords.iter().any(|k| k == "ArtsDigivolve"),
        "Option face must carry the ArtsDigivolve keyword; got {:?}",
        dual.option.keywords
    );
}

/// The Option face carries a <Use Req. ([DATA SQUAD] trait)> use_requirement.
#[test]
fn bt25_104_option_has_use_requirement() {
    let card = compiled("BT25-104");
    let dual = card.dual.as_ref().expect("dual metadata");
    assert!(
        dual.option.use_requirement.is_some(),
        "Option face must carry a use_requirement (DATA SQUAD color-ignore)"
    );
}

/// The card grants the 5 static keywords: Raid, Piercing, SecurityAttackPlus,
/// Blocker, Barrier (declarative grant_keyword clauses on the digimon face).
#[test]
fn bt25_104_grants_five_static_keywords() {
    let card = compiled("BT25-104");
    let granted: Vec<&str> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword, ..
            }) => Some(keyword.as_str()),
            _ => None,
        })
        .collect();
    for kw in ["Raid", "Piercing", "SecurityAttackPlus", "Blocker", "Barrier"] {
        assert!(
            granted.contains(&kw),
            "BT25-104 must grant the {kw} keyword; granted = {granted:?}"
        );
    }
}

/// Alt-paths: a trait-filtered digivolve (cost 5) + a burst_digivolve (cost 0).
#[test]
fn bt25_104_has_digivolve_and_burst_alt_paths() {
    let card = compiled("BT25-104");
    assert!(
        card.alt_paths.iter().any(|p| p.kind == CompiledAltPathKind::Digivolve
            && p.cost == Some(CompiledCost::Literal(5))),
        "must have a cost-5 DATA SQUAD digivolve alt-path"
    );
    assert!(
        card.alt_paths.iter().any(|p| p.kind == CompiledAltPathKind::BurstDigivolve
            && p.cost == Some(CompiledCost::Literal(0))),
        "must have a cost-0 burst_digivolve alt-path"
    );
}

/// The Digimon-face [WD][WA][OPT] inline clause exists: it fires on both
/// WhenDigivolving and WhenAttacking and is [Once Per Turn].
#[test]
fn bt25_104_has_wd_wa_opt_inline_clause() {
    let card = compiled("BT25-104");
    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::WhenDigivolving)
                    && t.when.contains(&CompiledTiming::WhenAttacking) =>
            {
                Some(t)
            }
            _ => None,
        })
        .next()
        .expect("[WD][WA] inline option-main clause must exist");
    assert!(clause.once_per_turn, "the [WD][WA] inline clause is [Once Per Turn]");
}

/// The Option-face [Main] clause exists (fires on MainFromHand).
#[test]
fn bt25_104_has_option_main_clause() {
    let card = compiled("BT25-104");
    assert!(
        card.effects.iter().any(|c| matches!(
            c,
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::MainFromHand)
        )),
        "Option-face [Main] clause must exist"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Section 2 — Option-face [Main] behavioral (-15000 DP + free Tamer play)
// ════════════════════════════════════════════════════════════════════════════

/// Helper: drain all pending selections taking the first legal action.
fn drain(runner: &mut DebugRunner) {
    let mut steps = 0;
    while runner.game.pending_selection.is_some() && steps < 30 {
        let p = runner.game.pending_selection.as_ref().unwrap().selecting_player;
        let a = runner.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
        runner.game.resolve_selection(p, a).ok();
        runner.game.drain_effect_queue();
        steps += 1;
    }
}

/// The Option [Main] gives 1 opponent Digimon -15000 DP for the turn.
#[test]
fn bt25_104_option_main_debuffs_opponent_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT25-104")
        .expect("BT25-104 in pack")
        .add_card(make_opp_digimon("OPP1"))
        .add_card(make_tamer("TAMER1"))
        .add_card(make_filler("FILL"))
        .hand(0, &["BT25-104"])
        .hand(1, &["FILL"])
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let opp = runner.place_on_field(1, "OPP1", Some(0));
    let dp_before = runner.effective_dp(opp);

    // Drive the Option [Main] directly (MainFromHand timing).
    runner.game.activate_hand_main(0, 0);
    drain(&mut runner);

    let dp_after = runner.effective_dp(opp);
    assert_eq!(
        dp_after,
        dp_before.map(|d| d - 15000),
        "opponent Digimon must get -15000 DP for the turn"
    );
}

/// The Option [Main] lets the controller optionally play 1 Tamer from hand free.
#[test]
fn bt25_104_option_main_optionally_plays_tamer_free() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT25-104")
        .expect("BT25-104 in pack")
        .add_card(make_opp_digimon("OPP1"))
        .add_card(make_tamer("TAMER1"))
        .add_card(make_filler("FILL"))
        .hand(0, &["BT25-104", "TAMER1"])
        .hand(1, &["FILL"])
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let _opp = runner.place_on_field(1, "OPP1", Some(0));
    let field_before = runner.battle_area_size(0);

    runner.game.activate_hand_main(0, 0);
    drain(&mut runner);

    // TAMER1 was played free onto the field (taking valid_action_ids[0] picks it).
    assert!(
        runner.battle_area_size(0) > field_before,
        "a Tamer must be played onto the field by the optional free-play"
    );
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "TAMER1"),
        "TAMER1 must be the played Tamer"
    );
}

/// The free-Tamer play is OPTIONAL: with no Tamer in hand, the [Main] still runs
/// the debuff and does not panic.
#[test]
fn bt25_104_option_main_tamer_play_is_optional() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT25-104")
        .expect("BT25-104 in pack")
        .add_card(make_opp_digimon("OPP1"))
        .add_card(make_filler("FILL"))
        .hand(0, &["BT25-104"]) // no Tamer in hand
        .hand(1, &["FILL"])
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let opp = runner.place_on_field(1, "OPP1", Some(0));
    let dp_before = runner.effective_dp(opp);

    runner.game.activate_hand_main(0, 0);
    drain(&mut runner);

    // The debuff still applied even though no Tamer was available.
    assert_eq!(
        runner.effective_dp(opp),
        dp_before.map(|d| d - 15000),
        "the -15000 DP debuff applies even with no Tamer to play"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Section 3 — GAP documentation
// ════════════════════════════════════════════════════════════════════════════

/// A [Marcus Damon] Tamer (name-matched by the aura).
fn make_marcus(id: &str) -> CardData {
    let mut c = make_test_card(id, "Marcus Damon");
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![CardColor::Yellow];
    c
}

/// [Your Turn] All of your [Marcus Damon]s are treated as 12000 DP Digimon and
/// gain <Rush> — the continuous mass TreatAsDigimon+synth aura
/// (G-DSL-AURA-TREAT-AS-DIGIMON-SYNTH, now closed via the floating-mass payload).
#[test]
fn bt25_104_your_turn_marcus_treated_as_12000_digimon_with_rush() {
    use digimon_engine::enums::ModifierType;
    let mut runner = DebugRunner::builder()
        .dsl_card("BT25-104")
        .expect("BT25-104 in pack")
        .add_card(make_marcus("MARCUS"))
        .add_card(make_tamer("OTHER")) // a non-Marcus Tamer (negative control)
        .add_card(make_filler("FILL"))
        .hand(0, &["FILL"])
        .hand(1, &["FILL"])
        .deck(0, &["FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();
    runner.set_first_player(0);

    let shine = runner.place_on_field(0, "BT25-104", Some(0));
    let marcus = runner.place_on_field(0, "MARCUS", Some(0));
    let other = runner.place_on_field(0, "OTHER", Some(0));

    // The [Your Turn] aura installs on the digivolve that brings the Burst Mode
    // face in (and re-installs at each start_of_your_turn).
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(shine));
    runner.game.drain_effect_queue();
    runner.game.tick_declarative_effects();

    // The Marcus Damon is treated as a 12000 DP Digimon and gains <Rush>.
    assert_eq!(
        runner.effective_dp(marcus),
        Some(12000),
        "the Marcus Damon is treated as a 12000 DP Digimon"
    );
    assert!(
        runner.modifiers().has(marcus, ModifierType::TreatAsDigimon),
        "the Marcus Damon carries the TreatAsDigimon (synth identity) modifier"
    );
    assert!(
        runner.game.has_keyword(marcus, Keyword::Rush),
        "the Marcus Damon gains <Rush>"
    );

    // A non-Marcus Tamer is unaffected (the aura is name-filtered).
    assert!(
        !runner.modifiers().has(other, ModifierType::TreatAsDigimon),
        "a non-Marcus Tamer is NOT treated as a Digimon"
    );
    assert!(
        !runner.game.has_keyword(other, Keyword::Rush),
        "a non-Marcus Tamer does NOT gain <Rush>"
    );
}
