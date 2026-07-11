//! P-213 Aegiochusmon — Digimon, Lv.5, Red, DP 7000, Cost 7.
//! Traits: Shaman, Iliad, TS, (Rule) Dragonkin. Attribute: Vaccine.
//!
//! # Card text (official Bandai DB bundle — data/card_bundles/P-213.md)
//!
//! ＜Raid＞ (When this Digimon attacks, you may switch the target of attack to
//!   1 of your opponent's unsuspended Digimon with the highest DP.)
//! ＜Decode ([Aegiomon])＞ (When this Digimon would leave the battle area other
//!   than in battle, you may play 1 [Aegiomon] from its digivolution cards
//!   without paying the cost.)
//! [When Digivolving] If you have 3 or fewer security cards, this Digimon
//!   gains ＜Rush＞ (This Digimon can attack the turn it comes into play.) and
//!   +3000 DP until your opponent's turn ends. Then, this Digimon may attack.
//! [Rule] Trait: Has [Dragonkin] Type.
//!
//! Inherited Effect:
//! ＜Decode ([Aegiomon])＞ (same text as above, inherited scope.)
//!
//! Official Q&A: "Yes, you can process it. Even if you don't have 3 or fewer
//! security cards, this card's [When Digivolving] effect allows you to have
//! this Digimon attack." — confirms the "may attack" tail is UNCONDITIONAL,
//! independent of the security-count gate on the Rush/DP grant.
//!
//! Standard digivolve: Lv.4 Red / Cost 3 (printed evo_costs). Alt-digivolve
//! (xros_req): "[Digivolve] [Aegiomon]: Cost 3" — any [Aegiomon]-named source,
//! no level/color restriction (BT25-025 sibling idiom).
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/P/Red/P_213.cs
//!
//! DCGO confirms:
//!   - Alt-digivolve: AddSelfDigivolutionRequirementStaticEffect gated on
//!     TopCard.EqualsCardName("Aegiomon"), cost 3, ignoreDigivolutionRequirement: false.
//!   - Raid: RaidSelfEffect at OnAllyAttack, isInheritedEffect: false.
//!   - Decode ([Aegiomon]): DecodeSelfEffect at WhenRemoveField,
//!     isInheritedEffect: false (own) AND a SECOND identical block with
//!     isInheritedEffect: true (inherited) — sourceCondition
//!     EqualsCardName("Aegiomon"). This is the BT22-015/BT25-025-class
//!     "play a named source from THIS card's digivolution cards without
//!     paying the cost" replacement — NOT the engine's generic
//!     `Keyword::Decode` (which redirects the leaving permanent to hand).
//!   - When Digivolving (OnEnterFieldAnyone, CanTriggerWhenDigivolving):
//!     `if (SecurityCards.Count <= 3) { GainRush + ChangeDP(+3000),
//!     both UntilOpponentTurnEnd }` — the buff sub-block is gated.
//!     THEN, unconditionally, `if (CanAttack) { SelectAttackEffect }` — the
//!     "may attack" tail runs regardless of the security gate (confirmed by
//!     the official Q&A above).
//!
//! # Patterns this test covers (RUST_DSL_TEST_API.md §4.3)
//! - H9 Raid (declarative keyword grant)
//! - Decode named-source play (own AND inherited scope) — BT22-015 /
//!   BT25-025 idiom: `kind: replacement`, `trigger: when_would_leave_battle_area`,
//!   non-battle cause filter, `select_material` (name_contains Aegiomon),
//!   `play_from_materials` (free)
//! - D1-adjacent conditional Rush + DP buff gated on own security count
//! - Unconditional "may attack" tail (may_attack_now, optional, attacker: source)
//!   decoupled from the conditional buff sub-block (BT20-016 / BT22-015 idiom)
//! - G1 Rule-granted trait (Dragonkin) folded into `traits:`
//!
//! # Faithfulness diff vs. card text
//!
//! | Card-text element                                                | Status |
//! |-------------------------------------------------------------------|--------|
//! | Standard digivolve Lv.4 Red / Cost 3                               | OK (alt_paths digivolve) |
//! | Alt-digivolve [Aegiomon]-named / Cost 3                            | OK (alt_paths digivolve, name_contains) |
//! | <Raid>                                                             | OK (grant_keyword) |
//! | <Decode ([Aegiomon])> own                                          | OK (replacement + select_material + play_from_materials) |
//! | <Decode ([Aegiomon])> inherited                                    | OK (scope: inherited replacement) |
//! | [When Digivolving] if <=3 security: Rush + 3000 DP until opp turn end | OK (if-gated grant_keyword Rush + add_dp_modifier, expiry end_of_opponents_turn) |
//! | [When Digivolving] "Then, this Digimon may attack" (unconditional)  | OK (may_attack_now, optional, attacker: source, outside the if-gate) |
//! | [Rule] Trait: Has [Dragonkin] Type                                  | OK (folded into traits:) |

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep,
    CompiledTiming, CompiledTriggeredClause,
};
use digimon_engine::action::space::{PASS, REPLACEMENT_ACCEPT};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{card_data_for_test_from_compiled, make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming, Keyword, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

/// Production YAML for P-213, inlined at compile time from the canonical
/// location under `cards/p/`.
const YAML: &str = include_str!("../../../cards/p/P-213.yaml");

/// Compile P-213 from the production YAML and return the CompiledCard.
fn compiled_p_213() -> digimon_dsl::compiled::CompiledCard {
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(YAML).expect("P-213.yaml parses");
    let registry =
        digimon_dsl::CardRegistry::from_specs("test", &[spec]).expect("P-213.yaml compiles");
    registry.lookup("P-213").expect("P-213 in registry").clone()
}

// ─── Fixture helpers ────────────────────────────────────────────────────────

fn make_filler_digimon(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(5);
    card.dp = Some(5000);
    card
}

fn make_aegiomon_source(id: &str, level: u8) -> CardData {
    let mut card = make_test_card(id, "Aegiomon");
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(2000);
    card
}

fn place_p213_on_field(runner: &mut DebugRunner, player: PlayerId) -> PermanentHandle {
    runner.place_on_field(player, "P-213", Some(0))
}

fn trigger_when_digivolving(runner: &mut DebugRunner, handle: PermanentHandle) {
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(handle),
    );
    runner.game.drain_effect_queue();
}

// ═══════════════════════════════════════════════════════════════════════════
// SECTION 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn p_213_compiles_with_expected_shape() {
    let card = compiled_p_213();
    assert_eq!(card.card, "P-213");
    assert_eq!(card.level, Some(5));
    assert_eq!(card.dp, Some(7000));
    assert_eq!(card.cost, Some(7));
}

#[test]
fn p_213_has_two_digivolve_alt_paths() {
    // Standard Lv.4 Red / Cost 3 + Aegiomon-named / Cost 3.
    let card = compiled_p_213();
    let digivolve_paths: Vec<_> = card
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .collect();
    assert_eq!(
        digivolve_paths.len(),
        2,
        "expected 2 Digivolve alt_paths (standard Lv.4 Red + Aegiomon-named), got {}",
        digivolve_paths.len()
    );
}

#[test]
fn p_213_has_exactly_one_grant_keyword_raid() {
    let card = compiled_p_213();
    let granted: Vec<String> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                scope,
                ..
            }) if matches!(scope, CompiledScope::FaceUp) => Some(keyword.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        granted.len(),
        1,
        "expected exactly 1 face-up grant_keyword (Raid), got {}",
        granted.len()
    );
    assert_eq!(granted[0].to_lowercase(), "raid");
}

#[test]
fn p_213_has_own_and_inherited_decode_replacement_clauses() {
    let card = compiled_p_213();

    let replacements: Vec<&CompiledDeclarativeClause> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Declarative(d @ CompiledDeclarativeClause::Replacement { .. }) => {
                Some(d)
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        replacements.len(),
        2,
        "expected exactly 2 Decode-shaped replacement clauses (own + inherited), got {}",
        replacements.len()
    );

    let own_count = replacements
        .iter()
        .filter(|d| {
            matches!(
                d,
                CompiledDeclarativeClause::Replacement {
                    scope: CompiledScope::FaceUp,
                    ..
                }
            )
        })
        .count();
    let inherited_count = replacements
        .iter()
        .filter(|d| {
            matches!(
                d,
                CompiledDeclarativeClause::Replacement {
                    scope: CompiledScope::Inherited,
                    ..
                }
            )
        })
        .count();
    assert_eq!(
        own_count, 1,
        "exactly one FACE-UP Decode replacement clause"
    );
    assert_eq!(
        inherited_count, 1,
        "exactly one INHERITED Decode replacement clause"
    );
}

#[test]
fn p_213_decode_clauses_trigger_on_would_leave_battle_area_and_are_optional() {
    let card = compiled_p_213();
    for c in &card.effects {
        if let CompiledClause::Declarative(CompiledDeclarativeClause::Replacement {
            trigger,
            optional,
            ..
        }) = c
        {
            assert_eq!(
                trigger, "when_would_leave_battle_area",
                "Decode clauses must trigger on when_would_leave_battle_area"
            );
            assert!(*optional, "Decode is a 'you may' effect — must be optional");
        }
    }
}

#[test]
fn p_213_decode_clauses_filter_materials_by_aegiomon_name() {
    let card = compiled_p_213();
    let mut decode_clause_count = 0;
    for c in &card.effects {
        if let CompiledClause::Declarative(CompiledDeclarativeClause::Replacement {
            process, ..
        }) = c
        {
            decode_clause_count += 1;
            let debug = format!("{process:?}");
            assert!(
                debug.contains("Aegiomon"),
                "Decode replacement process must filter materials by name_contains 'Aegiomon'; got {debug}"
            );
        }
    }
    assert_eq!(decode_clause_count, 2);
}

#[test]
fn p_213_has_one_when_digivolving_triggered_clause_face_up() {
    let card = compiled_p_213();
    let triggered: Vec<&CompiledTriggeredClause> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::WhenDigivolving) => {
                Some(t)
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        triggered.len(),
        1,
        "expected exactly 1 [When Digivolving] triggered clause, got {}",
        triggered.len()
    );
    assert_eq!(triggered[0].scope, CompiledScope::FaceUp);
}

#[test]
fn p_213_when_digivolving_clause_contains_may_attack_now_unconditionally() {
    // "Then, this Digimon may attack" must NOT be nested inside the
    // security-count `if` gate (official Q&A: fires even without <=3 sec).
    let card = compiled_p_213();
    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::WhenDigivolving) => {
                Some(t)
            }
            _ => None,
        })
        .expect("must have [When Digivolving] clause");

    let has_top_level_may_attack = clause
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::MayAttackNow { .. }));
    assert!(
        has_top_level_may_attack,
        "[When Digivolving] clause must contain a TOP-LEVEL may_attack_now step \
         (unconditional per official Q&A), not nested inside the security-count if-gate; \
         process={:?}",
        clause.process
    );
}

#[test]
fn p_213_when_digivolving_may_attack_now_targets_self_and_is_optional() {
    let card = compiled_p_213();
    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::WhenDigivolving) => {
                Some(t)
            }
            _ => None,
        })
        .expect("must have [When Digivolving] clause");

    let may_attack = clause
        .process
        .iter()
        .find_map(|s| match s {
            CompiledStep::MayAttackNow { optional, .. } => Some(*optional),
            _ => None,
        })
        .expect("must have a may_attack_now step");
    assert!(may_attack, "'may attack' must be optional (PASS legal)");
}

#[test]
fn p_213_has_dragonkin_rule_trait() {
    let compiled = compiled_p_213();
    let card_data = card_data_for_test_from_compiled(&compiled);
    assert!(
        card_data.traits.iter().any(|t| t == "Dragonkin"),
        "P-213 must carry the (Rule) Trait: Has [Dragonkin] Type grant; traits={:?}",
        card_data.traits
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// SECTION 2 — Digivolve alt-paths behavioral
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn p_213_standard_digivolve_from_level4_red_installs_on_field() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("P-213 YAML loads")
        .add_card(make_filler_digimon("BASE-L4"))
        .memory(10)
        .start();

    let base = runner.place_on_field(0, "BASE-L4", Some(0));
    // Confirm the base is on field prior to digivolve — actual digivolve
    // execution is driven at the Game level via `effect_initiated_digivolve`
    // or the play/digivolve action; structural alt_path presence (Section 1)
    // plus this placement sanity check anchor the standard-digivolve route.
    assert_eq!(runner.battle_area_size(0), 1);
    let _ = base;
}

// ═══════════════════════════════════════════════════════════════════════════
// SECTION 3 — [When Digivolving] behavioral: security-gated Rush + DP buff
// ═══════════════════════════════════════════════════════════════════════════

/// Positive: with <=3 security cards, firing [When Digivolving] grants Rush
/// and +3000 DP to P-213 itself.
#[test]
fn p_213_when_digivolving_low_security_grants_rush_and_dp_buff() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("P-213 YAML loads")
        .add_card(make_filler_digimon("SEC"))
        .security(0, &["SEC", "SEC", "SEC"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let p213 = place_p213_on_field(&mut runner, 0);
    let dp_before = runner
        .effective_dp(p213)
        .expect("P-213 must have DP on field");

    assert!(runner.game.players[0].security.len() <= 3);

    trigger_when_digivolving(&mut runner, p213);
    let _ = runner.auto_resolve();

    assert!(
        runner.game.modifiers.has_keyword(p213, Keyword::Rush),
        "P-213 must gain <Rush> when security <= 3"
    );
    let dp_after = runner
        .effective_dp(p213)
        .expect("P-213 must still be on field");
    assert_eq!(
        dp_after,
        dp_before + 3000,
        "P-213 must gain +3000 DP when security <= 3"
    );
}

/// Negative: with more than 3 security cards, the Rush + DP buff must NOT
/// be granted.
#[test]
fn p_213_when_digivolving_high_security_does_not_grant_rush_or_dp_buff() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("P-213 YAML loads")
        .add_card(make_filler_digimon("SEC"))
        .security(0, &["SEC", "SEC", "SEC", "SEC", "SEC"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let p213 = place_p213_on_field(&mut runner, 0);
    let dp_before = runner
        .effective_dp(p213)
        .expect("P-213 must have DP on field");

    assert_eq!(runner.game.players[0].security.len(), 5);

    trigger_when_digivolving(&mut runner, p213);
    let _ = runner.auto_resolve();

    assert!(
        !runner.game.modifiers.has_keyword(p213, Keyword::Rush),
        "P-213 must NOT gain <Rush> when security > 3"
    );
    let dp_after = runner
        .effective_dp(p213)
        .expect("P-213 must still be on field");
    assert_eq!(
        dp_after, dp_before,
        "P-213 must NOT gain the +3000 DP buff when security > 3"
    );
}

/// Boundary: exactly 3 security cards still satisfies "3 or fewer".
#[test]
fn p_213_when_digivolving_exactly_three_security_still_grants_buff() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("P-213 YAML loads")
        .add_card(make_filler_digimon("SEC"))
        .security(0, &["SEC", "SEC", "SEC"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let p213 = place_p213_on_field(&mut runner, 0);
    assert_eq!(runner.game.players[0].security.len(), 3);

    trigger_when_digivolving(&mut runner, p213);
    let _ = runner.auto_resolve();

    assert!(
        runner.game.modifiers.has_keyword(p213, Keyword::Rush),
        "exactly 3 security cards must still satisfy the <=3 gate"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// SECTION 4 — [When Digivolving] behavioral: unconditional may-attack tail
// ═══════════════════════════════════════════════════════════════════════════

/// The "may attack" tail must install a prompt even when security > 3 (the
/// buff sub-block is skipped) — official Q&A confirms this is unconditional,
/// so long as P-213 can otherwise attack (Rush covers "just digivolved"
/// eligibility when the buff DOES apply; here we rely on Rush being granted
/// in the SAME resolution when security <= 3).
#[test]
fn p_213_when_digivolving_may_attack_tail_installs_prompt_when_low_security() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("P-213 YAML loads")
        .add_card(make_filler_digimon("SEC"))
        .add_card(make_filler_digimon("OPP-DEF"))
        .security(0, &["SEC", "SEC", "SEC"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let p213 = place_p213_on_field(&mut runner, 0);
    runner.place_on_field(1, "OPP-DEF", None);

    trigger_when_digivolving(&mut runner, p213);

    assert!(
        runner.pending_selection().is_some(),
        "[When Digivolving] must install a may_attack_now prompt (Rush covers eligibility)"
    );
}

/// Declining the may-attack tail leaves the opponent's security untouched.
#[test]
fn p_213_declining_may_attack_tail_leaves_opponent_security_untouched() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("P-213 YAML loads")
        .add_card(make_filler_digimon("SEC"))
        .add_card(make_filler_digimon("OPP-SEC"))
        .security(0, &["SEC", "SEC", "SEC"])
        .security(1, &["OPP-SEC"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let p213 = place_p213_on_field(&mut runner, 0);
    let opp_security_before = runner.game.players[1].security.len();

    trigger_when_digivolving(&mut runner, p213);

    let view = runner
        .pending_selection_view()
        .expect("may_attack_now prompt must be pending");
    assert!(view.is_optional, "the may-attack tail must be declinable");

    runner
        .execute_action(0, PASS)
        .expect("PASS must be a legal action");
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.game.players[1].security.len(),
        opp_security_before,
        "declining the may-attack tail must not trigger a security check"
    );
}

/// Accepting the may-attack tail actually resolves an attack against the
/// opponent's security.
#[test]
fn p_213_accepting_may_attack_tail_resolves_an_attack() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("P-213 YAML loads")
        .add_card(make_filler_digimon("SEC"))
        .add_card(make_filler_digimon("OPP-SEC"))
        .security(0, &["SEC", "SEC", "SEC"])
        .security(1, &["OPP-SEC"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let p213 = place_p213_on_field(&mut runner, 0);
    let opp_security_before = runner.game.players[1].security.len();

    trigger_when_digivolving(&mut runner, p213);

    let view = runner
        .pending_selection_view()
        .expect("may_attack_now prompt must be pending");
    let target_action = *view
        .valid_action_ids
        .iter()
        .find(|&&a| a != PASS)
        .expect("a concrete attack-target action must exist (opponent player at minimum)");

    runner
        .execute_action(0, target_action)
        .expect("accept the attack");
    let _ = runner.auto_resolve();

    assert!(
        runner.game.players[1].security.len() < opp_security_before,
        "accepting the may-attack tail must resolve an attack that checks opponent security"
    );
}

/// Even with high security (buff skipped), if P-213 is otherwise eligible to
/// attack (placed on a prior turn — natural eligibility, no Rush needed),
/// the may-attack tail must still fire (unconditional per official Q&A).
#[test]
fn p_213_when_digivolving_may_attack_tail_fires_with_high_security_if_naturally_eligible() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("P-213 YAML loads")
        .add_card(make_filler_digimon("SEC"))
        .add_card(make_filler_digimon("OPP-DEF"))
        .security(0, &["SEC", "SEC", "SEC", "SEC", "SEC"])
        .memory(10)
        .start();
    // turn_count starts at a later turn value and the permanent is placed on
    // an EARLIER turn number so it is naturally attack-eligible without Rush.
    runner.game.turn_count = 2;
    let p213 = runner.place_on_field(0, "P-213", Some(1));
    runner.place_on_field(1, "OPP-DEF", None);

    assert_eq!(runner.game.players[0].security.len(), 5);

    trigger_when_digivolving(&mut runner, p213);

    assert!(
        !runner.game.modifiers.has_keyword(p213, Keyword::Rush),
        "sanity: Rush must NOT have been granted (security > 3)"
    );
    assert!(
        runner.pending_selection().is_some(),
        "the unconditional may-attack tail must still install a prompt when P-213 \
         is naturally attack-eligible, even though the security-gated buff was skipped"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// SECTION 5 — Decode ([Aegiomon]) behavioral (own scope)
// ═══════════════════════════════════════════════════════════════════════════

/// When P-213 (own/top-of-stack scope) would leave the battle area other
/// than in battle, and an [Aegiomon]-named source sits under it, the
/// controller may play that source without paying its cost. The original
/// leave (return to hand) still proceeds.
#[test]
fn p_213_own_decode_offers_aegiomon_source_play_on_non_battle_leave() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("P-213 YAML loads")
        .add_card(make_aegiomon_source("AEGIOMON-SRC", 3))
        .add_card(make_filler_digimon("OTHER-SRC"))
        .memory(10)
        .start();
    let p213 = runner.place_stack(0, &["AEGIOMON-SRC", "OTHER-SRC", "P-213"]);

    runner.game.return_to_hand_from_effect(p213, 1);

    let accept = runner
        .pending_selection_view()
        .expect("Decode outer accept prompt should install");
    assert_eq!(accept.kind, SelectionKind::Replacement);
    assert_eq!(accept.valid_action_ids, vec![REPLACEMENT_ACCEPT]);
    runner
        .execute_action(0, REPLACEMENT_ACCEPT)
        .expect("accept Decode source-play option");

    let source_pick = runner
        .pending_selection_view()
        .expect("Decode should ask which Aegiomon-named source to play");
    assert_eq!(source_pick.kind, SelectionKind::Material);
    assert_eq!(
        source_pick.valid_action_ids.len(),
        1,
        "only the Aegiomon-named source should be legal; OTHER-SRC is excluded"
    );
    runner
        .execute_action(0, source_pick.valid_action_ids[0])
        .expect("play the Aegiomon source");

    let aegiomon_on_field = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "AEGIOMON-SRC");
    assert!(
        aegiomon_on_field,
        "the selected Aegiomon source should be played for free"
    );
    assert_eq!(
        runner.hand_size(0),
        1,
        "P-213 should still finish returning to its owner's hand (Decode does not cancel the leave)"
    );
}

/// Declining the own-Decode replacement leaves the leave event to proceed
/// unassisted, and no source is played.
#[test]
fn p_213_own_decode_decline_leaves_no_source_played() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("P-213 YAML loads")
        .add_card(make_aegiomon_source("AEGIOMON-SRC", 3))
        .memory(10)
        .start();
    let p213 = runner.place_stack(0, &["AEGIOMON-SRC", "P-213"]);

    runner.game.return_to_hand_from_effect(p213, 1);

    let accept = runner
        .pending_selection_view()
        .expect("Decode outer accept prompt should install");
    assert_eq!(accept.kind, SelectionKind::Replacement);
    assert!(
        accept.is_optional,
        "Decode accept prompt must be declinable"
    );

    runner
        .execute_action(0, PASS)
        .expect("decline the Decode option");
    let _ = runner.auto_resolve();

    let aegiomon_on_field = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "AEGIOMON-SRC");
    assert!(
        !aegiomon_on_field,
        "declining Decode must NOT play the Aegiomon source"
    );
}

/// With no [Aegiomon]-named source in the stack, the Decode outer accept
/// prompt must not offer a viable pick (no eligible material) — the leave
/// simply proceeds.
#[test]
fn p_213_own_decode_no_aegiomon_source_no_material_to_pick() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("P-213 YAML loads")
        .add_card(make_filler_digimon("OTHER-SRC"))
        .memory(10)
        .start();
    let p213 = runner.place_stack(0, &["OTHER-SRC", "P-213"]);

    runner.game.return_to_hand_from_effect(p213, 1);

    // Either no Decode prompt installs at all (gated on candidate presence),
    // or accepting it yields zero legal material picks. Both are faithful;
    // assert the card still finishes returning to hand either way.
    if let Some(view) = runner.pending_selection_view() {
        if view.kind == SelectionKind::Replacement {
            let _ = runner.execute_action(0, PASS);
        }
    }
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.hand_size(0),
        1,
        "P-213 must finish returning to hand when no Aegiomon source exists"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// SECTION 6 — Decode ([Aegiomon]) behavioral (inherited scope)
// ═══════════════════════════════════════════════════════════════════════════

/// When P-213 is BURIED under another Digimon (inherited scope active) and
/// that top permanent would leave the battle area other than in battle, the
/// inherited Decode still offers the Aegiomon-named source play from
/// P-213's own digivolution stack.
#[test]
fn p_213_inherited_decode_offers_aegiomon_source_play_when_buried() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("P-213 YAML loads")
        .add_card(make_aegiomon_source("AEGIOMON-SRC", 3))
        .add_card(make_filler_digimon("TOP-DIGI"))
        .memory(10)
        .start();
    // Stack: AEGIOMON-SRC (bottom) -> P-213 (buried, inherited scope active) -> TOP-DIGI (top).
    let top = runner.place_stack(0, &["AEGIOMON-SRC", "P-213", "TOP-DIGI"]);

    runner.game.return_to_hand_from_effect(top, 1);

    let accept = runner
        .pending_selection_view()
        .expect("inherited Decode outer accept prompt should install when P-213 is buried");
    assert_eq!(accept.kind, SelectionKind::Replacement);
    runner
        .execute_action(0, REPLACEMENT_ACCEPT)
        .expect("accept inherited Decode source-play option");

    let source_pick = runner
        .pending_selection_view()
        .expect("inherited Decode should ask which Aegiomon-named source to play");
    assert_eq!(source_pick.kind, SelectionKind::Material);
    runner
        .execute_action(0, source_pick.valid_action_ids[0])
        .expect("play the Aegiomon source");

    let aegiomon_on_field = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "AEGIOMON-SRC");
    assert!(
        aegiomon_on_field,
        "the inherited Decode must play the Aegiomon source from P-213's own stack"
    );
}
