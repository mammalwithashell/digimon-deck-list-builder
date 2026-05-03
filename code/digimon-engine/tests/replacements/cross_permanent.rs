use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::replacement::ReplacementCause;

#[test]
fn dsl_source_permanent_can_protect_a_different_subject() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TARGET", "Target"))
        .from_dsl_yaml(CROSS_PERMANENT_REPLACEMENT_YAML)
        .expect("cross-permanent replacement YAML compiles")
        .start();

    r.place_on_field(0, "PROTECTOR", Some(0));
    let protected = r.place_on_field(0, "TARGET", Some(0));

    r.game.set_effect_source_player_for_test(Some(1));
    r.game
        .delete_permanent_with_cause(protected, ReplacementCause::OpponentEffect);
    r.game.set_effect_source_player_for_test(None);

    assert_eq!(
        r.battle_area_size(0),
        2,
        "DSL replacement source should cancel deletion of another own permanent"
    );
    let remaining: Vec<&str> = r
        .game
        .player(0)
        .battle_area
        .iter()
        .map(|perm| perm.top_card().card_id(&r.game.card_data))
        .collect();
    assert!(
        remaining.contains(&"PROTECTOR"),
        "replacement source should remain on field"
    );
    assert!(
        remaining.contains(&"TARGET"),
        "protected subject should remain on field"
    );
}

#[test]
fn dsl_compound_predicate_sibling_does_not_broaden_default_self_scope() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TARGET", "Target"))
        .from_dsl_yaml(COMPOUND_REPLACEMENT_YAML)
        .expect("compound replacement YAML compiles")
        .start();

    r.place_on_field(0, "PROTECTOR", Some(0));
    let protected = r.place_on_field(0, "TARGET", Some(0));

    r.game.set_effect_source_player_for_test(Some(1));
    r.game
        .delete_permanent_with_cause(protected, ReplacementCause::OpponentEffect);
    r.game.set_effect_source_player_for_test(None);

    let remaining: Vec<&str> = r
        .game
        .player(0)
        .battle_area
        .iter()
        .map(|perm| perm.top_card().card_id(&r.game.card_data))
        .collect();
    assert!(
        remaining.contains(&"PROTECTOR"),
        "replacement source should remain on field"
    );
    assert!(
        !remaining.contains(&"TARGET"),
        "source/cause-only any_of sibling must not protect a different subject"
    );
}

const CROSS_PERMANENT_REPLACEMENT_YAML: &str = r#"
card: PROTECTOR
name: Protector
kind: digimon
level: 6
color: [yellow]
cost: 11
dp: 11000
effects:
  - kind: replacement
    trigger: when_would_be_deleted
    active_when:
      replacement_subject_is_mine: true
      replacement_source_is_opponent: true
      replacement_cause: opponent_effect
    process:
      - cancel_replacement: {}
"#;

const COMPOUND_REPLACEMENT_YAML: &str = r#"
card: PROTECTOR
name: Protector
kind: digimon
level: 6
color: [yellow]
cost: 11
dp: 11000
effects:
  - kind: replacement
    trigger: when_would_be_deleted
    active_when:
      any_of:
        - replacement_subject_is_mine: true
          replacement_source_is_opponent: true
          replacement_cause: opponent_effect
        - replacement_source_is_opponent: true
          replacement_cause: opponent_effect
    process:
      - cancel_replacement: {}
"#;
