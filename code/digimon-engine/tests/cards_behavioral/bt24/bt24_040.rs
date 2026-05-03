//! BT24-040 Venusmon.
//! DCGO reference: DCGO/Assets/Scripts/CardEffect/BT24/Yellow/BT24_040.cs
//! Covers Lv5 [TS] evolution, gated play-cost reduction, shared source-trash
//! plus lock effect, and [All Turns][OPT] TS leave replacement.

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledDeclarativeClause,
};
use digimon_engine::action::space::{encode_attack, PASS, REPLACEMENT_ACCEPT};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming, Expiry, ModifierType};
use digimon_engine::modifiers::PlayerModifierEntry;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::TriggerSource;

#[test]
fn bt24_040_has_printed_stats_lv5_ts_alt_path_and_security_gated_cost_reduction() {
    let compiled = compiled_bt24_040();

    assert_eq!(compiled.card, "BT24-040");
    assert_eq!(compiled.name, "Venusmon");
    assert_eq!(compiled.level, Some(6));
    assert_eq!(
        compiled.color,
        vec![CompiledColor::Yellow, CompiledColor::Blue]
    );
    assert_eq!(compiled.cost, Some(12));
    assert_eq!(compiled.dp, Some(12000));
    for trait_name in ["Shaman", "Olympos XII", "Iliad", "TS"] {
        assert!(
            compiled.traits.iter().any(|t| t == trait_name),
            "missing trait {trait_name}"
        );
    }

    assert!(
        compiled.alt_paths.iter().any(|path| {
            path.kind == CompiledAltPathKind::Digivolve
                && path.cost == Some(CompiledCost::Literal(3))
                && path.from.as_ref().is_some_and(|from| {
                    from.all_of.iter().any(|pred| pred.level_eq == Some(5))
                        && from
                            .all_of
                            .iter()
                            .any(|pred| pred.trait_has.as_deref() == Some("TS"))
                })
        }),
        "Venusmon must digivolve from Lv5 [TS] for cost 3"
    );

    let cost_reduction = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction {
                when_playing_this,
                condition,
                amount,
                ..
            }) => Some((*when_playing_this, condition, amount)),
            _ => None,
        })
        .expect("Venusmon play-cost reduction clause");
    assert!(
        cost_reduction.0,
        "cost reduction must only apply to this card"
    );
    assert_eq!(cost_reduction.2, &Some(5));
    assert_eq!(
        cost_reduction
            .1
            .as_ref()
            .and_then(|pred| pred.security_count_lte),
        Some(3),
        "cost reduction must require your security count <= 3"
    );
}

#[test]
fn bt24_040_on_play_trashes_selected_sources_and_locks_two_specific_permanents() {
    let mut runner = shared_body_runner()
        .hand(0, &["BT24-040"])
        .memory(20)
        .start();
    let source_stack = runner.place_stack(1, &["SRC-A", "SRC-B", "STACK-TOP"]);
    let lock_digimon = runner.place_on_field(1, "LOCK-DIGIMON", Some(0));
    let lock_tamer = runner.place_on_field(1, "LOCK-TAMER", Some(0));

    runner.play(0, 0).expect("play Venusmon");
    choose_permanent(&mut runner, source_stack, "trash selected source stack");
    choose_permanent(&mut runner, lock_tamer, "lock selected Tamer");
    let second_lock_view = runner
        .pending_selection_view()
        .expect("second lock selection");
    assert!(
        !second_lock_view
            .valid_action_ids
            .contains(&encode_permanent(lock_tamer)),
        "second lock prompt must exclude the first lock target"
    );
    choose_permanent(&mut runner, lock_digimon, "lock selected Digimon");
    runner.auto_resolve().expect("finish Venusmon On Play");

    assert_eq!(stack_ids(&runner, source_stack), vec!["STACK-TOP"]);
    assert!(trash_contains(&runner, 1, "SRC-A"));
    assert!(trash_contains(&runner, 1, "SRC-B"));
    assert_locked(&runner, lock_tamer);
    assert_locked(&runner, lock_digimon);
    assert!(
        !runner
            .modifiers()
            .has(source_stack, ModifierType::CannotSuspend),
        "source-stack target was not one of the chosen lock targets"
    );
}

#[test]
fn bt24_040_when_digivolving_uses_same_source_trash_and_lock_body() {
    let mut runner = shared_body_runner()
        .add_card(make_trait_digimon("LV5-TS", 5, 7000, &["TS"]))
        .hand(0, &["BT24-040"])
        .memory(20)
        .start();
    let base = runner.place_on_field(0, "LV5-TS", Some(0));
    let source_stack = runner.place_stack(1, &["SRC-A", "SRC-B", "STACK-TOP"]);
    let lock_digimon = runner.place_on_field(1, "LOCK-DIGIMON", Some(0));
    let lock_tamer = runner.place_on_field(1, "LOCK-TAMER", Some(0));
    let evo_card = runner.game.players[0].hand.remove(0);
    assert!(runner.game.digivolve_onto(0, base.index as usize, evo_card));

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(base),
    );
    runner.game.drain_effect_queue();
    choose_permanent(&mut runner, source_stack, "trash selected source stack");
    choose_permanent(&mut runner, lock_digimon, "lock selected Digimon");
    choose_permanent(&mut runner, lock_tamer, "lock selected Tamer");
    runner
        .auto_resolve()
        .expect("finish Venusmon When Digivolving");

    assert_eq!(stack_ids(&runner, source_stack), vec!["STACK-TOP"]);
    assert_locked(&runner, lock_digimon);
    assert_locked(&runner, lock_tamer);
}

#[test]
fn bt24_040_protects_ts_digimon_from_non_own_effect_by_placing_other_sourceless_digimon_in_security(
) {
    let mut runner = protection_runner().start();
    runner.place_on_field(0, "BT24-040", Some(0));
    let target = runner.place_on_field(0, "TS-TARGET", Some(0));
    let cost_body = runner.place_on_field(0, "COST-BODY", Some(0));
    let stacked_body = runner.place_stack(0, &["STACKED-SOURCE", "STACKED-BODY"]);

    runner.game.return_to_hand_from_effect(target, 1);
    assert!(
        runner.pending_is_optional(),
        "replacement protection is a cost the player may decline"
    );
    runner
        .execute_action(0, REPLACEMENT_ACCEPT)
        .expect("accept replacement protection");
    let view = runner
        .pending_selection_view()
        .expect("choose replacement cost body");
    assert!(
        !view.valid_action_ids.contains(&encode_permanent(target)),
        "replacement subject cannot be selected as the other Digimon"
    );
    assert!(
        !view
            .valid_action_ids
            .contains(&encode_permanent(stacked_body)),
        "Digimon with digivolution cards cannot pay the cost"
    );
    choose_permanent(&mut runner, cost_body, "pay replacement cost");

    assert!(permanent_exists(&runner, 0, "TS-TARGET"));
    assert!(!permanent_exists(&runner, 0, "COST-BODY"));
    assert_eq!(bottom_security_id(&runner, 0), Some("COST-BODY"));
}

#[test]
fn bt24_040_protection_can_pay_with_opponent_sourceless_digimon() {
    let mut runner = protection_runner()
        .add_card(make_test_card("OPP-COST-BODY", "Opponent Cost Body"))
        .start();
    runner.place_on_field(0, "BT24-040", Some(0));
    let target = runner.place_on_field(0, "TS-TARGET", Some(0));
    let own_cost_body = runner.place_on_field(0, "COST-BODY", Some(0));
    let opponent_cost_body = runner.place_on_field(1, "OPP-COST-BODY", Some(0));

    runner.game.return_to_hand_from_effect(target, 1);
    assert!(
        runner.pending_is_optional(),
        "replacement protection is a cost the player may decline"
    );
    runner
        .execute_action(0, REPLACEMENT_ACCEPT)
        .expect("accept replacement protection");
    let view = runner
        .pending_selection_view()
        .expect("choose replacement cost body");
    assert!(
        view.valid_action_ids
            .contains(&encode_any_permanent(own_cost_body)),
        "own sourceless Digimon remains a legal cost body"
    );
    assert!(
        view.valid_action_ids
            .contains(&encode_any_permanent(opponent_cost_body)),
        "opponent sourceless Digimon must be a legal cost body"
    );
    choose_any_permanent(
        &mut runner,
        opponent_cost_body,
        "pay with opponent cost body",
    );

    assert!(permanent_exists(&runner, 0, "TS-TARGET"));
    assert!(permanent_exists(&runner, 0, "COST-BODY"));
    assert!(!permanent_exists(&runner, 1, "OPP-COST-BODY"));
    assert_eq!(bottom_security_id(&runner, 0), Some("OPP-COST-BODY"));
}

#[test]
fn bt24_040_replacement_is_not_offered_without_legal_cost_body_and_opt_remains_available() {
    let mut runner = protection_runner()
        .add_card(make_trait_digimon("TS-TARGET-B", 5, 7000, &["TS"]))
        .start();
    runner.place_stack(0, &["STACKED-SOURCE", "BT24-040"]);
    let target_a = runner.place_on_field(0, "TS-TARGET", Some(0));

    runner.game.return_to_hand_from_effect(target_a, 1);

    assert!(
        runner.pending_selection().is_none(),
        "replacement accept must not be offered when no legal cost body exists"
    );
    assert!(!permanent_exists(&runner, 0, "TS-TARGET"));
    assert!(
        hand_contains(&runner, 0, "TS-TARGET"),
        "unpayable replacement should let the original leave event resolve"
    );
    assert_eq!(
        runner.security_count(0),
        0,
        "unpayable replacement must not place a cost body in security"
    );

    let target_b = runner.place_on_field(0, "TS-TARGET-B", Some(0));
    let cost_body = runner.place_on_field(0, "COST-BODY", Some(0));

    runner.game.return_to_hand_from_effect(target_b, 1);

    let view = runner
        .pending_selection_view()
        .expect("later payable replacement should still be offered");
    assert!(
        view.valid_action_ids.contains(&REPLACEMENT_ACCEPT),
        "OPT must remain available after the earlier unpayable event"
    );
    runner
        .execute_action(0, REPLACEMENT_ACCEPT)
        .expect("accept later payable replacement");
    choose_any_permanent(&mut runner, cost_body, "pay later replacement cost");

    assert!(permanent_exists(&runner, 0, "TS-TARGET-B"));
    assert!(!permanent_exists(&runner, 0, "COST-BODY"));
    assert_eq!(bottom_security_id(&runner, 0), Some("COST-BODY"));
}

#[test]
fn bt24_040_replacement_is_not_offered_when_security_placement_cost_is_floodgated() {
    let mut runner = protection_runner()
        .add_card(make_trait_digimon("TS-TARGET-B", 5, 7000, &["TS"]))
        .add_card(make_test_card("COST-BODY-B", "Cost Body B"))
        .start();
    runner.place_on_field(0, "BT24-040", Some(0));
    let target_a = runner.place_on_field(0, "TS-TARGET", Some(0));
    runner.place_on_field(0, "COST-BODY", Some(0));

    runner.game.modifiers.add_player_modifier(
        0,
        PlayerModifierEntry::simple(
            ModifierType::CannotAddSecurityByEffect,
            0,
            Expiry::EndOfTurn,
            None,
            1,
        ),
    );

    runner.game.return_to_hand_from_effect(target_a, 1);

    assert!(
        runner.pending_selection().is_none(),
        "replacement accept must not be offered when security placement costs are floodgated"
    );
    assert!(!permanent_exists(&runner, 0, "TS-TARGET"));
    assert!(
        hand_contains(&runner, 0, "TS-TARGET"),
        "floodgated replacement should let the original leave event resolve"
    );
    assert!(permanent_exists(&runner, 0, "COST-BODY"));
    assert_eq!(
        runner.security_count(0),
        0,
        "blocked security-placement cost must leave security unchanged"
    );

    runner.game.modifiers.expire_player_end_of_turn(0);
    let target_b = runner.place_on_field(0, "TS-TARGET-B", Some(0));
    let cost_body_b = runner.place_on_field(0, "COST-BODY-B", Some(0));

    runner.game.return_to_hand_from_effect(target_b, 1);

    let view = runner
        .pending_selection_view()
        .expect("later payable replacement should still be offered after the floodgate expires");
    assert!(
        view.valid_action_ids.contains(&REPLACEMENT_ACCEPT),
        "OPT must remain available after the earlier floodgated event"
    );
    runner
        .execute_action(0, REPLACEMENT_ACCEPT)
        .expect("accept later payable replacement");
    choose_any_permanent(
        &mut runner,
        cost_body_b,
        "pay later replacement cost after floodgate expires",
    );

    assert!(permanent_exists(&runner, 0, "TS-TARGET-B"));
    assert!(!permanent_exists(&runner, 0, "COST-BODY-B"));
    assert_eq!(bottom_security_id(&runner, 0), Some("COST-BODY-B"));
}

#[test]
fn bt24_040_protection_does_not_fire_for_your_own_effects() {
    let mut runner = protection_runner().start();
    runner.place_on_field(0, "BT24-040", Some(0));
    let target = runner.place_on_field(0, "TS-TARGET", Some(0));
    runner.place_on_field(0, "COST-BODY", Some(0));

    runner.game.return_to_hand_from_effect(target, 0);

    assert!(
        runner.pending_selection().is_none(),
        "own effects must not install the replacement cost selection"
    );
    assert!(!permanent_exists(&runner, 0, "TS-TARGET"));
    assert_eq!(runner.security_count(0), 0);
}

#[test]
fn bt24_040_replacement_is_once_per_turn_even_with_another_cost_body_available() {
    let mut runner = protection_runner()
        .add_card(make_trait_digimon("TS-TARGET-B", 5, 7000, &["TS"]))
        .add_card(make_test_card("COST-BODY-B", "Cost Body B"))
        .start();
    runner.place_on_field(0, "BT24-040", Some(0));
    let target_a = runner.place_on_field(0, "TS-TARGET", Some(0));
    let target_b = runner.place_on_field(0, "TS-TARGET-B", Some(0));
    let cost_a = runner.place_on_field(0, "COST-BODY", Some(0));
    runner.place_on_field(0, "COST-BODY-B", Some(0));

    runner.game.return_to_hand_from_effect(target_a, 1);
    assert!(
        runner.pending_is_optional(),
        "replacement protection is a cost the player may decline"
    );
    runner
        .execute_action(0, REPLACEMENT_ACCEPT)
        .expect("accept replacement protection");
    choose_permanent(&mut runner, cost_a, "pay first replacement cost");
    assert!(permanent_exists(&runner, 0, "TS-TARGET"));

    runner.game.return_to_hand_from_effect(target_b, 1);

    assert!(
        runner.pending_selection().is_none(),
        "OPT replacement must not prompt for the second matching leave event"
    );
    assert!(!permanent_exists(&runner, 0, "TS-TARGET-B"));
    assert!(permanent_exists(&runner, 0, "COST-BODY-B"));
    assert_eq!(runner.security_count(0), 1);
}

#[test]
fn bt24_040_declining_replacement_lets_ts_digimon_leave_without_paying_cost_body() {
    let mut runner = protection_runner().start();
    runner.place_on_field(0, "BT24-040", Some(0));
    let target = runner.place_on_field(0, "TS-TARGET", Some(0));
    runner.place_on_field(0, "COST-BODY", Some(0));

    runner.game.return_to_hand_from_effect(target, 1);
    assert!(
        runner.pending_is_optional(),
        "replacement prompt should allow PASS before choosing a cost body"
    );
    runner
        .execute_action(0, PASS)
        .expect("decline replacement protection");

    assert!(!permanent_exists(&runner, 0, "TS-TARGET"));
    assert!(
        hand_contains(&runner, 0, "TS-TARGET"),
        "decline should commit the original return-to-hand event"
    );
    assert!(
        permanent_exists(&runner, 0, "COST-BODY"),
        "decline must not place any cost body in security"
    );
    assert_eq!(runner.security_count(0), 0);
}

fn venusmon_runner() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card("BT24-040")
        .expect("BT24-040 YAML loads")
}

fn shared_body_runner() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    venusmon_runner()
        .add_card(make_test_card("SRC-A", "Source A"))
        .add_card(make_test_card("SRC-B", "Source B"))
        .add_card(make_test_card("STACK-TOP", "Stack Top"))
        .add_card(make_test_card("LOCK-DIGIMON", "Lock Digimon"))
        .add_card(make_tamer("LOCK-TAMER"))
}

fn protection_runner() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    venusmon_runner()
        .add_card(make_trait_digimon("TS-TARGET", 5, 7000, &["TS"]))
        .add_card(make_test_card("COST-BODY", "Cost Body"))
        .add_card(make_test_card("STACKED-SOURCE", "Stacked Source"))
        .add_card(make_test_card("STACKED-BODY", "Stacked Body"))
}

fn choose_permanent(runner: &mut DebugRunner, handle: PermanentHandle, label: &str) {
    let view = runner.pending_selection_view().expect(label);
    let action = encode_permanent(handle);
    assert!(
        view.valid_action_ids.contains(&action),
        "{label}: expected action {action}, got {:?}",
        view.valid_action_ids
    );
    runner
        .execute_action(view.selecting_player, action)
        .expect(label);
}

fn choose_any_permanent(runner: &mut DebugRunner, handle: PermanentHandle, label: &str) {
    let view = runner.pending_selection_view().expect(label);
    let action = encode_any_permanent(handle);
    assert!(
        view.valid_action_ids.contains(&action),
        "{label}: expected action {action}, got {:?}",
        view.valid_action_ids
    );
    runner
        .execute_action(view.selecting_player, action)
        .expect(label);
}

fn encode_permanent(handle: PermanentHandle) -> u16 {
    encode_attack(0, handle.index as u16)
}

fn encode_any_permanent(handle: PermanentHandle) -> u16 {
    encode_attack(handle.player as u16, handle.index as u16)
}

fn assert_locked(runner: &DebugRunner, handle: PermanentHandle) {
    assert!(
        runner.modifiers().has(handle, ModifierType::CannotSuspend),
        "expected CannotSuspend on {handle:?}"
    );
    assert!(
        runner
            .modifiers()
            .has(handle, ModifierType::CannotActivateWhenDigivolvingEffects),
        "expected CannotActivateWhenDigivolvingEffects on {handle:?}"
    );
}

fn make_trait_digimon(id: &str, level: u8, dp: i32, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card.traits = traits.iter().map(|s| s.to_string()).collect();
    card
}

fn make_tamer(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Tamer;
    card.level = None;
    card.dp = None;
    card
}

fn stack_ids(runner: &DebugRunner, handle: PermanentHandle) -> Vec<&str> {
    runner.game.players[handle.player as usize].battle_area[handle.index as usize]
        .card_sources
        .iter()
        .map(|card| card.card_id(&runner.game.card_data))
        .collect()
}

fn trash_contains(runner: &DebugRunner, player: usize, id: &str) -> bool {
    runner.game.players[player]
        .trash
        .iter()
        .any(|card| card.card_id(&runner.game.card_data) == id)
}

fn permanent_exists(runner: &DebugRunner, player: usize, id: &str) -> bool {
    runner.game.players[player]
        .battle_area
        .iter()
        .any(|perm| perm.top_card().card_id(&runner.game.card_data) == id)
}

fn hand_contains(runner: &DebugRunner, player: usize, id: &str) -> bool {
    runner.game.players[player]
        .hand
        .iter()
        .any(|card| card.card_id(&runner.game.card_data) == id)
}

fn bottom_security_id(runner: &DebugRunner, player: usize) -> Option<&str> {
    runner.game.players[player]
        .security
        .first()
        .map(|card| card.card_id(&runner.game.card_data))
}

fn compiled_bt24_040() -> digimon_dsl::compiled::CompiledCard {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("cards")
        .join("bt24")
        .join("BT24-040.yaml");
    let yaml = std::fs::read_to_string(&path).expect("BT24-040 YAML exists");
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(&yaml).expect("BT24-040 YAML parses");
    let registry =
        digimon_dsl::CardRegistry::from_specs("test", &[spec]).expect("BT24-040 YAML compiles");
    registry
        .lookup("BT24-040")
        .expect("BT24-040 in registry")
        .clone()
}
