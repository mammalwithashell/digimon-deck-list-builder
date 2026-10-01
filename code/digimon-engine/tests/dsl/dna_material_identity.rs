//! G-DNA-MATERIAL-TREATED-AS-FOR-TARGET — the `dna_material_identity:` aura
//! payload: "[All Turns] This Digimon is also treated as Lv.6 [Slayerdramon]
//! for [Examon]'s DNA digivolution" (EX13-021 / EX13-041).
//!
//! The treatment is RESULT-scoped: the carrier gains an extra level / name
//! only while a DNA (or Blast-DNA) material requirement is evaluated for a
//! result card with the named name. DCGO `AddJogressLevelsClass` +
//! `Permanent.Names_ForDNA`.

use digimon_dsl::compiled::{CompiledClause, CompiledDeclarativeClause};
use digimon_engine::action::space::{DNA_DIGIVOLVE_START, PLAY_HAND_START};
use digimon_engine::debug_runner::{make_test_card_with_level, DebugRunner, DebugRunnerBuilder};
use digimon_engine::dsl::spec::CardSpec;
use digimon_engine::enums::{CardColor, GamePhase, ModifierType};
use digimon_engine::permanent::PermanentHandle;

const WING: &str = r#"
card: DMI-WING
name: Wing Test
kind: digimon
level: 5
color: [blue]
cost: 7
dp: 7000
effects:
  - kind: aura
    target: {}
    dna_material_identity:
      for_result: TestExa
      level: 6
      name: Slayerdramon
"#;

const PLAIN_BLUE5: &str = r#"
card: DMI-BLUE5
name: Plain Blue Five
kind: digimon
level: 5
color: [blue]
cost: 7
dp: 7000
"#;

const GREEN6: &str = r#"
card: DMI-GREEN6
name: Green Six
kind: digimon
level: 6
color: [green]
cost: 10
dp: 11000
"#;

const EXA: &str = r#"
card: DMI-EXA
name: TestExa
kind: digimon
level: 7
color: [green, blue]
cost: 15
dp: 15000
alt_paths:
  - kind: dna_digivolve
    materials:
      - { level_eq: 6, color_is: green }
      - { level_eq: 6, color_is: blue }
    cost: 0
"#;

const OTHER: &str = r#"
card: DMI-OTHER
name: OtherMon
kind: digimon
level: 7
color: [green, blue]
cost: 15
dp: 15000
alt_paths:
  - kind: dna_digivolve
    materials:
      - { level_eq: 6, color_is: green }
      - { level_eq: 6, color_is: blue }
    cost: 0
"#;

/// Name-recipe result (the BT20-045 Blast-DNA shape, but as a Main-phase DNA):
/// [Breakdramon] + [Slayerdramon].
const NAMED_EXA: &str = r#"
card: DMI-NAMED-EXA
name: TestExa
kind: digimon
level: 7
color: [green, blue]
cost: 15
dp: 15000
alt_paths:
  - kind: dna_digivolve
    materials:
      - { level_eq: 6, name_is: Breakdramon }
      - { level_eq: 6, name_is: Slayerdramon }
    cost: 0
"#;

/// Blast-DNA result (registered `blast_dna_digivolve` path, predicate matched).
const BLAST_EXA: &str = r#"
card: DMI-BLAST-EXA
name: TestExa
kind: digimon
level: 7
color: [green, blue]
cost: 15
dp: 15000
alt_paths:
  - kind: blast_dna_digivolve
    materials:
      - { kind: digimon, name_is: Breakdramon }
      - { kind: digimon, name_is: Slayerdramon }
    cost: 0
    stacks_unsuspended: true
effects:
  - kind: grant_keyword
    keyword: BlastDigivolve
"#;

fn builder() -> DebugRunnerBuilder {
    let mut b = DebugRunner::builder();
    for y in [WING, PLAIN_BLUE5, GREEN6, EXA, OTHER, NAMED_EXA, BLAST_EXA] {
        b = b.from_dsl_yaml(y).expect("inline DSL compiles");
    }
    b
}

fn main_runner(hand: &[&str], field: &[&str]) -> DebugRunner {
    let mut runner = builder().hand(0, hand).memory(10).start();
    runner.game.current_phase = GamePhase::Main;
    for id in field {
        runner.place_on_field(0, id, Some(0));
    }
    runner.game.tick_declarative_effects();
    runner
}

fn handle(index: u8) -> PermanentHandle {
    PermanentHandle { player: 0, index }
}

// ─── Parse / compile / validate ──────────────────────────────────────────────

#[test]
fn dna_material_identity_compiles_onto_the_aura() {
    let spec: CardSpec = serde_yml::from_str(WING).expect("parse");
    let compiled = digimon_dsl::compile::compile(&spec).expect("compile");
    let found = compiled.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
                dna_material_identity: Some(d),
                ..
            }) if d.for_result == "TestExa" && d.level == Some(6)
                && d.name.as_deref() == Some("Slayerdramon")
        )
    });
    assert!(found, "aura carries the compiled dna_material_identity payload");
}

fn compile_errors(yaml: &str) -> String {
    use digimon_engine::dsl::raw_rust_registry::StubRegistry;
    use digimon_engine::dsl::validator::{validate, ValidationContext};
    let spec: CardSpec = serde_yml::from_str(yaml).expect("parse");
    let reg = StubRegistry::empty();
    match validate(&spec, &ValidationContext { raw_rust: &reg }) {
        Ok(_) => String::new(),
        Err(errors) => errors
            .into_iter()
            .map(|e| format!("{}: {}", e.path, e.message))
            .collect::<Vec<_>>()
            .join("\n"),
    }
}

#[test]
fn dna_material_identity_requires_a_level_or_name() {
    let yaml = WING
        .replace("      level: 6\n", "")
        .replace("      name: Slayerdramon\n", "");
    assert!(
        compile_errors(&yaml).contains("needs a level and/or a name"),
        "{}",
        compile_errors(&yaml)
    );
}

#[test]
fn dna_material_identity_is_self_aura_only() {
    let yaml = WING.replace("    target: {}", "    target: { kind: digimon }");
    assert!(
        compile_errors(&yaml).contains("self-aura payload"),
        "{}",
        compile_errors(&yaml)
    );
}

#[test]
fn bare_dna_material_identity_modifier_string_is_rejected() {
    let yaml = r#"
card: DMI-BARE
name: Bare
kind: digimon
level: 5
color: [blue]
cost: 7
dp: 7000
effects:
  - kind: aura
    target: {}
    modifier: DnaMaterialIdentity
"#;
    assert!(
        compile_errors(yaml).contains("needs its payload"),
        "{}",
        compile_errors(yaml)
    );
}

// ─── Runtime identity query ──────────────────────────────────────────────────

#[test]
fn extras_apply_only_for_the_named_result() {
    let runner = main_runner(&["DMI-EXA", "DMI-OTHER"], &["DMI-WING"]);
    let exa = runner.game.players[0].hand[0].clone();
    let other = runner.game.players[0].hand[1].clone();
    assert!(runner
        .modifiers()
        .has(handle(0), ModifierType::DnaMaterialIdentity));
    let for_exa = runner.game.dna_material_extras(handle(0), &exa);
    assert_eq!(for_exa.levels, vec![6]);
    assert_eq!(for_exa.names, vec!["Slayerdramon".to_string()]);
    assert!(
        runner.game.dna_material_extras(handle(0), &other).is_empty(),
        "no treatment for a different DNA result"
    );
}

#[test]
fn treatment_does_not_change_the_printed_level_or_name_elsewhere() {
    let runner = main_runner(&[], &["DMI-WING"]);
    let perm = &runner.game.players[0].battle_area[0];
    assert_eq!(perm.level(&runner.game.card_data), Some(5));
    assert_eq!(
        perm.top_card().card_names(&runner.game.card_data),
        vec!["Wing Test"]
    );
}

// ─── Main-phase DNA (printed DnaCost table) ──────────────────────────────────

#[test]
fn lv5_carrier_is_a_lv6_material_for_the_named_result() {
    let mut runner = main_runner(&["DMI-EXA"], &["DMI-WING", "DMI-GREEN6"]);
    assert!(
        runner.game.has_valid_dna_route_for_hand_card(0, 0),
        "Lv.5 blue carrier + Lv.6 green satisfy Green Lv.6 + Blue Lv.6 for TestExa"
    );
    let mask = digimon_engine::action::mask::build_action_mask(&runner.game, 0);
    assert_eq!(mask[DNA_DIGIVOLVE_START as usize], 1.0, "DNA action is exposed");
    assert!(runner.game.initiate_dna_digivolve(0, 0));
    runner.game.resolve_selection(0, 0).expect("first material");
    runner.game.resolve_selection(0, 1).expect("second material");
    let field = &runner.game.players[0].battle_area;
    assert_eq!(field.len(), 1, "two materials merged into one permanent");
    let ids: Vec<&str> = field[0]
        .card_sources
        .iter()
        .map(|c| c.card_id(&runner.game.card_data))
        .collect();
    assert_eq!(ids.last(), Some(&"DMI-EXA"), "TestExa on top");
    assert!(ids.contains(&"DMI-WING") && ids.contains(&"DMI-GREEN6"));
}

#[test]
fn treatment_does_not_apply_to_another_dna_result() {
    let runner = main_runner(&["DMI-OTHER"], &["DMI-WING", "DMI-GREEN6"]);
    assert!(
        !runner.game.has_valid_dna_route_for_hand_card(0, 0),
        "OtherMon's recipe still needs a printed Lv.6 blue"
    );
}

#[test]
fn a_plain_lv5_is_not_a_lv6_material() {
    let runner = main_runner(&["DMI-EXA"], &["DMI-BLUE5", "DMI-GREEN6"]);
    assert!(!runner.game.has_valid_dna_route_for_hand_card(0, 0));
}

#[test]
fn treated_name_satisfies_a_named_dna_recipe() {
    let mut brk = make_test_card_with_level("DMI-BREAK", "Breakdramon", 6);
    brk.colors = vec![CardColor::Green];
    let mut runner = builder()
        .add_card(brk)
        .hand(0, &["DMI-NAMED-EXA"])
        .memory(10)
        .start();
    runner.game.current_phase = GamePhase::Main;
    runner.place_on_field(0, "DMI-WING", Some(0));
    runner.place_on_field(0, "DMI-BREAK", Some(0));
    runner.game.tick_declarative_effects();
    assert!(
        runner.game.has_valid_dna_route_for_hand_card(0, 0),
        "carrier is Lv.6 [Slayerdramon] for TestExa's [Breakdramon] + [Slayerdramon] recipe"
    );
}

#[test]
fn treatment_ends_when_the_carrier_is_no_longer_the_top_card() {
    // A face (non-inherited) static: once something digivolves on top of the
    // carrier, the stack is no longer "this Digimon" with the printed clause.
    let mut runner = main_runner(&["DMI-EXA"], &[]);
    // Stack [Wing Test (bottom), Plain Blue Five (top)] — Lv.5 blue on top.
    runner.place_field_stack(0, &["DMI-WING", "DMI-BLUE5"], false, 0);
    runner.place_on_field(0, "DMI-GREEN6", Some(0));
    runner.game.tick_declarative_effects();
    assert!(
        !runner.game.has_valid_dna_route_for_hand_card(0, 0),
        "an inherited-position Wing Test grants nothing"
    );
}

// ─── Blast DNA (registered predicate path) ───────────────────────────────────

#[test]
fn treated_name_satisfies_a_registered_blast_dna_field_material() {
    let mut brk = make_test_card_with_level("DMI-BREAK", "Breakdramon", 6);
    brk.colors = vec![CardColor::Green];
    let mut attacker = make_test_card_with_level("DMI-ATK", "Attacker", 6);
    attacker.dp = Some(1000);
    let mut runner = builder()
        .add_card(brk)
        .add_card(attacker)
        .hand(1, &["DMI-BLAST-EXA", "DMI-BREAK"])
        .start();
    let atk = runner.place_on_field(0, "DMI-ATK", Some(0));
    let wing = runner.place_on_field(1, "DMI-WING", Some(0));
    runner.game.tick_declarative_effects();

    let result = runner.attack_digimon(atk, wing, false);
    assert_eq!(result, digimon_engine::combat::AttackResult::InProgress);
    assert_eq!(runner.current_phase(), GamePhase::CounterTiming);
    let prompt = runner.pending_selection().expect("counter window");
    assert!(
        prompt.valid_action_ids.contains(&DNA_DIGIVOLVE_START),
        "Wing Test (as [Slayerdramon]) + Breakdramon in hand enable Blast DNA: {:?}",
        prompt.valid_action_ids
    );
    runner.execute_action(1, DNA_DIGIVOLVE_START).expect("Blast DNA");
    runner.execute_action(1, 0).expect("Wing Test as field material");
    runner
        .execute_action(1, PLAY_HAND_START + 1)
        .expect("Breakdramon as hand material");
    let stacks: Vec<Vec<String>> = runner.game.players[1]
        .battle_area
        .iter()
        .map(|p| {
            p.card_sources
                .iter()
                .map(|c| c.card_id(&runner.game.card_data).to_string())
                .collect()
        })
        .collect();
    assert_eq!(
        stacks,
        vec![vec![
            "DMI-WING".to_string(),
            "DMI-BREAK".to_string(),
            "DMI-BLAST-EXA".to_string()
        ]],
        "Blast DNA stacked Wing Test + Breakdramon under TestExa; trash: {:?}",
        runner.game.players[1]
            .trash
            .iter()
            .map(|c| c.card_id(&runner.game.card_data).to_string())
            .collect::<Vec<_>>()
    );
}

#[test]
fn blast_dna_without_the_treatment_is_not_offered() {
    let mut brk = make_test_card_with_level("DMI-BREAK", "Breakdramon", 6);
    brk.colors = vec![CardColor::Green];
    let mut attacker = make_test_card_with_level("DMI-ATK", "Attacker", 6);
    attacker.dp = Some(1000);
    let mut runner = builder()
        .add_card(brk)
        .add_card(attacker)
        .hand(1, &["DMI-BLAST-EXA", "DMI-BREAK"])
        .start();
    let atk = runner.place_on_field(0, "DMI-ATK", Some(0));
    let plain = runner.place_on_field(1, "DMI-BLUE5", Some(0));
    runner.game.tick_declarative_effects();
    let _ = runner.attack_digimon(atk, plain, false);
    let offered = runner
        .pending_selection()
        .map(|p| p.valid_action_ids.contains(&DNA_DIGIVOLVE_START))
        .unwrap_or(false);
    assert!(!offered, "a plain Lv.5 is not [Slayerdramon]");
}
