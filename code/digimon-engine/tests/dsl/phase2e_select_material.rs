//! Phase 2e Task 5: SelectMaterial resolves `of_permanent` via the
//! existing binding_ref machinery, installs a parking material-pick
//! selection, and binds the picked source as a CardHandle.

use digimon_dsl::compiled::{CompiledBindingRef, CompiledPredicate, CompiledStep};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::effect_context::EffectContext;

#[test]
fn select_material_binds_picked_source_handle() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .add_card(make_test_card("STACK", "STACK"))
        .add_card(make_test_card("M0", "M0"))
        .add_card(make_test_card("M1", "M1"))
        .hand(0, &["SRC"])
        .build();

    // Place a permanent on the field with two materials underneath.
    runner.place_on_field(0, "STACK", None);

    // Stack M0 and M1 under the permanent (insert at position 0, so top stays last).
    // After inserts: card_sources = [m1, m0, STACK_top]
    let m0_data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "M0")
        .unwrap();
    let m1_data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "M1")
        .unwrap();
    let m0_card_index = runner.game.next_card_index();
    let m1_card_index = runner.game.next_card_index();
    let m0 = CardSource::new(m0_data_idx, 0, m0_card_index);
    let m1 = CardSource::new(m1_data_idx, 0, m1_card_index);
    runner.game.players[0].battle_area[0]
        .card_sources
        .insert(0, m0);
    runner.game.players[0].battle_area[0]
        .card_sources
        .insert(0, m1);

    let perm_handle = runner.perm_handle(0, 0);
    let src_card = runner.game.players[0].hand[0].handle();
    let memory_before = runner.game.memory;

    // Pre-populate a binding that of_permanent: Named will resolve.
    let mut bindings = Bindings::new();
    bindings.insert_permanent("target", perm_handle);

    let steps = vec![
        CompiledStep::SelectMaterial {
            of_permanent: CompiledBindingRef::Named("target".to_string()),
            filter: CompiledPredicate::default(),
            bind_as: Some("mat".to_string()),
            prompt: "Pick a material".to_string(),
            prompt_key: None,
            optional: false,
        },
        CompiledStep::GainMemory(1),
    ];

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("select_material must install a pending selection");
    // 2 sources excluding top: card_sources len = 3 → 2 candidates (top excluded).
    assert_eq!(
        pending.valid_action_ids.len(),
        2,
        "expected exactly 2 material candidates (top excluded)"
    );

    let (action_id, selecting_player) = (pending.valid_action_ids[0], pending.selecting_player);
    runner
        .game
        .resolve_selection(selecting_player, action_id)
        .expect("resolve");

    assert!(runner.game.pending_selection.is_none());
    assert_eq!(runner.game.memory, memory_before + 1);
}

#[test]
fn select_material_missing_binding_silent_noop() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .hand(0, &["SRC"])
        .build();

    let src_card = runner.game.players[0].hand[0].handle();
    let memory_before = runner.game.memory;

    let steps = vec![
        CompiledStep::SelectMaterial {
            of_permanent: CompiledBindingRef::Named("missing".to_string()),
            filter: CompiledPredicate::default(),
            bind_as: Some("mat".to_string()),
            prompt: "Pick a material".to_string(),
            prompt_key: None,
            optional: false,
        },
        CompiledStep::GainMemory(1),
    ];

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        let mut bindings = Bindings::new();
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    // Silent no-op: no selection installed, the GainMemory tail still ran
    // synchronously because the selection step didn't park.
    assert!(runner.game.pending_selection.is_none());
    assert_eq!(
        runner.game.memory,
        memory_before + 1,
        "missing binding → SelectMaterial no-ops; the tail still runs synchronously"
    );
}

#[test]
fn empty_select_material_runs_outer_tail_synchronously() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .add_card(make_test_card("STACK", "STACK"))
        .hand(0, &["SRC"])
        .build();

    let perm_handle = runner.place_on_field(0, "STACK", None);
    let src_card = runner.game.players[0].hand[0].handle();
    let memory_before = runner.game.memory;

    let mut bindings = Bindings::new();
    bindings.insert_permanent("target", perm_handle);

    let steps = vec![
        CompiledStep::SelectMaterial {
            of_permanent: CompiledBindingRef::Named("target".to_string()),
            filter: CompiledPredicate::default(),
            bind_as: Some("mat".to_string()),
            prompt: "Pick a material".to_string(),
            prompt_key: None,
            optional: false,
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
        "empty SelectMaterial no-ops and outer tail continues"
    );
}
