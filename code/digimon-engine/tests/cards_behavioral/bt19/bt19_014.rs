use digimon_engine::action::space::encode_source_select;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::{make_test_card, make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

const CARD_ID: &str = "BT19-014";

fn digimon(id: &str, name: &str, level: u8, dp: i32, colors: &[CardColor]) -> CardData {
    let mut card = make_test_card_with_level(id, name, level);
    card.card_kind = CardKind::Digimon;
    card.dp = Some(dp);
    card.colors = colors.to_vec();
    card
}

fn tamer(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Tamer;
    card.level = None;
    card.dp = None;
    card.play_cost = 3;
    card
}

fn insert_source(runner: &mut DebugRunner, host: PermanentHandle, card_id: &str) -> CardHandle {
    let data_index = runner
        .game
        .card_data
        .iter()
        .position(|card| card.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card id {card_id}"));
    let card = CardSource::new(data_index, host.player, runner.game.next_card_index());
    let handle = card.handle();
    let sources = &mut runner.game.players[host.player as usize].battle_area[host.index as usize]
        .card_sources;
    let insert_at = sources.len().saturating_sub(1);
    sources.insert(insert_at, card);
    handle
}

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT19-014 YAML loads")
        .add_card(digimon(
            "BLUE-GREEN-SRC",
            "BlueGreenSource",
            4,
            4000,
            &[CardColor::Blue, CardColor::Green],
        ))
        .add_card(digimon(
            "SHOOT",
            "ShootingStarmon",
            4,
            5000,
            &[CardColor::Yellow],
        ))
        .add_card(digimon("OPP-A", "Opponent A", 4, 6000, &[CardColor::Red]))
        .add_card(digimon("OPP-B", "Opponent B", 4, 7000, &[CardColor::Blue]))
        .add_card(tamer("XROS-TAMER"))
        .memory(10)
        .start()
}

#[test]
fn bt19_014_on_play_debuffs_by_source_colors_and_offers_shootingstarmon() {
    let mut runner = runner();
    let shoutmon = runner.place_on_field(0, CARD_ID, Some(0));
    insert_source(&mut runner, shoutmon, "BLUE-GREEN-SRC");
    let tamer = runner.place_on_field(0, "XROS-TAMER", Some(0));
    let shooting = insert_source(&mut runner, tamer, "SHOOT");
    let opp_a = runner.place_on_field(1, "OPP-A", Some(0));
    let opp_b = runner.place_on_field(1, "OPP-B", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(shoutmon));
    runner.game.drain_effect_queue();

    assert_eq!(runner.game.effective_dp(opp_a), Some(4000));
    assert_eq!(runner.game.effective_dp(opp_b), Some(5000));
    let pending = runner
        .pending_selection()
        .expect("On Play should offer ShootingStarmon under a Tamer");
    let action = encode_source_select(tamer.index as u16, 0).unwrap();
    assert!(pending.valid_action_ids.contains(&action));
    runner
        .execute_action(0, action)
        .expect("choose ShootingStarmon");
    runner.auto_resolve().expect("play selected source");

    assert!(runner.game.players[0]
        .battle_area
        .iter()
        .any(|perm| perm.top_card().handle() == shooting));
}
