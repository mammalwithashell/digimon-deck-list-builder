//! Effect-digivolve requirement waivers (2026-10-01):
//!
//! - **G-DIGIVOLVE-IGNORE-LEVEL-PRINTED-COST** — "digivolve into X …
//!   ignoring level" (DCGO `CardEffectCommons.IgnoreRequirement.Level`,
//!   `CardSource.EvoCosts` / `AddDigivolutionRequirement.GetEvoCost`): only
//!   the base's LEVEL requirement drops. Printed circles still need a colour
//!   match and their printed memory cost is paid (then the effect's
//!   `CostDelta`); alt-digivolve paths are evaluated with their `from:`
//!   level leaves waived. DSL: `effect_initiated_digivolve { ignore_level:
//!   true }`. Driver: EX13-071 Richard Sampson.
//! - **G-TAMER-DIGIVOLVE-INTO-DIGIMON** — "this Tamer may digivolve into
//!   [X] … for a digivolution cost of N, ignoring digivolution
//!   requirements": a level-less Tamer base is accepted when requirements
//!   are waived; the Tamer and the cards under it become the new Digimon's
//!   digivolution cards (DCGO stacks onto `card.PermanentOfThisCard()`,
//!   BT22_090.cs). Driver: EX13-074 Rie Kishibe.

use digimon_dsl::compiled::{CompiledBindingRef, CompiledClause, CompiledCostDelta, CompiledStep};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_step;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind, CardSourceRef, CostDelta, PlaySource};
use digimon_engine::permanent::PermanentHandle;

const YELLOW: u8 = 2;

fn digimon(id: &str, level: u8, color: CardColor, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.colors = vec![color];
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

/// Lv.6 yellow result with ONE printed circle: Yellow Lv.5, cost `cost`.
fn lv6_yellow(id: &str, cost: u16) -> CardData {
    let mut c = digimon(id, 6, CardColor::Yellow, &[]);
    c.play_cost = 12;
    c.evo_costs = vec![EvoCost {
        card_color: YELLOW,
        level: 5,
        memory_cost: cost,
    }];
    c
}

fn tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.colors = vec![CardColor::Purple];
    c
}

fn base_builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .add_card(digimon("Y3", 3, CardColor::Yellow, &[]))
        .add_card(digimon("R3", 3, CardColor::Red, &[]))
        .add_card(digimon("R3-HB", 3, CardColor::Red, &["Holy Beast"]))
        .add_card(digimon("FILL", 3, CardColor::Red, &[]))
        .add_card(lv6_yellow("KEN", 3))
        .add_card(tamer("TAMER"))
}

fn top_id(r: &DebugRunner, h: PermanentHandle) -> String {
    r.game.players[0].battle_area[h.index as usize]
        .top_card()
        .card_id(&r.game.card_data)
        .to_string()
}

fn stack_ids(r: &DebugRunner, h: PermanentHandle) -> Vec<String> {
    r.game.players[0].battle_area[h.index as usize]
        .card_sources
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

fn ignore_level(r: &mut DebugRunner, from: CardSourceRef, target: PermanentHandle, delta: CostDelta) -> bool {
    r.game
        .effect_initiated_digivolve_from_source_ignore_level(0, from, target, delta, PlaySource::ByEffect)
}

// ─── Ignore level: printed circles ───────────────────────────────────────────

#[test]
fn ignore_level_pays_the_colour_matching_printed_cost_then_the_delta() {
    let mut r = base_builder()
        .hand(0, &["KEN"])
        .deck(0, &["FILL"; 4])
        .memory(5)
        .start();
    let base = r.place_on_field(0, "Y3", Some(0));
    let hand0 = r.hand_size(0);
    assert!(ignore_level(&mut r, CardSourceRef::Hand(0, 0), base, CostDelta::Reduce(1)));
    assert_eq!(top_id(&r, base), "KEN", "Lv.3 base digivolved into the Lv.6 card");
    assert_eq!(r.memory(), 5 - (3 - 1), "printed Yellow Lv.5 cost 3, reduced by 1");
    assert_eq!(r.hand_size(0), hand0 - 1 + 1, "result card left hand; digivolution draw 1");
}

#[test]
fn ignore_level_keeps_the_colour_requirement() {
    let mut r = base_builder()
        .hand(0, &["KEN"])
        .deck(0, &["FILL"; 4])
        .memory(5)
        .start();
    let base = r.place_on_field(0, "R3", Some(0));
    assert!(
        !ignore_level(&mut r, CardSourceRef::Hand(0, 0), base, CostDelta::Reduce(1)),
        "a red base matches no yellow circle even with level waived"
    );
    assert_eq!(top_id(&r, base), "R3");
    assert_eq!(r.memory(), 5);
    assert_eq!(r.hand_size(0), 1, "result card stays in hand");
}

#[test]
fn plain_effect_digivolve_still_enforces_level() {
    let mut r = base_builder()
        .hand(0, &["KEN"])
        .deck(0, &["FILL"; 4])
        .memory(5)
        .start();
    let base = r.place_on_field(0, "Y3", Some(0));
    assert!(!r.game.effect_initiated_digivolve_from_source(
        0,
        CardSourceRef::Hand(0, 0),
        base,
        CostDelta::Reduce(1),
        false,
        PlaySource::ByEffect,
    ));
    assert_eq!(top_id(&r, base), "Y3");
}

#[test]
fn ignore_level_works_from_trash() {
    let mut r = base_builder()
        .deck(0, &["FILL"; 4])
        .memory(5)
        .start();
    r.inject_trash(0, "KEN");
    let base = r.place_on_field(0, "Y3", Some(0));
    let t = r.game.players[0].trash.len() - 1;
    assert!(ignore_level(&mut r, CardSourceRef::Trash(0, t), base, CostDelta::Reduce(1)));
    assert_eq!(top_id(&r, base), "KEN");
    assert_eq!(r.memory(), 3);
}

// ─── Ignore level: DSL alt-digivolve paths ──────────────────────────────────

const ALT_KEN_YAML: &str = r#"
card: ALT-KEN
name: Alt Kentaurosmon
kind: digimon
level: 6
color: [yellow]
cost: 12
dp: 12000
alt_paths:
  - kind: digivolve
    from: { level_eq: 5, trait_has: "Holy Beast" }
    cost: 2
"#;

fn alt_runner() -> DebugRunner {
    let mut r = base_builder()
        .from_dsl_yaml(ALT_KEN_YAML)
        .expect("alt card compiles")
        .hand(0, &["ALT-KEN"])
        .deck(0, &["FILL"; 4])
        .memory(5)
        .start();
    // Printed circle (the YAML loader carries none): Yellow Lv.5 cost 4.
    let card = r
        .game
        .card_data
        .iter_mut()
        .find(|c| c.card_id == "ALT-KEN")
        .expect("ALT-KEN");
    card.evo_costs = vec![EvoCost {
        card_color: YELLOW,
        level: 5,
        memory_cost: 4,
    }];
    r
}

#[test]
fn ignore_level_strips_the_alt_path_level_leaf_but_keeps_its_trait_gate() {
    // Red Lv.3 [Holy Beast]: no colour match for the printed circle, but the
    // "Lv.5 w/[Holy Beast]: Cost 2" path applies once its level leaf drops.
    let mut r = alt_runner();
    let base = r.place_on_field(0, "R3-HB", Some(0));
    assert!(ignore_level(&mut r, CardSourceRef::Hand(0, 0), base, CostDelta::Reduce(1)));
    assert_eq!(top_id(&r, base), "ALT-KEN");
    assert_eq!(r.memory(), 5 - (2 - 1));
}

#[test]
fn ignore_level_alt_path_trait_gate_still_applies() {
    // Yellow Lv.3 without the trait: only the printed Yellow circle (4) fits.
    let mut r = alt_runner();
    let base = r.place_on_field(0, "Y3", Some(0));
    assert!(ignore_level(&mut r, CardSourceRef::Hand(0, 0), base, CostDelta::Reduce(1)));
    assert_eq!(r.memory(), 5 - (4 - 1));
}

#[test]
fn ignore_level_alt_path_rejects_a_base_matching_nothing() {
    let mut r = alt_runner();
    let base = r.place_on_field(0, "R3", Some(0));
    assert!(!ignore_level(&mut r, CardSourceRef::Hand(0, 0), base, CostDelta::Reduce(1)));
    assert_eq!(r.memory(), 5);
}

#[test]
fn without_subject_level_leaves_strips_positive_positions_only() {
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(
        r#"
card: P-STRIP
name: Strip
kind: digimon
level: 6
color: [yellow]
cost: 12
dp: 12000
alt_paths:
  - kind: digivolve
    from:
      all_of:
        - level_eq: 5
        - any_of: [ { level_gte: 4 }, { trait_has: X } ]
        - none_of: [ { level_eq: 3 } ]
    cost: 2
"#,
    )
    .expect("parse");
    let compiled = digimon_dsl::compile::compile(&spec).expect("compile");
    let from = compiled.alt_paths[0].from.as_ref().expect("from");
    let stripped = from.without_subject_level_leaves();
    assert_eq!(stripped.all_of[0].level_eq, None);
    assert!(stripped.all_of[1].any_of[0].level_gte.is_none());
    assert_eq!(stripped.all_of[1].any_of[1].trait_has.as_deref(), Some("X"));
    assert_eq!(
        stripped.all_of[2].none_of[0].level_eq,
        Some(3),
        "negated level leaves are kept"
    );
}

// ─── Tamer base (ignoring digivolution requirements) ─────────────────────────

#[test]
fn ignore_requirements_digivolves_a_tamer_and_keeps_its_cards_underneath() {
    let mut r = base_builder()
        .deck(0, &["FILL"; 4])
        .memory(5)
        .start();
    r.inject_trash(0, "KEN");
    let t = r.place_on_field(0, "TAMER", Some(0));
    r.push_source(t, "FILL");
    let trash_i = r.game.players[0].trash.len() - 1;
    let hand0 = r.hand_size(0);
    assert!(r.game.effect_initiated_digivolve_from_source_ignore_requirements(
        0,
        CardSourceRef::Trash(0, trash_i),
        t,
        CostDelta::Fixed(3),
        PlaySource::ByEffect,
    ));
    assert_eq!(
        stack_ids(&r, t),
        vec!["FILL".to_string(), "TAMER".to_string(), "KEN".to_string()],
        "Tamer + its under-card become the Digimon's digivolution cards"
    );
    let perm = &r.game.players[0].battle_area[t.index as usize];
    let identity = perm.synth_identity(&r.game.card_data, &r.game.modifiers, t);
    assert_eq!(identity.kind, CardKind::Digimon, "the permanent is now a Digimon");
    assert_eq!(r.memory(), 2, "fixed digivolution cost 3");
    assert_eq!(r.hand_size(0), hand0 + 1, "digivolution draw");
}

#[test]
fn a_tamer_base_still_needs_a_requirement_waiver() {
    let mut r = base_builder()
        .hand(0, &["KEN"])
        .deck(0, &["FILL"; 4])
        .memory(5)
        .start();
    let t = r.place_on_field(0, "TAMER", Some(0));
    assert!(!r.game.effect_initiated_digivolve_from_source(
        0,
        CardSourceRef::Hand(0, 0),
        t,
        CostDelta::Fixed(3),
        false,
        PlaySource::ByEffect,
    ));
    assert!(
        !ignore_level(&mut r, CardSourceRef::Hand(0, 0), t, CostDelta::Fixed(3)),
        "ignoring level alone gives a Tamer no colour-matching Digimon circle"
    );
    assert_eq!(top_id(&r, t), "TAMER");
    assert_eq!(r.memory(), 5);
}

// ─── DSL surface ─────────────────────────────────────────────────────────────

fn compile_step(extra: &str) -> Result<digimon_dsl::compiled::CompiledCard, String> {
    let yaml = format!(
        r#"
card: IL-DIGI
name: "Ignore Level Digivolve"
kind: option
color: [yellow]
cost: 0
effects:
  - when: main_from_hand
    process:
      - effect_initiated_digivolve:
          target: target
          source: picked
          cost: {{ reduce: 1 }}
{extra}
"#
    );
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(&yaml).map_err(|e| e.to_string())?;
    digimon_dsl::compile::compile(&spec).map_err(|e| format!("{e:?}"))
}

#[test]
fn dsl_ignore_level_parses_and_compiles() {
    let compiled = compile_step("          ignore_level: true").expect("compiles");
    let CompiledClause::Triggered(t) = &compiled.effects[0] else {
        panic!("triggered");
    };
    assert!(matches!(
        &t.process[0],
        CompiledStep::EffectInitiatedDigivolve {
            ignore_level: true,
            ignore_requirements: false,
            cost: CompiledCostDelta::Reduce(1),
            ..
        }
    ));
}

#[test]
fn dsl_ignore_level_and_ignore_requirements_are_mutually_exclusive() {
    let err = compile_step("          ignore_level: true\n          ignore_requirements: true")
        .expect_err("both waivers is a validation error");
    assert!(err.contains("mutually exclusive"), "{err}");
}

#[test]
fn dsl_ignore_level_step_lowers_to_the_ignore_level_digivolve() {
    let mut r = base_builder()
        .hand(0, &["KEN"])
        .deck(0, &["FILL"; 4])
        .memory(5)
        .start();
    let base = r.place_on_field(0, "Y3", Some(0));
    let src = r.game.players[0].battle_area[0].top_card().handle();
    let mut bindings = Bindings::new();
    bindings.insert_permanent("target", base);
    bindings.insert_hand_index("picked", 0, 0);
    let step = CompiledStep::EffectInitiatedDigivolve {
        target: CompiledBindingRef::Named("target".into()),
        from_hand: CompiledBindingRef::Named("picked".into()),
        cost: CompiledCostDelta::Reduce(1),
        ignore_requirements: false,
        ignore_level: true,
    };
    {
        let mut ctx = EffectContext::new(&mut r.game, src, Some(base), 0);
        run_step(&step, &mut ctx, &mut bindings);
    }
    assert_eq!(top_id(&r, base), "KEN");
    assert_eq!(r.memory(), 3);
}
