//! EX7-074 Vortex Resonance — Option card, Cost 3, Multi-color (Green + Yellow).
//!
//! # Card text (cards.json)
//!
//! While you have [LIBERATOR] trait Digimon or Tamer, you can ignore this card's color
//! requirements.
//!
//! [Main] Reveal the top 3 cards of your deck. Add 1 card with the [LIBERATOR] trait among
//! them to the hand. Return the rest to the bottom of the deck.
//! Then, 1 of your Digimon may digivolve into a Digimon card in your hand with the
//! digivolution cost reduced by 4.
//!
//! Inherited: Security Effect [Security] You may play 1 card with the [LIBERATOR] trait with a
//! play cost of 4 or less from your hand or trash without paying the cost. Then, add this
//! card to the hand.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX7/Green/EX7_074.cs
//!
//! # Patterns this test covers
//! - A1 Searching (top-3 reveal, add by LIBERATOR trait)
//! - D3 Color ignore / bypass through option use_requirement
//! - Effect-initiated digivolve with cost reduce 4 (main_from_hand option)
//! - Security: hand-or-trash play (LIBERATOR, cost ≤ 4) + native self-to-hand

#![allow(unused_imports, dead_code, unused_mut, unused_variables)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledColor, CompiledDeclarativeClause, CompiledDpConstraint, CompiledScope,
    CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::{PASS, PLAY_HAND_START};
use digimon_engine::build_action_mask;
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::selection::{OptionPlayResult, SelectionKind, TriggerSource};

// Load the YAML at compile time so tests run against the exact shipped spec.
const EX7_074_YAML: &str = include_str!("../../../cards/ex7/EX7-074.yaml");

// ---------------------------------------------------------------------------
// Fixture helpers
// ---------------------------------------------------------------------------

/// A Lv.4 LIBERATOR-trait Digimon (Green). Used both as a reveal-pick candidate
/// and as a board permanent enabling the color-bypass use_requirement.
fn make_liberator_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(5000);
    c.play_cost = 5;
    c.traits = vec!["LIBERATOR".to_string()];
    c.colors = vec![CardColor::Green];
    c
}

/// A LIBERATOR-trait Tamer (Green) — the alternate board predicate for the
/// color-bypass / use_requirement.
fn make_liberator_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.play_cost = 3;
    c.traits = vec!["LIBERATOR".to_string()];
    c.colors = vec![CardColor::Green];
    c
}

/// A non-LIBERATOR Digimon — never a reveal-add candidate.
fn make_filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(3000);
    c.play_cost = 3;
    c.colors = vec![CardColor::Red];
    c
}

/// A Lv.3 Green Digimon usable as a digivolve base on the field.
fn make_base_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(3000);
    c.play_cost = 3;
    c.colors = vec![CardColor::Green];
    c
}

/// A Lv.4 Green Digimon whose digivolution cost from a Lv.3 Green base is
/// `cost` memory. Used as the digivolve target card in hand.
fn make_evo_digimon(id: &str, cost: u16) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(6000);
    c.play_cost = 5;
    c.colors = vec![CardColor::Green];
    c.evo_costs = vec![EvoCost {
        card_color: CardColor::Green as u8,
        level: 3,
        memory_cost: cost,
    }];
    c
}

/// A LIBERATOR Digimon with a controlled play cost — for the Security/free-play
/// `play_cost_lte: 4` filter tests.
fn make_liberator_with_cost(id: &str, cost: u16) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(5000);
    c.play_cost = cost;
    c.traits = vec!["LIBERATOR".to_string()];
    c.colors = vec![CardColor::Green];
    c
}

/// A vanilla Tamer (no LIBERATOR trait) — used to confirm the digivolve
/// sub-step's hand filter rejects non-Digimon cards.
fn make_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.play_cost = 3;
    c.colors = vec![CardColor::Green];
    c
}

/// A 6000-DP attacker used to push the defender's security stack.
fn make_attacker(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(6000);
    c.play_cost = 5;
    c.colors = vec![CardColor::Red];
    c
}

fn ex7_074_runner() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(EX7_074_YAML)
        .expect("EX7-074 YAML must parse and compile")
        .memory(10)
        .start()
}

/// Resolve every *mandatory* pending selection by taking the first legal
/// action; stop at the first optional prompt (or when nothing is pending).
fn resolve_until_optional(runner: &mut DebugRunner) {
    let mut guard = 0;
    while runner.pending_selection().is_some() && !runner.pending_is_optional() {
        let view = runner
            .pending_selection_view()
            .expect("pending selection must have a view");
        let action = *view
            .valid_action_ids
            .first()
            .expect("mandatory selection must expose at least one action");
        runner
            .execute_action(view.selecting_player, action)
            .expect("mandatory selection resolves");
        guard += 1;
        assert!(guard < 30, "selection drain exceeded guard");
    }
}

// ---------------------------------------------------------------------------
// Section 1: Structural assertions
// ---------------------------------------------------------------------------

/// YAML must parse and compile without error.
#[test]
fn ex7_074_yaml_parses_without_error() {
    let _runner = ex7_074_runner();
}

/// EX7-074 is a Green + Yellow Option card with play cost 3.
/// cards.json `card_colors: [3, 2]` → Green, Yellow.
#[test]
fn ex7_074_is_green_yellow_option_cost_3() {
    let runner = ex7_074_runner();
    let compiled = runner
        .compiled_card("EX7-074")
        .expect("EX7-074 must be in compiled_cards");
    assert_eq!(
        compiled.kind,
        digimon_dsl::compiled::CompiledCardKind::Option,
        "EX7-074 must be an Option card"
    );
    assert_eq!(compiled.cost, Some(3), "EX7-074 must have play cost 3");
    assert_eq!(
        compiled.color,
        vec![CompiledColor::Green, CompiledColor::Yellow],
        "EX7-074 colors must be Green + Yellow (cards.json card_colors [3, 2])"
    );
}

/// EX7-074 must have exactly three clauses:
///   0: flood_gate declarative (IgnoreColorRequirement, conditional on LIBERATOR Digimon/Tamer)
///   1: main_from_hand triggered (reveal-3 + add-LIBERATOR + digivolve cost -4)
///   2: on_security inherited triggered (optional play LIBERATOR ≤4 from hand or trash)
#[test]
fn ex7_074_has_three_clauses() {
    let runner = ex7_074_runner();
    let compiled = runner
        .compiled_card("EX7-074")
        .expect("EX7-074 must be in compiled_cards");
    assert_eq!(
        compiled.effects.len(),
        3,
        "EX7-074 must have exactly 3 clauses (flood_gate + main + security); got {}",
        compiled.effects.len()
    );
}

/// Clause 0 is the flood_gate declarative for color bypass.
/// DCGO: IgnoreColorConditionClass at EffectTiming.None (non-triggered).
#[test]
fn ex7_074_clause_0_is_declarative_flood_gate() {
    let runner = ex7_074_runner();
    let compiled = runner
        .compiled_card("EX7-074")
        .expect("EX7-074 must be in compiled_cards");
    assert!(
        matches!(compiled.effects[0], CompiledClause::Declarative(_)),
        "Clause 0 must be a declarative (flood_gate for IgnoreColorRequirement); got {:?}",
        compiled.effects[0]
    );
}

/// Clause 1 is the Main triggered clause with main_from_hand timing.
/// DCGO: EffectTiming.OptionSkill — the standard main option timing.
#[test]
fn ex7_074_clause_1_is_main_from_hand_triggered() {
    let runner = ex7_074_runner();
    let compiled = runner
        .compiled_card("EX7-074")
        .expect("EX7-074 must be in compiled_cards");
    let main_clause = compiled.effects.get(1).expect("clause 1 must exist");
    assert!(
        matches!(main_clause, CompiledClause::Triggered(_)),
        "Clause 1 must be a triggered clause; got {:?}",
        main_clause
    );
    if let CompiledClause::Triggered(t) = main_clause {
        assert!(
            t.when.contains(&CompiledTiming::MainFromHand),
            "Clause 1 must have MainFromHand timing; got {:?}",
            t.when
        );
        assert_eq!(t.scope, CompiledScope::FaceUp, "Clause 1 must have FaceUp scope");
        assert!(
            !t.once_per_turn,
            "Main clause has no [Once Per Turn] in printed text"
        );
        // The printed [Main] text has no outer "you may" on the reveal step;
        // only the digivolve sub-step is optional (via select_own_permanent
        // with optional: true). DCGO: ActivateClass does not set canNoSelect
        // at the clause wrapper.
        assert!(
            !t.optional,
            "Main clause must NOT be optional at clause level (no outer 'you may')"
        );
    }
}

/// Clause 2 is the inherited Security triggered clause (on_security timing).
/// "You may" → optional: true. DCGO: EffectTiming.SecuritySkill.
#[test]
fn ex7_074_clause_2_is_inherited_security_optional() {
    let runner = ex7_074_runner();
    let compiled = runner
        .compiled_card("EX7-074")
        .expect("EX7-074 must be in compiled_cards");
    let sec_clause = compiled.effects.get(2).expect("clause 2 must exist");
    assert!(
        matches!(sec_clause, CompiledClause::Triggered(_)),
        "Clause 2 must be a triggered clause; got {:?}",
        sec_clause
    );
    if let CompiledClause::Triggered(t) = sec_clause {
        assert!(
            t.when.contains(&CompiledTiming::OnSecurity),
            "Clause 2 must have OnSecurity timing; got {:?}",
            t.when
        );
        assert_eq!(
            t.scope,
            CompiledScope::Inherited,
            "Clause 2 must have Inherited scope (security effect)"
        );
        assert!(
            t.optional,
            "Clause 2 must be optional ('you may' in printed text)"
        );
        assert!(
            !t.once_per_turn,
            "Security clause has no [Once Per Turn] in printed text"
        );
    }
}

// ---------------------------------------------------------------------------
// Section 2: Color bypass (D3) — declarative + use_requirement
// ---------------------------------------------------------------------------

/// The IgnoreColorRequirement flood_gate clause compiles correctly.
#[test]
fn ex7_074_color_bypass_compiles_as_declarative() {
    let runner = ex7_074_runner();
    let compiled = runner
        .compiled_card("EX7-074")
        .expect("EX7-074 must be in compiled_cards");
    assert!(
        matches!(compiled.effects[0], CompiledClause::Declarative(_)),
        "Flood gate clause (clause 0) must compile as Declarative (got {:?})",
        compiled.effects[0]
    );
}

/// Positive: with a LIBERATOR Digimon on the field, the use_requirement is
/// satisfied and EX7-074 becomes playable even off-color.
#[test]
fn ex7_074_color_bypass_unlocks_play_when_liberator_digimon_on_field() {
    let mut red_liberator = make_liberator_digimon("RED-LIBERATOR");
    red_liberator.colors = vec![CardColor::Red];

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(EX7_074_YAML)
        .expect("EX7-074 YAML must parse and compile")
        .add_card(red_liberator)
        .hand(0, &["EX7-074"])
        .memory(10)
        .start();

    let before = build_action_mask(&runner.game, 0);
    assert_eq!(
        before[PLAY_HAND_START as usize], 0.0,
        "EX7-074 should be blocked without matching color or LIBERATOR bypass"
    );

    runner.place_on_field(0, "RED-LIBERATOR", Some(0));

    let after = build_action_mask(&runner.game, 0);
    assert_eq!(
        after[PLAY_HAND_START as usize], 1.0,
        "EX7-074 should be playable when LIBERATOR board presence enables its use requirement"
    );
}

/// Positive: a LIBERATOR Tamer also satisfies the use_requirement
/// ("[LIBERATOR] trait Digimon or Tamer").
#[test]
fn ex7_074_color_bypass_unlocks_play_when_liberator_tamer_on_field() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(EX7_074_YAML)
        .expect("EX7-074 YAML must parse and compile")
        .add_card(make_liberator_tamer("LIB-TAMER"))
        .hand(0, &["EX7-074"])
        .memory(10)
        .start();

    assert_eq!(
        build_action_mask(&runner.game, 0)[PLAY_HAND_START as usize],
        0.0,
        "EX7-074 should be blocked off-color before any LIBERATOR is on field"
    );

    runner.place_on_field(0, "LIB-TAMER", Some(0));

    assert_eq!(
        build_action_mask(&runner.game, 0)[PLAY_HAND_START as usize],
        1.0,
        "EX7-074 should be playable when a LIBERATOR Tamer enables its use requirement"
    );
}

/// Negative: a non-LIBERATOR permanent on the field does NOT enable the
/// color bypass — the off-color Option stays blocked.
#[test]
fn ex7_074_color_bypass_inactive_without_liberator_on_field() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(EX7_074_YAML)
        .expect("EX7-074 YAML must parse and compile")
        .add_card(make_filler("RED-FILLER"))
        .hand(0, &["EX7-074"])
        .memory(10)
        .start();

    runner.place_on_field(0, "RED-FILLER", Some(0));

    assert_eq!(
        build_action_mask(&runner.game, 0)[PLAY_HAND_START as usize],
        0.0,
        "A non-LIBERATOR permanent must not enable the color-requirement bypass"
    );
}

/// The Security clause uses native play_cost_lte filters and add_this_option_to_hand
/// (no legacy raw_rust self-to-hand shim).
#[test]
fn ex7_074_security_uses_play_cost_filters_and_native_self_to_hand() {
    let runner = ex7_074_runner();
    let compiled = runner
        .compiled_card("EX7-074")
        .expect("EX7-074 must be in compiled_cards");
    let CompiledClause::Triggered(security) = &compiled.effects[2] else {
        panic!("security clause must be triggered");
    };

    fn has_select_with_cost_lte(steps: &[CompiledStep], cap: i32) -> bool {
        steps.iter().any(|step| match step {
            CompiledStep::SelectHand { filter, .. } | CompiledStep::SelectTrash { filter, .. } => {
                filter.play_cost_lte == Some(CompiledDpConstraint::Literal(cap))
                    || filter.all_of.iter().any(|nested| {
                        nested.play_cost_lte == Some(CompiledDpConstraint::Literal(cap))
                    })
            }
            CompiledStep::If { then, .. } => has_select_with_cost_lte(then, cap),
            _ => false,
        })
    }

    assert!(
        has_select_with_cost_lte(&security.process, 4),
        "security hand/trash selections must enforce play_cost_lte: 4"
    );
    assert!(
        security
            .process
            .iter()
            .any(|step| matches!(step, CompiledStep::AddThisOptionToHand)),
        "security clause must use native add_this_option_to_hand"
    );
    assert!(
        !security
            .process
            .iter()
            .any(|step| matches!(step, CompiledStep::RawRust { .. })),
        "EX7-074 should not use the legacy raw_rust self-to-hand shim"
    );
}

// ---------------------------------------------------------------------------
// Section 3: Main clause — reveal-3 + add LIBERATOR to hand
// ---------------------------------------------------------------------------

/// Firing the [Main] effect reveals the top 3 cards and installs the reveal
/// pick prompt. Drives the process via `activate_hand_main` so EX7-074 stays
/// in hand and the prompt can be inspected before further resolution.
#[test]
fn ex7_074_main_reveals_three_and_installs_pick_prompt() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(EX7_074_YAML)
        .expect("EX7-074 YAML parses")
        .add_card(make_liberator_digimon("LIB"))
        .add_card(make_filler("FILL"))
        .hand(0, &["EX7-074"])
        // deck.last() is the top of deck; LIB must be in the revealed top 3.
        .deck(0, &["FILL", "FILL", "FILL", "FILL", "FILL", "LIB"])
        .memory(10)
        .start();
    runner.game.enter_main_phase();

    let fired = runner.game.activate_hand_main(0, 0);
    assert!(fired, "activate_hand_main must return true for EX7-074");

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Reveal),
        "EX7-074 [Main] must install a Reveal pick prompt after revealing the top 3"
    );
    let view = runner
        .pending_selection_view()
        .expect("reveal selection must have a view");
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only the single LIBERATOR card among the revealed three is selectable"
    );
}

/// After reveal + selecting 1 LIBERATOR card, the selected card moves to hand.
/// DCGO: mode AddHand → selected card added to controller's hand.
#[test]
fn ex7_074_main_adds_liberator_from_reveal_to_hand() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(EX7_074_YAML)
        .expect("EX7-074 YAML parses")
        .add_card(make_liberator_digimon("LIB-PICK"))
        .add_card(make_filler("FILL"))
        .hand(0, &["EX7-074"])
        // Top of deck is deck.last(); reveal pulls the top 3.
        .deck(0, &["FILL", "FILL", "FILL", "FILL", "FILL", "LIB-PICK"])
        .memory(10)
        .start();
    runner.game.enter_main_phase();

    let hand_before = runner.hand_size(0); // EX7-074 only
    assert!(runner.game.activate_hand_main(0, 0));

    // Pick the revealed LIBERATOR card explicitly.
    let view = runner
        .pending_selection_view()
        .expect("reveal pick prompt must install");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("LIBERATOR reveal pick resolves");
    resolve_until_optional(&mut runner);

    // activate_hand_main keeps EX7-074 in hand, and LIB-PICK was added.
    assert_eq!(
        runner.hand_size(0),
        hand_before + 1,
        "the revealed LIBERATOR card must be added to hand (net +1)"
    );
    assert!(
        runner.game.players[0]
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "LIB-PICK"),
        "the specifically seeded LIBERATOR card must be in hand"
    );
}

/// Negative: with no LIBERATOR card among the revealed three, the reveal pick
/// is optional (skippable) and the effect resolves cleanly without an add.
#[test]
fn ex7_074_main_reveal_with_no_liberator_resolves_with_no_add() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(EX7_074_YAML)
        .expect("EX7-074 YAML parses")
        .add_card(make_filler("FILL"))
        .hand(0, &["EX7-074"])
        .deck(0, &["FILL", "FILL", "FILL", "FILL", "FILL", "FILL"])
        .memory(10)
        .start();
    runner.game.enter_main_phase();

    let hand_before = runner.hand_size(0);
    assert!(runner.game.activate_hand_main(0, 0));

    // The reveal pick must be optional when no LIBERATOR is revealed.
    if let Some(SelectionKind::Reveal) = runner.pending_kind() {
        assert!(
            runner.pending_is_optional(),
            "reveal pick must be optional when no LIBERATOR card is revealed"
        );
        runner
            .execute_action(0, PASS)
            .expect("declining the empty reveal pick resolves");
    }
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "no card may be added to hand when no LIBERATOR is revealed"
    );
}

/// The non-selected revealed cards return to the BOTTOM of the deck.
/// DCGO: remainingCardsPlace RemainingCardsPlace.DeckBottom.
/// Engine stores deck front=bottom → returned cards land at low indices.
#[test]
fn ex7_074_main_reveal_remainder_placed_at_deck_bottom() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(EX7_074_YAML)
        .expect("EX7-074 YAML parses")
        .add_card(make_liberator_digimon("LIB-PICK"))
        .add_card(make_filler("REM-A"))
        .add_card(make_filler("REM-B"))
        .add_card(make_filler("FILL"))
        .hand(0, &["EX7-074"])
        // Top 3 revealed = REM-A, REM-B, LIB-PICK (deck.last is top).
        .deck(0, &["FILL", "FILL", "FILL", "REM-A", "REM-B", "LIB-PICK"])
        .memory(10)
        .start();
    runner.game.enter_main_phase();

    let deck_before = runner.game.players[0].deck.len();
    assert!(runner.game.activate_hand_main(0, 0));

    let view = runner
        .pending_selection_view()
        .expect("reveal pick prompt must install");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("LIBERATOR reveal pick resolves");
    resolve_until_optional(&mut runner);

    // 3 revealed - 1 added = 2 returned: net deck loss of 1.
    assert_eq!(
        runner.game.players[0].deck.len(),
        deck_before - 1,
        "reveal 3, add 1, return 2: deck shrinks by exactly 1"
    );
    // The two non-picked cards are now at the bottom (lowest indices).
    let bottom_two: Vec<String> = runner.game.players[0].deck[..2]
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    assert!(
        bottom_two.contains(&"REM-A".to_string()) && bottom_two.contains(&"REM-B".to_string()),
        "the two unpicked revealed cards must return to deck bottom; got {bottom_two:?}"
    );
}

// ---------------------------------------------------------------------------
// Section 4: Main clause — optional digivolve sub-step (cost reduce 4)
// ---------------------------------------------------------------------------

/// After the reveal+add step, the digivolve sub-step's own-Digimon selection
/// is optional (DCGO: canNoSelect: true) — PASS must be legal.
#[test]
fn ex7_074_digivolve_substep_is_optional_declinable() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(EX7_074_YAML)
        .expect("EX7-074 YAML parses")
        .add_card(make_liberator_digimon("LIB-PICK"))
        .add_card(make_base_digimon("BASE"))
        .add_card(make_evo_digimon("EVO", 5))
        .add_card(make_filler("FILL"))
        .hand(0, &["EX7-074", "EVO"])
        .deck(0, &["FILL", "FILL", "FILL", "FILL", "FILL", "LIB-PICK"])
        .memory(10)
        .start();
    runner.game.enter_main_phase();
    runner.place_on_field(0, "BASE", Some(0));

    assert!(runner.game.activate_hand_main(0, 0));
    // Resolve the reveal pick.
    let view = runner
        .pending_selection_view()
        .expect("reveal pick prompt installs");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("reveal pick resolves");
    resolve_until_optional(&mut runner);

    // The digivolve own-Digimon prompt must now be pending and optional.
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::OwnField),
        "digivolve sub-step installs an own-Digimon selection"
    );
    assert!(
        runner.pending_is_optional(),
        "digivolve sub-step's own-Digimon pick must be optional (DCGO canNoSelect: true)"
    );
    assert_eq!(
        build_action_mask(&runner.game, 0)[PASS as usize],
        1.0,
        "PASS must be legal for the optional digivolve sub-step"
    );

    // Declining leaves the field unchanged (no digivolve).
    let stack_before = runner.game.players[0].battle_area[0].stack_size();
    runner
        .execute_action(0, PASS)
        .expect("declining the digivolve sub-step resolves");
    runner.auto_resolve().ok();
    assert_eq!(
        runner.game.players[0].battle_area[0].stack_size(),
        stack_before,
        "declining the optional digivolve must not change the base Digimon's stack"
    );
}

/// The digivolve sub-step reduces the digivolution cost by 4.
/// DCGO: reduceCostTuple (reduceCost: 4). EVO's printed evo cost is 6 → pays 2.
#[test]
fn ex7_074_digivolve_substep_cost_reduced_by_4() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(EX7_074_YAML)
        .expect("EX7-074 YAML parses")
        .add_card(make_liberator_digimon("LIB-PICK"))
        .add_card(make_base_digimon("BASE"))
        .add_card(make_evo_digimon("EVO", 6))
        .add_card(make_filler("FILL"))
        .hand(0, &["EX7-074", "EVO"])
        .deck(0, &["FILL", "FILL", "FILL", "FILL", "FILL", "LIB-PICK"])
        .memory(10)
        .start();
    runner.game.enter_main_phase();
    runner.place_on_field(0, "BASE", Some(0));

    assert!(runner.game.activate_hand_main(0, 0));
    // Reveal pick.
    let reveal = runner
        .pending_selection_view()
        .expect("reveal pick prompt installs");
    runner
        .execute_action(0, reveal.valid_action_ids[0])
        .expect("reveal pick resolves");
    resolve_until_optional(&mut runner);

    // Pick the BASE Digimon to digivolve.
    assert_eq!(runner.pending_kind(), Some(SelectionKind::OwnField));
    let target = runner.pending_selection_view().unwrap();
    let memory_before = runner.memory();
    runner
        .execute_action(0, target.valid_action_ids[0])
        .expect("digivolve target pick resolves");

    // Pick the EVO card from hand.
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Hand),
        "after the target pick, a hand-card selection installs"
    );
    let evo = runner.pending_selection_view().unwrap();
    runner
        .execute_action(0, evo.valid_action_ids[0])
        .expect("digivolve evolution-card pick resolves");
    runner.auto_resolve().ok();

    // EVO printed evo cost 6, reduced by 4 → pays 2 memory.
    assert_eq!(
        memory_before - runner.memory(),
        2,
        "printed evo cost 6 reduced by 4 must cost 2 memory; before={memory_before}, after={}",
        runner.memory()
    );
    // The base Digimon's stack grew (digivolve happened).
    assert_eq!(
        runner.game.players[0].battle_area[0].stack_size(),
        2,
        "the BASE Digimon must have digivolved into EVO"
    );
    assert_eq!(
        runner.game.players[0].battle_area[0]
            .top_card()
            .card_id(&runner.game.card_data),
        "EVO",
        "EVO must be the new top card after the digivolve"
    );
}

/// Cost reduction floors at 0: an evo cost ≤ 4 reduced by 4 costs 0 memory.
#[test]
fn ex7_074_digivolve_substep_cost_floors_at_zero() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(EX7_074_YAML)
        .expect("EX7-074 YAML parses")
        .add_card(make_liberator_digimon("LIB-PICK"))
        .add_card(make_base_digimon("BASE"))
        .add_card(make_evo_digimon("EVO", 3))
        .add_card(make_filler("FILL"))
        .hand(0, &["EX7-074", "EVO"])
        .deck(0, &["FILL", "FILL", "FILL", "FILL", "FILL", "LIB-PICK"])
        .memory(10)
        .start();
    runner.game.enter_main_phase();
    runner.place_on_field(0, "BASE", Some(0));

    assert!(runner.game.activate_hand_main(0, 0));
    let reveal = runner.pending_selection_view().unwrap();
    runner
        .execute_action(0, reveal.valid_action_ids[0])
        .expect("reveal pick resolves");
    resolve_until_optional(&mut runner);

    assert_eq!(runner.pending_kind(), Some(SelectionKind::OwnField));
    let target = runner.pending_selection_view().unwrap();
    let memory_before = runner.memory();
    runner
        .execute_action(0, target.valid_action_ids[0])
        .expect("digivolve target pick resolves");
    let evo = runner.pending_selection_view().unwrap();
    runner
        .execute_action(0, evo.valid_action_ids[0])
        .expect("digivolve evolution-card pick resolves");
    runner.auto_resolve().ok();

    assert_eq!(
        runner.memory(),
        memory_before,
        "evo cost 3 reduced by 4 must floor at 0 memory spent"
    );
    assert_eq!(
        runner.game.players[0].battle_area[0].stack_size(),
        2,
        "the digivolve still happens even at 0 cost"
    );
}

/// The digivolve sub-step's hand selection only offers Digimon cards.
/// DCGO: CanSelectHandCardCondition checks cardSource.IsDigimon.
///
/// The reveal-add pick is a LIBERATOR *Tamer* so it does not itself become a
/// digivolve candidate — leaving only the EVO Digimon eligible (the Tamer in
/// hand must be filtered out).
#[test]
fn ex7_074_digivolve_substep_only_offers_digimon_hand_cards() {
    let mut lib_tamer = make_liberator_tamer("LIB-TAMER-PICK");
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(EX7_074_YAML)
        .expect("EX7-074 YAML parses")
        .add_card(lib_tamer)
        .add_card(make_base_digimon("BASE"))
        .add_card(make_evo_digimon("EVO", 5))
        .add_card(make_tamer("TAMER-IN-HAND"))
        .add_card(make_filler("FILL"))
        .hand(0, &["EX7-074", "EVO", "TAMER-IN-HAND"])
        .deck(0, &["FILL", "FILL", "FILL", "FILL", "FILL", "LIB-TAMER-PICK"])
        .memory(10)
        .start();
    runner.game.enter_main_phase();
    runner.place_on_field(0, "BASE", Some(0));

    assert!(runner.game.activate_hand_main(0, 0));
    let reveal = runner.pending_selection_view().unwrap();
    runner
        .execute_action(0, reveal.valid_action_ids[0])
        .expect("reveal pick resolves");
    resolve_until_optional(&mut runner);

    assert_eq!(runner.pending_kind(), Some(SelectionKind::OwnField));
    let target = runner.pending_selection_view().unwrap();
    runner
        .execute_action(0, target.valid_action_ids[0])
        .expect("digivolve target pick resolves");

    // Only the EVO Digimon card should be a valid hand pick — not the Tamer.
    assert_eq!(runner.pending_kind(), Some(SelectionKind::Hand));
    let evo = runner.pending_selection_view().unwrap();
    assert_eq!(
        evo.valid_action_ids.len(),
        1,
        "only the Digimon card (EVO) may be selected — the Tamer must be filtered out"
    );
}

/// If no own Digimon is on the field, the digivolve sub-step's own-permanent
/// selection has no eligible target — no digivolve happens, effect resolves.
#[test]
fn ex7_074_digivolve_substep_skipped_when_no_field_digimon() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(EX7_074_YAML)
        .expect("EX7-074 YAML parses")
        .add_card(make_liberator_digimon("LIB-PICK"))
        .add_card(make_evo_digimon("EVO", 5))
        .add_card(make_filler("FILL"))
        .hand(0, &["EX7-074", "EVO"])
        .deck(0, &["FILL", "FILL", "FILL", "FILL", "FILL", "LIB-PICK"])
        .memory(10)
        .start();
    runner.game.enter_main_phase();
    // No Digimon on P0's field.

    let battle_before = runner.battle_area_size(0);
    assert!(runner.game.activate_hand_main(0, 0));
    let reveal = runner.pending_selection_view().unwrap();
    runner
        .execute_action(0, reveal.valid_action_ids[0])
        .expect("reveal pick resolves");
    // Drain the rest; with no field Digimon the optional own-permanent pick
    // has no eligible target — declining (or auto-empty) completes the effect.
    let mut guard = 0;
    while runner.pending_selection().is_some() {
        let opt = runner.pending_is_optional();
        let view = runner.pending_selection_view().unwrap();
        let action = if opt {
            PASS
        } else {
            *view.valid_action_ids.first().expect("mandatory pick has actions")
        };
        runner.execute_action(view.selecting_player, action).ok();
        guard += 1;
        assert!(guard < 30, "selection drain guard");
    }

    assert_eq!(
        runner.battle_area_size(0),
        battle_before,
        "no digivolve may occur when P0 has no Digimon on field"
    );
}

/// Full-flow integration: play EX7-074 from hand as a real Option play and
/// auto_resolve every prompt — no panic, the option ends up in the trash.
#[test]
fn ex7_074_full_main_flow_no_panic() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(EX7_074_YAML)
        .expect("EX7-074 YAML parses")
        .add_card(make_liberator_digimon("LIB"))
        .add_card(make_base_digimon("BASE"))
        .add_card(make_evo_digimon("EVO", 5))
        .add_card(make_filler("FILL"))
        .hand(0, &["EX7-074", "EVO"])
        .deck(0, &["FILL", "FILL", "FILL", "FILL", "FILL", "LIB"])
        .memory(10)
        .start();
    runner.game.enter_main_phase();
    runner.place_on_field(0, "BASE", Some(0));

    let result = runner.game.play_option_from_hand(0, 0);
    assert!(
        matches!(result, OptionPlayResult::Pending | OptionPlayResult::Trashed),
        "playing EX7-074 must enter resolution; got {result:?}"
    );
    runner.auto_resolve().ok();

    // EX7-074 has no "place this card in the battle area" text — it is trashed
    // after resolving, like a regular Option.
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "EX7-074"),
        "EX7-074 must be trashed after its Main effect resolves"
    );
}

// ---------------------------------------------------------------------------
// Section 5: Inherited Security clause — play LIBERATOR (≤4) from hand or trash
// ---------------------------------------------------------------------------

/// Positive (both zones eligible): a LIBERATOR card in BOTH hand and trash →
/// the zone-choice prompt ("From hand" / "From trash") installs first.
/// DCGO: "From which area do you play a card?" EffectChoice prompt.
#[test]
fn ex7_074_security_installs_zone_choice_when_liberator_in_both_zones() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(EX7_074_YAML)
        .expect("EX7-074 YAML parses")
        .add_card(make_attacker("ATTACKER"))
        .add_card(make_liberator_with_cost("LIB-HAND", 3))
        .add_card(make_liberator_with_cost("LIB-TRASH", 3))
        .add_card(make_filler("FILL"))
        .hand(1, &["LIB-HAND"])
        .security(1, &["EX7-074"])
        .deck(1, &["LIB-TRASH"])
        .memory(10)
        .start();

    // Seed P1's trash with a LIBERATOR card.
    let seed = runner.game.players[1].deck.pop().expect("LIB-TRASH in deck");
    runner.game.players[1].trash.push(seed);

    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));
    let result = runner.attack_player(attacker, 1, false);
    assert_eq!(
        result,
        AttackResult::InProgress,
        "security resolution should pause combat for the EX7-074 clause"
    );

    // The "you may" Security clause first installs an outer accept/decline
    // prompt (G-OUTER-OPTIONAL-NOT-INSTALLED). Accept it to run the body.
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Replacement),
        "optional Security clause installs an outer accept/decline prompt first"
    );
    runner
        .accept_optional_trigger()
        .expect("accept the optional Security clause");

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::EffectChoice),
        "with LIBERATOR cards in both hand and trash, a zone-choice prompt installs"
    );
    let view = runner
        .pending_selection_view()
        .expect("zone-choice prompt must have a view");
    let labels: Vec<&str> = view
        .effect_choices
        .as_ref()
        .expect("zone-choice exposes labels")
        .iter()
        .map(|c| c.label.as_str())
        .collect();
    assert_eq!(
        labels,
        vec!["From hand", "From trash"],
        "the zone-choice labels must be hand/trash"
    );
}

/// Positive (hand branch): choosing "From hand" then a LIBERATOR card plays it
/// from hand for free (no memory paid) and moves it to the battle area.
#[test]
fn ex7_074_security_plays_liberator_from_hand_free() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(EX7_074_YAML)
        .expect("EX7-074 YAML parses")
        .add_card(make_attacker("ATTACKER"))
        .add_card(make_liberator_with_cost("LIB-HAND", 4))
        .add_card(make_filler("FILL"))
        .hand(1, &["LIB-HAND"])
        .security(1, &["EX7-074"])
        .memory(5)
        .start();

    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));
    let memory_before = runner.memory();
    let field_before = runner.battle_area_size(1);

    runner.attack_player(attacker, 1, false);

    // Accept the outer optional "you may" prompt.
    if runner.pending_kind() == Some(SelectionKind::Replacement) {
        runner
            .accept_optional_trigger()
            .expect("accept the optional Security clause");
    }
    // Only hand has a LIBERATOR — drive the zone choice to the hand branch.
    if runner.pending_kind() == Some(SelectionKind::EffectChoice) {
        runner.execute_branch(0).expect("choose 'From hand'");
    }
    // Pick the LIBERATOR hand card.
    if runner.pending_kind() == Some(SelectionKind::Hand) {
        let view = runner.pending_selection_view().unwrap();
        runner
            .execute_action(view.selecting_player, view.valid_action_ids[0])
            .expect("LIBERATOR hand pick resolves");
    }
    runner.auto_resolve().expect("security flow resolves");

    assert_eq!(
        runner.battle_area_size(1),
        field_before + 1,
        "the LIBERATOR card must be played from hand into the battle area"
    );
    assert_eq!(
        runner.game.players[1].battle_area[0]
            .top_card()
            .card_id(&runner.game.card_data),
        "LIB-HAND",
        "the played permanent must be the seeded LIBERATOR card"
    );
    assert_eq!(
        runner.memory(),
        memory_before,
        "the free play must not change memory (played without paying the cost)"
    );
}

/// Positive (trash branch): choosing "From trash" then a LIBERATOR card plays
/// it from trash for free.
#[test]
fn ex7_074_security_plays_liberator_from_trash_free() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(EX7_074_YAML)
        .expect("EX7-074 YAML parses")
        .add_card(make_attacker("ATTACKER"))
        .add_card(make_liberator_with_cost("LIB-TRASH", 4))
        .add_card(make_filler("FILL"))
        .security(1, &["EX7-074"])
        .deck(1, &["LIB-TRASH"])
        .memory(5)
        .start();

    // Seed P1's trash with the LIBERATOR card.
    let seed = runner.game.players[1].deck.pop().expect("LIB-TRASH in deck");
    runner.game.players[1].trash.push(seed);

    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));
    let memory_before = runner.memory();
    let field_before = runner.battle_area_size(1);

    runner.attack_player(attacker, 1, false);

    // Accept the outer optional "you may" prompt.
    if runner.pending_kind() == Some(SelectionKind::Replacement) {
        runner
            .accept_optional_trigger()
            .expect("accept the optional Security clause");
    }
    // Only trash has a LIBERATOR — drive the zone choice to the trash branch.
    if runner.pending_kind() == Some(SelectionKind::EffectChoice) {
        runner.execute_branch(1).expect("choose 'From trash'");
    }
    if runner.pending_kind() == Some(SelectionKind::Trash) {
        let view = runner.pending_selection_view().unwrap();
        runner
            .execute_action(view.selecting_player, view.valid_action_ids[0])
            .expect("LIBERATOR trash pick resolves");
    }
    runner.auto_resolve().expect("security flow resolves");

    assert_eq!(
        runner.battle_area_size(1),
        field_before + 1,
        "the LIBERATOR card must be played from trash into the battle area"
    );
    assert_eq!(
        runner.game.players[1].battle_area[0]
            .top_card()
            .card_id(&runner.game.card_data),
        "LIB-TRASH",
        "the played permanent must be the seeded trash LIBERATOR card"
    );
    assert_eq!(
        runner.memory(),
        memory_before,
        "the free play must not change memory"
    );
}

/// "Then, add this card to the hand." — after the Security clause resolves,
/// EX7-074 itself moves from the security stack to the defender's hand
/// (it is NOT trashed).
#[test]
fn ex7_074_security_adds_self_to_hand_after_resolving() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(EX7_074_YAML)
        .expect("EX7-074 YAML parses")
        .add_card(make_attacker("ATTACKER"))
        .add_card(make_liberator_with_cost("LIB-HAND", 3))
        .add_card(make_filler("FILL"))
        .hand(1, &["LIB-HAND"])
        .security(1, &["EX7-074"])
        .memory(5)
        .start();

    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));
    runner.attack_player(attacker, 1, false);
    runner.auto_resolve().expect("security flow resolves");

    assert_eq!(
        runner.security_count(1),
        0,
        "EX7-074 must leave the security stack"
    );
    assert!(
        runner.game.players[1]
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "EX7-074"),
        "EX7-074 must be added to the defender's hand after the Security clause"
    );
    assert_eq!(
        runner.trash_size(1),
        0,
        "EX7-074 must NOT be trashed — it is added to hand"
    );
}

/// Negative: with no eligible LIBERATOR card (cost ≤ 4) in hand or trash, the
/// Security clause plays nothing but still adds EX7-074 to hand.
#[test]
fn ex7_074_security_no_play_when_no_liberator_available() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(EX7_074_YAML)
        .expect("EX7-074 YAML parses")
        .add_card(make_attacker("ATTACKER"))
        .add_card(make_filler("FILL"))
        .hand(1, &["FILL"])
        .security(1, &["EX7-074"])
        .memory(5)
        .start();

    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));
    let field_before = runner.battle_area_size(1);

    runner.attack_player(attacker, 1, false);
    runner.auto_resolve().expect("security flow resolves");

    assert_eq!(
        runner.battle_area_size(1),
        field_before,
        "no permanent may be played when no LIBERATOR (cost ≤4) is available"
    );
    // "Then, add this card to the hand" still fires.
    assert!(
        runner.game.players[1]
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "EX7-074"),
        "EX7-074 must still be added to hand even when nothing is played"
    );
}

/// The Security free-play filter excludes LIBERATOR cards with play cost > 4.
/// DCGO: GetCostItself <= 4. EX7-074 YAML enforces play_cost_lte: 4 on the
/// hand/trash selections.
#[test]
fn ex7_074_security_filters_liberator_by_cost_lte_4() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(EX7_074_YAML)
        .expect("EX7-074 YAML parses")
        .add_card(make_attacker("ATTACKER"))
        .add_card(make_liberator_with_cost("LIB-CHEAP", 3))
        .add_card(make_liberator_with_cost("LIB-EXPENSIVE", 5))
        .add_card(make_filler("FILL"))
        .hand(1, &["LIB-CHEAP", "LIB-EXPENSIVE"])
        .security(1, &["EX7-074"])
        .memory(5)
        .start();

    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));
    runner.attack_player(attacker, 1, false);

    // Accept the outer optional "you may" prompt.
    if runner.pending_kind() == Some(SelectionKind::Replacement) {
        runner
            .accept_optional_trigger()
            .expect("accept the optional Security clause");
    }
    // Only hand has LIBERATOR cards → no zone choice; a Hand pick installs.
    if runner.pending_kind() == Some(SelectionKind::EffectChoice) {
        runner.execute_branch(0).expect("choose 'From hand'");
    }
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Hand),
        "the Security hand selection must install"
    );
    let view = runner
        .pending_selection_view()
        .expect("hand selection view");

    // Exactly one card (the cost-3 LIBERATOR) is eligible — the cost-5 one is
    // filtered out by play_cost_lte: 4.
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only the cost-3 LIBERATOR is eligible; the cost-5 LIBERATOR must be filtered out"
    );
}

/// Full security integration via the real combat path — no panic when the
/// defender simply declines the optional Security clause (PASS).
#[test]
fn ex7_074_security_optional_decline_resolves_cleanly() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(EX7_074_YAML)
        .expect("EX7-074 YAML parses")
        .add_card(make_attacker("ATTACKER"))
        .add_card(make_liberator_with_cost("LIB-HAND", 3))
        .add_card(make_filler("FILL"))
        .hand(1, &["LIB-HAND"])
        .security(1, &["EX7-074"])
        .memory(5)
        .start();

    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));
    let field_before = runner.battle_area_size(1);
    runner.attack_player(attacker, 1, false);

    // If the optional clause exposes a top-level decline, take it.
    if runner.pending_selection().is_some() && runner.pending_is_optional() {
        runner.execute_action(1, PASS).ok();
    }
    runner.auto_resolve().expect("security flow resolves after decline");

    // The optional clause may decline the play, but the always-on
    // "add this card to the hand" tail still fires.
    assert!(
        runner.battle_area_size(1) >= field_before,
        "declining the optional Security play must not corrupt the battle area"
    );
    assert_eq!(
        runner.security_count(1),
        0,
        "EX7-074 still leaves the security stack after the clause resolves"
    );
}
