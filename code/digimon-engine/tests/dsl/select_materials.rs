//! Count-capped / name-unique `select_materials` multi-pick over a
//! permanent's digivolution-source stack (Phase 2 Track J substrate).
//!
//! `select_materials` is the batch sibling of the single-pick
//! `select_material`. It picks *up to N* digivolution sources from a
//! carrier permanent in ONE count-capped multi-pick, optionally
//! constrained by a name-uniqueness predicate ("1 of each different
//! name"). The picked sources are bound as a `CardList` that
//! `play_from_materials` can consume as a batch.
//!
//! Canonical scenario (King Drasil): a carrier with multiple Royal
//! Knight digivolution sources of duplicate AND distinct names. The
//! player chooses at most one source per distinct name; the chosen
//! sources leave the stack and enter the battle area as fresh
//! permanents. When `play_from_materials` consumes the batch with
//! `suppress_on_play: true`, the played Digimon's [On Play] effects
//! do not fire.
//!
//! No-approximations (CLAUDE.md §17): every source pick surfaces
//! through `pending_selection`; the name-uniqueness constraint shapes
//! the legal action mask, it never auto-picks.

use std::sync::Arc;

use digimon_dsl::compiled::{
    CompiledBindingRef, CompiledCostDelta, CompiledCountBound, CompiledDistinctBy,
    CompiledPredicate, CompiledStep,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind};

/// A Royal-Knight-trait Digimon test card used as a digivolution source.
fn royal_knight(card_id: &str, card_name: &str) -> CardData {
    let mut card = make_test_card(card_id, card_name);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::White];
    card.level = Some(6);
    card.play_cost = 12;
    card.dp = Some(11000);
    card.traits = vec!["Royal Knight".to_string()];
    card
}

/// Per-On-Play memory gain. Small enough that three stacked plays stay
/// inside the [-10, +10] memory gauge from a `memory(0)` start, so the
/// delta is observable (a `memory(10)` start would clamp every gain).
const ON_PLAY_GAIN: i16 = 1;

/// Hand-written [On Play] effect: gain `ON_PLAY_GAIN` memory. A test can
/// tell whether the On Play fired by inspecting the memory delta.
struct OnPlayGainMemory;
impl CardEffect for OnPlayGainMemory {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("On Play: gain memory")
            .process(|ctx| {
                ctx.gain_memory(ON_PLAY_GAIN);
            })
            .build()]
    }
}

/// Build a carrier permanent on P0's battle area whose digivolution
/// stack (bottom→top) is the given source ids followed by the carrier
/// top card. Returns the runner with the carrier at field index 0.
fn carrier_with_sources(source_ids: &[&str]) -> DebugRunner {
    // Each distinct Royal Knight name used as a source. Duplicate names
    // share a card_id so the name-uniqueness predicate can collapse them.
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(royal_knight("RK-ALPHA", "Alphamon"))
        .add_card(royal_knight("RK-OMEGA", "Omnimon"))
        .add_card(royal_knight("RK-GANK", "Gankoomon"))
        // memory(0): mid-gauge so [On Play] gains are observable (a +10
        // start would clamp every gain at the +10 ceiling).
        .memory(0)
        .start();
    // The On Play effect is registered after build via register_effect.
    runner.register_effect("RK-ALPHA", Arc::new(OnPlayGainMemory));
    runner.register_effect("RK-OMEGA", Arc::new(OnPlayGainMemory));
    runner.register_effect("RK-GANK", Arc::new(OnPlayGainMemory));

    // Build the stack: sources then CARRIER on top.
    let mut stack: Vec<&str> = source_ids.to_vec();
    stack.push("CARRIER");
    runner.place_stack(0, &stack);
    runner
}

/// Resolve every distinct card_id present among a carrier's
/// digivolution sources (top excluded).
fn source_card_ids(runner: &DebugRunner) -> Vec<String> {
    let perm = &runner.game.players[0].battle_area[0];
    let n = perm.card_sources.len().saturating_sub(1);
    perm.card_sources[..n]
        .iter()
        .map(|cs| runner.game.card_data[cs.data_index].card_id.clone())
        .collect()
}

#[test]
fn select_materials_name_uniqueness_caps_mask_to_one_per_name() {
    // Carrier stack: Alphamon, Omnimon, Omnimon (dup), Gankoomon.
    // Four sources, three distinct names.
    let mut runner = carrier_with_sources(&["RK-ALPHA", "RK-OMEGA", "RK-OMEGA", "RK-GANK"]);
    assert_eq!(
        source_card_ids(&runner),
        vec!["RK-ALPHA", "RK-OMEGA", "RK-OMEGA", "RK-GANK"],
    );

    let perm_handle = runner.perm_handle(0, 0);
    let src_card = runner.game.players[0].battle_area[0].top_card().handle();

    let mut bindings = Bindings::new();
    bindings.insert_permanent("carrier", perm_handle);

    let steps = vec![CompiledStep::SelectMaterials {
        of_permanent: CompiledBindingRef::Named("carrier".to_string()),
        max: CompiledCountBound::Literal(4),
        filter: CompiledPredicate {
            trait_has: Some("Royal Knight".to_string()),
            ..CompiledPredicate::default()
        },
        uniqueness: Some(CompiledDistinctBy::Name),
        bind_as: Some("picked".to_string()),
        prompt: "Pick 1 of each different name".to_string(),
        prompt_key: None,
        optional_zero: false,
    }];

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    // Step 1: all four sources are selectable (no pick made yet).
    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("select_materials must install a pending selection");
    assert_eq!(
        pending.valid_action_ids.len(),
        4,
        "step 1: all four Royal Knight sources are selectable"
    );

    // Pick the first Omnimon (index 1 in the stack).
    let omega_action = pending.valid_action_ids[1];
    let selecting_player = pending.selecting_player;
    runner
        .game
        .resolve_selection(selecting_player, omega_action)
        .expect("first pick");

    // Step 2: the OTHER Omnimon is filtered out by uniqueness=name.
    // Alphamon + Gankoomon remain (2 distinct names not yet picked).
    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("pending must re-arm after the first pick");
    assert_eq!(
        pending.valid_action_ids.len(),
        2,
        "after picking Omnimon, the duplicate Omnimon must be masked out \
         by uniqueness=name; Alphamon + Gankoomon stay selectable"
    );

    // Both remaining picks are distinct names — finish the multi-pick.
    let next = pending.valid_action_ids[0];
    runner
        .game
        .resolve_selection(selecting_player, next)
        .expect("second pick");
    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("pending must re-arm after the second pick");
    assert_eq!(
        pending.valid_action_ids.len(),
        1,
        "third step: only the final distinct name remains"
    );
    let last = pending.valid_action_ids[0];
    runner
        .game
        .resolve_selection(selecting_player, last)
        .expect("third pick");

    // Three distinct-name picks committed; no fourth (the dup was masked).
    assert!(
        runner.game.pending_selection.is_none(),
        "multi-pick auto-commits once no distinct names remain"
    );
}

#[test]
fn select_materials_batch_play_from_materials_suppresses_on_play() {
    // Carrier stack: Alphamon, Omnimon, Gankoomon — three distinct names.
    let mut runner = carrier_with_sources(&["RK-ALPHA", "RK-OMEGA", "RK-GANK"]);

    let perm_handle = runner.perm_handle(0, 0);
    let src_card = runner.game.players[0].battle_area[0].top_card().handle();
    let memory_before = runner.game.memory;
    let battle_before = runner.game.players[0].battle_area.len();

    let mut bindings = Bindings::new();
    bindings.insert_permanent("carrier", perm_handle);

    // select_materials → play_from_materials(source: picked, suppress_on_play).
    let steps = vec![
        CompiledStep::SelectMaterials {
            of_permanent: CompiledBindingRef::Named("carrier".to_string()),
            max: CompiledCountBound::Literal(4),
            filter: CompiledPredicate {
                trait_has: Some("Royal Knight".to_string()),
                ..CompiledPredicate::default()
            },
            uniqueness: Some(CompiledDistinctBy::Name),
            bind_as: Some("picked".to_string()),
            prompt: "Pick 1 of each different name".to_string(),
            prompt_key: None,
            optional_zero: false,
        },
        CompiledStep::PlayFromMaterials {
            target: CompiledBindingRef::Named("carrier".to_string()),
            source_index: CompiledBindingRef::Named("picked".to_string()),
            cost_delta: Some(CompiledCostDelta::Free),
            bind_as: None,
            suppress_on_play: true,
        },
    ];

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    // Drive the multi-pick: select all three distinct-name sources.
    for _ in 0..3 {
        let pending = runner
            .game
            .pending_selection
            .as_ref()
            .expect("pending selection while picks remain");
        let action = pending.valid_action_ids[0];
        let selecting_player = pending.selecting_player;
        runner
            .game
            .resolve_selection(selecting_player, action)
            .expect("pick");
    }
    assert!(
        runner.game.pending_selection.is_none(),
        "all three distinct names picked; multi-pick committed"
    );

    // Carrier stack drained of its three sources — only the carrier top
    // card remains.
    assert_eq!(
        runner.game.players[0].battle_area[0].card_sources.len(),
        1,
        "all three picked sources left the carrier's digivolution stack"
    );

    // Three new permanents entered P0's battle area (one per picked source).
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        battle_before + 3,
        "each picked source became a fresh battle-area permanent"
    );

    // Drain the queue: if any On Play had been (wrongly) enqueued, this is
    // where it would fire. With suppression it stays empty.
    runner.game.drain_effect_queue();

    // suppress_on_play composition: NONE of the played Digimon's [On Play]
    // (gain memory) fired. Free cost keeps memory; suppression keeps it
    // unchanged from before the batch play.
    assert_eq!(
        runner.game.memory, memory_before,
        "suppress_on_play: true must suppress every played source's On Play"
    );
}

#[test]
fn select_materials_without_suppression_fires_on_play_per_played_source() {
    // Negative branch: without suppress_on_play, each played source's
    // [On Play] fires. Proves the suppression flag is load-bearing.
    let mut runner = carrier_with_sources(&["RK-ALPHA", "RK-OMEGA", "RK-GANK"]);

    let perm_handle = runner.perm_handle(0, 0);
    let src_card = runner.game.players[0].battle_area[0].top_card().handle();
    let memory_before = runner.game.memory;

    let mut bindings = Bindings::new();
    bindings.insert_permanent("carrier", perm_handle);

    let steps = vec![
        CompiledStep::SelectMaterials {
            of_permanent: CompiledBindingRef::Named("carrier".to_string()),
            max: CompiledCountBound::Literal(4),
            filter: CompiledPredicate {
                trait_has: Some("Royal Knight".to_string()),
                ..CompiledPredicate::default()
            },
            uniqueness: Some(CompiledDistinctBy::Name),
            bind_as: Some("picked".to_string()),
            prompt: "Pick 1 of each different name".to_string(),
            prompt_key: None,
            optional_zero: false,
        },
        CompiledStep::PlayFromMaterials {
            target: CompiledBindingRef::Named("carrier".to_string()),
            source_index: CompiledBindingRef::Named("picked".to_string()),
            cost_delta: Some(CompiledCostDelta::Free),
            bind_as: None,
            suppress_on_play: false,
        },
    ];

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    for _ in 0..3 {
        let pending = runner
            .game
            .pending_selection
            .as_ref()
            .expect("pending selection while picks remain");
        let action = pending.valid_action_ids[0];
        let selecting_player = pending.selecting_player;
        runner
            .game
            .resolve_selection(selecting_player, action)
            .expect("pick");
    }

    // Drain the triggered-effect queue so the enqueued On Play clauses run.
    runner.game.drain_effect_queue();

    // Three sources played, each [On Play] gains `ON_PLAY_GAIN` memory.
    assert_eq!(
        runner.game.memory,
        memory_before + 3 * ON_PLAY_GAIN,
        "without suppression, each of the three played sources fires \
         its On Play (gain memory)"
    );
}

// ─── DSL lowering ────────────────────────────────────────────────────────────

#[test]
fn select_materials_yaml_lowers_uniqueness_max_and_filter() {
    // Authored YAML reaches the engine selection: a `select_materials`
    // step with `max`, `uniqueness: name`, and a `filter` predicate must
    // lower into `CompiledStep::SelectMaterials` carrying all three.
    let yaml = r#"
card: SM-LOWER
name: "select_materials lowering"
kind: digimon
level: 7
color: [white]
cost: 14
dp: 13000
effects:
  - when: when_digivolving
    process:
      - select_materials:
          target: king_drasil
          max: 4
          uniqueness: name
          filter:
            trait_has: "Royal Knight"
          bind_as: picked
          prompt: "Pick 1 of each different name"
      - play_from_materials:
          target: king_drasil
          source_index: picked
          cost_delta: free
          suppress_on_play: true
"#;

    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).expect("YAML parses");
    let compiled = digimon_dsl::compile::compile(&spec).expect("YAML compiles");
    let digimon_dsl::compiled::CompiledClause::Triggered(triggered) = &compiled.effects[0] else {
        panic!("expected triggered effect");
    };

    match &triggered.process[0] {
        CompiledStep::SelectMaterials {
            of_permanent,
            max,
            filter,
            uniqueness,
            bind_as,
            prompt,
            optional_zero,
            ..
        } => {
            assert_eq!(
                of_permanent,
                &CompiledBindingRef::Named("king_drasil".to_string()),
                "target carrier binding must lower onto of_permanent"
            );
            assert_eq!(max, &CompiledCountBound::Literal(4), "max: 4 must lower");
            assert_eq!(
                uniqueness,
                &Some(CompiledDistinctBy::Name),
                "uniqueness: name must lower to CompiledDistinctBy::Name"
            );
            assert_eq!(
                filter.trait_has.as_deref(),
                Some("Royal Knight"),
                "filter predicate must reach the engine selection"
            );
            assert_eq!(bind_as.as_deref(), Some("picked"));
            assert_eq!(prompt, "Pick 1 of each different name");
            assert!(!optional_zero, "default optional_zero is false");
        }
        other => panic!("expected SelectMaterials, got {other:?}"),
    }

    // The follow-on play_from_materials consumes the bound batch and
    // composes with the S1.1 suppress_on_play flag.
    match &triggered.process[1] {
        CompiledStep::PlayFromMaterials {
            target,
            source_index,
            suppress_on_play,
            ..
        } => {
            assert_eq!(
                target,
                &CompiledBindingRef::Named("king_drasil".to_string())
            );
            assert_eq!(
                source_index,
                &CompiledBindingRef::Named("picked".to_string()),
                "play_from_materials consumes the picked batch binding"
            );
            assert!(suppress_on_play, "suppress_on_play: true must lower");
        }
        other => panic!("expected PlayFromMaterials, got {other:?}"),
    }
}

#[test]
fn select_materials_empty_carrier_runs_tail_synchronously() {
    // A carrier with no digivolution sources (only its top card) yields
    // no candidates: select_materials no-ops and the tail runs.
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("CARRIER", "Carrier"))
        .memory(0)
        .start();
    let perm_handle = runner.place_on_field(0, "CARRIER", None);
    let src_card = runner.game.players[0].battle_area[0].top_card().handle();
    let memory_before = runner.game.memory;

    let mut bindings = Bindings::new();
    bindings.insert_permanent("carrier", perm_handle);

    let steps = vec![
        CompiledStep::SelectMaterials {
            of_permanent: CompiledBindingRef::Named("carrier".to_string()),
            max: CompiledCountBound::Literal(4),
            filter: CompiledPredicate::default(),
            uniqueness: Some(CompiledDistinctBy::Name),
            bind_as: Some("picked".to_string()),
            prompt: "Pick materials".to_string(),
            prompt_key: None,
            optional_zero: false,
        },
        CompiledStep::GainMemory(3),
    ];

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    assert!(runner.game.pending_selection.is_none());
    assert_eq!(
        runner.game.memory,
        memory_before + 3,
        "empty carrier → select_materials no-ops, the tail still runs"
    );
}
