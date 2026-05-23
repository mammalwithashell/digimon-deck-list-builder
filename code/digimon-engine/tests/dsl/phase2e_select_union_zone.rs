//! Phase 2e Task 6: SelectUnionZone over hand+trash binds the picked
//! CardHandle into Bindings.

use digimon_dsl::compiled::{CompiledPlayerRef, CompiledPredicate, CompiledStep, CompiledZone};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::effect_context::EffectContext;

fn push_to_trash(runner: &mut DebugRunner, player: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap();
    let card_index = runner.game.next_card_index();
    runner.game.players[player as usize]
        .trash
        .push(CardSource::new(data_idx, player, card_index));
}

#[test]
fn select_union_zone_picks_from_hand_or_trash() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .add_card(make_test_card("H0", "H0"))
        .add_card(make_test_card("T0", "T0"))
        .hand(0, &["SRC", "H0"])
        .build();

    push_to_trash(&mut runner, 0, "T0");
    let src_card = runner.game.players[0].hand[0].handle();
    let memory_before = runner.game.memory;

    let steps = vec![
        CompiledStep::SelectUnionZone {
            of: CompiledPlayerRef::You,
            zones: vec![CompiledZone::Hand, CompiledZone::Trash],
            material_of: None,
            filter: CompiledPredicate::default(),
            bind_as: Some("union_pick".to_string()),
            prompt: "Pick from hand or trash".to_string(),
            prompt_key: None,
            optional: false,
            then: vec![],
        },
        CompiledStep::GainMemory(1),
    ];

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        let mut bindings = Bindings::new();
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("select_union_zone must install a pending selection");
    // 2 hand cards (SRC + H0) + 1 trash card (T0) = 3 candidates.
    assert_eq!(pending.valid_action_ids.len(), 3);

    let (action_id, selecting_player) = (pending.valid_action_ids[0], pending.selecting_player);
    runner
        .game
        .resolve_selection(selecting_player, action_id)
        .expect("resolve");
    assert!(runner.game.pending_selection.is_none());
    assert_eq!(runner.game.memory, memory_before + 1);
}
