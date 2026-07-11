use digimon_dsl::compiled::{CompiledClause, CompiledDeclarativeClause, CompiledScope};
use digimon_engine::enums::{CardColor, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

use super::support::{
    plain_digimon, select_first_non_pass, select_hand_card, top_card_id, vb_digimon, with_evo_cost,
    DebugRunner,
};

const CARD_ID: &str = "EX12-044";

fn fire_when_attacking(runner: &mut DebugRunner, handle: PermanentHandle) {
    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(handle),
    );
    runner.game.drain_effect_queue();
}

#[test]
fn ex12_044_on_play_gives_one_opponent_digimon_minus_4000_dp() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-044 YAML loads")
        .add_card(plain_digimon("OPP", CardColor::Purple, 5, 7000))
        .start();

    let angewomon = runner.place_on_field(0, CARD_ID, Some(0));
    let opp = runner.place_on_field(1, "OPP", Some(0));
    runner.fire_on_play(0, angewomon.index as usize);

    let view = runner
        .pending_selection_view()
        .expect("Angewomon should select an opponent Digimon");
    assert_eq!(view.kind, SelectionKind::OppField);
    select_first_non_pass(&mut runner);
    runner.auto_resolve().expect("resolve DP modifier");

    assert_eq!(runner.effective_dp(opp), Some(3000));
}

#[test]
fn ex12_044_when_attacking_with_same_level_source_pair_digivolves_from_hand_cost_reduced() {
    let evo = with_evo_cost(
        vb_digimon("VB-LV6", CardColor::Yellow, 6, 11000),
        CardColor::Yellow,
        5,
        3,
    );

    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-044 YAML loads")
        .add_card(plain_digimon("SRC-L4A", CardColor::Yellow, 4, 4000))
        .add_card(plain_digimon("SRC-L4B", CardColor::Green, 4, 4000))
        .add_card(evo)
        .hand(0, &["VB-LV6"])
        .memory(5)
        .start();

    let angewomon = runner.place_stack(0, &["SRC-L4A", "SRC-L4B", CARD_ID]);
    let memory_before = runner.memory();

    fire_when_attacking(&mut runner, angewomon);
    let offer = runner
        .pending_selection_view()
        .expect("same-level source pair should offer hand digivolution");
    assert!(
        offer.is_optional,
        "printed 'may digivolve' must be optional"
    );
    select_first_non_pass(&mut runner);
    select_hand_card(&mut runner, 0, "VB-LV6");
    runner
        .auto_resolve()
        .expect("resolve Angewomon hand digivolution");

    assert_eq!(top_card_id(&runner, 0, angewomon.index as usize), "VB-LV6");
    assert_eq!(
        memory_before - runner.memory(),
        1,
        "cost 3 should be reduced by 2"
    );
}

#[test]
fn ex12_044_inherited_decode_grants_replacement_keyword() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-044 YAML loads")
        .add_card(vb_digimon("CARRIER", CardColor::Yellow, 6, 11000))
        .start();
    let card = runner.compiled_card(CARD_ID).expect("compiled EX12-044");

    let decode_replacements = card
        .effects
        .iter()
        .filter(|clause| {
            matches!(
                clause,
                CompiledClause::Declarative(CompiledDeclarativeClause::Replacement {
                    scope: CompiledScope::Inherited,
                    ..
                })
            )
        })
        .count();
    assert_eq!(
        decode_replacements, 1,
        "EX12-044 inherited Decode should compile as one inherited replacement clause"
    );
}
