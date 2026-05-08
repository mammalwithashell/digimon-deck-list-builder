//! BT22-015 Omnimon — Digimon, Lv.7, Red+White, DP 15000, Cost 15.
//! Traits: Holy Warrior, Royal Knight, CS.
//! Evo: Lv.6 Red / Cost 6
//! DNA Digivolve: Lv.6 w/[Greymon] + Lv.6 w/[Garurumon] / Cost 0
//!
//! # Card text (cards.json — verbatim)
//!
//! ＜Blocker＞
//! ＜Decode (Red/Black Lv.3)＞
//!   (When this Digimon would leave the battle area other than in battle,
//!    you may play 1 Red or Black Level 3 Digimon card from its digivolution
//!    cards without paying the cost.)
//! ＜Decode (Blue/Yellow Lv.3)＞
//! [On Play] [When Attacking] Delete 1 of your opponent's Digimon with the
//!   lowest DP.
//! [When Digivolving] For every 2 same-level cards this Digimon's stack has,
//!   return 1 of your opponent's Digimon to the bottom of the deck. Then,
//!   this Digimon may attack.
//!
//! Inherited / Security: (none)
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT22/Red/BT22_015.cs
//!
//! # Patterns this test covers
//! - Structural: 2 alt_paths (Digivolve Lv.6/cost-6 + DNA Greymon+Garurumon
//!   cost 0) + grant_keyword Blocker + 2 triggered clauses ([On Play, When
//!   Attacking] delete-lowest-DP and [When Digivolving] may_attack_now).
//! - F1-adjacent Lowest-DP delete (`dp_lte: { aggregate: { selector:
//!   lowest_dp, scope: opponent } }`) — same shape as BT24-017 / BT22-013.
//! - G-MAY-ATTACK-NOW (resolved 2026-05-03) — `may_attack_now` with
//!   targets: any after [When Digivolving].
//!
//! # Sister cards (cross-reference)
//! - BT22-013 WarGreymon — same lowest-DP delete pattern (G-PRED-DP-LTE).
//! - BT17-078 Omnimon — sister Omnimon variant, same Greymon+Garurumon DNA
//!   recipe shape. BT17-078 is white-mono / cost 9; BT22-015 is red+white /
//!   cost 15.
//! - BT17-081 Tai Kamiya & Matt Ishida — sister `may_attack_now` user
//!   (theirs is `targets: player`; ours is `targets: any`).
//!
//! # Faithfulness diff vs. card text
//!
//! | Card-text element                                              | Status         |
//! |----------------------------------------------------------------|----------------|
//! | Standard digivolve Lv.6 Red / Cost 6                            | OK             |
//! | DNA Lv.6 Greymon-named + Lv.6 Garurumon-named / Cost 0          | OK             |
//! | <Blocker>                                                       | OK (grant_keyword) |
//! | <Decode (Red/Black Lv.3)>                                       | BLOCKED (G-DECODE-PLAY-FROM-OWN-DIGIVOLUTION-SOURCES) |
//! | <Decode (Blue/Yellow Lv.3)>                                     | BLOCKED (G-DECODE-PLAY-FROM-OWN-DIGIVOLUTION-SOURCES) |
//! | [On Play] Delete opp lowest-DP                                   | PARTIAL (G-PRED-DP-LTE) |
//! | [When Attacking] Delete opp lowest-DP                            | PARTIAL (G-PRED-DP-LTE) |
//! | [When Digivolving] Bottom-deck N opp Digimon (N = same-level pairs) | BLOCKED (G-FORMULA-SAME-LEVEL-PAIRS-REPEAT-TARGET) |
//! | [When Digivolving] "Then, this Digimon may attack"               | OK (may_attack_now optional/any) |
//!
//! # Known engine/DSL gaps affecting these tests
//!
//! - **G-DECODE-PLAY-FROM-OWN-DIGIVOLUTION-SOURCES** (NEW): printed Decode
//!   on this card differs from generic `Keyword::Decode`; it allows playing
//!   a Lv.3 Red/Black or Blue/Yellow source from this card's own
//!   digivolution stack on leave-other-than-battle. No DSL verb / keyword
//!   variant exists. Decode clauses are OMITTED from the YAML.
//! - **G-FORMULA-SAME-LEVEL-PAIRS-REPEAT-TARGET** (PARTIALLY RESOLVED):
//!   formula `SameLevelPairsInSources` is implemented but cannot feed a
//!   repeat-count target selection. The bottom-deck arm of [When Digivolving]
//!   is OMITTED.
//! - **G-PRED-DP-LTE**: `dp_lte` predicate (with `aggregate: lowest_dp`)
//!   parses + compiles but the engine evaluator does not narrow targets to
//!   the lowest-DP one. The delete clauses offer all opp Digimon as legal
//!   targets. Sister tests blocked on the same gap: BT24-017, BT22-013.

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep,
    CompiledTiming, CompiledTriggeredClause,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming, PlayerId};
use digimon_engine::events::GameEvent;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

/// Production YAML for BT22-015, inlined at compile time from the canonical
/// location under `cards/bt22/`.
const YAML: &str = include_str!("../../../cards/bt22/BT22-015.yaml");

/// Compile BT22-015 from the production YAML and return the CompiledCard.
fn compiled_bt22_015() -> digimon_dsl::compiled::CompiledCard {
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(YAML).expect("BT22-015.yaml parses");
    let registry =
        digimon_dsl::CardRegistry::from_specs("test", &[spec]).expect("BT22-015.yaml compiles");
    registry
        .lookup("BT22-015")
        .expect("BT22-015 in registry")
        .clone()
}

// ─── Fixture helpers ────────────────────────────────────────────────────────

fn make_opp_digimon(id: &str, name: &str, dp: i32) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Digimon;
    card.level = Some(5);
    card.dp = Some(dp);
    card
}

fn make_filler_digimon(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(6);
    card.dp = Some(11000);
    card
}

fn place_bt22_on_field(runner: &mut DebugRunner, player: PlayerId) -> PermanentHandle {
    runner.place_on_field(player, "BT22-015", Some(0))
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 1 — Structural assertions (parse / compile / topology)
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn bt22_015_compiles_with_two_alt_paths_and_grant_keyword_and_two_triggered() {
    let card = compiled_bt22_015();

    assert_eq!(
        card.alt_paths.len(),
        2,
        "expected exactly 2 alt_paths (Digivolve + DnaDigivolve), got {}",
        card.alt_paths.len()
    );

    let triggered: Vec<&CompiledTriggeredClause> = card
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
        "expected exactly 2 triggered clauses ([On Play, When Attacking] + [When Digivolving]); got {}",
        triggered.len()
    );

    // grant_keyword Blocker — exactly 1 face-up declarative.
    let grant_count = card
        .effects
        .iter()
        .filter(|c| {
            matches!(
                c,
                CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { .. })
            )
        })
        .count();
    assert_eq!(
        grant_count, 1,
        "expected exactly 1 grant_keyword (Blocker), got {}",
        grant_count
    );
}

#[test]
fn bt22_015_first_alt_path_is_digivolve_lv6_red_cost6() {
    let card = compiled_bt22_015();
    let path = &card.alt_paths[0];
    assert_eq!(path.kind, CompiledAltPathKind::Digivolve);
    assert_eq!(
        path.cost,
        Some(digimon_dsl::compiled::CompiledCost::Literal(6)),
        "standard digivolve must be Lv.6/Red/cost-6 (printed cards.json evo_costs)"
    );
    assert!(
        !path.ignore_requirements,
        "standard digivolve must NOT ignore requirements"
    );
}

#[test]
fn bt22_015_second_alt_path_is_dna_digivolve_greymon_garurumon_cost0() {
    let card = compiled_bt22_015();
    let path = &card.alt_paths[1];
    assert_eq!(path.kind, CompiledAltPathKind::DnaDigivolve);
    assert_eq!(
        path.cost,
        Some(digimon_dsl::compiled::CompiledCost::Literal(0)),
        "DNA digivolve must be cost 0 (printed dna_costs)"
    );
    let mats = &path.materials;
    assert_eq!(
        mats.len(),
        2,
        "DNA digivolve must have exactly 2 materials (Greymon + Garurumon), got {}",
        mats.len()
    );
}

#[test]
fn bt22_015_grant_keyword_is_blocker() {
    let card = compiled_bt22_015();
    let granted: Vec<String> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                ..
            }) => Some(keyword.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(granted.len(), 1, "expected exactly 1 grant_keyword");
    assert_eq!(
        granted[0].to_lowercase(),
        "blocker",
        "grant_keyword must be Blocker (printed <Blocker>)"
    );
}

#[test]
fn bt22_015_on_play_when_attacking_clause_is_face_up_mandatory() {
    let card = compiled_bt22_015();
    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if matches!(t.scope, CompiledScope::FaceUp)
                    && t.when.contains(&CompiledTiming::OnPlay)
                    && t.when.contains(&CompiledTiming::WhenAttacking) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("must have a face-up [On Play, When Attacking] clause");

    assert!(
        !clause.optional,
        "delete-lowest-DP clause is mandatory (printed has no 'may')"
    );
    assert!(
        !clause.once_per_turn,
        "no [Once Per Turn] on the delete-lowest-DP clause"
    );
}

#[test]
fn bt22_015_when_digivolving_clause_contains_may_attack_now() {
    let card = compiled_bt22_015();
    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if matches!(t.scope, CompiledScope::FaceUp)
                    && t.when.contains(&CompiledTiming::WhenDigivolving) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("must have a face-up [When Digivolving] clause");

    let has_may_attack_now = clause
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::MayAttackNow { .. }));
    assert!(
        has_may_attack_now,
        "[When Digivolving] clause must contain a may_attack_now step (printed 'Then, this Digimon may attack')"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 2 — [On Play] / [When Attacking] delete-lowest-DP behavioral
// ═══════════════════════════════════════════════════════════════════════════════

fn trigger_when_attacking(runner: &mut DebugRunner, handle: PermanentHandle) {
    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(handle),
    );
    runner.game.drain_effect_queue();
}

fn trigger_on_play(runner: &mut DebugRunner, handle: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(handle));
    runner.game.drain_effect_queue();
}

/// Positive: firing [On Play] with a single legal opp Digimon installs the
/// delete prompt and the auto-resolve removes that Digimon.
#[test]
fn bt22_015_on_play_deletes_only_legal_opp_digimon() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT22-015 YAML loads")
        .add_card(make_opp_digimon("OPP-LOW", "OppLow", 3000))
        .memory(15)
        .start();

    let bt22 = place_bt22_on_field(&mut runner, 0);
    runner.place_on_field(1, "OPP-LOW", None);
    let opp_before = runner.battle_area_size(1);

    trigger_on_play(&mut runner, bt22);
    let _ = runner.auto_resolve();

    let opp_after = runner.battle_area_size(1);
    assert_eq!(
        opp_after,
        opp_before - 1,
        "[On Play] must delete the (only / lowest-DP) opp Digimon"
    );
}

/// Positive: firing [When Attacking] with a single legal opp Digimon
/// installs the delete prompt and the auto-resolve removes that Digimon.
#[test]
fn bt22_015_when_attacking_deletes_only_legal_opp_digimon() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT22-015 YAML loads")
        .add_card(make_opp_digimon("OPP-LOW", "OppLow", 3000))
        .memory(15)
        .start();

    let bt22 = place_bt22_on_field(&mut runner, 0);
    runner.place_on_field(1, "OPP-LOW", None);
    let opp_before = runner.battle_area_size(1);

    trigger_when_attacking(&mut runner, bt22);
    let _ = runner.auto_resolve();

    let opp_after = runner.battle_area_size(1);
    assert_eq!(
        opp_after,
        opp_before - 1,
        "[When Attacking] must delete the (only / lowest-DP) opp Digimon"
    );
}

/// Negative: with NO opp Digimon, the [On Play] clause must not panic and
/// must leave the field unchanged.
#[test]
fn bt22_015_on_play_no_opp_digimon_does_not_panic() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT22-015 YAML loads")
        .memory(15)
        .start();

    let bt22 = place_bt22_on_field(&mut runner, 0);
    let opp_before = runner.battle_area_size(1);

    trigger_on_play(&mut runner, bt22);
    // No assertion needed beyond non-panic and field unchanged.
    assert_eq!(runner.battle_area_size(1), opp_before);
}

/// BLOCKED — G-PRED-DP-LTE: with two opp Digimon at different DPs, only
/// the LOWEST-DP one should be a legal delete target. The predicate
/// evaluator does not yet honor `dp_lte` on permanents; the prompt offers
/// both. Sister gap as BT24-017, BT22-013.
#[test]
#[ignore = "BLOCKED: G-PRED-DP-LTE — dp_lte predicate (with aggregate lowest_dp) parses and \
            compiles but is not evaluated for permanents in code/digimon-engine/src/dsl_cards/predicate.rs; \
            the lowest-DP filter degenerates to 'any opp Digimon'. Same gap blocks BT24-017 \
            (`bt24_017_delete_targets_only_lowest_dp_digimon`) and BT22-013 \
            (`bt22_013_when_digivolving_branch_1_only_lowest_dp_is_a_legal_target`)."]
fn bt22_015_on_play_only_lowest_dp_is_a_legal_target() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT22-015 YAML loads")
        .add_card(make_opp_digimon("OPP-LOW", "OppLow", 3000))
        .add_card(make_opp_digimon("OPP-HIGH", "OppHigh", 11000))
        .memory(15)
        .start();

    let bt22 = place_bt22_on_field(&mut runner, 0);
    runner.place_on_field(1, "OPP-LOW", None);
    runner.place_on_field(1, "OPP-HIGH", None);

    trigger_on_play(&mut runner, bt22);

    let view = runner
        .pending_selection_view()
        .expect("delete-target selection installs");
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only the lowest-DP (3000) opp Digimon should be a legal delete target"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 3 — [When Digivolving] may_attack_now behavioral
// ═══════════════════════════════════════════════════════════════════════════════

fn trigger_when_digivolving(runner: &mut DebugRunner, handle: PermanentHandle) {
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(handle),
    );
    runner.game.drain_effect_queue();
}

/// Positive: firing [When Digivolving] installs the may_attack_now prompt
/// (an attack-target selection bound to BT22-015 as the attacker). The
/// player can decline (optional: true).
#[test]
fn bt22_015_when_digivolving_installs_attack_prompt() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT22-015 YAML loads")
        .add_card(make_filler_digimon("OPP-DEF"))
        .memory(15)
        .start();
    runner.game.turn_count = 1;

    let bt22 = place_bt22_on_field(&mut runner, 0);
    runner.place_on_field(1, "OPP-DEF", None);

    trigger_when_digivolving(&mut runner, bt22);

    // Some kind of pending selection should install — the attack prompt
    // (or the upstream effect-resolution flow that culminates in it). The
    // exact SelectionKind shape depends on engine wiring of
    // may_attack_now; existence of a pending selection is the acceptance
    // criterion at this level.
    assert!(
        runner.pending_selection().is_some(),
        "[When Digivolving] must install a may_attack_now (attack-target) selection"
    );
}

/// Negative: with no opp Digimon at all, may_attack_now (`targets: any`)
/// can still target the opp PLAYER directly, so a prompt should still
/// install. The acceptance criterion is non-panic. Player-only attacks
/// remain a legal option per DCGO `canAttackPlayerCondition: () => true`.
#[test]
fn bt22_015_when_digivolving_no_opp_digimon_does_not_panic() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT22-015 YAML loads")
        .memory(15)
        .start();
    runner.game.turn_count = 1;

    let bt22 = place_bt22_on_field(&mut runner, 0);

    trigger_when_digivolving(&mut runner, bt22);
    // Non-panic is the acceptance criterion. The optional/any-target
    // attack prompt may or may not install depending on whether the
    // attacker can attack at all (turn_count gating, suspension state),
    // but must not panic.
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 4 — BLOCKED tests for omitted clauses
// ═══════════════════════════════════════════════════════════════════════════════

/// BLOCKED — G-DECODE-PLAY-FROM-OWN-DIGIVOLUTION-SOURCES: when BT22-015
/// would leave the battle area outside of battle, the player should be
/// offered the chance to play a Lv.3 Red/Black source from BT22-015's
/// digivolution stack without paying the cost. No DSL verb / keyword
/// variant exists today that surfaces a play-from-source-stack option on
/// leave-other-than-battle.
#[test]
#[ignore = "BLOCKED: G-DECODE-PLAY-FROM-OWN-DIGIVOLUTION-SOURCES — printed Decode on this card \
            differs from generic Keyword::Decode (which redirects the leaving permanent to hand). \
            BT22-015's Decode allows playing a Lv.3 source from THIS card's digivolution stack \
            on leave-other-than-battle; no DSL verb or keyword variant exists. The two Decode \
            clauses are OMITTED from the YAML until the gap closes."]
fn bt22_015_decode_red_black_offers_play_from_source_stack_on_leave() {
    // Setup: BT22-015 on field with a Red Lv.3 source under it; trigger a
    // would-leave-other-than-battle event (e.g. a delete from an effect, not
    // from combat). The player should be offered the optional play of the
    // Red Lv.3 source from the digivolution stack, free.
    //
    // Until the gap closes, no replacement-clause body exists in the YAML
    // for the Decode clauses, so no prompt installs. This test serves as the
    // regression once a DSL replacement-clause variant lands that can issue
    // a `play_from_source_stack` step.
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT22-015 YAML loads")
        .memory(15)
        .start();
    let _ = runner;
}

/// BLOCKED — G-DECODE-PLAY-FROM-OWN-DIGIVOLUTION-SOURCES: same as above
/// for the Blue/Yellow Lv.3 Decode clause.
#[test]
#[ignore = "BLOCKED: G-DECODE-PLAY-FROM-OWN-DIGIVOLUTION-SOURCES — see Red/Black sibling test."]
fn bt22_015_decode_blue_yellow_offers_play_from_source_stack_on_leave() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT22-015 YAML loads")
        .memory(15)
        .start();
    let _ = runner;
}

/// BLOCKED — G-FORMULA-SAME-LEVEL-PAIRS-REPEAT-TARGET: with K same-level
/// pairs in BT22-015's digivolution stack, the player should be offered up
/// to K opp Digimon to bottom-deck. The formula leaf is resolved
/// (SameLevelPairsInSources), but feeding it as the COUNT bound on a
/// player-visible repeat-target select is open. The bottom-deck arm of
/// [When Digivolving] is OMITTED from the YAML until the gap closes.
#[test]
#[ignore = "BLOCKED: G-FORMULA-SAME-LEVEL-PAIRS-REPEAT-TARGET — formula leaf \
            (SameLevelPairsInSources) is resolved, but feeding it as a repeat-count bound on \
            a player-visible target selection is open. The bottom-deck arm of [When Digivolving] \
            is OMITTED from the YAML; the may_attack_now arm IS implemented and tested above."]
fn bt22_015_when_digivolving_bottom_decks_n_opp_digimon_per_same_level_pair() {
    // Setup intent: stack BT22-015 over sources with several same-level
    // pairs (e.g. 4 Lv.6 sources → 2 pairs → bottom-deck up to 2 opp
    // Digimon). With K opp Digimon on the field, the prompt should accept
    // up to min(K, 2) bottom-deck targets. Until the gap closes, no
    // bottom-deck arm exists in the YAML, so this assertion cannot run.
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT22-015 YAML loads")
        .memory(15)
        .start();
    let _ = runner;
}
