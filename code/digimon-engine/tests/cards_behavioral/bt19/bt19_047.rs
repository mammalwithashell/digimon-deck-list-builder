use digimon_engine::action::space::encode_source_select;
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::{make_test_card, make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::TriggerSource;

const CARD_ID: &str = "BT19-047";

fn atlur_ballistamon() -> CardData {
    let mut card = make_test_card_with_level("BT19-051", "AtlurBallistamon", 5);
    card.colors = vec![CardColor::Green, CardColor::Black];
    card.traits = vec!["Machine".to_string(), "Xros Heart".to_string()];
    card.evo_costs = vec![EvoCost {
        card_color: CardColor::Black as u8,
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
        .expect("BT19-047 YAML loads")
        .add_card(atlur_ballistamon())
        .add_card(tamer("XROS-TAMER"))
        .memory(10)
        .start()
}

#[test]
fn bt19_047_on_play_may_digivolve_from_atlurballistamon_under_tamer() {
    let mut runner = runner();
    let ballistamon = runner.place_on_field(0, CARD_ID, Some(0));
    let tamer = runner.place_on_field(0, "XROS-TAMER", Some(0));
    let atlur = insert_source(&mut runner, tamer, "BT19-051");

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(ballistamon));
    runner.game.drain_effect_queue();

    let pending = runner
        .pending_selection()
        .expect("On Play should offer AtlurBallistamon under a Tamer");
    assert!(pending.is_optional, "printed 'may digivolve' is declinable");
    let action = encode_source_select(tamer.index as u16, 0).unwrap();
    assert!(pending.valid_action_ids.contains(&action));
    runner
        .execute_action(0, action)
        .expect("choose AtlurBallistamon under Tamer");
    runner.auto_resolve().expect("settle digivolution");

    assert_eq!(
        runner.game.players[0].battle_area[ballistamon.index as usize]
            .top_card()
            .handle(),
        atlur
    );
}

#[test]
fn bt19_047_inherited_grants_blocker_to_xros_heart_carrier_on_opponents_turn() {
    let mut runner = runner();
    let carrier = runner.place_stack(0, &[CARD_ID, "BT19-051"]);
    assert!(
        !runner.game.has_keyword(carrier, Keyword::Blocker),
        "Ballistamon inherited Blocker is opponent-turn only"
    );

    runner.pass_turn();

    assert!(
        runner.game.has_keyword(carrier, Keyword::Blocker),
        "Ballistamon inherited text should grant Blocker to a Xros Heart carrier on opponent's turn"
    );
}

#[test]
fn bt19_047_save_is_auto_installed_from_printed_keyword() {
    let mut runner = runner();
    let ballistamon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "XROS-TAMER", Some(0));

    runner
        .game
        .delete_permanent_with_cause(ballistamon, ReplacementCause::OpponentEffect);

    let pending = runner
        .pending_selection()
        .expect("Save should offer a Tamer destination after deletion");
    assert!(pending.is_optional);
}
