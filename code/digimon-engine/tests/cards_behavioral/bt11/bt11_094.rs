//! BT11-094 Mirei Mikagura.

use digimon_engine::action::build_action_mask;
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

const YAML: &str = include_str!("../../../cards/bt11/BT11-094.yaml");

#[test]
fn bt11_094_start_of_turn_gains_memory() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT11-094 YAML loads")
        .memory(0)
        .start();
    let mirei = runner.place_on_field(0, "BT11-094", Some(0));

    fire(&mut runner, EffectTiming::StartOfYourTurn, mirei);

    assert_eq!(runner.memory(), 1);
}

#[test]
fn bt11_094_digivolve_into_angewomon_plays_ladydevimon_from_hand() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT11-094 YAML loads")
        .add_card(lv4("BASE", "Base"))
        .add_card(lv5("ANGE", "Angewomon"))
        .add_card(lv5("LADY", "LadyDevimon"))
        .hand(0, &["LADY"])
        .memory(10)
        .start();
    runner.place_on_field(0, "BT11-094", Some(0));
    let ange = runner.place_stack(0, &["BASE", "ANGE"]);

    enqueue_digivolve_event_for(&mut runner, ange);
    let accept = runner
        .pending_selection_view()
        .expect("Mirei optional trigger")
        .valid_action_ids[0];
    assert_mask_allows(&runner, 0, accept);
    runner.accept_optional_trigger().expect("accept Mirei");
    runner.auto_resolve().expect("play opposite name");

    assert!(permanent_exists(&runner, 0, "LADY"));
}

#[test]
fn bt11_094_security_clause_loads() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT11-094 YAML loads")
        .start();
    let compiled = runner
        .compiled_card("BT11-094")
        .expect("compiled card present");

    assert!(compiled.effects.iter().any(|clause| {
        matches!(
            clause,
            digimon_dsl::compiled::CompiledClause::Triggered(t)
                if t.when.contains(&digimon_dsl::compiled::CompiledTiming::OnSecurity)
        )
    }));
}

fn fire(runner: &mut DebugRunner, timing: EffectTiming, handle: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(timing, TriggerSource::Permanent(handle));
    runner.game.drain_effect_queue();
}

fn enqueue_digivolve_event_for(runner: &mut DebugRunner, digivolved: PermanentHandle) {
    let card = runner.game.players[digivolved.player as usize].battle_area
        [digivolved.index as usize]
        .top_card()
        .handle();
    runner.game.enqueue_triggered(
        EffectTiming::OnDigivolve,
        TriggerSource::Digivolved {
            player: digivolved.player,
            permanent: digivolved,
            card,
            effect_initiated: false,
            dna_origin: false,
        },
    );
    runner.game.drain_effect_queue();
}

fn lv4(id: &str, name: &str) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.colors = vec![CardColor::Yellow];
    card
}

fn lv5(id: &str, name: &str) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Digimon;
    card.level = Some(5);
    card.colors = vec![CardColor::Yellow];
    card.evo_costs = vec![EvoCost {
        card_color: CardColor::Yellow as u8,
        level: 4,
        memory_cost: 3,
    }];
    card
}

fn permanent_exists(runner: &DebugRunner, player: usize, card_id: &str) -> bool {
    runner.game.players[player]
        .battle_area
        .iter()
        .any(|permanent| permanent.top_card().card_id(&runner.game.card_data) == card_id)
}

fn assert_mask_allows(runner: &DebugRunner, player: u8, action: u16) {
    assert_eq!(
        build_action_mask(&runner.game, player)[action as usize],
        1.0,
        "pending action {action} must be legal in the action mask"
    );
}
