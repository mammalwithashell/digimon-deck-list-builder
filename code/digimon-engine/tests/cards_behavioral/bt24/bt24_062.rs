//! BT24-062 MasterBlimpmon — full behavioral tests.
//!
//! Printed (official Bandai DB bundle + card face):
//!   [Digivolve] Lv.4 w/[Machine]/[Cyborg]/[TS] trait: Cost 3
//!   Assembly -2: [Blimpmon]/Tamer card w/[TS] trait
//!   <Blocker>
//!   <Armor Purge>
//!   [End of Attack] [End of Opponent's Turn] [Once Per Turn] You may play 1
//!     play cost 5 or lower card with the [Machine], [Cyborg] or [TS] trait
//!     from this Digimon's digivolution cards without paying the cost.
//! Inherited:
//!   [Your Turn] This Digimon's attack target can't change.

use digimon_dsl::compiled::{CompiledAltPathKind, CompiledColor};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming, Expiry, GamePhase, Keyword, ModifierType};
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::{SelectionKind, TriggerSource};

fn blocker_card(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(6000);
    c
}

fn defender_card(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(5000);
    c
}

fn host_card(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(6);
    c.dp = Some(11000);
    c
}

fn master_blimpmon_runner() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card("BT24-062")
        .expect("BT24-062 YAML loads")
        .add_card(blocker_card("BLK"))
        .add_card(defender_card("DEF"))
        .add_card(host_card("HOST"))
}

#[test]
fn bt24_062_yaml_compiles_with_correct_printed_face() {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("cards")
        .join("bt24")
        .join("BT24-062.yaml");
    let yaml = std::fs::read_to_string(&path).expect("BT24-062 YAML exists");
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(&yaml).expect("BT24-062 YAML parses");
    let registry =
        digimon_dsl::CardRegistry::from_specs("bt24", &[spec]).expect("BT24-062 YAML compiles");
    let compiled = registry
        .lookup("BT24-062")
        .expect("BT24-062 in registry")
        .clone();

    assert_eq!(compiled.card, "BT24-062");
    assert_eq!(compiled.name, "MasterBlimpmon");
    assert_eq!(compiled.level, Some(5));
    assert_eq!(compiled.cost, Some(7));
    assert_eq!(compiled.dp, Some(7000));
    assert_eq!(
        compiled.color,
        vec![CompiledColor::Black, CompiledColor::Blue]
    );
    for trait_name in ["Machine", "Iliad", "TS"] {
        assert!(
            compiled.traits.iter().any(|t| t == trait_name),
            "missing trait {trait_name}"
        );
    }
    assert!(
        compiled
            .alt_paths
            .iter()
            .any(|p| p.kind == CompiledAltPathKind::Assembly),
        "Assembly -2 alt path must compile"
    );
    assert_eq!(
        compiled.effects.len(),
        4,
        "Blocker + Armor Purge + End of Attack/End of Opponent's Turn + Inherited target lock"
    );
}

#[test]
fn bt24_062_has_blocker_and_armor_purge_on_field() {
    let mut base = make_test_card("BASE-LV4", "BaseLv4");
    base.level = Some(4);
    base.dp = Some(4000);

    let mut r = master_blimpmon_runner()
        .add_card(base)
        .memory(10)
        .start();
    r.skip_mulligan();
    let stack = r.place_stack(0, &["BASE-LV4", "BT24-062"]);
    r.game.tick_declarative_effects();

    assert!(r.game.has_keyword(stack, Keyword::Blocker));
    assert!(r.game.has_keyword(stack, Keyword::ArmorPurge));

    // Armor Purge saves the permanent when deleted by trashing the top card.
    r.game
        .delete_permanent_with_cause(stack, ReplacementCause::OpponentEffect);
    let sel = r
        .pending_selection_view()
        .expect("Armor Purge must offer optional replacement");
    let accept = *sel.valid_action_ids.iter().find(|&&a| a != PASS).unwrap();
    r.execute_action(0, accept).unwrap();
    assert!(r.pending_selection_view().is_none());

    assert_eq!(r.game.players[0].battle_area.len(), 1);
    assert_eq!(
        r.game.players[0].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "BASE-LV4",
        "Armor Purge must trash MasterBlimpmon and leave BASE-LV4 on field"
    );
}

#[test]
fn bt24_062_end_of_attack_plays_qualifying_source_free_and_respects_filters() {
    let mut machine_cost5 = make_test_card("SRC-MACH-5", "Mach5");
    machine_cost5.card_kind = CardKind::Digimon;
    machine_cost5.level = Some(4);
    machine_cost5.play_cost = 5;
    machine_cost5.dp = Some(5000);
    machine_cost5.traits = vec!["Machine".to_string()];

    let mut machine_cost6 = make_test_card("SRC-MACH-6", "Mach6");
    machine_cost6.card_kind = CardKind::Digimon;
    machine_cost6.level = Some(5);
    machine_cost6.play_cost = 6;
    machine_cost6.dp = Some(6000);
    machine_cost6.traits = vec!["Machine".to_string()];

    let mut plain_cost3 = make_test_card("SRC-PLAIN-3", "Plain3");
    plain_cost3.card_kind = CardKind::Digimon;
    plain_cost3.level = Some(3);
    plain_cost3.play_cost = 3;
    plain_cost3.dp = Some(2000);
    plain_cost3.traits = vec!["Beast".to_string()];

    let mut r = master_blimpmon_runner()
        .add_card(machine_cost5)
        .add_card(machine_cost6)
        .add_card(plain_cost3)
        .memory(4)
        .start();
    r.skip_mulligan();

    let perm = r.place_stack(0, &["SRC-PLAIN-3", "SRC-MACH-6", "SRC-MACH-5", "BT24-062"]);
    r.game
        .enqueue_triggered(EffectTiming::EndOfAttack, TriggerSource::Permanent(perm));
    r.game.drain_effect_queue();

    // Accept optional trigger / select qualifying source.
    let sel = r
        .pending_selection_view()
        .expect("[End of Attack] must offer source selection");
    let non_pass: Vec<u16> = sel
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != PASS)
        .collect();
    // If outer optional prompt surfaced first, accept it to enter source selection.
    if sel.kind == SelectionKind::EffectChoice {
        r.execute_action(0, non_pass[0]).unwrap();
    }
    let src_sel = r
        .pending_selection_view()
        .expect("source selection should be pending");
    let src_picks: Vec<u16> = src_sel
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != PASS)
        .collect();
    assert_eq!(
        src_picks.len(),
        1,
        "only SRC-MACH-5 qualifies (cost 6 and Beast trait must be excluded)"
    );
    r.execute_action(0, src_picks[0]).unwrap();
    assert!(r.pending_selection_view().is_none());

    assert_eq!(r.game.memory, 4, "playing from digivolution cards is free");
    assert!(
        r.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&r.game.card_data) == "SRC-MACH-5"),
        "SRC-MACH-5 must be played onto the battle area"
    );
}

#[test]
fn bt24_062_end_of_opponents_turn_plays_ts_tamer_source_free() {
    let mut ts_tamer = make_test_card("SRC-TS-TAMER", "TSTamer");
    ts_tamer.card_kind = CardKind::Tamer;
    ts_tamer.play_cost = 4;
    ts_tamer.traits = vec!["TS".to_string()];

    let mut r = master_blimpmon_runner()
        .add_card(ts_tamer)
        .deck(0, &["DEF", "DEF"])
        .deck(1, &["DEF", "DEF"])
        .memory(3)
        .start();
    r.skip_mulligan();

    let perm = r.place_stack(0, &["SRC-TS-TAMER", "BT24-062"]);
    r.game.pass_turn();
    assert!(!r.game.game_over, "pass_turn must not deck out");
    assert_eq!(r.game.turn_player(), 1);

    r.game
        .enqueue_triggered(EffectTiming::EndOfOpponentsTurn, TriggerSource::Permanent(perm));
    r.game.drain_effect_queue();

    let sel = r
        .pending_selection_view()
        .expect("[End of Opponent's Turn] must fire on opponent's turn end");
    let pick = *sel.valid_action_ids.iter().find(|&&a| a != PASS).unwrap();
    r.execute_action(0, pick).unwrap();
    if let Some(sel2) = r.pending_selection_view() {
        let pick2 = *sel2.valid_action_ids.iter().find(|&&a| a != PASS).unwrap();
        r.execute_action(0, pick2).unwrap();
    }
    assert!(r.pending_selection_view().is_none());

    assert!(
        r.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&r.game.card_data) == "SRC-TS-TAMER"),
        "TS Tamer source must be played onto the battle area at End of Opponent's Turn"
    );
}

#[test]
fn bt24_062_inherited_target_lock_suppresses_block_window_on_controllers_turn() {
    let mut r = master_blimpmon_runner().memory(10).start();
    let atk = r.place_stack(0, &["BT24-062", "HOST"]);
    let _def = r.place_on_field(1, "DEF", Some(0));
    let blk = r.place_on_field(1, "BLK", Some(0));
    r.game
        .modifiers
        .grant_keyword(blk, Keyword::Blocker, Expiry::Permanent, 1);

    r.game.tick_declarative_effects();

    assert!(
        r.game
            .modifiers
            .has(atk, ModifierType::CannotSwitchAttackTarget),
        "inherited declarative tick must install CannotSwitchAttackTarget on host during controller's turn"
    );

    let result = r.attack_digimon(atk, r.perm_handle(1, 0), false);
    assert_ne!(
        result,
        AttackResult::InProgress,
        "Block window must be suppressed → attack resolves synchronously"
    );
    assert_ne!(
        r.current_phase(),
        GamePhase::BlockTiming,
        "BlockTiming must not install when CannotSwitchAttackTarget is set"
    );
    assert!(r.game.pending_selection.is_none());
}

#[test]
fn bt24_062_target_lock_is_inherited_only_and_inactive_on_opponents_turn() {
    let mut r = master_blimpmon_runner().memory(10).start();
    let top_only = r.place_on_field(0, "BT24-062", Some(0));
    let hosted = r.place_stack(0, &["BT24-062", "HOST"]);

    r.game.tick_declarative_effects();
    assert!(
        !r.game
            .modifiers
            .has(top_only, ModifierType::CannotSwitchAttackTarget),
        "target lock is inherited-only and must not apply when BT24-062 is the top card"
    );
    assert!(
        r.game
            .modifiers
            .has(hosted, ModifierType::CannotSwitchAttackTarget),
        "target lock must apply when BT24-062 is a digivolution card"
    );

    r.game.pass_turn();
    assert_eq!(r.game.turn_player(), 1, "expected opponent to be on turn");
    r.game.tick_declarative_effects();

    assert!(
        !r.game
            .modifiers
            .has(hosted, ModifierType::CannotSwitchAttackTarget),
        "inherited target lock must NOT be active on the opponent's turn"
    );
}

#[test]
fn bt24_062_inherited_target_lock_does_not_affect_other_attackers() {
    let mut r = master_blimpmon_runner()
        .add_card({
            let mut c = make_test_card("OTHER", "Other Attacker");
            c.card_kind = CardKind::Digimon;
            c.level = Some(5);
            c.dp = Some(5000);
            c
        })
        .memory(10)
        .start();
    let _atk_locked = r.place_stack(0, &["BT24-062", "HOST"]);
    let other = r.place_on_field(0, "OTHER", Some(0));
    let _def = r.place_on_field(1, "DEF", Some(0));
    let blk = r.place_on_field(1, "BLK", Some(0));
    r.game
        .modifiers
        .grant_keyword(blk, Keyword::Blocker, Expiry::Permanent, 1);

    r.game.tick_declarative_effects();

    let result = r.attack_digimon(other, r.perm_handle(1, 0), false);
    assert_eq!(
        result,
        AttackResult::InProgress,
        "the unlocked attacker must surface BlockTiming"
    );
    assert_eq!(r.current_phase(), GamePhase::BlockTiming);
}
