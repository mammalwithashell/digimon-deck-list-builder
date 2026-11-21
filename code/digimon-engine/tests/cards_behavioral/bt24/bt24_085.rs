//! BT24-085 Dan Yuki & Kanan Yuki.

use std::sync::Arc;

use digimon_dsl::compiled::{CompiledClause, CompiledColor, CompiledStep, CompiledTiming};
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{decode_attack, encode_attack, PASS, PLAY_HAND_START};
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming};
use digimon_engine::selection::{SelectionKind, TriggerSource};
use digimon_engine::{CardEffect, Effect};

const YAML: &str = include_str!("../../../cards/bt24/BT24-085.yaml");

struct OptionMainNoop;

impl CardEffect for OptionMainNoop {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::when_attacking(card)
            .option_main()
            .name("OptionMain no-op")
            .process(|_| {})
            .build()]
    }
}

fn ts_digimon(id: &str) -> digimon_engine::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(5000);
    card.traits = vec!["TS".to_string()];
    card
}

fn ts_option(id: &str, cost: u16) -> digimon_engine::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Option;
    card.play_cost = cost;
    card.traits = vec!["TS".to_string()];
    card
}

fn plain_option(id: &str, cost: u16) -> digimon_engine::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Option;
    card.play_cost = cost;
    card.traits = vec!["Not TS".to_string()];
    card
}

fn enqueue_dan_kanan_eot(runner: &mut DebugRunner, tamer: digimon_engine::PermanentHandle) {
    runner
        .game
        .enqueue_triggered(EffectTiming::EndOfYourTurn, TriggerSource::Permanent(tamer));
    runner.game.drain_effect_queue();
}

#[test]
fn bt24_085_yaml_compiles_with_option_use_and_may_attack_clause() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT24-085 YAML parses")
        .build();

    let compiled = runner
        .compiled_card("BT24-085")
        .expect("BT24-085 compiled card present");

    assert_eq!(compiled.card, "BT24-085");
    assert_eq!(compiled.name, "Dan Yuki & Kanan Yuki");
    assert_eq!(
        compiled.kind,
        digimon_dsl::compiled::CompiledCardKind::Tamer
    );
    assert_eq!(compiled.cost, Some(4));
    assert_eq!(
        compiled.color,
        vec![CompiledColor::Green, CompiledColor::Red]
    );
    assert!(compiled.traits.iter().any(|t| t == "TS"));

    let eot = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::EndOfYourTurn) => {
                Some(t)
            }
            _ => None,
        })
        .expect("EndOfYourTurn clause exists");
    assert!(eot.process.iter().any(|step| {
        matches!(
            step,
            CompiledStep::UseOptionFromHand {
                use_cost_lte_opponent_memory: true,
                ..
            }
        )
    }));
    assert!(eot
        .process
        .iter()
        .any(|step| matches!(step, CompiledStep::MayAttackNow { .. })));
}

#[test]
fn bt24_085_start_of_main_gains_memory_at_four_or_less() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT24-085 YAML parses")
        .add_card(ts_digimon("TS-FILLER"))
        .memory(4)
        .start();

    let tamer = runner.place_on_field(0, "BT24-085", Some(0));
    runner.game.enqueue_triggered(
        EffectTiming::StartOfYourMainPhase,
        TriggerSource::Permanent(tamer),
    );
    runner.game.drain_effect_queue();

    assert_eq!(runner.game.memory, 5);
}

#[test]
fn bt24_085_suspended_tamer_has_no_end_of_turn_activation() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT24-085 YAML parses")
        .add_card(ts_digimon("TS-ATTACKER"))
        .memory(-2)
        .start();
    let tamer = runner.place_on_field(0, "BT24-085", Some(0));
    runner.place_on_field(0, "TS-ATTACKER", Some(0));
    runner.game.players[0].battle_area[tamer.index as usize].is_suspended = true;

    enqueue_dan_kanan_eot(&mut runner, tamer);

    assert!(runner.pending_selection().is_none(), "a suspended Tamer cannot pay the activation cost");
}

#[test]
fn bt24_085_end_of_turn_uses_eligible_ts_option_then_opens_may_attack() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT24-085 YAML parses")
        .add_card(ts_digimon("TS-ATTACKER"))
        .add_card(ts_option("TS-LOW", 2))
        .add_card(ts_option("TS-HIGH", 5))
        .add_card(plain_option("PLAIN-LOW", 1))
        .hand(0, &["TS-LOW", "TS-HIGH", "PLAIN-LOW"])
        .memory(-2)
        .start();
    runner.register_effect("TS-LOW", Arc::new(OptionMainNoop));
    runner.register_effect("TS-HIGH", Arc::new(OptionMainNoop));
    runner.register_effect("PLAIN-LOW", Arc::new(OptionMainNoop));

    // Turn 1 with the attacker played on turn 0: it is not summoning-sick,
    // so it can attack (the pick is `can_attack`-filtered,
    // G-DSL-CAN-ATTACK-PREDICATE).
    runner.game.turn_count = 1;
    let tamer = runner.place_on_field(0, "BT24-085", Some(0));
    let attacker = runner.place_on_field(0, "TS-ATTACKER", Some(0));
    let hand_before = runner.hand_size(0);

    enqueue_dan_kanan_eot(&mut runner, tamer);

    assert!(
        runner.pending_is_optional(),
        "printed end-of-turn 'may' effect must start as a declineable prompt"
    );
    runner
        .accept_optional_trigger()
        .expect("accept BT24-085 end-of-turn effect");

    let option_prompt = runner
        .pending_selection_view()
        .expect("TS Option hand-use prompt");
    assert_eq!(option_prompt.kind, SelectionKind::Hand);
    assert!(
        option_prompt.is_optional,
        "printed 'you may use 1 TS Option' choice must be declineable"
    );
    assert_eq!(
        option_prompt.valid_action_ids,
        vec![PLAY_HAND_START],
        "only the TS Option with use cost at or below opponent memory should be selectable"
    );
    assert_eq!(
        build_action_mask(&runner.game, 0)[PASS as usize],
        1.0,
        "optional Option-use prompt must expose PASS"
    );

    runner
        .execute_action(0, PLAY_HAND_START)
        .expect("use the eligible low-cost TS Option");

    assert!(
        runner.game.players[0].battle_area[tamer.index as usize].is_suspended,
        "accepting the effect pays the printed suspend-this-Tamer activation cost"
    );
    assert_eq!(
        runner.hand_size(0),
        hand_before - 1,
        "the selected Option should leave hand through the normal Option lifecycle"
    );
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "TS-LOW"),
        "the selected Option should be disposed to trash after its no-op OptionMain resolves"
    );

    let attacker_prompt = runner
        .pending_selection_view()
        .expect("follow-up TS Digimon attacker selection");
    assert_eq!(attacker_prompt.kind, SelectionKind::OwnField);
    assert!(
        attacker_prompt.is_optional,
        "printed follow-up 'may attack' choice must be declineable"
    );
    let attacker_action = encode_attack(0, attacker.index as u16);
    assert!(
        attacker_prompt.valid_action_ids.contains(&attacker_action),
        "the own TS Digimon should be selectable for the follow-up may-attack"
    );
    runner
        .execute_action(attacker_prompt.selecting_player, attacker_action)
        .expect("choose the TS Digimon that may attack");

    let target_prompt = runner
        .pending_selection_view()
        .expect("chosen TS Digimon should open an attack-target prompt");
    assert_eq!(target_prompt.kind, SelectionKind::Target);
    assert!(
        target_prompt.is_optional,
        "may_attack_now target prompt should remain declineable before commitment"
    );
    assert!(
        target_prompt
            .valid_action_ids
            .iter()
            .any(|action| decode_attack(*action).0 == attacker.index as u16),
        "attack target choices should be for the selected TS Digimon"
    );
}

#[test]
fn bt24_085_arts_digivolve_waits_for_its_attack_follow_up() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML).unwrap()
        .dsl_card("BT26-029").unwrap()
        .dsl_card("BT26-033").unwrap()
        .hand(0, &["BT26-033", "BT26-033"])
        .deck(0, &["BT26-029"; 6])
        .deck(1, &["BT26-029"; 6])
        .memory(-3)
        .start();
    runner.game.turn_count = 1;
    let tamer = runner.place_on_field(0, "BT24-085", Some(0));
    let base = runner.place_on_field(0, "BT26-029", Some(0));
    enqueue_dan_kanan_eot(&mut runner, tamer);
    runner.accept_optional_trigger().unwrap();
    runner.execute_action(0, PLAY_HAND_START).unwrap();
    let arts = runner.pending_selection_view().expect("Arts Digivolve choice");
    assert!(arts.prompt.contains("Arts Digivolve"));
    runner.execute_action(0, encode_attack(0, base.index as u16)).unwrap();
    let next = runner.pending_selection_view().expect("attack follow-up");
    assert_eq!(next.kind, SelectionKind::OwnField);
    assert!(next.prompt.contains("may attack"), "got {}", next.prompt);
}

/// G-ENGINE-TURN-END-MID-EFFECT: the real End-phase flow. Memory is already on
/// the opponent's side, so `end_turn` fires BT24-085's [End of Your Turn]. The
/// Option it uses finishes its own lifecycle (which calls `check_turn_end`)
/// BEFORE "Then, 1 of your [TS] Digimon may attack"; that must not re-enter
/// `end_turn` and rotate the turn mid-effect. The turn ends only once the
/// whole effect — including the may-attack follow-up — has resolved (6-1-4-1).
#[test]
fn bt24_085_turn_does_not_pass_mid_effect_after_used_option() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT24-085 YAML parses")
        .add_card(ts_digimon("TS-ATTACKER"))
        .add_card(ts_option("TS-LOW", 2))
        .hand(0, &["TS-LOW"])
        .deck(0, &["TS-ATTACKER", "TS-ATTACKER", "TS-ATTACKER"])
        .deck(1, &["TS-ATTACKER", "TS-ATTACKER", "TS-ATTACKER"])
        .memory(-2)
        .start();
    runner.register_effect("TS-LOW", Arc::new(OptionMainNoop));

    runner.game.turn_count = 1;
    let tamer = runner.place_on_field(0, "BT24-085", Some(0));
    let attacker = runner.place_on_field(0, "TS-ATTACKER", Some(0));
    assert_eq!(runner.turn_player(), 0);
    let turn_before = runner.game.turn_count;

    runner.game.end_turn();
    assert!(
        runner.pending_is_optional(),
        "the End phase fires BT24-085's optional [End of Your Turn]"
    );
    runner
        .accept_optional_trigger()
        .expect("accept BT24-085 end-of-turn effect");
    runner
        .execute_action(0, PLAY_HAND_START)
        .expect("use the eligible TS Option");
    assert!(runner.game.players[0].battle_area[tamer.index as usize].is_suspended);

    assert_eq!(
        runner.turn_player(),
        0,
        "the turn must not pass while the effect's may-attack step is still pending"
    );
    assert_eq!(runner.game.turn_count, turn_before, "no turn rotation mid-effect");
    let attacker_prompt = runner
        .pending_selection_view()
        .expect("the may-attack attacker pick runs after the used Option");
    assert_eq!(attacker_prompt.kind, SelectionKind::OwnField);
    assert_eq!(attacker_prompt.selecting_player, 0);
    assert!(
        attacker_prompt
            .valid_action_ids
            .contains(&encode_attack(0, attacker.index as u16)),
        "the TS Digimon is still offered as the attacker (it is still P0's turn)"
    );

    // Declining the follow-up finishes the effect; only now does the End
    // phase complete and the turn pass.
    runner
        .execute_action(0, PASS)
        .expect("decline the optional may-attack");
    assert!(runner.game.pending_selection.is_none());
    assert_eq!(
        runner.turn_player(),
        1,
        "once the effect has fully resolved, the End phase hands the turn to P1"
    );
}

/// Printed [End of Your Turn]: "By suspending this Tamer, you may use 1 [TS]
/// trait Option card ... Then, 1 of your Digimon with the [TS] trait may
/// attack." Nothing in the printed text (or DCGO's `CanActivateCondition`,
/// which only needs the Tamer on the battle area, the suspend cost payable and
/// memory <= 0) requires a [TS] Digimon in the BATTLE AREA to start the effect.
/// The YAML used to gate the whole clause on one, so a [TS] Digimon that was
/// only in the breeding area (or no [TS] Digimon at all) silently dropped the
/// optional Option use. The attack is a separate optional follow-up whose
/// attacker pick is already `can_attack`-filtered.
#[test]
fn bt24_085_end_of_turn_offered_with_ts_digimon_only_in_breeding_area() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT24-085 YAML parses")
        .add_card(ts_digimon("TS-EGG"))
        .add_card(ts_option("TS-LOW", 2))
        .hand(0, &["TS-LOW"])
        .memory(-2)
        .start();
    runner.register_effect("TS-LOW", Arc::new(OptionMainNoop));

    runner.game.turn_count = 1;
    let tamer = runner.place_on_field(0, "BT24-085", Some(0));
    runner.place_in_breeding(0, "TS-EGG");
    assert!(
        runner.game.players[0].battle_area.iter().all(|p| p.top_card().card_id(&runner.game.card_data) == "BT24-085"),
        "precondition: the only battle-area permanent is the Tamer"
    );

    enqueue_dan_kanan_eot(&mut runner, tamer);

    assert!(
        runner.pending_is_optional(),
        "the optional [End of Your Turn] use must be offered although the \
         [TS] Digimon is in the breeding area, not the battle area"
    );
    runner
        .accept_optional_trigger()
        .expect("accept BT24-085 end-of-turn effect");
    let option_prompt = runner
        .pending_selection_view()
        .expect("TS Option hand-use prompt");
    assert_eq!(option_prompt.kind, SelectionKind::Hand);
    runner
        .execute_action(0, PLAY_HAND_START)
        .expect("use the TS Option");

    assert!(runner.game.players[0].battle_area[tamer.index as usize].is_suspended);
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "TS-LOW"),
        "the TS Option was used from hand"
    );
    // The breeding-area Digimon cannot attack, so the may-attack follow-up has
    // no candidate and the effect simply ends.
    let _ = runner.auto_resolve();
    assert!(runner.game.pending_selection.is_none());
}

#[test]
fn bt24_085_end_of_turn_offered_with_no_ts_digimon_anywhere() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT24-085 YAML parses")
        .add_card(ts_option("TS-LOW", 2))
        .hand(0, &["TS-LOW"])
        .memory(-2)
        .start();
    runner.register_effect("TS-LOW", Arc::new(OptionMainNoop));

    runner.game.turn_count = 1;
    let tamer = runner.place_on_field(0, "BT24-085", Some(0));
    enqueue_dan_kanan_eot(&mut runner, tamer);

    assert!(
        runner.pending_is_optional(),
        "using a [TS] Option does not depend on owning a [TS] Digimon"
    );
}

/// The suspend cost is still a real gate: a Tamer that is already suspended
/// cannot pay "By suspending this Tamer".
#[test]
fn bt24_085_end_of_turn_not_offered_when_tamer_already_suspended() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT24-085 YAML parses")
        .add_card(ts_option("TS-LOW", 2))
        .hand(0, &["TS-LOW"])
        .memory(-2)
        .start();
    runner.register_effect("TS-LOW", Arc::new(OptionMainNoop));

    runner.game.turn_count = 1;
    let tamer = runner.place_on_field(0, "BT24-085", Some(0));
    runner.game.players[0].battle_area[tamer.index as usize].is_suspended = true;
    enqueue_dan_kanan_eot(&mut runner, tamer);

    assert!(runner.game.pending_selection.is_none());
}
