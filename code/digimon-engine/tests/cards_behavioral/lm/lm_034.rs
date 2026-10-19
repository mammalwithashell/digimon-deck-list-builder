//! LM-034 Wisteria Memory Boost!: official observation 3bbaa1e5...7275f3,
//! actual card image, base DCGO LM_034.cs, and general rules 4-19/16-16.
//! Exercises printed alternate color access, reveal choices/order, Delay and security.

use digimon_dsl::compiled::{
    CompiledCard, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep,
    CompiledTiming,
};
use digimon_engine::action::space::SEL_REVEAL_START;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

const CARD_ID: &str = "LM-034";
const YAML: &str = include_str!("../../../cards/lm/LM-034.yaml");

// ── Card-data factories ──────────────────────────────────────────────────────

/// A neutral filler card — no special colour, default Digimon shape. Used as
/// non-eligible reveal padding.
fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

/// A Blue Digimon — eligible for the Clause 1 bucket (kind: digimon,
/// color_is: blue|red). Also relevant as a non-Red board permanent for
/// Clause 0 negative gate tests.
fn make_blue_digimon(card_id: &str, name: &str) -> CardData {
    let mut c = make_test_card(card_id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 4;
    c.colors = vec![CardColor::Blue];
    c
}

/// A Red Digimon — eligible for the Clause 1 bucket and gates the Clause 0
/// colour bypass (DCGO CanUseCondition: own field has Red Digimon/Tamer).
fn make_red_digimon(card_id: &str, name: &str) -> CardData {
    let mut c = make_test_card(card_id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 4;
    c.colors = vec![CardColor::Red];
    c
}

/// A Red Tamer — alternative gate for the Clause 0 any_of bypass condition.
fn make_red_tamer(card_id: &str, name: &str) -> CardData {
    let mut c = make_test_card(card_id, name);
    c.card_kind = CardKind::Tamer;
    c.play_cost = 3;
    c.colors = vec![CardColor::Red];
    c
}

/// A non-Digimon, non-Red filler — used for negative reveal-bucket cases
/// (option/tamer/black filler that fails the bucket filter).
fn make_black_digimon(card_id: &str, name: &str) -> CardData {
    let mut c = make_test_card(card_id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 4;
    c.colors = vec![CardColor::Black];
    c
}

/// Helper: walk the revealed-cards list to find the action ID for a specific
/// revealed card_id. Mirrors `revealed_action_for_id` in bt22_099.
fn revealed_action_for_id(runner: &DebugRunner, id: &str) -> Option<u16> {
    runner
        .game
        .revealed_cards
        .iter()
        .enumerate()
        .find_map(|(idx, card)| {
            (card.card_id(&runner.game.card_data) == id).then_some(SEL_REVEAL_START + idx as u16)
        })
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions (YAML parse + clause shape)
// ═══════════════════════════════════════════════════════════════════════════════

/// LM-034 YAML must parse and compile without errors.
#[test]
fn lm_034_yaml_parses_and_compiles() {
    let _builder = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-034 YAML must parse and compile without errors");
}

/// LM-034 must compile as an Option card with cost 3.
#[test]
fn lm_034_is_option_cost_3() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let card = runner
        .compiled_card(CARD_ID)
        .expect("LM-034 compiled card must be registered");

    assert_eq!(
        card.kind,
        digimon_dsl::compiled::CompiledCardKind::Option,
        "LM-034 must be an Option card"
    );
    assert_eq!(card.cost, Some(3), "LM-034 prints Cost 3");
}

/// Color access is top-level metadata; Main, Delay and Security are effect bodies.
#[test]
fn lm_034_has_main_delay_and_security_with_use_requirement() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let card = runner.compiled_card(CARD_ID).expect("LM-034 compiled");
    assert_eq!(
        card.effects.len(),
        3,
        "expected Main, Delay and Security effect bodies; got {}",
        card.effects.len()
    );
}

#[test]
fn lm_034_has_printed_alternate_color_use_requirement() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .start();
    assert!(runner
        .compiled_card(CARD_ID)
        .unwrap()
        .use_requirement
        .is_some());
}

/// Clause 1: main_from_hand triggered, FaceUp scope, NOT optional (DCGO
/// `SetUpActivateClass(..., -1, false, ...)`; no "you may" in printed text).
#[test]
fn lm_034_clause_1_main_from_hand_face_up_mandatory() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let card = runner.compiled_card(CARD_ID).expect("LM-034 compiled");

    match &card.effects[0] {
        CompiledClause::Triggered(t) => {
            assert!(
                t.when.contains(&CompiledTiming::MainFromHand),
                "clause 1 must fire at MainFromHand; got {:?}",
                t.when
            );
            assert_eq!(
                t.scope,
                CompiledScope::FaceUp,
                "clause 1 must have FaceUp scope"
            );
            assert!(
                !t.optional,
                "clause 1 outer trigger is NOT optional — DCGO SetUpActivateClass(..., -1, false, ...); \
                 no 'you may' in printed text"
            );
            assert!(
                !t.once_per_turn,
                "clause 1 carries no [Once Per Turn] in printed text"
            );
        }
        other => panic!(
            "clause 1 must be Triggered(main_from_hand); got {:?}",
            other
        ),
    }
}

/// Clause 2: declarative Delay with player-activated Main timing.
#[test]
fn lm_034_clause_2_is_player_activated_delay() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let card = runner.compiled_card(CARD_ID).expect("LM-034 compiled");

    match &card.effects[1] {
        CompiledClause::Declarative(CompiledDeclarativeClause::Delay { trigger, .. }) => {
            // TRIGGER CORRECTION (2026-08-24): this assertion used to demand
            // `EndOfYourNextTurn` and call that "standard <Delay>" -- backwards.
            // The printed timing is `[Main]`, and DCGO registers the clause at
            // `EffectTiming.OnDeclaration` (its [Main] activated-declaration
            // timing), so this is a player-activated §16-16 Delay. `Delayed`
            // lowers to `DelayTrigger::MainPhaseActivated`, a [Main] mask bit;
            // NOT activating is the §16-16-2 decline. `EndOfYourNextTurn` both
            // fired it at the wrong time and gave the player no say.
            assert_eq!(
                *trigger,
                CompiledTiming::Delayed,
                "[Main] <Delay> must lower to Delayed (player-activated); got {:?}",
                trigger
            );
        }
        other => panic!("clause 2 must be Declarative(Delay); got {:?}", other),
    }
}

/// Delay clause process must contain GainMemory(2).
#[test]
fn lm_034_delay_process_has_gain_memory_2() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let card = runner.compiled_card(CARD_ID).expect("LM-034 compiled");

    match &card.effects[1] {
        CompiledClause::Declarative(CompiledDeclarativeClause::Delay { process, .. }) => {
            assert!(!process.is_empty(), "Delay process must be non-empty");
            let has_gain_2 = process
                .iter()
                .any(|s| matches!(s, CompiledStep::GainMemory(2)));
            assert!(
                has_gain_2,
                "Delay process must contain GainMemory(2); got {:?}",
                process
            );
        }
        other => panic!("clause 2 must be Delay; got {:?}", other),
    }
}

/// Clause 3: inherited scope, OnSecurity timing, process contains the
/// place_self_as_delay_option step (DCGO PlaceSelfDelayOptionSecurityEffect).
#[test]
fn lm_034_clause_3_inherited_security_places_self_as_delay() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let card = runner.compiled_card(CARD_ID).expect("LM-034 compiled");

    match &card.effects[2] {
        CompiledClause::Triggered(t) => {
            assert_eq!(
                t.scope,
                CompiledScope::Inherited,
                "clause 3 must have Inherited scope"
            );
            assert!(
                t.when.contains(&CompiledTiming::OnSecurity),
                "clause 3 must fire at OnSecurity; got {:?}",
                t.when
            );
            assert!(
                !t.optional,
                "Security clause must be mandatory ([Security] effects are not opt-out per \
                 RULES_CONTEXT.md §16; DCGO PlaceSelfDelayOptionSecurityEffect has no canNoSelect)"
            );
            // Verify the placement step is present in the body.
            let has_place_step = t
                .process
                .iter()
                .any(|s| matches!(s, CompiledStep::PlaceSelfAsDelayOption));
            assert!(
                has_place_step,
                "clause 3 process must contain PlaceSelfAsDelayOption; got {:?}",
                t.process
            );
        }
        other => panic!(
            "clause 3 must be Triggered(inherited on_security); got {:?}",
            other
        ),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Behavioral: Clause 1 [Main] reveal-3 + add 1 blue/red Digimon + bottom rest
// ═══════════════════════════════════════════════════════════════════════════════
//
// Strategy mirrors bt22_099 / p_035: use `activate_hand_main` to drive the
// MainFromHand effect process directly without relying on the full play-flow
// (which would exercise color-requirement checks against Blue). The [Main]
// body is identical in shape to BT22-099 Clause 1, with the trait predicate
// substituted by a colour-disjunction `kind: digimon + (blue | red)` filter.

/// Positive: top-3 reveal contains [BLUE-DIGI, FILLER, FILLER] — the player
/// must pick the Blue Digimon, which lands in hand; the two FILLERs go to
/// the bottom of the deck.
#[test]
fn lm_034_main_picks_blue_digimon_and_bottoms_remainder() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_blue_digimon("BLUE-DIGI", "Blue Digimon"))
        .add_card(filler("FILL-A"))
        .add_card(filler("FILL-B"))
        // Builder convention: last in slice = top of deck (drawn first).
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL-A", "FILL-B", "BLUE-DIGI"])
        .deck(1, &["FILL-A"])
        .memory(10)
        .start();

    let fired = runner.game.activate_hand_main(0, 0);
    assert!(fired, "activate_hand_main must return true for LM-034");

    // The blue/red-Digimon bucket prompt must install (single eligible card).
    assert!(
        runner.game.pending_selection.is_some(),
        "select_reveal_buckets prompt must install for the digi bucket when an eligible \
         Digimon is in the reveal"
    );

    // Pick BLUE-DIGI for the bucket.
    let blue_action = revealed_action_for_id(&runner, "BLUE-DIGI")
        .expect("BLUE-DIGI must appear among the revealed-card actions");
    runner
        .execute_action(0, blue_action)
        .expect("pick BLUE-DIGI for the digi bucket");

    // Drive any trailing remainder-placement order prompts.
    let _ = runner.auto_resolve();

    // BLUE-DIGI must be in P0's hand.
    let hand_ids: Vec<String> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    assert!(
        hand_ids.contains(&"BLUE-DIGI".to_string()),
        "BLUE-DIGI must be added to hand; hand was {hand_ids:?}"
    );
    assert!(
        !hand_ids.iter().any(|id| id == "FILL-A" || id == "FILL-B"),
        "FILLERs must NOT enter hand; hand was {hand_ids:?}"
    );

    // The two FILLERs must be at the bottom of the deck.
    let deck_ids: Vec<String> = runner.game.players[0]
        .deck
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    let bottom_two: Vec<String> = deck_ids.iter().take(2).cloned().collect();
    assert!(
        bottom_two.iter().any(|id| id == "FILL-A") && bottom_two.iter().any(|id| id == "FILL-B"),
        "FILL-A and FILL-B must be at the bottom of the deck; bottom-2 was {bottom_two:?}"
    );
}

/// Positive (Red branch): top-3 reveal contains [RED-DIGI, FILLER, FILLER] —
/// the player must pick the Red Digimon (Red is also valid per the bucket's
/// colour disjunction), which lands in hand.
#[test]
fn lm_034_main_picks_red_digimon_and_bottoms_remainder() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_red_digimon("RED-DIGI", "Red Digimon"))
        .add_card(filler("FILL-A"))
        .add_card(filler("FILL-B"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL-A", "FILL-B", "RED-DIGI"])
        .deck(1, &["FILL-A"])
        .memory(10)
        .start();

    let fired = runner.game.activate_hand_main(0, 0);
    assert!(fired, "activate_hand_main must return true for LM-034");

    assert!(
        runner.game.pending_selection.is_some(),
        "select_reveal_buckets prompt must install for the digi bucket when a Red Digimon \
         is in the reveal (Red is in the bucket colour disjunction)"
    );

    let red_action = revealed_action_for_id(&runner, "RED-DIGI")
        .expect("RED-DIGI must appear among the revealed-card actions");
    runner
        .execute_action(0, red_action)
        .expect("pick RED-DIGI for the digi bucket");

    let _ = runner.auto_resolve();

    let hand_ids: Vec<String> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    assert!(
        hand_ids.contains(&"RED-DIGI".to_string()),
        "RED-DIGI must be added to hand; hand was {hand_ids:?}"
    );
}

/// Negative: no eligible Digimon in the reveal — the bucket has zero
/// candidates and silently resolves; all 3 revealed cards return to the
/// bottom of the deck. Hand stays at size 1 (LM-034 itself; activate_hand_main
/// does not consume it).
#[test]
fn lm_034_main_no_blue_or_red_digimon_bottoms_all_three() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        // BLACK Digimon is the right kind but wrong colour — fails the bucket.
        .add_card(make_black_digimon("BLK-A", "Black A"))
        .add_card(make_black_digimon("BLK-B", "Black B"))
        .add_card(make_black_digimon("BLK-C", "Black C"))
        .hand(0, &[CARD_ID])
        .deck(0, &["BLK-A", "BLK-B", "BLK-C"])
        .deck(1, &["BLK-A"])
        .memory(10)
        .start();

    let deck_before = runner.deck_size(0);
    runner.game.activate_hand_main(0, 0);
    let _ = runner.auto_resolve();

    // No black Digimon should have been added to hand — only LM-034 remains.
    let hand_ids: Vec<String> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    for blk in ["BLK-A", "BLK-B", "BLK-C"] {
        assert!(
            !hand_ids.contains(&blk.to_string()),
            "{blk} (Black, wrong colour) must NOT enter hand; hand was {hand_ids:?}"
        );
    }

    // All 3 revealed cards must be back in the deck — net deck size unchanged.
    let deck_after = runner.deck_size(0);
    assert_eq!(
        deck_after, deck_before,
        "all 3 revealed Black Digimon must return to deck bottom (no eligible card to hand); \
         deck size before={deck_before}, after={deck_after}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Deck accounting after the [Main] reveal
// ═══════════════════════════════════════════════════════════════════════════════

/// Mirror of bt22_099 / p_035 deck-accounting test: when 1 eligible Digimon
/// is added to hand, deck shrinks by exactly 1 (3 revealed - 2 returned bottom
/// = 1 removed to hand).
#[test]
fn lm_034_main_deck_size_shrinks_by_one_when_eligible_digi_added() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_blue_digimon("BLUE-DIGI", "Blue Digimon"))
        .add_card(filler("FILL-A"))
        .add_card(filler("FILL-B"))
        .add_card(filler("FILL-C"))
        .add_card(filler("FILL-D"))
        .hand(0, &[CARD_ID])
        // Top of deck = last in slice. BLUE-DIGI is on top, in the reveal-3.
        .deck(
            0,
            &[
                "FILL-A",
                "FILL-B",
                "FILL-C",
                "FILL-D",
                "FILL-A",
                "BLUE-DIGI",
            ],
        )
        .deck(1, &["FILL-A"])
        .memory(10)
        .start();

    let deck_before = runner.deck_size(0);
    runner.game.activate_hand_main(0, 0);

    if let Some(action) = revealed_action_for_id(&runner, "BLUE-DIGI") {
        runner
            .execute_action(0, action)
            .expect("pick BLUE-DIGI for the digi bucket");
    }
    let _ = runner.auto_resolve();
    let deck_after = runner.deck_size(0);

    assert_eq!(
        deck_before - deck_after,
        1,
        "Deck must shrink by exactly 1 (3 revealed, 1 to hand, 2 returned to bottom); \
         before={deck_before}, after={deck_after}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — Anti-helpers (silence unused-warning across test fork)
// ═══════════════════════════════════════════════════════════════════════════════

#[allow(dead_code)]
fn _unused_silencer() {
    let _ = make_red_tamer("X", "X");
    let _ = make_black_digimon("Y", "Y");
}

// Tail audit: official 3bbaa1e5..., LM-034 image, rules 4-19 / 16-16, DCGO LM_034.cs.

use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{
    EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_MAIN, FIELD_EFFECT_START, PASS, PLAY_HAND_START,
};
use digimon_engine::combat::AttackResult;
use digimon_engine::enums::DelayTrigger;
use digimon_engine::permanent::OptionState;
use digimon_engine::selection::OptionPlayResult;

fn digimon(id: &str, color: CardColor) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![color];
    c
}
fn tamer(id: &str, color: CardColor) -> CardData {
    let mut c = digimon(id, color);
    c.card_kind = CardKind::Tamer;
    c
}
fn option(id: &str, color: CardColor) -> CardData {
    let mut c = digimon(id, color);
    c.card_kind = CardKind::Option;
    c
}
#[test]
fn lm_034_delay_activation_gains_2_memory_via_main_phase_action() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-034 YAML parses")
        .add_card(tamer("BLUE-TAMER", CardColor::Blue))
        .add_card(filler("FILL"))
        .hand(0, &["LM-034"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(10)
        .start();

    runner.place_on_field(0, "BLUE-TAMER", Some(0));
    runner.game.enter_main_phase();

    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending
    );
    runner
        .auto_resolve()
        .expect("resolve LM-034 reveal selection and delay placement");

    let delay_idx = runner
        .game
        .player(0)
        .battle_area
        .iter()
        .position(|permanent| {
            matches!(
                permanent.option_state,
                OptionState::Delayed {
                    trigger: DelayTrigger::MainPhaseActivated,
                    ..
                }
            )
        })
        .expect("playing LM-034 should park it as a MainPhaseActivated delayed option");

    let bit = (FIELD_EFFECT_START
        + delay_idx as u16 * EFFECTS_PER_PERMANENT
        + FIELD_EFFECT_SLOT_FOR_MAIN) as usize;
    assert_eq!(
        build_action_mask(&runner.game, 0)[bit],
        0.0,
        "LM-034 <Delay> must not be activatable on the placing turn"
    );

    runner.end_turn();
    runner.game.enter_main_phase();
    runner.end_turn();
    assert_eq!(runner.game.turn_player(), 0);
    runner.game.enter_main_phase();
    runner.game.set_memory(0);

    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[bit], 1.0,
        "LM-034 <Delay> activation is a legal action"
    );
    assert_eq!(mask[PASS as usize], 1.0, "declining stays legal");

    let saved = runner.game.clone();
    runner.game.decode_action(PASS, 0);
    assert!(runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID));
    assert!(!runner.game.players[0]
        .trash
        .iter()
        .any(|c| c.card_id(&runner.game.card_data) == CARD_ID));
    runner.game = saved.clone();
    runner.game.decode_action(bit as u16, 0);
    assert!(runner.game.players[0]
        .trash
        .iter()
        .any(|c| c.card_id(&runner.game.card_data) == "LM-034"));
    runner.game = saved;
    assert_eq!(runner.memory(), 0, "cloning preserves unactivated Delay");
    runner.game.decode_action(bit as u16, 0);

    assert_eq!(
        runner.memory(),
        2,
        "LM-034 <Delay> body gains 2 memory when the player activates it"
    );
    assert!(
        !runner
            .game
            .player(0)
            .battle_area
            .iter()
            .any(|permanent| matches!(permanent.option_state, OptionState::Delayed { .. })),
        "LM-034 is trashed as the <Delay> activation cost"
    );
}

#[test]
fn lm_034_inherited_security_places_self_in_battle_area() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-034 YAML parses")
        .add_card(digimon("ATTACKER", CardColor::Black))
        .add_card(filler("FILL"))
        .security(1, &["LM-034"])
        .deck(0, &["FILL"])
        .memory(0)
        .start();
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));

    let result = runner.attack_player(attacker, 1, false);

    assert_eq!(result, AttackResult::SecurityCheckSurvived);
    assert_eq!(runner.security_count(1), 0, "LM-034 should leave security");
    assert_eq!(
        runner.trash_size(1),
        0,
        "LM-034 should be placed in battle area instead of trashed"
    );
    let placed = runner
        .game
        .player(1)
        .battle_area
        .iter()
        .find(|permanent| permanent.top_card().card_id(&runner.game.card_data) == "LM-034")
        .expect("LM-034 should be placed as a battle-area Option permanent");
    assert!(matches!(
        placed.option_state,
        OptionState::Delayed {
            trigger: DelayTrigger::MainPhaseActivated,
            ..
        }
    ));
}

#[test]
fn lm_034_color_access_owner_kind_and_breeding_matrix() {
    for (kind, color, owner, breeding, allowed) in [
        (CardKind::Digimon, CardColor::Blue, 0, true, true),
        (CardKind::Digimon, CardColor::Red, 0, true, true),
        (CardKind::DigiEgg, CardColor::Blue, 0, true, true),
        (CardKind::DigiEgg, CardColor::Red, 0, true, true),
        (CardKind::Tamer, CardColor::Red, 0, false, true),
        (CardKind::Digimon, CardColor::Blue, 0, false, true),
        (CardKind::Tamer, CardColor::Red, 0, false, true),
        (CardKind::Digimon, CardColor::Red, 0, true, true),
        (CardKind::Option, CardColor::Blue, 0, false, false),
        (CardKind::Option, CardColor::Red, 0, false, false),
        (CardKind::Digimon, CardColor::Red, 1, false, false),
        (CardKind::Digimon, CardColor::Blue, 1, false, false),
        (CardKind::Tamer, CardColor::Black, 0, false, false),
    ] {
        let mut c = digimon("ACCESS", color);
        c.card_kind = kind;
        let mut r = DebugRunner::builder()
            .from_dsl_yaml(YAML)
            .unwrap()
            .add_card(c)
            .hand(0, &["LM-034"])
            .memory(10)
            .start();
        r.skip_mulligan();
        r.place_on_field(owner, "ACCESS", None);
        if breeding {
            let p = r.game.players[owner as usize].battle_area.remove(0);
            r.game.players[owner as usize].breeding_area = Some(p);
        }
        assert_eq!(
            build_action_mask(&r.game, 0)[PLAY_HAND_START as usize] > 0.0,
            allowed,
            "{kind:?} {color:?} owner={owner} breeding={breeding}"
        );
    }
}

fn audit_reveal(top: &[&str]) -> DebugRunner {
    use digimon_engine::card_source::CardSource;
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .unwrap()
        .add_card(digimon("B", CardColor::Blue))
        .add_card(digimon("G", CardColor::Red))
        .add_card(digimon("R", CardColor::Black))
        .add_card(tamer("T", CardColor::Red))
        .add_card(option("O", CardColor::Blue))
        .hand(0, &["LM-034"])
        .memory(10)
        .start();
    r.skip_mulligan();
    r.place_on_field(0, "B", None);
    for id in top.iter().rev() {
        let index = r
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == *id)
            .unwrap();
        let serial = r.game.next_card_index();
        r.game.players[0]
            .deck
            .push(CardSource::new(index, 0, serial));
    }
    assert_eq!(build_action_mask(&r.game, 0)[PLAY_HAND_START as usize], 1.0);
    r.game.decode_action(PLAY_HAND_START, 0);
    r
}

fn audit_order(r: &mut DebugRunner, reverse: bool) {
    use digimon_engine::selection::SelectionKind;
    for _ in 0..4 {
        let Some(v) = r.pending_selection_view() else {
            return;
        };
        assert!(matches!(v.kind, SelectionKind::OrderedPermutation { .. }));
        assert_eq!(build_action_mask(&r.game, 0)[PASS as usize], 0.0);
        let a = if reverse {
            *v.valid_action_ids.last().unwrap()
        } else {
            v.valid_action_ids[0]
        };
        r.execute_action(0, a).unwrap();
    }
    panic!("remainder did not resolve");
}

#[test]
fn lm_034_mandatory_search_offers_both_colors_and_clone_preserves_order_choices() {
    use digimon_engine::action::space::SEL_REVEAL_START;
    let mut r = audit_reveal(&["B", "G", "T", "R"]);
    let m = build_action_mask(&r.game, 0);
    assert_eq!(m[PASS as usize], 0.0);
    assert_eq!(m[SEL_REVEAL_START as usize], 1.0);
    assert_eq!(m[SEL_REVEAL_START as usize + 1], 1.0);
    assert_eq!(m[SEL_REVEAL_START as usize + 2], 0.0);
    let saved = r.game.clone();
    r.execute_action(0, SEL_REVEAL_START).unwrap();
    let order = r.pending_selection_view().unwrap();
    assert_eq!(order.valid_action_ids.len(), 2);
    audit_order(&mut r, false);
    let first: Vec<_> = r.game.players[0]
        .deck
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_owned())
        .collect();
    assert_eq!(r.hand_size(0), 1);
    assert_eq!(r.game.players[0].hand[0].card_id(&r.game.card_data), "B");
    assert_eq!(r.memory(), 7);
    assert_eq!(r.game.players[0].battle_area.len(), 2);
    r.game = saved;
    r.execute_action(0, SEL_REVEAL_START).unwrap();
    audit_order(&mut r, true);
    let second: Vec<_> = r.game.players[0]
        .deck
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_owned())
        .collect();
    assert_ne!(
        first, second,
        "both legal bottom orders must be player choices"
    );
    assert_eq!(first.last().map(String::as_str), Some("R"));
    assert_eq!(second.last().map(String::as_str), Some("R"));
}

#[test]
fn lm_034_no_eligible_and_short_deck_still_place_delay() {
    for top in [&["O", "T", "R"][..], &["G"][..], &[][..]] {
        let mut r = audit_reveal(top);
        if top == ["G"] {
            let v = r.pending_selection_view().unwrap();
            assert_eq!(v.valid_action_ids.len(), 1);
            r.execute_action(0, v.valid_action_ids[0]).unwrap();
        }
        audit_order(&mut r, false);
        assert_eq!(r.hand_size(0), usize::from(top == ["G"]));
        assert!(r.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&r.game.card_data) == "LM-034"));
        assert_eq!(r.memory(), 7);
    }
}
