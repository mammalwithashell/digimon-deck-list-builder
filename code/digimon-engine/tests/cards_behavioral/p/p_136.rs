//! P-136 Arisa Kinosaki — Tamer, Yellow, Cost 4, LIBERATOR.
//!
//! Printed text from `data/cards.json`:
//!
//! [On Play] You may play 1 [Shoemon] from your hand without paying the cost.
//!
//! [Your Turn] [Once Per Turn] When one of your Digimon digivolves into a
//! Digimon with the [Puppet] trait, by suspending this Tamer, gain 1 memory.
//!
//! Security Effect [Security] Play this card without paying the cost.
//!
//! Implemented YAML slices:
//! - [On Play] optional exact [Shoemon] hand play.
//! - [Your Turn][OPT] Puppet-digivolve observer with source-bound suspend cost
//!   via Track B's `activation_cost: { suspend_self: true }`.
//! - [Security] play this Tamer.

use digimon_dsl::compiled::{CompiledCardKind, CompiledClause, CompiledColor, CompiledTiming};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

fn shoemon(id: &str) -> CardData {
    let mut card = make_test_card(id, "Shoemon");
    card.card_kind = CardKind::Digimon;
    card.level = Some(3);
    card.dp = Some(1000);
    card.play_cost = 3;
    card.colors = vec![CardColor::Yellow];
    card.traits = vec!["Puppet".to_string(), "LIBERATOR".to_string()];
    card
}

fn non_shoemon(id: &str) -> CardData {
    let mut card = make_test_card(id, "ShoeShoemon");
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(4000);
    card.play_cost = 5;
    card.colors = vec![CardColor::Yellow];
    card.traits = vec!["Puppet".to_string(), "LIBERATOR".to_string()];
    card
}

fn attacker(id: &str) -> CardData {
    let mut card = make_test_card(id, "Attacker");
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(9000);
    card.play_cost = 4;
    card.colors = vec![CardColor::Red];
    card
}

fn arisa_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("P-136")
        .expect("P-136 YAML loads")
        .memory(10)
        .start()
}

#[test]
fn p_136_has_printed_metadata_and_supported_clauses() {
    let runner = arisa_runner();
    let compiled = runner.compiled_card("P-136").expect("P-136 must compile");

    assert_eq!(compiled.name, "Arisa Kinosaki");
    assert_eq!(compiled.kind, CompiledCardKind::Tamer);
    assert_eq!(compiled.cost, Some(4));
    assert_eq!(compiled.color, vec![CompiledColor::Yellow]);
    assert_eq!(compiled.traits, vec!["LIBERATOR".to_string()]);

    let triggered: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|clause| match clause {
            CompiledClause::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .collect();

    assert!(
        triggered
            .iter()
            .any(|clause| clause.when.contains(&CompiledTiming::OnPlay)),
        "supported YAML must include the printed On Play clause"
    );
    assert!(
        triggered
            .iter()
            .any(|clause| clause.when.contains(&CompiledTiming::OnSecurity)),
        "supported YAML must include the printed Security clause"
    );
    assert!(
        triggered
            .iter()
            .any(|clause| clause.when.contains(&CompiledTiming::OnDigivolve)),
        "Puppet-digivolve observer is authored via activation_cost: suspend_self"
    );
}

#[test]
fn p_136_on_play_clause_is_optional() {
    let runner = arisa_runner();
    let compiled = runner.compiled_card("P-136").expect("P-136 must compile");
    let on_play = compiled
        .effects
        .iter()
        .filter_map(|clause| match clause {
            CompiledClause::Triggered(triggered)
                if triggered.when.contains(&CompiledTiming::OnPlay) =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .next()
        .expect("On Play clause must exist");

    assert!(
        on_play.optional,
        "On Play clause must be optional: 'You may'"
    );
}

#[test]
fn p_136_on_play_offers_only_exact_shoemon_from_hand() {
    let mut runner = DebugRunner::builder()
        .dsl_card("P-136")
        .expect("P-136 YAML loads")
        .add_card(shoemon("SHOEMON-OK"))
        .add_card(non_shoemon("NOT-SHOEMON"))
        .hand(0, &["P-136", "SHOEMON-OK", "NOT-SHOEMON"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Arisa");

    assert!(
        runner.pending_selection().is_some(),
        "playing P-136 should offer its optional Shoemon hand-play prompt"
    );
    assert!(
        runner.pending_is_optional(),
        "Shoemon hand-play prompt must allow PASS"
    );
    assert_eq!(
        runner.pending_action_count(),
        1,
        "only exact-name Shoemon should be selectable; ShoeShoemon must be filtered out"
    );
}

#[test]
fn p_136_on_play_can_decline_without_playing_from_hand() {
    let mut runner = DebugRunner::builder()
        .dsl_card("P-136")
        .expect("P-136 YAML loads")
        .add_card(shoemon("SHOEMON-OK"))
        .hand(0, &["P-136", "SHOEMON-OK"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Arisa");
    let hand_before = runner.hand_size(0);
    let field_before = runner.battle_area_size(0);

    runner
        .execute_action(0, PASS)
        .expect("decline optional Shoemon play");
    runner.auto_resolve().expect("finish declined On Play");

    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "declining must leave Shoemon in hand"
    );
    assert_eq!(
        runner.battle_area_size(0),
        field_before,
        "declining must not play another permanent"
    );
}

#[test]
fn p_136_on_play_plays_shoemon_for_free() {
    let mut runner = DebugRunner::builder()
        .dsl_card("P-136")
        .expect("P-136 YAML loads")
        .add_card(shoemon("SHOEMON-OK"))
        .hand(0, &["P-136", "SHOEMON-OK"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Arisa");
    let memory_after_tamer_cost = runner.memory();
    let field_before = runner.battle_area_size(0);

    let action = runner
        .pending_selection_view()
        .expect("Shoemon selection must be pending")
        .valid_action_ids[0];
    runner
        .execute_action(0, action)
        .expect("select Shoemon from hand");
    runner.auto_resolve().expect("resolve Shoemon play");

    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "SHOEMON-OK"),
        "selected Shoemon should be played to the field"
    );
    assert_eq!(
        runner.battle_area_size(0),
        field_before + 1,
        "Shoemon free play should add exactly one permanent"
    );
    assert_eq!(
        runner.memory(),
        memory_after_tamer_cost,
        "play_from_hand_free must not spend Shoemon's play cost"
    );
}

#[test]
fn p_136_on_play_does_not_prompt_without_shoemon_in_hand() {
    let mut runner = DebugRunner::builder()
        .dsl_card("P-136")
        .expect("P-136 YAML loads")
        .add_card(non_shoemon("NOT-SHOEMON"))
        .hand(0, &["P-136", "NOT-SHOEMON"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Arisa");

    assert!(
        runner.pending_selection().is_none(),
        "On Play must not prompt when no exact Shoemon card is in hand"
    );
}

#[test]
fn p_136_security_plays_itself_without_paying_cost() {
    let mut runner = DebugRunner::builder()
        .dsl_card("P-136")
        .expect("P-136 YAML loads")
        .add_card(attacker("ATTACKER"))
        .security(1, &["P-136"])
        .memory(10)
        .start();
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));

    runner.attack_player(attacker, 1, false);
    runner.auto_resolve().expect("resolve security play");

    assert!(
        runner.game.players[1]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "P-136"),
        "P-136 should be played from the defender's security"
    );
    assert_eq!(
        runner.security_count(1),
        0,
        "security card should be consumed by the security check"
    );
}

fn puppet_top(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(5000);
    card.play_cost = 5;
    card.colors = vec![CardColor::Yellow];
    card.traits = vec!["Puppet".to_string()];
    card
}

fn non_puppet_top(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(5000);
    card.play_cost = 5;
    card.colors = vec![CardColor::Yellow];
    card.traits = vec![];
    card
}

fn non_puppet_base(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(3);
    card.dp = Some(3000);
    card.play_cost = 3;
    card.colors = vec![CardColor::Yellow];
    card.traits = vec![];
    card
}

fn fire_on_digivolve(
    runner: &mut DebugRunner,
    permanent: digimon_engine::permanent::PermanentHandle,
) {
    let event_card = runner.top_card(permanent);
    runner.game.enqueue_triggered(
        digimon_engine::enums::EffectTiming::OnDigivolve,
        digimon_engine::selection::TriggerSource::Digivolved {
            player: permanent.player,
            permanent,
            card: event_card,
            effect_initiated: false,
            dna_origin: false,
        },
    );
    runner.game.drain_effect_queue();
}

#[test]
fn p_136_your_turn_observer_suspends_this_tamer_and_gains_memory_on_puppet_digivolve() {
    let mut runner = DebugRunner::builder()
        .dsl_card("P-136")
        .expect("P-136 YAML loads")
        .add_card(non_puppet_base("BASE-OWN"))
        .add_card(puppet_top("PUPPET-EVO"))
        .memory(0)
        .start();
    let arisa = runner.place_on_field(0, "P-136", Some(0));
    let stack = runner.place_stack(0, &["BASE-OWN", "PUPPET-EVO"]);

    let memory_before = runner.memory();
    fire_on_digivolve(&mut runner, stack);
    runner.auto_resolve().expect("resolve observer");

    assert!(
        runner.game.player(0).battle_area[arisa.index as usize].is_suspended,
        "Arisa suspends as activation cost"
    );
    assert_eq!(
        runner.memory(),
        memory_before + 1,
        "+1 memory after paying the suspend-self cost"
    );
}

#[test]
fn p_136_your_turn_observer_does_not_gain_memory_when_this_tamer_is_already_suspended() {
    let mut runner = DebugRunner::builder()
        .dsl_card("P-136")
        .expect("P-136 YAML loads")
        .add_card(non_puppet_base("BASE-OWN"))
        .add_card(puppet_top("PUPPET-EVO"))
        .memory(0)
        .start();
    let arisa = runner.place_on_field(0, "P-136", Some(0));
    runner.game.player_mut(0).battle_area[arisa.index as usize].is_suspended = true;
    let stack = runner.place_stack(0, &["BASE-OWN", "PUPPET-EVO"]);

    let memory_before = runner.memory();
    fire_on_digivolve(&mut runner, stack);
    runner.auto_resolve().expect("resolve observer");

    assert!(
        runner.game.player(0).battle_area[arisa.index as usize].is_suspended,
        "already-suspended Arisa stays suspended"
    );
    assert_eq!(
        runner.memory(),
        memory_before,
        "cost cannot be paid; +1 memory must not fire"
    );
}

#[test]
fn p_136_your_turn_observer_does_not_trigger_when_evo_is_non_puppet() {
    let mut runner = DebugRunner::builder()
        .dsl_card("P-136")
        .expect("P-136 YAML loads")
        .add_card(non_puppet_base("BASE-OWN"))
        .add_card(non_puppet_top("NON-PUPPET-EVO"))
        .memory(0)
        .start();
    let arisa = runner.place_on_field(0, "P-136", Some(0));
    let stack = runner.place_stack(0, &["BASE-OWN", "NON-PUPPET-EVO"]);

    let memory_before = runner.memory();
    fire_on_digivolve(&mut runner, stack);
    runner.auto_resolve().expect("resolve observer");

    assert!(
        !runner.game.player(0).battle_area[arisa.index as usize].is_suspended,
        "Arisa should not suspend for non-Puppet digivolve"
    );
    assert_eq!(
        runner.memory(),
        memory_before,
        "non-Puppet digivolve must not gain memory"
    );
}

#[test]
fn p_136_your_turn_observer_does_not_trigger_when_opponent_digivolves_into_puppet() {
    let mut runner = DebugRunner::builder()
        .dsl_card("P-136")
        .expect("P-136 YAML loads")
        .add_card(non_puppet_base("BASE-OPP"))
        .add_card(puppet_top("PUPPET-EVO-OPP"))
        .memory(0)
        .start();
    let arisa = runner.place_on_field(0, "P-136", Some(0));
    let opp_stack = runner.place_stack(1, &["BASE-OPP", "PUPPET-EVO-OPP"]);

    let memory_before = runner.memory();
    fire_on_digivolve(&mut runner, opp_stack);
    runner.auto_resolve().expect("resolve observer");

    assert!(
        !runner.game.player(0).battle_area[arisa.index as usize].is_suspended,
        "Arisa should not suspend for opponent's Puppet digivolve"
    );
    assert_eq!(
        runner.memory(),
        memory_before,
        "opponent digivolve must not gain memory"
    );
}
