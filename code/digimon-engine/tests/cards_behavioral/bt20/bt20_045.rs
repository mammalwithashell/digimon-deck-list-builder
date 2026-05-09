//! BT20-045 Examon

use digimon_dsl::compiled::{CompiledAltPathKind, CompiledCost};
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{DNA_DIGIVOLVE_START, PLAY_HAND_START};
use digimon_engine::debug_runner::{make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardColor, GamePhase};

#[test]
fn bt20_045_loads_keyword_slice() {
    DebugRunner::builder()
        .dsl_card("BT20-045")
        .expect("BT20-045 must load from embedded DSL pack")
        .start();
}

#[test]
fn bt20_045_has_blast_dna_digivolve_path() {
    let runner = DebugRunner::builder()
        .dsl_card("BT20-045")
        .expect("BT20-045 must load from embedded DSL pack")
        .start();
    let card = runner.compiled_card("BT20-045").expect("compiled card");

    assert!(card.alt_paths.iter().any(|path| {
        path.kind == CompiledAltPathKind::BlastDnaDigivolve
            && path.cost == Some(CompiledCost::Literal(0))
            && path
                .materials
                .iter()
                .any(|mat| mat.filter.name_is.as_deref() == Some("Breakdramon"))
            && path
                .materials
                .iter()
                .any(|mat| mat.filter.name_is.as_deref() == Some("Slayerdramon"))
    }));
}

#[test]
fn bt20_045_counter_blast_dna_uses_breakdramon_and_slayerdramon() {
    let mut breakdramon = make_test_card_with_level("TEST-BREAKDRAMON", "Breakdramon", 6);
    breakdramon.colors = vec![CardColor::Green, CardColor::Red];
    breakdramon.dp = Some(12000);

    let mut slayerdramon = make_test_card_with_level("TEST-SLAYERDRAMON", "Slayerdramon", 6);
    slayerdramon.colors = vec![CardColor::Blue];
    slayerdramon.dp = Some(12000);

    let mut attacker = make_test_card_with_level("TEST-ATTACKER", "Attacker", 6);
    attacker.colors = vec![CardColor::Red];
    attacker.dp = Some(17000);

    let mut runner = DebugRunner::builder()
        .dsl_card("BT20-045")
        .expect("BT20-045 YAML loads")
        .add_card(breakdramon)
        .add_card(slayerdramon)
        .add_card(attacker)
        .hand(1, &["BT20-045", "TEST-SLAYERDRAMON"])
        .start();

    let attacking = runner.place_on_field(0, "TEST-ATTACKER", Some(0));
    let breakdramon = runner.place_on_field(1, "TEST-BREAKDRAMON", Some(0));

    let result = runner.attack_digimon(attacking, breakdramon, false);
    assert_eq!(result, digimon_engine::combat::AttackResult::InProgress);
    assert_eq!(runner.current_phase(), GamePhase::CounterTiming);

    let counter_prompt = runner
        .pending_selection()
        .expect("Counter window should offer Examon Blast DNA");
    assert!(
        counter_prompt
            .valid_action_ids
            .contains(&DNA_DIGIVOLVE_START),
        "BT20-045 in hand slot 0 should be a Counter Blast DNA candidate: {:?}",
        counter_prompt.valid_action_ids
    );
    let mask = build_action_mask(&runner.game, 1);
    assert_eq!(mask[DNA_DIGIVOLVE_START as usize], 1.0);

    runner
        .execute_action(1, DNA_DIGIVOLVE_START)
        .expect("choose BT20-045 for Counter Blast DNA");
    assert_eq!(runner.current_phase(), GamePhase::SelectMaterial);
    assert_eq!(
        runner
            .pending_selection()
            .expect("field material prompt")
            .valid_action_ids,
        vec![0]
    );

    runner
        .execute_action(1, 0)
        .expect("choose Breakdramon as the field material");
    assert_eq!(
        runner
            .pending_selection()
            .expect("hand material prompt")
            .valid_action_ids,
        vec![PLAY_HAND_START + 1]
    );

    runner
        .execute_action(1, PLAY_HAND_START + 1)
        .expect("choose Slayerdramon as the hand material");

    let evolved = &runner.game.players[1].battle_area[0];
    assert_eq!(
        evolved.top_card().card_id(&runner.game.card_data),
        "BT20-045"
    );
    assert!(evolved
        .card_sources
        .iter()
        .any(|card| card.card_id(&runner.game.card_data) == "TEST-SLAYERDRAMON"));
    assert_eq!(runner.hand_size(1), 0);
}

#[ignore = "pending: G-HIGHEST-DP-SWEEP and G-SUSPEND-OBSERVER-UNSUSPEND — DNA-gated highest-DP bottom-deck and suspend observer unsuspend"]
#[test]
fn bt20_045_highest_dp_sweep_and_suspend_observer() {}
