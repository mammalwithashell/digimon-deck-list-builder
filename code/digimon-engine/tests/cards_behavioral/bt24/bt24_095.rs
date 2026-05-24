//! BT24-095 Sonic Shot.

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledDeclarativeClause, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::{encode_attack, PASS};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{OptionPlayResult, SelectionKind};

const YAML: &str = include_str!("../../../cards/bt24/BT24-095.yaml");

#[test]
fn bt24_095_has_color_bypass_main_security_and_link_requirement() {
    let runner = sonic_runner().start();
    let compiled = runner
        .compiled_card("BT24-095")
        .expect("BT24-095 compiled card present");

    assert_eq!(compiled.card, "BT24-095");
    assert_eq!(compiled.name, "Sonic Shot");
    assert_eq!(compiled.kind, CompiledCardKind::Option);
    assert_eq!(compiled.cost, Some(3));
    assert!(compiled.traits.iter().any(|t| t == "TS"));
    assert!(compiled.use_requirement.is_some());
    assert!(compiled.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Declarative(CompiledDeclarativeClause::FloodGate { modifier, .. })
            if modifier == "IgnoreColorRequirement"
    )));

    let main = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(t) if t.when == vec![CompiledTiming::MainFromHand] => Some(t),
            _ => None,
        })
        .expect("MainFromHand clause");
    assert!(main
        .process
        .iter()
        .any(|step| matches!(step, CompiledStep::SelectOpponentPermanent { .. })));
    assert!(main
        .process
        .iter()
        .any(|step| matches!(step, CompiledStep::Suspend { .. })));
    assert!(main
        .process
        .iter()
        .any(|step| matches!(step, CompiledStep::AddModifier { modifier, .. } if modifier == "CannotUnsuspend")));
    assert!(main.process.iter().any(|step| {
        matches!(
            step,
            CompiledStep::LinkToOwnDigimon {
                optional: true,
                free: true,
                ..
            }
        )
    }));

    assert!(compiled.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Triggered(t) if t.scope == digimon_dsl::compiled::CompiledScope::Inherited
            && t.when == vec![CompiledTiming::OnSecurity]
    )));
    assert!(compiled.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Declarative(CompiledDeclarativeClause::LinkRequirement { cost: 3, filter, .. })
            if filter.trait_has.as_deref() == Some("TS")
    )));
}

#[test]
fn bt24_095_main_suspends_opponent_digimon_and_prevents_unsuspend_then_declines_link() {
    let mut runner = sonic_runner().hand(0, &["BT24-095"]).memory(20).start();
    let host = runner.place_on_field(0, "TS-DIGIMON", Some(0));
    let target = runner.place_on_field(1, "OPP-DIGIMON", Some(1));
    runner.game.enter_main_phase();

    assert_eq!(play_sonic_standard(&mut runner), OptionPlayResult::Pending);

    let target_view = runner
        .pending_selection_view()
        .expect("Sonic Shot target prompt");
    assert_eq!(target_view.kind, SelectionKind::OppField);
    runner
        .execute_action(
            target_view.selecting_player,
            encode_attack(0, target.index as u16),
        )
        .expect("choose opponent Digimon");

    let target_handle = PermanentHandle {
        player: 1,
        index: target.index,
    };
    assert!(runner.game.player(1).battle_area[target.index as usize].is_suspended);
    assert!(
        runner
            .game
            .modifiers
            .has(target_handle, ModifierType::CannotUnsuspend),
        "selected permanent should be locked through the opponent's next unsuspend phase"
    );

    let link_view = runner
        .pending_selection_view()
        .expect("optional link prompt follows suspend");
    assert_eq!(link_view.kind, SelectionKind::OwnField);
    assert!(link_view
        .valid_action_ids
        .contains(&encode_attack(0, host.index as u16)));
    assert!(runner.pending_is_optional());
    runner
        .execute_action(link_view.selecting_player, PASS)
        .expect("decline optional free link");
}

#[test]
fn bt24_095_main_can_link_to_ts_digimon_after_suspending_target() {
    let mut runner = sonic_runner().hand(0, &["BT24-095"]).memory(20).start();
    let host = runner.place_on_field(0, "TS-DIGIMON", Some(0));
    let target = runner.place_on_field(1, "OPP-TAMER", Some(1));
    runner.game.enter_main_phase();

    assert_eq!(play_sonic_standard(&mut runner), OptionPlayResult::Pending);
    let target_view = runner
        .pending_selection_view()
        .expect("Sonic Shot target prompt");
    runner
        .execute_action(
            target_view.selecting_player,
            encode_attack(0, target.index as u16),
        )
        .expect("choose opponent Tamer");

    let link_view = runner
        .pending_selection_view()
        .expect("optional link prompt");
    runner
        .execute_action(
            link_view.selecting_player,
            encode_attack(0, host.index as u16),
        )
        .expect("link Sonic Shot to the TS Digimon");

    let linked = &runner.game.player(0).battle_area[host.index as usize].linked_cards;
    assert_eq!(linked.len(), 1);
    assert_eq!(linked[0].card_id(&runner.game.card_data), "BT24-095");
}

fn sonic_runner() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT24-095 YAML loads")
        .add_card(make_ts_digimon("TS-DIGIMON", 4))
        .add_card(make_digimon("OPP-DIGIMON", 4))
        .add_card(make_tamer("OPP-TAMER", &[], CardColor::Green))
}

fn play_sonic_standard(runner: &mut digimon_engine::debug_runner::DebugRunner) -> OptionPlayResult {
    let result = runner.game.play_option_from_hand(0, 0);
    if matches!(
        runner.game.pending_selection.as_ref().map(|s| &s.kind),
        Some(SelectionKind::EffectChoice)
    ) {
        let first = runner
            .game
            .pending_selection
            .as_ref()
            .unwrap()
            .valid_action_ids[0];
        runner
            .game
            .resolve_selection(0, first)
            .expect("choose Standard [Main] mode");
    }
    result
}

fn make_ts_digimon(id: &str, level: u8) -> digimon_engine::CardData {
    let mut card = make_digimon(id, level);
    card.traits = vec!["TS".to_string()];
    card
}

fn make_digimon(id: &str, level: u8) -> digimon_engine::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Green];
    card.level = Some(level);
    card.dp = Some(i32::from(level) * 1000);
    card.play_cost = level as u16;
    card
}

fn make_tamer(id: &str, traits: &[&str], color: CardColor) -> digimon_engine::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Tamer;
    card.colors = vec![color];
    card.level = None;
    card.dp = None;
    card.play_cost = 3;
    card.traits = traits.iter().map(|s| s.to_string()).collect();
    card
}
