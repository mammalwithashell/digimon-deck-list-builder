//! EX8-022 Frigimon — Digimon, Lv.4, Blue/Yellow, DP 5000, Cost 5.
//! Traits: Ice-Snow, LIBERATOR. Attribute: Vaccine. Rarity: R.
//! Standard digivolution: Lv.3 blue for 3 memory (printed evo box).
//! Alt digivolution (printed evo box): "Digivolve Lv.3 w/[Ice-Snow] trait: Cost 2".
//!
//! # Card text (card image EX8-022 — authoritative)
//!
//! **Effect:**
//! <Iceclad> (Other than against Security Digimon, compare the number of
//! digivolution cards instead of DP in this Digimon's battles.)
//! [On Play] [When Digivolving] Trash the bottom 2 digivolution cards of 1 of
//! your opponent's Digimon. Then, if your opponent has no Digimon with
//! digivolution cards, gain 1 memory.
//!
//! **Inherited:**
//! [When Attacking] Give 1 of your opponent's Digimon <Security A. -1>
//! (This Digimon checks 1 fewer security card.) until the end of their turn.
//!
//! NOTE — cards.json's inherited reminder text says "checks 1 additional
//! security card", contradicting the <Security A. -1> badge. The card image
//! badge clearly reads "Security A. -1" and DCGO resolves it as
//! ChangeDigimonSAttack(changeValue: -1) — Security Attack MINUS 1 ("checks
//! 1 FEWER"). Image + DCGO win per CLAUDE.md source priority.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX8/Blue/EX8_022.cs
//!   - Alternate Digivolution Requirement (None):
//!     AddSelfDigivolutionRequirementStaticEffect(permanentCondition:
//!     EqualsTraits("Ice-Snow") && Level == 3, digivolutionCost: 2).
//!   - Iceclad (None): IcecladSelfStaticEffect(isInheritedEffect: false).
//!   - On Play + When Digivolving (OnEnterFieldAnyone, two ActivateClass):
//!     if ANY opponent battle-area Digimon exists (no sources requirement!),
//!     mandatory SelectPermanentEffect(maxCount: 1, canNoSelect: false) →
//!     TrashDigivolutionCardsFromTopOrBottom(trashCount: 2, isFromTop: false).
//!     THEN (unconditionally — outside the target-exists guard): if the
//!     opponent has NO Digimon with DigivolutionCards, AddMemory(1).
//!   - When Attacking - ESS (OnAllyAttack, isInheritedEffect: true):
//!     mandatory single pick over opponent battle-area Digimon →
//!     ChangeDigimonSAttack(changeValue: -1, UntilOpponentTurnEnd).
//!
//! # Patterns this test covers
//! - Iceclad combat semantics (cf. tests/keyword_phase_f/iceclad.rs):
//!   stack-count compare replaces DP compare in Digimon-vs-Digimon battles.
//! - [On Play][When Digivolving] trash_bottom_sources(2) + conditional
//!   gain_memory (BT25-026 trash-bottom shape + DCGO's unconditional
//!   memory-check leg).
//! - Inherited [When Attacking] mandatory single-target SecurityAttackChange
//!   -1 with end_of_opponents_turn expiry (copied from EX8-019).

#![allow(dead_code)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledCardKind, CompiledClause, CompiledColor, CompiledCost,
    CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Keyword, ModifierType, PlaySource};
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "EX8-022";

// ─── Fixture builders ─────────────────────────────────────────────────────────

/// A plain Lv.4 opponent Digimon. DP parameterized so the Iceclad battle tests
/// can prove DP is NOT the comparator.
fn opp_digimon(id: &str, dp: i32) -> CardData {
    let mut c = make_test_card(id, "Opp Digimon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(dp);
    c.play_cost = 4;
    c.colors = vec![CardColor::Red];
    c
}

/// A generic digivolution-source filler card (stacked under tops).
fn source_filler(id: &str) -> CardData {
    let mut c = make_test_card(id, "Source Filler");
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(1000);
    c.play_cost = 3;
    c.colors = vec![CardColor::Red];
    c
}

/// A RED Lv.3 Digimon with the [Ice-Snow] trait — matches ONLY Frigimon's
/// printed alt evo box ("Lv.3 w/[Ice-Snow] trait: Cost 2"), not the standard
/// Lv.3-blue path, so digivolving over it proves the alt path works and
/// charges its printed cost 2.
fn red_ice_snow_lv3(id: &str) -> CardData {
    let mut c = make_test_card(id, "Red IceSnow Lv3");
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(2000);
    c.play_cost = 3;
    c.colors = vec![CardColor::Red];
    c.traits = vec!["Ice-Snow".to_string()];
    c
}

/// A carrier Digimon for player 0 — EX8-022 sits underneath it as a
/// digivolution source so its inherited [When Attacking] effect fires.
fn carrier(id: &str) -> CardData {
    let mut c = make_test_card(id, "Carrier");
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(7000);
    c.play_cost = 7;
    c.colors = vec![CardColor::Blue];
    c
}

/// An opponent (player 1) attacker used to observe the Security A. -1
/// behaviorally: with the -1 modifier its base 1 security check drops to 0.
fn opp_attacker(id: &str) -> CardData {
    let mut c = make_test_card(id, "Opp Attacker");
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(6000);
    c.play_cost = 4;
    c.colors = vec![CardColor::Red];
    c
}

/// A weak Digimon used as security stack filler — if a security check happens
/// it is consumed, so `security.len()` detects whether any check ran.
fn security_filler(id: &str) -> CardData {
    let mut c = make_test_card(id, "Security Filler");
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(0);
    c.play_cost = 3;
    c.colors = vec![CardColor::Red];
    c
}

fn filler(id: &str) -> CardData {
    make_test_card(id, "Filler")
}

/// Push a registered card into a player's hand; returns the hand index.
fn put_in_hand(runner: &mut DebugRunner, player: u8, card_id: &str) -> usize {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("put_in_hand: card {card_id} not registered"));
    let card_index = runner.game.next_card_index();
    runner.game.players[player as usize]
        .hand
        .push(CardSource::new(data_idx, player, card_index));
    runner.game.player(player).hand.len() - 1
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 1 — Structural assertions
// ─────────────────────────────────────────────────────────────────────────────

/// EX8-022 compiles with its printed stats: Lv.4 blue/yellow Digimon, cost 5,
/// DP 5000, traits Ice-Snow + LIBERATOR, the standard Lv.3-blue cost-3 evo
/// path, and the printed "Digivolve Lv.3 w/[Ice-Snow] trait: Cost 2" alt path
/// (DCGO gates on EqualsTraits("Ice-Snow") && Level == 3).
#[test]
fn ex8_022_compiles_with_printed_stats_and_digivolve_paths() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-022 found in embedded DSL pack")
        .start();

    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("EX8-022 in compiled cards");

    assert_eq!(compiled.card, "EX8-022");
    assert_eq!(compiled.name, "Frigimon");
    assert_eq!(compiled.kind, CompiledCardKind::Digimon);
    assert_eq!(compiled.level, Some(4));
    assert_eq!(
        compiled.color,
        vec![CompiledColor::Blue, CompiledColor::Yellow],
        "cards.json card_colors [1, 2] = blue + yellow (multi-color)"
    );
    assert_eq!(compiled.cost, Some(5));
    assert_eq!(compiled.dp, Some(5000));
    for trait_name in ["Ice-Snow", "LIBERATOR"] {
        assert!(
            compiled.traits.iter().any(|t| t == trait_name),
            "missing trait {trait_name}"
        );
    }

    // Standard printed evo box: Lv.3 blue for 3 memory.
    assert!(
        compiled.alt_paths.iter().any(|path| {
            path.kind == CompiledAltPathKind::Digivolve
                && path.cost == Some(CompiledCost::Literal(3))
                && path.from.as_ref().is_some_and(|from| {
                    (from.level_eq == Some(3)
                        || from.all_of.iter().any(|pred| pred.level_eq == Some(3)))
                        && (from.color_is == Some(CompiledColor::Blue)
                            || from
                                .all_of
                                .iter()
                                .any(|pred| pred.color_is == Some(CompiledColor::Blue)))
                })
        }),
        "Frigimon must digivolve from a blue Lv.3 for cost 3 (printed evo box)"
    );

    // Printed alt evo box: Lv.3 with [Ice-Snow] trait for 2 memory.
    assert!(
        compiled.alt_paths.iter().any(|path| {
            path.kind == CompiledAltPathKind::Digivolve
                && path.cost == Some(CompiledCost::Literal(2))
                && path.from.as_ref().is_some_and(|from| {
                    (from.level_eq == Some(3)
                        || from.all_of.iter().any(|pred| pred.level_eq == Some(3)))
                        && (from.trait_has.as_deref() == Some("Ice-Snow")
                            || from
                                .all_of
                                .iter()
                                .any(|pred| pred.trait_has.as_deref() == Some("Ice-Snow")))
                })
        }),
        "Frigimon must digivolve from a Lv.3 [Ice-Snow] Digimon for cost 2 (printed alt evo box)"
    );
}

/// The printed <Iceclad> keyword is encoded as a face-up grant_keyword clause
/// AND is live on the placed permanent via the canonical `Game::has_keyword`
/// query (the combat resolver's lookup site).
#[test]
fn ex8_022_iceclad_keyword_attached() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-022 found in embedded DSL pack")
        .start();

    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("EX8-022 in compiled cards");
    let has_iceclad_clause = compiled.effects.iter().any(|clause| {
        matches!(
            clause,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                scope,
                ..
            }) if keyword == "Iceclad" && *scope != CompiledScope::Inherited
        )
    });
    assert!(
        has_iceclad_clause,
        "EX8-022 must encode <Iceclad> as a face-up grant_keyword clause"
    );

    let frigimon = runner.place_on_field(0, CARD_ID, Some(0));
    assert!(
        runner.game.has_keyword(frigimon, Keyword::Iceclad),
        "the placed Frigimon must answer has_keyword(Iceclad) — the combat \
         resolver consumes Iceclad through this exact query"
    );
}

/// The [On Play][When Digivolving] clause is face-up, fires at both timings,
/// and is NOT optional and NOT once-per-turn (no printed qualifiers).
#[test]
fn ex8_022_on_play_wd_clause_structural_shape() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-022 found in embedded DSL pack")
        .start();
    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("EX8-022 in compiled cards");

    let triggered = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnPlay)
                    && t.when.contains(&CompiledTiming::WhenDigivolving) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("EX8-022 must encode an [On Play][When Digivolving] clause");

    assert_eq!(
        triggered.scope,
        CompiledScope::FaceUp,
        "the trash-bottom-2 clause is face-up card text"
    );
    assert!(
        !triggered.optional,
        "no printed 'you may' — the effect (and its selection) is mandatory"
    );
    assert!(
        !triggered.once_per_turn,
        "printed text carries no [Once Per Turn]"
    );
}

/// The inherited [When Attacking] clause is Inherited-scoped, triggered at
/// when_attacking, and NOT optional (no printed "you may").
#[test]
fn ex8_022_inherited_when_attacking_clause_structural_shape() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-022 found in embedded DSL pack")
        .start();
    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("EX8-022 in compiled cards");

    let triggered = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(t) if t.scope == CompiledScope::Inherited => Some(t),
            _ => None,
        })
        .expect("EX8-022 must encode a triggered inherited clause");

    assert!(
        triggered.when.contains(&CompiledTiming::WhenAttacking),
        "the inherited clause fires at [When Attacking]"
    );
    assert!(
        !triggered.optional,
        "no printed 'you may' — the effect (and its selection) is mandatory"
    );
    assert!(
        !triggered.once_per_turn,
        "printed text carries no [Once Per Turn]"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 2 — <Iceclad> battle semantics (stack-count compare, not DP)
// ─────────────────────────────────────────────────────────────────────────────

/// Iceclad attacker wins on source count despite LOWER DP: Frigimon (5000 DP)
/// with 1 source under it (2 cards total) beats a 8000-DP defender with 1
/// card. Without Iceclad the DP compare (5000 < 8000) would delete Frigimon.
#[test]
fn ex8_022_iceclad_wins_battle_on_source_count_despite_lower_dp() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-022 found in embedded DSL pack")
        .add_card(source_filler("SRC-A"))
        .add_card(opp_digimon("BIG-DEF", 8000))
        .start();

    let frigimon = runner.place_stack(0, &["SRC-A", CARD_ID]);
    let defender = runner.place_on_field(1, "BIG-DEF", Some(0));

    assert_eq!(
        runner.game.players[0].battle_area[frigimon.index as usize]
            .card_sources
            .len(),
        2,
        "Frigimon stack: 1 source + top = 2 cards"
    );

    let result = runner.attack_digimon(frigimon, defender, false);

    assert_eq!(
        result,
        AttackResult::AttackerWins,
        "Iceclad compares stack sizes (2 > 1), not DP (5000 < 8000)"
    );
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        1,
        "Frigimon survives via source-count advantage"
    );
    assert_eq!(
        runner.game.players[1].battle_area.len(),
        0,
        "the higher-DP defender is deleted — fewer sources loses under Iceclad"
    );
}

/// Equal source counts under Iceclad = mutual destruction, regardless of the
/// DP disparity (5000 vs 8000 would otherwise be DefenderWins).
#[test]
fn ex8_022_iceclad_equal_sources_mutual_destruction() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-022 found in embedded DSL pack")
        .add_card(opp_digimon("BIG-DEF", 8000))
        .start();

    let frigimon = runner.place_on_field(0, CARD_ID, Some(0));
    let defender = runner.place_on_field(1, "BIG-DEF", Some(0));

    let result = runner.attack_digimon(frigimon, defender, false);

    assert_eq!(
        result,
        AttackResult::MutualDestruction,
        "Iceclad: equal stack sizes (1 == 1) → both deleted"
    );
    assert_eq!(runner.game.players[0].battle_area.len(), 0);
    assert_eq!(runner.game.players[1].battle_area.len(), 0);
}

/// Iceclad attacker LOSES with fewer sources despite higher DP: bare Frigimon
/// (5000 DP, 1 card) attacks a 1000-DP defender with a 3-card stack. Without
/// Iceclad the DP compare (5000 > 1000) would delete the defender.
#[test]
fn ex8_022_iceclad_loses_with_fewer_sources_despite_higher_dp() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-022 found in embedded DSL pack")
        .add_card(source_filler("SRC-A"))
        .add_card(source_filler("SRC-B"))
        .add_card(opp_digimon("WEAK-DEF", 1000))
        .start();

    let frigimon = runner.place_on_field(0, CARD_ID, Some(0));
    let defender = runner.place_stack(1, &["SRC-A", "SRC-B", "WEAK-DEF"]);

    assert_eq!(
        runner.game.players[1].battle_area[defender.index as usize]
            .card_sources
            .len(),
        3
    );

    let result = runner.attack_digimon(frigimon, defender, false);

    assert_eq!(
        result,
        AttackResult::DefenderWins,
        "Iceclad compares stack sizes (1 < 3) — Frigimon loses despite 5000 vs 1000 DP"
    );
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        0,
        "Frigimon deleted — fewer sources loses under Iceclad"
    );
    assert_eq!(
        runner.game.players[1].battle_area.len(),
        1,
        "the 1000-DP defender survives via source-count advantage"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 3 — [On Play][When Digivolving] trash bottom 2 + memory gain
// ─────────────────────────────────────────────────────────────────────────────

/// POSITIVE: the opponent's only Digimon carries 2 digivolution cards (3-card
/// stack). On Play installs a MANDATORY single-target selection; choosing the
/// stack trashes its bottom 2 sources (they land in the opponent's trash, the
/// top card stays), and — the opponent now having no Digimon with digivolution
/// cards — Frigimon's controller gains 1 memory.
#[test]
fn ex8_022_on_play_trashes_bottom_2_sources_then_gains_memory_when_opponent_left_sourceless() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-022 found in embedded DSL pack")
        .add_card(source_filler("SRC-A"))
        .add_card(source_filler("SRC-B"))
        .add_card(opp_digimon("OPP", 6000))
        .memory(5)
        .start();
    runner.game.turn_count = 1;

    let opp = runner.place_stack(1, &["SRC-A", "SRC-B", "OPP"]);
    assert_eq!(
        runner.game.players[1].battle_area[opp.index as usize]
            .card_sources
            .len(),
        3
    );

    let frigimon = runner.place_on_field(0, CARD_ID, Some(0));
    let memory_before = runner.game.memory;
    runner.fire_on_play(0, frigimon.index as usize);

    let view = runner
        .pending_selection_view()
        .expect("[On Play] must install the opponent-Digimon selection");
    assert_eq!(view.kind, SelectionKind::OppField);
    assert!(
        !runner.pending_is_optional(),
        "printed text has no 'you may' — the selection must not expose PASS"
    );
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("choose the opponent's stacked Digimon");
    runner.auto_resolve().expect("finish the On Play effect");

    assert_eq!(
        runner.game.players[1].battle_area[opp.index as usize]
            .card_sources
            .len(),
        1,
        "the bottom 2 digivolution cards are trashed; the top card stays"
    );
    assert_eq!(
        runner.game.players[1].trash.len(),
        2,
        "both trashed sources land in the opponent's trash"
    );
    assert_eq!(
        runner.game.memory,
        memory_before + 1,
        "opponent now has no Digimon with digivolution cards → gain 1 memory"
    );
}

/// NEGATIVE (memory gate): the opponent has TWO sourced Digimon. Trashing
/// bottom-2 from the selected one still leaves the other with digivolution
/// cards, so NO memory is gained.
#[test]
fn ex8_022_on_play_no_memory_gain_while_opponent_still_has_sourced_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-022 found in embedded DSL pack")
        .add_card(source_filler("SRC-A"))
        .add_card(source_filler("SRC-B"))
        .add_card(source_filler("SRC-C"))
        .add_card(opp_digimon("OPP", 6000))
        .add_card(opp_digimon("OPP2", 4000))
        .memory(5)
        .start();
    runner.game.turn_count = 1;

    let _opp_a = runner.place_stack(1, &["SRC-A", "SRC-B", "OPP"]);
    let _opp_b = runner.place_stack(1, &["SRC-C", "OPP2"]);

    let frigimon = runner.place_on_field(0, CARD_ID, Some(0));
    let memory_before = runner.game.memory;
    runner.fire_on_play(0, frigimon.index as usize);

    let view = runner
        .pending_selection_view()
        .expect("[On Play] must install the opponent-Digimon selection");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("choose one of the opponent's Digimon");
    runner.auto_resolve().expect("finish the On Play effect");

    // Whichever stack was picked, the OTHER still carries a digivolution card.
    let opp_still_sourced = runner.game.players[1]
        .battle_area
        .iter()
        .any(|p| p.card_sources.len() >= 2);
    assert!(
        opp_still_sourced,
        "test setup: one opponent stack must still carry digivolution cards"
    );
    assert_eq!(
        runner.game.memory, memory_before,
        "a sourced opponent Digimon remains → no memory gain"
    );
}

/// DCGO gating: the trash-target selection is offered over ANY opponent
/// Digimon — DCGO's canTargetCondition is just "opponent battle-area Digimon",
/// with no digivolution-cards requirement. Selecting a sourceless Digimon
/// trashes nothing, and the memory check (no sourced opponent Digimon) then
/// passes → gain 1 memory.
#[test]
fn ex8_022_on_play_targets_sourceless_digimon_and_still_gains_memory() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-022 found in embedded DSL pack")
        .add_card(opp_digimon("OPP", 6000))
        .memory(5)
        .start();
    runner.game.turn_count = 1;

    let opp = runner.place_on_field(1, "OPP", Some(0));

    let frigimon = runner.place_on_field(0, CARD_ID, Some(0));
    let memory_before = runner.game.memory;
    runner.fire_on_play(0, frigimon.index as usize);

    let view = runner.pending_selection_view().expect(
        "the selection must be offered even though the only opponent Digimon \
         has no digivolution cards (DCGO targets ANY opponent Digimon)",
    );
    assert!(!runner.pending_is_optional(), "mandatory — no PASS");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("choose the sourceless opponent Digimon");
    runner.auto_resolve().expect("finish the On Play effect");

    assert_eq!(
        runner.game.players[1].battle_area[opp.index as usize]
            .card_sources
            .len(),
        1,
        "nothing to trash — the top card is never removed"
    );
    assert_eq!(runner.game.players[1].trash.len(), 0);
    assert_eq!(
        runner.game.memory,
        memory_before + 1,
        "opponent has no Digimon with digivolution cards → gain 1 memory"
    );
}

/// DCGO gating: with NO opponent Digimon at all, the selection is skipped but
/// the memory leg still runs (DCGO's AddMemory check sits OUTSIDE the
/// has-target guard): "no Digimon with digivolution cards" is vacuously true
/// → gain 1 memory.
#[test]
fn ex8_022_on_play_with_no_opponent_digimon_skips_selection_but_still_gains_memory() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-022 found in embedded DSL pack")
        .memory(5)
        .start();
    runner.game.turn_count = 1;

    let frigimon = runner.place_on_field(0, CARD_ID, Some(0));
    let memory_before = runner.game.memory;
    runner.fire_on_play(0, frigimon.index as usize);

    assert!(
        runner.pending_selection().is_none(),
        "no opponent Digimon — no selection to install"
    );
    assert_eq!(
        runner.game.memory,
        memory_before + 1,
        "the memory leg runs even with no opponent Digimon (DCGO: the check \
         is outside the target-exists guard and passes vacuously)"
    );
}

/// WHEN DIGIVOLVING via the printed alt evo box: Frigimon digivolves over a
/// RED Lv.3 [Ice-Snow] base (matching ONLY the alt path) for its printed cost
/// 2, then the [When Digivolving] effect fires: trash bottom 2 of the
/// opponent's only sourced Digimon → gain 1 memory.
#[test]
fn ex8_022_when_digivolving_via_ice_snow_alt_path_costs_2_and_fires_effect() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-022 found in embedded DSL pack")
        .add_card(red_ice_snow_lv3("ICEBASE"))
        .add_card(source_filler("SRC-A"))
        .add_card(source_filler("SRC-B"))
        .add_card(opp_digimon("OPP", 6000))
        .add_card(filler("FILL"))
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(8)
        .start();
    runner.game.turn_count = 1;

    let opp = runner.place_stack(1, &["SRC-A", "SRC-B", "OPP"]);
    let base = runner.place_on_field(0, "ICEBASE", Some(0));
    let hand_idx = put_in_hand(&mut runner, 0, CARD_ID);

    let memory_before = runner.game.memory;
    let digivolved =
        runner
            .game
            .digivolve_from_hand(0, hand_idx, base.index as usize, PlaySource::ByHand);
    assert!(
        digivolved,
        "Frigimon must digivolve over the red Lv.3 [Ice-Snow] base via the \
         printed alt evo box (Lv.3 w/[Ice-Snow]: cost 2)"
    );
    assert_eq!(
        runner.game.memory,
        memory_before - 2,
        "the alt path charges its printed cost 2 (the standard Lv.3-blue \
         path does not match a red base)"
    );

    runner.game.drain_effect_queue();
    let view = runner
        .pending_selection_view()
        .expect("[When Digivolving] must install the opponent-Digimon selection");
    assert_eq!(view.kind, SelectionKind::OppField);
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("choose the opponent's stacked Digimon");
    runner.auto_resolve().expect("finish the WD effect");

    assert_eq!(
        runner.game.players[1].battle_area[opp.index as usize]
            .card_sources
            .len(),
        1,
        "the bottom 2 digivolution cards are trashed"
    );
    assert_eq!(
        runner.game.memory,
        memory_before - 2 + 1,
        "after the trash the opponent has no sourced Digimon → gain 1 memory"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 4 — Inherited [When Attacking] Security A. -1 (EX8-019 pattern)
// ─────────────────────────────────────────────────────────────────────────────

/// POSITIVE: a carrier stacked on EX8-022 attacks; the inherited [When
/// Attacking] effect installs a MANDATORY single-target opponent-Digimon
/// selection (no PASS — no printed "you may"), and the chosen Digimon gets
/// SecurityAttackChange -1 (MINUS — the cards.json reminder text saying
/// "1 additional" is wrong; image badge + DCGO say -1 / "1 fewer").
#[test]
fn ex8_022_inherited_when_attacking_mandatory_select_gives_security_attack_minus_1() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-022 found in embedded DSL pack")
        .add_card(carrier("CARRIER"))
        .add_card(opp_attacker("OPP"))
        .start();
    let stack = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    let opp = runner.place_on_field(1, "OPP", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(stack));
    runner.game.drain_effect_queue();

    let view = runner
        .pending_selection_view()
        .expect("inherited [When Attacking] must install an opponent-target selection");
    assert_eq!(view.kind, SelectionKind::OppField);
    assert!(
        !runner.pending_is_optional(),
        "printed text has no 'you may' — the selection must not expose PASS"
    );
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("choose the opponent Digimon");
    runner
        .auto_resolve()
        .expect("finish the Security A. -1 grant");

    assert_eq!(
        runner
            .game
            .modifiers
            .sum(opp, ModifierType::SecurityAttackChange),
        -1,
        "the chosen opponent Digimon gets <Security A. -1> (MINUS 1, not plus)"
    );
}

/// BEHAVIORAL + EXPIRY: the granted Security A. -1 persists through the end of
/// the granting player's turn ("until the end of THEIR turn" = the opponent's
/// turn end; DCGO EffectDuration.UntilOpponentTurnEnd), suppresses the marked
/// Digimon's security check during the opponent's turn (base 1 - 1 = 0 checks),
/// and expires when the opponent's turn ends.
#[test]
fn ex8_022_security_attack_minus_1_zero_checks_and_expires_at_end_of_their_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-022 found in embedded DSL pack")
        .add_card(carrier("CARRIER"))
        .add_card(opp_attacker("OPP"))
        .add_card(security_filler("SEC"))
        .add_card(filler("FILL"))
        // Both players need draw fuel across two turn ends (deck-out would
        // end the game and mask the expiry under test).
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL"])
        .security(0, &["SEC", "SEC", "SEC"])
        .start();
    let stack = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    let opp = runner.place_on_field(1, "OPP", Some(0));

    // Player 0's turn: fire the inherited WA and mark the opponent Digimon.
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(stack));
    runner.game.drain_effect_queue();
    let view = runner.pending_selection_view().expect("opponent target");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("choose the opponent Digimon");
    runner
        .auto_resolve()
        .expect("finish the Security A. -1 grant");
    assert_eq!(
        runner
            .game
            .modifiers
            .sum(opp, ModifierType::SecurityAttackChange),
        -1
    );

    // End of player 0's turn: "until the end of THEIR turn" must SURVIVE the
    // granting player's own turn end.
    runner.end_turn();
    assert_eq!(
        runner
            .game
            .modifiers
            .sum(opp, ModifierType::SecurityAttackChange),
        -1,
        "Security A. -1 must persist into the opponent's turn (their turn has not ended)"
    );

    // Opponent's turn: the marked Digimon attacks player 0. Base 1 security
    // check - 1 = 0 checks, so no security card is consumed.
    let security_before = runner.game.players[0].security.len();
    assert_eq!(security_before, 3);
    let result = runner.attack_player(opp, 0, false);
    assert_eq!(
        result,
        AttackResult::SecurityCheckSurvived,
        "0 checks: the marked attacker survives without consuming security"
    );
    assert_eq!(
        runner.game.players[0].security.len(),
        3,
        "<Security A. -1> cancels the base check — no security card consumed"
    );

    // End of the opponent's turn: the modifier expires.
    runner.end_turn();
    assert_eq!(
        runner
            .game
            .modifiers
            .sum(opp, ModifierType::SecurityAttackChange),
        0,
        "Security A. -1 expires at the end of the opponent's turn"
    );
}

/// NEGATIVE (gate): with NO opponent Digimon on the field, the inherited
/// [When Attacking] effect must not install a selection (the DSL's empty
/// mandatory select is silently skipped — DCGO CanActivateCondition gate).
#[test]
fn ex8_022_inherited_when_attacking_skips_with_no_opponent_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-022 found in embedded DSL pack")
        .add_card(carrier("CARRIER"))
        .start();
    let stack = runner.place_stack(0, &[CARD_ID, "CARRIER"]);

    runner
        .game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(stack));
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "no opponent Digimon — the mandatory selection must be silently skipped"
    );
}
