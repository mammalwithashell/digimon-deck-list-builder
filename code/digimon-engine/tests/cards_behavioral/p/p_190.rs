//! P-190 Tweetmon — behavioral tests.
//!
//! Printed (official Bandai DB):
//!   <Link> [Appmon] trait: Cost 1
//!   Link DP: DP+2000
//!   [On Play] <Draw 1>
//!   [When Linking] <Draw 1>

use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

#[test]
fn p_190_on_play_draws_1_and_link_aura_grants_2000_dp() {
    let mut filler = make_test_card("P-190-FILL", "Filler");
    filler.card_kind = CardKind::Digimon;
    filler.level = Some(3);
    filler.dp = Some(2000);

    let mut host = make_test_card("P-190-HOST", "Gatchmon");
    host.card_kind = CardKind::Digimon;
    host.level = Some(3);
    host.colors = vec![CardColor::Red];
    host.dp = Some(2000);
    host.traits = vec!["Stnd.".to_string(), "Appmon".to_string()];

    let mut r = DebugRunner::builder()
        .dsl_card("P-190")
        .unwrap()
        .add_card(filler)
        .add_card(host)
        .deck(0, &["P-190-FILL", "P-190-FILL"])
        .start();
    r.skip_mulligan();

    let hand_before = r.game.players[0].hand.len();
    let tw = r.place_on_field(0, "P-190", Some(0));
    r.fire_on_play(0, tw.index as usize);
    assert_eq!(
        r.game.players[0].hand.len(),
        hand_before + 1,
        "P-190 [On Play] must <Draw 1>"
    );
}
