use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{
    encode_attack, FIELD_EFFECT_SLOT_FOR_MAIN, FIELD_EFFECT_START, PASS, TRASH_EFFECT_START,
};
use digimon_engine::debug_runner::{make_test_card, make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;

fn material_save_yaml(value: u8) -> String {
    format!(
        r#"
card: MS-TEST
name: Material Save Test
kind: digimon
level: 5
color: [red]
cost: 8
dp: 8000
traits: [Xros Heart]
alt_paths:
  - kind: digixros
    materials:
      - filter: {{ name_contains: Shoutmon }}
        repeat: {{ min: 1, max: 1 }}
      - filter: {{ name_contains: Ballistamon }}
        repeat: {{ min: 1, max: 1 }}
    cost:
      formula:
        base: 8
        per: material_count
        delta: -2
effects:
  - kind: grant_keyword
    keyword: MaterialSave
    value: {value}
"#
    )
}

fn xros_material(card_id: &str, name: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card_with_level(card_id, name, 3);
    card.colors = vec![CardColor::Red];
    card.traits = vec!["Xros Heart".to_string()];
    card
}

fn tamer_card(card_id: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(card_id, card_id);
    card.card_kind = CardKind::Tamer;
    card.level = None;
    card.dp = None;
    card.play_cost = 3;
    card.colors = vec![CardColor::Red];
    card
}

fn runner_with_material_save(value: u8, include_tamer: bool) -> DebugRunner {
    let mut builder = DebugRunner::builder()
        .from_dsl_yaml(&material_save_yaml(value))
        .expect("compile Material Save fixture")
        .add_card(xros_material("SRC-SHOUT", "Shoutmon"))
        .add_card(xros_material("SRC-BALLISTA", "Ballistamon"))
        .add_card(xros_material("SRC-OTHER", "Unrelated"));
    if include_tamer {
        builder = builder.add_card(tamer_card("TAMER"));
    }
    builder.memory(10).start()
}

fn place_material_save_stack(
    runner: &mut DebugRunner,
) -> digimon_engine::permanent::PermanentHandle {
    runner.place_stack(0, &["SRC-SHOUT", "SRC-BALLISTA", "SRC-OTHER", "MS-TEST"])
}

fn top_ids_under_tamer(runner: &DebugRunner) -> Vec<String> {
    runner.game.players[0]
        .battle_area
        .iter()
        .find(|permanent| permanent.top_card().card_id(&runner.game.card_data) == "TAMER")
        .expect("Tamer remains on field")
        .card_sources
        .iter()
        .map(|card| card.card_id(&runner.game.card_data).to_string())
        .collect()
}

#[test]
fn material_save_does_not_emit_main_phase_action() {
    let mut runner = runner_with_material_save(2, true);
    let carrier = place_material_save_stack(&mut runner);
    runner.place_on_field(0, "TAMER", Some(0));

    let mask = build_action_mask(&runner.game, 0);
    let material_save_main_bit = FIELD_EFFECT_START
        + carrier.index as u16 * digimon_engine::action::space::EFFECTS_PER_PERMANENT
        + FIELD_EFFECT_SLOT_FOR_MAIN;
    assert_eq!(
        mask[material_save_main_bit as usize], 0.0,
        "Material Save must not be surfaced as a main-phase field activation"
    );
}

#[test]
fn material_save_decline_leaves_sources_in_trash() {
    let mut runner = runner_with_material_save(2, true);
    let carrier = place_material_save_stack(&mut runner);
    runner.place_on_field(0, "TAMER", Some(0));

    runner
        .game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);

    let prompt = runner
        .pending_selection()
        .expect("Material Save Tamer prompt should be offered");
    assert_eq!(prompt.kind, SelectionKind::OwnField);
    assert!(prompt.is_optional);

    runner
        .execute_action(0, PASS)
        .expect("decline Material Save");
    assert!(runner.pending_selection().is_none());
    assert_eq!(
        top_ids_under_tamer(&runner),
        vec!["TAMER".to_string()],
        "declining Material Save leaves the Tamer stack unchanged"
    );
    let saved_source_ids: Vec<_> = runner.game.players[0]
        .trash
        .iter()
        .map(|card| card.card_id(&runner.game.card_data).to_string())
        .collect();
    assert!(saved_source_ids.iter().any(|id| id == "SRC-SHOUT"));
    assert!(saved_source_ids.iter().any(|id| id == "SRC-BALLISTA"));
}

#[test]
fn material_save_filters_recipe_sources_and_enforces_cap() {
    let mut runner = runner_with_material_save(1, true);
    let carrier = place_material_save_stack(&mut runner);
    runner.place_on_field(0, "TAMER", Some(0));

    runner
        .game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);

    let tamer_prompt = runner
        .pending_selection()
        .expect("Material Save should ask for a Tamer destination");
    assert!(tamer_prompt.is_optional);
    assert!(
        tamer_prompt.valid_action_ids.contains(&encode_attack(0, 0)),
        "remaining Tamer should be selectable after the carrier leaves"
    );
    runner
        .execute_action(0, encode_attack(0, 0))
        .expect("choose Tamer destination");

    let source_prompt = runner
        .pending_selection()
        .expect("Material Save should ask for eligible sources");
    assert!(matches!(
        source_prompt.kind,
        SelectionKind::CountCappedMultiSelect { .. }
    ));
    let valid_source_ids: Vec<_> = source_prompt
        .valid_action_ids
        .iter()
        .map(|action| {
            let trash_index = action.saturating_sub(TRASH_EFFECT_START) as usize;
            runner.game.players[0].trash[trash_index]
                .card_id(&runner.game.card_data)
                .to_string()
        })
        .collect();
    assert!(valid_source_ids.iter().any(|id| id == "SRC-SHOUT"));
    assert!(valid_source_ids.iter().any(|id| id == "SRC-BALLISTA"));
    assert!(
        !valid_source_ids.iter().any(|id| id == "SRC-OTHER"),
        "source selection must be filtered by the carrier's DigiXros recipe"
    );

    let first_pick = source_prompt.valid_action_ids[0];
    runner
        .execute_action(0, first_pick)
        .expect("select one Material Save source");
    assert!(
        runner.pending_selection().is_none(),
        "Material Save 1 should commit immediately after one source"
    );
    let tamer_sources = top_ids_under_tamer(&runner);
    let saved_count = tamer_sources
        .iter()
        .filter(|id| *id == "SRC-SHOUT" || *id == "SRC-BALLISTA")
        .count();
    assert_eq!(saved_count, 1);
    assert!(!tamer_sources.iter().any(|id| id == "SRC-OTHER"));
}

#[test]
fn material_save_skips_when_no_tamer_or_no_eligible_source_exists() {
    let mut no_tamer = runner_with_material_save(2, false);
    let carrier = place_material_save_stack(&mut no_tamer);
    no_tamer
        .game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    assert!(no_tamer.pending_selection().is_none());

    let mut no_eligible = runner_with_material_save(2, true);
    let carrier = no_eligible.place_stack(0, &["SRC-OTHER", "MS-TEST"]);
    no_eligible.place_on_field(0, "TAMER", Some(0));
    no_eligible
        .game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    assert!(no_eligible.pending_selection().is_none());
}
