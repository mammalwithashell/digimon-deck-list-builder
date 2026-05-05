//! EX7-027 Chaperomon.
//! Printed text covered here: <Overclock (Puppet Trait)> and [When Digivolving]
//! you may play 1 level 3 Puppet Digimon card from hand without paying cost.
//!
//! Partial: inherited leave-prevention by deleting Token/Puppet is tracked as a
//! reusable replacement/cost follow-up.

use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{EffectTiming, Keyword};
use digimon_engine::selection::{SelectionKind, TriggerSource};

#[test]
fn ex7_027_has_overclock_while_face_up() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX7-027")
        .expect("EX7-027 YAML loads")
        .start();
    let chapero = runner.place_on_field(0, "EX7-027", Some(0));

    assert!(runner.game.has_keyword(chapero, Keyword::Overclock));
}

#[test]
fn ex7_027_when_digivolving_may_play_level3_puppet_from_hand() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX7-027")
        .expect("EX7-027 YAML loads")
        .add_card(make_test_card("BASE", "Base"))
        .add_card(make_puppet_level3("PUPPET-L3"))
        .hand(0, &["PUPPET-L3"])
        .memory(10)
        .start();
    let chapero = runner.place_stack(0, &["BASE", "EX7-027"]);

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(chapero),
    );
    runner.game.drain_effect_queue();

    let view = runner
        .pending_selection_view()
        .expect("level 3 Puppet prompt");
    assert_eq!(view.kind, SelectionKind::Hand);
    assert!(
        runner.pending_is_optional(),
        "play-from-hand selection is optional"
    );
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("select Puppet");
    runner.auto_resolve().expect("finish play");

    assert!(runner.game.players[0]
        .battle_area
        .iter()
        .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "PUPPET-L3"));
}

fn make_puppet_level3(id: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.level = Some(3);
    card.traits = vec!["Puppet".to_string()];
    card
}
