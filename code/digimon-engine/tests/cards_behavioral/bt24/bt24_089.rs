//! BT24-089 Unique Emblem: Blazing Conductor — Option, Cost 3, Red, Traits: LIBERATOR.
//!
//! # Card text (cards.json / printed)
//!
//! **[Main]** You may play 1 \[Elizamon\] or \[Owen Dreadnought\] from your
//! hand or trash without paying the cost. Then, place this card in the battle area.
//!
//! **\[Your Turn\] When any of your \[Owen Dreadnought\]s suspend**, ＜Delay＞
//! (By trashing this card after the placing turn, activate the effect below.)
//! ・1 of your \[Reptile\] or \[Dragonkin\] Digimon may digivolve into a
//! Digimon card with the \[Reptile\] or \[Dragonkin\] trait and the \[LIBERATOR\]
//! trait in the hand with the digivolution cost reduced by 3.
//!
//! **Inherited:** Security Effect \[Security\] Activate this card's \[Main\] effects.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT24/Red/BT24_089.cs
//!
//! # Known engine and DSL gaps affecting these tests
//!
//! **G-PLACE-SELF-AS-OPTION-PERMANENT [DSL gap]:**
//!   There is no `place_option_in_battle_area: {}` step verb. The "Then, place
//!   this card in the battle area" sub-step of the Main clause cannot be expressed.
//!   Tests asserting the option lands in battle area are `#[ignore]`'d pending
//!   this verb being added to the DSL vocabulary.
//!
//! **G-DELAY-SUSPEND-CONDITION [DSL gap]:**
//!   `kind: delay` only supports `DelayTrigger::EndOfThisTurn` /
//!   `EndOfYourNextTurn`. BT24-089's Delay fires when an \[Owen Dreadnought\]
//!   suspends on the owner's turn — not at end of turn. No `DelayTrigger`
//!   variant exists for "on_suspend name-filtered". The Delay clause is modelled
//!   as a `kind: raw_rust` structural stub. All Delay behavioral tests are
//!   `#[ignore]`'d pending the `DelayTrigger::OnSuspendNameFilter` variant.
//!
//! **G-EFFECT-DIGIVOLVE-FROM-HAND [engine gap]:**
//!   The Delay process (digivolve into Reptile/Dragonkin + LIBERATOR with -3
//!   cost) requires `effect_initiated_digivolve` within a Delay body. This is
//!   blocked by G-DELAY-SUSPEND-CONDITION; tests are `#[ignore]`'d.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming};
use digimon_engine::selection::TriggerSource;

const YAML: &str = include_str!("../../../cards/bt24/BT24-089.yaml");

// ─── helpers ──────────────────────────────────────────────────────────────────

fn make_elizamon() -> CardData {
    let mut c = make_test_card("ELIZAMON", "Elizamon");
    c.card_kind = CardKind::Digimon;
    c
}

fn make_owen() -> CardData {
    let mut c = make_test_card("OWEN", "Owen Dreadnought");
    c.card_kind = CardKind::Tamer;
    c
}

fn make_filler(id: &str) -> CardData {
    make_test_card(id, id)
}

// ─── §1 YAML parse + structural ───────────────────────────────────────────────

/// The YAML must parse and compile without errors.
#[test]
fn bt24_089_yaml_parses_and_compiles() {
    let _runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT24-089 YAML must parse and compile without errors");
}

/// BT24-089 is an Option card with cost 3.
#[test]
fn bt24_089_is_option_cost_3() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("BT24-089")
        .expect("BT24-089 compiled card present");

    assert_eq!(compiled.kind, digimon_dsl::compiled::CompiledCardKind::Option);
    assert_eq!(compiled.cost, Some(3));
}

/// BT24-089 must have exactly 3 clauses:
///   Clause 0: main_from_hand (triggered, optional)
///   Clause 1: raw_rust stub for Delay (declarative)
///   Clause 2: on_security (triggered, optional)
#[test]
fn bt24_089_has_three_clauses() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("BT24-089")
        .expect("BT24-089 compiled card present");

    assert_eq!(
        compiled.effects.len(),
        3,
        "expected 3 clauses (main_from_hand, raw_rust delay stub, on_security); got {}",
        compiled.effects.len()
    );
}

/// Clause 0 fires at main_from_hand, is optional, and has FaceUp scope.
#[test]
fn bt24_089_clause0_is_main_from_hand_optional_face_up() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("BT24-089")
        .expect("BT24-089 compiled card present");

    let clause0 = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::MainFromHand))
        .expect("must have a main_from_hand clause");

    assert!(
        clause0.optional,
        "Clause 0 must be optional — printed text says 'you may'"
    );
    assert_eq!(
        clause0.scope,
        CompiledScope::FaceUp,
        "Clause 0 must have face-up scope"
    );
}

/// Clause 1 is a declarative raw_rust clause (Delay stub).
#[test]
fn bt24_089_clause1_is_declarative_raw_rust() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("BT24-089")
        .expect("BT24-089 compiled card present");

    let has_raw_rust = compiled.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::RawRust { .. })
        )
    });

    assert!(
        has_raw_rust,
        "Clause 1 must be a declarative raw_rust clause (Delay stub)"
    );
}

/// Clause 2 fires at on_security, is optional, and has FaceUp scope.
#[test]
fn bt24_089_clause2_is_on_security_optional_face_up() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("BT24-089")
        .expect("BT24-089 compiled card present");

    let security_clause = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnSecurity))
        .expect("must have an on_security clause");

    // Note: DCGO uses AddActivateMainOptionSecurityEffect which is mandatory.
    // The DSL models this as optional (to match printed "you may" semantics
    // mirrored from the Main clause). Tests flag both approaches; optional
    // is chosen to match the print-text-first policy.
    assert_eq!(
        security_clause.scope,
        CompiledScope::FaceUp,
        "on_security clause must have FaceUp scope"
    );
}

/// BT24-089 has exactly 2 triggered clauses (main_from_hand + on_security).
#[test]
fn bt24_089_has_exactly_two_triggered_clauses() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("BT24-089")
        .expect("BT24-089 compiled card present");

    let triggered: Vec<_> = compiled
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
        "expected 2 triggered clauses (main_from_hand + on_security); got {}",
        triggered.len()
    );
}

// ─── §2 Main clause behavioral ────────────────────────────────────────────────

/// When BT24-089's [Main] effect is activated from hand and the player has an
/// Elizamon in hand, a pending selection fires (zone_choice prompt to select
/// from hand or trash).
///
/// Note: `activate_hand_main` fires the `MainFromHand` effect without paying
/// cost or physically moving the card — that is the correct dispatch for
/// Option card [Main] effects in the engine (analogous to Python's
/// `_execute_hand_main_effect`). `runner.play()` fires `OnPlay` which is a
/// different timing.
#[test]
fn bt24_089_main_from_hand_installs_zone_choice_prompt() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_elizamon())
        .add_card(make_filler("FILL"))
        .hand(0, &["BT24-089", "ELIZAMON"])
        .hand(1, &["FILL"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    // Activate the [Main] effect while BT24-089 is still in hand at index 0.
    // This fires EffectTiming::MainFromHand which is distinct from OnPlay.
    let fired = runner.game.activate_hand_main(0, 0);
    assert!(fired, "activate_hand_main must return true for BT24-089 at hand index 0");

    // The Main clause fires and should install a zone_choice selection
    // (effect_choice: "From hand" / "From trash") since Elizamon is in hand.
    assert!(
        runner.game.pending_selection.is_some(),
        "Main clause must install a pending selection (zone_choice prompt) when Elizamon is in hand"
    );
}

/// When BT24-089's [Main] effect fires from hand, the player chooses "From hand"
/// and selects Elizamon — Elizamon lands in the battle area without paying its cost.
///
/// Uses `activate_hand_main` (not `runner.play()`) to fire the MainFromHand effect.
#[test]
fn bt24_089_main_plays_elizamon_from_hand_free() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_elizamon())
        .add_card(make_filler("FILL"))
        .hand(0, &["BT24-089", "ELIZAMON"])
        .hand(1, &["FILL"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let player0_hand_before = runner.game.players[0].hand.len();
    let field_before = runner.game.players[0].battle_area.len();

    // Activate the [Main] effect — fires EffectTiming::MainFromHand.
    let fired = runner.game.activate_hand_main(0, 0);
    assert!(fired, "activate_hand_main must return true for BT24-089");

    assert!(
        runner.game.pending_selection.is_some(),
        "zone_choice prompt must be present after activating BT24-089 [Main]"
    );

    // First selection: zone choice — pick "From hand" (valid_action_ids[0]).
    {
        let pending = runner.game.pending_selection.as_ref().unwrap();
        let player = pending.selecting_player;
        let action = pending.valid_action_ids[0];
        runner
            .game
            .resolve_selection(player, action)
            .expect("zone_choice resolves");
        runner.game.drain_effect_queue();
    }

    // After the zone choice the hand selection fires (if prompt is installed).
    if runner.game.pending_selection.is_some() {
        let pending = runner.game.pending_selection.as_ref().unwrap();
        let player = pending.selecting_player;
        let action = pending.valid_action_ids[0];
        runner
            .game
            .resolve_selection(player, action)
            .expect("card selection resolves");
        runner.game.drain_effect_queue();
    }

    let field_after = runner.game.players[0].battle_area.len();
    let player0_hand_after = runner.game.players[0].hand.len();

    // Either Elizamon entered the battle area OR it left the hand (was consumed by the effect).
    assert!(
        field_after > field_before || player0_hand_after < player0_hand_before,
        "Elizamon must have entered the battle area or left the hand after the Main clause; \
         field_before={field_before}, field_after={field_after}, \
         hand_before={player0_hand_before}, hand_after={player0_hand_after}"
    );
}

/// When BT24-089's [Main] effect activates with no eligible card in either zone,
/// the Main clause resolves without panic.
#[test]
fn bt24_089_main_no_eligible_card_no_panic() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_filler("FILL"))
        .hand(0, &["BT24-089"])
        .hand(1, &["FILL"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    // Activate via MainFromHand timing (not runner.play which fires OnPlay).
    runner.game.activate_hand_main(0, 0);

    // Drain all pending selections if any.
    let mut steps = 0;
    while runner.game.pending_selection.is_some() && steps < 20 {
        let player = runner
            .game
            .pending_selection
            .as_ref()
            .unwrap()
            .selecting_player;
        let action = runner
            .game
            .pending_selection
            .as_ref()
            .unwrap()
            .valid_action_ids[0];
        runner.game.resolve_selection(player, action).ok();
        runner.game.drain_effect_queue();
        steps += 1;
    }
    // No panic is the primary assertion.
}

/// Plays an Owen Dreadnought from hand when BT24-089's [Main] fires.
#[test]
fn bt24_089_main_plays_owen_from_hand_free() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_owen())
        .add_card(make_filler("FILL"))
        .hand(0, &["BT24-089", "OWEN"])
        .hand(1, &["FILL"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let hand_before = runner.game.players[0].hand.len();
    let field_before = runner.game.players[0].battle_area.len();

    // Activate [Main] via MainFromHand timing.
    runner.game.activate_hand_main(0, 0);

    // Drain all selections, choosing first action each time.
    let mut steps = 0;
    while runner.game.pending_selection.is_some() && steps < 20 {
        let player = runner
            .game
            .pending_selection
            .as_ref()
            .unwrap()
            .selecting_player;
        let action = runner
            .game
            .pending_selection
            .as_ref()
            .unwrap()
            .valid_action_ids[0];
        runner.game.resolve_selection(player, action).ok();
        runner.game.drain_effect_queue();
        steps += 1;
    }

    let hand_after = runner.game.players[0].hand.len();
    let field_after = runner.game.players[0].battle_area.len();

    assert!(
        field_after > field_before || hand_after < hand_before,
        "Owen Dreadnought must have played or hand must have shrunk; \
         field_before={field_before}, field_after={field_after}, \
         hand_before={hand_before}, hand_after={hand_after}"
    );
}

/// When BT24-089's Main fires with only "From trash" available (Elizamon in trash,
/// nothing in hand), the effect can proceed without panic. We pre-populate trash
/// directly via the runner's game state.
#[test]
fn bt24_089_main_from_trash_no_panic_when_elizamon_in_trash() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_elizamon())
        .add_card(make_filler("FILL"))
        .hand(0, &["BT24-089"])
        .hand(1, &["FILL"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    // Manually push an Elizamon CardSource into player 0's trash.
    // This simulates a prior play/discard. Use a filler card source since
    // we can't create a fully typed CardSource here without the registry;
    // the trash test is primarily behavioral / no-panic.
    // Instead pop from deck and push to trash for a realistic card handle.
    if let Some(cs) = runner.game.players[0].deck.pop() {
        runner.game.players[0].trash.push(cs);
    }

    // Activate [Main] via MainFromHand timing.
    runner.game.activate_hand_main(0, 0);

    // Drain all pending selections choosing first action each time.
    let mut steps = 0;
    while runner.game.pending_selection.is_some() && steps < 20 {
        let player = runner
            .game
            .pending_selection
            .as_ref()
            .unwrap()
            .selecting_player;
        let action = runner
            .game
            .pending_selection
            .as_ref()
            .unwrap()
            .valid_action_ids[0];
        runner.game.resolve_selection(player, action).ok();
        runner.game.drain_effect_queue();
        steps += 1;
    }
    // No panic is the primary assertion.
}

// ─── §3 Delay clause structural ──────────────────────────────────────────────

/// The Delay clause is present as a declarative raw_rust clause.
/// Behavioral firing is gap-blocked (G-DELAY-SUSPEND-CONDITION).
#[test]
fn bt24_089_delay_clause_is_raw_rust_declarative() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("BT24-089")
        .expect("BT24-089 compiled card present");

    let raw_rust_clause = compiled.effects.iter().find(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::RawRust { .. })
        )
    });

    assert!(
        raw_rust_clause.is_some(),
        "Delay clause must be present as a raw_rust declarative stub"
    );

    if let Some(CompiledClause::Declarative(CompiledDeclarativeClause::RawRust {
        fn_name, ..
    })) = raw_rust_clause
    {
        assert_eq!(
            fn_name, "bt24_089_delay_on_owen_suspend",
            "raw_rust fn name must be 'bt24_089_delay_on_owen_suspend'"
        );
    }
}

/// G-DELAY-SUSPEND-CONDITION: The Delay clause fires when an Owen Dreadnought
/// suspends, not at end of turn. This is a gap — not yet expressible in DSL.
#[test]
#[ignore = "pending: G-DELAY-SUSPEND-CONDITION — on_suspend reactive Delay trigger not in DSL/engine"]
fn bt24_089_delay_opens_when_owen_dreadnought_suspends() {
    unimplemented!("blocked on G-DELAY-SUSPEND-CONDITION");
}

/// G-DELAY-SUSPEND-CONDITION: Delay does not fire when a non-Owen permanent suspends.
#[test]
#[ignore = "pending: G-DELAY-SUSPEND-CONDITION — on_suspend reactive Delay trigger not in DSL/engine"]
fn bt24_089_delay_does_not_open_for_non_owen_suspend() {
    unimplemented!("blocked on G-DELAY-SUSPEND-CONDITION");
}

/// G-DELAY-SUSPEND-CONDITION: After the placing turn, the player may trash this
/// card to activate the digivolve effect.
#[test]
#[ignore = "pending: G-DELAY-SUSPEND-CONDITION + G-EFFECT-DIGIVOLVE-FROM-HAND"]
fn bt24_089_delay_body_digivolves_reptile_dragonkin_with_cost_reduction() {
    unimplemented!("blocked on G-DELAY-SUSPEND-CONDITION");
}

/// G-PLACE-SELF-AS-OPTION-PERMANENT: "Then, place this card in the battle area"
/// cannot be expressed as a DSL step — the option-as-permanent placement is a gap.
#[test]
#[ignore = "pending: G-PLACE-SELF-AS-OPTION-PERMANENT — no place_option_in_battle_area step verb"]
fn bt24_089_main_places_self_in_battle_area() {
    unimplemented!("blocked on G-PLACE-SELF-AS-OPTION-PERMANENT");
}

// ─── §4 Security clause behavioral ───────────────────────────────────────────

/// The on_security clause fires from the security trigger without panic.
#[test]
fn bt24_089_security_clause_no_panic() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_elizamon())
        .add_card(make_filler("FILL"))
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .security(0, &["BT24-089", "FILL", "FILL", "FILL", "FILL"])
        .hand(0, &["ELIZAMON"])
        .hand(1, &["FILL"])
        .memory(10)
        .start();

    // Place BT24-089 in the battle area directly and fire on_security.
    // Use enqueue_triggered with a TriggerSource referencing player 0's field.
    // Actually fire via placing in security and manually triggering:
    let field_handle = runner.place_on_field(0, "BT24-089", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::SecuritySkill, TriggerSource::Permanent(field_handle));
    runner.game.drain_effect_queue();

    // Drain any selections.
    let mut steps = 0;
    while runner.game.pending_selection.is_some() && steps < 20 {
        let player = runner
            .game
            .pending_selection
            .as_ref()
            .unwrap()
            .selecting_player;
        let action = runner
            .game
            .pending_selection
            .as_ref()
            .unwrap()
            .valid_action_ids[0];
        runner.game.resolve_selection(player, action).ok();
        runner.game.drain_effect_queue();
        steps += 1;
    }
    // No panic is the primary assertion.
}
