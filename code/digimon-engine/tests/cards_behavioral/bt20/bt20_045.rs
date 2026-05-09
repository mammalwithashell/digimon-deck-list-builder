//! BT20-045 Examon

use digimon_dsl::compiled::{CompiledAltPathKind, CompiledCost};
use digimon_engine::action::space::{encode_digivolve, PLAY_HAND_START};
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card_with_level, DebugRunner};
use digimon_engine::enums::CardColor;

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
    let mut breakdramon = make_test_card_with_level("BREAKDRAMON-MAT", "Breakdramon", 6);
    breakdramon.colors = vec![CardColor::Green];
    breakdramon.dp = Some(10000);
    let mut slayerdramon = make_test_card_with_level("SLAYERDRAMON-MAT", "Slayerdramon", 6);
    slayerdramon.colors = vec![CardColor::Blue];
    slayerdramon.dp = Some(10000);
    let mut attacker = make_test_card_with_level("ATTACKER", "Attacker", 4);
    attacker.dp = Some(4000);

    let mut runner = DebugRunner::builder()
        .dsl_card("BT20-045")
        .expect("BT20-045 YAML loads")
        .add_card(breakdramon)
        .add_card(slayerdramon)
        .add_card(attacker)
        .hand(1, &["BT20-045", "SLAYERDRAMON-MAT"])
        .memory(0)
        .start();

    let atk = runner.place_on_field(0, "ATTACKER", Some(0));
    let target = runner.place_on_field(1, "BREAKDRAMON-MAT", Some(0));
    assert_eq!(runner.attack_digimon(atk, target, false), AttackResult::InProgress);

    let field_pick = encode_digivolve(0, 0);
    let pending = runner
        .pending_selection()
        .expect("Counter window should offer BT20-045 Blast DNA field material");
    assert!(pending.valid_action_ids.contains(&field_pick));
    let selecting_player = pending.selecting_player;
    runner
        .execute_action(selecting_player, field_pick)
        .expect("select field material");

    let pending = runner
        .pending_selection()
        .expect("Blast DNA should ask for Slayerdramon from hand");
    assert_eq!(pending.valid_action_ids, vec![PLAY_HAND_START + 1]);
    let selecting_player = pending.selecting_player;
    runner
        .execute_action(selecting_player, PLAY_HAND_START + 1)
        .expect("select hand material");

    let stack_ids: Vec<_> = runner.game.player(1).battle_area[0]
        .card_sources
        .iter()
        .map(|card| card.card_id(&runner.game.card_data).to_string())
        .collect();
    assert_eq!(
        stack_ids,
        vec!["BREAKDRAMON-MAT", "SLAYERDRAMON-MAT", "BT20-045"]
    );
}

#[ignore = "pending: G-HIGHEST-DP-SWEEP and G-SUSPEND-OBSERVER-UNSUSPEND — DNA-gated highest-DP bottom-deck and suspend observer unsuspend"]
#[test]
fn bt20_045_highest_dp_sweep_and_suspend_observer() {}
