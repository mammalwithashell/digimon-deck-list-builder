//! EX1-066 Analog Youth — Tamer, White, Cost 2.
//!
//! # Card text (official Bandai DB — data/card_bundles/EX1-066.md, authoritative)
//!
//! [On Play] Reveal the top 3 cards of your deck. Add 1 Digimon card among
//!   them to your hand. Trash the remaining cards.
//! [All Turns] When one of your level 5 or higher Digimon with a
//!   digivolution card is deleted, you may suspend this Tamer. If you do,
//!   gain 1 memory, then hatch 1 Digi-Egg card to your Breeding Area if it's
//!   empty.
//! Inherited: [Security] Play this card without paying its memory cost.
//!
//! Official Q&A: "Yes, you can [suspend even if you can't hatch]. You can't
//! hatch a Digi-Egg card, however." — the suspend + memory gain are NOT
//! gated on `can_hatch`; only the hatch sub-step is.
//!
//! # DCGO C# reference (READ-ONLY)
//! `DCGO/Assets/Scripts/CardEffect/EX1/White/EX1_066.cs`
//!
//! - `OnEnterFieldAnyone`: `SimplifiedRevealDeckTopCardsAndSelect(revealCount:
//!   3, canSelectCardCondition: cardSource.IsDigimon, mode: AddHand,
//!   maxCount: 1, remainingCardsPlace: Trash)`.
//! - `OnDestroyedAnyone`: `PermanentCondition` gates on
//!   `IsPermanentExistsOnOwnerBattleAreaDigimon && !HasNoDigivolutionCards &&
//!   Level >= 5`; `ActivateCoroutine` unconditionally suspends self + adds 1
//!   memory, then hatches only `if (card.Owner.CanHatch)`.
//! - `SecuritySkill`: `PlaySelfTamerSecurityEffect`.
//!
//! # Patterns this test covers
//!
//! - [On Play] reveal-3 / add-1-Digimon / trash-the-rest composite
//!   (`reveal_top_deck` -> `select_reveal` (optional, `kind: digimon` filter)
//!   -> `add_to_hand_from_reveal` -> `per_selected { selection: revealed }`
//!   -> `trash_from_reveal`), same idiom as BT21-097 / BT25-098.
//! - [All Turns] `on_any_deletion` observer scoped to the controller's OWN
//!   Lv.5+ Digimon that has >=1 digivolution source
//!   (`event_target_owner`/`event_target_kind`/`event_target_level_gte`/
//!   `event_target_has_digivolution_cards`), gated `optional: true` with an
//!   `activation_cost: { suspend_self: true }`, matching BT8-094's
//!   on_any_deletion shape.
//! - Unconditional `gain_memory: 1` once the suspend cost is paid, followed
//!   by a `can_hatch`-gated `hatch` step (Q&A: suspend/memory succeed even
//!   when hatch is impossible).
//! - [Security] `play_from_security` (self, free) inherited clause.

use digimon_dsl::compiled::{CompiledClause, CompiledStep, CompiledTiming};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::CardKind;
use digimon_engine::permanent::Permanent;
use digimon_engine::replacement::ReplacementCause;

const CARD_ID: &str = "EX1-066";

fn hand_ids(runner: &DebugRunner, player: u8) -> Vec<String> {
    runner.game.players[player as usize]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect()
}

fn trash_ids(runner: &DebugRunner, player: u8) -> Vec<String> {
    runner.game.players[player as usize]
        .trash
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect()
}

/// Push cards onto player 0's deck top. Last element in `ids` ends up on top
/// (i.e. the first one revealed).
fn stack_deck_top(runner: &mut DebugRunner, ids: &[&str]) {
    for id in ids {
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == *id)
            .unwrap_or_else(|| panic!("card {id} registered"));
        let card_index = runner.game.next_card_index();
        runner.game.players[0]
            .deck
            .push(CardSource::new(data_idx, 0, card_index));
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Structural: compiles + registers as a White/Tamer/cost-2 card with the
// expected three clauses.
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn ex1_066_compiles_as_white_tamer_cost_2() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX1-066 YAML parses and compiles")
        .build();
    let card = runner.compiled_card(CARD_ID).expect("EX1-066 compiled");
    assert_eq!(card.effects.len(), 3, "On Play + All Turns + Security");
}

#[test]
fn ex1_066_has_security_play_from_security_clause() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX1-066 YAML parses and compiles")
        .build();
    let card = runner.compiled_card(CARD_ID).expect("EX1-066 compiled");
    let security = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSecurity) => Some(t),
            _ => None,
        })
        .expect("EX1-066 must have a [Security] clause");
    assert!(security
        .process
        .iter()
        .any(|step| matches!(step, CompiledStep::PlayFromSecurity)));
}

// ─── Clause 1: [On Play] reveal 3 / add 1 Digimon / trash the rest ────────

#[test]
fn ex1_066_has_on_play_reveal_clause_with_expected_steps() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX1-066 YAML parses and compiles")
        .build();
    let card = runner.compiled_card(CARD_ID).expect("EX1-066 compiled");
    let on_play = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnPlay) => Some(t),
            _ => None,
        })
        .expect("EX1-066 must have an [On Play] clause");

    assert!(on_play
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::RevealTopDeck { count: 3, .. })));
    assert!(on_play
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::SelectReveal { .. })));
    assert!(on_play
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::AddToHandFromReveal { .. })));

    fn process_contains_trash_from_reveal(steps: &[CompiledStep]) -> bool {
        steps.iter().any(|s| match s {
            CompiledStep::TrashFromReveal { .. } => true,
            CompiledStep::PerSelected { body, .. } | CompiledStep::ForEach { body, .. } => {
                process_contains_trash_from_reveal(body)
            }
            _ => false,
        })
    }
    assert!(
        process_contains_trash_from_reveal(&on_play.process),
        "the [On Play] clause must trash the un-picked reveals"
    );
}

/// Positive: 3 revealed cards, exactly 1 Digimon among them → the Digimon is
/// added to hand, the other 2 are trashed.
#[test]
fn ex1_066_on_play_adds_the_lone_digimon_and_trashes_the_rest() {
    let mut digimon_card = make_test_card("EX1066-DIGI-A", "Digi A");
    digimon_card.card_kind = CardKind::Digimon;
    digimon_card.level = Some(3);
    digimon_card.dp = Some(3000);
    digimon_card.play_cost = 0;

    let mut tamer_card = make_test_card("EX1066-TAMER-B", "Tamer B");
    tamer_card.card_kind = CardKind::Tamer;
    tamer_card.level = None;
    tamer_card.dp = None;
    tamer_card.play_cost = 0;

    let mut option_card = make_test_card("EX1066-OPT-C", "Option C");
    option_card.card_kind = CardKind::Option;
    option_card.level = None;
    option_card.dp = None;
    option_card.play_cost = 0;

    let filler = make_test_card("EX1066-FILLER-A", "Filler A");

    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX1-066 loads")
        .add_card(digimon_card)
        .add_card(tamer_card)
        .add_card(option_card)
        .add_card(filler)
        .deck(0, &["EX1066-FILLER-A"])
        .security(1, &["EX1066-FILLER-A"])
        .memory(3)
        .start();

    // Reveal order: top-of-deck first. stack_deck_top pushes bottom-up, so
    // the LAST id in the slice is drawn/revealed first.
    stack_deck_top(
        &mut runner,
        &["EX1066-OPT-C", "EX1066-TAMER-B", "EX1066-DIGI-A"],
    );

    // place_on_field + fire_play_event_triggers mirrors the production
    // play-flow (matches BT21-097's pattern) without double-firing OnPlay
    // the way `runner.play()` + a manual `fire_play_event_triggers` would.
    runner.place_on_field(0, CARD_ID, Some(0));
    let hand_before = runner.hand_size(0);
    let trash_before = runner.trash_size(0);

    runner.fire_play_event_triggers(0, 0, false, false);

    let view = runner
        .pending_selection_view()
        .expect("reveal add-to-hand selection pending");
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only the Digimon card (EX1066-DIGI-A) is eligible to add"
    );
    // Printed text has no "may": with >=1 Digimon revealed the pick is FORCED
    // (DCGO SimplifiedRevealDeckTopCardsAndSelect defaults canNoSelect=false) —
    // no legal PASS may be exposed to the action space.
    assert!(
        !view.is_optional,
        "the add-to-hand pick is mandatory when a Digimon is revealed"
    );
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("add the lone Digimon to hand");
    runner.auto_resolve().expect("resolve trash-the-rest");

    assert!(
        hand_ids(&runner, 0).contains(&"EX1066-DIGI-A".to_string()),
        "the Digimon is added to hand: {:?}",
        hand_ids(&runner, 0)
    );
    let trash = trash_ids(&runner, 0);
    assert!(
        trash.contains(&"EX1066-TAMER-B".to_string())
            && trash.contains(&"EX1066-OPT-C".to_string()),
        "the two non-Digimon reveals are trashed: {trash:?}"
    );
    assert_eq!(
        runner.trash_size(0),
        trash_before + 2,
        "exactly the two non-added reveals leave to trash"
    );
    assert_eq!(
        runner.hand_size(0),
        hand_before + 1,
        "hand gains exactly the picked Digimon"
    );
}

/// Negative: none of the 3 revealed cards are Digimon → the pick fizzles
/// (no eligible candidate), all 3 are trashed, no card is added to hand.
#[test]
fn ex1_066_on_play_with_no_digimon_revealed_trashes_all_three() {
    let mut tamer_card = make_test_card("EX1066-TAMER-D", "Tamer D");
    tamer_card.card_kind = CardKind::Tamer;
    tamer_card.level = None;
    tamer_card.dp = None;
    tamer_card.play_cost = 0;

    let mut option_card_1 = make_test_card("EX1066-OPT-E", "Option E");
    option_card_1.card_kind = CardKind::Option;
    option_card_1.level = None;
    option_card_1.dp = None;
    option_card_1.play_cost = 0;

    let mut option_card_2 = make_test_card("EX1066-OPT-F", "Option F");
    option_card_2.card_kind = CardKind::Option;
    option_card_2.level = None;
    option_card_2.dp = None;
    option_card_2.play_cost = 0;

    let filler = make_test_card("EX1066-FILLER-B", "Filler B");

    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX1-066 loads")
        .add_card(tamer_card)
        .add_card(option_card_1)
        .add_card(option_card_2)
        .add_card(filler)
        .deck(0, &["EX1066-FILLER-B"])
        .security(1, &["EX1066-FILLER-B"])
        .memory(3)
        .start();

    stack_deck_top(
        &mut runner,
        &["EX1066-OPT-F", "EX1066-OPT-E", "EX1066-TAMER-D"],
    );

    runner.place_on_field(0, CARD_ID, Some(0));
    let hand_before_hand_ids = hand_ids(&runner, 0);
    let trash_before = runner.trash_size(0);

    runner.fire_play_event_triggers(0, 0, false, false);
    runner
        .auto_resolve()
        .expect("no eligible pick -> fizzle -> trash all 3");

    assert_eq!(
        runner.trash_size(0),
        trash_before + 3,
        "all 3 non-Digimon reveals are trashed when nothing is eligible"
    );
    let hand_after: Vec<String> = hand_ids(&runner, 0)
        .into_iter()
        .filter(|id| !hand_before_hand_ids.contains(id))
        .collect();
    assert!(
        hand_after.is_empty(),
        "no reveal is added to hand when none are Digimon: {hand_after:?}"
    );
}

// ─── Clause 2: [All Turns] Lv.5+-with-source deletion → suspend/memory/hatch

#[test]
fn ex1_066_has_all_turns_deletion_observer_clause() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX1-066 YAML parses and compiles")
        .build();
    let card = runner.compiled_card(CARD_ID).expect("EX1-066 compiled");
    let observer = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnAnyDeletion) => {
                Some(t)
            }
            _ => None,
        })
        .expect("EX1-066 must have an [All Turns] on_any_deletion clause");
    assert!(
        observer.optional,
        "[All Turns] deletion clause must be optional (the suspend cost is opt-in)"
    );
    assert!(
        observer.active_when.is_some(),
        "[All Turns] gate must be encoded via active_when"
    );
    assert!(
        observer.condition.is_some(),
        "deletion observer must gate on event context (owner + kind + level + has-source)"
    );
    fn process_contains_hatch(steps: &[CompiledStep]) -> bool {
        steps.iter().any(|s| match s {
            CompiledStep::Hatch { .. } => true,
            CompiledStep::If {
                then, else_branch, ..
            } => process_contains_hatch(then) || process_contains_hatch(else_branch),
            CompiledStep::PerSelected { body, .. } | CompiledStep::ForEach { body, .. } => {
                process_contains_hatch(body)
            }
            _ => false,
        })
    }
    assert!(
        process_contains_hatch(&observer.process),
        "clause body must include a hatch step"
    );
}

fn setup_deletion_runner(egg_id: &str) -> DebugRunner {
    let mut lv5_source = make_test_card("EX1066-LV5-SRC", "Lv5 Source");
    lv5_source.card_kind = CardKind::Digimon;
    lv5_source.level = Some(4);
    lv5_source.dp = Some(4000);
    lv5_source.play_cost = 0;

    let mut lv5_top = make_test_card("EX1066-LV5-TOP", "Lv5 Top");
    lv5_top.card_kind = CardKind::Digimon;
    lv5_top.level = Some(5);
    lv5_top.dp = Some(8000);
    lv5_top.play_cost = 0;

    let filler = make_test_card("EX1066-FILLER-DEL", "Filler Del");
    let mut egg = make_test_card(egg_id, egg_id);
    egg.card_kind = CardKind::DigiEgg;
    egg.level = Some(2);
    egg.play_cost = 0;

    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX1-066 loads")
        .add_card(lv5_source)
        .add_card(lv5_top)
        .add_card(filler)
        .add_card(egg)
        .deck(0, &["EX1066-FILLER-DEL"])
        .security(1, &["EX1066-FILLER-DEL"])
        .digitama(0, &[egg_id])
        .start()
}

/// Positive: own Lv.5 Digimon WITH a digivolution source is deleted -> the
/// suspend prompt is offered; accepting suspends the Tamer, gains 1 memory,
/// and hatches (breeding area was empty).
#[test]
fn ex1_066_all_turns_offers_suspend_when_own_lv5_with_source_deleted_and_accepting_hatches() {
    let mut runner = setup_deletion_runner("EX1066-EGG-A");
    let analog_youth = runner.place_on_field(0, CARD_ID, Some(0));
    let target = runner.place_stack(0, &["EX1066-LV5-SRC", "EX1066-LV5-TOP"]);

    let memory_before = runner.memory();

    runner
        .game
        .delete_permanent_with_cause(target, ReplacementCause::OwnEffect);

    let view = runner
        .pending_selection_view()
        .expect("deletion of own Lv.5-with-source Digimon must offer the suspend choice");
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("accept the suspend cost");
    runner.auto_resolve().expect("finish resolving");

    assert!(
        runner.game.players[0].battle_area[analog_youth.index as usize].is_suspended,
        "accepting suspends this Tamer"
    );
    assert_eq!(
        runner.memory(),
        memory_before + 1,
        "accepting gains 1 memory"
    );
    assert!(
        runner.game.players[0].breeding_area.is_some(),
        "breeding area was empty -> hatch succeeds"
    );
}

/// Positive-but-occupied: breeding area is already occupied -> the suspend +
/// memory gain still happen (Q&A: you may suspend even if you can't hatch),
/// but no additional hatch occurs (breeding area stays with its original
/// occupant).
#[test]
fn ex1_066_all_turns_suspend_and_memory_succeed_even_when_hatch_is_impossible() {
    let mut runner = setup_deletion_runner("EX1066-EGG-B");
    runner.place_on_field(0, CARD_ID, Some(0));
    let target = runner.place_stack(0, &["EX1066-LV5-SRC", "EX1066-LV5-TOP"]);

    // Occupy the breeding area BEFORE the deletion so CanHatch is false.
    let occupant_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "EX1066-FILLER-DEL")
        .expect("filler registered");
    let occupant_card = CardSource::new(occupant_idx, 0, runner.game.next_card_index());
    runner.game.players[0].breeding_area =
        Some(Permanent::new(occupant_card, runner.game.turn_count));

    let memory_before = runner.memory();
    let digitama_before = runner.game.players[0].digitama_deck.len();

    runner
        .game
        .delete_permanent_with_cause(target, ReplacementCause::OwnEffect);

    let view = runner
        .pending_selection_view()
        .expect("deletion must offer the suspend choice even though hatch is impossible");
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("accept the suspend cost");
    runner.auto_resolve().expect("finish resolving");

    assert_eq!(
        runner.memory(),
        memory_before + 1,
        "memory gain still happens per the official Q&A"
    );
    assert_eq!(
        runner.game.players[0].digitama_deck.len(),
        digitama_before,
        "no hatch occurs -- the digitama deck is untouched (breeding area occupied)"
    );
}

/// Negative: own level-4 Digimon (below the Lv.5 threshold) with a source is
/// deleted -> Clause 2 must NOT fire.
#[test]
fn ex1_066_all_turns_does_not_fire_for_own_lv4_with_source_deletion() {
    let mut lv4_source = make_test_card("EX1066-LV4-SRC", "Lv4 Source");
    lv4_source.card_kind = CardKind::Digimon;
    lv4_source.level = Some(3);
    lv4_source.dp = Some(2000);
    lv4_source.play_cost = 0;

    let mut lv4_top = make_test_card("EX1066-LV4-TOP", "Lv4 Top");
    lv4_top.card_kind = CardKind::Digimon;
    lv4_top.level = Some(4);
    lv4_top.dp = Some(5000);
    lv4_top.play_cost = 0;

    let filler = make_test_card("EX1066-FILLER-LV4", "Filler Lv4");

    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX1-066 loads")
        .add_card(lv4_source)
        .add_card(lv4_top)
        .add_card(filler)
        .deck(0, &["EX1066-FILLER-LV4"])
        .security(1, &["EX1066-FILLER-LV4"])
        .start();

    runner.place_on_field(0, CARD_ID, Some(0));
    let target = runner.place_stack(0, &["EX1066-LV4-SRC", "EX1066-LV4-TOP"]);
    let memory_before = runner.memory();

    runner
        .game
        .delete_permanent_with_cause(target, ReplacementCause::OwnEffect);

    assert!(
        runner.pending_selection().is_none(),
        "Lv.4 deletion (below the Lv.5 threshold) must NOT offer the suspend choice"
    );
    assert_eq!(
        runner.memory(),
        memory_before,
        "no memory gain for a sub-Lv.5 deletion"
    );
}

/// Negative: own Lv.5+ Digimon deleted WITHOUT any digivolution source (a
/// bare Lv.5+ permanent with no sources under it) must NOT fire Clause 2.
#[test]
fn ex1_066_all_turns_does_not_fire_for_own_lv5_without_digivolution_card() {
    let mut lv5_bare = make_test_card("EX1066-LV5-BARE", "Lv5 Bare");
    lv5_bare.card_kind = CardKind::Digimon;
    lv5_bare.level = Some(5);
    lv5_bare.dp = Some(9000);
    lv5_bare.play_cost = 0;

    let filler = make_test_card("EX1066-FILLER-BARE", "Filler Bare");

    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX1-066 loads")
        .add_card(lv5_bare)
        .add_card(filler)
        .deck(0, &["EX1066-FILLER-BARE"])
        .security(1, &["EX1066-FILLER-BARE"])
        .start();

    runner.place_on_field(0, CARD_ID, Some(0));
    let target = runner.place_on_field(0, "EX1066-LV5-BARE", Some(0));
    let memory_before = runner.memory();

    runner
        .game
        .delete_permanent_with_cause(target, ReplacementCause::OwnEffect);

    assert!(
        runner.pending_selection().is_none(),
        "a Lv.5+ Digimon with NO digivolution card must NOT offer the suspend choice"
    );
    assert_eq!(
        runner.memory(),
        memory_before,
        "no memory gain when the deleted Digimon has no digivolution card"
    );
}

/// Negative: opponent's Lv.5+ Digimon-with-source is deleted -> Clause 2 is
/// scoped to the controller's OWN Digimon only ("one of YOUR ... Digimon").
#[test]
fn ex1_066_all_turns_does_not_fire_for_opponent_digimon_deletion() {
    let mut lv5_source = make_test_card("EX1066-OPP-LV5-SRC", "Opp Lv5 Source");
    lv5_source.card_kind = CardKind::Digimon;
    lv5_source.level = Some(4);
    lv5_source.dp = Some(4000);
    lv5_source.play_cost = 0;

    let mut lv5_top = make_test_card("EX1066-OPP-LV5-TOP", "Opp Lv5 Top");
    lv5_top.card_kind = CardKind::Digimon;
    lv5_top.level = Some(5);
    lv5_top.dp = Some(8000);
    lv5_top.play_cost = 0;

    let filler = make_test_card("EX1066-FILLER-OPP", "Filler Opp");

    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX1-066 loads")
        .add_card(lv5_source)
        .add_card(lv5_top)
        .add_card(filler)
        .deck(0, &["EX1066-FILLER-OPP"])
        .deck(1, &["EX1066-FILLER-OPP"])
        .start();

    runner.place_on_field(0, CARD_ID, Some(0));
    let target = runner.place_stack(1, &["EX1066-OPP-LV5-SRC", "EX1066-OPP-LV5-TOP"]);
    let memory_before = runner.memory();

    runner
        .game
        .delete_permanent_with_cause(target, ReplacementCause::OwnEffect);

    assert!(
        runner.pending_selection().is_none(),
        "opponent's Lv.5+-with-source deletion must NOT trigger the controller's observer"
    );
    assert_eq!(
        runner.memory(),
        memory_before,
        "no memory gain from an opponent's Digimon deletion"
    );
}

/// Decline test: player declines the suspend cost -- Tamer stays unsuspended,
/// no memory gain, no hatch.
#[test]
fn ex1_066_all_turns_player_may_decline_suspend_cost() {
    let mut runner = setup_deletion_runner("EX1066-EGG-C");
    let analog_youth = runner.place_on_field(0, CARD_ID, Some(0));
    let target = runner.place_stack(0, &["EX1066-LV5-SRC", "EX1066-LV5-TOP"]);

    let memory_before = runner.memory();
    let digitama_before = runner.game.players[0].digitama_deck.len();

    runner
        .game
        .delete_permanent_with_cause(target, ReplacementCause::OwnEffect);

    runner.pending_selection_view().expect("must offer choice");
    runner.decline_optional_trigger().expect("decline");
    runner.auto_resolve().expect("resolve after decline");

    assert!(
        !runner.game.players[0].battle_area[analog_youth.index as usize].is_suspended,
        "declining must leave this Tamer unsuspended"
    );
    assert_eq!(
        runner.memory(),
        memory_before,
        "declining must not gain memory"
    );
    assert_eq!(
        runner.game.players[0].digitama_deck.len(),
        digitama_before,
        "declining must not hatch"
    );
}

/// Negative: the Tamer itself is already suspended when the deletion occurs
/// -> the (unpayable) suspend cost must not be offered.
#[test]
fn ex1_066_all_turns_does_not_fire_when_source_already_suspended() {
    let mut runner = setup_deletion_runner("EX1066-EGG-D");
    let analog_youth = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.players[0].battle_area[analog_youth.index as usize].is_suspended = true;
    let target = runner.place_stack(0, &["EX1066-LV5-SRC", "EX1066-LV5-TOP"]);

    let memory_before = runner.memory();

    runner
        .game
        .delete_permanent_with_cause(target, ReplacementCause::OwnEffect);

    assert!(
        runner.pending_selection().is_none(),
        "an already-suspended Analog Youth must not offer the (unpayable) suspend cost"
    );
    assert_eq!(
        runner.memory(),
        memory_before,
        "no memory gain when the cost cannot be paid"
    );
}
