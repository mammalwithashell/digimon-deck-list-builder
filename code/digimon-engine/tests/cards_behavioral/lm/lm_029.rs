//! LM-029 Yellow Scramble - Option, Cost 2, Yellow.
//!
//! # Card text (cards.json)
//!
//! [Main] 1 of your yellow Digimon may digivolve into a yellow Digimon card in
//! the hand with the digivolution cost reduced by 3. Then, place this card in
//! the battle area.
//!
//! [Start of Your Turn] If your opponent has a Digimon, <Delay> (By trashing
//! this card after the placing turn, activate the effect below.)
//! Return 1 yellow Digimon card from your trash to the top of the deck. Then,
//! if you don't have a Digimon, you may play 1 yellow Digimon card with 2000 DP
//! or less from your trash without paying the cost.
//!
//! Inherited: Security Effect [Security] You may play 1 yellow Digimon card
//! with 2000 DP or less from your trash without paying the cost. Then, add this
//! card to the hand.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/LM/Yellow/LM_029.cs
//!
//! # Patterns this test covers
//! - Clause A (Main): effect_initiated_digivolve with cost reduction 3 from an
//!   Option card (main_from_hand timing, optional permanent + hand selections).
//! - Clause C (Security, inherited): on_security with select_trash (yellow
//!   Digimon DP <= 2000), play_from_trash_free, mandatory add_this_option_to_hand
//!   tail. Positive (a valid <=2000 DP candidate) and negative (an over-DP
//!   candidate excluded) coverage of the DP filter.
//! - PendingSelection/action-mask visibility for field and hand choices.
//!
//! # Closed gaps
//! - **G-ZONE-SELECTED-TRASH-TO-DECK-TOP**: the DSL `move_trash_card_to_deck_top`
//!   verb moves a single selected trash card to the deck top (Vec end = drawn
//!   first). The [Start of Your Turn] Delay clause uses it; adding the
//!   `kind: delay` clause also makes the engine auto-seat LM-029 in the battle
//!   area as a Delay Option, so the Main clause's "place this card in the
//!   battle area" sub-step works.
//!
//! # Resolved gaps now exercised
//! - **G-PRED-DP-LTE** (Track A, 2026-05-17): `dp_lte` is now evaluated for
//!   card-zone (trash) subjects via `eval_card_fields`. The Security clause's
//!   `dp_lte: 2000` predicate is enforced at selection time.
//! - **G-OPTIONAL-SELECTION-CONTINUE-TAIL** (Track H, 2026-05-17): declining the
//!   optional `select_trash` play still runs the mandatory add-this-option-to-hand
//!   tail.

#![allow(unused_imports, dead_code)]

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledDeclarativeClause, CompiledScope,
    CompiledStep, CompiledTiming,
};
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::selection::SelectionKind;

const YAML: &str = include_str!("../../../cards/lm/LM-029.yaml");

fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

fn digimon(id: &str, color: CardColor, level: u8, dp: i32, play_cost: u16) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![color];
    card.level = Some(level);
    card.dp = Some(dp);
    card.play_cost = play_cost;
    card
}

fn yellow_digimon(id: &str, level: u8, dp: i32, play_cost: u16) -> CardData {
    digimon(id, CardColor::Yellow, level, dp, play_cost)
}

/// A yellow Digimon with DP <= 2000 — eligible for the Security trash play.
fn yellow_small(id: &str) -> CardData {
    yellow_digimon(id, 3, 2000, 3)
}

/// A yellow Digimon with DP > 2000 — excluded by the `dp_lte: 2000` filter.
fn yellow_large(id: &str) -> CardData {
    yellow_digimon(id, 5, 8000, 8)
}

fn yellow_evo(id: &str) -> CardData {
    let mut card = yellow_digimon(id, 4, 5000, 5);
    card.evo_costs = vec![EvoCost {
        card_color: CardColor::Yellow as u8,
        level: 3,
        memory_cost: 4,
    }];
    card
}

fn lm_029_runner() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-029 YAML must parse and compile")
        .memory(10)
        .start()
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn lm_029_yaml_parses_and_compiles() {
    let _runner = lm_029_runner();
}

#[test]
fn lm_029_is_yellow_option_cost_2() {
    let runner = lm_029_runner();
    let compiled = runner.compiled_card("LM-029").expect("LM-029 compiled");

    assert_eq!(compiled.kind, CompiledCardKind::Option);
    assert_eq!(compiled.color, vec![CompiledColor::Yellow]);
    assert_eq!(compiled.cost, Some(2));
}

/// LM-029 ships three clauses: Main (main_from_hand), Delay
/// (start_of_your_turn), and Security (on_security inherited).
#[test]
fn lm_029_has_main_delay_and_security_clauses_without_raw_rust() {
    let runner = lm_029_runner();
    let compiled = runner.compiled_card("LM-029").expect("LM-029 compiled");

    assert_eq!(
        compiled.effects.len(),
        3,
        "LM-029 ships the Main, Delay, and Security clauses"
    );
    assert!(
        compiled.effects.iter().all(|clause| !matches!(
            clause,
            CompiledClause::Declarative(CompiledDeclarativeClause::RawRust { .. })
        )),
        "LM-029 must not use raw_rust placeholders"
    );

    let delay = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Declarative(CompiledDeclarativeClause::Delay {
                scope, trigger, ..
            }) => Some((scope, trigger)),
            _ => None,
        })
        .expect("Delay clause must exist");
    assert_eq!(*delay.0, CompiledScope::FaceUp);
    assert_eq!(
        *delay.1,
        CompiledTiming::StartOfYourTurn,
        "the Scramble Delay is a start-of-your-turn Delay"
    );

    let main = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::MainFromHand) => {
                Some(t)
            }
            _ => None,
        })
        .expect("Main clause must exist");
    assert_eq!(main.scope, CompiledScope::FaceUp);
    assert!(
        main.optional,
        "printed Main text says the yellow Digimon may digivolve"
    );

    let security = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSecurity) => Some(t),
            _ => None,
        })
        .expect("Security clause must exist");
    assert_eq!(security.scope, CompiledScope::Inherited);
    assert!(
        security.optional,
        "printed Security text says 'you may' — the clause is optional"
    );
}

/// Clause A contains an EffectInitiatedDigivolve step.
#[test]
fn lm_029_main_clause_contains_effect_initiated_digivolve_step() {
    let runner = lm_029_runner();
    let compiled = runner.compiled_card("LM-029").expect("LM-029 compiled");

    let main = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::MainFromHand) => {
                Some(t)
            }
            _ => None,
        })
        .expect("Main clause must exist");

    assert!(
        main.process
            .iter()
            .any(|step| matches!(step, CompiledStep::EffectInitiatedDigivolve { .. })),
        "Main clause must contain EffectInitiatedDigivolve step"
    );
}

/// Clause C (Security) contains both the optional trash play and the mandatory
/// add-this-option-to-hand tail.
#[test]
fn lm_029_security_clause_contains_trash_play_and_add_to_hand_steps() {
    let runner = lm_029_runner();
    let compiled = runner.compiled_card("LM-029").expect("LM-029 compiled");

    let security = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSecurity) => Some(t),
            _ => None,
        })
        .expect("Security clause must exist");

    assert!(
        security
            .process
            .iter()
            .any(|step| matches!(step, CompiledStep::PlayFromTrashFree { .. })),
        "Security clause must contain a PlayFromTrashFree step for the optional trash play"
    );
    assert!(
        security
            .process
            .iter()
            .any(|step| matches!(step, CompiledStep::AddThisOptionToHand { .. })),
        "Security clause must contain the mandatory AddThisOptionToHand tail"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Clause A: [Main] digivolve with cost -3
// ═══════════════════════════════════════════════════════════════════════════════

/// Condition gating (positive): with a yellow Digimon on the field and a yellow
/// Digimon in hand, the Main effect installs the field-selection prompt.
#[test]
fn lm_029_main_exposes_yellow_field_then_yellow_hand_choices_in_mask() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-029 YAML parses")
        .add_card(yellow_digimon("YELLOW-BASE", 3, 2000, 3))
        .add_card(digimon("BLUE-BASE", CardColor::Blue, 3, 2000, 3))
        .add_card(yellow_evo("YELLOW-EVO"))
        .add_card(digimon("BLUE-HAND", CardColor::Blue, 4, 5000, 5))
        .add_card(filler("FILL"))
        .hand(0, &["LM-029", "YELLOW-EVO", "BLUE-HAND"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .memory(10)
        .start();

    let base = runner.place_on_field(0, "YELLOW-BASE", Some(0));
    runner.place_on_field(0, "BLUE-BASE", Some(0));

    assert!(runner.game.activate_hand_main(0, 0));

    let field_view = runner
        .pending_selection_view()
        .expect("Main effect must ask which yellow Digimon digivolves");
    assert_eq!(field_view.kind, SelectionKind::OwnField);
    assert!(
        runner.pending_is_optional(),
        "field selection must expose PASS for the printed may"
    );
    assert_eq!(
        field_view.valid_action_ids.len(),
        1,
        "only the yellow field Digimon should be selectable"
    );
    let mask = build_action_mask(&runner.game, field_view.selecting_player);
    assert_eq!(mask[field_view.valid_action_ids[0] as usize], 1.0);
    runner
        .execute_action(field_view.selecting_player, field_view.valid_action_ids[0])
        .expect("select yellow field Digimon");

    let hand_view = runner
        .pending_selection_view()
        .expect("Main effect must ask which yellow hand card to digivolve into");
    assert_eq!(hand_view.kind, SelectionKind::Hand);
    assert_eq!(
        hand_view.valid_action_ids.len(),
        1,
        "only the yellow hand Digimon should be selectable"
    );
    let mask = build_action_mask(&runner.game, hand_view.selecting_player);
    assert_eq!(mask[hand_view.valid_action_ids[0] as usize], 1.0);
    runner
        .execute_action(hand_view.selecting_player, hand_view.valid_action_ids[0])
        .expect("select yellow hand Digimon");
    runner.auto_resolve().expect("finish digivolve");

    assert_eq!(
        runner.memory(),
        9,
        "evo cost 4 reduced by 3 should pay 1 memory"
    );
    assert!(
        runner
            .game
            .player(0)
            .hand
            .iter()
            .all(|card| card.card_id(&runner.game.card_data) != "YELLOW-EVO"),
        "selected yellow evo card must leave hand"
    );
    let evolved = &runner.game.player(0).battle_area[base.index as usize];
    assert_eq!(
        evolved.top_card().card_id(&runner.game.card_data),
        "YELLOW-EVO"
    );
}

/// Condition gating (negative): declining the optional Main digivolve leaves the
/// field and hand unchanged.
#[test]
fn lm_029_main_decline_leaves_field_and_hand_unchanged() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-029 YAML parses")
        .add_card(yellow_digimon("YELLOW-BASE-DECLINE", 3, 2000, 3))
        .add_card(yellow_evo("YELLOW-EVO-DECLINE"))
        .add_card(filler("FILL"))
        .hand(0, &["LM-029", "YELLOW-EVO-DECLINE"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .memory(10)
        .start();

    runner.place_on_field(0, "YELLOW-BASE-DECLINE", Some(0));
    let hand_before = runner.hand_size(0);
    let stack_before = runner.game.player(0).battle_area[0].card_sources.len();

    assert!(runner.game.activate_hand_main(0, 0));
    assert!(runner.pending_is_optional());
    runner
        .execute_action(0, digimon_engine::action::space::PASS)
        .expect("decline optional Main digivolve");

    assert_eq!(runner.hand_size(0), hand_before);
    assert_eq!(
        runner.game.player(0).battle_area[0].card_sources.len(),
        stack_before
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Clause C: [Security] (inherited) optional yellow DP<=2000 trash
// play, then mandatory add-this-option-to-hand
// ═══════════════════════════════════════════════════════════════════════════════

/// Positive DP filter: when the defender's trash holds a yellow Digimon with
/// DP <= 2000, the [Security] clause exposes the optional trash play, plays the
/// small yellow Digimon when chosen, and then runs the mandatory tail that adds
/// LM-029 to the defender's hand.
#[test]
fn lm_029_security_plays_small_yellow_digimon_from_trash_then_adds_to_hand() {
    let mut attacker = filler("ATTACKER");
    attacker.card_kind = CardKind::Digimon;
    attacker.colors = vec![CardColor::Red];
    attacker.level = Some(4);
    attacker.dp = Some(6000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-029 YAML parses")
        .add_card(attacker)
        .add_card(yellow_small("YELLOW-SMALL"))
        .add_card(filler("FILL"))
        .security(1, &["LM-029"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["YELLOW-SMALL"])
        .memory(0)
        .start();

    // Seed the defender's trash with a yellow Digimon DP <= 2000.
    let small = runner.game.players[1]
        .deck
        .pop()
        .expect("small yellow seed in deck");
    runner.game.players[1].trash.push(small);

    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));
    assert_eq!(runner.hand_size(1), 0, "precondition: defender hand empty");
    assert_eq!(runner.security_count(1), 1, "precondition: LM-029 in security");

    let _ = runner.attack_player(attacker, 1, false);
    runner.auto_resolve().expect("security selections resolve");

    // The optional trash play happened: the small yellow Digimon is on the
    // defender's field.
    assert_eq!(
        runner.battle_area_size(1),
        1,
        "small yellow Digimon was played from trash"
    );
    let played_id = runner.game.players[1].battle_area[0].card_sources[0]
        .card_id(&runner.game.card_data)
        .to_string();
    assert_eq!(played_id, "YELLOW-SMALL");
    assert_eq!(
        runner.trash_size(1),
        0,
        "small yellow Digimon left the trash when played"
    );

    // The mandatory tail must always run: LM-029 left security and went to hand.
    assert_eq!(runner.security_count(1), 0, "LM-029 left the security stack");
    assert_eq!(
        runner.hand_size(1),
        1,
        "LM-029 must be added to the defender's hand by the mandatory tail"
    );
    let hand_id = runner.game.players[1].hand[0]
        .card_id(&runner.game.card_data)
        .to_string();
    assert_eq!(hand_id, "LM-029", "the card added to hand must be LM-029 itself");
}

/// Negative DP filter: when the defender's trash contains ONLY a yellow Digimon
/// with DP > 2000, the `dp_lte: 2000` predicate rejects it — nothing is played
/// from the trash, while the mandatory add-this-option-to-hand tail still runs.
#[test]
fn lm_029_security_does_not_play_over_dp_yellow_digimon_from_trash() {
    let mut attacker = filler("ATTACKER-LARGE");
    attacker.card_kind = CardKind::Digimon;
    attacker.colors = vec![CardColor::Red];
    attacker.level = Some(4);
    attacker.dp = Some(6000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-029 YAML parses")
        .add_card(attacker)
        .add_card(yellow_large("YELLOW-LARGE"))
        .add_card(filler("FILL"))
        .security(1, &["LM-029"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["YELLOW-LARGE"])
        .memory(0)
        .start();

    // Seed the defender's trash with a yellow Digimon DP > 2000.
    let large = runner.game.players[1]
        .deck
        .pop()
        .expect("large yellow seed in deck");
    runner.game.players[1].trash.push(large);

    let attacker = runner.place_on_field(0, "ATTACKER-LARGE", Some(0));
    assert_eq!(runner.security_count(1), 1, "precondition: LM-029 in security");

    let _ = runner.attack_player(attacker, 1, false);
    runner.auto_resolve().expect("security selections resolve");

    // The DP filter must reject the >2000 DP Digimon: nothing played.
    assert_eq!(
        runner.battle_area_size(1),
        0,
        "LM-029 Security must not play a >2000 DP Digimon from trash (G-PRED-DP-LTE)"
    );
    assert_eq!(
        runner.trash_size(1),
        1,
        "the large yellow Digimon must remain in the defender's trash"
    );

    // The mandatory tail must always run: LM-029 added to the defender's hand.
    assert_eq!(runner.security_count(1), 0, "LM-029 left the security stack");
    assert_eq!(
        runner.hand_size(1),
        1,
        "LM-029's mandatory tail must add it to the defender's hand"
    );
    let hand_id = runner.game.players[1].hand[0]
        .card_id(&runner.game.card_data)
        .to_string();
    assert_eq!(hand_id, "LM-029", "the card added to hand must be LM-029 itself");
}

/// Non-yellow trash candidates are excluded by the `color_is: yellow` filter,
/// and the Security clause produces no panic when the trash mixes colors.
#[test]
fn lm_029_security_excludes_non_yellow_trash_candidates() {
    let mut attacker = filler("ATTACKER-MIX");
    attacker.card_kind = CardKind::Digimon;
    attacker.colors = vec![CardColor::Red];
    attacker.level = Some(4);
    attacker.dp = Some(6000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-029 YAML parses")
        .add_card(attacker)
        .add_card(digimon("BLUE-SMALL", CardColor::Blue, 3, 2000, 3))
        .add_card(filler("FILL"))
        .security(1, &["LM-029"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["BLUE-SMALL"])
        .memory(0)
        .start();

    // Seed the defender's trash with a NON-yellow Digimon DP <= 2000.
    let blue = runner.game.players[1]
        .deck
        .pop()
        .expect("blue seed in deck");
    runner.game.players[1].trash.push(blue);

    let attacker = runner.place_on_field(0, "ATTACKER-MIX", Some(0));

    let _ = runner.attack_player(attacker, 1, false);
    runner.auto_resolve().expect("security selections resolve");

    // The blue Digimon is not yellow → not an eligible candidate, nothing played.
    assert_eq!(
        runner.battle_area_size(1),
        0,
        "a non-yellow Digimon must not be played from trash"
    );
    assert!(
        runner
            .game
            .player(1)
            .trash
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "BLUE-SMALL"),
        "non-yellow trash card must remain in trash"
    );

    // The mandatory tail still runs.
    assert_eq!(runner.security_count(1), 0, "LM-029 left the security stack");
    assert_eq!(
        runner.hand_size(1),
        1,
        "LM-029's mandatory tail must add it to the defender's hand"
    );
}

/// Empty trash: the optional trash play has no candidate (nothing played), but
/// the mandatory add-this-option-to-hand tail still fires.
#[test]
fn lm_029_security_adds_this_option_to_hand_with_empty_trash() {
    let mut attacker = filler("ATTACKER-EMPTY");
    attacker.card_kind = CardKind::Digimon;
    attacker.colors = vec![CardColor::Red];
    attacker.level = Some(4);
    attacker.dp = Some(6000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-029 YAML parses")
        .add_card(attacker)
        .add_card(filler("FILL"))
        .security(1, &["LM-029"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .memory(0)
        .start();

    let attacker = runner.place_on_field(0, "ATTACKER-EMPTY", Some(0));
    assert_eq!(runner.trash_size(1), 0, "precondition: defender trash empty");
    assert_eq!(runner.security_count(1), 1, "precondition: LM-029 in security");

    let result = runner.attack_player(attacker, 1, false);
    runner.auto_resolve().expect("security selections resolve");

    assert_eq!(result, AttackResult::SecurityCheckSurvived);
    assert_eq!(
        runner.battle_area_size(1),
        0,
        "no Digimon played from an empty trash"
    );
    assert_eq!(runner.security_count(1), 0, "LM-029 left security");
    assert!(
        runner
            .game
            .player(1)
            .hand
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "LM-029"),
        "LM-029 must be added to hand even with an empty trash"
    );
}

/// Optionality: declining the optional trash play leaves the eligible yellow
/// Digimon in trash, while the mandatory add-this-option-to-hand tail still
/// fires (G-OPTIONAL-SELECTION-CONTINUE-TAIL — closed by Phase 2 Track H).
#[test]
fn lm_029_security_decline_trash_play_still_adds_this_option_to_hand() {
    let mut attacker = filler("ATTACKER-DECLINE");
    attacker.card_kind = CardKind::Digimon;
    attacker.colors = vec![CardColor::Red];
    attacker.level = Some(4);
    attacker.dp = Some(6000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-029 YAML parses")
        .add_card(attacker)
        .add_card(yellow_small("YELLOW-SMALL-DECLINE"))
        .add_card(filler("FILL"))
        .security(1, &["LM-029"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["YELLOW-SMALL-DECLINE"])
        .memory(0)
        .start();

    // Seed the defender's trash with an eligible yellow Digimon DP <= 2000.
    let small = runner.game.players[1]
        .deck
        .pop()
        .expect("small yellow seed in deck");
    runner.game.players[1].trash.push(small);

    let attacker = runner.place_on_field(0, "ATTACKER-DECLINE", Some(0));

    let _ = runner.attack_player(attacker, 1, false);

    // The optional trash selection must be exposed (no auto-pick) — decline it.
    let view = runner
        .pending_selection_view()
        .expect("optional Security trash play must surface a pending selection");
    assert_eq!(view.kind, SelectionKind::Trash);
    assert!(
        runner.pending_is_optional(),
        "the printed 'you may' must expose PASS on the trash selection"
    );
    runner
        .execute_action(view.selecting_player, digimon_engine::action::space::PASS)
        .expect("decline optional Security trash play");
    runner.auto_resolve().expect("finish security effect");

    // Declining leaves the yellow Digimon in trash, nothing played.
    assert_eq!(
        runner.battle_area_size(1),
        0,
        "declining the optional play must not play a Digimon"
    );
    assert_eq!(
        runner.trash_size(1),
        1,
        "declined yellow Digimon remains in trash"
    );

    // The mandatory tail still runs.
    assert_eq!(runner.security_count(1), 0, "LM-029 left security");
    assert!(
        runner
            .game
            .player(1)
            .hand
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "LM-029"),
        "LM-029 is still added to hand after declining the optional play"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — Clause B: [Start of Your Turn] Delay
// ═══════════════════════════════════════════════════════════════════════════════

/// Push a card_id into a player's trash by registry lookup. Mirrors the
/// trash-seeding idiom used by the Security tests above.
fn push_to_trash(runner: &mut DebugRunner, player: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("push_to_trash: unknown card_id {card_id}"));
    let card_index = runner.game.next_card_index();
    runner.game.players[player as usize].trash.push(
        digimon_engine::card_source::CardSource::new(data_idx, player, card_index),
    );
}

/// Seat LM-029 (at player 0's `hand[0]`) as a Delay Option through the real
/// Option pipeline. With no yellow digivolve base on the field the Main clause
/// has nothing to do, so the Option disposes straight to a Delay permanent in
/// the battle area. The caller must first satisfy LM-029's yellow color
/// requirement (a yellow permanent in battle or breeding).
fn seat_lm_029_as_delay_option(runner: &mut DebugRunner) {
    runner.game.enter_main_phase();
    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        digimon_engine::selection::OptionPlayResult::Trashed,
        "LM-029 disposes straight to a Delay Option permanent on Main resolution"
    );
}

fn lm_029_is_delayed_option(runner: &DebugRunner) -> bool {
    runner
        .game
        .player(0)
        .battle_area
        .iter()
        .any(|perm| {
            perm.top_card().card_id(&runner.game.card_data) == "LM-029"
                && matches!(perm.option_state, digimon_engine::permanent::OptionState::Delayed { .. })
        })
}

/// [Main] "Then, place this card in the battle area." Adding a `kind: delay`
/// clause makes the engine classify LM-029 as a Delay Option, so the Option
/// pipeline auto-seats it in the battle area on resolution — no explicit
/// placement step needed (same shape as BT22-098 / LM-054).
#[test]
fn lm_029_main_places_this_card_in_battle_area_after_resolution() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-029 YAML parses")
        .add_card(yellow_digimon("YELLOW-ANCHOR", 3, 2000, 3))
        .add_card(filler("FILL"))
        .hand(0, &["LM-029"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .memory(10)
        .start();

    // A yellow Digimon in breeding satisfies LM-029's color requirement
    // without putting a Digimon on the battle area.
    runner.place_in_breeding(0, "YELLOW-ANCHOR");

    seat_lm_029_as_delay_option(&mut runner);

    assert!(
        lm_029_is_delayed_option(&runner),
        "LM-029 must be seated in the battle area as a Delay Option after the Main clause resolves"
    );
    assert_eq!(
        runner.battle_area_size(0),
        1,
        "exactly LM-029 should be on the battle area"
    );
}

/// [Start of Your Turn] If your opponent has a Digimon, <Delay>: trash this
/// card, then return 1 yellow Digimon from your trash to the TOP of the deck.
/// The delay fires automatically at the start of the owner's next turn. Because
/// the card is returned to the deck TOP (Vec end, drawn first), that turn's
/// Draw phase draws it straight back into hand — the observable proof of
/// `move_trash_card_to_deck_top` versus the deck-bottom verb.
#[test]
fn lm_029_delay_fires_at_start_of_your_turn_and_returns_yellow_trash_to_deck_top() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-029 YAML parses")
        .add_card(yellow_digimon("YELLOW-TRASH", 3, 2000, 3))
        .add_card(yellow_digimon("YELLOW-ANCHOR", 3, 2000, 3))
        .add_card(digimon("OPP-DIGIMON", CardColor::Red, 4, 5000, 5))
        .add_card(filler("FILL"))
        .hand(0, &["LM-029"])
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .memory(10)
        .start();

    // Opponent must control a Digimon for the Delay's active_when gate.
    runner.place_on_field(1, "OPP-DIGIMON", Some(0));
    // A yellow Digimon in breeding satisfies LM-029's color requirement.
    runner.place_in_breeding(0, "YELLOW-ANCHOR");
    // A yellow Digimon waits in player 0's trash.
    push_to_trash(&mut runner, 0, "YELLOW-TRASH");

    seat_lm_029_as_delay_option(&mut runner);
    assert!(lm_029_is_delayed_option(&runner));

    // P0 ends turn -> P1's turn -> P1 ends turn -> begin_turn for P0 fires
    // the StartOfYourNextTurn Delay.
    runner.end_turn();
    runner.end_turn();
    assert_eq!(runner.game.turn_player(), 0, "back on player 0's turn");

    // Step (a) is a mandatory trash pick of a yellow Digimon.
    let view = runner
        .pending_selection_view()
        .expect("Delay must prompt a mandatory yellow-Digimon trash selection");
    assert_eq!(view.kind, SelectionKind::Trash);
    assert!(
        !runner.pending_is_optional(),
        "returning a yellow Digimon to deck top is mandatory per printed text"
    );
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only the yellow Digimon in trash should be selectable"
    );
    let mask = build_action_mask(&runner.game, view.selecting_player);
    assert_eq!(mask[view.valid_action_ids[0] as usize], 1.0);
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("return the yellow Digimon to the deck top");
    runner.auto_resolve().expect("finish the Delay body");

    // LM-029 was trashed as the Delay cost.
    assert!(
        !lm_029_is_delayed_option(&runner),
        "LM-029 must leave the battle area — trashed as the Delay activation cost"
    );
    assert!(
        runner
            .game
            .player(0)
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "LM-029"),
        "LM-029 must be in the trash after paying the Delay cost"
    );

    // The yellow Digimon left the trash.
    assert!(
        runner
            .game
            .player(0)
            .trash
            .iter()
            .all(|c| c.card_id(&runner.game.card_data) != "YELLOW-TRASH"),
        "the returned yellow Digimon must have left the trash"
    );
    // It was placed on the deck TOP, so this turn's Draw phase drew it back
    // into hand. The deck (8 FILL) would never surface it from the bottom.
    assert!(
        runner
            .game
            .player(0)
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "YELLOW-TRASH"),
        "deck top (drawn first) must surface the returned yellow Digimon into hand \
         on the turn-start draw — proof the verb appends to the deck end, not index 0"
    );
}

/// [Start of Your Turn] Delay tail: "Then, if you don't have a Digimon, you
/// may play 1 yellow Digimon card with 2000 DP or less from your trash without
/// paying the cost." With no Digimon controlled the optional play is offered,
/// the candidate set enforces the `2000 DP or less` filter, and taking it
/// free-plays the card.
#[test]
fn lm_029_delay_then_optionally_plays_small_yellow_digimon_if_you_have_no_digimon() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-029 YAML parses")
        // Trash pool: index 0 = YELLOW-SMALL (DP 2000, playable in step b),
        // index 1 = YELLOW-RETURN (DP 2000, returned in step a),
        // index 2 = YELLOW-BIG (DP 8000, too big for step b's DP filter).
        .add_card(yellow_digimon("YELLOW-SMALL", 3, 2000, 3))
        .add_card(yellow_digimon("YELLOW-RETURN", 3, 2000, 3))
        .add_card(yellow_digimon("YELLOW-BIG", 5, 8000, 8))
        .add_card(yellow_digimon("YELLOW-ANCHOR", 3, 2000, 3))
        .add_card(digimon("OPP-DIGIMON", CardColor::Red, 4, 5000, 5))
        .add_card(filler("FILL"))
        .hand(0, &["LM-029"])
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .memory(10)
        .start();

    runner.place_on_field(1, "OPP-DIGIMON", Some(0));
    // A yellow Digimon in breeding satisfies LM-029's color requirement
    // without putting a Digimon on the battle area, so step (b)'s
    // "if you don't have a Digimon" gate stays true.
    runner.place_in_breeding(0, "YELLOW-ANCHOR");
    push_to_trash(&mut runner, 0, "YELLOW-SMALL");
    push_to_trash(&mut runner, 0, "YELLOW-RETURN");
    push_to_trash(&mut runner, 0, "YELLOW-BIG");

    seat_lm_029_as_delay_option(&mut runner);

    runner.end_turn();
    runner.end_turn();
    assert_eq!(runner.game.turn_player(), 0);

    // Step (a): mandatory trash pick. All three yellow Digimon are eligible
    // (step a has no DP filter). Resolve YELLOW-RETURN's action id from its
    // live trash index and pick it.
    let view = runner
        .pending_selection_view()
        .expect("Delay step (a) yellow-Digimon trash selection");
    assert_eq!(view.kind, SelectionKind::Trash);
    assert_eq!(
        view.valid_action_ids.len(),
        3,
        "all three yellow Digimon in trash are eligible to return to deck top"
    );
    let return_trash_index = runner
        .game
        .player(0)
        .trash
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "YELLOW-RETURN")
        .expect("YELLOW-RETURN is in the trash");
    let return_action =
        digimon_engine::action::space::TRASH_EFFECT_START + return_trash_index as u16;
    assert!(
        view.valid_action_ids.contains(&return_action),
        "YELLOW-RETURN must be a selectable trash candidate"
    );
    runner
        .execute_action(view.selecting_player, return_action)
        .expect("return YELLOW-RETURN to deck top");

    // Player 0 controls no battle-area Digimon (LM-029 was trashed as the
    // Delay cost; the anchor is in breeding) -> step (b)'s optional play fires.
    let tail = runner
        .pending_selection_view()
        .expect("with no Digimon controlled, the optional small-yellow trash play must be offered");
    assert_eq!(tail.kind, SelectionKind::Trash);
    assert!(
        runner.pending_is_optional(),
        "the 'you may play' tail is optional"
    );
    // Only YELLOW-SMALL (DP 2000) qualifies — YELLOW-BIG (DP 8000) is filtered
    // out by `dp_lte: 2000`. LM-029 in trash is an Option, not a Digimon.
    assert_eq!(
        tail.valid_action_ids.len(),
        1,
        "only the DP-2000 yellow Digimon qualifies; the DP-8000 one is filtered out"
    );
    runner
        .execute_action(tail.selecting_player, tail.valid_action_ids[0])
        .expect("take the optional free play");
    runner.auto_resolve().expect("finish the Delay body");

    assert_eq!(
        runner.battle_area_size(0),
        1,
        "the optional free play should put exactly one yellow Digimon on the field"
    );
    assert_eq!(
        runner.game.player(0).battle_area[0]
            .top_card()
            .card_id(&runner.game.card_data),
        "YELLOW-SMALL",
        "the played card is the DP-2000 yellow Digimon"
    );
    // `play_from_trash_free` plays the card without paying its 3-cost: the
    // card left the trash directly into the battle area.
    assert!(
        runner
            .game
            .player(0)
            .trash
            .iter()
            .all(|c| c.card_id(&runner.game.card_data) != "YELLOW-SMALL"),
        "the free-played yellow Digimon must have left the trash"
    );
}

/// Negative branch of the Delay tail: when the controller already has a
/// Digimon on the field the "if you don't have a Digimon" optional play is
/// gated off entirely — only step (a) runs.
#[test]
fn lm_029_delay_skips_optional_play_when_you_already_have_a_digimon() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-029 YAML parses")
        .add_card(yellow_digimon("YELLOW-RETURN", 3, 2000, 3))
        .add_card(yellow_digimon("YELLOW-SMALL", 3, 2000, 3))
        .add_card(yellow_digimon("YELLOW-ANCHOR", 3, 2000, 3))
        .add_card(digimon("BLUE-DIGIMON", CardColor::Blue, 4, 5000, 5))
        .add_card(digimon("OPP-DIGIMON", CardColor::Red, 4, 5000, 5))
        .add_card(filler("FILL"))
        .hand(0, &["LM-029"])
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .memory(10)
        .start();

    runner.place_on_field(1, "OPP-DIGIMON", Some(0));
    push_to_trash(&mut runner, 0, "YELLOW-RETURN");
    push_to_trash(&mut runner, 0, "YELLOW-SMALL");
    // A yellow Digimon in breeding satisfies LM-029's color requirement
    // without being a yellow battle-area digivolve base for the Main clause.
    runner.place_in_breeding(0, "YELLOW-ANCHOR");
    // Player 0 keeps a (non-yellow) Digimon on the battle area through the
    // Delay so step (b)'s "if you don't have a Digimon" gate stays false.
    runner.place_on_field(0, "BLUE-DIGIMON", Some(0));

    seat_lm_029_as_delay_option(&mut runner);

    runner.end_turn();
    runner.end_turn();
    assert_eq!(runner.game.turn_player(), 0);

    let view = runner
        .pending_selection_view()
        .expect("Delay step (a) mandatory trash pick still runs");
    assert_eq!(view.kind, SelectionKind::Trash);
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("return a yellow Digimon to deck top");
    runner.auto_resolve().expect("finish the Delay body");

    assert_eq!(
        runner.pending_kind(),
        None,
        "the optional trash play must NOT be offered when you control a Digimon"
    );
    assert_eq!(
        runner.battle_area_size(0),
        1,
        "only the pre-existing Digimon remains; no extra card was played"
    );
    assert!(
        runner.game.player(0).battle_area.iter().all(|perm| {
            perm.top_card().card_id(&runner.game.card_data) == "BLUE-DIGIMON"
        }),
        "no yellow Digimon from trash was played"
    );
    // Step (a) still ran: one yellow Digimon left the trash; LM-029 (Delay
    // cost) plus the remaining yellow Digimon stay in the trash.
    assert_eq!(
        runner.trash_size(0),
        2,
        "step (a) returned one yellow Digimon to deck top; LM-029 and the \
         second yellow Digimon remain in the trash"
    );
}
