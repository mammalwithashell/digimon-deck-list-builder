//! Phase 2g — `<OnDnaDigivolve>` clause fires from both DNA digivolve paths.
//!
//! Card text precedent: any "[When this Digimon DNA-digivolves]" effect.
//! Authored via the DSL example card `TST_DNA_TRIGGER.yaml` whose only
//! effect is a triggered `on_dna_digivolve` clause that draws 1 card.
//!
//! The DNA cost itself is not currently expressible in the YAML schema;
//! it's attached via `CardData::dna_costs` in the test setup. The DSL's
//! sole job here is to supply the `OnDnaDigivolve` triggered effect, which
//! is registered against the result-card id via `register_effect`.

use std::sync::Arc;

use digimon_engine::debug_runner::{
    make_test_card, make_test_card_with_level, make_test_dna_card, DebugRunner,
};
use digimon_engine::dsl_cards::DslCardEffect;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::GamePhase;

const DNA_TRIGGER_YAML: &str =
    include_str!("../../cards/_examples/TST_DNA_TRIGGER.yaml");

fn build_dsl_trigger_effect() -> Arc<DslCardEffect> {
    let spec: digimon_dsl::CardSpec =
        serde_yml::from_str(DNA_TRIGGER_YAML).expect("YAML parses");
    let compiled = digimon_dsl::compile::compile(&spec).expect("compiles cleanly");
    Arc::new(DslCardEffect::new(Arc::new(compiled)))
}

#[test]
fn dsl_on_dna_digivolve_fires_from_user_action_path() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card_with_level("TST-LV5", "FiveDigi", 5))
        .add_card(make_test_card_with_level("TST-LV6", "SixDigi", 6))
        .add_card({
            let mut card =
                make_test_dna_card("TST-DNA-TRIGGER", "TestDnaTrigger", 5, 6, 0);
            card.level = Some(7);
            card
        })
        .add_card(make_test_card("TST-DECK-A", "DeckCard"))
        .hand(0, &["TST-DNA-TRIGGER"])
        .deck(0, &["TST-DECK-A", "TST-DECK-A"])
        .memory(5)
        .start();
    runner.register_effect("TST-DNA-TRIGGER", build_dsl_trigger_effect());

    runner.place_on_field(0, "TST-LV5", None);
    runner.place_on_field(0, "TST-LV6", None);
    runner.game.current_phase = GamePhase::Main;

    let pre_hand = runner.game.players[0].hand.len();
    let ok = runner.game.initiate_dna_digivolve(0, 0);
    assert!(ok, "initiate_dna_digivolve must accept valid hand index");
    runner
        .game
        .resolve_selection(0, 0)
        .expect("first-stage resolution succeeds");
    runner
        .game
        .resolve_selection(0, 1)
        .expect("second-stage resolution succeeds");

    // Hand: -1 (DNA card consumed) + 1 (digivolution bonus draw)
    //       + 1 (OnDnaDigivolve draw clause) = net +1 from pre.
    assert_eq!(
        runner.game.players[0].hand.len(),
        pre_hand + 1,
        "OnDnaDigivolve clause must draw 1 in addition to the digivolution bonus"
    );
}

#[test]
fn dsl_on_dna_digivolve_fires_from_effect_path() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card_with_level("TST-LV5", "FiveDigi", 5))
        .add_card(make_test_card_with_level("TST-LV6", "SixDigi", 6))
        .add_card({
            let mut card =
                make_test_dna_card("TST-DNA-TRIGGER", "TestDnaTrigger", 5, 6, 0);
            card.level = Some(7);
            card
        })
        .add_card(make_test_card("TST-DECK-A", "DeckCard"))
        .hand(0, &["TST-DNA-TRIGGER"])
        .deck(0, &["TST-DECK-A", "TST-DECK-A"])
        .memory(5)
        .start();
    runner.register_effect("TST-DNA-TRIGGER", build_dsl_trigger_effect());

    let handle_a = runner.place_on_field(0, "TST-LV5", None);
    let handle_b = runner.place_on_field(0, "TST-LV6", None);
    let hand_card = runner.game.players[0].hand[0].handle();

    let result = {
        let mut ctx = EffectContext::new(&mut runner.game, hand_card, None, 0);
        ctx.effect_initiated_dna_digivolve(handle_a, handle_b, hand_card, 0, true)
    };
    assert!(
        result.is_some(),
        "effect_initiated_dna_digivolve should succeed for valid handles"
    );

    // Effect-initiated path does NOT grant the digivolution bonus draw.
    // Hand: started with TST-DNA-TRIGGER (1) -1 (consumed into stack)
    //       +1 (OnDnaDigivolve clause draws 1) = 1.
    assert_eq!(
        runner.game.players[0].hand.len(),
        1,
        "OnDnaDigivolve clause draws 1 (no digivolution bonus from effect path)"
    );
}
