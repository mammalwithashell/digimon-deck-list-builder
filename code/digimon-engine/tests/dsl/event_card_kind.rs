//! Runtime event-card-kind predicate attribution, independent of card-specific tests.
use digimon_engine::{
    debug_runner::{make_test_card, DebugRunner},
    enums::CardKind,
};

#[test]
fn event_card_kind_reads_the_played_card_instead_of_the_observer() {
    const YAML: &str = r#"
card: EVENT-KIND-OBSERVER
name: Event Kind Observer
kind: tamer
color: [yellow]
cost: 0
effects:
  - when: on_enter_field_anyone
    trigger_if: { event_card_kind: digimon }
    process:
      - gain_memory: 1
"#;
    for (kind, expected) in [(CardKind::Digimon, 1), (CardKind::Tamer, 0)] {
        let mut subject = make_test_card("SUBJECT", "Subject");
        subject.card_kind = kind;
        subject.play_cost = 0;
        if kind == CardKind::Tamer {
            subject.level = None;
            subject.dp = None;
        }
        let mut runner = DebugRunner::builder()
            .from_dsl_yaml(YAML)
            .unwrap()
            .add_card(subject)
            .add_card(make_test_card("FILL", "Filler"))
            .hand(0, &["SUBJECT"])
            .deck(0, &["FILL"; 6])
            .deck(1, &["FILL"; 6])
            .memory(4)
            .start();
        runner.skip_mulligan();
        runner.place_on_field(0, "EVENT-KIND-OBSERVER", None);
        runner.game.enter_main_phase();
        let before = runner.game.memory;
        runner.play(0, 0).unwrap();
        runner.auto_resolve().unwrap();
        assert_eq!(runner.game.memory, before + expected, "event kind {kind:?}");
    }
}
