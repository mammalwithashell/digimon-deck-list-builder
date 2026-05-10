//! P-128 Cody Hida - Tamer, Black, Cost 3.
//!
//! Printed text from `data/cards.json` / `code/digimon-engine/cards/p/P-128.json`:
//!
//! [Start of Your Main Phase] If you have a Digimon with the [Free] trait,
//! gain 1 memory.
//!
//! [On Play] Activate 1 of the effects below:
//! - You may play 1 [Armadillomon] from your hand without paying the cost.
//! - 1 of your Digimon may digivolve into [Ankylomon] in the hand without
//!   paying the cost.
//!
//! Security Effect [Security] Play this card without paying the cost.

use std::path::Path;

use digimon_dsl::compiled::{CompiledCardKind, CompiledClause, CompiledColor, CompiledTiming};
use digimon_engine::action::space::{PASS, PLAY_HAND_START};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::selection::SelectionKind;

fn p_128_yaml() -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("cards/p/P-128.yaml"))
        .expect("P-128 YAML must exist at cards/p/P-128.yaml")
}

fn p_128_runner() -> DebugRunner {
    let yaml = p_128_yaml();
    DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-128 YAML must parse and compile")
        .memory(10)
        .start()
}

fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

fn free_digimon(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(3);
    card.dp = Some(2000);
    card.colors = vec![CardColor::Yellow];
    card.traits = vec!["Free".to_string()];
    card
}

fn non_free_digimon(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(3);
    card.dp = Some(2000);
    card.colors = vec![CardColor::Yellow];
    card.traits = vec!["Mammal".to_string()];
    card
}

fn armadillomon(id: &str) -> CardData {
    let mut card = free_digimon(id);
    card.card_name = "Armadillomon".to_string();
    card.play_cost = 3;
    card
}

fn not_armadillomon(id: &str) -> CardData {
    let mut card = free_digimon(id);
    card.card_name = "ShoeArmadillomon".to_string();
    card.play_cost = 3;
    card
}

fn ankylomon(id: &str) -> CardData {
    let mut card = free_digimon(id);
    card.card_name = "Ankylomon".to_string();
    card.level = Some(4);
    card.dp = Some(4000);
    card.play_cost = 4;
    card.evo_costs = vec![EvoCost {
        card_color: CardColor::Yellow as u8,
        level: 3,
        memory_cost: 3,
    }];
    card
}

fn not_ankylomon(id: &str) -> CardData {
    let mut card = ankylomon(id);
    card.card_name = "Not Ankylomon".to_string();
    card
}

fn battle_area_contains_card(runner: &DebugRunner, player: usize, card_id: &str) -> bool {
    runner.game.players[player]
        .battle_area
        .iter()
        .any(|perm| perm.top_card().card_id(&runner.game.card_data).eq(card_id))
}

fn hand_contains_card(runner: &DebugRunner, player: usize, card_id: &str) -> bool {
    runner.game.players[player]
        .hand
        .iter()
        .any(|card| card.card_id(&runner.game.card_data) == card_id)
}

#[test]
fn p_128_yaml_parses_and_compiles() {
    let _runner = p_128_runner();
}

#[test]
fn p_128_has_printed_metadata_and_three_supported_clauses() {
    let runner = p_128_runner();
    let compiled = runner.compiled_card("P-128").expect("P-128 compiled");

    assert_eq!(compiled.name, "Cody Hida");
    assert_eq!(compiled.kind, CompiledCardKind::Tamer);
    assert_eq!(compiled.color, vec![CompiledColor::Black]);
    assert_eq!(compiled.cost, Some(3));

    let timings: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|clause| match clause {
            CompiledClause::Triggered(triggered) => Some(triggered.when.clone()),
            _ => None,
        })
        .collect();

    assert!(
        timings
            .iter()
            .any(|when| when.contains(&CompiledTiming::StartOfYourMainPhase)),
        "P-128 must include the printed start-of-main memory clause"
    );
    assert!(
        timings
            .iter()
            .any(|when| when.contains(&CompiledTiming::OnPlay)),
        "P-128 must include the printed modal On Play clause"
    );
    assert!(
        timings
            .iter()
            .any(|when| when.contains(&CompiledTiming::OnSecurity)),
        "P-128 must include the printed Security play clause"
    );
}

#[test]
fn p_128_start_main_gains_memory_if_you_have_free_digimon() {
    let yaml = p_128_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-128 YAML parses")
        .add_card(free_digimon("FREE-DIGIMON"))
        .add_card(filler("FILL"))
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(0)
        .start();

    runner.place_on_field(0, "P-128", Some(0));
    runner.place_on_field(0, "FREE-DIGIMON", Some(0));
    runner.game.memory = 0;

    runner.end_turn();
    runner.end_turn();

    assert_eq!(
        runner.memory(),
        1,
        "Cody should gain exactly 1 memory at start of main with a [Free] Digimon"
    );
}

#[test]
fn p_128_start_main_does_not_gain_memory_without_free_digimon() {
    let yaml = p_128_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-128 YAML parses")
        .add_card(non_free_digimon("NON-FREE"))
        .add_card(filler("FILL"))
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(0)
        .start();

    runner.place_on_field(0, "P-128", Some(0));
    runner.place_on_field(0, "NON-FREE", Some(0));
    runner.game.memory = 0;

    runner.end_turn();
    runner.end_turn();

    assert_eq!(
        runner.memory(),
        0,
        "Cody must not gain memory when no Digimon has the [Free] trait"
    );
}

#[test]
fn p_128_on_play_offers_visible_choice_between_armadillomon_and_ankylomon_modes() {
    let yaml = p_128_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-128 YAML parses")
        .add_card(armadillomon("ARMADILLOMON-HAND"))
        .add_card(ankylomon("ANKYLOMON-HAND"))
        .hand(0, &["P-128", "ARMADILLOMON-HAND", "ANKYLOMON-HAND"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Cody Hida");

    let view = runner
        .pending_selection_view()
        .expect("On Play should install an effect-choice prompt");
    assert_eq!(view.kind, SelectionKind::EffectChoice);
    let labels: Vec<&str> = view
        .effect_choices
        .as_ref()
        .expect("effect choices")
        .iter()
        .map(|choice| choice.label.as_str())
        .collect();
    assert_eq!(
        labels,
        vec!["Play Armadillomon", "Digivolve into Ankylomon"],
        "P-128 must expose both printed On Play branches as a visible choice"
    );
}

#[test]
fn p_128_on_play_armadillomon_branch_plays_exact_name_from_hand_for_free() {
    let yaml = p_128_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-128 YAML parses")
        .add_card(armadillomon("ARMADILLOMON-HAND"))
        .add_card(not_armadillomon("NOT-ARMADILLOMON"))
        .hand(0, &["P-128", "ARMADILLOMON-HAND", "NOT-ARMADILLOMON"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Cody Hida");
    let memory_after_cody_cost = runner.memory();
    let field_before = runner.battle_area_size(0);

    runner
        .execute_branch(0)
        .expect("choose Armadillomon branch");
    assert_eq!(runner.pending_kind(), Some(SelectionKind::Hand));
    assert!(
        runner.pending_is_optional(),
        "printed Armadillomon play says may, so PASS must be legal"
    );
    assert_eq!(
        runner
            .pending_selection_view()
            .expect("hand selection")
            .valid_action_ids,
        vec![PLAY_HAND_START],
        "only exact-name Armadillomon should be selectable"
    );

    runner
        .execute_action(0, PLAY_HAND_START)
        .expect("select Armadillomon");
    runner.auto_resolve().expect("finish On Play");

    assert!(battle_area_contains_card(&runner, 0, "ARMADILLOMON-HAND"));
    assert!(!battle_area_contains_card(&runner, 0, "NOT-ARMADILLOMON"));
    assert_eq!(runner.battle_area_size(0), field_before + 1);
    assert_eq!(
        runner.memory(),
        memory_after_cody_cost,
        "play_from_hand_free must not spend Armadillomon's play cost"
    );
}

#[test]
fn p_128_on_play_armadillomon_branch_can_be_declined() {
    let yaml = p_128_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-128 YAML parses")
        .add_card(armadillomon("ARMADILLOMON-HAND"))
        .hand(0, &["P-128", "ARMADILLOMON-HAND"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Cody Hida");
    runner
        .execute_branch(0)
        .expect("choose Armadillomon branch");
    let hand_before = runner.hand_size(0);
    let field_before = runner.battle_area_size(0);

    runner
        .execute_action(0, PASS)
        .expect("decline Armadillomon play");
    runner.auto_resolve().expect("finish declined branch");

    assert_eq!(runner.hand_size(0), hand_before);
    assert_eq!(runner.battle_area_size(0), field_before);
    assert!(hand_contains_card(&runner, 0, "ARMADILLOMON-HAND"));
}

#[test]
fn p_128_on_play_ankylomon_branch_digivolves_selected_digimon_for_free() {
    let yaml = p_128_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-128 YAML parses")
        .add_card(non_free_digimon("YELLOW-BASE"))
        .add_card(ankylomon("ANKYLOMON-HAND"))
        .add_card(not_ankylomon("NOT-ANKYLOMON"))
        .add_card(filler("FILL"))
        .hand(0, &["P-128", "ANKYLOMON-HAND", "NOT-ANKYLOMON"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let base = runner.place_on_field(0, "YELLOW-BASE", Some(0));
    runner.play(0, 0).expect("play Cody Hida");
    let memory_after_cody_cost = runner.memory();

    runner
        .execute_branch(1)
        .expect("choose Ankylomon digivolve branch");
    assert_eq!(runner.pending_kind(), Some(SelectionKind::OwnField));
    assert!(
        runner.pending_is_optional(),
        "printed Ankylomon digivolve says may, so the base pick must allow PASS"
    );
    assert_eq!(
        runner.pending_action_count(),
        1,
        "only Digimon, not Cody, should be offered as digivolve bases"
    );
    let base_action = runner.pending_selection_view().unwrap().valid_action_ids[0];
    runner
        .execute_action(0, base_action)
        .expect("choose Digimon to digivolve");

    assert_eq!(runner.pending_kind(), Some(SelectionKind::Hand));
    assert!(
        !runner.pending_is_optional(),
        "after choosing a base, the Ankylomon card pick is mandatory"
    );
    assert_eq!(
        runner.pending_selection_view().unwrap().valid_action_ids,
        vec![PLAY_HAND_START],
        "only exact-name Ankylomon should be selectable from hand"
    );
    runner
        .execute_action(0, PLAY_HAND_START)
        .expect("choose Ankylomon from hand");
    runner.auto_resolve().expect("finish digivolve branch");

    let carrier = &runner.game.players[0].battle_area[base.index as usize];
    assert_eq!(
        carrier.top_card().card_id(&runner.game.card_data),
        "ANKYLOMON-HAND",
        "selected Ankylomon should become the top card of the chosen stack"
    );
    assert!(!hand_contains_card(&runner, 0, "ANKYLOMON-HAND"));
    assert_eq!(
        runner.memory(),
        memory_after_cody_cost,
        "effect_initiated_digivolve cost 0 must not spend Ankylomon's digivolution cost"
    );
}

#[test]
fn p_128_on_play_ankylomon_branch_can_be_declined_after_mode_choice() {
    let yaml = p_128_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-128 YAML parses")
        .add_card(non_free_digimon("YELLOW-BASE"))
        .add_card(ankylomon("ANKYLOMON-HAND"))
        .hand(0, &["P-128", "ANKYLOMON-HAND"])
        .memory(10)
        .start();

    let base = runner.place_on_field(0, "YELLOW-BASE", Some(0));
    runner.play(0, 0).expect("play Cody Hida");
    runner
        .execute_branch(1)
        .expect("choose Ankylomon digivolve branch");
    assert_eq!(runner.pending_kind(), Some(SelectionKind::OwnField));

    runner
        .execute_action(0, PASS)
        .expect("decline Ankylomon digivolve");
    runner.auto_resolve().expect("finish declined branch");

    assert!(
        runner.pending_selection().is_none(),
        "declining the optional base pick must not continue to the Ankylomon hand prompt"
    );
    assert_eq!(
        runner.game.players[0].battle_area[base.index as usize]
            .top_card()
            .card_id(&runner.game.card_data),
        "YELLOW-BASE"
    );
    assert!(hand_contains_card(&runner, 0, "ANKYLOMON-HAND"));
}

#[test]
fn p_128_security_plays_itself_without_paying_cost() {
    let yaml = p_128_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-128 YAML parses")
        .add_card(free_digimon("ATTACKER"))
        .security(1, &["P-128"])
        .memory(10)
        .start();
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));

    runner.attack_player(attacker, 1, false);
    runner.auto_resolve().expect("resolve security play");

    assert!(battle_area_contains_card(&runner, 1, "P-128"));
    assert_eq!(
        runner.security_count(1),
        0,
        "Cody should leave security when played by the Security effect"
    );
}
