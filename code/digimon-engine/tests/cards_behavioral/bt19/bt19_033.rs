use digimon_engine::action::space::encode_source_select;
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::{make_test_card, make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::TriggerSource;

const CARD_ID: &str = "BT19-033";

fn jaeger_dorulumon() -> CardData {
    let mut card = make_test_card_with_level("BT19-038", "JaegerDorulumon", 5);
    card.colors = vec![CardColor::Yellow, CardColor::Green];
    card.traits = vec!["Beastkin".to_string(), "Xros Heart".to_string()];
    card.evo_costs = vec![EvoCost {
        card_color: CardColor::Yellow as u8,
        level: 4,
        memory_cost: 3,
    }];
    card
}

fn tamer(card_id: &str) -> CardData {
    let mut card = make_test_card(card_id, card_id);
    card.card_kind = CardKind::Tamer;
    card.level = None;
    card.dp = None;
    card.play_cost = 4;
    card
}

fn insert_source(runner: &mut DebugRunner, host: PermanentHandle, card_id: &str) -> CardHandle {
    let data_index = runner
        .game
        .card_data
        .iter()
        .position(|card| card.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card id {card_id}"));
    let instance_id = runner.game.next_card_index();
    let card = CardSource::new(data_index, host.player, instance_id);
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
        .expect("BT19-033 YAML loads")
        .add_card(jaeger_dorulumon())
        .add_card(tamer("XROS-TAMER"))
        .memory(10)
        .start()
}

#[test]
fn bt19_033_on_play_may_digivolve_from_jaegerdorulumon_under_tamer() {
    let mut runner = runner();
    let dorulumon = runner.place_on_field(0, CARD_ID, Some(0));
    let tamer = runner.place_on_field(0, "XROS-TAMER", Some(0));
    let jaeger = insert_source(&mut runner, tamer, "BT19-038");

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(dorulumon));
    runner.game.drain_effect_queue();

    let pending = runner
        .pending_selection()
        .expect("On Play should offer JaegerDorulumon under a Tamer");
    assert!(pending.is_optional, "printed 'may digivolve' is declinable");
    let action = encode_source_select(tamer.index as u16, 0).unwrap();
    assert!(pending.valid_action_ids.contains(&action));
    runner
        .execute_action(0, action)
        .expect("choose JaegerDorulumon under Tamer");
    runner.auto_resolve().expect("settle digivolution");

    assert_eq!(
        runner.game.players[0].battle_area[dorulumon.index as usize]
            .top_card()
            .handle(),
        jaeger
    );
}

#[test]
fn bt19_033_inherited_grants_piercing_to_xros_heart_carrier_on_your_turn() {
    let mut runner = runner();
    let carrier = runner.place_stack(0, &[CARD_ID, "BT19-038"]);

    assert!(
        runner.game.has_keyword(carrier, Keyword::Piercing),
        "Dorulumon inherited text should grant Piercing to a Xros Heart carrier on your turn"
    );

    runner.pass_turn();

    assert!(
        !runner.game.has_keyword(carrier, Keyword::Piercing),
        "Dorulumon inherited Piercing is your-turn only"
    );
}

#[test]
fn bt19_033_save_is_auto_installed_from_printed_keyword() {
    let mut runner = runner();
    let dorulumon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "XROS-TAMER", Some(0));

    runner
        .game
        .delete_permanent_with_cause(dorulumon, ReplacementCause::OpponentEffect);

    let pending = runner
        .pending_selection()
        .expect("Save should offer a Tamer destination after deletion");
    assert!(pending.is_optional);
}
