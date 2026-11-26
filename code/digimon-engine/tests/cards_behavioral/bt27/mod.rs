mod bt27_008;
mod bt27_009;
mod bt27_013;
mod bt27_015;
mod bt27_016;
mod bt27_021;
mod bt27_034;
mod bt27_043;
mod bt27_060;
mod bt27_065;
mod bt27_068;
mod bt27_070;
mod bt27_072;
mod bt27_084;
mod bt27_099;
mod bt27_100;
mod bt27_103;

use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

fn board(card: &str) -> DebugRunner {
    let mut level4 = make_test_card("LEVEL4", "Level 4");
    level4.level = Some(4);
    level4.dp = Some(7000);
    let mut examon_option = make_test_card("EXAMON-OPTION", "Examon Option");
    examon_option.card_kind = CardKind::Option;
    let mut filler = make_test_card("FILLER", "Filler");
    filler.colors = vec![CardColor::Red, CardColor::Yellow, CardColor::Black];
    let mut builder = DebugRunner::builder()
        .dsl_card(card)
        .unwrap()
        .add_card(filler)
        .add_card(level4)
        .add_card(make_test_card("JESMON", "Jesmon"))
        .add_card(make_test_card("HUCKMON", "Huckmon"))
        .add_card(make_test_card("KNIGHTMON", "Knightmon"))
        .add_card(make_test_card("EXAMON", "Examon"))
        .add_card(make_test_card("CRANIAMON", "Craniamon"))
        .add_card(make_test_card("KENTAUR", "Kentaurosmon"))
        .add_card(examon_option)
        .deck(0, &["FILLER"; 12])
        .deck(1, &["FILLER"; 12]);
    if card == "BT27-021" {
        builder = builder.deck(0, &["EXAMON-OPTION", "EXAMON", "FILLER"]);
    }
    if card == "BT27-009" {
        builder = builder
            .dsl_card("BT27-013")
            .unwrap()
            .security(1, &["FILLER"]);
    }
    if card == "BT27-015" {
        builder = builder
            .dsl_card("BT27-008")
            .unwrap()
            .dsl_card("BT27-009")
            .unwrap()
            .dsl_card("BT27-013")
            .unwrap();
    }
    if card == "BT27-043" || card == "BT27-099" {
        builder = builder.security(0, &["FILLER"]);
    }
    if card == "BT27-099" {
        builder = builder
            .dsl_card("BT27-043")
            .unwrap()
            .security(0, &["BT27-099"]);
    }
    if card == "BT27-100" {
        builder = builder.security(0, &["BT27-100"]);
    }
    if card == "BT27-065" || card == "BT27-016" {
        builder = builder.security(1, &["FILLER"]);
    }
    if card == "BT27-072" {
        builder = builder
            .dsl_card("BT27-065")
            .unwrap()
            .dsl_card("BT27-060")
            .unwrap();
    }
    builder.start()
}

fn trigger(r: &mut DebugRunner, timing: EffectTiming, source: PermanentHandle) {
    r.game
        .enqueue_triggered(timing, TriggerSource::Permanent(source));
    r.game.drain_effect_queue();
}

fn resolve_only_bt27_target(r: &mut DebugRunner) {
    let pick = r
        .pending_selection_view()
        .expect("Expected a target choice");
    assert_eq!(pick.valid_action_ids.len(), 1, "{pick:?}");
    assert!(
        digimon_engine::build_action_mask(&r.game, pick.selecting_player)
            [pick.valid_action_ids[0] as usize]
            > 0.5
    );
    r.execute_action(pick.selecting_player, pick.valid_action_ids[0])
        .unwrap();
}
