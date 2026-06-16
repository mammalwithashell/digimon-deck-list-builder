//! Phase 2g: source selections can bind stable cross-permanent source refs and
//! consume them from later DSL steps.

use digimon_dsl::compiled::{CompiledBindingRef, CompiledCountBound, CompiledPredicate, CompiledStep};
use digimon_engine::action::space::{encode_source_select, PASS};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::{run_steps, RunOutcome};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::selection::SelectionKind;

#[test]
fn select_own_sources_binds_source_refs_for_trashing() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC-A", "Source A"))
        .add_card(make_test_card("TOP-A", "Top A"))
        .add_card(make_test_card("SRC-B", "Source B"))
        .add_card(make_test_card("TOP-B", "Top B"))
        .add_card(make_test_card("EFFECT", "Effect"))
        .hand(0, &["EFFECT"])
        .start();

    let stack_a = runner.place_stack(0, &["SRC-A", "TOP-A"]);
    let stack_b = runner.place_stack(0, &["SRC-B", "TOP-B"]);
    let source_card = runner.game.players[0].hand[0].handle();

    let steps = vec![CompiledStep::SelectOwnSources {
        target: None,
        filter: CompiledPredicate::default(),
        min: 2,
        max: 2,
        bind_as: Some("chosen_sources".to_string()),
        prompt: "Choose two sources".to_string(),
        then: vec![CompiledStep::TrashSelectedSources {
            source_refs: "chosen_sources".to_string(),
        }],
    }];

    let mut bindings = Bindings::new();
    let outcome = {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, None, 0);
        run_steps(&steps, &mut ctx, &mut bindings)
    };

    assert_eq!(outcome, RunOutcome::Parked);
    assert_eq!(
        runner.game.pending_selection.as_ref().map(|s| s.kind),
        Some(SelectionKind::SourceMulti {
            min: 2,
            max: 2,
            picked: 0,
        })
    );

    let action_a = encode_source_select(stack_a.index as u16, 0).expect("stack A source action");
    let action_b = encode_source_select(stack_b.index as u16, 0).expect("stack B source action");
    runner.execute_action(0, action_a).expect("pick source A");
    runner.execute_action(0, action_b).expect("pick source B");

    assert!(runner.game.pending_selection.is_none());
    assert_eq!(
        runner.game.players[0].battle_area[stack_a.index as usize]
            .card_sources
            .len(),
        1
    );
    assert_eq!(
        runner.game.players[0].battle_area[stack_b.index as usize]
            .card_sources
            .len(),
        1
    );
    assert_eq!(runner.game.players[0].trash.len(), 2);
}

#[test]
fn empty_select_own_sources_silently_skips_outer_tail_when_min_positive() {
    // When SelectOwnSources has min > 0 and no source candidates exist, the
    // mandatory cost cannot be paid. Per the no-approximations silent-skip
    // contract the ENTIRE effect body — including subsequent outer-tail steps —
    // is skipped. Memory stays at 0 (neither GainMemory(7) nor GainMemory(3)
    // runs). This mirrors how the EX10-032 buff clause behaves when no
    // Mineral/Rock source is available.
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("EFFECT", "Effect"))
        .hand(0, &["EFFECT"])
        .start();
    let source_card = runner.game.players[0].hand[0].handle();

    let steps = vec![
        CompiledStep::SelectOwnSources {
            target: None,
            filter: CompiledPredicate::default(),
            min: 1,
            max: 1,
            bind_as: Some("chosen_sources".to_string()),
            prompt: "Choose source".to_string(),
            then: vec![CompiledStep::GainMemory(7)],
        },
        CompiledStep::GainMemory(3),
    ];

    let mut bindings = Bindings::new();
    let outcome = {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, None, 0);
        run_steps(&steps, &mut ctx, &mut bindings)
    };

    assert_eq!(outcome, RunOutcome::Synchronous);
    assert!(runner.game.pending_selection.is_none());
    // Neither the `then` body nor the outer tail ran — mandatory cost unpayable.
    assert_eq!(runner.game.memory, 0);
}

#[test]
fn digi_burst_two_selects_exact_self_sources_and_fires_source_trash_per_card() {
    let burst_yaml = r#"
card: DSL-DIGI-BURST-2
name: Digi-Burst Two
kind: digimon
level: 5
color: [black]
cost: 8
dp: 7000
effects:
  - when: main_on_field
    process:
      - digi_burst:
          count: 2
          bind_as: burst_sources
          prompt: "Choose 2 digivolution cards under this Digimon"
          then:
            - gain_memory: 3
"#;
    let observer_yaml = r#"
card: DSL-SOURCE-TRASH-OBSERVER
name: Source Trash Observer
kind: digimon
level: 3
color: [red]
cost: 3
dp: 1000
effects:
  - when: on_digivolution_card_trashed
    condition:
      all_of:
        - event_target_owner: you
    process:
      - gain_memory: 1
"#;

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(burst_yaml)
        .expect("burst fixture compiles")
        .from_dsl_yaml(observer_yaml)
        .expect("observer fixture compiles")
        .add_card(make_test_card("SRC-A", "Source A"))
        .add_card(make_test_card("SRC-B", "Source B"))
        .add_card(make_test_card("SRC-C", "Source C"))
        .add_card(make_test_card("OTHER-SRC", "Other Source"))
        .add_card(make_test_card("OTHER-HOST", "Other Host"))
        .build();
    let carrier = runner.place_stack(0, &["SRC-A", "SRC-B", "SRC-C", "DSL-DIGI-BURST-2"]);
    let other = runner.place_stack(0, &["OTHER-SRC", "OTHER-HOST"]);
    runner.place_on_field(0, "DSL-SOURCE-TRASH-OBSERVER", None);

    assert!(
        runner.game.activate_field_main(0, carrier.index as usize),
        "Digi-Burst fixture should expose its Main effect"
    );
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::SourceMulti {
            min: 2,
            max: 2,
            picked: 0,
        })
    );

    let pick_a = encode_source_select(carrier.index as u16, 0).expect("source A action");
    let pick_b = encode_source_select(carrier.index as u16, 1).expect("source B action");
    let pick_c = encode_source_select(carrier.index as u16, 2).expect("source C action");
    let other_pick = encode_source_select(other.index as u16, 0).expect("other source action");
    {
        let selection = runner
            .game
            .pending_selection
            .as_ref()
            .expect("source cost pending");
        assert!(selection.valid_action_ids.contains(&pick_a));
        assert!(selection.valid_action_ids.contains(&pick_b));
        assert!(selection.valid_action_ids.contains(&pick_c));
        assert!(
            !selection.valid_action_ids.contains(&other_pick),
            "Digi-Burst must not expose sources under another own stack"
        );
        assert!(
            !selection.valid_action_ids.contains(&PASS),
            "exact-N Digi-Burst must not allow passing before N sources are chosen"
        );
    }

    runner
        .execute_action(0, pick_a)
        .expect("first Digi-Burst source pick resolves");
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::SourceMulti {
            min: 2,
            max: 2,
            picked: 1,
        })
    );
    {
        let selection = runner
            .game
            .pending_selection
            .as_ref()
            .expect("second source still pending");
        assert!(!selection.valid_action_ids.contains(&pick_a));
        assert!(selection.valid_action_ids.contains(&pick_b));
        assert!(selection.valid_action_ids.contains(&pick_c));
        assert!(!selection.valid_action_ids.contains(&other_pick));
        assert!(!selection.valid_action_ids.contains(&PASS));
    }

    runner
        .execute_action(0, pick_b)
        .expect("second Digi-Burst source pick resolves");

    assert!(runner.game.pending_selection.is_none());
    assert_eq!(
        runner.game.players[0].battle_area[carrier.index as usize]
            .card_sources
            .len(),
        2,
        "two selected sources should be trashed from the carrier stack"
    );
    assert_eq!(
        runner.game.players[0].battle_area[other.index as usize]
            .card_sources
            .len(),
        2,
        "other own stack should be untouched"
    );
    assert_eq!(runner.game.players[0].trash.len(), 2);
    assert_eq!(
        runner.game.memory, 5,
        "two source-trash observer firings (+2) should resolve before/alongside the nested body (+3)"
    );
}

// ─── S7 `select_opponent_sources` (G-SELECT-OPPONENT-SOURCES) ─────────────────

#[test]
fn select_opponent_sources_binds_opponent_source_refs_for_trashing() {
    // BT16-085 DNA branch shape: pick digivolution cards under the OPPONENT's
    // battle-area stacks and trash them.
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("OPP-SRC-A", "Opp Source A"))
        .add_card(make_test_card("OPP-TOP-A", "Opp Top A"))
        .add_card(make_test_card("OPP-SRC-B", "Opp Source B"))
        .add_card(make_test_card("OPP-TOP-B", "Opp Top B"))
        .add_card(make_test_card("OWN-SRC", "Own Source"))
        .add_card(make_test_card("OWN-TOP", "Own Top"))
        .add_card(make_test_card("EFFECT", "Effect"))
        .hand(0, &["EFFECT"])
        .start();

    let opp_a = runner.place_stack(1, &["OPP-SRC-A", "OPP-TOP-A"]);
    let opp_b = runner.place_stack(1, &["OPP-SRC-B", "OPP-TOP-B"]);
    let own = runner.place_stack(0, &["OWN-SRC", "OWN-TOP"]);
    let source_card = runner.game.players[0].hand[0].handle();

    let steps = vec![CompiledStep::SelectOpponentSources {
        target: None,
        filter: CompiledPredicate::default(),
        min: CompiledCountBound::Literal(2),
        max: CompiledCountBound::Literal(2),
        clamp_to_available: false,
        bind_as: Some("chosen_sources".to_string()),
        prompt: "Choose two opponent sources".to_string(),
        then: vec![CompiledStep::TrashSelectedSources {
            source_refs: "chosen_sources".to_string(),
        }],
    }];

    let mut bindings = Bindings::new();
    let outcome = {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, None, 0);
        run_steps(&steps, &mut ctx, &mut bindings)
    };
    assert_eq!(outcome, RunOutcome::Parked);
    assert_eq!(
        runner.game.pending_selection.as_ref().map(|s| s.kind),
        Some(SelectionKind::SourceMulti {
            min: 2,
            max: 2,
            picked: 0,
        })
    );

    // The controller (P0) chooses; candidates are OPPONENT (P1) sources.
    // Source-select action IDs carry no player dimension, so the candidate
    // count (one per opponent source) is the soundness check that own sources
    // are excluded: 2 opponent stacks × 1 source each = exactly 2 candidates
    // (the PASS action is not added at an exact-N minimum).
    let action_a = encode_source_select(opp_a.index as u16, 0).expect("opp A source action");
    let action_b = encode_source_select(opp_b.index as u16, 0).expect("opp B source action");
    {
        let pending = runner.game.pending_selection.as_ref().expect("pending");
        assert_eq!(pending.selecting_player, 0, "controller picks");
        assert!(pending.valid_action_ids.contains(&action_a));
        assert!(pending.valid_action_ids.contains(&action_b));
        assert_eq!(
            pending.valid_action_ids.len(),
            2,
            "only the 2 opponent sources are candidates — own sources excluded"
        );
    }

    runner.execute_action(0, action_a).expect("pick opp A");
    runner.execute_action(0, action_b).expect("pick opp B");

    assert!(runner.game.pending_selection.is_none());
    assert_eq!(
        runner.game.players[1].battle_area[opp_a.index as usize]
            .card_sources
            .len(),
        1
    );
    assert_eq!(
        runner.game.players[1].battle_area[opp_b.index as usize]
            .card_sources
            .len(),
        1
    );
    assert_eq!(
        runner.game.players[0].battle_area[own.index as usize]
            .card_sources
            .len(),
        2,
        "controller's own stack untouched"
    );
    // Trashed opponent sources go to their OWNER's (P1's) trash.
    assert_eq!(runner.game.players[1].trash.len(), 2);
}

#[test]
fn select_opponent_sources_restricts_to_target_opponent_permanent() {
    // `target:` binding limits the picker to one opponent permanent's sources.
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("OPP-SRC-A", "Opp Source A"))
        .add_card(make_test_card("OPP-TOP-A", "Opp Top A"))
        .add_card(make_test_card("OPP-SRC-B", "Opp Source B"))
        .add_card(make_test_card("OPP-TOP-B", "Opp Top B"))
        .add_card(make_test_card("EFFECT", "Effect"))
        .hand(0, &["EFFECT"])
        .start();

    let opp_a = runner.place_stack(1, &["OPP-SRC-A", "OPP-TOP-A"]);
    let opp_b = runner.place_stack(1, &["OPP-SRC-B", "OPP-TOP-B"]);
    let source_card = runner.game.players[0].hand[0].handle();

    let steps = vec![CompiledStep::SelectOpponentSources {
        target: Some(CompiledBindingRef::Binding("opp_stack".to_string())),
        filter: CompiledPredicate::default(),
        min: CompiledCountBound::Literal(1),
        max: CompiledCountBound::Literal(1),
        clamp_to_available: false,
        bind_as: Some("chosen".to_string()),
        prompt: "Choose a source under the chosen opponent Digimon".to_string(),
        then: vec![CompiledStep::TrashSelectedSources {
            source_refs: "chosen".to_string(),
        }],
    }];

    let mut bindings = Bindings::new();
    bindings.insert_permanent("opp_stack", opp_a);
    let outcome = {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, None, 0);
        run_steps(&steps, &mut ctx, &mut bindings)
    };
    assert_eq!(outcome, RunOutcome::Parked);

    let action_a = encode_source_select(opp_a.index as u16, 0).expect("opp A source action");
    let action_b = encode_source_select(opp_b.index as u16, 0).expect("opp B source action");
    {
        let pending = runner.game.pending_selection.as_ref().expect("pending");
        assert!(pending.valid_action_ids.contains(&action_a));
        assert!(
            !pending.valid_action_ids.contains(&action_b),
            "target binding must restrict to the chosen opponent permanent"
        );
    }
    runner.execute_action(0, action_a).expect("pick opp A");
    assert!(runner.game.pending_selection.is_none());
    assert_eq!(runner.game.players[1].trash.len(), 1);
}

#[test]
fn empty_select_opponent_sources_silently_skips_outer_tail_when_min_positive() {
    // No opponent stacks with sources → mandatory cost (min:1) cannot be paid.
    // The entire effect body — including subsequent outer-tail steps — is
    // silently skipped. Memory stays at 0. Mirrors the SelectOwnSources
    // contract for the opponent-side variant.
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("EFFECT", "Effect"))
        .hand(0, &["EFFECT"])
        .start();
    let source_card = runner.game.players[0].hand[0].handle();

    let steps = vec![
        CompiledStep::SelectOpponentSources {
            target: None,
            filter: CompiledPredicate::default(),
            min: CompiledCountBound::Literal(1),
            max: CompiledCountBound::Literal(1),
            clamp_to_available: false,
            bind_as: Some("chosen".to_string()),
            prompt: "Choose opponent source".to_string(),
            then: vec![CompiledStep::GainMemory(7)],
        },
        CompiledStep::GainMemory(3),
    ];

    let mut bindings = Bindings::new();
    let outcome = {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, None, 0);
        run_steps(&steps, &mut ctx, &mut bindings)
    };
    assert_eq!(outcome, RunOutcome::Synchronous);
    assert!(runner.game.pending_selection.is_none());
    // Neither the `then` body nor the outer tail ran — mandatory cost unpayable.
    assert_eq!(runner.game.memory, 0);
}

// ─── G-DSL-SELECT-SOURCES-FORMULA-COUNT (EX11-057 substrate) ──────────────────
//
// `select_opponent_sources` accepts formula-valued `min`/`max` (resolved once
// at execution time) and an opt-in `clamp_to_available` flag that mirrors
// DCGO `TrashDigivolutionCards.cs`'s
// `maxDigivolutionDiscardCount = Math.Min(digivolutionCardsSum, maxCount)`.
// Default semantics (no flag) stay the unpayable-drop behavior committed
// ladder cards rely on.

fn make_frost_digimon(id: &str) -> digimon_engine::card_data::CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = digimon_engine::enums::CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(2000);
    c.traits = vec!["Frost".to_string()];
    c
}

const FORMULA_TRASHER_YAML: &str = r#"
card: DSL-OPP-SRC-FORMULA
name: Formula Source Trasher
kind: digimon
level: 4
color: [blue]
cost: 4
dp: 4000
effects:
  - when: main_on_field
    process:
      - select_opponent_sources:
          min:
            formula:
              base: 0
              per:
                card_count_in_zone:
                  zone: battle_area
                  of: you
                  filter: { trait_has: Frost }
              delta: 1
          max:
            formula:
              base: 0
              per:
                card_count_in_zone:
                  zone: battle_area
                  of: you
                  filter: { trait_has: Frost }
              delta: 1
          bind_as: picks
          prompt: "Trash 1 opponent digivolution card per Frost Digimon"
          then:
            - trash_selected_sources:
                source_refs: picks
            - gain_memory: 5
      - gain_memory: 1
"#;

const CLAMPED_FORMULA_TRASHER_YAML: &str = r#"
card: DSL-OPP-SRC-CLAMP
name: Clamped Formula Source Trasher
kind: digimon
level: 4
color: [blue]
cost: 4
dp: 4000
effects:
  - when: main_on_field
    process:
      - select_opponent_sources:
          min:
            formula:
              base: 0
              per:
                card_count_in_zone:
                  zone: battle_area
                  of: you
                  filter: { trait_has: Frost }
              delta: 1
          max:
            formula:
              base: 0
              per:
                card_count_in_zone:
                  zone: battle_area
                  of: you
                  filter: { trait_has: Frost }
              delta: 1
          clamp_to_available: true
          bind_as: picks
          prompt: "Trash 1 opponent digivolution card per Frost Digimon"
          then:
            - trash_selected_sources:
                source_refs: picks
            - gain_memory: 5
      - gain_memory: 1
"#;

/// Formula `min`/`max` resolve at execution time: 2 own Frost Digimon → an
/// exact-2 mandatory cross-permanent pick (no PASS below the resolved min).
#[test]
fn select_opponent_sources_formula_counts_resolve_at_runtime() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(FORMULA_TRASHER_YAML)
        .expect("formula-count fixture compiles")
        .add_card(make_frost_digimon("FROST-A"))
        .add_card(make_frost_digimon("FROST-B"))
        .add_card(make_test_card("OPP-SRC-A", "Opp Source A"))
        .add_card(make_test_card("OPP-TOP-A", "Opp Top A"))
        .add_card(make_test_card("OPP-SRC-B", "Opp Source B"))
        .add_card(make_test_card("OPP-TOP-B", "Opp Top B"))
        .build();

    let carrier = runner.place_on_field(0, "DSL-OPP-SRC-FORMULA", Some(0));
    runner.place_on_field(0, "FROST-A", Some(0));
    runner.place_on_field(0, "FROST-B", Some(0));
    let opp_a = runner.place_stack(1, &["OPP-SRC-A", "OPP-TOP-A"]);
    let opp_b = runner.place_stack(1, &["OPP-SRC-B", "OPP-TOP-B"]);

    assert!(
        runner.game.activate_field_main(0, carrier.index as usize),
        "fixture exposes its Main effect"
    );
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::SourceMulti {
            min: 2,
            max: 2,
            picked: 0,
        }),
        "min/max formulas must resolve to the live Frost count (2)"
    );
    {
        let pending = runner.game.pending_selection.as_ref().expect("pending");
        assert!(
            !pending.valid_action_ids.contains(&PASS),
            "mandatory: no PASS below the resolved min"
        );
    }

    let action_a = encode_source_select(opp_a.index as u16, 0).expect("opp A source action");
    let action_b = encode_source_select(opp_b.index as u16, 0).expect("opp B source action");
    runner.execute_action(0, action_a).expect("pick opp A");
    runner.execute_action(0, action_b).expect("pick opp B");

    assert!(runner.game.pending_selection.is_none());
    assert_eq!(
        runner.game.players[1].trash.len(),
        2,
        "exactly 2 opponent digivolution cards trashed (cross-permanent)"
    );
    assert_eq!(
        runner.game.memory, 6,
        "then-body (+5) and outer tail (+1) both ran"
    );
}

/// `clamp_to_available: true`: formula resolves 3 but only 1 candidate exists →
/// the pick clamps to an exact-1 selection (DCGO min(total, N)) and the
/// continuation still runs.
#[test]
fn select_opponent_sources_clamp_to_available_clamps_min_and_max() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(CLAMPED_FORMULA_TRASHER_YAML)
        .expect("clamped formula-count fixture compiles")
        .add_card(make_frost_digimon("FROST-A"))
        .add_card(make_frost_digimon("FROST-B"))
        .add_card(make_frost_digimon("FROST-C"))
        .add_card(make_test_card("OPP-SRC-A", "Opp Source A"))
        .add_card(make_test_card("OPP-TOP-A", "Opp Top A"))
        .build();

    let carrier = runner.place_on_field(0, "DSL-OPP-SRC-CLAMP", Some(0));
    runner.place_on_field(0, "FROST-A", Some(0));
    runner.place_on_field(0, "FROST-B", Some(0));
    runner.place_on_field(0, "FROST-C", Some(0));
    let opp_a = runner.place_stack(1, &["OPP-SRC-A", "OPP-TOP-A"]);

    assert!(
        runner.game.activate_field_main(0, carrier.index as usize),
        "fixture exposes its Main effect"
    );
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::SourceMulti {
            min: 1,
            max: 1,
            picked: 0,
        }),
        "min/max must clamp from the resolved 3 down to the 1 available candidate"
    );

    let action_a = encode_source_select(opp_a.index as u16, 0).expect("opp A source action");
    runner.execute_action(0, action_a).expect("pick opp A");

    assert!(runner.game.pending_selection.is_none());
    assert_eq!(runner.game.players[1].trash.len(), 1, "the 1 available source trashed");
    assert_eq!(
        runner.game.memory, 6,
        "clamped pick completes: then-body (+5) and outer tail (+1) both ran"
    );
}

/// `clamp_to_available: true` with ZERO candidates: the pick is silently
/// skipped (DCGO `if maxCount <= 0 yield break`) and the rest of the clause
/// still runs — no unpayable-cost abort.
#[test]
fn select_opponent_sources_clamp_to_available_zero_candidates_skips_pick() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(CLAMPED_FORMULA_TRASHER_YAML)
        .expect("clamped formula-count fixture compiles")
        .add_card(make_frost_digimon("FROST-A"))
        .add_card(make_test_card("OPP-TOP-A", "Opp Top A"))
        .build();

    let carrier = runner.place_on_field(0, "DSL-OPP-SRC-CLAMP", Some(0));
    runner.place_on_field(0, "FROST-A", Some(0));
    // Opponent permanent with NO digivolution sources.
    runner.place_on_field(1, "OPP-TOP-A", Some(0));

    assert!(
        runner.game.activate_field_main(0, carrier.index as usize),
        "fixture exposes its Main effect"
    );

    assert!(
        runner.game.pending_selection.is_none(),
        "zero candidates → no source-pick prompt"
    );
    assert!(runner.game.players[1].trash.is_empty(), "nothing trashed");
    assert_eq!(
        runner.game.memory, 1,
        "the outer tail (+1) still runs; the dropped pick's then-body (+5) does not"
    );
}

/// Formula resolving to 0 (no Frost Digimon) skips the pick entirely even
/// though opponent candidates exist — the EX11-057 "N = 0" gate.
#[test]
fn select_opponent_sources_formula_zero_skips_pick_entirely() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(CLAMPED_FORMULA_TRASHER_YAML)
        .expect("clamped formula-count fixture compiles")
        .add_card(make_test_card("OPP-SRC-A", "Opp Source A"))
        .add_card(make_test_card("OPP-TOP-A", "Opp Top A"))
        .build();

    let carrier = runner.place_on_field(0, "DSL-OPP-SRC-CLAMP", Some(0));
    let _opp_a = runner.place_stack(1, &["OPP-SRC-A", "OPP-TOP-A"]);

    assert!(
        runner.game.activate_field_main(0, carrier.index as usize),
        "fixture exposes its Main effect"
    );

    assert!(
        runner.game.pending_selection.is_none(),
        "formula resolves 0 → no source-pick prompt"
    );
    assert!(runner.game.players[1].trash.is_empty(), "nothing trashed");
    assert_eq!(runner.game.memory, 1, "outer tail (+1) still runs");
}

/// Back-compat regression: WITHOUT `clamp_to_available`, a literal `min`
/// above the live candidate count keeps the historical drop-continuation
/// semantics (committed EX7-021 / EX7-023 / EX11-017 / EX8-066 availability
/// ladders are built on this exact behavior).
#[test]
fn select_opponent_sources_unclamped_min_above_available_still_drops_continuation() {
    let yaml = r#"
card: DSL-OPP-SRC-UNCLAMPED
name: Unclamped Exact Two Trasher
kind: digimon
level: 4
color: [blue]
cost: 4
dp: 4000
effects:
  - when: main_on_field
    process:
      - select_opponent_sources:
          min: 2
          max: 2
          bind_as: picks
          prompt: "Trash exactly 2 opponent digivolution cards"
          then:
            - trash_selected_sources:
                source_refs: picks
            - gain_memory: 5
      - gain_memory: 1
"#;
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("unclamped fixture compiles")
        .add_card(make_test_card("OPP-SRC-A", "Opp Source A"))
        .add_card(make_test_card("OPP-TOP-A", "Opp Top A"))
        .build();

    let carrier = runner.place_on_field(0, "DSL-OPP-SRC-UNCLAMPED", Some(0));
    let opp_a = runner.place_stack(1, &["OPP-SRC-A", "OPP-TOP-A"]);

    assert!(
        runner.game.activate_field_main(0, carrier.index as usize),
        "fixture exposes its Main effect"
    );
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::SourceMulti {
            min: 2,
            max: 2,
            picked: 0,
        }),
        "unclamped: the selection installs with the authored exact-2 counts"
    );

    let action_a = encode_source_select(opp_a.index as u16, 0).expect("opp A source action");
    runner.execute_action(0, action_a).expect("pick the only candidate");

    // After the only candidate is picked, no candidates remain and
    // picked (1) < min (2): the continuation is dropped — nothing is
    // trashed and neither the then-body nor the outer tail runs.
    assert!(runner.game.pending_selection.is_none());
    assert!(
        runner.game.players[1].trash.is_empty(),
        "drop-continuation: the pick never commits"
    );
    assert_eq!(
        runner.game.memory, 0,
        "drop-continuation: neither then-body (+5) nor outer tail (+1) runs"
    );
}

#[test]
fn select_own_sources_filters_cards_from_source_carrier_only() {
    let mut rock = make_test_card("ROCK-SRC", "Rock Source");
    rock.traits = vec!["Rock".to_string()];
    let mut mineral = make_test_card("MIN-SRC", "Mineral Source");
    mineral.traits = vec!["Mineral".to_string()];
    let mut other_rock = make_test_card("OTHER-ROCK", "Other Rock Source");
    other_rock.traits = vec!["Rock".to_string()];

    let mut runner = DebugRunner::builder()
        .add_card(rock)
        .add_card(mineral)
        .add_card(make_test_card("BAD-SRC", "Nonmatching Source"))
        .add_card(make_test_card("TOP-A", "Top A"))
        .add_card(other_rock)
        .add_card(make_test_card("TOP-B", "Top B"))
        .add_card(make_test_card("EFFECT", "Effect"))
        .hand(0, &["EFFECT"])
        .start();

    let carrier = runner.place_stack(0, &["ROCK-SRC", "MIN-SRC", "BAD-SRC", "TOP-A"]);
    let other_stack = runner.place_stack(0, &["OTHER-ROCK", "TOP-B"]);
    let source_card = runner.game.players[0].hand[0].handle();

    let mut rock_filter = CompiledPredicate::default();
    rock_filter.trait_has = Some("Rock".to_string());
    let mut mineral_filter = CompiledPredicate::default();
    mineral_filter.trait_has = Some("Mineral".to_string());
    let mut filter = CompiledPredicate::default();
    filter.any_of = vec![rock_filter, mineral_filter];

    let steps = vec![CompiledStep::SelectOwnSources {
        target: Some(CompiledBindingRef::Source),
        filter,
        min: 2,
        max: 2,
        bind_as: Some("chosen_sources".to_string()),
        prompt: "Choose two Rock/Mineral sources".to_string(),
        then: vec![CompiledStep::TrashSelectedSources {
            source_refs: "chosen_sources".to_string(),
        }],
    }];

    let mut bindings = Bindings::new();
    let outcome = {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(carrier), 0);
        run_steps(&steps, &mut ctx, &mut bindings)
    };

    assert_eq!(outcome, RunOutcome::Parked);
    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("source selection should be pending");
    let rock_action = encode_source_select(carrier.index as u16, 0).expect("rock source action");
    let mineral_action =
        encode_source_select(carrier.index as u16, 1).expect("mineral source action");
    let bad_action = encode_source_select(carrier.index as u16, 2).expect("bad source action");
    let other_rock_action =
        encode_source_select(other_stack.index as u16, 0).expect("other rock source action");
    assert!(pending.valid_action_ids.contains(&rock_action));
    assert!(pending.valid_action_ids.contains(&mineral_action));
    assert!(!pending.valid_action_ids.contains(&bad_action));
    assert!(!pending.valid_action_ids.contains(&other_rock_action));

    runner.execute_action(0, rock_action).expect("pick rock");
    runner
        .execute_action(0, mineral_action)
        .expect("pick mineral");

    assert!(runner.game.pending_selection.is_none());
    assert_eq!(runner.game.players[0].trash.len(), 2);
    assert_eq!(
        runner.game.players[0].battle_area[other_stack.index as usize]
            .card_sources
            .len(),
        2
    );
}
