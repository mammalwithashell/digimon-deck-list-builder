//! ST4 Giga Green starter deck behavioral smoke tests.

use digimon_dsl::compiled::{CompiledClause, CompiledTiming};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{
    CardColor, CardKind, GamePhase, Keyword, ModifierType, PlaySource, PlayerId,
};
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;

const ST4_YAMLS: &[(&str, &str)] = &[
    ("ST4-01", include_str!("../../../cards/st4/ST4-01.yaml")),
    ("ST4-02", include_str!("../../../cards/st4/ST4-02.yaml")),
    ("ST4-03", include_str!("../../../cards/st4/ST4-03.yaml")),
    ("ST4-04", include_str!("../../../cards/st4/ST4-04.yaml")),
    ("ST4-05", include_str!("../../../cards/st4/ST4-05.yaml")),
    ("ST4-06", include_str!("../../../cards/st4/ST4-06.yaml")),
    ("ST4-07", include_str!("../../../cards/st4/ST4-07.yaml")),
    ("ST4-08", include_str!("../../../cards/st4/ST4-08.yaml")),
    ("ST4-09", include_str!("../../../cards/st4/ST4-09.yaml")),
    ("ST4-10", include_str!("../../../cards/st4/ST4-10.yaml")),
    ("ST4-11", include_str!("../../../cards/st4/ST4-11.yaml")),
    ("ST4-12", include_str!("../../../cards/st4/ST4-12.yaml")),
    ("ST4-13", include_str!("../../../cards/st4/ST4-13.yaml")),
    ("ST4-14", include_str!("../../../cards/st4/ST4-14.yaml")),
    ("ST4-15", include_str!("../../../cards/st4/ST4-15.yaml")),
    ("ST4-16", include_str!("../../../cards/st4/ST4-16.yaml")),
];

fn digimon(id: &str, color: CardColor, level: u8, dp: i32) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![color];
    card.level = Some(level);
    card.dp = Some(dp);
    card
}

fn option_card(id: &str, color: CardColor) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Option;
    card.colors = vec![color];
    card.level = None;
    card.dp = None;
    card
}

fn digivolve_target(id: &str, color: CardColor, level: u8, dp: i32, from_level: u8) -> CardData {
    let mut card = digimon(id, color, level, dp);
    card.evo_costs = vec![EvoCost {
        card_color: 3,
        level: from_level,
        memory_cost: 0,
    }];
    card
}

/// Digivolve from hand, auto-resolving the rule-17 cost-choice prompt (when the
/// base satisfies more than one distinct-cost route) by taking the CHEAPEST
/// option. Used by setup digivolves that don't care which cost is paid — e.g.
/// ST4-10 here, whose synthetic evo-cost 0 + YAML alt-path cost 3 give two
/// distinct costs over a green Lv.4 base. Returns whether the digivolve
/// proceeded.
fn digivolve_cheapest(
    runner: &mut DebugRunner,
    player: PlayerId,
    hand: usize,
    field: usize,
    source: PlaySource,
) -> bool {
    if runner.game.digivolve_from_hand(player, hand, field, source) {
        return true; // single distinct cost — completed immediately
    }
    // A cost-choice EffectChoice was installed; its options are cheapest-first,
    // so the first valid action id is the cheapest route.
    if runner.pending_kind() == Some(SelectionKind::EffectChoice) {
        if let Some(aid) = runner
            .pending_selection()
            .and_then(|s| s.valid_action_ids.first().copied())
        {
            return runner.execute_action(player, aid).is_ok();
        }
    }
    false
}

fn make_p0_turn(runner: &mut DebugRunner) {
    runner.game.turn_order = vec![0, 1];
    runner.game.turn_player_idx = 0;
    runner.game.memory_pair = (0, 1);
    runner.game.current_phase = GamePhase::Main;
}

fn resolve_all(runner: &mut DebugRunner) {
    for _ in 0..8 {
        if runner.pending_selection().is_none() {
            return;
        }
        runner.auto_resolve().expect("pending selection resolves");
    }
    assert!(
        runner.pending_selection().is_none(),
        "pending selection did not resolve"
    );
}

#[test]
fn st4_deck_library_recipe_has_canonical_counts() {
    let library: serde_json::Value =
        serde_json::from_str(include_str!("../../../../../data/deck_library.json"))
            .expect("deck_library.json parses");
    let entry = &library["archetypes"]["ST-4 Giga Green"]["decklists"][0];
    assert_eq!(entry["deck_id"], "starter_st4_giga_green");

    let cards: Vec<String> = serde_json::from_str(
        entry["decklist"]
            .as_str()
            .expect("decklist is encoded JSON"),
    )
    .expect("decklist parses");
    let mut counts = std::collections::BTreeMap::<String, usize>::new();
    for card in &cards {
        *counts.entry(card.clone()).or_default() += 1;
    }

    assert_eq!(cards.len(), 54);
    assert_eq!(counts.get("ST4-01"), Some(&4));
    let main_count: usize = cards.iter().filter(|id| id.as_str() != "ST4-01").count();
    assert_eq!(main_count, 50);

    let expected = [
        ("ST4-01", 4),
        ("ST4-02", 4),
        ("ST4-03", 4),
        ("ST4-04", 4),
        ("ST4-05", 4),
        ("ST4-06", 4),
        ("ST4-07", 4),
        ("ST4-08", 2),
        ("ST4-09", 4),
        ("ST4-10", 4),
        ("ST4-11", 2),
        ("ST4-12", 2),
        ("ST4-13", 2),
        ("ST4-14", 4),
        ("ST4-15", 4),
        ("ST4-16", 2),
    ];
    for (card_id, count) in expected {
        assert_eq!(counts.get(card_id), Some(&count), "{card_id} count");
    }
}

#[test]
fn st4_all_yaml_specs_parse_and_compile() {
    for (card_id, yaml) in ST4_YAMLS {
        let spec: digimon_dsl::spec::CardSpec =
            serde_yml::from_str(yaml).unwrap_or_else(|e| panic!("{card_id} parses: {e}"));
        digimon_dsl::compile::compile(&spec)
            .unwrap_or_else(|e| panic!("{card_id} compiles: {e:?}"));
    }
}

#[test]
fn st4_01_and_st4_04_st4_06_inherited_dp_effects_are_gated() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST4-01")
        .expect("ST4-01 YAML loads")
        .add_card(digimon("LV5", CardColor::Green, 5, 7000))
        .add_card(digimon("LV6", CardColor::Green, 6, 11000))
        .start();
    make_p0_turn(&mut runner);
    let lv5 = runner.place_stack(0, &["ST4-01", "LV5"]);
    let lv6 = runner.place_stack(0, &["ST4-01", "LV6"]);

    assert_eq!(runner.effective_dp(lv5), Some(7000));
    assert_eq!(runner.effective_dp(lv6), Some(12000));

    for source_id in ["ST4-04", "ST4-06"] {
        let mut runner = DebugRunner::builder()
            .dsl_card(source_id)
            .expect("inherited DP YAML loads")
            .add_card(digimon("ATTACKER", CardColor::Green, 6, 4000))
            .add_card(digimon("DEFENDER", CardColor::Red, 4, 5000))
            .memory(10)
            .start();
        make_p0_turn(&mut runner);
        let attacker = runner.place_stack(0, &[source_id, "ATTACKER"]);
        let defender = runner.place_on_field(1, "DEFENDER", Some(0));

        assert_eq!(
            runner.attack_digimon(attacker, defender, true),
            AttackResult::AttackerWins,
            "{source_id} should add +2000 DP when attacking an opponent Digimon"
        );
        assert_eq!(runner.effective_dp(attacker), Some(6000));
    }
}

#[test]
fn st4_03_reveal_search_adds_only_green_digimon() {
    let mut hit = DebugRunner::builder()
        .dsl_card("ST4-03")
        .expect("ST4-03 YAML loads")
        .add_card(digimon("GREEN-HIT", CardColor::Green, 4, 3000))
        .hand(0, &["ST4-03"])
        .deck(0, &["GREEN-HIT"])
        .memory(10)
        .start();
    make_p0_turn(&mut hit);

    assert!(hit.play(0, 0).is_some());
    resolve_all(&mut hit);
    assert_eq!(hit.hand_size(0), 1);
    assert_eq!(hit.deck_size(0), 0);

    let mut miss = DebugRunner::builder()
        .dsl_card("ST4-03")
        .expect("ST4-03 YAML loads")
        .add_card(digimon("RED-MISS", CardColor::Red, 4, 3000))
        .hand(0, &["ST4-03"])
        .deck(0, &["RED-MISS"])
        .memory(10)
        .start();
    make_p0_turn(&mut miss);

    assert!(miss.play(0, 0).is_some());
    resolve_all(&mut miss);
    assert_eq!(miss.hand_size(0), 0);
    assert_eq!(miss.deck_size(0), 1, "miss returns to bottom of deck");
}

#[test]
fn st4_10_reveal_search_adds_only_level_six_or_higher_digimon() {
    let mut hit = DebugRunner::builder()
        .dsl_card("ST4-10")
        .expect("ST4-10 YAML loads")
        .add_card(digivolve_target("ST4-10", CardColor::Green, 5, 7000, 4))
        .add_card(digimon("BASE", CardColor::Green, 4, 4000))
        .add_card(digimon("LV6-HIT", CardColor::Green, 6, 11000))
        .add_card(option_card("DRAW", CardColor::Green))
        .hand(0, &["ST4-10"])
        .deck(0, &["LV6-HIT", "DRAW"])
        .memory(10)
        .start();
    make_p0_turn(&mut hit);
    let base = hit.place_on_field(0, "BASE", Some(0));

    assert!(digivolve_cheapest(
        &mut hit,
        0,
        0,
        base.index as usize,
        PlaySource::ByDigivolve
    ));
    resolve_all(&mut hit);
    assert_eq!(
        hit.hand_size(0),
        2,
        "rule draw plus ST4-10 search hit should both add cards"
    );
    assert_eq!(hit.deck_size(0), 0);

    let mut miss = DebugRunner::builder()
        .dsl_card("ST4-10")
        .expect("ST4-10 YAML loads")
        .add_card(digivolve_target("ST4-10", CardColor::Green, 5, 7000, 4))
        .add_card(digimon("BASE", CardColor::Green, 4, 4000))
        .add_card(digimon("LV5-MISS", CardColor::Green, 5, 7000))
        .add_card(option_card("DRAW", CardColor::Green))
        .hand(0, &["ST4-10"])
        .deck(0, &["LV5-MISS", "DRAW"])
        .memory(10)
        .start();
    make_p0_turn(&mut miss);
    let base = miss.place_on_field(0, "BASE", Some(0));

    assert!(digivolve_cheapest(
        &mut miss,
        0,
        0,
        base.index as usize,
        PlaySource::ByDigivolve
    ));
    resolve_all(&mut miss);
    assert_eq!(miss.hand_size(0), 1, "only the rule draw adds a card");
    assert_eq!(miss.deck_size(0), 1, "missed reveal returns to deck bottom");
}

#[test]
fn st4_08_grants_blocker_and_loses_memory_when_attacking() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST4-08")
        .expect("ST4-08 YAML loads")
        .add_card(digimon("DEF", CardColor::Red, 4, 3000))
        .memory(10)
        .start();
    let attacker = runner.place_on_field(0, "ST4-08", Some(0));
    let defender = runner.place_on_field(1, "DEF", Some(0));

    assert!(runner.game.has_keyword(attacker, Keyword::Blocker));
    let before = runner.memory();
    runner.attack_digimon(attacker, defender, true);

    assert_eq!(runner.memory(), before - 2);
}

#[test]
fn st4_11_trashes_security_only_when_carrier_survives_battle() {
    let mut win = DebugRunner::builder()
        .dsl_card("ST4-11")
        .expect("ST4-11 YAML loads")
        .add_card(digimon("ATTACKER-WIN", CardColor::Green, 6, 9000))
        .add_card(digimon("DEFENDER-LOSE", CardColor::Red, 5, 3000))
        .add_card(make_test_card("SEC", "Security"))
        .security(1, &["SEC", "SEC"])
        .start();
    let attacker = win.place_stack(0, &["ST4-11", "ATTACKER-WIN"]);
    let defender = win.place_on_field(1, "DEFENDER-LOSE", Some(0));

    assert_eq!(
        win.attack_digimon(attacker, defender, true),
        AttackResult::AttackerWins
    );
    assert_eq!(win.security_count(1), 1);

    let mut tie = DebugRunner::builder()
        .dsl_card("ST4-11")
        .expect("ST4-11 YAML loads")
        .add_card(digimon("ATTACKER-TIE", CardColor::Green, 6, 6000))
        .add_card(digimon("DEFENDER-TIE", CardColor::Red, 5, 6000))
        .add_card(make_test_card("SEC", "Security"))
        .security(1, &["SEC", "SEC"])
        .start();
    let attacker = tie.place_stack(0, &["ST4-11", "ATTACKER-TIE"]);
    let defender = tie.place_on_field(1, "DEFENDER-TIE", Some(0));

    assert_eq!(
        tie.attack_digimon(attacker, defender, true),
        AttackResult::MutualDestruction
    );
    assert_eq!(tie.security_count(1), 2);

    let mut unrelated = DebugRunner::builder()
        .dsl_card("ST4-11")
        .expect("ST4-11 YAML loads")
        .add_card(digimon("CARRIER", CardColor::Green, 6, 9000))
        .add_card(digimon("ALLY", CardColor::Green, 6, 9000))
        .add_card(digimon("DEFENDER", CardColor::Red, 4, 3000))
        .add_card(make_test_card("SEC", "Security"))
        .security(1, &["SEC", "SEC"])
        .start();
    unrelated.place_stack(0, &["ST4-11", "CARRIER"]);
    let ally = unrelated.place_on_field(0, "ALLY", Some(0));
    let defender = unrelated.place_on_field(1, "DEFENDER", Some(0));

    assert_eq!(
        unrelated.attack_digimon(ally, defender, true),
        AttackResult::AttackerWins
    );
    assert_eq!(unrelated.security_count(1), 2);

    let mut non_battle = DebugRunner::builder()
        .dsl_card("ST4-11")
        .expect("ST4-11 YAML loads")
        .add_card(digimon("CARRIER", CardColor::Green, 6, 9000))
        .add_card(digimon("TARGET", CardColor::Red, 4, 3000))
        .add_card(make_test_card("SEC", "Security"))
        .security(1, &["SEC", "SEC"])
        .start();
    non_battle.place_stack(0, &["ST4-11", "CARRIER"]);
    let target = non_battle.place_on_field(1, "TARGET", Some(0));

    non_battle
        .game
        .delete_permanent_with_cause(target, ReplacementCause::OpponentEffect);
    resolve_all(&mut non_battle);
    assert_eq!(non_battle.security_count(1), 2);

    let mut opt = DebugRunner::builder()
        .dsl_card("ST4-11")
        .expect("ST4-11 YAML loads")
        .add_card(digimon("ATTACKER", CardColor::Green, 6, 9000))
        .add_card(digimon("DEF-A", CardColor::Red, 4, 3000))
        .add_card(digimon("DEF-B", CardColor::Red, 4, 3000))
        .add_card(make_test_card("SEC", "Security"))
        .security(1, &["SEC", "SEC", "SEC"])
        .start();
    let attacker = opt.place_stack(0, &["ST4-11", "ATTACKER"]);
    let def_a = opt.place_on_field(1, "DEF-A", Some(0));

    assert_eq!(
        opt.attack_digimon(attacker, def_a, true),
        AttackResult::AttackerWins
    );
    opt.game.players[0].battle_area[attacker.index as usize].is_suspended = false;
    let def_b = opt.place_on_field(1, "DEF-B", Some(0));
    assert_eq!(
        opt.attack_digimon(attacker, def_b, true),
        AttackResult::AttackerWins
    );
    assert_eq!(opt.security_count(1), 2, "ST4-11 is once per turn");
}

#[test]
fn st4_12_suppresses_attack_and_block_until_opponents_turn_ends() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST4-12")
        .expect("ST4-12 YAML loads")
        .add_card(digivolve_target("ST4-12", CardColor::Green, 6, 11000, 5))
        .add_card(digimon("BASE", CardColor::Green, 5, 7000))
        .add_card(digimon("TARGET", CardColor::Red, 4, 4000))
        .add_card(option_card("FILLER", CardColor::Green))
        .hand(0, &["ST4-12"])
        .deck(0, &["FILLER", "FILLER"])
        .deck(1, &["FILLER", "FILLER"])
        .memory(10)
        .start();
    make_p0_turn(&mut runner);
    let base = runner.place_on_field(0, "BASE", Some(0));
    let target = runner.place_on_field(1, "TARGET", Some(0));

    assert!(digivolve_cheapest(
        &mut runner,
        0,
        0,
        base.index as usize,
        PlaySource::ByDigivolve
    ));
    resolve_all(&mut runner);

    assert!(runner
        .game
        .modifiers
        .has(target, ModifierType::CannotAttack));
    assert!(runner.game.modifiers.has(target, ModifierType::CannotBlock));

    runner.end_turn();
    assert!(runner
        .game
        .modifiers
        .has(target, ModifierType::CannotAttack));
    runner.end_turn();
    assert!(!runner
        .game
        .modifiers
        .has(target, ModifierType::CannotAttack));
    assert!(!runner.game.modifiers.has(target, ModifierType::CannotBlock));
}

#[test]
fn st4_13_pierces_and_digi_bursts_to_suspend() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST4-13")
        .expect("ST4-13 YAML loads")
        .add_card(digimon("SRC-A", CardColor::Green, 4, 3000))
        .add_card(digimon("SRC-B", CardColor::Green, 4, 3000))
        .add_card(digimon("DEF", CardColor::Red, 4, 3000))
        .memory(10)
        .start();
    let hercules = runner.place_stack(0, &["SRC-A", "SRC-B", "ST4-13"]);
    let defender = runner.place_on_field(1, "DEF", Some(0));

    assert!(runner.game.has_keyword(hercules, Keyword::Piercing));
    assert!(runner.game.activate_field_main(0, hercules.index as usize));
    resolve_all(&mut runner);

    assert!(
        runner.game.players[1].battle_area[defender.index as usize].is_suspended,
        "Digi-Burst effect should suspend the selected opponent Digimon"
    );
    assert_eq!(
        runner.game.players[0].battle_area[hercules.index as usize]
            .card_sources
            .len(),
        1,
        "Digi-Burst 2 should trash the two sources"
    );
}

#[test]
fn st4_14_optional_suspend_cost_gains_memory_on_opponent_suspend() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST4-14")
        .expect("ST4-14 YAML loads")
        .add_card(digimon("TARGET", CardColor::Red, 4, 3000))
        .memory(0)
        .start();
    make_p0_turn(&mut runner);
    let izzy = runner.place_on_field(0, "ST4-14", Some(0));
    let target = runner.place_on_field(1, "TARGET", Some(0));

    runner.game.suspend(target);
    resolve_all(&mut runner);

    assert!(runner.game.players[0].battle_area[izzy.index as usize].is_suspended);
    assert_eq!(runner.memory(), 1);
}

#[test]
fn st4_14_security_plays_tamer_for_free() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST4-14")
        .expect("ST4-14 YAML loads")
        .add_card(digimon("ATTACKER", CardColor::Red, 4, 9000))
        .security(1, &["ST4-14"])
        .memory(10)
        .start();
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));

    runner.attack_player(attacker, 1, false);
    resolve_all(&mut runner);

    assert!(runner.game.players[1]
        .battle_area
        .iter()
        .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "ST4-14"));
}

#[test]
fn st4_15_main_suspends_and_st4_16_main_returns_suspended_digimon() {
    let mut spray = DebugRunner::builder()
        .dsl_card("ST4-15")
        .expect("ST4-15 YAML loads")
        .add_card(digimon("TARGET", CardColor::Red, 4, 3000))
        .hand(0, &["ST4-15"])
        .memory(10)
        .start();
    make_p0_turn(&mut spray);
    let target = spray.place_on_field(1, "TARGET", Some(0));

    assert!(spray.play(0, 0).is_some());
    resolve_all(&mut spray);
    assert!(spray.game.players[1].battle_area[target.index as usize].is_suspended);

    let mut shocker = DebugRunner::builder()
        .dsl_card("ST4-16")
        .expect("ST4-16 YAML loads")
        .add_card(digimon("TARGET", CardColor::Red, 4, 3000))
        .hand(0, &["ST4-16"])
        .memory(10)
        .start();
    make_p0_turn(&mut shocker);
    let target = shocker.place_on_field(1, "TARGET", Some(0));
    shocker.game.players[1].battle_area[target.index as usize].is_suspended = true;

    assert!(shocker.play(0, 0).is_some());
    resolve_all(&mut shocker);
    assert!(shocker.game.players[1].battle_area.is_empty());
    assert_eq!(
        shocker.hand_size(1),
        1,
        "returned target goes to owner's hand"
    );
}

#[test]
fn st4_15_security_suspends_and_adds_to_hand() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST4-15")
        .expect("ST4-15 YAML loads")
        .add_card(digimon("ATTACKER", CardColor::Red, 4, 9000))
        .add_card(digimon("TARGET", CardColor::Red, 4, 3000))
        .security(1, &["ST4-15"])
        .memory(10)
        .start();
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));
    let target = runner.place_on_field(0, "TARGET", Some(0));

    runner.attack_player(attacker, 1, false);
    resolve_all(&mut runner);

    assert!(runner.game.players[0].battle_area[target.index as usize].is_suspended);
    assert_eq!(runner.hand_size(1), 1, "Needle Spray adds itself to hand");
}

#[test]
fn st4_16_security_returns_suspended_digimon_without_adding_to_hand() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST4-16")
        .expect("ST4-16 YAML loads")
        .add_card(digimon("ATTACKER", CardColor::Red, 4, 9000))
        .security(1, &["ST4-16"])
        .memory(10)
        .start();
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));

    runner.attack_player(attacker, 1, false);
    resolve_all(&mut runner);

    assert!(
        runner.game.players[0].battle_area.is_empty(),
        "Electro Shocker should return the suspended attacker to hand"
    );
    assert_eq!(
        runner.hand_size(1),
        0,
        "Electro Shocker does not add itself"
    );
}

#[test]
fn st4_15_and_st4_16_have_security_clauses() {
    let runner = DebugRunner::builder()
        .dsl_card("ST4-15")
        .expect("ST4-15 YAML loads")
        .dsl_card("ST4-16")
        .expect("ST4-16 YAML loads")
        .start();

    for card_id in ["ST4-15", "ST4-16"] {
        let card = runner.compiled_card(card_id).expect("compiled card");
        assert!(card.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSecurity)
        )));
    }
}

// --- Mandatory-selection faithfulness (printed text grants no "may"/"up to") ---
// Each printed clause is a forced directive; the engine must NOT surface a PASS
// to decline it when a legal target exists. We assert the pending selection is
// non-optional at the decision point.

#[test]
fn st4_03_reveal_add_is_mandatory_when_eligible() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST4-03")
        .expect("ST4-03 YAML loads")
        .add_card(digimon("GREEN-HIT", CardColor::Green, 4, 3000))
        .hand(0, &["ST4-03"])
        .deck(0, &["GREEN-HIT"])
        .memory(10)
        .start();
    make_p0_turn(&mut runner);

    assert!(runner.play(0, 0).is_some());
    let prompt = runner
        .pending_selection()
        .expect("ST4-03 reveals a green Digimon and prompts to add it");
    assert!(
        !prompt.is_optional,
        "\"add it to your hand\" is mandatory: an eligible green Digimon cannot be declined"
    );
}

#[test]
fn st4_10_reveal_add_is_mandatory_when_eligible() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST4-10")
        .expect("ST4-10 YAML loads")
        .add_card(digivolve_target("ST4-10", CardColor::Green, 5, 7000, 4))
        .add_card(digimon("BASE", CardColor::Green, 4, 4000))
        .add_card(digimon("LV6-HIT", CardColor::Green, 6, 11000))
        .add_card(option_card("DRAW", CardColor::Green))
        .hand(0, &["ST4-10"])
        .deck(0, &["LV6-HIT", "DRAW"])
        .memory(10)
        .start();
    make_p0_turn(&mut runner);
    let base = runner.place_on_field(0, "BASE", Some(0));

    assert!(digivolve_cheapest(
        &mut runner,
        0,
        0,
        base.index as usize,
        PlaySource::ByDigivolve
    ));
    let prompt = runner
        .pending_selection()
        .expect("ST4-10 reveals a level-6 Digimon and prompts to add it");
    assert!(
        !prompt.is_optional,
        "\"Add 1 level 6 or higher Digimon among them\" is mandatory when one exists"
    );
}

#[test]
fn st4_13_digi_burst_suspend_is_mandatory() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST4-13")
        .expect("ST4-13 YAML loads")
        .add_card(digimon("SRC-A", CardColor::Green, 4, 3000))
        .add_card(digimon("SRC-B", CardColor::Green, 4, 3000))
        .add_card(digimon("DEF", CardColor::Red, 4, 3000))
        .memory(10)
        .start();
    let hercules = runner.place_stack(0, &["SRC-A", "SRC-B", "ST4-13"]);
    runner.place_on_field(1, "DEF", Some(0));

    assert!(runner.game.activate_field_main(0, hercules.index as usize));
    // Step the Digi-Burst source-trash picks one action at a time (auto_resolve
    // would drain past the suspend selection we want to inspect).
    for _ in 0..8 {
        if runner.pending_kind() == Some(SelectionKind::OppField) {
            break;
        }
        let (player, action) = {
            let pending = runner
                .pending_selection()
                .expect("a Digi-Burst source selection before the suspend");
            (pending.selecting_player, pending.valid_action_ids[0])
        };
        runner
            .execute_action(player, action)
            .expect("resolve Digi-Burst source pick");
    }
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::OppField),
        "Digi-Burst must lead into the suspend target selection"
    );
    let prompt = runner.pending_selection().expect("suspend selection pending");
    assert!(
        !prompt.is_optional,
        "\"Suspend 1 of your opponent's Digimon\" is mandatory once Digi-Burst is paid"
    );
}

#[test]
fn st4_15_main_suspend_is_mandatory() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST4-15")
        .expect("ST4-15 YAML loads")
        .add_card(digimon("TARGET", CardColor::Red, 4, 3000))
        .hand(0, &["ST4-15"])
        .memory(10)
        .start();
    make_p0_turn(&mut runner);
    runner.place_on_field(1, "TARGET", Some(0));

    assert!(runner.play(0, 0).is_some());
    let prompt = runner
        .pending_selection()
        .expect("ST4-15 installs the suspend selection");
    assert!(
        !prompt.is_optional,
        "\"Suspend 1 of your opponent's Digimon\" is mandatory"
    );
}

#[test]
fn st4_16_main_return_is_mandatory() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST4-16")
        .expect("ST4-16 YAML loads")
        .add_card(digimon("TARGET", CardColor::Red, 4, 3000))
        .hand(0, &["ST4-16"])
        .memory(10)
        .start();
    make_p0_turn(&mut runner);
    let target = runner.place_on_field(1, "TARGET", Some(0));
    runner.game.players[1].battle_area[target.index as usize].is_suspended = true;

    assert!(runner.play(0, 0).is_some());
    let prompt = runner
        .pending_selection()
        .expect("ST4-16 installs the return selection");
    assert!(
        !prompt.is_optional,
        "\"Return 1 of your opponent's suspended Digimon\" is mandatory"
    );
}
