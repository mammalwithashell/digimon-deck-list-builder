//! BT12-043 ShineGreymon — Digimon, Lv.6, Yellow/Red, DP 12000, Cost 12.
//! Traits: Light Dragon. Form: Mega. Attribute: Vaccine.
//!
//! # Card text (card image BT12-043.webp + official bundle — verbatim)
//!
//! [When Digivolving] For each yellow and/or red Tamer you have in play,
//! 1 of your opponent's Digimon and all of your opponent's Security Digimon
//! get -3000 DP for the turn.
//! [Your Turn] All of your [Marcus Damon]s in play get +3000 DP and gain
//! ＜Security Attack +1＞. (This Digimon checks 1 additional security card.)
//!
//! Official Q&A: "No, you can't [pick 0 Digimon]. You choose 1 of your
//! opponent's Digimon, then that Digimon and all security Digimon get -3000
//! DP for each of your yellow or red Tamers." — the opponent-Digimon pick is
//! MANDATORY when a legal target exists, and one shared multiplier (3000 *
//! yellow/red Tamer count) hits both the picked Digimon and all opponent
//! Security Digimon.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT12/Yellow/BT12_043.cs
//!   - Region 1 (None): AddSelfDigivolutionRequirementStaticEffect —
//!     TopCard.ContainsCardName("RizeGreymon") && Level == 5, cost 3.
//!   - Region 2 (OnEnterFieldAnyone): CanActivateCondition requires >=1 own
//!     battle-area Tamer colored Yellow or Red; ActivateCoroutine recomputes
//!     the same count, mandatorily selects 1 opponent Digimon (canNoSelect:
//!     false) when any exists -> ChangeDigimonDP(-3000*count,
//!     UntilEachTurnEnd); UNCONDITIONALLY (outside the opponent-Digimon
//!     existence guard) applies ChangeSecurityDigimonCardDPPlayerEffect
//!     (-3000*count, UntilEachTurnEnd) to all opponent Security Digimon.
//!   - Region 3 + 4 (None): ChangeDPStaticEffect(+3000) /
//!     ChangeSAttackStaticEffect(+1), both gated on
//!     IsPermanentExistsOnOwnerBattleAreaDigimon (requires Permanent.IsDigimon
//!     — true for a genuine Digimon OR a permanent under an active
//!     ITreatAsDigimonEffect) AND CardNames.Contains("Marcus Damon"), AND
//!     IsOwnerTurn. No printed "Marcus Damon" card is Digimon-kind (all 5
//!     printings — BT4-092, BT12-092, BT13-095, BT17-087, BT21-086 — are
//!     Tamers), so this is a dormant synergy gate: it only grants DP/Security
//!     A. to a Marcus Damon Tamer that ANOTHER effect (AD1-021, BT13-020,
//!     BT25-104) has simultaneously made "treated as a Digimon".
//!
//! # Substrate (all shipped — no engine/DSL gap)
//! - alt_paths: kind: digivolve, from: { level_eq, color_is } for the
//!   cards.json-dropped Red Lv.5/4 standard circle (BT12-038/BT11-018 idiom).
//! - alt_paths: kind: digivolve, from: { level_eq, name_contains } for the
//!   "w/[RizeGreymon] in name" special alt-path (AD1-016/BT21-042/ST24-07
//!   idiom — DCGO ContainsCardName is a substring scan).
//! - condition: count_gte over a colored-Tamer filter (BT13-088 idiom).
//! - select_opponent_permanent (mandatory) + add_dp_modifier with a
//!   base_per_delta card_count_in_zone formula (BT23-101 idiom).
//! - add_player_modifier { modifier: SecurityDpChange, value: <formula> } for
//!   the opponent (ST3-13/ST1-14/BT15-092 idiom — defender_security_dp_adjustment
//!   consumer), authored as a SEPARATE sibling step so it always runs.
//! - kind: aura with a name+kind FILTER target + active_when: your_turn,
//!   consuming synth_identity via the TreatAsDigimon mass-aura path
//!   (BT25-104 idiom: `kind: digimon, name_is: "Marcus Damon"` gate).

#![allow(dead_code, unused_imports)]

#[path = "../../support/dsl_card_data.rs"]
mod dsl_card_data;

use digimon_dsl::compiled::{CompiledAltPathKind, CompiledClause, CompiledColor, CompiledScope};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Keyword, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{PendingSelection, TriggerSource};

fn compiled() -> digimon_dsl::compiled::CompiledCard {
    dsl_card_data::compiled("BT12-043")
}

fn first_action(p: &PendingSelection) -> u16 {
    *p.valid_action_ids.first().expect("a legal action")
}

fn fire(runner: &mut DebugRunner, timing: EffectTiming, h: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(timing, TriggerSource::Permanent(h));
    runner.game.drain_effect_queue();
}

// ─── Card-data helpers ────────────────────────────────────────────────────────

fn tamer(id: &str, color: CardColor) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.colors = vec![color];
    c.play_cost = 2;
    c
}

fn opp_digimon(id: &str, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(dp);
    c.colors = vec![CardColor::Black];
    c.play_cost = 5;
    c
}

fn filler() -> CardData {
    make_test_card("FILLER", "Filler")
}

/// A [Marcus Damon] Tamer (name-matched by the [Your Turn] auras). Not
/// natively Digimon-kind — matches every printed Marcus Damon card.
fn make_marcus(id: &str) -> CardData {
    let mut c = make_test_card(id, "Marcus Damon");
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.colors = vec![CardColor::Yellow];
    c.play_cost = 3;
    c
}

fn base_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT12-043")
        .expect("BT12-043 in pack")
        .add_card(tamer("Y-TAMER", CardColor::Yellow))
        .add_card(tamer("R-TAMER", CardColor::Red))
        .add_card(tamer("B-TAMER", CardColor::Blue))
        .add_card(opp_digimon("OPP-A", 11000))
        .add_card(opp_digimon("OPP-B", 9000))
        .add_card(make_marcus("MARCUS"))
        .add_card(tamer("OTHER-TAMER", CardColor::Yellow))
        .add_card(filler())
        .deck(0, &["FILLER"; 10])
        .deck(1, &["FILLER"; 10])
        .memory(10)
        .start()
}

// ════════════════════════════════════════════════════════════════════════════
// SECTION 1 — Structural
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn bt12_043_compiles_from_dsl_pack() {
    let _ = compiled();
}

#[test]
fn bt12_043_metadata_yellow_red_lv6_12000() {
    let card = compiled();
    assert_eq!(card.card, "BT12-043");
    assert_eq!(card.name, "ShineGreymon");
    assert_eq!(card.level, Some(6));
    assert_eq!(card.dp, Some(12000));
    assert!(card.color.contains(&CompiledColor::Yellow));
    assert!(card.color.contains(&CompiledColor::Red));
    assert_eq!(card.cost, Some(12));
}

/// Two authored standard-circle-restoration + special alt-paths:
/// Red Lv.5/4 (cards.json-dropped standard circle) + Lv.5 w/RizeGreymon-in-
/// name/cost 3 (special digivolve). The Yellow Lv.5/4 circle loads from
/// evo_costs metadata, not an authored alt_path.
#[test]
fn bt12_043_has_two_authored_alt_paths() {
    let card = compiled();
    assert_eq!(
        card.alt_paths.len(),
        2,
        "Red standard-circle restoration + RizeGreymon special alt-path"
    );
    let has_red_lv5_cost4 = card.alt_paths.iter().any(|p| {
        matches!(p.kind, CompiledAltPathKind::Digivolve)
            && p.cost == Some(digimon_dsl::compiled::CompiledCost::Literal(4))
            && format!("{:?}", p.from).to_lowercase().contains("red")
            && format!("{:?}", p.from).contains("5")
    });
    assert!(
        has_red_lv5_cost4,
        "must restore the cards.json-dropped Red Lv.5/cost 4 standard circle: {:?}",
        card.alt_paths
    );
    let has_rizegreymon_cost3 = card.alt_paths.iter().any(|p| {
        matches!(p.kind, CompiledAltPathKind::Digivolve)
            && p.cost == Some(digimon_dsl::compiled::CompiledCost::Literal(3))
            && format!("{:?}", p.from).contains("RizeGreymon")
    });
    assert!(
        has_rizegreymon_cost3,
        "must have the '[Digivolve] 3 from Lv.5 w/[RizeGreymon] in name' alt-path: {:?}",
        card.alt_paths
    );
}

// ════════════════════════════════════════════════════════════════════════════
// SECTION 2 — [When Digivolving] gate + mandatory opp-Digimon debuff + all-security debuff
// ════════════════════════════════════════════════════════════════════════════

/// Zero own yellow/red Tamers -> the whole clause is gated off (no select, no
/// security modifier). Mirrors DCGO's CanActivateCondition == false.
#[test]
fn bt12_043_when_digivolving_no_effect_with_zero_colored_tamers() {
    let mut runner = base_runner();
    let shine = runner.place_on_field(0, "BT12-043", Some(0));
    // Only a BLUE tamer present — does not satisfy yellow/red.
    let _blue = runner.place_on_field(0, "B-TAMER", Some(0));
    let opp = runner.place_on_field(1, "OPP-A", Some(0));
    let dp_before = runner.effective_dp(opp).expect("opp dp");

    fire(&mut runner, EffectTiming::WhenDigivolving, shine);
    let _ = runner.auto_resolve();

    assert!(
        runner.game.pending_selection.is_none(),
        "no pending selection when there are zero yellow/red Tamers"
    );
    assert_eq!(
        runner.effective_dp(opp),
        Some(dp_before),
        "opponent Digimon DP is unaffected with zero qualifying Tamers"
    );
    assert_eq!(
        runner
            .modifiers()
            .player_modifier_value(1, ModifierType::SecurityDpChange),
        0,
        "no opponent SecurityDpChange modifier installed with zero qualifying Tamers"
    );
}

/// One yellow Tamer -> -3000 DP to the mandatorily-picked opponent Digimon,
/// and a matching -3000 SecurityDpChange modifier on the opponent.
#[test]
fn bt12_043_when_digivolving_scales_with_one_yellow_tamer() {
    let mut runner = base_runner();
    let shine = runner.place_on_field(0, "BT12-043", Some(0));
    let _y = runner.place_on_field(0, "Y-TAMER", Some(0));
    let opp_a = runner.place_on_field(1, "OPP-A", Some(0)); // 11000
    let dp_before = runner.effective_dp(opp_a).expect("opp dp");

    fire(&mut runner, EffectTiming::WhenDigivolving, shine);

    // Mandatory pick of 1 opponent Digimon (no PASS/decline option).
    let (player, action) = {
        let p = runner
            .game
            .pending_selection
            .as_ref()
            .expect("mandatory opponent-Digimon select pending");
        assert!(
            !p.is_optional,
            "the opponent-Digimon pick is mandatory (Q&A: 'No, you can't [pick 0]')"
        );
        (p.selecting_player, first_action(p))
    };
    runner
        .execute_action(player, action)
        .expect("pick the opponent Digimon");
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.effective_dp(opp_a),
        Some((dp_before - 3000).max(0)),
        "1 yellow Tamer -> -3000 DP to the picked opponent Digimon"
    );
    assert_eq!(
        runner
            .modifiers()
            .player_modifier_value(1, ModifierType::SecurityDpChange),
        -3000,
        "1 yellow Tamer -> -3000 SecurityDpChange on the opponent"
    );
}

/// Two qualifying Tamers (1 yellow + 1 red) -> -6000 scaling on both the
/// picked Digimon and the security modifier (count, not distinct color).
#[test]
fn bt12_043_when_digivolving_scales_with_yellow_and_red_tamers() {
    let mut runner = base_runner();
    let shine = runner.place_on_field(0, "BT12-043", Some(0));
    let _y = runner.place_on_field(0, "Y-TAMER", Some(0));
    let _r = runner.place_on_field(0, "R-TAMER", Some(0));
    let opp_a = runner.place_on_field(1, "OPP-A", Some(0)); // 11000
    let dp_before = runner.effective_dp(opp_a).expect("opp dp");

    fire(&mut runner, EffectTiming::WhenDigivolving, shine);
    let (player, action) = {
        let p = runner
            .game
            .pending_selection
            .as_ref()
            .expect("mandatory opponent-Digimon select pending");
        (p.selecting_player, first_action(p))
    };
    runner
        .execute_action(player, action)
        .expect("pick the opponent Digimon");
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.effective_dp(opp_a),
        Some((dp_before - 6000).max(0)),
        "2 qualifying Tamers (yellow + red) -> -6000 DP to the picked opponent Digimon"
    );
    assert_eq!(
        runner
            .modifiers()
            .player_modifier_value(1, ModifierType::SecurityDpChange),
        -6000,
        "2 qualifying Tamers -> -6000 SecurityDpChange on the opponent"
    );
}

/// Only the SELECTED opponent Digimon takes the DP hit — a bystander opponent
/// Digimon is unaffected (the debuff is "1 of your opponent's Digimon", not
/// all of them).
#[test]
fn bt12_043_when_digivolving_only_selected_opp_digimon_is_debuffed() {
    let mut runner = base_runner();
    let shine = runner.place_on_field(0, "BT12-043", Some(0));
    let _y = runner.place_on_field(0, "Y-TAMER", Some(0));
    let opp_a = runner.place_on_field(1, "OPP-A", Some(0));
    let opp_b = runner.place_on_field(1, "OPP-B", Some(0));
    let dp_b_before = runner.effective_dp(opp_b).expect("opp_b dp");

    fire(&mut runner, EffectTiming::WhenDigivolving, shine);
    let (player, action) = {
        let p = runner
            .game
            .pending_selection
            .as_ref()
            .expect("mandatory opponent-Digimon select pending");
        // Deterministically target opp_a via its encoded action if present,
        // otherwise fall back to the first legal action. Field-selection
        // prompts encode each valid slot as `encode_attack(0, slot_index)`
        // (install_field_selection — reuses the ATTACK action-space half).
        let want = digimon_engine::action::space::encode_attack(0, opp_a.index as u16);
        let chosen = if p.valid_action_ids.contains(&want) {
            want
        } else {
            first_action(p)
        };
        (p.selecting_player, chosen)
    };
    runner
        .execute_action(player, action)
        .expect("pick an opponent Digimon");
    let _ = runner.auto_resolve();

    // opp_b's DP is untouched by the single-target pick regardless of which
    // permanent was chosen (only one of opp_a/opp_b loses 3000, never both).
    let dp_a_after = runner.effective_dp(opp_a).expect("opp_a still present");
    let dp_b_after = runner.effective_dp(opp_b).expect("opp_b still present");
    let a_hit = dp_a_after == 11000 - 3000;
    let b_hit = dp_b_after == dp_b_before - 3000;
    assert!(
        a_hit ^ b_hit,
        "exactly one of the two opponent Digimon (the picked one) loses 3000 DP; got a_after={dp_a_after} b_after={dp_b_after}"
    );
}

/// The security debuff fires UNCONDITIONALLY once the Tamer-count gate
/// passes — even when the opponent has ZERO battle-area Digimon (so the
/// mandatory-pick step is naturally skipped), matching DCGO's separate
/// (outside the HasMatchConditionPermanent guard) security-DP application.
#[test]
fn bt12_043_when_digivolving_security_debuff_fires_with_no_opp_digimon() {
    let mut runner = base_runner();
    let shine = runner.place_on_field(0, "BT12-043", Some(0));
    let _y = runner.place_on_field(0, "Y-TAMER", Some(0));
    // No opponent battle-area Digimon placed at all.

    fire(&mut runner, EffectTiming::WhenDigivolving, shine);
    let _ = runner.auto_resolve();

    assert!(
        runner.game.pending_selection.is_none(),
        "no opponent Digimon on field -> the mandatory pick step is a no-op, no pending selection left behind"
    );
    assert_eq!(
        runner
            .modifiers()
            .player_modifier_value(1, ModifierType::SecurityDpChange),
        -3000,
        "the security debuff still applies (-3000 for 1 Tamer) even with zero opponent battle-area Digimon"
    );
    assert_eq!(
        runner.game.defender_security_dp_adjustment(1),
        -3000,
        "P1's defender security DP adjustment reflects the -3000 debuff"
    );
}

/// Verifies the security-DP modifier is genuinely consumed by
/// `defender_security_dp_adjustment` — the actual mechanism the combat layer
/// reads when P1's security is checked.
#[test]
fn bt12_043_when_digivolving_security_modifier_feeds_defender_adjustment() {
    let mut runner = base_runner();
    let shine = runner.place_on_field(0, "BT12-043", Some(0));
    let _y = runner.place_on_field(0, "Y-TAMER", Some(0));
    let _r = runner.place_on_field(0, "R-TAMER", Some(0));
    let opp_a = runner.place_on_field(1, "OPP-A", Some(0));

    fire(&mut runner, EffectTiming::WhenDigivolving, shine);
    let (player, action) = {
        let p = runner
            .game
            .pending_selection
            .as_ref()
            .expect("mandatory opponent-Digimon select pending");
        (p.selecting_player, first_action(p))
    };
    runner
        .execute_action(player, action)
        .expect("pick the opponent Digimon");
    let _ = runner.auto_resolve();
    let _ = opp_a;

    assert_eq!(
        runner.game.defender_security_dp_adjustment(1),
        -6000,
        "2 qualifying Tamers -> P1's defender security DP adjustment is -6000"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// SECTION 3 — [Your Turn] Marcus Damon (treated-as-Digimon) +3000 DP / Security A. +1
// ════════════════════════════════════════════════════════════════════════════

/// A plain Marcus Damon Tamer (NOT treated as a Digimon by any other effect)
/// is unaffected — the aura's `kind: digimon` filter mirrors DCGO's
/// `Permanent.IsDigimon` gate, which is false for an untransformed Tamer.
#[test]
fn bt12_043_your_turn_plain_marcus_tamer_unaffected() {
    let mut runner = base_runner();
    let _shine = runner.place_on_field(0, "BT12-043", Some(0));
    let marcus = runner.place_on_field(0, "MARCUS", Some(0));
    runner.game.tick_declarative_effects();

    assert!(
        !runner
            .modifiers()
            .has(marcus, ModifierType::SecurityAttackChange),
        "a plain (non-Digimon) Marcus Damon Tamer does not gain Security A. +1"
    );
    // A Tamer's effective_dp is meaningless (Tamers have no DP), but the
    // ChangeDp modifier itself must not be installed on it.
    assert!(
        !runner.modifiers().has(marcus, ModifierType::ChangeDp),
        "a plain (non-Digimon) Marcus Damon Tamer does not gain the +3000 ChangeDp modifier"
    );
}

/// A non-Marcus Tamer (even if independently treated as a Digimon) is
/// unaffected — the aura is name-filtered.
#[test]
fn bt12_043_your_turn_non_marcus_tamer_unaffected() {
    let mut runner = base_runner();
    let _shine = runner.place_on_field(0, "BT12-043", Some(0));
    let other = runner.place_on_field(0, "OTHER-TAMER", Some(0));
    runner.game.tick_declarative_effects();

    assert!(
        !runner
            .modifiers()
            .has(other, ModifierType::SecurityAttackChange),
        "a non-[Marcus Damon] Tamer never gains Security A. +1 from this aura"
    );
    assert!(
        !runner.modifiers().has(other, ModifierType::ChangeDp),
        "a non-[Marcus Damon] Tamer never gains the +3000 DP from this aura"
    );
}

/// The synergy case: a Marcus Damon Tamer that ANOTHER effect has made
/// "treated as a Digimon" (TreatAsDigimon synth-identity modifier) DOES
/// receive both the +3000 DP and Security A. +1 grants from BT12-043's
/// [Your Turn] auras, mirroring DCGO's `Permanent.IsDigimon` (which honors
/// an active `ITreatAsDigimonEffect`).
#[test]
fn bt12_043_your_turn_treated_as_digimon_marcus_gets_dp_and_security_attack() {
    let mut runner = base_runner();
    let shine = runner.place_on_field(0, "BT12-043", Some(0));
    let marcus = runner.place_on_field(0, "MARCUS", Some(0));

    // Simulate another card's TreatAsDigimon grant on the Marcus Damon Tamer
    // directly via the engine API (equivalent to AD1-021/BT13-020/BT25-104's
    // own effect installing the modifier).
    use digimon_engine::modifiers::{ModifierEntry, ModifierPayload};
    runner.game.modifiers.add(
        marcus,
        ModifierEntry {
            modifier: ModifierType::TreatAsDigimon,
            value: 0,
            payload: ModifierPayload::SynthIdentity {
                kind: CardKind::Digimon,
                level: 0,
                colors: vec![],
                traits: vec![],
                dp: 6000,
            },
            expiry: digimon_engine::enums::Expiry::EndOfYourTurn,
            source_permanent: None,
            source_player: 0,
            materialized_declarative: false,
            cause_filter: None,
            replacement_condition: None,
            until_condition: None,
            effect_immunity_filter: None,
            disable_effect_timing: None,
            pending_skips: 0,
            install_order: 0,
        },
    );

    runner.game.tick_declarative_effects();

    assert_eq!(
        runner.effective_dp(marcus),
        Some(6000 + 3000),
        "the treated-as-Digimon Marcus Damon gets the base synth 6000 DP + 3000 from BT12-043's [Your Turn] aura"
    );
    assert!(
        runner
            .modifiers()
            .has(marcus, ModifierType::SecurityAttackChange),
        "the treated-as-Digimon Marcus Damon gains <Security A. +1>"
    );
    let _ = shine;
}

/// The [Your Turn] auras are turn-scoped: they must NOT apply on the
/// opponent's turn (DCGO Condition = IsOwnerTurn). Uses `Expiry::Permanent`
/// on the injected TreatAsDigimon so the synth-identity state itself
/// survives the turn transition, isolating the `your_turn` gate as the only
/// variable under test.
#[test]
fn bt12_043_your_turn_auras_do_not_apply_on_opponents_turn() {
    let mut runner = base_runner();
    runner.skip_mulligan();
    let shine = runner.place_on_field(0, "BT12-043", Some(0));
    let marcus = runner.place_on_field(0, "MARCUS", Some(0));

    use digimon_engine::modifiers::{ModifierEntry, ModifierPayload};
    runner.game.modifiers.add(
        marcus,
        ModifierEntry {
            modifier: ModifierType::TreatAsDigimon,
            value: 0,
            payload: ModifierPayload::SynthIdentity {
                kind: CardKind::Digimon,
                level: 0,
                colors: vec![],
                traits: vec![],
                dp: 6000,
            },
            expiry: digimon_engine::enums::Expiry::Permanent,
            source_permanent: None,
            source_player: 0,
            materialized_declarative: false,
            cause_filter: None,
            replacement_condition: None,
            until_condition: None,
            effect_immunity_filter: None,
            disable_effect_timing: None,
            pending_skips: 0,
            install_order: 0,
        },
    );

    // Sanity: on P0's own turn, both auras are live.
    runner.game.tick_declarative_effects();
    assert!(
        runner
            .modifiers()
            .has(marcus, ModifierType::SecurityAttackChange),
        "sanity: Security A. +1 is live on the controller's own turn"
    );

    // Advance to P1's turn — BT12-043 and Marcus remain P0's permanents, but
    // it is no longer P0's turn.
    runner.end_turn();
    let _ = runner.auto_resolve();
    runner.game.tick_declarative_effects();

    assert!(
        !runner
            .modifiers()
            .has(marcus, ModifierType::SecurityAttackChange),
        "the Security A. +1 aura does not apply when it is not the controller's turn"
    );
    assert!(
        !runner.modifiers().has(marcus, ModifierType::ChangeDp),
        "the +3000 DP aura does not apply when it is not the controller's turn"
    );
    let _ = shine;
}
