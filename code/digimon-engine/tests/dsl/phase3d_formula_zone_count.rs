use digimon_dsl::common::PlayerRef;
use digimon_dsl::compiled::{
    CompiledFormula, CompiledPerSelector, CompiledPlayerRef, CompiledZone,
};
use digimon_dsl::formula::{CardCountInZoneSpec, FormulaSpec, PerSelector};
use digimon_dsl::predicate::Zone;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::formula_eval;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::PlayerId;

fn push_card_to_trash(runner: &mut DebugRunner, player: PlayerId, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|card| card.card_id == card_id)
        .expect("card_id registered");
    let next = runner.game.next_card_index();
    runner.game.players[player as usize]
        .trash
        .push(CardSource::new(data_idx, player, next));
}

#[test]
fn card_count_in_zone_counts_controller_trash() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .add_card(make_test_card("TRASH-A", "TRASH-A"))
        .add_card(make_test_card("TRASH-B", "TRASH-B"))
        .add_card(make_test_card("OPP-TRASH", "OPP-TRASH"))
        .build();
    let target = runner.place_on_field(0, "SRC", None);
    push_card_to_trash(&mut runner, 0, "TRASH-A");
    push_card_to_trash(&mut runner, 0, "TRASH-B");
    push_card_to_trash(&mut runner, 1, "OPP-TRASH");

    let formula = CompiledFormula::BasePerDelta {
        base: 1000,
        per: CompiledPerSelector::CardCountInZoneScoped {
            zone: CompiledZone::Trash,
            of: CompiledPlayerRef::You,
        },
        delta: 2000,
    };

    let src_card = runner.game.players[0].battle_area[0].top_card().handle();
    let ctx = EffectContext::new(&mut runner.game, src_card, Some(target), 0);

    assert_eq!(formula_eval::evaluate(&formula, &ctx, target), 5000);
}

#[test]
fn card_count_in_zone_can_sum_any_players_security() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .add_card(make_test_card("SEC-A", "SEC-A"))
        .add_card(make_test_card("SEC-B", "SEC-B"))
        .add_card(make_test_card("SEC-C", "SEC-C"))
        .security(0, &["SEC-A", "SEC-B"])
        .security(1, &["SEC-C"])
        .build();
    let target = runner.place_on_field(0, "SRC", None);

    let formula = CompiledFormula::BasePerDelta {
        base: 0,
        per: CompiledPerSelector::CardCountInZoneScoped {
            zone: CompiledZone::Security,
            of: CompiledPlayerRef::Any,
        },
        delta: 1,
    };

    let src_card = runner.game.players[0].battle_area[0].top_card().handle();
    let ctx = EffectContext::new(&mut runner.game, src_card, Some(target), 0);

    assert_eq!(formula_eval::evaluate(&formula, &ctx, target), 3);
}

#[test]
fn card_count_in_zone_surface_parses_payload() {
    let formula: FormulaSpec = serde_yml::from_str(
        r#"
base: 1000
per:
  card_count_in_zone:
    zone: trash
    of: you
delta: 2000
"#,
    )
    .expect("formula parses");

    match formula {
        FormulaSpec::BasePerDelta {
            per:
                PerSelector::CardCountInZone(CardCountInZoneSpec {
                    zone: Zone::Trash,
                    of: PlayerRef::You,
                }),
            ..
        } => {}
        other => panic!("expected card_count_in_zone payload, got {other:?}"),
    }
}
