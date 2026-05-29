use digimon_engine::action::space::encode_attack;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card_with_level, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardKind, EffectTiming, Expiry};
use digimon_engine::selection::TriggerSource;

const CARD_ID: &str = "AD1-006";

fn digimon(id: &str, name: &str, level: u8, dp: i32) -> CardData {
    let mut card = make_test_card_with_level(id, name, level);
    card.card_kind = CardKind::Digimon;
    card.dp = Some(dp);
    card
}

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("AD1-006 YAML loads")
        .add_card(digimon("BUFF-SRC", "BuffSource", 5, 5000))
        .add_card(digimon("ELIGIBLE", "Eligible", 6, 15000))
        .add_card(digimon("TOO-BIG", "TooBig", 6, 17000))
        .memory(10)
        .start()
}

#[test]
fn ad1_006_current_dp_filter_uses_buffed_acting_digimon() {
    let mut runner = runner();
    let x7 = runner.place_stack(0, &["BUFF-SRC", CARD_ID]);
    {
        let source_card = runner.game.players[0].battle_area[x7.index as usize]
            .top_card()
            .handle();
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(x7), 0);
        ctx.add_dp_modifier(x7, 3000, Expiry::EndOfTurn);
    }
    let eligible = runner.place_on_field(1, "ELIGIBLE", Some(0));
    let too_big = runner.place_on_field(1, "TOO-BIG", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(x7));
    runner.game.drain_effect_queue();

    let pending = runner
        .pending_selection()
        .expect("optional bottom-deck target selection should install");
    assert!(pending.is_optional);
    assert!(pending
        .valid_action_ids
        .contains(&encode_attack(0, eligible.index as u16)));
    assert!(!pending
        .valid_action_ids
        .contains(&encode_attack(0, too_big.index as u16)));
}
