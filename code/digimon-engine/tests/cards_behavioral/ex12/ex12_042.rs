use digimon_engine::enums::{CardColor, Keyword};

use super::support::{hand_contains, plain_digimon, vb_digimon, DebugRunner};

const CARD_ID: &str = "EX12-042";

#[test]
fn ex12_042_has_blocker_and_inherited_barrier() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-042 YAML loads")
        .add_card(vb_digimon("CARRIER", CardColor::Yellow, 5, 7000))
        .start();

    let gatomon = runner.place_on_field(0, CARD_ID, Some(0));
    assert!(
        runner.game.has_keyword(gatomon, Keyword::Blocker),
        "Gatomon should have printed Blocker"
    );

    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    assert!(
        runner.game.has_keyword(carrier, Keyword::Barrier),
        "carrier inherits Barrier from EX12-042"
    );
}

#[test]
fn ex12_042_on_play_adds_top_security_to_hand_then_recovers() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-042 YAML loads")
        .add_card(plain_digimon("SECURITY-CARD", CardColor::Yellow, 3, 2000))
        .add_card(plain_digimon("RECOVERY-CARD", CardColor::Yellow, 3, 2000))
        .security(0, &["SECURITY-CARD"])
        .deck(0, &["RECOVERY-CARD"])
        .memory(5)
        .start();

    let gatomon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.fire_on_play(0, gatomon.index as usize);
    runner.auto_resolve().expect("resolve Gatomon on-play");

    assert!(hand_contains(&runner, 0, "SECURITY-CARD"));
    assert_eq!(runner.security_count(0), 1);
    assert_eq!(
        runner.game.players[0].security[0].card_id(&runner.game.card_data),
        "RECOVERY-CARD",
        "Recovery +1 should replace the security card that was added to hand"
    );
}
