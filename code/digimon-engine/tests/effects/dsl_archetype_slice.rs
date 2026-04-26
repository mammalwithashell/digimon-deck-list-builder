//! Mechanic tests for the first real DSL archetype slice.

use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::EffectTiming;

use crate::dsl_card_data::card_data_from_compiled;

#[test]
fn nokia_aura_grants_plus_1000_dp_to_own_greymon_name() {
    let mut runner = DebugRunner::builder()
        .add_card(card_data_from_compiled("BT22-084"))
        .add_card(card_data_from_compiled("BT17-015"))
        .build();

    let nokia = runner.place_on_field(0, "BT22-084", Some(0));
    let wargreymon = runner.place_on_field(0, "BT17-015", Some(0));
    let base_dp = runner
        .effective_dp(wargreymon)
        .expect("WarGreymon has printed DP");

    let source_card = runner.game.players[0].battle_area[nokia.index as usize]
        .top_card()
        .handle();
    let aura = runner
        .game
        .effects_for_card("BT22-084", source_card)
        .expect("Nokia DSL effects are registered")
        .into_iter()
        .find(|effect| effect.timing == EffectTiming::Declarative)
        .expect("Nokia has a declarative aura");
    let process = aura.process.as_ref().expect("aura has process");

    {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(nokia), 0);
        process(&mut ctx);
    }

    assert_eq!(
        runner.effective_dp(wargreymon),
        Some(base_dp + 1000),
        "Nokia's DSL aura should buff the Greymon-name permanent"
    );
}
