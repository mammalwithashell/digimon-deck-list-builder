use digimon_engine::debug_runner::{make_test_card, DebugRunner};

#[test]
fn security_scope_end_of_opponents_turn_plays_this_security_card() {
    let yaml = r#"
card: SEC-EOT
name: Security EOT Digimon
kind: digimon
level: 6
cost: 11
dp: 11000
color: [black]
traits: [Cyborg]

effects:
  - scope: security
    when: end_of_opponents_turn
    summary: "Security: play this card at end of opponent's turn"
    process:
      - play_from_security: {}
"#;

    let filler = ["FILLER"; 5];
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("security-scope DSL parses")
        .add_card(make_test_card("FILLER", "Filler"))
        .security(0, &["SEC-EOT", "FILLER"])
        .deck(0, &filler)
        .deck(1, &filler)
        .memory(3)
        .start();

    runner.end_turn();
    assert_eq!(runner.game.turn_player(), 1);
    assert_eq!(runner.game.players[0].battle_area.len(), 0);

    runner.end_turn();

    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "SEC-EOT"),
        "security-scope EOT effect must play the triggering security card"
    );
    assert_eq!(
        runner.game.players[0].security.len(),
        1,
        "only the triggering security card is consumed"
    );
    assert_eq!(
        runner.game.players[0].security[0].card_id(&runner.game.card_data),
        "FILLER",
        "the top non-triggering security card must remain in security"
    );
}
