//! BT11-043 KingSukamon.

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledDeclarativeClause,
    CompiledModifierValue, CompiledStep,
};
use digimon_engine::action::space::REPLACEMENT_ACCEPT;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::TriggerSource;

fn digimon(id: &str, name: &str, level: u8, dp: i32, color: CardColor) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card.colors = vec![color];
    card
}

fn king_sukamon_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT11-043")
        .expect("BT11-043 found in embedded DSL pack")
        .add_card(digimon("TARGET", "Targetmon", 4, 8000, CardColor::Red))
        .add_card(digimon(
            "SUKA-A",
            "Sukamon Alpha",
            3,
            3000,
            CardColor::Yellow,
        ))
        .add_card(digimon(
            "SUKA-B",
            "Chuumon and Sukamon",
            3,
            3000,
            CardColor::Black,
        ))
        .add_card(digimon(
            "SUKA-C",
            "Sukamon Gamma",
            3,
            3000,
            CardColor::White,
        ))
        .add_card(digimon("CARRIER", "Carrier", 6, 9000, CardColor::Yellow))
        .memory(10)
        .start()
}

fn push_to_trash(runner: &mut DebugRunner, player: u8, card_id: &str) -> CardHandle {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|card| card.card_id == card_id)
        .unwrap_or_else(|| panic!("{card_id} missing from card data"));
    let next = runner.game.next_card_index();
    let card = CardSource::new(data_idx, player, next);
    let handle = card.handle();
    runner.game.players[player as usize].trash.push(card);
    handle
}

fn fire(runner: &mut DebugRunner, timing: EffectTiming, handle: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(timing, TriggerSource::Permanent(handle));
    runner.game.drain_effect_queue();
}

fn fire_and_pick_first_target(
    runner: &mut DebugRunner,
    timing: EffectTiming,
    handle: PermanentHandle,
) {
    fire(runner, timing, handle);
    let pick = runner
        .pending_selection_view()
        .expect("KingSukamon transform should select an opponent Digimon");
    runner
        .execute_action(pick.selecting_player, pick.valid_action_ids[0])
        .expect("select opponent Digimon");
    runner.auto_resolve().expect("resolve transform tail");
}

fn colors_for_rules(runner: &DebugRunner, handle: PermanentHandle) -> Vec<CardColor> {
    runner.game.players[handle.player as usize].battle_area[handle.index as usize].colors_for_rules(
        &runner.game.card_data,
        &runner.game.modifiers,
        handle,
    )
}

fn base_dp_for_rules(runner: &DebugRunner, handle: PermanentHandle) -> Option<i32> {
    runner.game.players[handle.player as usize].battle_area[handle.index as usize]
        .base_dp_for_rules(&runner.game.card_data, &runner.game.modifiers, handle)
}

fn has_name_for_rules(runner: &DebugRunner, handle: PermanentHandle, name: &str) -> bool {
    runner.game.players[handle.player as usize].battle_area[handle.index as usize]
        .contains_card_name_for_rules(name, &runner.game.card_data, &runner.game.modifiers, handle)
}

fn field_ids(runner: &DebugRunner, player: u8) -> Vec<String> {
    runner.game.players[player as usize]
        .battle_area
        .iter()
        .map(|perm| perm.top_card().card_id(&runner.game.card_data).to_string())
        .collect()
}

#[test]
fn bt11_043_has_metadata_routes_transform_and_inherited_replacement() {
    let runner = king_sukamon_runner();
    let card = runner
        .compiled_card("BT11-043")
        .expect("BT11-043 compiled card present");

    assert_eq!(card.name, "KingSukamon");
    assert_eq!(card.level, Some(5));
    assert_eq!(card.cost, Some(7));
    assert_eq!(card.dp, Some(6000));
    assert_eq!(
        card.color,
        vec![CompiledColor::Yellow, CompiledColor::Black]
    );
    assert_eq!(card.traits, vec!["Mutant"]);
    assert_eq!(card.attribute.as_deref(), Some("Virus"));
    assert!(card.alt_paths.iter().any(|path| {
        path.kind == CompiledAltPathKind::Digivolve
            && path.cost == Some(CompiledCost::Literal(4))
            && path.from.as_ref().is_some_and(|from| {
                from.level_eq == Some(4) && from.color_is == Some(CompiledColor::Yellow)
            })
    }));
    assert!(card.alt_paths.iter().any(|path| {
        path.kind == CompiledAltPathKind::Digivolve
            && path.cost == Some(CompiledCost::Literal(3))
            && path.from.as_ref().is_some_and(|from| {
                from.level_eq == Some(4) && from.name_contains.as_deref() == Some("Sukamon")
            })
    }));

    let has_transform_steps = card.effects.iter().any(|clause| match clause {
        CompiledClause::Triggered(triggered) => {
            triggered.process.iter().any(|step| {
                matches!(
                    step,
                    CompiledStep::ChangeColor {
                        color: CompiledColor::White,
                        ..
                    }
                )
            }) && triggered.process.iter().any(|step| {
                matches!(
                    step,
                    CompiledStep::AddModifier { modifier, value, .. }
                        if modifier == "ChangeCardDP"
                            && *value == CompiledModifierValue::Literal(3000)
                )
            }) && triggered.process.iter().any(|step| {
                matches!(
                    step,
                    CompiledStep::ChangeOriginalName { name, .. } if name == "Sukamon"
                )
            })
        }
        _ => false,
    });
    assert!(
        has_transform_steps,
        "On Play/When Digivolving branch must transform color, DP, and original name"
    );

    assert!(card.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Declarative(CompiledDeclarativeClause::Replacement { optional: true, .. })
    )));
}

#[test]
fn bt11_043_transform_gate_uses_sukamon_trash_count_and_expires_after_opponent_turn() {
    let mut runner = king_sukamon_runner();
    let king = runner.place_on_field(0, "BT11-043", Some(0));
    let target = runner.place_on_field(1, "TARGET", Some(0));
    for id in ["SUKA-A", "SUKA-B", "SUKA-C"] {
        push_to_trash(&mut runner, 0, id);
    }

    fire_and_pick_first_target(&mut runner, EffectTiming::OnPlay, king);

    assert_eq!(colors_for_rules(&runner, target), vec![CardColor::White]);
    assert_eq!(base_dp_for_rules(&runner, target), Some(3000));
    assert!(has_name_for_rules(&runner, target, "Sukamon"));

    runner.end_turn();
    assert_eq!(colors_for_rules(&runner, target), vec![CardColor::White]);
    runner.game.modifiers.expire_end_of_turn(1);

    assert_eq!(colors_for_rules(&runner, target), vec![CardColor::Red]);
    assert_eq!(base_dp_for_rules(&runner, target), Some(8000));
    assert!(has_name_for_rules(&runner, target, "Targetmon"));
}

#[test]
fn bt11_043_transform_does_not_prompt_when_neither_trash_condition_is_met() {
    let mut runner = king_sukamon_runner();
    let king = runner.place_on_field(0, "BT11-043", Some(0));
    let target = runner.place_on_field(1, "TARGET", Some(0));

    fire(&mut runner, EffectTiming::OnPlay, king);

    assert!(
        runner.pending_selection().is_none(),
        "no target selection should install unless a printed trash condition is true"
    );
    assert_eq!(colors_for_rules(&runner, target), vec![CardColor::Red]);
    assert_eq!(base_dp_for_rules(&runner, target), Some(8000));
    assert!(has_name_for_rules(&runner, target, "Targetmon"));
}

#[test]
fn bt11_043_when_attacking_gains_security_attack_for_each_other_sukamon_in_play() {
    let mut runner = king_sukamon_runner();
    let king = runner.place_on_field(0, "BT11-043", Some(0));
    runner.place_on_field(0, "SUKA-A", Some(0));
    runner.place_on_field(1, "SUKA-B", Some(0));
    runner.place_on_field(1, "TARGET", Some(0));

    fire(&mut runner, EffectTiming::WhenAttacking, king);

    assert_eq!(
        runner
            .game
            .modifiers
            .sum(king, ModifierType::SecurityAttackChange),
        2,
        "KingSukamon should count other Sukamon-name Digimon in play, across both fields"
    );
}

#[test]
fn bt11_043_inherited_deletes_other_sukamon_to_prevent_deletion() {
    let mut runner = king_sukamon_runner();
    let carrier = runner.place_stack(0, &["BT11-043", "CARRIER"]);
    runner.place_on_field(0, "SUKA-A", Some(0));

    runner
        .game
        .delete_permanent_with_cause(carrier, ReplacementCause::OwnEffect);
    let accept = runner
        .pending_selection_view()
        .expect("optional replacement prompt")
        .selecting_player;
    runner
        .execute_action(accept, REPLACEMENT_ACCEPT)
        .expect("accept replacement");
    let pick = runner
        .pending_selection_view()
        .expect("delete-cost selection should choose another Sukamon");
    runner
        .execute_action(pick.selecting_player, pick.valid_action_ids[0])
        .expect("delete other Sukamon");
    runner.auto_resolve().expect("finish replacement");

    let p0_ids = field_ids(&runner, 0);
    assert!(
        p0_ids.contains(&"CARRIER".to_string()),
        "carrier should remain because deletion was prevented"
    );
    assert!(
        !p0_ids.contains(&"SUKA-A".to_string()),
        "chosen other Sukamon should be deleted as the replacement cost"
    );
}
