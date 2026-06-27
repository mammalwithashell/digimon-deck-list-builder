//! BT24-101 Jupitermon.
//! DCGO reference: DCGO/Assets/Scripts/CardEffect/BT24/Yellow/BT24_101.cs
//! Covers shared security trash/DP/Recovery, security-loss retaliation, and TS protection.

use digimon_dsl::compiled::{CompiledAltPathKind, CompiledColor, CompiledCost, CompiledFormula};
use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::{encode_digivolve, PASS, REPLACEMENT_ACCEPT};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::TriggerSource;

#[test]
fn bt24_101_has_standard_yellow_lv5_digivolve_cost_5() {
    let compiled = compiled_bt24_101();
    let path = compiled.alt_paths.iter().find(|path| {
        path.kind == CompiledAltPathKind::Digivolve
            && matches!(path.cost, Some(CompiledCost::Literal(5)))
            && path.from.as_ref().is_some_and(|from| {
                from.all_of.iter().any(|pred| pred.level_eq == Some(5))
                    && from
                        .all_of
                        .iter()
                        .any(|pred| pred.color_is == Some(CompiledColor::Yellow))
            })
    });

    assert!(
        path.is_some(),
        "Jupitermon must have its printed standard Lv5 yellow digivolution for cost 5"
    );
}

#[test]
fn bt24_101_has_lv5_ts_alt_digivolve_cost_5() {
    // The official Bandai DB / card image print "[Digivolve] Lv.5 w/[TS] trait: Cost 5"
    // — the same cost as the standard Yellow Lv.5 route, i.e. a gate-only alt (lets an
    // off-colour [TS] base digivolve for 5), NOT a cheaper route. An earlier authoring
    // of cost 3 was a transcription error, corrected under promote-official-bandai-card-source.
    let compiled = compiled_bt24_101();
    let path = compiled.alt_paths.iter().find(|path| {
        path.kind == CompiledAltPathKind::Digivolve
            && matches!(path.cost, Some(CompiledCost::Literal(5)))
            && path.from.as_ref().is_some_and(|from| {
                from.all_of.iter().any(|pred| pred.level_eq == Some(5))
                    && from
                        .all_of
                        .iter()
                        .any(|pred| pred.trait_has.as_deref() == Some("TS"))
            })
    });

    assert!(
        path.is_some(),
        "Jupitermon must have Lv5 [TS] alternate digivolution for cost 5 (official-DB value)"
    );
}

#[test]
fn bt24_101_has_lv5_aegiochusmon_alt_digivolve_cost_equal_to_security_count() {
    let compiled = compiled_bt24_101();
    let path = compiled.alt_paths.iter().find(|path| {
        path.kind == CompiledAltPathKind::Digivolve
            && matches!(
                path.cost,
                Some(CompiledCost::Formula(CompiledFormula::BasePerDelta {
                    base: 0,
                    delta: 1,
                    ..
                }))
            )
            && path.from.as_ref().is_some_and(|from| {
                from.all_of.iter().any(|pred| pred.level_eq == Some(5))
                    && from
                        .all_of
                        .iter()
                        .any(|pred| pred.name_contains.as_deref() == Some("Aegiochusmon"))
            })
    });

    assert!(
        path.is_some(),
        "Jupitermon must have Lv5 Aegiochusmon-name alternate digivolution with security-count cost"
    );
}

#[test]
fn bt24_101_generic_yellow_lv5_base_can_digivolve_for_cost_5() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT24-101")
        .expect("BT24-101 YAML loads")
        .add_card(make_colored_trait_digimon(
            "YELLOW-LV5",
            5,
            7000,
            CardColor::Yellow,
            &[],
        ))
        .hand(0, &["BT24-101"])
        .memory(10)
        .start();
    let base = runner.place_on_field(0, "YELLOW-LV5", Some(0));
    let action = encode_digivolve(0, base.index as u16);

    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[action as usize], 1.0,
        "Jupitermon's printed standard Lv5 yellow digivolve route should be action-mask legal"
    );

    runner.game.decode_action(action, 0);

    assert_eq!(
        runner.memory(),
        5,
        "standard Lv5 yellow digivolve should cost 5 memory"
    );
    assert_eq!(
        runner.game.players[0].battle_area[base.index as usize]
            .top_card()
            .card_id(&runner.game.card_data),
        "BT24-101"
    );
}

#[test]
fn bt24_101_aegiochusmon_alt_digivolve_spends_current_security_count() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT24-101")
        .expect("BT24-101 YAML loads")
        .add_card(make_trait_digimon("Aegiochusmon-LV5", 5, 7000, &[]))
        .add_card(make_test_card("SEC-A", "Security A"))
        .add_card(make_test_card("SEC-B", "Security B"))
        .add_card(make_test_card("SEC-C", "Security C"))
        .hand(0, &["BT24-101"])
        .security(0, &["SEC-A", "SEC-B", "SEC-C"])
        .memory(10)
        .start();
    let base = runner.place_on_field(0, "Aegiochusmon-LV5", Some(0));

    runner
        .game
        .decode_action(encode_digivolve(0, base.index as u16), 0);

    assert_eq!(
        runner.memory(),
        7,
        "3 security should make the alt path cost 3"
    );
    assert_eq!(
        runner.game.players[0].battle_area[base.index as usize]
            .top_card()
            .card_id(&runner.game.card_data),
        "BT24-101"
    );
}

#[test]
fn bt24_101_aegiochusmon_alt_digivolve_recomputes_when_security_count_changes() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT24-101")
        .expect("BT24-101 YAML loads")
        .add_card(make_trait_digimon("Aegiochusmon-LV5", 5, 7000, &[]))
        .add_card(make_test_card("SEC-A", "Security A"))
        .hand(0, &["BT24-101"])
        .security(0, &["SEC-A"])
        .memory(10)
        .start();
    let base = runner.place_on_field(0, "Aegiochusmon-LV5", Some(0));

    runner
        .game
        .decode_action(encode_digivolve(0, base.index as u16), 0);

    assert_eq!(
        runner.memory(),
        9,
        "1 security should make the alt path cost 1"
    );
}

#[test]
fn bt24_101_aegiochusmon_ts_base_prompts_cost_choice_and_ts_route_costs_5() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT24-101")
        .expect("BT24-101 YAML loads")
        .add_card(make_trait_digimon("Aegiochusmon-TS-LV5", 5, 7000, &["TS"]))
        .add_card(make_test_card("SEC-A", "Security A"))
        .add_card(make_test_card("SEC-B", "Security B"))
        .add_card(make_test_card("SEC-C", "Security C"))
        .add_card(make_test_card("SEC-D", "Security D"))
        .hand(0, &["BT24-101"])
        .security(0, &["SEC-A", "SEC-B", "SEC-C", "SEC-D"])
        .memory(10)
        .start();
    let base = runner.place_on_field(0, "Aegiochusmon-TS-LV5", Some(0));

    // The base is BOTH a [TS] Lv.5 (cost 5 — the printed [TS] alt, same cost as the
    // standard route per the card image; corrected from a mis-authored 3 under
    // promote-official-bandai-card-source) AND named Aegiochusmon (cost = 4, the security
    // count). Distinct costs {5, 4} → rule 17: the engine PROMPTS rather than auto-picking
    // the cheaper Aegiochusmon route. Choose the cost-5 [TS] route.
    runner
        .game
        .decode_action(encode_digivolve(0, base.index as u16), 0);
    let choices = runner
        .pending_selection_view()
        .expect("a digivolution cost-choice prompt must be installed")
        .effect_choices
        .expect("the prompt carries the cost options");
    assert!(
        choices.iter().any(|c| c.label.contains('4')),
        "the dynamic Aegiochusmon route (cost 4 = security count) must also be offered: {:?}",
        choices.iter().map(|c| c.label.clone()).collect::<Vec<_>>()
    );
    let ts = choices
        .iter()
        .find(|c| c.label.contains('5'))
        .expect("a cost-5 [TS] option");
    runner
        .execute_action(0, ts.action_id)
        .expect("pick the cost-5 [TS] route");

    assert_eq!(
        runner.memory(),
        5,
        "the chosen [TS] route costs 5 (10 - 5 = 5)"
    );
    assert_eq!(
        runner.game.players[0].battle_area[base.index as usize]
            .top_card()
            .card_id(&runner.game.card_data),
        "BT24-101"
    );
}

#[test]
fn bt24_101_on_play_trashes_own_top_security_debuffs_and_recovers_at_one_or_less() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT24-101")
        .expect("BT24-101 YAML loads")
        .add_card(make_trait_digimon("OPPONENT", 5, 14000, &[]))
        .add_card(make_test_card("SEC-BOTTOM", "Security Bottom"))
        .add_card(make_test_card("SEC-TOP", "Security Top"))
        .add_card(make_test_card("RECOVER-A", "Recover A"))
        .add_card(make_test_card("RECOVER-B", "Recover B"))
        .security(0, &["SEC-BOTTOM", "SEC-TOP"])
        .deck(0, &["RECOVER-A", "RECOVER-B"])
        .hand(0, &["BT24-101"])
        .memory(20)
        .start();
    let target = runner.place_on_field(1, "OPPONENT", Some(0));

    runner.play(0, 0).expect("play Jupitermon");
    pick_first_pending(&mut runner, "select DP target");
    runner.auto_resolve().expect("finish Jupitermon On Play");

    assert!(trash_contains(&runner, 0, "SEC-TOP"));
    assert_eq!(runner.dp_of(target), Some(1000));
    assert_eq!(
        security_ids(&runner, 0),
        vec!["SEC-BOTTOM", "RECOVER-B", "RECOVER-A"]
    );
}

#[test]
fn bt24_101_when_digivolving_uses_the_shared_effect_body() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT24-101")
        .expect("BT24-101 YAML loads")
        .add_card(make_trait_digimon("LV5-TS", 5, 6000, &["TS"]))
        .add_card(make_trait_digimon("OPPONENT", 5, 14000, &[]))
        .add_card(make_test_card("SEC", "Security"))
        .hand(0, &["BT24-101"])
        .security(0, &["SEC"])
        .memory(20)
        .start();
    let base = runner.place_on_field(0, "LV5-TS", Some(0));
    let target = runner.place_on_field(1, "OPPONENT", Some(0));
    let evo_card = runner.game.players[0].hand.remove(0);
    assert!(runner.game.digivolve_onto(0, base.index as usize, evo_card));

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(base),
    );
    runner.game.drain_effect_queue();
    pick_first_pending(&mut runner, "select DP target");
    runner
        .auto_resolve()
        .expect("finish Jupitermon When Digivolving");

    assert!(trash_contains(&runner, 0, "SEC"));
    assert_eq!(runner.dp_of(target), Some(1000));
}

#[test]
fn bt24_101_when_own_security_is_removed_trashes_opponent_top_security_once() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT24-101")
        .expect("BT24-101 YAML loads")
        .add_card(make_test_card("MY-SEC-A", "My Security A"))
        .add_card(make_test_card("MY-SEC-B", "My Security B"))
        .add_card(make_test_card("OPP-SEC-A", "Opponent Security A"))
        .add_card(make_test_card("OPP-SEC-B", "Opponent Security B"))
        .security(0, &["MY-SEC-A", "MY-SEC-B"])
        .security(1, &["OPP-SEC-A", "OPP-SEC-B"])
        .start();
    runner.place_on_field(0, "BT24-101", Some(0));

    let source_card = runner.game.players[0].battle_area[0].top_card().handle();
    {
        let mut ctx = digimon_engine::effect_context::EffectContext::new(
            &mut runner.game,
            source_card,
            None,
            0,
        );
        assert!(ctx.trash_top_security(0));
    }

    assert_eq!(runner.security_count(0), 1);
    assert_eq!(runner.security_count(1), 1);
    assert!(trash_contains(&runner, 1, "OPP-SEC-B"));

    {
        let mut ctx = digimon_engine::effect_context::EffectContext::new(
            &mut runner.game,
            source_card,
            None,
            0,
        );
        assert!(ctx.trash_top_security(0));
    }

    assert_eq!(
        runner.security_count(1),
        1,
        "once-per-turn security-loss retaliation should not refire"
    );
}

#[test]
fn bt24_101_protects_own_ts_digimon_or_tamer_by_trashing_top_security() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT24-101")
        .expect("BT24-101 YAML loads")
        .add_card(make_trait_digimon("TS-DIGIMON", 5, 6000, &["TS"]))
        .add_card(make_trait_tamer("TS-TAMER", &["TS"]))
        .add_card(make_trait_digimon("NON-TS", 5, 6000, &[]))
        .add_card(make_test_card("SEC-A", "Security A"))
        .add_card(make_test_card("SEC-B", "Security B"))
        .security(0, &["SEC-A", "SEC-B"])
        .start();
    runner.place_on_field(0, "BT24-101", Some(0));
    let ts_digimon = runner.place_on_field(0, "TS-DIGIMON", Some(0));
    runner.place_on_field(0, "TS-TAMER", Some(0));
    runner.place_on_field(0, "NON-TS", Some(0));

    runner.game.return_to_hand_from_effect(ts_digimon, 1);
    assert!(
        runner.pending_is_optional(),
        "replacement protection is a cost the player may decline"
    );
    runner
        .execute_action(0, REPLACEMENT_ACCEPT)
        .expect("accept replacement protection");

    assert!(permanent_exists(&runner, 0, "TS-DIGIMON"));
    assert_eq!(runner.security_count(0), 1, "security cost was paid");
    assert!(trash_contains(&runner, 0, "SEC-B"));

    let ts_tamer = permanent_handle_for(&runner, 0, "TS-TAMER").expect("TS Tamer remains");
    runner.game.return_to_hand_from_effect(ts_tamer, 1);
    assert_eq!(
        runner.security_count(0),
        1,
        "replacement is once per turn and cannot pay the second security this turn"
    );

    let non_ts = permanent_handle_for(&runner, 0, "NON-TS").expect("non-TS remains");
    runner.game.return_to_hand_from_effect(non_ts, 1);

    assert!(
        !permanent_exists(&runner, 0, "TS-TAMER"),
        "second OPT protection cannot fire even with another top security"
    );
    assert!(
        !permanent_exists(&runner, 0, "NON-TS"),
        "non-TS Digimon are not protected"
    );
}

#[test]
fn bt24_101_declining_replacement_lets_ts_digimon_leave_and_keeps_security() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT24-101")
        .expect("BT24-101 YAML loads")
        .add_card(make_trait_digimon("TS-DIGIMON", 5, 6000, &["TS"]))
        .add_card(make_test_card("SEC-A", "Security A"))
        .add_card(make_test_card("SEC-B", "Security B"))
        .security(0, &["SEC-A", "SEC-B"])
        .start();
    runner.place_on_field(0, "BT24-101", Some(0));
    let ts_digimon = runner.place_on_field(0, "TS-DIGIMON", Some(0));

    runner.game.return_to_hand_from_effect(ts_digimon, 1);
    assert!(
        runner.pending_is_optional(),
        "replacement prompt should allow PASS before paying the security cost"
    );
    runner
        .execute_action(0, PASS)
        .expect("decline replacement protection");

    assert!(!permanent_exists(&runner, 0, "TS-DIGIMON"));
    assert!(
        hand_contains(&runner, 0, "TS-DIGIMON"),
        "decline should commit the original return-to-hand event"
    );
    assert_eq!(
        security_ids(&runner, 0),
        vec!["SEC-A", "SEC-B"],
        "decline must not trash top security"
    );
    assert!(!trash_contains(&runner, 0, "SEC-B"));
}

fn make_trait_digimon(id: &str, level: u8, dp: i32, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card.traits = traits.iter().map(|s| s.to_string()).collect();
    card
}

fn make_colored_trait_digimon(
    id: &str,
    level: u8,
    dp: i32,
    color: CardColor,
    traits: &[&str],
) -> CardData {
    let mut card = make_trait_digimon(id, level, dp, traits);
    card.colors = vec![color];
    card
}

fn make_trait_tamer(id: &str, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Tamer;
    card.traits = traits.iter().map(|s| s.to_string()).collect();
    card
}

fn pick_first_pending(runner: &mut DebugRunner, label: &str) {
    let view = runner.pending_selection_view().expect(label);
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect(label);
}

fn trash_contains(runner: &DebugRunner, player: usize, id: &str) -> bool {
    runner.game.players[player]
        .trash
        .iter()
        .any(|card| card.card_id(&runner.game.card_data) == id)
}

fn security_ids(runner: &DebugRunner, player: usize) -> Vec<&str> {
    runner.game.players[player]
        .security
        .iter()
        .map(|card| card.card_id(&runner.game.card_data))
        .collect()
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

#[allow(dead_code)]
fn permanent_handle_for(runner: &DebugRunner, player: usize, id: &str) -> Option<PermanentHandle> {
    runner.game.players[player]
        .battle_area
        .iter()
        .enumerate()
        .find(|(_, perm)| perm.top_card().card_id(&runner.game.card_data) == id)
        .map(|(index, _)| PermanentHandle {
            player: player as u8,
            index: index as u8,
        })
}

fn compiled_bt24_101() -> digimon_dsl::compiled::CompiledCard {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("cards")
        .join("bt24")
        .join("BT24-101.yaml");
    let yaml = std::fs::read_to_string(&path).expect("BT24-101 YAML exists");
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(&yaml).expect("BT24-101 YAML parses");
    let registry =
        digimon_dsl::CardRegistry::from_specs("test", &[spec]).expect("BT24-101 YAML compiles");
    registry
        .lookup("BT24-101")
        .expect("BT24-101 in registry")
        .clone()
}
