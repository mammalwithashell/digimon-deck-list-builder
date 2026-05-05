//! ST19-12 Cendrillmon.
//! Printed text covered here: <Overclock (Puppet Trait)>, <Blocker>, and
//! [When Digivolving] you may play 2 Familiar Tokens.

use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{EffectTiming, Keyword};
use digimon_engine::selection::TriggerSource;

#[test]
fn st19_12_has_overclock_and_blocker_while_face_up() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST19-12")
        .expect("ST19-12 YAML loads")
        .start();
    let cendrill = runner.place_on_field(0, "ST19-12", Some(0));

    assert!(runner.game.has_keyword(cendrill, Keyword::Overclock));
    assert!(runner.game.has_keyword(cendrill, Keyword::Blocker));
}

#[test]
fn st19_12_when_digivolving_may_play_two_familiar_tokens() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST19-12")
        .expect("ST19-12 YAML loads")
        .add_card(make_test_card("BASE", "Base"))
        .start();
    let cendrill = runner.place_stack(0, &["BASE", "ST19-12"]);

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(cendrill),
    );
    runner.game.drain_effect_queue();
    runner.auto_resolve().expect("accept optional token play");

    let familiar_count = runner.game.players[0]
        .battle_area
        .iter()
        .filter(|perm| perm.top_card().card_name(&runner.game.card_data) == "Familiar Token")
        .count();
    assert_eq!(familiar_count, 2, "two Familiar Tokens are played");
}
