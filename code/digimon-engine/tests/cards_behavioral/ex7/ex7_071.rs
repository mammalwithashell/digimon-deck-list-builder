//! EX7-071 Hurricane Screw Shot — Option, Purple, Use cost 6.
//! Traits: Three Musketeers.
//!
//! # Card text (official Bandai DB — data/card_bundles/EX7-071.md, verbatim;
//! confirmed against the card image)
//!
//! ```text
//! When effects trash this card from digivolution cards, gain 1 memory.
//! While you have a [Three Musketeers] trait Digimon, you may ignore this
//! card's color requirements.
//! [Main] Delete 1 of your opponent's level 3, 1 of their level 4, and 1 of
//! their level 5 Digimon. Then, place this card as the bottom digivolution
//! card of 1 of your [Three Musketeers] trait Digimon.
//! ```
//!
//! Security Effect:
//! ```text
//! [Security] Delete 1 of your opponent's level 3, 1 of their level 4, and 1
//! of their level 5 Digimon.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX7/Purple/EX7_071.cs
//!
//! - OnDigivolutionCardDiscarded (inherited): CanUse =
//!   CanTriggerOnTrashSelfDigivolutionCard; CanActivate = IsExistOnTrash(card)
//!   && CanAddMemory -> AddMemory(1).
//! - None timing: IgnoreColorConditionClass gated on >=1 Three Musketeers
//!   Digimon on the battle area, restricted to this card only — functionally
//!   the `use_requirement` idiom (ST23-15 sibling).
//! - OptionSkill ([Main]): three INDEPENDENT sequential SelectPermanentEffect
//!   calls (level 3 / level 4 / level 5 opponent Digimon), each self-skipping
//!   via HasMatchConditionPermanent when no candidate exists (EX4-073
//!   precedent: "each arm independently no-ops if no legal target exists").
//!   All three accumulate into one batch Destroy. THEN a
//!   CanSelectPermanentCondition select over the controller's own [Three
//!   Musketeers] Digimon (self-skips on 0 candidates) ->
//!   AddDigivolutionCardsBottom (face-up, NOT face down).
//! - SecuritySkill ([Security]): the SAME triple independent delete, batch
//!   destroyed, with NO trailing self-placement step.
//!
//! # Self-placement tail — G-OPTION-PLACE-SELF-UNDER-PERMANENT-DSL (✅ RESOLVED 2026-07-03)
//!
//! The printed [Main] tail "Then, place this card as the bottom digivolution
//! card of 1 of your [Three Musketeers] trait Digimon" is authored via the
//! `place_self_under_permanent` DSL step, which lowers to the purpose-built
//! engine primitive (`EffectContext::place_self_under_permanent`,
//! `code/digimon-engine/src/effect_context/action/lifecycle.rs` — its doc
//! comment cites "P-180 / EX7-070 / EX7-071"). The primitive claims the
//! in-flight `pending_option`, so the resolved Option is seated FACE-UP under
//! the chosen Digimon instead of trashed; with no [Three Musketeers] Digimon
//! the select self-skips and the Option disposes to trash normally.
//! DSL-level substrate proof: `tests/dsl/option_lifecycle_cluster.rs`
//! (gap1_* + gap1_dsl_*). See the resolution note in
//! `code/digimon-engine/cards/ex7/EX7-071.yaml`.
//!
//! # Patterns this test covers
//! - D3 Color ignore / bypass (`use_requirement`)
//! - Triple independent level-filtered opponent delete, each self-skipping
//!   (EX4-073 "each arm independently no-ops" idiom)
//! - Inherited on_digivolution_card_trashed -> gain 1 memory (no host-trait
//!   restriction, unlike the Mineral/Rock LIBERATOR sibling idiom)
//! - [Main] vs [Security] divergence: Security omits the self-placement tail

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledScope, CompiledTiming, CompiledTriggeredClause,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, PlayerId};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "EX7-071";

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn opp_digimon_lvl(id: &str, level: u8, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = 0;
    c
}

/// A [Three Musketeers] trait Digimon — the placement target for the [Main]
/// self-placement tail, and the color-ignore enabler for `use_requirement`.
fn tm_digimon(id: &str, level: u8, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = 0;
    c.traits = vec!["Three Musketeers".to_string()];
    c
}

/// A plain (non-[Three Musketeers]) own Digimon — must never be offered as
/// the self-placement target. Colored PURPLE (matching EX7-071's printed
/// color) so the Option's normal color requirement is satisfied even without
/// a [Three Musketeers] Digimon in play — isolating the "no TM Digimon"
/// condition from an unrelated color-requirement failure.
fn plain_digimon(id: &str, level: u8, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = 0;
    c.colors = vec![CardColor::Purple];
    c.traits = vec!["Beast".to_string()];
    c
}

fn base_builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX7-071 YAML loads from the embedded pack")
        .add_card(opp_digimon_lvl("OPP-3", 3, 2000))
        .add_card(opp_digimon_lvl("OPP-4", 4, 4000))
        .add_card(opp_digimon_lvl("OPP-5", 5, 6000))
        .add_card(tm_digimon("TM-OWN", 5, 6000))
        .add_card(plain_digimon("PLAIN-OWN", 5, 6000))
        .add_card(make_test_card("FILL", "Filler"))
}

/// Auto-resolve all remaining prompts by always taking the first legal
/// action — used once a specific branch decision is already locked in, or
/// when the test only cares about end-state aggregates.
fn drain_prompts(runner: &mut DebugRunner) {
    while let Some(v) = runner.pending_selection_view() {
        let act = v
            .valid_action_ids
            .first()
            .copied()
            .expect("a resolvable action");
        runner.execute_action(v.selecting_player, act).ok();
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 1 — Structural assertions
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn ex7_071_compiles_with_printed_metadata() {
    let runner = base_builder().start();
    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("EX7-071 in compiled cards");

    assert_eq!(compiled.card, CARD_ID);
    assert_eq!(compiled.name, "Hurricane Screw Shot");
    assert_eq!(compiled.cost, Some(6));
    assert!(
        compiled.traits.iter().any(|t| t == "Three Musketeers"),
        "printed [Three Musketeers] trait missing"
    );
}

#[test]
fn ex7_071_has_use_requirement_for_three_musketeers() {
    let runner = base_builder().start();
    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("EX7-071 in compiled cards");
    assert!(
        compiled.use_requirement.is_some(),
        "EX7-071 must carry a use_requirement (color-ignore while a [Three \
         Musketeers] Digimon is in play)"
    );
}

#[test]
fn ex7_071_has_main_inherited_and_security_clauses() {
    let runner = base_builder().start();
    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("EX7-071 in compiled cards");

    let triggered: Vec<&CompiledTriggeredClause> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    let has_main = triggered
        .iter()
        .any(|t| t.when.contains(&CompiledTiming::MainFromHand));
    let has_inherited = triggered.iter().any(|t| {
        t.scope == CompiledScope::Inherited
            && t.when.contains(&CompiledTiming::OnDigivolutionCardTrashed)
    });
    let has_security = triggered
        .iter()
        .any(|t| t.when.contains(&CompiledTiming::OnSecurity));

    assert!(has_main, "EX7-071 must carry a [Main] (main_from_hand) clause");
    assert!(
        has_inherited,
        "EX7-071 must carry an inherited on_digivolution_card_trashed clause"
    );
    assert!(has_security, "EX7-071 must carry an on_security clause");
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 2 — [Main]: triple level-filtered delete
// ─────────────────────────────────────────────────────────────────────────────

fn main_builder() -> DebugRunnerBuilder {
    base_builder()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(10)
}

/// POSITIVE: with one opponent Digimon at each of level 3/4/5, playing the
/// [Main] deletes all three.
#[test]
fn ex7_071_main_deletes_all_three_levels() {
    let mut runner = main_builder().start();
    runner.place_on_field(0, "TM-OWN", Some(0));
    let opp3 = runner.place_on_field(1, "OPP-3", Some(0));
    let opp4 = runner.place_on_field(1, "OPP-4", Some(0));
    let opp5 = runner.place_on_field(1, "OPP-5", Some(0));

    runner.game.play_option_from_hand(0, 0);
    drain_prompts(&mut runner);

    assert_eq!(
        runner.battle_area_size(1),
        0,
        "all three opponent Digimon (level 3/4/5) must be deleted"
    );
}

/// After the triple delete, [Main] places the resolved Option as the bottom
/// digivolution card of a chosen [Three Musketeers] Digimon (the
/// `place_self_under_permanent` DSL step — G-OPTION-PLACE-SELF-UNDER-PERMANENT-DSL,
/// resolved 2026-07-03). The Option must NOT be trashed.
#[test]
fn ex7_071_main_self_places_as_bottom_source_of_tm_digimon() {
    let mut runner = main_builder().start();
    runner.place_on_field(0, "TM-OWN", Some(0));
    runner.place_on_field(1, "OPP-3", Some(0));
    runner.place_on_field(1, "OPP-4", Some(0));
    runner.place_on_field(1, "OPP-5", Some(0));

    runner.game.play_option_from_hand(0, 0);
    drain_prompts(&mut runner);

    let still_in_trash = runner.game.players[0]
        .trash
        .iter()
        .any(|c| c.card_id(&runner.game.card_data) == CARD_ID);
    assert!(
        !still_in_trash,
        "EX7-071 must be placed as a bottom digivolution source, not trashed"
    );
    let tm_own = runner.game.players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&runner.game.card_data) == "TM-OWN")
        .expect("TM-OWN still on field");
    assert!(
        tm_own
            .card_sources
            .iter()
            .any(|s| s.card_id(&runner.game.card_data) == CARD_ID),
        "EX7-071 must be placed under TM-OWN's digivolution stack"
    );
}

/// PARTIAL FIELD: only a level 4 opponent Digimon exists (no level 3 or
/// level 5). The level 3 and level 5 arms self-skip; only the level 4
/// Digimon is deleted.
#[test]
fn ex7_071_main_partial_field_only_deletes_matching_levels() {
    let mut runner = main_builder().start();
    runner.place_on_field(0, "TM-OWN", Some(0));
    runner.place_on_field(1, "OPP-4", Some(0));

    runner.game.play_option_from_hand(0, 0);
    drain_prompts(&mut runner);

    assert_eq!(
        runner.battle_area_size(1),
        0,
        "the sole level 4 opponent Digimon must be deleted; the level 3/5 \
         arms must self-skip without panicking"
    );
}

/// NEGATIVE: no opponent Digimon at all. All three delete arms self-skip;
/// the game must not panic and no deletion happens.
#[test]
fn ex7_071_main_no_opponent_digimon_all_arms_self_skip() {
    let mut runner = main_builder().start();
    runner.place_on_field(0, "TM-OWN", Some(0));

    runner.game.play_option_from_hand(0, 0);
    drain_prompts(&mut runner);

    assert_eq!(
        runner.battle_area_size(1),
        0,
        "opponent had no Digimon to begin with; still zero after resolution"
    );
}

/// Only a level 3 opponent Digimon exists — verifies the level 3 arm alone
/// fires and the level 4 / level 5 arms self-skip (mirror of the level-4
/// partial-field test, pinned on the FIRST arm specifically).
#[test]
fn ex7_071_main_only_level3_present_deletes_only_level3() {
    let mut runner = main_builder().start();
    runner.place_on_field(0, "TM-OWN", Some(0));
    runner.place_on_field(1, "OPP-3", Some(0));

    runner.game.play_option_from_hand(0, 0);
    drain_prompts(&mut runner);

    assert_eq!(
        runner.battle_area_size(1),
        0,
        "the sole level 3 opponent Digimon must be deleted by the first arm"
    );
}

/// Only a level 5 opponent Digimon exists — verifies the LAST arm alone
/// fires (level 3 / level 4 arms self-skip first, in sequence).
#[test]
fn ex7_071_main_only_level5_present_deletes_only_level5() {
    let mut runner = main_builder().start();
    runner.place_on_field(0, "TM-OWN", Some(0));
    runner.place_on_field(1, "OPP-5", Some(0));

    runner.game.play_option_from_hand(0, 0);
    drain_prompts(&mut runner);

    assert_eq!(
        runner.battle_area_size(1),
        0,
        "the sole level 5 opponent Digimon must be deleted by the third arm"
    );
}

/// Multiple candidates at the SAME level: the select prompt must offer both,
/// but only ONE is deleted per arm (the printed text is "1 of their level 3
/// ... Digimon", singular).
#[test]
fn ex7_071_main_multiple_same_level_candidates_offers_choice_deletes_only_one() {
    let mut runner = main_builder()
        .add_card(opp_digimon_lvl("OPP-3B", 3, 3000))
        .start();
    runner.place_on_field(0, "TM-OWN", Some(0));
    let opp3a = runner.place_on_field(1, "OPP-3", Some(0));
    let opp3b = runner.place_on_field(1, "OPP-3B", Some(0));

    runner.game.play_option_from_hand(0, 0);

    let view = runner
        .pending_selection_view()
        .expect("the level 3 arm must offer a choice between OPP-3 and OPP-3B");
    let candidate_count = view
        .valid_action_ids
        .iter()
        .filter(|&&a| a != PASS)
        .count();
    assert_eq!(
        candidate_count, 2,
        "both level 3 Digimon must be legal candidates for the single pick"
    );

    drain_prompts(&mut runner);

    assert_eq!(
        runner.battle_area_size(1),
        1,
        "only ONE of the two level 3 Digimon is deleted; the other survives"
    );
}

/// A level 6 opponent Digimon (out of range) must never be a candidate for
/// any of the three arms and must survive.
#[test]
fn ex7_071_main_level6_digimon_is_never_a_candidate() {
    let mut runner = main_builder()
        .add_card(opp_digimon_lvl("OPP-6", 6, 8000))
        .start();
    runner.place_on_field(0, "TM-OWN", Some(0));
    runner.place_on_field(1, "OPP-6", Some(0));
    runner.place_on_field(1, "OPP-3", Some(0));

    runner.game.play_option_from_hand(0, 0);
    drain_prompts(&mut runner);

    assert_eq!(
        runner.battle_area_size(1),
        1,
        "OPP-6 (level 6, out of range for all three arms) must survive"
    );
    let survivor_id = runner.game.players[1].battle_area[0]
        .top_card()
        .card_id(&runner.game.card_data)
        .to_string();
    assert_eq!(survivor_id, "OPP-6", "the surviving Digimon must be OPP-6");
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 3 — [Main] self-placement tail
// ─────────────────────────────────────────────────────────────────────────────

/// With no [Three Musketeers] trait Digimon of the controller's own, the
/// mandatory placement select self-skips (zero candidates — DCGO's silent
/// skip), the place step no-ops on the unset binding, and the resolved
/// Option disposes to trash like a normal Standard Option. Same end state as
/// before the tail was authored, but now for the RIGHT reason (a real
/// candidate check self-skipping, not an unauthored clause).
#[test]
fn ex7_071_main_no_tm_digimon_falls_back_to_trash() {
    let mut runner = main_builder().start();
    runner.place_on_field(0, "PLAIN-OWN", Some(0));
    runner.place_on_field(1, "OPP-3", Some(0));

    runner.game.play_option_from_hand(0, 0);
    drain_prompts(&mut runner);

    let in_trash = runner.game.players[0]
        .trash
        .iter()
        .any(|c| c.card_id(&runner.game.card_data) == CARD_ID);
    assert!(
        in_trash,
        "with no [Three Musketeers] candidate the placement select \
         self-skips and the resolved Option goes to trash via the engine's \
         default Standard dispose"
    );
    let plain_own = runner.game.players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&runner.game.card_data) == "PLAIN-OWN")
        .expect("PLAIN-OWN still on field");
    assert!(
        !plain_own
            .card_sources
            .iter()
            .any(|s| s.card_id(&runner.game.card_data) == CARD_ID),
        "PLAIN-OWN (no [Three Musketeers] trait) must never receive the \
         placed card"
    );
}

/// A plain (non-[Three Musketeers]) own Digimon must never be offered as a
/// candidate for the self-placement pick, even when a TM Digimon coexists
/// (the select filter is `{ kind: digimon, trait_has: "Three Musketeers" }`).
#[test]
fn ex7_071_main_self_placement_only_offers_tm_trait_digimon() {
    let mut runner = main_builder().start();
    runner.place_on_field(0, "TM-OWN", Some(0));
    runner.place_on_field(0, "PLAIN-OWN", Some(0));
    // No opponent Digimon -> all three delete arms self-skip immediately, so
    // the very next pending selection must be the self-placement pick.
    runner.game.play_option_from_hand(0, 0);

    let view = runner
        .pending_selection_view()
        .expect("the self-placement pick must install (TM-OWN exists)");
    let candidate_count = view
        .valid_action_ids
        .iter()
        .filter(|&&a| a != PASS)
        .count();
    assert_eq!(
        candidate_count, 1,
        "only TM-OWN (the [Three Musketeers] Digimon) may be offered; \
         PLAIN-OWN must be excluded"
    );

    drain_prompts(&mut runner);

    let plain_own = runner.game.players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&runner.game.card_data) == "PLAIN-OWN")
        .expect("PLAIN-OWN still on field");
    assert!(
        !plain_own
            .card_sources
            .iter()
            .any(|s| s.card_id(&runner.game.card_data) == CARD_ID),
        "PLAIN-OWN must never receive the placed card"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 4 — Inherited: gain 1 memory when trashed from digivolution cards
// ─────────────────────────────────────────────────────────────────────────────

/// When an effect trashes EX7-071 from a Digimon's digivolution cards (any
/// own Digimon's stack — no host-trait restriction on this card), the
/// controller gains 1 memory.
#[test]
fn ex7_071_gains_memory_when_trashed_from_digivolution_cards() {
    // A trasher fixture: On Play, trash 1 of the controller's own
    // digivolution sources (any source, no filter) — mirrors the
    // `select_own_sources` cost idiom used across the pool (EX10-025 etc.).
    const TRASHER_YAML: &str = r#"
card: DSL-EX7-071-TRASHER
name: Sources Trasher
kind: digimon
level: 4
color: [purple]
cost: 0
dp: 3000
effects:
  - when: on_play
    summary: "[On Play] Trash 1 of your own digivolution sources"
    process:
      - select_own_sources:
          min: 1
          max: 1
          bind_as: cost_sources
          prompt: "Trash 1 digivolution source"
          then:
            - trash_selected_sources: { source_refs: cost_sources }
"#;
    let mut runner = base_builder()
        .from_dsl_yaml(TRASHER_YAML)
        .expect("inline trasher fixture compiles")
        .memory(0)
        .start();

    // Stack EX7-071 as a digivolution source under a trasher Digimon, then
    // play a second copy of the trasher from hand to fire the trash cost —
    // simpler: place the trasher stack directly with EX7-071 buried, then
    // fire OnPlay manually is unavailable for a stacked (not-just-played)
    // permanent, so instead trigger via playing the trasher fresh and
    // burying EX7-071 as its OWN pre-existing source via place_stack.
    let stack = runner.place_stack(0, &["EX7-071", "DSL-EX7-071-TRASHER"]);

    let memory_before = runner.memory();
    // Fire the trasher's OnPlay batch manually (it is already on the field
    // via place_stack, not played) to trigger the source-trash cost.
    runner.fire_on_play(0, stack.index as usize);
    drain_prompts(&mut runner);

    assert_eq!(
        runner.memory(),
        memory_before + 1,
        "trashing EX7-071 from a Digimon's digivolution cards must gain 1 memory"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 5 — [Security]: same triple delete, no self-placement
// ─────────────────────────────────────────────────────────────────────────────

fn security_builder() -> DebugRunnerBuilder {
    base_builder()
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(0)
}

/// The [Security] effect deletes 1 opponent level 3, 1 level 4, and 1 level
/// 5 Digimon — same triple delete as [Main] — when EX7-071 is checked as a
/// security card during an attack.
#[test]
fn ex7_071_security_deletes_all_three_levels() {
    let mut runner = security_builder().security(1, &[CARD_ID]).start();
    let attacker = runner.place_on_field(0, "OPP-3", Some(0));
    // Reuse OPP-4/OPP-5 as the ATTACKER's (P0's) own board is irrelevant here
    // — the [Security] effect's "opponent" is from P1's perspective (P0).
    let opp4 = runner.place_on_field(0, "OPP-4", Some(0));
    let opp5 = runner.place_on_field(0, "OPP-5", Some(0));

    let _ = runner.attack_player(attacker, 1, false);
    runner.auto_resolve().ok();

    assert_eq!(
        runner.battle_area_size(0),
        0,
        "all three of the attacker's (P1's opponent's) Digimon at level 3/4/5 \
         must be deleted by the [Security] effect"
    );
}

/// The [Security] effect must NOT place the card as a digivolution source —
/// after resolving as a security card it follows the normal security-card
/// resolution path (trashed), never buried under a Three Musketeers Digimon.
#[test]
fn ex7_071_security_does_not_self_place_as_digivolution_source() {
    let mut runner = security_builder().security(1, &[CARD_ID]).start();
    // Give the defender (P1, EX7-071's owner) a Three Musketeers Digimon —
    // if the Main-effect placement tail were mistakenly reused here, this
    // Digimon would become a false-positive placement target.
    runner.place_on_field(1, "TM-OWN", Some(0));
    let attacker = runner.place_on_field(0, "OPP-3", Some(0));

    let _ = runner.attack_player(attacker, 1, false);
    runner.auto_resolve().ok();

    let tm_own = runner.game.players[1]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&runner.game.card_data) == "TM-OWN")
        .expect("TM-OWN still on P1's field");
    assert!(
        !tm_own
            .card_sources
            .iter()
            .any(|s| s.card_id(&runner.game.card_data) == CARD_ID),
        "[Security] must never place EX7-071 as a digivolution source — \
         that tail is [Main]-only"
    );
}

/// NEGATIVE: no matching opponent Digimon at all — the [Security] triple
/// delete self-skips on every arm without panicking.
#[test]
fn ex7_071_security_no_matching_digimon_self_skips() {
    let mut runner = security_builder()
        .add_card(opp_digimon_lvl("OPP-6", 6, 8000))
        .security(1, &[CARD_ID])
        .start();
    // Level 6 is out of range for all three level 3/4/5 arms.
    let attacker = runner.place_on_field(0, "OPP-6", Some(0));

    let _ = runner.attack_player(attacker, 1, false);
    runner.auto_resolve().ok();

    assert_eq!(
        runner.battle_area_size(0),
        1,
        "OPP-6 (level 6) never matches any of the three level-filtered arms"
    );
}
