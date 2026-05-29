use digimon_engine::action::space::{encode_source_select, SEL_REVEAL_START};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::{make_test_card, make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::TriggerSource;

const CARD_ID: &str = "BT19-008";

fn omni_shoutmon() -> CardData {
    let mut card = make_test_card_with_level("BT21-021", "OmniShoutmon", 5);
    card.colors = vec![CardColor::Red, CardColor::Yellow];
    card.traits = vec![
        "Dragonkin".to_string(),
        "Xros Heart".to_string(),
        "Hero".to_string(),
    ];
    card.evo_costs = vec![EvoCost {
        card_color: 0,
        level: 3,
        memory_cost: 4,
    }];
    card
}

fn tamer(card_id: &str, traits: &[&str]) -> CardData {
    let mut card = make_test_card(card_id, card_id);
    card.card_kind = CardKind::Tamer;
    card.level = None;
    card.dp = None;
    card.play_cost = 4;
    card.traits = traits
        .iter()
        .map(|trait_name| (*trait_name).to_string())
        .collect();
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

fn push_to_deck(runner: &mut DebugRunner, player: u8, card_id: &str) {
    let data_index = runner
        .game
        .card_data
        .iter()
        .position(|card| card.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card id {card_id}"));
    let instance_id = runner.game.next_card_index();
    runner.game.players[player as usize]
        .deck
        .push(CardSource::new(data_index, player, instance_id));
}

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT19-008 YAML loads")
        .add_card(omni_shoutmon())
        .add_card(tamer("XROS-TAMER", &["Xros Heart"]))
        .add_card(tamer("PLAIN-TAMER", &[]))
        .add_card(make_test_card("FILL", "FILL"))
        .memory(10)
        .start()
}

#[test]
fn bt19_008_on_play_may_digivolve_from_omnishoutmon_under_tamer() {
    let mut runner = runner();
    let shoutmon = runner.place_on_field(0, CARD_ID, Some(0));
    let tamer = runner.place_on_field(0, "XROS-TAMER", Some(0));
    let omni = insert_source(&mut runner, tamer, "BT21-021");

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(shoutmon));
    runner.game.drain_effect_queue();

    let pending = runner
        .pending_selection()
        .expect("On Play should offer OmniShoutmon under a Tamer");
    assert!(pending.is_optional, "printed 'may digivolve' is declinable");
    let action = encode_source_select(tamer.index as u16, 0).unwrap();
    assert!(pending.valid_action_ids.contains(&action));
    runner
        .execute_action(0, action)
        .expect("choose OmniShoutmon under Tamer");
    runner.auto_resolve().expect("settle digivolution");

    assert_eq!(
        runner.game.players[0].battle_area[shoutmon.index as usize]
            .top_card()
            .handle(),
        omni
    );
}

#[test]
fn bt19_008_on_deletion_plays_revealed_xros_heart_tamer_and_bottoms_remainder() {
    let mut runner = runner();
    let shoutmon = runner.place_on_field(0, CARD_ID, Some(0));
    push_to_deck(&mut runner, 0, "FILL");
    push_to_deck(&mut runner, 0, "PLAIN-TAMER");
    push_to_deck(&mut runner, 0, "XROS-TAMER");

    runner
        .game
        .delete_permanent_with_cause(shoutmon, ReplacementCause::OpponentEffect);

    let pending = runner
        .pending_selection()
        .expect("On Deletion should offer a revealed Xros Heart Tamer");
    assert!(
        pending.valid_action_ids.contains(&SEL_REVEAL_START),
        "top revealed Xros Heart Tamer should be selectable"
    );
    runner
        .execute_action(0, SEL_REVEAL_START)
        .expect("play revealed Xros Heart Tamer");
    runner
        .auto_resolve()
        .expect("bottom-deck remaining revealed cards and finish deletion flow");

    assert!(
        runner.game.players[0].battle_area.iter().any(|permanent| {
            permanent.top_card().card_id(&runner.game.card_data) == "XROS-TAMER"
        }),
        "selected revealed Xros Heart Tamer must be played"
    );
    assert!(
        runner
            .game
            .revealed_cards
            .iter()
            .all(|card| card.card_id(&runner.game.card_data) != "XROS-TAMER"),
        "played Tamer must leave the reveal pool"
    );
}

#[test]
fn bt19_008_inherited_grants_rush_to_xros_heart_carrier() {
    let mut runner = runner();
    let carrier = runner.place_stack(0, &[CARD_ID, "BT21-021"]);

    assert!(
        runner.game.has_keyword(carrier, Keyword::Rush),
        "Shoutmon inherited text should grant Rush to an Xros Heart carrier"
    );
}
