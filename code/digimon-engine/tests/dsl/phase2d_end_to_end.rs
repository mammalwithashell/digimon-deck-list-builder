//! Phase 2d Task 9: full pipeline exercising every Phase 2d primitive in
//! a single step list.
//!
//! Step list:
//!   - select_count_capped_multi { of: opponent, zone: trash, max: 2 } → bind picks
//!   - per_selected { selection: picks, body: [draw 1, gain_memory 1] }
//!   - for_each { over: { kind: digimon, owner: you }, body: [add_dp_modifier +1000 EndOfTurn] }

use digimon_dsl::compiled::{
    CompiledBindingRef, CompiledCardKind, CompiledModifierValue, CompiledPlayerRef,
    CompiledPredicate, CompiledStep, CompiledZone,
};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::effect_context::EffectContext;

use crate::phase2d_helpers::resolve_count_capped_picks;

fn push_to_trash(runner: &mut DebugRunner, player: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("push_to_trash: unknown card_id {card_id}"));
    let card_index = runner.game.next_card_index();
    let card = CardSource::new(data_idx, player, card_index);
    runner.game.players[player as usize].trash.push(card);
}

fn push_to_deck(runner: &mut DebugRunner, player: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("push_to_deck: unknown card_id {card_id}"));
    let card_index = runner.game.next_card_index();
    let card = CardSource::new(data_idx, player, card_index);
    runner.game.players[player as usize].deck.push(card);
}

#[test]
fn multi_pick_then_per_selected_then_for_each_round_trip() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .add_card(make_test_card("D1", "D1"))
        .add_card(make_test_card("D2", "D2"))
        .add_card(make_test_card("T1", "T1"))
        .add_card(make_test_card("T2", "T2"))
        .add_card(make_test_card("DR1", "DR1"))
        .add_card(make_test_card("DR2", "DR2"))
        .hand(0, &["SRC"])
        .build();

    push_to_trash(&mut runner, 1, "T1");
    push_to_trash(&mut runner, 1, "T2");
    push_to_deck(&mut runner, 0, "DR1");
    push_to_deck(&mut runner, 0, "DR2");

    runner.place_on_field(0, "D1", None);
    runner.place_on_field(0, "D2", None);

    let src_card = runner.game.players[0].hand[0].handle();
    let memory_before = runner.game.memory;
    let hand_before = runner.game.players[0].hand.len();

    let steps = vec![
        CompiledStep::SelectCountCappedMulti {
            of: CompiledPlayerRef::Opponent,
            zone: CompiledZone::Trash,
            max: 2,
            filter: CompiledPredicate::default(),
            bind_as: Some("picks".to_string()),
            prompt: "Pick up to 2".to_string(),
            prompt_key: None,
            optional_zero: false,
            distinct_by: None,
        },
        CompiledStep::PerSelected {
            selection: "picks".to_string(),
            bind_as: "p".to_string(),
            body: vec![
                CompiledStep::Draw {
                    of: CompiledPlayerRef::You,
                    count: 1,
                },
                CompiledStep::GainMemory(1),
            ],
        },
        CompiledStep::ForEach {
            over: CompiledPredicate {
                kind: Some(CompiledCardKind::Digimon),
                owner: Some(CompiledPlayerRef::You),
                ..CompiledPredicate::default()
            },
            bind_as: "tgt".to_string(),
            body: vec![CompiledStep::AddDpModifier {
                target: CompiledBindingRef::Named("tgt".to_string()),
                value: CompiledModifierValue::Literal(1000),
                expiry: "end_of_turn".to_string(),
            }],
        },
    ];

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        let mut bindings = Bindings::new();
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    // Resolve the multi-pick — pick both candidates (max=2 → auto-commit).
    resolve_count_capped_picks(&mut runner, &[0, 0]);

    assert!(
        runner.game.pending_selection.is_none(),
        "no selection must remain after auto-commit at max"
    );

    // 2 picks → +2 memory + 2 cards drawn.
    assert_eq!(runner.game.memory, memory_before + 2, "+2 memory from 2 picks");
    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before + 2,
        "+2 cards drawn from 2 picks"
    );

    // ForEach should run after the multi-pick resolves (outer-tail capture
    // from Task 7). Both P0 digimon should have +1000 DP this turn.
    // Base DP from make_test_card is 2000 → expect 3000 effective.
    assert_eq!(runner.game.players[0].battle_area.len(), 2, "P0 has 2 digimon");
    for i in 0..runner.game.players[0].battle_area.len() {
        let h = runner.perm_handle(0, i);
        let dp = runner.game.effective_dp(h).expect("digimon DP");
        assert_eq!(dp, 3000, "P0 perm {i}: base 2000 + ForEach +1000 = 3000");
    }
}
