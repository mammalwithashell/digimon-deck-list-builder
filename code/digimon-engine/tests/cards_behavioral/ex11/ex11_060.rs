//! EX11-060 Arisa Kinosaki.
//!
//! Printed text covered here:
//! - [Start of Your Turn] If you have 2 or less memory, set it to 3.
//! - [Security] Play this card without paying the cost.
//!
//! Partial: the all-turns Token/Puppet deletion observer remains blocked until
//! deletion triggers carry deleted-object context, Overclock cause context, and
//! the suspend-this-Tamer cost plus optional play branch can be surfaced through
//! current action/pending-selection contracts.

use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, GamePhase, Keyword};
use digimon_engine::{
    action::space::{
        encode_attack, EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_OVERCLOCK, FIELD_EFFECT_START,
    },
    card_data::CardData,
    permanent::PermanentHandle,
    replacement::ReplacementCause,
    selection::SelectionKind,
};

#[test]
fn ex11_060_start_of_turn_sets_memory_to_3_when_lte_2() {
    let filler = make_test_card("FILLER-EX11-060", "Filler");
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-060")
        .expect("EX11-060 YAML loads")
        .add_card(filler)
        .deck(0, &["FILLER-EX11-060"])
        .deck(1, &["FILLER-EX11-060"])
        .memory(2)
        .start();

    runner.place_on_field(0, "EX11-060", Some(0));
    runner.game.memory = 2;

    runner.end_turn();
    runner.end_turn();

    assert_eq!(
        runner.memory(),
        3,
        "Arisa sets memory to 3 at start of your turn when memory is 2 or less"
    );
}

#[test]
fn ex11_060_start_of_turn_does_not_lower_memory_above_2() {
    let filler = make_test_card("FILLER-EX11-060", "Filler");
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-060")
        .expect("EX11-060 YAML loads")
        .add_card(filler)
        .deck(0, &["FILLER-EX11-060"])
        .deck(1, &["FILLER-EX11-060"])
        .memory(5)
        .start();

    runner.place_on_field(0, "EX11-060", Some(0));
    runner.game.memory = 5;

    runner.end_turn();
    runner.end_turn();

    assert_eq!(
        runner.memory(),
        5,
        "Arisa must not set memory to 3 when memory is above the printed threshold"
    );
}

#[test]
fn ex11_060_security_plays_itself_without_paying_cost() {
    let mut attacker = make_test_card("ATTACKER-EX11-060", "Attacker");
    attacker.card_kind = CardKind::Digimon;
    attacker.level = Some(4);
    attacker.dp = Some(9000);
    attacker.play_cost = 0;

    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-060")
        .expect("EX11-060 YAML loads")
        .add_card(attacker)
        .security(1, &["EX11-060"])
        .memory(10)
        .start();
    let attacker = runner.place_on_field(0, "ATTACKER-EX11-060", Some(0));

    runner.attack_player(attacker, 1, false);
    runner.auto_resolve().expect("resolve security play");

    assert!(runner.game.players[1]
        .battle_area
        .iter()
        .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "EX11-060"));
}

#[test]
fn ex11_060_all_turns_draws_when_own_puppet_is_deleted_by_non_overclock() {
    let mut runner = arisa_deletion_runner();
    let arisa = runner.place_on_field(0, "EX11-060", Some(0));
    let puppet = runner.place_on_field(0, "PUPPET-SAC", Some(0));

    runner
        .game
        .delete_permanent_with_cause(puppet, ReplacementCause::OwnEffect);

    choose_arisa_activation(&mut runner, "ordinary deletion offers Arisa activation");
    runner
        .auto_resolve()
        .expect("finish non-Overclock Arisa branch");

    let arisa_perm = &runner.game.player(0).battle_area[arisa.index as usize];
    assert!(
        arisa_perm.is_suspended,
        "Arisa's visible cost should suspend this Tamer"
    );
    assert!(
        runner
            .game
            .player(0)
            .hand
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "DRAW-FILLER-EX11-060"),
        "Arisa should Draw 1 after the suspend cost is paid"
    );
    assert!(
        !runner
            .game
            .player(0)
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "PUPPET-HAND"),
        "ordinary deletion must not expose the Overclock-only hand-play branch"
    );
    assert!(
        runner.pending_selection().is_none(),
        "ordinary deletion should resolve after suspend and draw"
    );
}

#[test]
fn ex11_060_all_turns_overclock_deletion_may_play_level_4_or_lower_puppet() {
    let mut runner = arisa_deletion_runner();
    let _arisa = runner.place_on_field(0, "EX11-060", Some(0));
    let overclock = runner.place_on_field(0, "PUPPET-OVERCLOCK", Some(0));
    let sacrifice = runner.place_on_field(0, "PUPPET-SAC", Some(0));
    runner.game.current_phase = GamePhase::EndOfTurnAction;

    let overclock_action = encode_field_effect(overclock, FIELD_EFFECT_SLOT_FOR_OVERCLOCK);
    runner.game.decode_action(overclock_action, 0);
    runner
        .game
        .resolve_selection(0, encode_attack(0, sacrifice.index as u16))
        .expect("choose Puppet sacrifice for Overclock");

    choose_arisa_activation(&mut runner, "Overclock deletion offers Arisa activation");

    let hand_pick = runner
        .pending_selection_view()
        .expect("Overclock branch should offer optional Puppet hand-play");
    assert_eq!(hand_pick.kind, SelectionKind::Hand);
    assert!(
        hand_pick
            .valid_action_ids
            .iter()
            .any(|&action| action != digimon_engine::action::space::PASS),
        "hand-play branch should expose the level 4 Puppet candidate"
    );
    let play_action = hand_pick
        .valid_action_ids
        .iter()
        .copied()
        .find(|&action| action != digimon_engine::action::space::PASS)
        .expect("Puppet hand card action");
    runner
        .execute_action(hand_pick.selecting_player, play_action)
        .expect("choose level 4 Puppet from hand");
    runner
        .auto_resolve()
        .expect("finish Overclock Arisa branch and resumed attack");

    let arisa_perm = runner
        .game
        .player(0)
        .battle_area
        .iter()
        .find(|perm| perm.top_card().card_id(&runner.game.card_data) == "EX11-060")
        .expect("Arisa remains on field");
    assert!(
        arisa_perm.is_suspended,
        "Arisa's visible cost should suspend this Tamer"
    );
    assert!(
        runner
            .game
            .player(0)
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "PUPPET-HAND"),
        "Overclock branch should play the selected level 4 Puppet from hand"
    );
}

fn arisa_deletion_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX11-060")
        .expect("EX11-060 YAML loads")
        .add_card(puppet_digimon("PUPPET-SAC", 4, 4000))
        .add_card(puppet_digimon("PUPPET-HAND", 4, 4000))
        .add_card(overclock_puppet("PUPPET-OVERCLOCK"))
        .add_card(make_test_card("DRAW-FILLER-EX11-060", "Draw Filler"))
        .hand(0, &["PUPPET-HAND"])
        .deck(0, &["DRAW-FILLER-EX11-060"])
        .security(1, &["DRAW-FILLER-EX11-060"])
        .start()
}

fn puppet_digimon(id: &str, level: u8, dp: i32) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Yellow];
    card.level = Some(level);
    card.dp = Some(dp);
    card.play_cost = 4;
    card.traits = vec!["Puppet".to_string()];
    card
}

fn overclock_puppet(id: &str) -> CardData {
    let mut card = puppet_digimon(id, 4, 7000);
    card.keywords = vec![Keyword::Overclock];
    card.effect_text =
        "＜Overclock ([Puppet] Trait)＞ (At the end of your turn, by deleting 1 of your Tokens or other [Puppet] trait Digimon, this Digimon attacks a player without suspending.)"
            .to_string();
    card
}

fn encode_field_effect(handle: PermanentHandle, effect_slot: u16) -> u16 {
    FIELD_EFFECT_START + handle.index as u16 * EFFECTS_PER_PERMANENT + effect_slot
}

fn choose_arisa_activation(runner: &mut DebugRunner, label: &str) {
    let view = runner.pending_selection_view().expect(label);
    assert_eq!(view.kind, SelectionKind::EffectChoice);
    let action = view.valid_action_ids[0];
    runner
        .execute_action(view.selecting_player, action)
        .expect("accept Arisa activation");
}
