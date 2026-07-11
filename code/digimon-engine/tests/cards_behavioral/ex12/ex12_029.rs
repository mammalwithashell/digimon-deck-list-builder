use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::enums::{CardColor, Keyword, ModifierType};

use super::support::{
    blue_sw, hand_index, plain_digimon, play_declining_optional_materials, select_first_non_pass,
    DebugRunner,
};

const CARD_ID: &str = "EX12-029";

#[test]
fn ex12_029_inherited_when_attacking_is_not_optional_after_trigger() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-029 YAML loads")
        .start();
    let card = runner.compiled_card(CARD_ID).expect("compiled EX12-029");
    let inherited = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(triggered)
                if triggered.scope == CompiledScope::Inherited
                    && triggered.when.contains(&CompiledTiming::WhenAttacking) =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("printed inherited [When Attacking] clause");

    assert!(
        !inherited.optional,
        "printed inherited body has no 'you may' after the trigger resolves"
    );
}

#[test]
fn ex12_029_has_blocker_keyword() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-029 YAML loads")
        .start();

    let sagomon = runner.place_on_field(0, CARD_ID, Some(0));

    assert!(
        runner.game.has_keyword(sagomon, Keyword::Blocker),
        "Sagomon has printed <Blocker>"
    );
}

#[test]
fn ex12_029_on_play_locks_opponent_then_grants_alliance_and_attack_prompt() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-029 YAML loads")
        .add_card(blue_sw("SW-ALLY", 4))
        .add_card(plain_digimon("OPP", CardColor::Blue, 4, 5000))
        .add_card(plain_digimon("SEC", CardColor::Blue, 3, 3000))
        .hand(0, &[CARD_ID])
        .security(1, &["SEC"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;
    let ally = runner.place_on_field(0, "SW-ALLY", Some(0));
    let opp = runner.place_on_field(1, "OPP", Some(0));

    let play_slot = hand_index(&runner, 0, CARD_ID);
    play_declining_optional_materials(&mut runner, 0, play_slot);
    select_first_non_pass(&mut runner);
    assert!(
        runner.modifiers().has(opp, ModifierType::CannotSuspend),
        "target can't suspend until their turn ends"
    );

    select_first_non_pass(&mut runner);
    let view = runner
        .pending_selection_view()
        .expect("Alliance rider should expose optional attack targets");
    assert!(view.is_optional, "the immediate attack rider is optional");
    assert!(
        runner.game.has_keyword(ally, Keyword::Alliance),
        "the selected other [SW] Digimon gains Alliance"
    );
}
