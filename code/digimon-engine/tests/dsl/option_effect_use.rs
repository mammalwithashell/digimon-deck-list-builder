use std::sync::Arc;

use digimon_dsl::compiled::{CompiledCardKind, CompiledPlayerRef, CompiledPredicate, CompiledStep};
use digimon_engine::action::space::PLAY_HAND_START;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::{run_steps, RunOutcome};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::CardKind;

struct OptionMainNoop;

impl CardEffect for OptionMainNoop {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::when_attacking(card)
            .option_main()
            .name("OptionMain no-op")
            .process(|_| {})
            .build()]
    }
}

fn ts_option(id: &str, cost: u16) -> digimon_engine::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Option;
    card.play_cost = cost;
    card.traits = vec!["TS".to_string()];
    card
}

#[test]
fn use_option_from_hand_filters_by_trait_and_opponent_memory_ceiling() {
    let mut src = make_test_card("SRC-USE-OPT", "Source");
    src.card_kind = CardKind::Tamer;

    let mut runner = DebugRunner::builder()
        .add_card(src)
        .add_card(ts_option("TS-LOW", 2))
        .add_card(ts_option("TS-HIGH", 5))
        .hand(1, &["SRC-USE-OPT", "TS-LOW", "TS-HIGH"])
        .memory(3)
        .start();
    runner.register_effect("TS-LOW", Arc::new(OptionMainNoop));
    runner.register_effect("TS-HIGH", Arc::new(OptionMainNoop));

    let source_card = runner.game.players[1].hand[0].handle();
    let steps = vec![CompiledStep::UseOptionFromHand {
        of: CompiledPlayerRef::You,
        filter: CompiledPredicate {
            kind: Some(CompiledCardKind::Option),
            trait_has: Some("TS".to_string()),
            ..CompiledPredicate::default()
        },
        use_cost_lte_opponent_memory: true,
        optional: false,
        prompt: None,
    }];

    {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, None, 1);
        let mut bindings = Bindings::new();
        assert_eq!(
            run_steps(&steps, &mut ctx, &mut bindings),
            RunOutcome::Parked
        );
    }

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("eligible option should install a hand-selection prompt");
    assert_eq!(
        pending.valid_action_ids,
        vec![PLAY_HAND_START + 1],
        "only the TS Option under the opponent-memory ceiling should be selectable"
    );

    let before_resolution = runner.game.memory;
    runner
        .game
        .resolve_selection(1, PLAY_HAND_START + 1)
        .expect("selected low-cost Option resolves");

    assert!(
        runner.game.players[1]
            .trash
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "TS-LOW"),
        "effect-driven use should dispose the selected Option through the normal trash path"
    );
    assert_eq!(
        runner.game.memory, before_resolution,
        "effect-driven use should not pay the selected Option's use cost"
    );
}
