//! BT19-072 LordKnightmon - Digimon, Lv.6, Purple/White.

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledCardKind, CompiledClause, CompiledColor, CompiledCost,
    CompiledPredicate, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::encode_attack;
use digimon_engine::combat::{AttackInitiator, AttackOpen, TargetConstraint};
use digimon_engine::debug_runner::make_test_card;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::CardKind;
use digimon_engine::selection::{AttackTarget, SelectionKind};

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT19-072")
        .expect("BT19-072 YAML loads")
        .memory(10)
        .start()
}

fn predicate_contains_kind(predicate: &CompiledPredicate, kind: CompiledCardKind) -> bool {
    predicate.kind == Some(kind)
        || predicate
            .any_of
            .iter()
            .any(|part| predicate_contains_kind(part, kind))
        || predicate
            .all_of
            .iter()
            .any(|part| predicate_contains_kind(part, kind))
}

fn predicate_has_level_lte(predicate: &CompiledPredicate, level: u8) -> bool {
    predicate.level_lte == Some(level)
        || predicate
            .any_of
            .iter()
            .any(|part| predicate_has_level_lte(part, level))
        || predicate
            .all_of
            .iter()
            .any(|part| predicate_has_level_lte(part, level))
}

#[test]
fn bt19_072_has_printed_metadata_and_route() {
    let runner = runner();
    let card = runner.compiled_card("BT19-072").expect("BT19-072 present");
    assert_eq!(card.name, "LordKnightmon");
    assert_eq!(card.kind, CompiledCardKind::Digimon);
    assert_eq!(card.level, Some(6));
    assert_eq!(card.cost, Some(11));
    assert_eq!(card.dp, Some(11000));
    assert_eq!(
        card.color,
        vec![CompiledColor::Purple, CompiledColor::White]
    );
    assert!(card.traits.iter().any(|name| name == "Royal Knight"));
    assert_eq!(card.attribute.as_deref(), Some("Virus"));
    assert!(card.alt_paths.iter().any(|path| {
        path.kind == CompiledAltPathKind::Digivolve
            && path.cost == Some(CompiledCost::Literal(3))
            && path.from.as_ref().is_some_and(|from| {
                from.level_eq == Some(5) && from.color_is == Some(CompiledColor::Purple)
            })
    }));
}

#[test]
fn bt19_072_on_play_when_digivolving_selects_level_four_or_lower_digimon_from_trash() {
    let runner = runner();
    let card = runner.compiled_card("BT19-072").expect("BT19-072 present");
    let clause = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(triggered)
                if triggered.when.contains(&CompiledTiming::OnPlay)
                    && triggered.when.contains(&CompiledTiming::WhenDigivolving) =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("trash play clause exists");
    assert!(clause.optional);
    assert!(clause.process.iter().any(|step| matches!(
        step,
        CompiledStep::SelectTrash { filter, .. }
            if predicate_contains_kind(filter, CompiledCardKind::Digimon)
                && predicate_has_level_lte(filter, 4)
    )));
    assert!(clause
        .process
        .iter()
        .any(|step| matches!(step, CompiledStep::PlayFromTrashFree { .. })));
}

#[test]
fn bt19_072_opponents_turn_switches_attack_target_to_royal_knight() {
    let mut attacker_card = make_test_card("ATK", "Attacker");
    attacker_card.card_kind = CardKind::Digimon;
    attacker_card.dp = Some(12000);

    let mut royal_card = make_test_card("RK-TARGET", "Royal Knight Target");
    royal_card.card_kind = CardKind::Digimon;
    royal_card.dp = Some(3000);
    royal_card.traits = vec!["Royal Knight".to_string()];

    let mut security_card = make_test_card("SEC", "Security");
    security_card.dp = Some(2000);

    let mut runner = DebugRunner::builder()
        .dsl_card("BT19-072")
        .expect("BT19-072 YAML loads")
        .add_card(attacker_card)
        .add_card(royal_card)
        .add_card(security_card)
        .security(0, &["SEC"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let _lord = runner.place_on_field(0, "BT19-072", Some(0));
    let royal = runner.place_on_field(0, "RK-TARGET", Some(0));
    let attacker = runner.place_on_field(1, "ATK", Some(0));
    let security_before = runner.game.players[0].security.len();

    runner.game.begin_attack_open(AttackOpen {
        attacker,
        initiator: AttackInitiator::Effect {
            source: None,
            optional: false,
        },
        suspend_attacker: false,
        target_constraint: TargetConstraint::Forced(AttackTarget::Player(0)),
        allow_cancel: false,
        cost_upgrade: None,
    });

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("LordKnightmon should prompt for an own Royal Knight retarget");
    assert_eq!(
        pending.kind,
        SelectionKind::OwnField,
        "retarget choice should select one of your Royal Knight Digimon"
    );
    assert_eq!(
        pending.selecting_player, 0,
        "defending controller should choose the retarget"
    );

    let choose_royal = encode_attack(0, royal.index as u16);
    assert!(
        pending.valid_action_ids.contains(&choose_royal),
        "Royal Knight Digimon should be a legal retarget"
    );
    runner
        .game
        .resolve_selection(0, choose_royal)
        .expect("resolve Royal Knight retarget");

    assert_eq!(
        runner.game.players[0].security.len(),
        security_before,
        "redirected attack must not check security"
    );
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .all(|perm| perm.top_card().card_id(&runner.game.card_data) != "RK-TARGET"),
        "the selected Royal Knight should take the redirected battle"
    );
}
