//! BT27-075's printed cost clause: `data/card_bundles/BT27-075.md`.

use digimon_engine::action::space::{encode_breeding_select, PLAY_HAND_START};
use digimon_engine::build_action_mask;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, GamePhase};
use digimon_engine::selection::SelectionKind;

fn protogizmon_payment() -> DebugRunner {
    let yaml = r#"
card: BT27-075
name: ProtoGizmon
kind: digimon
color: [purple]
level: 3
cost: 3
dp: 3000
effects:
  - kind: cost_reduction
    when_playing_this: true
    optional: true
    amount: 2
    text: "When this card would be played, by deleting 1 of your level 2 Digimon in the breeding area, reduce the cost by 2."
    pay_cost:
      - select_own_breeding_permanent:
          bind_as: payer
          filter: { kind: digimon, level_eq: 2 }
          prompt: Choose your level 2 Digimon in the breeding area to delete
      - pay_breeding_deletion: { target: payer }
"#;
    let mut egg = make_test_card("PAY-EGG", "Level 2");
    egg.card_kind = CardKind::DigiEgg;
    egg.level = Some(2);
    let mut rookie = make_test_card("PAY-ROOKIE", "Level 3");
    rookie.level = Some(3);
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .unwrap()
        .add_card(egg)
        .add_card(rookie)
        .deck(0, &["PAY-ROOKIE"; 5])
        .deck(1, &["PAY-ROOKIE"; 5])
        .hand(0, &["BT27-075"])
        .memory(10)
        .start();
    runner.set_phase(GamePhase::Main);
    runner
}

#[test]
fn bt27_075_breeding_payment_resumes_from_a_cloned_selection() {
    let mut runner = protogizmon_payment();
    let egg = runner.place_in_breeding(0, "PAY-EGG").card;
    runner.place_in_breeding(1, "PAY-EGG");
    let before = runner.memory();
    runner.game.decode_action(PLAY_HAND_START, 0);
    let mut declined = DebugRunner::wrap(runner.game.clone());
    declined.decline_optional_trigger().unwrap();
    assert_eq!(before - declined.memory(), 3);
    assert!(declined.game.players[0].breeding_area.is_some());

    runner.accept_optional_trigger().unwrap();
    let pick = encode_breeding_select(0).unwrap();
    let pending = runner.pending_selection().unwrap();
    assert_eq!(pending.kind, SelectionKind::BreedingPermanent);
    assert_eq!(pending.valid_action_ids, [pick]);
    assert_eq!(build_action_mask(&runner.game, 0)[pick as usize], 1.0);
    assert_eq!(before, runner.memory());
    assert!(runner.game.players[0].trash.is_empty());

    let clone = DebugRunner::wrap(runner.game.clone());
    let mut stale = DebugRunner::wrap(runner.game.clone());
    let replacement = stale.place_in_breeding(0, "PAY-EGG").card;
    stale.execute_action(0, pick).unwrap();
    assert_eq!(before - stale.memory(), 3);
    assert_eq!(
        stale.game.players[0]
            .breeding_area
            .as_ref()
            .unwrap()
            .top_card()
            .handle(),
        replacement
    );
    assert!(stale.game.players[0].trash.is_empty());

    for mut branch in [runner, clone] {
        branch.execute_action(0, pick).unwrap();
        assert_eq!(before - branch.memory(), 1);
        assert_eq!(branch.battle_area_size(0), 1);
        assert!(branch.game.players[0].breeding_area.is_none());
        assert_eq!(branch.game.players[0].trash[0].handle(), egg);
        assert!(branch.game.players[1].breeding_area.is_some());
        assert!(branch.pending_selection().is_none());
    }
}

#[test]
fn bt27_075_does_not_offer_payment_without_an_own_level_two_in_breeding() {
    for own_breeding in [None, Some("PAY-ROOKIE")] {
        let mut runner = protogizmon_payment();
        runner.place_in_breeding(1, "PAY-EGG");
        if let Some(card) = own_breeding {
            runner.place_in_breeding(0, card);
        }
        let before = runner.memory();
        runner.game.decode_action(PLAY_HAND_START, 0);
        assert!(runner.pending_selection().is_none());
        assert_eq!(before - runner.memory(), 3);
        assert!(runner.game.players[0].trash.is_empty());
        assert!(runner.game.players[1].breeding_area.is_some());
    }
}
