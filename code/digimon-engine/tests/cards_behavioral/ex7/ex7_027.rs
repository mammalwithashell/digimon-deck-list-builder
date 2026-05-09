//! EX7-027 Chaperomon.
//! Printed text covered here: <Overclock (Puppet Trait)>, [When Digivolving]
//! play 1 level 3 Puppet Digimon card from hand without paying cost, and the
//! inherited Token/other-Puppet leave-prevention replacement.

use digimon_engine::action::space::encode_attack;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{EffectTiming, Keyword};
use digimon_engine::replacement::ReplacementCause;
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

#[test]
fn ex7_027_inherited_prevents_opponent_effect_leave_by_deleting_token_or_other_puppet() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX7-027")
        .expect("EX7-027 YAML loads")
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(make_puppet_level3("PUPPET-COST"))
        .add_card(make_token("TOKEN-COST"))
        .start();
    let carrier = runner.place_stack(0, &["EX7-027", "CARRIER"]);
    let puppet = runner.place_on_field(0, "PUPPET-COST", Some(0));
    let token = runner.place_on_field(0, "TOKEN-COST", Some(0));

    runner
        .game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);

    let accept = runner
        .pending_selection_view()
        .expect("inherited replacement accept prompt");
    assert_eq!(accept.kind, SelectionKind::Replacement);
    assert!(accept.is_optional);
    runner
        .execute_action(0, accept.valid_action_ids[0])
        .expect("accept inherited replacement");

    let view = runner
        .pending_selection_view()
        .expect("Token/other-Puppet cost prompt");
    assert_eq!(view.kind, SelectionKind::OwnField);
    assert!(view.valid_action_ids.contains(&encode_permanent(puppet)));
    assert!(view.valid_action_ids.contains(&encode_permanent(token)));
    runner
        .execute_action(0, encode_permanent(token))
        .expect("delete token cost");
    runner.auto_resolve().expect("finish replacement");

    assert!(permanent_exists(&runner, 0, "CARRIER"));
    assert!(!permanent_exists(&runner, 0, "TOKEN-COST"));
}

#[test]
fn ex7_027_inherited_does_not_prevent_own_effect_leave() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX7-027")
        .expect("EX7-027 YAML loads")
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(make_puppet_level3("PUPPET-COST"))
        .start();
    let carrier = runner.place_stack(0, &["EX7-027", "CARRIER"]);
    runner.place_on_field(0, "PUPPET-COST", Some(0));

    runner
        .game
        .delete_permanent_with_cause(carrier, ReplacementCause::OwnEffect);

    assert!(
        runner.pending_selection_view().is_none(),
        "own effects must not offer the inherited prevention prompt"
    );
    assert!(!permanent_exists(&runner, 0, "CARRIER"));
    assert!(permanent_exists(&runner, 0, "PUPPET-COST"));
}

fn make_puppet_level3(id: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.level = Some(3);
    card.traits = vec!["Puppet".to_string()];
    card
}

fn make_token(id: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = digimon_engine::enums::CardKind::Token;
    card.traits = vec!["Token".to_string()];
    card
}

fn encode_permanent(handle: digimon_engine::permanent::PermanentHandle) -> u16 {
    encode_attack(handle.player as u16, handle.index as u16)
}

fn permanent_exists(runner: &DebugRunner, player: usize, card_id: &str) -> bool {
    runner.game.players[player]
        .battle_area
        .iter()
        .any(|permanent| permanent.top_card().card_id(&runner.game.card_data) == card_id)
}
