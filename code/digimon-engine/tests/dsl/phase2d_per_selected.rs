//! Phase 2d Task 6: PerSelected over a CardList from
//! SelectCountCappedMulti. Pattern: "for each card you picked, gain
//! memory and draw 1".

use digimon_dsl::compiled::{CompiledPlayerRef, CompiledPredicate, CompiledStep, CompiledZone};
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
fn per_selected_drives_body_once_per_pick() {
    // P1 has 3 trash cards. P0 picks all 3 (max=3 → auto-commit), then
    // per-pick gains 1 memory and draws 1 card.
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .add_card(make_test_card("T1", "T1"))
        .add_card(make_test_card("T2", "T2"))
        .add_card(make_test_card("T3", "T3"))
        .add_card(make_test_card("DRAW1", "DRAW1"))
        .add_card(make_test_card("DRAW2", "DRAW2"))
        .add_card(make_test_card("DRAW3", "DRAW3"))
        .hand(0, &["SRC"])
        .build();

    push_to_trash(&mut runner, 1, "T1");
    push_to_trash(&mut runner, 1, "T2");
    push_to_trash(&mut runner, 1, "T3");
    push_to_deck(&mut runner, 0, "DRAW1");
    push_to_deck(&mut runner, 0, "DRAW2");
    push_to_deck(&mut runner, 0, "DRAW3");

    let src_card = runner.game.players[0].hand[0].handle();
    let memory_before = runner.game.memory;
    let hand_before = runner.game.players[0].hand.len();

    let steps = vec![
        CompiledStep::SelectCountCappedMulti {
            of: CompiledPlayerRef::Opponent,
            zone: CompiledZone::Trash,
            min: 0,
            max: digimon_dsl::compiled::CompiledCountBound::Literal(3),
            filter: CompiledPredicate::default(),
            bind_as: Some("picks".to_string()),
            prompt: "Pick up to 3".to_string(),
            prompt_key: None,
            optional_zero: false,
            distinct_by: None,
        },
        CompiledStep::PerSelected {
            selection: "picks".to_string(),
            bind_as: "p".to_string(),
            body: vec![
                CompiledStep::GainMemory(1),
                CompiledStep::Draw {
                    of: CompiledPlayerRef::You,
                    count: 1,
                },
            ],
        },
    ];

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        let mut bindings = Bindings::new();
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    // Pick the first remaining candidate three times. After 3 picks,
    // candidates are exhausted → engine commits and fires PerSelected.
    resolve_count_capped_picks(&mut runner, &[0, 0, 0]);

    assert!(
        runner.game.pending_selection.is_none(),
        "no selection must remain after final pick"
    );
    assert_eq!(runner.game.memory, memory_before + 3, "3 picks → +3 memory");
    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before + 3,
        "3 picks → +3 cards drawn"
    );
}
