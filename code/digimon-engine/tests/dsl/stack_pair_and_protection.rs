//! Substrate for the EX13 leave-protection cluster (2026-10-01):
//!
//! - **G-ENGINE-SAME-LEVEL-SOURCE-PAIR-SELECTION** — `select_materials
//!   { min, same_by: level }` + `trash_selected_materials`: a multi-pick over a
//!   permanent's digivolution cards where every pick shares ONE level and a
//!   first pick is only offered from a level holding at least `min`
//!   candidates (so the pick can always be completed). EX13-016 Omnimon "by
//!   trashing 2 same-level cards from its digivolution cards".
//! - **G-ENGINE-STACKED-CARD-RETURN-PROTECTION** — `ModifierType::
//!   ImmuneFromStackReturn` + controller-scoped `ImmuneFromStackTrashing`
//!   (`ModifierRegistry::blocks_effect_from`), and the aura `modifier_from:`
//!   slot that installs them opponent-scoped. EX13-023 UlforceVeedramon.
//!
//! No-approximations: every pick is a pending selection; the constraints
//! shape the legal mask, nothing is auto-picked.

use digimon_dsl::compiled::{
    CompiledBindingRef, CompiledClause, CompiledCountBound, CompiledDeclarativeClause,
    CompiledEffectController, CompiledPredicate, CompiledSameBy, CompiledStep,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl::raw_rust_registry::StubRegistry;
use digimon_engine::dsl::validator::{validate, ValidationContext};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind, Expiry, ModifierType};
use digimon_engine::modifiers::{EffectControllerFilter, EffectImmunityFilter, ModifierEntry};
use digimon_engine::permanent::PermanentHandle;

fn lv(id: &str, level: u8) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Red];
    c.level = Some(level);
    c.dp = Some(1000 * level as i32);
    c.play_cost = level as u16;
    c
}

fn tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c
}

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .add_card(lv("TOP", 7))
        .add_card(lv("A4", 4))
        .add_card(lv("B4", 4))
        .add_card(lv("C5", 5))
        .add_card(lv("D6", 6))
        .add_card(lv("E6", 6))
        .add_card(tamer("TM"))
        .deck(0, &["A4"; 5])
        .deck(1, &["A4"; 5])
        .memory(0)
        .start()
}

fn same_level_pair_steps() -> Vec<CompiledStep> {
    vec![
        CompiledStep::SelectMaterials {
            of_permanent: CompiledBindingRef::Named("carrier".to_string()),
            max: CompiledCountBound::Literal(2),
            filter: CompiledPredicate::default(),
            uniqueness: None,
            bind_as: Some("pair".to_string()),
            prompt: "Trash 2 same-level digivolution cards".to_string(),
            prompt_key: None,
            optional_zero: false,
            min: 2,
            same_by: Some(CompiledSameBy::Level),
        },
        CompiledStep::TrashSelectedMaterials {
            target: CompiledBindingRef::Named("carrier".to_string()),
            cards: CompiledBindingRef::Named("pair".to_string()),
        },
        CompiledStep::GainMemory(1),
    ]
}

fn run(r: &mut DebugRunner, carrier: PermanentHandle, steps: &[CompiledStep]) {
    let top = r.game.players[0].battle_area[carrier.index as usize]
        .top_card()
        .handle();
    let mut bindings = Bindings::new();
    bindings.insert_permanent("carrier", carrier);
    let mut ctx = EffectContext::new(&mut r.game, top, Some(carrier), 0);
    run_steps(steps, &mut ctx, &mut bindings);
}

fn source_ids(r: &DebugRunner, h: PermanentHandle) -> Vec<String> {
    let p = &r.game.players[h.player as usize].battle_area[h.index as usize];
    let n = p.card_sources.len() - 1;
    p.card_sources[..n]
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

fn trash_ids(r: &DebugRunner, p: usize) -> Vec<String> {
    r.game.players[p]
        .trash
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

#[test]
fn same_level_pair_first_pick_only_from_levels_with_a_partner() {
    let mut r = runner();
    // Stack bottom→top: A4, C5, B4, D6, TOP. Levels: 4,5,4,6 → only the two
    // level-4 cards can start a same-level pair.
    let carrier = r.place_stack(0, &["A4", "C5", "B4", "D6", "TOP"]);
    let mem = r.memory();
    run(&mut r, carrier, &same_level_pair_steps());
    let pending = r.game.pending_selection.as_ref().expect("pair pick pending");
    assert_eq!(
        pending.valid_action_ids.len(),
        2,
        "only A4 and B4 are offered (C5 / D6 have no same-level partner)"
    );
    assert!(!pending.is_optional, "min 2: PASS is not legal before 2 picks");
    let ids = pending.valid_action_ids.clone();
    // Stack indices 0 (A4) and 2 (B4): actions are range_start + index.
    assert_eq!(ids[1] - ids[0], 2, "offered A4 (index 0) and B4 (index 2)");
    let who = pending.selecting_player;
    r.game.resolve_selection(who, ids[0]).expect("pick A4");
    let pending = r.game.pending_selection.as_ref().expect("second pick pending");
    assert_eq!(
        pending.valid_action_ids,
        vec![ids[1]],
        "after A4 only the other level-4 card (B4) is offered"
    );
    assert!(!pending.is_optional, "still below the floor of 2");
    r.game.resolve_selection(who, ids[1]).expect("pick B4");
    assert!(r.game.pending_selection.is_none(), "auto-commits at max 2");
    assert_eq!(source_ids(&r, carrier), vec!["C5", "D6"], "both level-4 cards left");
    let trash = trash_ids(&r, 0);
    assert!(trash.contains(&"A4".to_string()) && trash.contains(&"B4".to_string()));
    assert_eq!(r.memory(), mem + 1, "the tail ran after the pair");
}

#[test]
fn same_level_pair_offers_every_level_that_has_a_pair() {
    let mut r = runner();
    // Levels 4,6,4,6,5 → four cards (both pairs) start a pair; the 5 doesn't.
    let carrier = r.place_stack(0, &["A4", "D6", "B4", "E6", "C5", "TOP"]);
    run(&mut r, carrier, &same_level_pair_steps());
    let pending = r.game.pending_selection.as_ref().expect("pending");
    assert_eq!(pending.valid_action_ids.len(), 4);
    let ids = pending.valid_action_ids.clone();
    let who = pending.selecting_player;
    // Pick D6 (stack index 1) → only E6 (index 3) remains.
    r.game.resolve_selection(who, ids[1]).expect("pick D6");
    let pending = r.game.pending_selection.as_ref().expect("pending");
    assert_eq!(pending.valid_action_ids, vec![ids[3]], "only the other level 6");
}

#[test]
fn same_level_pair_unpayable_picks_nothing_and_skips_the_tail() {
    let mut r = runner();
    // Levels 4,5,6 + a level-less Tamer card: no level appears twice.
    let carrier = r.place_stack(0, &["A4", "TM", "C5", "D6", "TOP"]);
    let mem = r.memory();
    run(&mut r, carrier, &same_level_pair_steps());
    assert!(r.game.pending_selection.is_none(), "no pair → no prompt");
    assert_eq!(source_ids(&r, carrier).len(), 4, "nothing trashed");
    assert_eq!(r.memory(), mem, "the cost's tail does not run");
}

#[test]
fn same_level_pair_resolves_on_a_cloned_game() {
    // Clone-safety (rule 28): the multi-pick parks on the resumable VM, so a
    // cloned game resolves the same picks with the same constraints.
    let mut r = runner();
    let carrier = r.place_stack(0, &["A4", "C5", "B4", "D6", "TOP"]);
    run(&mut r, carrier, &same_level_pair_steps());
    let mut g = r.game.clone();
    let pending = g.pending_selection.as_ref().expect("pending on clone");
    let ids = pending.valid_action_ids.clone();
    let who = pending.selecting_player;
    assert_eq!(ids.len(), 2);
    g.resolve_selection(who, ids[0]).expect("clone pick 1");
    let pending = g.pending_selection.as_ref().expect("clone pick 2 pending");
    assert_eq!(pending.valid_action_ids, vec![ids[1]]);
    g.resolve_selection(who, ids[1]).expect("clone pick 2");
    assert!(g.pending_selection.is_none());
    assert_eq!(g.players[0].battle_area[0].card_sources.len(), 3, "C5, D6, TOP");
}

#[test]
fn same_by_level_compiles_from_yaml_and_rejects_uniqueness() {
    let yaml = r#"
card: DSL-PAIR
name: Pair Harness
kind: digimon
level: 7
color: [red]
cost: 10
dp: 11000
effects:
  - when: main_on_field
    process:
      - select_materials:
          of_permanent: source
          min: 2
          max: 2
          same_by: level
          bind_as: pair
          prompt: "Trash 2 same-level cards"
      - trash_selected_materials: { target: source, cards: pair }
"#;
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).expect("YAML parses");
    let compiled = digimon_dsl::compile::compile(&spec).expect("YAML compiles");
    let CompiledClause::Triggered(t) = &compiled.effects[0] else {
        panic!("triggered");
    };
    match &t.process[0] {
        CompiledStep::SelectMaterials { min, same_by, .. } => {
            assert_eq!(*min, 2);
            assert_eq!(*same_by, Some(CompiledSameBy::Level));
        }
        other => panic!("expected SelectMaterials, got {other:?}"),
    }
    assert!(matches!(t.process[1], CompiledStep::TrashSelectedMaterials { .. }));

    let bad = yaml.replace("same_by: level", "same_by: level\n          uniqueness: name");
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(&bad).expect("YAML parses");
    let errs = validate(&spec, &ValidationContext { raw_rust: &StubRegistry::empty() })
        .expect_err("must be rejected");
    assert!(
        errs.iter().any(|e| e.path.ends_with("same_by")),
        "same_by + uniqueness must be rejected: {errs:?}"
    );
}

// ─── Stacked-card protections ────────────────────────────────────────────────

fn protect(r: &mut DebugRunner, h: PermanentHandle, ty: ModifierType) {
    r.game.modifiers.add(
        h,
        ModifierEntry::simple(ty, 0, Expiry::Permanent, h.player).with_effect_immunity_filter(
            EffectImmunityFilter {
                source_kind: None,
                controller: EffectControllerFilter::OpponentOnly,
            },
        ),
    );
}

fn ctx_for(r: &mut DebugRunner, player: u8) -> EffectContext<'_> {
    let card = r.game.players[0].battle_area[0].top_card().handle();
    EffectContext::new(&mut r.game, card, None, player)
}

#[test]
fn stack_return_protection_blocks_only_opponent_returns() {
    let mut r = runner();
    let h = r.place_stack(0, &["A4", "B4", "TOP"]);
    protect(&mut r, h, ModifierType::ImmuneFromStackReturn);
    let a4 = r.game.players[0].battle_area[0].card_sources[0].handle();
    let b4 = r.game.players[0].battle_area[0].card_sources[1].handle();
    assert!(!ctx_for(&mut r, 1).return_card_source_to_hand(h, a4), "opponent → hand blocked");
    assert!(!ctx_for(&mut r, 1).return_card_source_to_deck(h, a4, true), "opponent → deck blocked");
    assert_eq!(source_ids(&r, h), vec!["A4", "B4"]);
    assert!(ctx_for(&mut r, 0).return_card_source_to_hand(h, b4), "own effect still returns");
    assert_eq!(source_ids(&r, h), vec!["A4"]);
}

#[test]
fn stack_trash_protection_is_controller_scoped_and_blocks_de_digivolve() {
    let mut r = runner();
    let h = r.place_stack(0, &["A4", "C5", "D6", "TOP"]);
    protect(&mut r, h, ModifierType::ImmuneFromStackTrashing);
    let a4 = r.game.players[0].battle_area[0].card_sources[0].handle();
    assert!(!ctx_for(&mut r, 1).trash_card_source(h, a4), "opponent trash blocked");
    assert_eq!(ctx_for(&mut r, 1).trash_bottom_sources(h, 1), 0);
    assert!(!ctx_for(&mut r, 1).trash_top_source(h));
    assert_eq!(ctx_for(&mut r, 1).de_digivolve(h, Some(3), Some(1)), 0, "<De-Digivolve> blocked");
    assert_eq!(r.game.players[0].battle_area[0].card_sources.len(), 4, "stack intact");
    assert!(ctx_for(&mut r, 0).trash_card_source(h, a4), "own effect still trashes");
    assert_eq!(source_ids(&r, h), vec!["C5", "D6"]);
}

#[test]
fn aura_modifier_from_compiles_opponent_scoped() {
    let yaml = r#"
card: DSL-AURA
name: Aura Harness
kind: digimon
level: 6
color: [blue]
cost: 10
dp: 12000
effects:
  - kind: aura
    target: {}
    active_when: { source_is_unsuspended: true }
    modifier: ImmuneFromStackReturn
    modifier_from: opponent
"#;
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).expect("YAML parses");
    let compiled = digimon_dsl::compile::compile(&spec).expect("YAML compiles");
    let CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
        modifier,
        modifier_from,
        ..
    }) = &compiled.effects[0]
    else {
        panic!("aura");
    };
    assert_eq!(modifier.as_deref(), Some("ImmuneFromStackReturn"));
    assert_eq!(*modifier_from, Some(CompiledEffectController::Opponent));

    let bad = r#"
card: DSL-AURA
name: Aura Harness
kind: digimon
level: 6
color: [blue]
cost: 10
dp: 12000
effects:
  - kind: aura
    target: {}
    dp_modifier: 1000
    modifier_from: opponent
"#;
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(bad).expect("YAML parses");
    let errs = validate(&spec, &ValidationContext { raw_rust: &StubRegistry::empty() })
        .expect_err("must be rejected");
    assert!(
        errs.iter().any(|e| e.path.ends_with("modifier_from")),
        "modifier_from without modifier must be rejected: {errs:?}"
    );
}
