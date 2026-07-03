//! P-207 Minervamon — Digimon, Lv.6, Yellow/Purple, DP 12000, Cost 12.
//! Traits: Shaman, Olympos XII, Iliad, TS. Attribute: Virus.
//!
//! # Card text (official Bandai DB bundle — data/card_bundles/P-207.md)
//!
//! ＜Alliance＞ (When this Digimon attacks, by suspending 1 of your other
//! Digimon, add the suspended Digimon's DP to this Digimon and it gains
//! ＜Security A. +1＞ for the attack.)
//! [On Play] [When Digivolving] You may play 1 level 4 or lower Digimon card
//! with [Avian], [Bird], [Beast], [Animal] or [Sovereign], other than [Sea
//! Animal], in any of its traits or the [TS] trait from your hand without
//! paying the cost.
//! [When Attacking] [Once Per Turn] You may play 1 level 4 or lower Digimon
//! card with [Avian], [Bird], [Beast], [Animal] or [Sovereign], other than
//! [Sea Animal], in any of its traits or the [TS] trait from your trash
//! without paying the cost.
//!
//! Digivolution requirements (official Bandai DB — authoritative):
//!   Standard: Yellow Lv.5 / cost 4 (cards.json evo_costs)
//!             Purple Lv.5 / cost 4 (MISSING from cards.json — API drops the
//!             second colour circle; added as an explicit alt_path)
//!   Special:  [Digivolve] Lv.5 w/[Beastkin]/[TS] trait: Cost 3
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/P/Yellow/P_207.cs
//!
//! # Patterns this test covers
//! - H10 Alliance keyword (declarative grant — structural + runtime-installed)
//! - Alt-digivolve: off-colour standard circle (Purple Lv.5/cost 4) recovered
//!   from the official DB after the lossy cards.json ingest dropped it
//! - Alt-digivolve: exact-trait-gated special circle (Beastkin OR TS, cost 3)
//! - E-adjacent: "you may play X from hand/trash free" with a substring-trait
//!   OR-filter (Avian/Bird/Beast/Animal(not Sea Animal)/Sovereign) OR exact
//!   TS-trait, shared between On Play and When Digivolving
//! - E2 OPT + optional decline (When Attacking clause)
//! - Negative trait-filter coverage per branch (Avian/Bird/Beast/Sovereign
//!   substring, Animal-minus-Sea-Animal carve-out, TS exact, level cap)

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledScope,
    CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, Keyword, PlaySource};
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "P-207";

// ── Fixture builders ─────────────────────────────────────────────────────────

fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

/// A Lv.5 Yellow base — matches the printed standard circle already present
/// in cards.json evo_costs.
fn yellow_lv5_base(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.colors = vec![CardColor::Yellow];
    c
}

/// A Lv.5 Purple base — matches the off-colour standard circle dropped by
/// the cards.json ingest but confirmed on the official Bandai DB bundle.
fn purple_lv5_base(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.colors = vec![CardColor::Purple];
    c
}

/// A Lv.5 base with the exact [Beastkin] trait (any colour) — matches the
/// special digivolve condition's first arm.
fn beastkin_lv5_base(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.colors = vec![CardColor::Red];
    c.traits = vec!["Beastkin".to_string()];
    c
}

/// A Lv.5 base with the exact [TS] trait (any colour) — matches the special
/// digivolve condition's second arm.
fn ts_lv5_base(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.colors = vec![CardColor::Green];
    c.traits = vec!["TS".to_string()];
    c
}

/// A Lv.5 base with NEITHER [Beastkin] nor [TS] nor a matching standard
/// colour — must be rejected by every alt-path.
fn plain_lv5_base(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.colors = vec![CardColor::Red];
    c
}

/// A level-`level` Digimon carrying exactly the traits given, for the
/// On Play/When Digivolving/When Attacking free-play filter tests.
fn candidate(id: &str, level: u8, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.colors = vec![CardColor::Red];
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn minervamon_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("P-207 YAML must parse and compile")
        .start()
}

fn hand_contains(runner: &DebugRunner, player: usize, card_id: &str) -> bool {
    runner.game.players[player]
        .hand
        .iter()
        .any(|c| c.card_id(&runner.game.card_data) == card_id)
}

fn trash_contains(runner: &DebugRunner, player: usize, card_id: &str) -> bool {
    runner.game.players[player]
        .trash
        .iter()
        .any(|c| c.card_id(&runner.game.card_data) == card_id)
}

fn field_contains(runner: &DebugRunner, player: usize, card_id: &str) -> bool {
    runner.game.players[player]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == card_id)
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 1 — Structural assertions
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn p_207_yaml_parses_and_compiles() {
    let _runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("P-207 YAML must parse and compile without errors");
}

#[test]
fn p_207_has_printed_metadata() {
    let runner = minervamon_runner();
    let compiled = runner.compiled_card(CARD_ID).expect("P-207 compiled");

    assert_eq!(compiled.name, "Minervamon");
    assert_eq!(compiled.kind, digimon_dsl::compiled::CompiledCardKind::Digimon);
    assert_eq!(compiled.level, Some(6));
    assert_eq!(compiled.cost, Some(12));
    assert_eq!(compiled.dp, Some(12000));
    for t in ["Shaman", "Olympos XII", "Iliad", "TS"] {
        assert!(
            compiled.traits.iter().any(|x| x == t),
            "missing trait {t}; traits={:?}",
            compiled.traits
        );
    }
}

/// P-207 must compile to exactly 3 clauses: grant_keyword(Alliance),
/// [On Play][When Digivolving] shared body, [When Attacking][OPT] body.
#[test]
fn p_207_has_three_clauses() {
    let runner = minervamon_runner();
    let compiled = runner.compiled_card(CARD_ID).expect("P-207 compiled");
    assert_eq!(
        compiled.effects.len(),
        3,
        "expected 3 clauses (Alliance grant, OnPlay/WhenDigivolving, WhenAttacking OPT); got {}",
        compiled.effects.len()
    );
}

#[test]
fn p_207_declares_alliance_keyword_grant() {
    let runner = minervamon_runner();
    let compiled = runner.compiled_card(CARD_ID).expect("P-207 compiled");

    let has_alliance = compiled.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { keyword, .. })
                if keyword == "Alliance"
        )
    });
    assert!(has_alliance, "P-207 must declare an Alliance keyword grant");
}

#[test]
fn p_207_on_play_when_digivolving_clause_is_optional_and_not_once_per_turn() {
    let runner = minervamon_runner();
    let compiled = runner.compiled_card(CARD_ID).expect("P-207 compiled");

    let clause = compiled
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
        .expect("P-207 must have a combined [On Play][When Digivolving] clause");

    assert!(clause.optional, "the 'you may' must be optional=true");
    assert!(
        !clause.once_per_turn,
        "DCGO maxUsableCount: -1 — the On Play/When Digivolving clause is NOT once per turn"
    );
    assert_eq!(clause.scope, CompiledScope::FaceUp);
}

#[test]
fn p_207_when_attacking_clause_is_optional_and_once_per_turn() {
    let runner = minervamon_runner();
    let compiled = runner.compiled_card(CARD_ID).expect("P-207 compiled");

    let clause = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when == vec![CompiledTiming::WhenAttacking] =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("P-207 must have a [When Attacking] clause");

    assert!(clause.optional, "the 'you may' must be optional=true");
    assert!(
        clause.once_per_turn,
        "DCGO maxUsableCount: 1 — the When Attacking clause IS once per turn"
    );
    assert_eq!(clause.scope, CompiledScope::FaceUp);
}

#[test]
fn p_207_alt_paths_present_yellow_purple_standard_beastkin_and_ts() {
    let runner = minervamon_runner();
    let compiled = runner.compiled_card(CARD_ID).expect("P-207 compiled");

    let digivolve_paths: Vec<_> = compiled
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .collect();

    assert_eq!(
        digivolve_paths.len(),
        4,
        "expected exactly 4 digivolve alt_paths (Yellow standard, Purple \
         standard, Beastkin special, TS special); got {}",
        digivolve_paths.len()
    );

    let has_cost = |cost: &Option<CompiledCost>, n: i64| {
        matches!(cost, Some(CompiledCost::Literal(v)) if *v as i64 == n)
    };
    assert_eq!(
        digivolve_paths.iter().filter(|p| has_cost(&p.cost, 4)).count(),
        2,
        "must have exactly 2 cost-4 alt-paths (Yellow + Purple standard circles)"
    );
    assert_eq!(
        digivolve_paths.iter().filter(|p| has_cost(&p.cost, 3)).count(),
        2,
        "must have exactly 2 cost-3 alt-paths (Beastkin, TS special circle)"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 2 — Alliance keyword: declared + runtime-installed
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn p_207_alliance_keyword_runtime_installed() {
    let mut runner = minervamon_runner();
    let mine = runner.place_on_field(0, CARD_ID, Some(0));

    runner.game.tick_declarative_effects();

    assert!(
        runner.game.has_keyword(mine, Keyword::Alliance),
        "P-207 must have the Alliance keyword after a declarative tick"
    );
}

/// Both P0 and P1 copies of P-207 grant Alliance to their OWN controller's
/// permanent — confirms the grant is self-scoped (own, not inherited or
/// opponent-targeted), matching DCGO's `isInheritedEffect: false`.
#[test]
fn p_207_alliance_keyword_is_self_scoped_for_both_controllers() {
    let mut runner = minervamon_runner();
    let mine = runner.place_on_field(0, CARD_ID, Some(0));
    let opp = runner.place_on_field(1, CARD_ID, Some(0));
    runner.game.tick_declarative_effects();

    assert!(runner.game.has_keyword(mine, Keyword::Alliance));
    assert!(runner.game.has_keyword(opp, Keyword::Alliance));
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 3 — On Play / When Digivolving filter: positive branches
// ─────────────────────────────────────────────────────────────────────────────

/// Drives the shared [On Play][When Digivolving] clause via DIGIVOLVING
/// (not raw hand-play) because P-207's own printed play cost is 12 — above
/// the engine's hard memory-pool cap of 10 (`Rules::memory_range == (-10,
/// 10)`), so paying it via `play_from_hand` would cross memory negative and
/// trigger a real `check_turn_end()` turn-pass + sign-flip, corrupting any
/// "the free play costs 0 memory" delta assertion with an unrelated
/// turn-boundary side effect. Digivolving from a Yellow Lv.5 base (cost 4)
/// exercises the identical shared clause body (`when: [on_play,
/// when_digivolving]`) while staying well within the memory range.
fn digivolve_and_get_free_play_prompt(traits: &[&str], level: u8) -> DebugRunner {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("parses")
        .add_card(yellow_lv5_base("YBASE"))
        .add_card(candidate("CAND", level, traits))
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "CAND")
        .unwrap();
    let next = runner.game.next_card_index();
    let cand_card = digimon_engine::card_source::CardSource::new(data_idx, 0, next);
    runner.game.players[0].hand.push(cand_card);

    let base = runner.place_on_field(0, "YBASE", Some(0));
    let hand_idx = runner
        .game
        .players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == CARD_ID)
        .expect("Minervamon in hand");
    assert!(
        runner.game.digivolve_from_hand(
            0,
            hand_idx,
            base.index as usize,
            PlaySource::ByHand,
        ),
        "P-207 must digivolve from a Yellow Lv.5 base for cost 4"
    );
    runner
}

#[test]
fn p_207_on_play_offers_avian_trait_candidate() {
    let runner = digivolve_and_get_free_play_prompt(&["Avian"], 4);
    let view = runner
        .pending_selection_view()
        .expect("Avian-trait level-4 candidate must be offered");
    assert_eq!(view.kind, SelectionKind::Hand);
    assert!(view.is_optional, "'you may' must allow PASS");
}

#[test]
fn p_207_on_play_offers_bird_trait_candidate() {
    let runner = digivolve_and_get_free_play_prompt(&["Bird"], 4);
    assert!(runner.pending_selection().is_some(), "Bird-trait candidate must be offered");
}

#[test]
fn p_207_on_play_offers_beast_trait_candidate() {
    let runner = digivolve_and_get_free_play_prompt(&["Beast"], 4);
    assert!(runner.pending_selection().is_some(), "Beast-trait candidate must be offered");
}

#[test]
fn p_207_on_play_offers_animal_trait_candidate() {
    let runner = digivolve_and_get_free_play_prompt(&["Animal"], 4);
    assert!(runner.pending_selection().is_some(), "Animal-trait candidate must be offered");
}

#[test]
fn p_207_on_play_offers_sovereign_trait_candidate() {
    let runner = digivolve_and_get_free_play_prompt(&["Sovereign"], 4);
    assert!(runner.pending_selection().is_some(), "Sovereign-trait candidate must be offered");
}

#[test]
fn p_207_on_play_offers_ts_trait_candidate_even_without_other_traits() {
    let runner = digivolve_and_get_free_play_prompt(&["TS"], 4);
    assert!(runner.pending_selection().is_some(), "TS-trait candidate must be offered");
}

#[test]
fn p_207_on_play_offers_level_4_or_lower_candidate() {
    // Level 1 candidate is still <= 4.
    let runner = digivolve_and_get_free_play_prompt(&["Beast"], 1);
    assert!(runner.pending_selection().is_some(), "level 1 Beast candidate must be offered");
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 4 — On Play / When Digivolving filter: negative branches
// ─────────────────────────────────────────────────────────────────────────────

/// NEGATIVE: [Sea Animal] substring-matches "Animal" but is explicitly
/// excluded by the printed "other than [Sea Animal]" carve-out.
#[test]
fn p_207_on_play_excludes_sea_animal_trait_candidate() {
    let runner = digivolve_and_get_free_play_prompt(&["Sea Animal"], 4);
    assert!(
        runner.pending_selection().is_none(),
        "[Sea Animal]-only candidate must NOT be offered — no other qualifying trait present"
    );
}

/// NEGATIVE: level 5 exceeds "level 4 or lower".
#[test]
fn p_207_on_play_excludes_level_5_candidate() {
    let runner = digivolve_and_get_free_play_prompt(&["Beast"], 5);
    assert!(
        runner.pending_selection().is_none(),
        "level-5 Beast candidate must NOT be offered — exceeds level 4 cap"
    );
}

/// NEGATIVE: a Digimon with none of the qualifying traits is not offered.
#[test]
fn p_207_on_play_excludes_non_matching_trait_candidate() {
    let runner = digivolve_and_get_free_play_prompt(&["Dragon"], 4);
    assert!(
        runner.pending_selection().is_none(),
        "non-matching-trait candidate must NOT be offered"
    );
}

/// NEGATIVE: with no eligible hand card at all, no prompt installs (empty
/// select_hand candidate set silently returns — matches DCGO's
/// HasMatchConditionOwnersHand outer gate).
#[test]
fn p_207_on_play_no_prompt_when_hand_empty_of_candidates() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("parses")
        .add_card(yellow_lv5_base("YBASE"))
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let base = runner.place_on_field(0, "YBASE", Some(0));
    assert!(
        runner
            .game
            .digivolve_from_hand(0, 0, base.index as usize, PlaySource::ByHand),
        "P-207 must digivolve from a Yellow Lv.5 base for cost 4"
    );
    assert!(
        runner.pending_selection().is_none(),
        "no eligible hand candidate — no free-play prompt installs"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 5 — On Play behavioral outcome (accept / decline)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn p_207_on_play_accepting_plays_the_candidate_free_and_unsuspended() {
    let mut runner = digivolve_and_get_free_play_prompt(&["Beast"], 4);
    let view = runner.pending_selection_view().expect("prompt installed");
    let action = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|a| *a != PASS)
        .expect("a non-PASS action must be offered");

    let memory_before = runner.memory();
    runner
        .execute_action(0, action)
        .expect("accept the free play");
    let _ = runner.auto_resolve();

    assert!(
        field_contains(&runner, 0, "CAND"),
        "the candidate must land on the battle area"
    );
    assert!(
        !hand_contains(&runner, 0, "CAND"),
        "the candidate must leave hand"
    );
    assert_eq!(
        runner.memory(),
        memory_before,
        "the play must be free — memory must not change"
    );

    let candidate_handle = runner
        .game
        .players[0]
        .battle_area
        .iter()
        .enumerate()
        .find(|(_, p)| p.top_card().card_id(&runner.game.card_data) == "CAND")
        .map(|(i, _)| digimon_engine::permanent::PermanentHandle {
            player: 0,
            index: i as u8,
        })
        .expect("candidate permanent handle");
    assert!(
        !runner.game.players[0].battle_area[candidate_handle.index as usize].is_suspended,
        "the free play must enter unsuspended (DCGO isTapped: false)"
    );
}

#[test]
fn p_207_on_play_declining_leaves_candidate_in_hand() {
    let mut runner = digivolve_and_get_free_play_prompt(&["Beast"], 4);
    assert!(runner.pending_is_optional(), "the pick must be declinable");

    runner.execute_action(0, PASS).expect("decline the free play");
    let _ = runner.auto_resolve();

    assert!(
        hand_contains(&runner, 0, "CAND"),
        "declining leaves the candidate in hand"
    );
    assert!(
        !field_contains(&runner, 0, "CAND"),
        "declining must not place the candidate on the field"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 6 — When Digivolving fires the shared body
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn p_207_when_digivolving_offers_the_same_free_play() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("parses")
        .add_card(yellow_lv5_base("BASE"))
        .add_card(candidate("CAND", 4, &["Beast"]))
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID, "CAND"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let base = runner.place_on_field(0, "BASE", Some(0));
    let hand_idx = runner
        .game
        .players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == CARD_ID)
        .expect("Minervamon in hand");

    assert!(
        runner
            .game
            .digivolve_from_hand(0, hand_idx, base.index as usize, PlaySource::ByHand),
        "P-207 must digivolve from a Yellow Lv.5 base for cost 4"
    );
    let _ = runner.auto_resolve();

    assert!(
        runner.pending_selection().is_some()
            || field_contains(&runner, 0, "CAND"),
        "When Digivolving must offer (or the auto_resolve must have already \
         accepted) the free-play prompt for the Beast candidate"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 6b — On Play (genuinely via `play_from_hand`, not digivolve)
// ─────────────────────────────────────────────────────────────────────────────
//
// P-207's own printed play cost (12) exceeds the engine's default memory
// pool cap of 10 (`Rules::memory_range == (-10, 10)`), so a legal same-turn
// play requires a WIDENED memory range (matching the established
// `bt13_007.rs::cost_reduction_runner` idiom — `with_rules` + a wider
// `memory_range`) rather than the default rules. This test exists
// specifically to confirm the [On Play] timing (distinct from [When
// Digivolving], tested above) fires the shared free-play body when
// Minervamon is played directly from hand.

#[test]
fn p_207_on_play_via_hand_play_offers_free_play_prompt() {
    let mut rules = digimon_engine::rules::Rules::standard();
    rules.memory_range = (-20, 20);

    let mut runner = DebugRunner::builder()
        .with_rules(rules)
        .dsl_card(CARD_ID)
        .expect("parses")
        .add_card(candidate("CAND", 4, &["Beast"]))
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID, "CAND"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(20)
        .start();

    let memory_before_play = runner.memory();
    runner.play(0, 0).expect("play Minervamon");
    assert_eq!(
        runner.memory(),
        memory_before_play - 12,
        "Minervamon's own play cost (12) must be paid when playing from hand"
    );

    let view = runner
        .pending_selection_view()
        .expect("[On Play] must offer the Beast candidate free-play prompt");
    assert_eq!(view.kind, SelectionKind::Hand);
    assert!(view.is_optional, "'you may' must allow PASS");

    let memory_before_free_play = runner.memory();
    let action = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|a| *a != PASS)
        .expect("a non-PASS action must be offered");
    runner
        .execute_action(0, action)
        .expect("accept the free play");
    let _ = runner.auto_resolve();

    assert!(
        field_contains(&runner, 0, "CAND"),
        "the candidate must land on the battle area via [On Play]"
    );
    assert_eq!(
        runner.memory(),
        memory_before_free_play,
        "the free play itself must not cost additional memory"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 7 — When Attacking [OPT]: filter positive/negative + behavior
// ─────────────────────────────────────────────────────────────────────────────

fn attack_and_get_trash_prompt(traits: &[&str], level: u8) -> DebugRunner {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("parses")
        .add_card(candidate("TCAND", level, traits))
        .add_card(filler("FILL"))
        .hand(0, &[])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .security(1, &["FILL"])
        .memory(12)
        .start();
    // Seed the trash directly (avoids needing a full discard flow).
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "TCAND")
        .unwrap();
    let next = runner.game.next_card_index();
    runner.game.players[0]
        .trash
        .push(digimon_engine::card_source::CardSource::new(data_idx, 0, next));

    let mine = runner.place_on_field(0, CARD_ID, Some(0));
    runner.attack_player(mine, 1, false);
    runner
}

#[test]
fn p_207_when_attacking_offers_trash_candidate() {
    let runner = attack_and_get_trash_prompt(&["Beast"], 4);
    let view = runner
        .pending_selection_view()
        .expect("trash free-play prompt must install on attack");
    assert_eq!(view.kind, SelectionKind::Trash);
    assert!(view.is_optional, "'you may' must allow PASS");
}

#[test]
fn p_207_when_attacking_excludes_sea_animal_trash_candidate() {
    let runner = attack_and_get_trash_prompt(&["Sea Animal"], 4);
    assert!(
        runner.pending_selection().is_none(),
        "[Sea Animal]-only trash candidate must NOT be offered"
    );
}

#[test]
fn p_207_when_attacking_excludes_level_5_trash_candidate() {
    let runner = attack_and_get_trash_prompt(&["Beast"], 5);
    assert!(
        runner.pending_selection().is_none(),
        "level-5 trash candidate must NOT be offered — exceeds level 4 cap"
    );
}

#[test]
fn p_207_when_attacking_accepting_plays_from_trash_free_and_unsuspended() {
    let mut runner = attack_and_get_trash_prompt(&["Beast"], 4);
    let view = runner.pending_selection_view().expect("prompt installed");
    let action = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|a| *a != PASS)
        .expect("a non-PASS action must be offered");

    let memory_before = runner.memory();
    runner
        .execute_action(0, action)
        .expect("accept the free play from trash");
    let _ = runner.auto_resolve();

    assert!(
        field_contains(&runner, 0, "TCAND"),
        "the candidate must land on the battle area from trash"
    );
    assert!(
        !trash_contains(&runner, 0, "TCAND"),
        "the candidate must leave trash"
    );
    assert_eq!(
        runner.memory(),
        memory_before,
        "the play must be free — memory must not change"
    );
}

#[test]
fn p_207_when_attacking_declining_leaves_candidate_in_trash() {
    let mut runner = attack_and_get_trash_prompt(&["Beast"], 4);
    runner
        .execute_action(0, PASS)
        .expect("decline the free play from trash");
    let _ = runner.auto_resolve();

    assert!(
        trash_contains(&runner, 0, "TCAND"),
        "declining leaves the candidate in trash"
    );
    assert!(
        !field_contains(&runner, 0, "TCAND"),
        "declining must not place the candidate on the field"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 8 — OPT lockout (When Attacking clause)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn p_207_when_attacking_opt_blocks_second_attack_same_turn() {
    let mut runner = attack_and_get_trash_prompt(&["Beast"], 4);
    // Decline the first offer to keep state simple (still counts as an
    // activation per DCGO — the OPT counter increments on trigger firing,
    // not on accepting the pick; the trigger fired and installed a prompt).
    runner
        .execute_action(0, PASS)
        .expect("decline the first free play");
    let _ = runner.auto_resolve();

    // Second attack in the same turn: manually unsuspend the attacker first
    // so `can_attack` doesn't reject the declaration outright (that would
    // give false OPT coverage — see the bt7_032 idiom).
    let mine = runner.perm_handle(0, 0);
    runner.game.players[0].battle_area[mine.index as usize].is_suspended = false;

    // A second defender security card so the attack can resolve.
    let fill_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "FILL")
        .unwrap();
    let next = runner.game.next_card_index();
    let security_card = digimon_engine::card_source::CardSource::new(fill_idx, 1, next);
    runner.game.players[1].security.push(security_card);

    runner.attack_player(mine, 1, false);

    assert!(
        runner.pending_selection().is_none(),
        "OPT must lock out the second When Attacking activation in the same turn"
    );
}

#[test]
fn p_207_when_attacking_opt_clears_after_end_turn() {
    let mut runner = attack_and_get_trash_prompt(&["Beast"], 4);
    runner
        .execute_action(0, PASS)
        .expect("decline the first free play");
    let _ = runner.auto_resolve();

    // Rotate through a full turn cycle back to player 0.
    runner.end_turn();
    runner.end_turn();

    // Re-seed a fresh trash candidate + security for the new attack.
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "TCAND")
        .unwrap();
    let next = runner.game.next_card_index();
    runner.game.players[0]
        .trash
        .push(digimon_engine::card_source::CardSource::new(data_idx, 0, next));
    let fill_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "FILL")
        .unwrap();
    let next2 = runner.game.next_card_index();
    let security_card = digimon_engine::card_source::CardSource::new(fill_idx, 1, next2);
    runner.game.players[1].security.push(security_card);

    let mine = runner.perm_handle(0, 0);
    runner.game.players[0].battle_area[mine.index as usize].is_suspended = false;
    runner.attack_player(mine, 1, false);

    assert!(
        runner.pending_selection().is_some(),
        "OPT lockout must clear after end_turn — the When Attacking clause fires again"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 9 — Alt-digivolve behavioral: Purple standard circle
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn p_207_digivolves_from_purple_lv5_for_cost_4() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("parses")
        .add_card(purple_lv5_base("PBASE"))
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(4)
        .start();

    let base = runner.place_on_field(0, "PBASE", Some(0));
    let memory_before = runner.memory();
    assert!(
        runner
            .game
            .digivolve_from_hand(0, 0, base.index as usize, PlaySource::ByHand),
        "P-207 must digivolve from a Purple Lv.5 base for cost 4"
    );
    assert_eq!(
        runner.memory(),
        memory_before - 4,
        "the Purple standard circle must cost 4 memory"
    );
}

#[test]
fn p_207_digivolves_from_yellow_lv5_for_cost_4() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("parses")
        .add_card(yellow_lv5_base("YBASE"))
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(4)
        .start();

    let base = runner.place_on_field(0, "YBASE", Some(0));
    let memory_before = runner.memory();
    assert!(
        runner
            .game
            .digivolve_from_hand(0, 0, base.index as usize, PlaySource::ByHand),
        "P-207 must digivolve from a Yellow Lv.5 base for cost 4 (printed evo_costs)"
    );
    assert_eq!(runner.memory(), memory_before - 4);
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 10 — Alt-digivolve behavioral: Beastkin / TS special circle
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn p_207_digivolves_from_beastkin_lv5_for_cost_3() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("parses")
        .add_card(beastkin_lv5_base("BKBASE"))
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(3)
        .start();

    let base = runner.place_on_field(0, "BKBASE", Some(0));
    let memory_before = runner.memory();
    assert!(
        runner
            .game
            .digivolve_from_hand(0, 0, base.index as usize, PlaySource::ByHand),
        "P-207 must digivolve from an exact-trait [Beastkin] Lv.5 base for cost 3"
    );
    assert_eq!(runner.memory(), memory_before - 3);
}

#[test]
fn p_207_digivolves_from_ts_lv5_for_cost_3() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("parses")
        .add_card(ts_lv5_base("TSBASE"))
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(3)
        .start();

    let base = runner.place_on_field(0, "TSBASE", Some(0));
    let memory_before = runner.memory();
    assert!(
        runner
            .game
            .digivolve_from_hand(0, 0, base.index as usize, PlaySource::ByHand),
        "P-207 must digivolve from an exact-trait [TS] Lv.5 base for cost 3"
    );
    assert_eq!(runner.memory(), memory_before - 3);
}

/// NEGATIVE: a Lv.5 base with neither Beastkin nor TS nor a standard colour
/// (Yellow/Purple) must be rejected by every alt-path.
#[test]
fn p_207_does_not_digivolve_from_plain_lv5_base() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("parses")
        .add_card(plain_lv5_base("PLAINBASE"))
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(4)
        .start();

    let base = runner.place_on_field(0, "PLAINBASE", Some(0));
    assert!(
        !runner
            .game
            .digivolve_from_hand(0, 0, base.index as usize, PlaySource::ByHand),
        "P-207 must NOT digivolve from a plain Red Lv.5 base (no matching circle)"
    );
}

/// NEGATIVE: [Beastkin] must be an EXACT trait match, not substring — a
/// trait like "Beastkin Warrior" (containing "Beastkin" as a substring but
/// not equal to it) must NOT satisfy the special digivolve gate. DCGO uses
/// `EqualsTraits`, not `ContainsTraits`, for this alt-path.
#[test]
fn p_207_beastkin_alt_path_requires_exact_trait_not_substring() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("parses")
        .add_card(candidate("SUBSTR-BASE", 5, &["Beastkin Warrior"]))
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(4)
        .start();

    let base = runner.place_on_field(0, "SUBSTR-BASE", Some(0));
    assert!(
        !runner
            .game
            .digivolve_from_hand(0, 0, base.index as usize, PlaySource::ByHand),
        "a 'Beastkin Warrior'-trait base must NOT satisfy the exact-trait \
         [Beastkin] alt-path (DCGO EqualsTraits, not ContainsTraits)"
    );
}
