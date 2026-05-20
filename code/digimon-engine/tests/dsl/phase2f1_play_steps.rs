//! Phase 2f1 Task 4 — play / digivolve / placement step lowerings.
//!
//! This test drives `CompiledStep::PlayFromHand` with a `Free` cost-delta
//! through the synchronous step dispatcher and asserts the wiring works:
//! - Memory is unchanged (Free → 0 cost).
//! - The hand index is consumed (hand shrinks by 1).
//! - The card lands on the battle area.

use std::sync::Arc;

use digimon_dsl::compiled::{
    CompiledBindingRef, CompiledCostDelta, CompiledPlayerRef, CompiledStep,
};
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::{run_step, run_steps, RunOutcome};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::rules::Rules;

struct OptionalDslPlayReducer;
impl CardEffect for OptionalDslPlayReducer {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::before_pay_cost(card)
            .name("Optional DSL play reducer")
            .optional()
            .cost_reduction(1)
            .build()]
    }
}

#[test]
fn play_from_hand_step_with_free_cost_delta_consumes_hand_and_keeps_memory() {
    // Setup: P0 hand has 2 cards. TST-A is the in-hand "source" of the
    // effect (we use its handle as `source_card` in the EffectContext);
    // TST-B is the card we want to play via `PlayFromHand`. Memory pre-
    // funded but irrelevant — Free cost-delta means no memory deduction.
    let card_a = make_test_card("TST-A", "A");
    let card_b = make_test_card("TST-B", "B");
    let mut runner = DebugRunner::builder()
        .add_card(card_a)
        .add_card(card_b)
        .hand(0, &["TST-A", "TST-B"])
        .memory(5)
        .start();

    let memory_before = runner.game.memory;
    let hand_before = runner.game.players[0].hand.len();
    let battle_before = runner.game.players[0].battle_area.len();

    // The "casting" card is TST-A at hand index 0 (the source of the effect).
    // Because we're going to drop the card at hand index 1 (TST-B), the
    // source's index doesn't shift. Source of EffectContext is just used
    // for trigger fan-out; we use TST-A's handle so the source is in a real
    // zone and not floating.
    let src_card = runner.game.players[0].hand[0].handle();

    // Bind "idx" → HandIndex(1) so PlayFromHand drops TST-B.
    let mut bindings = Bindings::new();
    bindings.insert_hand_index("idx", 0, 1);

    let step = CompiledStep::PlayFromHand {
        of: CompiledPlayerRef::You,
        hand_index: CompiledBindingRef::Named("idx".into()),
        cost_delta: Some(CompiledCostDelta::Free),
    };

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        run_step(&step, &mut ctx, &mut bindings);
    }

    // Memory unchanged — Free resolves to 0.
    assert_eq!(
        runner.game.memory, memory_before,
        "memory should be unchanged with CostDelta::Free"
    );
    // Hand shrinks by 1.
    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before - 1,
        "hand should shrink by 1 (TST-B left for battle area)"
    );
    // Battle area gains 1 permanent.
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        battle_before + 1,
        "battle area should gain 1 permanent (TST-B)"
    );
}

#[test]
fn play_from_hand_step_parks_tail_behind_pending_cost_reducer() {
    let mut rules = Rules::standard();
    rules.memory_range = (-20, 25);
    let mut runner = DebugRunner::builder()
        .with_rules(rules)
        .add_card(make_test_card("REDUCER", "Reducer"))
        .add_card(make_test_card("PLAY-ME", "PlayMe"))
        .hand(0, &["PLAY-ME"])
        .memory(20)
        .start();
    runner.register_effect("REDUCER", Arc::new(OptionalDslPlayReducer));
    let reducer = runner.place_on_field(0, "REDUCER", Some(0));
    runner.game_mut().enter_main_phase();

    let mut bindings = Bindings::new();
    bindings.insert_hand_index("idx", 0, 0);
    let steps = vec![
        CompiledStep::PlayFromHand {
            of: CompiledPlayerRef::You,
            hand_index: CompiledBindingRef::Named("idx".into()),
            cost_delta: Some(CompiledCostDelta::Printed),
        },
        CompiledStep::GainMemory(3),
    ];

    let source_card = runner.game.players[0].battle_area[reducer.index as usize]
        .top_card()
        .handle();
    let outcome = {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(reducer), 0);
        run_steps(&steps, &mut ctx, &mut bindings)
    };

    assert_eq!(outcome, RunOutcome::Parked);
    assert!(runner.pending_selection().is_some());
    assert_eq!(
        runner.memory(),
        20,
        "tail GainMemory must not run before the reducer choice resolves"
    );
    assert_eq!(runner.game.players[0].hand.len(), 1);
    assert!(runner.game.players[0]
        .battle_area
        .iter()
        .all(|perm| perm.top_card().card_id(&runner.game.card_data) != "PLAY-ME"));

    runner.execute_branch(0).expect("accept reducer");

    assert!(runner.pending_selection().is_none());
    assert_eq!(
        runner.memory(),
        21,
        "PLAY-ME costs 3, reducer lowers by 1, then deferred tail gains 3"
    );
    assert!(runner.game.players[0].hand.is_empty());
    assert!(runner.game.players[0]
        .battle_area
        .iter()
        .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "PLAY-ME"));
}

// ─── PlayFromHandFree ────────────────────────────────────────────────────────
//
// Per-variant happy-path coverage for Phase 2f1 Task 5. Each test below drives
// one of the 11 newly-wired variants through `run_step` (or `run_steps`) and
// asserts the observable mutation matches the engine primitive.

#[test]
fn play_from_hand_free_step_consumes_hand_and_keeps_memory() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("TST-PFHF", "PlayFromHandFreeTest"))
        .hand(0, &["TST-PFHF"])
        .memory(5)
        .start();

    let src_card = runner.game.players[0].hand[0].handle();
    let memory_before = runner.game.memory;
    let hand_before = runner.game.players[0].hand.len();
    let battle_before = runner.game.players[0].battle_area.len();

    let mut bindings = Bindings::new();
    bindings.insert_hand_index("idx", 0, 0);

    let step = CompiledStep::PlayFromHandFree {
        of: CompiledPlayerRef::You,
        hand_index: CompiledBindingRef::Named("idx".into()),
        bind_as: None,
    };

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        run_step(&step, &mut ctx, &mut bindings);
    }

    assert_eq!(
        runner.game.memory, memory_before,
        "PlayFromHandFree must not change memory"
    );
    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before - 1,
        "hand shrinks by 1"
    );
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        battle_before + 1,
        "battle area gains 1 permanent"
    );
}

#[test]
fn play_from_hand_free_step_uses_bound_hand_owner() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("P0-SRC", "P0Source"))
        .add_card(make_test_card("P1-PLAY", "P1Play"))
        .hand(0, &["P0-SRC"])
        .hand(1, &["P1-PLAY"])
        .memory(5)
        .start();

    let src_card = runner.game.players[0].hand[0].handle();
    let p0_hand_before = runner.game.players[0].hand.len();
    let p1_battle_before = runner.game.players[1].battle_area.len();

    let mut bindings = Bindings::new();
    bindings.insert_hand_index_for("idx", 1, 0);

    let step = CompiledStep::PlayFromHandFree {
        of: CompiledPlayerRef::You,
        hand_index: CompiledBindingRef::Named("idx".into()),
        bind_as: None,
    };

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        run_step(&step, &mut ctx, &mut bindings);
    }

    assert_eq!(
        runner.game.players[0].hand.len(),
        p0_hand_before,
        "P0 hand is untouched"
    );
    assert!(
        runner.game.players[1].hand.is_empty(),
        "P1 hand card is consumed"
    );
    assert_eq!(
        runner.game.players[1].battle_area.len(),
        p1_battle_before + 1,
        "P1 plays the card carried by the owned hand binding"
    );
    assert_eq!(
        runner.game.players[1].battle_area[p1_battle_before]
            .top_card()
            .card_id(&runner.game.card_data),
        "P1-PLAY"
    );
}

// ─── PlayFromTrash ───────────────────────────────────────────────────────────

#[test]
fn play_from_trash_step_with_free_cost_delta_consumes_trash_and_keeps_memory() {
    use digimon_engine::card_source::CardSource;

    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("TST-PFT", "PlayFromTrashTest"))
        .memory(5)
        .start();

    // Push the card into P0's trash directly.
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "TST-PFT")
        .unwrap();
    let card_index = runner.game.next_card_index();
    let card = CardSource::new(data_idx, 0, card_index);
    let src_card = card.handle();
    runner.game.players[0].trash.push(card);

    let memory_before = runner.game.memory;
    let trash_before = runner.game.players[0].trash.len();
    let battle_before = runner.game.players[0].battle_area.len();

    let mut bindings = Bindings::new();
    bindings.insert_trash_index("idx", 0, 0);

    let step = CompiledStep::PlayFromTrash {
        of: CompiledPlayerRef::You,
        trash_index: CompiledBindingRef::Named("idx".into()),
        cost_delta: Some(CompiledCostDelta::Free),
    };

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        run_step(&step, &mut ctx, &mut bindings);
    }

    assert_eq!(
        runner.game.memory, memory_before,
        "Free cost delta keeps memory"
    );
    assert_eq!(
        runner.game.players[0].trash.len(),
        trash_before - 1,
        "trash shrinks by 1"
    );
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        battle_before + 1,
        "battle area gains 1 permanent"
    );
}

// ─── PlayFromTrashFree ───────────────────────────────────────────────────────

#[test]
fn play_from_trash_free_step_consumes_trash_and_keeps_memory() {
    use digimon_engine::card_source::CardSource;

    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("TST-PFTF", "PlayFromTrashFreeTest"))
        .memory(5)
        .start();

    // Push the card into P0's trash directly.
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "TST-PFTF")
        .unwrap();
    let card_index = runner.game.next_card_index();
    let card = CardSource::new(data_idx, 0, card_index);
    let src_card = card.handle();
    runner.game.players[0].trash.push(card);

    let memory_before = runner.game.memory;
    let trash_before = runner.game.players[0].trash.len();
    let battle_before = runner.game.players[0].battle_area.len();

    // Bind by trash index (the dispatcher resolves the index → CardHandle
    // before calling `play_from_trash_free_unsuspended`).
    let mut bindings = Bindings::new();
    bindings.insert_trash_index("idx", 0, 0);

    let step = CompiledStep::PlayFromTrashFree {
        of: CompiledPlayerRef::You,
        trash_index: CompiledBindingRef::Named("idx".into()),
        suppress_on_play: false,
    };

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        run_step(&step, &mut ctx, &mut bindings);
    }

    assert_eq!(
        runner.game.memory, memory_before,
        "PlayFromTrashFree keeps memory"
    );
    assert_eq!(
        runner.game.players[0].trash.len(),
        trash_before - 1,
        "trash shrinks by 1 after dispatcher resolves index → handle"
    );
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        battle_before + 1,
        "battle area gains 1 permanent"
    );
    // The new permanent's top card is the one we placed in trash.
    let top = runner.game.players[0].battle_area[0].top_card();
    assert_eq!(
        top.handle(),
        src_card,
        "permanent top must be the same CardSource handle as the trashed card"
    );
}

// ─── PlayFromSecurity ────────────────────────────────────────────────────────

#[test]
fn play_from_security_step_consumes_security_and_keeps_memory() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("TST-PFS", "PlayFromSecurityTest"))
        .security(0, &["TST-PFS"])
        .memory(5)
        .start();

    // The dispatcher uses ctx.player as the controller; we pass 0 as the
    // EffectContext's player.
    let src_card = runner.game.players[0].security[0].handle();
    let memory_before = runner.game.memory;
    let security_before = runner.game.players[0].security.len();
    let battle_before = runner.game.players[0].battle_area.len();

    let mut bindings = Bindings::new();

    // PlayFromSecurity carries no fields.
    let step = CompiledStep::PlayFromSecurity;

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        run_step(&step, &mut ctx, &mut bindings);
    }

    assert_eq!(
        runner.game.memory, memory_before,
        "PlayFromSecurity keeps memory"
    );
    assert_eq!(
        runner.game.players[0].security.len(),
        security_before - 1,
        "security stack shrinks by 1"
    );
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        battle_before + 1,
        "battle area gains 1 permanent"
    );
}

// ─── PlayFromMaterials ───────────────────────────────────────────────────────

#[test]
fn play_from_materials_step_extracts_source_and_plays_it_free() {
    use digimon_engine::card_source::CardSource;

    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("T-BASE", "Base"))
        .add_card(make_test_card("T-TOP", "Top"))
        .memory(5)
        .start();

    // Build a 2-source permanent on P0's battle area: base + top.
    let perm_handle = runner.place_on_field(0, "T-BASE", None);
    let base_h = runner.game.players[0].battle_area[0].card_sources[0].handle();
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "T-TOP")
        .unwrap();
    let card_index = runner.game.next_card_index();
    let top_src = CardSource::new(data_idx, 0, card_index);
    runner.game.players[0].battle_area[0]
        .card_sources
        .push(top_src);
    assert_eq!(runner.game.players[0].battle_area[0].stack_size(), 2);

    let src_card = runner.game.players[0].battle_area[0].top_card().handle();
    let memory_before = runner.game.memory;

    // Bind the permanent and the literal source index (0 = bottom = base).
    let mut bindings = Bindings::new();
    bindings.insert_permanent("tgt", perm_handle);
    bindings.insert_literal("src_idx", 0);

    let step = CompiledStep::PlayFromMaterials {
        target: CompiledBindingRef::Named("tgt".into()),
        source_index: CompiledBindingRef::Named("src_idx".into()),
        cost_delta: Some(CompiledCostDelta::Free),
        bind_as: None,
    };

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        run_step(&step, &mut ctx, &mut bindings);
    }

    assert_eq!(
        runner.game.memory, memory_before,
        "Free cost delta keeps memory"
    );
    // Base source extracted; original permanent now has 1 source (top only).
    assert_eq!(
        runner.game.players[0].battle_area[0].card_sources.len(),
        1,
        "base source removed from original permanent"
    );
    // A new permanent appeared on P0's battle area; its top card matches the
    // extracted base handle.
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        2,
        "a new permanent should have been added"
    );
    // Find the new permanent (the one whose top is base_h).
    let new_perm_idx = runner.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().handle() == base_h)
        .expect("new permanent must contain the extracted base source");
    assert_ne!(
        new_perm_idx, perm_handle.index as usize,
        "new permanent must occupy a different slot from the source perm"
    );
}

// ─── PlayToken ───────────────────────────────────────────────────────────────

#[test]
fn play_token_step_creates_token_permanent_in_battle_area() {
    // The default token registry (built by Game::new) registers
    // "petrification" → "TOKEN_PETRIFICATION". Use that name so the
    // dispatcher's `ctx.play_token(p, token_name)` lookup succeeds.
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("TST-PT", "PlayTokenTest"))
        .hand(0, &["TST-PT"])
        .memory(5)
        .start();

    let src_card = runner.game.players[0].hand[0].handle();
    let battle_before = runner.game.players[0].battle_area.len();

    let mut bindings = Bindings::new();
    let step = CompiledStep::PlayToken {
        controller: CompiledPlayerRef::You,
        token_name: "petrification".to_string(),
        bind_as: None,
    };

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        run_step(&step, &mut ctx, &mut bindings);
    }

    assert_eq!(
        runner.game.players[0].battle_area.len(),
        battle_before + 1,
        "battle area should gain a token permanent"
    );
    // The newly-created permanent's top card is a Token-kind card.
    let top = runner.game.players[0].battle_area[battle_before].top_card();
    let cd = &runner.game.card_data[top.data_index];
    assert_eq!(
        cd.card_kind,
        digimon_engine::enums::CardKind::Token,
        "created permanent must be CardKind::Token"
    );
    assert_eq!(
        cd.card_id, "TOKEN_PETRIFICATION",
        "created permanent's card_id must match the registered token's card_id"
    );
}

// ─── suppress_on_play flag — DSL lowering + behavior (Task S1.1) ──────────────
//
// Verifies the `suppress_on_play: true` YAML flag survives parse → compile
// into the `CompiledStep` variant, and that lowering it through the step
// dispatcher actually suppresses the played permanent's [On Play] effect.

/// `play_from_trash_free: { suppress_on_play: true }` in YAML reaches the
/// `CompiledStep::PlayFromTrashFree { suppress_on_play: true }` engine flag.
/// PUPPETS-G030: `suppress_on_play` is honored ONLY by `play_from_trash_free`.
#[test]
fn suppress_on_play_flag_lowers_from_yaml_to_compiled_step() {
    let yaml = r#"
card: TST-SUP
name: SuppressTest
kind: digimon
level: 3
color: [purple]
cost: 1
dp: 3000
effects:
  - when: on_play
    process:
      - select_trash:
          of: you
          bind_as: revived
          filter:
            level_lte: 3
          prompt: "Pick a Digimon to play"
      - play_from_trash_free:
          of: you
          hand_index: revived
          suppress_on_play: true
"#;
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).expect("YAML parses");
    let compiled = digimon_dsl::compile::compile(&spec).expect("compiles cleanly");

    let play_step = find_play_from_trash_free_step(&compiled);
    match play_step {
        CompiledStep::PlayFromTrashFree {
            suppress_on_play, ..
        } => assert!(
            *suppress_on_play,
            "suppress_on_play: true in YAML must lower to the compiled flag"
        ),
        other => panic!("expected PlayFromTrashFree, got {other:?}"),
    }
}

/// Locate the single `PlayFromTrashFree` step inside a compiled card's first
/// triggered clause's `process`.
fn find_play_from_trash_free_step(
    compiled: &digimon_dsl::compiled::CompiledCard,
) -> &CompiledStep {
    use digimon_dsl::compiled::CompiledClause;
    compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .flat_map(|t| t.process.iter())
        .find(|s| matches!(s, CompiledStep::PlayFromTrashFree { .. }))
        .expect("a PlayFromTrashFree step is present")
}

/// When `suppress_on_play` is omitted, the compiled step defaults to `false`
/// — existing card YAML is unaffected.
#[test]
fn suppress_on_play_flag_defaults_to_false_when_omitted() {
    let yaml = r#"
card: TST-NOSUP
name: NoSuppressTest
kind: digimon
level: 3
color: [purple]
cost: 1
dp: 3000
effects:
  - when: on_play
    process:
      - select_trash:
          of: you
          bind_as: revived
          filter:
            level_lte: 3
          prompt: "Pick a Digimon to play"
      - play_from_trash_free:
          of: you
          hand_index: revived
"#;
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).expect("YAML parses");
    let compiled = digimon_dsl::compile::compile(&spec).expect("compiles cleanly");
    match find_play_from_trash_free_step(&compiled) {
        CompiledStep::PlayFromTrashFree {
            suppress_on_play, ..
        } => assert!(
            !*suppress_on_play,
            "omitted suppress_on_play must default to false"
        ),
        other => panic!("expected PlayFromTrashFree, got {other:?}"),
    }
}

/// PUPPETS-G030: `suppress_on_play` is supported ONLY by `play_from_trash_free`.
/// Authoring it on `play_from_trash` must produce a `ValidationError` so an
/// author cannot silently expect a no-op flag to take effect.
#[test]
fn suppress_on_play_on_play_from_trash_is_rejected_at_compile_time() {
    let yaml = r#"
card: TST-BADSUP
name: BadSuppressTest
kind: digimon
level: 3
color: [purple]
cost: 1
dp: 3000
effects:
  - when: on_play
    process:
      - select_trash:
          of: you
          bind_as: revived
          filter:
            level_lte: 3
          prompt: "Pick a Digimon to play"
      - play_from_trash:
          of: you
          hand_index: revived
          cost_delta: free
          suppress_on_play: true
"#;
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).expect("YAML parses");
    let err = digimon_dsl::compile::compile(&spec)
        .expect_err("suppress_on_play on play_from_trash must fail compilation");
    assert!(
        err.iter()
            .any(|e| e.message.contains("suppress_on_play is only supported on play_from_trash_free")),
        "expected a play_from_trash suppress_on_play rejection, got {err:?}"
    );
}

/// End-to-end: the lowered `CompiledStep::PlayFromTrashFree { suppress_on_play:
/// true }` step, run through the dispatcher, plays the trash card into the
/// battle area but does NOT fire its [On Play] effect.
#[test]
fn suppress_on_play_step_lowering_skips_played_permanents_on_play() {
    use digimon_engine::card_source::CardSource;
    use digimon_engine::effect::Effect;

    // A Digimon whose [On Play] gains 7 memory — distinct sentinel value.
    struct OnPlayGain7;
    impl CardEffect for OnPlayGain7 {
        fn effects(&self, card: CardHandle) -> Vec<Effect> {
            vec![Effect::on_play(card)
                .name("On Play: gain 7 memory")
                .process(|ctx| {
                    ctx.gain_memory(7);
                })
                .build()]
        }
    }

    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("TST-SRC", "Source"))
        .add_card(make_test_card("TST-ONPLAY", "OnPlayCard"))
        .memory(0)
        .start();
    runner.register_effect("TST-ONPLAY", Arc::new(OnPlayGain7));

    // Push the [On Play] card into P0's trash.
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "TST-ONPLAY")
        .unwrap();
    let card_index = runner.game.next_card_index();
    runner.game.players[0]
        .trash
        .push(CardSource::new(data_idx, 0, card_index));

    let src_card = {
        let card_index = runner.game.next_card_index();
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "TST-SRC")
            .unwrap();
        let card = CardSource::new(data_idx, 0, card_index);
        let handle = card.handle();
        runner.game.players[0].hand.push(card);
        handle
    };

    let mut bindings = Bindings::new();
    bindings.insert_trash_index("idx", 0, 0);

    // suppress_on_play = true — the [On Play] gain-7 must NOT fire.
    let suppressed = CompiledStep::PlayFromTrashFree {
        of: CompiledPlayerRef::You,
        trash_index: CompiledBindingRef::Named("idx".into()),
        suppress_on_play: true,
    };
    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        run_step(&suppressed, &mut ctx, &mut bindings);
    }

    assert_eq!(
        runner.game.players[0].battle_area.len(),
        1,
        "the trash card must still enter the battle area"
    );
    assert_eq!(
        runner.game.memory, 0,
        "suppress_on_play: true must skip the played permanent's [On Play] \
         gain-7 memory effect"
    );
}
