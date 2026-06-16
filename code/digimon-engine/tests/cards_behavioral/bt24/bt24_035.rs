//! BT24-035 Gatomon.

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledCost, CompiledScope, CompiledStep,
    CompiledTiming,
};
use digimon_engine::action::space::{encode_attack, PLAY_HAND_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Keyword};
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "BT24-035";
const SILPHYMON_ID: &str = "BT24-037";

#[test]
fn bt24_035_metadata_alt_paths_trigger_and_inherited_barrier() {
    let runner = gatomon_runner().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled BT24-035");

    assert_eq!(card.kind, CompiledCardKind::Digimon);
    assert_eq!(card.level, Some(4));
    assert_eq!(card.color, vec![CompiledColor::Yellow]);
    assert_eq!(card.cost, Some(4));
    assert_eq!(card.dp, Some(4000));
    assert!(card.traits.iter().any(|trait_name| trait_name == "TS"));
    assert!(card.alt_paths.iter().any(|path| {
        path.cost == Some(CompiledCost::Literal(2))
            && path.from.as_ref().is_some_and(|from| {
                from.level_eq == Some(3) && from.color_is == Some(CompiledColor::Yellow)
            })
    }));
    assert!(card.alt_paths.iter().any(|path| {
        path.cost == Some(CompiledCost::Literal(2))
            && path.from.as_ref().is_some_and(|from| {
                from.all_of.iter().any(|pred| pred.level_eq == Some(3))
                    && from
                        .all_of
                        .iter()
                        .any(|pred| pred.trait_has.as_deref() == Some("TS"))
            })
    }));

    let trigger = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(trigger)
                if trigger.when.contains(&CompiledTiming::OnPlay)
                    && trigger.when.contains(&CompiledTiming::WhenDigivolving) =>
            {
                Some(trigger)
            }
            _ => None,
        })
        .expect("shared On Play / When Digivolving clause");
    assert!(trigger.process.iter().any(|step| matches!(
        step,
        CompiledStep::If { then, .. }
            if then.iter().any(|inner| matches!(inner, CompiledStep::Optional(_)))
    )));

    assert!(card.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Declarative(digimon_dsl::compiled::CompiledDeclarativeClause::GrantKeyword {
            scope: CompiledScope::Inherited,
            keyword,
            ..
        }) if keyword == "Barrier"
    )));
}

#[test]
fn bt24_035_debuffs_then_dna_digivolves_into_silphymon_on_your_turn() {
    let mut runner = gatomon_runner()
        .dsl_card(SILPHYMON_ID)
        .expect("BT24-037 YAML loads")
        .add_card(level4("YELLOW-MAT", CardColor::Yellow))
        .add_card(level4("RED-MAT", CardColor::Red))
        .add_card(level4("GREEN-MAT", CardColor::Green))
        .add_card(level4("BLUE-MAT", CardColor::Blue))
        .add_card(opponent_digimon("OPP", 7000))
        .hand(0, &[SILPHYMON_ID])
        .memory(10)
        .start();
    let gatomon = runner.place_on_field(0, CARD_ID, Some(0));
    let yellow = runner.place_on_field(0, "YELLOW-MAT", Some(0));
    let red = runner.place_on_field(0, "RED-MAT", Some(0));
    runner.place_on_field(0, "BLUE-MAT", Some(0));
    let opponent = runner.place_on_field(1, "OPP", Some(0));
    runner.game.turn_count = 1;

    fire_on_play(&mut runner, gatomon);
    assert_eq!(runner.pending_kind(), Some(SelectionKind::OppField));
    let debuff_prompt = runner
        .pending_selection_view()
        .expect("opponent Digimon selection");
    assert!(
        debuff_prompt
            .valid_action_ids
            .iter()
            .any(|action| *action != digimon_engine::action::space::PASS),
        "opponent Digimon should be offered for the DP reduction"
    );
    let debuff_action = debuff_prompt.valid_action_ids[0];
    runner
        .execute_action(debuff_prompt.selecting_player, debuff_action)
        .expect("select opponent Digimon for -3000 DP");
    assert_eq!(runner.effective_dp(opponent), Some(4000));

    if runner.pending_kind() == Some(SelectionKind::EffectChoice) {
        runner
            .execute_branch(0)
            .expect("accept optional DNA follow-up");
    }
    assert_eq!(runner.pending_kind(), Some(SelectionKind::Hand));
    let hand_action = PLAY_HAND_START;
    runner
        .execute_action(0, hand_action)
        .expect("select Silphymon in hand");

    let first_dna = runner.pending_selection_view().expect("first DNA material");
    assert_eq!(first_dna.kind, SelectionKind::AnyField);
    let yellow_action = encode_attack(0, yellow.index as u16);
    assert!(
        first_dna.valid_action_ids.contains(&yellow_action),
        "yellow level 4 material should be offered first"
    );
    runner
        .execute_action(first_dna.selecting_player, yellow_action)
        .expect("select yellow material");

    let second_dna = runner
        .pending_selection_view()
        .expect("second DNA material");
    let red_action = encode_attack(0, red.index as u16);
    assert!(
        second_dna.valid_action_ids.contains(&red_action),
        "red level 4 material should be offered second"
    );
    assert!(
        !second_dna
            .valid_action_ids
            .contains(&encode_attack(0, yellow.index as u16)),
        "second DNA material must exclude the first pick"
    );
    runner
        .execute_action(second_dna.selecting_player, red_action)
        .expect("select red material");
    runner.auto_resolve().expect("settle DNA digivolve");

    assert!(runner.game.players[0]
        .battle_area
        .iter()
        .any(|perm| perm.top_card().card_id(&runner.game.card_data) == SILPHYMON_ID));
}

#[test]
fn bt24_035_inherited_barrier_is_available_to_carrier() {
    let mut runner = gatomon_runner()
        .add_card(level4("CARRIER", CardColor::Yellow))
        .start();
    let carrier = runner.place_on_field(0, "CARRIER", Some(0));
    runner.push_source(carrier, CARD_ID);
    runner.game.tick_declarative_effects();

    assert!(runner.game.has_keyword(carrier, Keyword::Barrier));
}

fn gatomon_runner() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT24-035 YAML loads")
}

fn fire_on_play(
    runner: &mut digimon_engine::debug_runner::DebugRunner,
    source: digimon_engine::PermanentHandle,
) {
    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(source));
    runner.game.drain_effect_queue();
}

fn level4(id: &str, color: CardColor) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![color];
    card.level = Some(4);
    card.dp = Some(5000);
    card.play_cost = 4;
    card
}

fn opponent_digimon(id: &str, dp: i32) -> CardData {
    let mut card = level4(id, CardColor::Purple);
    card.dp = Some(dp);
    card
}
