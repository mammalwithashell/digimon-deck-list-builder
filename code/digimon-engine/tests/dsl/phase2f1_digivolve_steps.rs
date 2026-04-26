//! Phase 2f1 Task 5 — DSL behavioral tests for the digivolve-step variants
//! wired in Task 4: `EffectInitiatedDigivolve`, `EffectInitiatedDnaDigivolve`.
//!
//! Each test drives the variant through `run_step` (synchronous family) and
//! asserts the observable mutation matches the engine primitive.

use digimon_dsl::compiled::{CompiledBindingRef, CompiledCostDelta, CompiledStep};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_step;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::Permanent;

/// Helper: build a Lv.4 Red Digimon with an evo cost matching Lv.3 Red 0 memory.
/// `effect_initiated_digivolve` requires a matching evo cost on the result
/// card (level matches base, color matches when `ignore_color=false`); the
/// dispatcher passes `ignore_requirements: true` here, but the engine still
/// looks up an evo_costs row by level even with `ignore_color=true`.
fn evo_lv4_red(card_id: &str) -> CardData {
    CardData {
        card_id: card_id.to_string(),
        card_name: card_id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(4),
        dp: Some(3000),
        play_cost: 5,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: vec![EvoCost {
            card_color: 0, // Red
            level: 3,
            memory_cost: 0,
        }],
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        effect_class_name: card_id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
    }
}

// ─── EffectInitiatedDigivolve ────────────────────────────────────────────────

#[test]
fn effect_initiated_digivolve_step_grows_target_stack_with_hand_card() {
    // Set up: a Lv.3 base permanent on P0's battle area, and a Lv.4 evo card
    // (with matching evo_costs) in P0's hand. The dispatcher's variant lowers
    // to `ctx.effect_initiated_digivolve(player, hand_index, target, ...)`.
    let base = make_test_card("BASE3", "Base3"); // Lv.3 Red — make_test_card default
    let evo = evo_lv4_red("EVO4");

    let mut runner = DebugRunner::builder()
        .add_card(base)
        .add_card(evo)
        .hand(0, &["EVO4"])
        .memory(5)
        .start();

    // Seed BASE3 directly on the field.
    let target = {
        let g = &mut runner.game;
        let turn = g.turn_count;
        let data_idx = g
            .card_data
            .iter()
            .position(|c| c.card_id == "BASE3")
            .unwrap();
        let card_idx = g.next_card_index();
        let card = CardSource::new(data_idx, 0, card_idx);
        g.players[0].battle_area.push(Permanent::new(card, turn));
        digimon_engine::permanent::PermanentHandle { player: 0, index: 0 }
    };

    let memory_before = runner.game.memory;
    let hand_before = runner.game.players[0].hand.len();
    let battle_before = runner.game.players[0].battle_area.len();
    assert_eq!(runner.game.players[0].battle_area[0].stack_size(), 1);

    let src_card = runner.game.players[0].hand[0].handle();

    let mut bindings = Bindings::new();
    bindings.insert_permanent("tgt", target);
    bindings.insert_hand_index("from", 0, 0);

    let step = CompiledStep::EffectInitiatedDigivolve {
        target: CompiledBindingRef::Named("tgt".into()),
        from_hand: CompiledBindingRef::Named("from".into()),
        cost: CompiledCostDelta::Literal(0),
        ignore_requirements: true,
    };

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        run_step(&step, &mut ctx, &mut bindings);
    }

    // Stack grew by 1 (digivolved, not split).
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        battle_before,
        "battle area count unchanged — digivolve grows the existing stack"
    );
    assert_eq!(
        runner.game.players[0].battle_area[0].stack_size(),
        2,
        "target stack should now have 2 sources (BASE3 below, EVO4 on top)"
    );
    // The new top of stack is the EVO4 card from hand.
    let top = runner.game.players[0].battle_area[0].top_card();
    assert_eq!(top.handle(), src_card, "top of stack must be the hand card");
    // Hand consumed.
    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before - 1,
        "hand should shrink by 1"
    );
    // Memory unchanged with cost=0 / ignore_requirements=true.
    assert_eq!(runner.game.memory, memory_before, "cost=0 keeps memory unchanged");
}

// ─── EffectInitiatedDnaDigivolve ─────────────────────────────────────────────

#[test]
fn effect_initiated_dna_digivolve_step_merges_two_perms_with_card_handle_top() {
    // The dispatcher resolves `from_hand` for DNA via `ResolvedBinding::Card`
    // (not HandIndex). We must bind `from` as a card handle.
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("TST-A", "DnaSourceA"))
        .add_card(make_test_card("TST-B", "DnaSourceB"))
        .add_card(make_test_card("TST-DNA-RESULT", "DnaResult"))
        .hand(0, &["TST-DNA-RESULT"])
        .memory(5)
        .start();

    let handle_a = runner.place_on_field(0, "TST-A", None);
    let handle_b = runner.place_on_field(0, "TST-B", None);

    let source_a_handle = runner.game.players[0].battle_area[handle_a.index as usize]
        .top_card()
        .handle();
    let source_b_handle = runner.game.players[0].battle_area[handle_b.index as usize]
        .top_card()
        .handle();
    let hand_card_handle = runner.game.players[0].hand[0].handle();

    assert_eq!(runner.game.players[0].battle_area.len(), 2);
    assert_eq!(runner.game.players[0].hand.len(), 1);
    let memory_before = runner.game.memory;

    let mut bindings = Bindings::new();
    bindings.insert_permanent("a", handle_a);
    bindings.insert_permanent("b", handle_b);
    // CRITICAL: the dispatcher resolves `from_hand` via ResolvedBinding::Card,
    // so we MUST insert as a card handle (not a hand index).
    bindings.insert_card("from", hand_card_handle);

    let step = CompiledStep::EffectInitiatedDnaDigivolve {
        target_a: CompiledBindingRef::Named("a".into()),
        target_b: CompiledBindingRef::Named("b".into()),
        from_hand: CompiledBindingRef::Named("from".into()),
        cost: CompiledCostDelta::Literal(0),
        ignore_requirements: true,
    };

    {
        let mut ctx = EffectContext::new(&mut runner.game, hand_card_handle, None, 0);
        run_step(&step, &mut ctx, &mut bindings);
    }

    // Both source perms merged into ONE permanent.
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        1,
        "two source permanents should be replaced by one merged permanent"
    );
    // Result card consumed from hand.
    assert_eq!(
        runner.game.players[0].hand.len(),
        0,
        "hand must be empty after DNA digivolve consumes the result card"
    );
    assert_eq!(runner.game.memory, memory_before, "cost=0 keeps memory unchanged");

    let merged = &runner.game.players[0].battle_area[0];
    // Top of the merged stack is the hand card.
    assert_eq!(
        merged.top_card().handle(),
        hand_card_handle,
        "top of merged stack must be the result card from hand"
    );
    // Both source handles plus the result are present in the stack.
    let handles: Vec<_> = merged.card_sources.iter().map(|c| c.handle()).collect();
    assert_eq!(handles.len(), 3, "merged stack must contain 3 sources");
    assert!(
        handles.contains(&source_a_handle),
        "merged stack contains source A"
    );
    assert!(
        handles.contains(&source_b_handle),
        "merged stack contains source B"
    );
    assert!(
        handles.contains(&hand_card_handle),
        "merged stack contains the result card"
    );
}
