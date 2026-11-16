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

/// The Digimon face is Red+Yellow, and so is the Option face.
///
/// This test previously asserted a Yellow-only Option face, which was wrong:
/// the official Bandai DB's "DUAL Color" field for BT25-104 reads "Red Yellow"
/// (mirrored as `dual_colors` in data/card_official.json), and the printed
/// Option band lights both the Red and Yellow slots of its colour stripe.
/// Option colour decides option-use colour legality
/// (action/mask.rs `option_color_match_available`), so the difference is live:
/// a Red-only board can use this Option face.
#[test]
fn bt25_104_face_colors() {
    let card = compiled("BT25-104");
    let dual = card.dual.as_ref().expect("dual metadata");
    use digimon_dsl::compiled::CompiledColor;
    assert!(dual.digimon.colors.contains(&CompiledColor::Red));
    assert!(dual.digimon.colors.contains(&CompiledColor::Yellow));
    assert_eq!(
        dual.option.colors,
        vec![CompiledColor::Red, CompiledColor::Yellow]
    );
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
                keyword,
                ..
            }) => Some(keyword.as_str()),
            _ => None,
        })
        .collect();
    for kw in [
        "Raid",
        "Piercing",
        "SecurityAttackPlus",
        "Blocker",
        "Barrier",
    ] {
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
        card.alt_paths
            .iter()
            .any(|p| p.kind == CompiledAltPathKind::Digivolve
                && p.cost == Some(CompiledCost::Literal(5))),
        "must have a cost-5 DATA SQUAD digivolve alt-path"
    );
    assert!(
        card.alt_paths
            .iter()
            .any(|p| p.kind == CompiledAltPathKind::BurstDigivolve
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
    assert!(
        clause.once_per_turn,
        "the [WD][WA] inline clause is [Once Per Turn]"
    );
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
        let p = runner
            .game
            .pending_selection
            .as_ref()
            .unwrap()
            .selecting_player;
        let a = runner
            .game
            .pending_selection
            .as_ref()
            .unwrap()
            .valid_action_ids[0];
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
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(shine),
    );
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

// ════════════════════════════════════════════════════════════════════════════
// Section — the Digimon-face WD/WA clause activates the OPTION side
// ════════════════════════════════════════════════════════════════════════════

/// Official Q&A Q6496/Q6498: "Activate 1 [Main] effect on this card's Option
/// side" runs that effect as an OPTION card effect (DCGO
/// `ActivateMainOfOptionSide` -> `SetIsDigimonEffect(false)`). So the immunity a
/// target has decides the outcome by KIND:
///   - "isn't affected by Digimon effects" (EX10-010)      -> does NOT stop it
///   - "isn't affected by opponent's effects" (Magnamon X) -> stops it
///   - "isn't affected by Option effects" (Yuuga, the BlackWarGreymon Tamer)
///                                                          -> stops it
/// A recorded DCGO game deleted EX10-010 this way; the engine had classified the
/// inline clause as a Digimon effect and the Digimon-effect immunity blocked it.
fn option_side_debuff_dp_change(source_kind: Option<digimon_engine::enums::EffectSourceKind>) -> i32 {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::{EffectControllerFilter, EffectImmunityFilter, ModifierEntry};

    let mut runner = DebugRunner::builder()
        .dsl_card("BT25-104")
        .expect("BT25-104 in pack")
        .add_card(make_opp_digimon("OPP1"))
        .add_card(make_filler("FILL"))
        .hand(0, &["FILL"])
        .hand(1, &["FILL"])
        .deck(0, &["FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();
    runner.set_first_player(0);

    let shine = runner.place_on_field(0, "BT25-104", Some(0));
    let opp = runner.place_on_field(1, "OPP1", Some(0));
    let mut immunity =
        ModifierEntry::simple(ModifierType::CannotBeAffected, 0, Expiry::Permanent, 0);
    immunity.effect_immunity_filter = Some(EffectImmunityFilter {
        source_kind,
        controller: EffectControllerFilter::OpponentOnly,
    });
    runner.game.modifiers.add(opp, immunity);
    let before = runner.effective_dp(opp).expect("opp has DP");

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(shine),
    );
    runner.game.drain_effect_queue();
    drain(&mut runner);

    runner.effective_dp(opp).expect("opp still has DP") - before
}

#[test]
fn bt25_104_wd_option_effect_is_not_stopped_by_digimon_effect_immunity() {
    use digimon_engine::enums::EffectSourceKind;
    assert_eq!(
        option_side_debuff_dp_change(Some(EffectSourceKind::Digimon)),
        -15000,
        "an Option-side effect is not a Digimon effect, so Digimon-effect immunity (EX10-010) does not stop it"
    );
}

#[test]
fn bt25_104_wd_option_effect_is_stopped_by_all_opponent_effects_immunity() {
    assert_eq!(
        option_side_debuff_dp_change(None),
        0,
        "'isn't affected by opponent's effects' (Magnamon X) still stops an Option effect"
    );
}

#[test]
fn bt25_104_wd_option_effect_is_stopped_by_option_effect_immunity() {
    use digimon_engine::enums::EffectSourceKind;
    assert_eq!(
        option_side_debuff_dp_change(Some(EffectSourceKind::Option)),
        0,
        "'isn't affected by Option effects' (Yuuga) stops an Option-side effect"
    );
}

/// End-to-end replica of a recorded DCGO game: the opponent's EX10-010
/// BlackWarGreymon ACE ("isn't affected by opponent's Digimon's effects" while
/// the opponent has a 13000+ DP Digimon) takes the -15000 from BT25-104's
/// Option-side clause (an Option effect), drops to 0 DP and is deleted at the
/// rule check, and its `Ace Overflow <-4>` costs the opponent 4 memory - which
/// the recording read as 4 -> 8 from P0's side.
#[test]
fn bt25_104_option_side_effect_deletes_real_ex10_010_and_its_overflow_moves_memory() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT25-104")
        .expect("BT25-104 in pack")
        .dsl_card("EX10-010")
        .expect("EX10-010 in pack")
        .add_card(make_filler("FILL"))
        .hand(0, &["FILL"])
        .hand(1, &["FILL"])
        .deck(0, &["FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(4)
        .start();
    runner.set_first_player(0);

    let shine = runner.place_on_field(0, "BT25-104", Some(0));
    let opp = runner.place_on_field(1, "EX10-010", Some(0));
    runner.game.tick_declarative_effects();
    assert_eq!(
        runner.effective_dp(opp),
        Some(15000),
        "EX10-010 is 12000 +3000 while the opponent has a 13000+ DP Digimon (BT25-104 is 15000)"
    );

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(shine),
    );
    runner.game.drain_effect_queue();
    drain(&mut runner);
    runner.game.tick_declarative_effects();

    assert!(
        runner.game.players[1].battle_area.is_empty(),
        "EX10-010 took -15000 from the Option-side effect (0 DP) and was deleted"
    );
    assert_eq!(
        runner.memory(),
        8,
        "EX10-010's Ace Overflow <-4> cost the opponent 4 memory (4 -> 8 from P0's side)"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// [Burst Digivolve] (CR 6-5-1-2-3, 8-3) through the real action surface.
// Shared scenario: `crate::burst_digivolve_common`.
// ─────────────────────────────────────────────────────────────────────────────

mod burst_digivolve {
    use crate::burst_digivolve_common::*;
    use digimon_engine::enums::CardColor;

    const SPEC: BurstSpec = BurstSpec {
        card: "BT25-104",
        host_name: "ShineGreymon",
        host_color: CardColor::Red,
        tamer_name: "Marcus Damon",
    };

    /// 8-3-3: offered via the mask with [Marcus Damon] on the field; returns the
    /// Tamer, pays 0, draws 1, stacks on the [ShineGreymon].
    #[test]
    fn bt25_104_burst_offered_returns_tamer_costs_zero_draws_one() {
        burst_and_check(SPEC, Host::BurstOnly);
    }

    /// The burst cost can't be paid without a [Marcus Damon].
    #[test]
    fn bt25_104_burst_not_offered_without_tamer() {
        assert_not_offered_without_tamer(SPEC);
    }

    /// 8-3-2-1: the top card is trashed at the end of the burst turn,
    /// tracked by permanent identity (the host shifted slots).
    #[test]
    fn bt25_104_burst_top_card_trashed_at_turn_end() {
        assert_turn_end_trash(SPEC, Host::BurstOnly);
    }

    /// Normal (cost 5) and burst routes both legal: a real player choice.
    #[test]
    fn bt25_104_burst_vs_normal_route_is_a_player_choice() {
        assert_route_choice(SPEC);
    }

    /// Choosing the burst route from the route prompt, then the turn-end trash.
    #[test]
    fn bt25_104_burst_chosen_from_route_prompt_then_trashed() {
        assert_turn_end_trash(SPEC, Host::AlsoNormal);
    }

    /// 8-3-2-2: an effect that digivolves can't burst digivolve.
    #[test]
    fn bt25_104_effect_digivolve_cannot_burst() {
        assert_effect_cannot_burst(SPEC);
    }

    /// Two qualifying Tamers: the player picks which returns.
    #[test]
    fn bt25_104_burst_two_tamers_is_a_player_choice() {
        assert_two_tamers_choice(SPEC);
    }
}

// ════════════════════════════════════════════════════════════════════════════
// Section 5 — Free Tamer pick legality (select-hand audit 2026-11-13)
// ════════════════════════════════════════════════════════════════════════════
//
// DCGO BT25_104.cs:281-285 `CanSelectTamerCondition`: `IsTamer &&
// CanPlayAsNewPermanent(card, payCost: false, activateClass)`; the inline
// [WD][WA] clause runs the same Option [Main] (`ActivateMainOfOptionSide`,
// BT25_104.cs:126). `CanPlayAsNewPermanent` (GameContextDeterminarion.cs:
// 419-432) needs an empty battle-area frame and no play prohibition
// (`CanEnterField`). A Tamer that can't be played stays in hand (CR 4.3
// 7-1-2-3, 1-3-2), so it is not a choice. YAML: both Tamer picks add
// `can_play_from_effect_without_cost: true`.

/// The non-PASS hand candidates of the pending pick, as sorted card ids; empty
/// when no hand pick is pending.
fn offered_hand_ids(runner: &DebugRunner, p: u8) -> Vec<String> {
    use digimon_engine::action::space::{PASS, PLAY_HAND_START};
    use digimon_engine::owned_card_choice::{CardChoiceOperation, CardChoiceZone};
    use digimon_engine::selection::SelectionKind;
    let Some(view) = runner.pending_selection_view() else {
        return Vec::new();
    };
    if !matches!(view.kind, SelectionKind::Hand) {
        return Vec::new();
    }
    let hand = &runner.game.players[p as usize].hand;
    let id_at = |i: usize| hand.get(i).map(|c| c.card_id(&runner.game.card_data).to_string());
    let mut ids: Vec<String> = match view.card_choice {
        Some(choice) => choice
            .rows
            .iter()
            .filter(|row| {
                matches!(row.operation, CardChoiceOperation::CommitCard)
                    && matches!(row.zone, Some(CardChoiceZone::Hand))
            })
            .filter_map(|row| row.position.as_deref()?.parse::<usize>().ok())
            .filter_map(id_at)
            .collect(),
        None => view
            .valid_action_ids
            .iter()
            .copied()
            .filter(|&a| a != PASS)
            .filter_map(|a| a.checked_sub(PLAY_HAND_START))
            .filter_map(|i| id_at(i as usize))
            .collect(),
    };
    ids.sort();
    ids
}

/// Answer the mandatory "-15000 DP" opponent pick (any non-hand prompt of
/// player 0) with its first legal action, so the Tamer pick comes up next.
fn answer_dp_pick(runner: &mut DebugRunner) {
    use digimon_engine::selection::SelectionKind;
    if let Some(view) = runner.pending_selection_view() {
        if !matches!(view.kind, SelectionKind::Hand) {
            let action = view.valid_action_ids[0];
            runner
                .execute_action(view.selecting_player, action)
                .expect("answer the -15000 DP pick");
        }
    }
}

fn tamer_pick_runner(hand: &[&str]) -> DebugRunner {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT25-104")
        .expect("BT25-104 in pack")
        .add_card(make_opp_digimon("OPP1"))
        .add_card(make_tamer("TAMER1"))
        .add_card(make_filler("FILL"))
        .hand(0, hand)
        .hand(1, &["FILL"])
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();
    runner.set_first_player(0);
    runner
}

fn prohibit_tamer_effect_play(runner: &mut DebugRunner) {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::PlayerModifierEntry;
    runner.game.modifiers.add_player_modifier(
        0,
        PlayerModifierEntry::simple(ModifierType::CannotPlayTamerByEffect, 0, Expiry::Permanent, None, 1),
    );
}

/// Seat BT25-104 and an opponent Digimon, run `setup`, fire the inline
/// [When Digivolving] clause, answer the DP pick, return the Tamer candidates.
fn wd_tamer_offered(setup: impl FnOnce(&mut DebugRunner)) -> Vec<String> {
    let mut runner = tamer_pick_runner(&["TAMER1"]);
    let shine = runner.place_on_field(0, "BT25-104", Some(0));
    runner.place_on_field(1, "OPP1", Some(0));
    setup(&mut runner);
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(shine));
    runner.game.drain_effect_queue();
    answer_dp_pick(&mut runner);
    offered_hand_ids(&runner, 0)
}

/// Use the Option face from hand (TAMER1 also in hand), run `setup` first,
/// answer the DP pick, return the Tamer candidates.
fn option_main_tamer_offered(setup: impl FnOnce(&mut DebugRunner)) -> Vec<String> {
    let mut runner = tamer_pick_runner(&["BT25-104", "TAMER1"]);
    runner.place_on_field(1, "OPP1", Some(0));
    setup(&mut runner);
    runner.game.activate_hand_main(0, 0);
    runner.game.drain_effect_queue();
    answer_dp_pick(&mut runner);
    offered_hand_ids(&runner, 0)
}

/// Regression guard: with nothing in the way the Tamer is offered.
#[test]
fn bt25_104_wd_tamer_pick_offers_playable_tamer() {
    assert_eq!(wd_tamer_offered(|_| {}), vec!["TAMER1".to_string()]);
}

/// "You can't play Tamers by effects": the Tamer is not offered.
#[test]
fn bt25_104_wd_tamer_pick_excludes_tamer_under_effect_play_prohibition() {
    let offered = wd_tamer_offered(prohibit_tamer_effect_play);
    assert!(offered.is_empty(), "{offered:?}");
}

/// A full battle area: the Tamer is not offered.
#[test]
fn bt25_104_wd_tamer_pick_excludes_tamer_when_battle_area_full() {
    let offered = wd_tamer_offered(|r| {
        r.game.rules.field_slots = r.game.players[0].battle_area.len() as u8;
    });
    assert!(offered.is_empty(), "{offered:?}");
}

/// Regression guard: the Option-face [Main] offers the Tamer.
#[test]
fn bt25_104_option_main_tamer_pick_offers_playable_tamer() {
    assert_eq!(option_main_tamer_offered(|_| {}), vec!["TAMER1".to_string()]);
}

/// The Option-face [Main] under "can't play Tamers by effects": not offered.
#[test]
fn bt25_104_option_main_tamer_pick_excludes_tamer_under_effect_play_prohibition() {
    let offered = option_main_tamer_offered(prohibit_tamer_effect_play);
    assert!(offered.is_empty(), "{offered:?}");
}

/// The Option-face [Main] with a full battle area: not offered.
#[test]
fn bt25_104_option_main_tamer_pick_excludes_tamer_when_battle_area_full() {
    let offered = option_main_tamer_offered(|r| {
        r.place_on_field(0, "FILL", None);
        r.game.rules.field_slots = r.game.players[0].battle_area.len() as u8;
    });
    assert!(offered.is_empty(), "{offered:?}");
}
