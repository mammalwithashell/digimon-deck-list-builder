use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, DelayTrigger};
use digimon_engine::permanent::OptionState;
use digimon_engine::replacement::ReplacementCause;
const YAML: &str = include_str!("../cards/bt17/BT17-095.yaml");
fn runner(
    owner: u8,
    name: &str,
    level: u8,
    delayed: bool,
) -> (DebugRunner, digimon_engine::permanent::PermanentHandle) {
    let mut target = make_test_card("LEAVING", name);
    target.card_kind = CardKind::Digimon;
    target.level = Some(level);
    target.dp = Some(11000);
    target.colors = vec![CardColor::Red];
    let mut omni = make_test_card("OMNI", "Omnimon");
    omni.level = Some(7);
    omni.dp = Some(15000);
    let mut partner = make_test_card("PARTNER", "Garurumon");
    partner.level = Some(6);
    partner.dp = Some(11000);
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .unwrap()
        .add_card(target)
        .add_card(omni)
        .add_card(partner)
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .hand(0, &["OMNI", "PARTNER"])
        .memory(10)
        .start();
    let option = r.place_on_field(0, "BT17-095", Some(0));
    if delayed {
        let turn = r.game.turn_count;
        r.game.players[0].battle_area[option.index as usize].option_state = OptionState::Delayed {
            owner: 0,
            trash_on_turn: turn + 2,
            trigger: DelayTrigger::EndOfYourNextTurn,
            placed_on_turn: turn,
        };
        r.game.turn_count += 1;
    }
    let target = r.place_on_field(owner, "LEAVING", None);
    (r, target)
}
fn remains(r: &DebugRunner) -> bool {
    r.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&r.game.card_data) == "BT17-095")
}
#[test]
fn raw_standard_option_is_rule_trashed_not_a_delay_replacement() {
    let (mut r, target) = runner(1, "Greymon", 6, false);
    r.game
        .delete_permanent_with_cause(target, ReplacementCause::OwnEffect);
    r.game.drain_effect_queue();
    assert!(r.game.pending_selection.is_none());
    assert!(!remains(&r));
    assert!(r.game.players[0]
        .trash
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == "BT17-095"));
}
#[test]
fn mature_delay_rejects_each_nonmatching_subject_and_stays() {
    for (owner, name, level) in [(1, "Greymon", 6), (0, "OtherL6", 6), (0, "Greymon", 3)] {
        let (mut r, target) = runner(owner, name, level, true);
        r.game
            .delete_permanent_with_cause(target, ReplacementCause::OwnEffect);
        r.game.drain_effect_queue();
        assert!(
            r.game.pending_selection.is_none(),
            "unexpected replacement: owner={owner},name={name},level={level}"
        );
        assert!(
            remains(&r),
            "valid Delay Option must survive: {owner}/{name}/{level}"
        );
        assert!(r.game.players[owner as usize]
            .trash
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "LEAVING"));
    }
}
#[test]
fn same_mature_delay_fixture_does_offer_matching_own_level_six() {
    let (mut r, target) = runner(0, "Greymon", 6, true);
    r.game
        .delete_permanent_with_cause(target, ReplacementCause::OwnEffect);
    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("matching positive control must offer replacement");
    assert_eq!(pending.selecting_player, 0);
    assert!(pending.is_optional);
    assert!(remains(&r));
}
