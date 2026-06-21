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
//! | <Decode (Red/Black Lv.3)>                                       | OK (color/level-gated source play) |
//! | <Decode (Blue/Yellow Lv.3)>                                     | OK (color/level-gated source play) |
//! | [On Play] Delete opp lowest-DP                                   | PARTIAL (G-PRED-DP-LTE) |
//! | [When Attacking] Delete opp lowest-DP                            | PARTIAL (G-PRED-DP-LTE) |
//! | [When Digivolving] Bottom-deck N opp Digimon (N = same-level pairs) | OK |
//! | [When Digivolving] "Then, this Digimon may attack"               | OK (may_attack_now optional/any) |
//!
//! # Known engine/DSL gaps affecting these tests
//!
//! - **G-DECODE-PLAY-FROM-OWN-DIGIVOLUTION-SOURCES** (RESOLVED 2026-05-07
//!   for BT22-015): printed Decode on this card differs from generic
//!   `Keyword::Decode`; it plays a color/level-gated source from this card's
//!   own digivolution stack on leave-other-than-battle, then the original
//!   leave event proceeds.
//! - **G-FORMULA-SAME-LEVEL-PAIRS-REPEAT-TARGET** (RESOLVED 2026-05-07):
//!   formula `SameLevelPairsInSources` feeds `select_count_capped_multi.max`
//!   and `zone: battle_area` binds the selected opponent permanents.
//! - **G-PRED-DP-LTE**: `dp_lte` predicate (with `aggregate: lowest_dp`)
//!   parses + compiles but the engine evaluator does not narrow targets to
//!   the lowest-DP one. The delete clauses offer all opp Digimon as legal
//!   targets. Sister tests blocked on the same gap: BT24-017, BT22-013.

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep,
    CompiledTiming, CompiledTriggeredClause,
};
use digimon_engine::action::space::{PASS, REPLACEMENT_ACCEPT};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, PlayerId};
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
    make_filler_digimon_with_level(id, 6)
}

fn make_filler_digimon_with_level(id: &str, level: u8) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(11000);
    card
}

fn make_colored_level_digimon(id: &str, color: CardColor, level: u8) -> CardData {
    let mut card = make_filler_digimon_with_level(id, level);
    card.colors = vec![color];
    card
}

fn battle_area_has_top_card(runner: &DebugRunner, player: PlayerId, card_id: &str) -> bool {
    runner.game.players[player as usize]
        .battle_area
        .iter()
        .any(|perm| perm.top_card().card_id(&runner.game.card_data) == card_id)
}

fn place_bt22_on_field(runner: &mut DebugRunner, player: PlayerId) -> PermanentHandle {
    runner.place_on_field(player, "BT22-015", Some(0))
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 1 — Structural assertions (parse / compile / topology)
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn bt22_015_compiles_with_three_alt_paths_and_grant_keyword_and_two_triggered() {
    let card = compiled_bt22_015();

    // Standard Lv.6/Red Digivolve (0) + printed "Lv.6 w/[CS] trait: Cost 5" alt (1,
    // added under promote-official-bandai-card-source) + DnaDigivolve (2).
    assert_eq!(
        card.alt_paths.len(),
        3,
        "expected 3 alt_paths (Digivolve + CS-trait Digivolve + DnaDigivolve), got {}",
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
fn bt22_015_third_alt_path_is_dna_digivolve_greymon_garurumon_cost0() {
    let card = compiled_bt22_015();
    // Index 2: the CS-trait alt (index 1) was inserted between the standard route (0)
    // and the DNA route under promote-official-bandai-card-source.
    let path = &card.alt_paths[2];
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
// SECTION 4 — Decode and remaining blocked coverage
// ═══════════════════════════════════════════════════════════════════════════════

/// When BT22-015 would leave the battle area outside of battle, the player
/// may play a Red/Black Lv.3 source from BT22-015's digivolution stack
/// without paying the cost. Decode does not cancel the original leave event.
#[test]
fn bt22_015_decode_red_black_offers_play_from_source_stack_on_leave() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT22-015 YAML loads")
        .add_card(make_colored_level_digimon("RED-L3", CardColor::Red, 3))
        .add_card(make_colored_level_digimon("GREEN-L3", CardColor::Green, 3))
        .add_card(make_colored_level_digimon("RED-L4", CardColor::Red, 4))
        .memory(15)
        .start();
    let bt22 = runner.place_stack(0, &["RED-L3", "GREEN-L3", "RED-L4", "BT22-015"]);

    runner.game.return_to_hand_from_effect(bt22, 1);

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
        .expect("Decode should ask which matching source to play");
    assert_eq!(source_pick.kind, SelectionKind::Material);
    assert_eq!(
        source_pick.valid_action_ids.len(),
        1,
        "only Red/Black Lv.3 sources should be legal; off-color Lv.3 and Red Lv.4 are excluded"
    );
    runner
        .execute_action(0, source_pick.valid_action_ids[0])
        .expect("play Red Lv.3 source");

    assert!(
        battle_area_has_top_card(&runner, 0, "RED-L3"),
        "the selected Red Lv.3 source should be played for free"
    );
    assert!(
        !battle_area_has_top_card(&runner, 0, "BT22-015"),
        "Decode does not cancel or redirect the original leave event"
    );
    assert_eq!(
        runner.hand_size(0),
        1,
        "BT22-015 should still finish returning to its owner's hand"
    );
}

/// Same as the Red/Black Decode clause, but gated to Blue/Yellow Lv.3
/// sources.
#[test]
fn bt22_015_decode_blue_yellow_offers_play_from_source_stack_on_leave() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT22-015 YAML loads")
        .add_card(make_colored_level_digimon(
            "YELLOW-L3",
            CardColor::Yellow,
            3,
        ))
        .add_card(make_colored_level_digimon("GREEN-L3", CardColor::Green, 3))
        .add_card(make_colored_level_digimon("BLUE-L4", CardColor::Blue, 4))
        .memory(15)
        .start();
    let bt22 = runner.place_stack(0, &["YELLOW-L3", "GREEN-L3", "BLUE-L4", "BT22-015"]);

    runner.game.return_to_hand_from_effect(bt22, 1);
    runner
        .execute_action(0, REPLACEMENT_ACCEPT)
        .expect("accept Decode source-play option");

    let source_pick = runner
        .pending_selection_view()
        .expect("Decode should ask which matching source to play");
    assert_eq!(
        source_pick.valid_action_ids.len(),
        1,
        "only Blue/Yellow Lv.3 sources should be legal; off-color Lv.3 and Blue Lv.4 are excluded"
    );
    runner
        .execute_action(0, source_pick.valid_action_ids[0])
        .expect("play Yellow Lv.3 source");

    assert!(
        battle_area_has_top_card(&runner, 0, "YELLOW-L3"),
        "the selected Yellow Lv.3 source should be played for free"
    );
    assert!(!battle_area_has_top_card(&runner, 0, "BT22-015"));
}

/// With K same-level pairs in BT22-015's digivolution stack, the player is
/// offered up to K opponent Digimon to bottom-deck before the follow-up
/// attack prompt.
#[test]
fn bt22_015_when_digivolving_bottom_decks_n_opp_digimon_per_same_level_pair() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT22-015 YAML loads")
        .add_card(make_filler_digimon_with_level("SRC-L6A", 6))
        .add_card(make_filler_digimon_with_level("SRC-L6B", 6))
        .add_card(make_filler_digimon_with_level("SRC-L5A", 5))
        .add_card(make_filler_digimon_with_level("SRC-L5B", 5))
        .add_card(make_filler_digimon_with_level("SRC-L4", 4))
        .add_card(make_filler_digimon("OPP-A"))
        .add_card(make_filler_digimon("OPP-B"))
        .add_card(make_filler_digimon("OPP-C"))
        .memory(15)
        .start();
    runner.game.turn_count = 1;

    let bt22 = runner.place_stack(
        0,
        &[
            "SRC-L6A", "SRC-L6B", "SRC-L5A", "SRC-L5B", "SRC-L4", "BT22-015",
        ],
    );
    runner.place_on_field(1, "OPP-A", None);
    runner.place_on_field(1, "OPP-B", None);
    runner.place_on_field(1, "OPP-C", None);
    let opp_deck_before = runner.deck_size(1);

    trigger_when_digivolving(&mut runner, bt22);

    let pending = runner
        .pending_selection_view()
        .expect("same-level pair bottom-deck selection installs before may_attack_now");
    assert_eq!(
        pending.selecting_player, 0,
        "BT22-015's controller chooses the opponent Digimon to bottom-deck"
    );
    assert_eq!(
        pending.valid_action_ids.len(),
        3,
        "three opponent Digimon should be legal candidates before the first pick"
    );
    assert!(
        !pending.valid_action_ids.contains(&PASS),
        "the mandatory bottom-deck arm cannot be declined while targets exist"
    );
    // Bottom-decking opponent Digimon is surfaced as an opponent-field
    // selection so the UI's field-click router can map board clicks (the pick
    // cap of 2 same-level pairs is still enforced by the engine, just no longer
    // exposed via the selection-kind tag).
    assert_eq!(
        pending.kind,
        SelectionKind::OppField,
        "expected opponent-field selection, got {:?}",
        pending.kind
    );

    let first = runner
        .pending_selection_view()
        .expect("first pick still pending")
        .valid_action_ids[0];
    runner
        .execute_action(0, first)
        .expect("select first opponent Digimon to bottom-deck");
    let second = runner
        .pending_selection_view()
        .expect("second pick still pending")
        .valid_action_ids[0];
    runner
        .execute_action(0, second)
        .expect("select second opponent Digimon to bottom-deck");

    assert_eq!(
        runner.battle_area_size(1),
        1,
        "exactly two opponent Digimon should be moved from battle area"
    );
    assert_eq!(
        runner.deck_size(1),
        opp_deck_before + 2,
        "bottom-decked Digimon should be placed in opponent deck"
    );
    assert!(
        runner.pending_selection().is_some(),
        "the 'Then, this Digimon may attack' prompt should still follow"
    );
}
