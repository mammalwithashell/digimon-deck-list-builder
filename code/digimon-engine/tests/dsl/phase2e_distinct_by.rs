//! Phase 2e Task 8: `distinct_by: card_number` removes other zone indices
//! that share the picked card's printed card_id from the next-step
//! candidate list.

use digimon_dsl::compiled::{
    CompiledDistinctBy, CompiledPlayerRef, CompiledPredicate, CompiledStep, CompiledZone,
};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::effect_context::EffectContext;

fn push_to_trash(runner: &mut DebugRunner, player: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("push_to_trash: unknown card_id {card_id}"));
    let card_index = runner.game.next_card_index();
    runner.game.players[player as usize]
        .trash
        .push(CardSource::new(data_idx, player, card_index));
}

#[test]
fn distinct_by_card_number_filters_duplicates_after_pick() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .add_card(make_test_card("DUP", "DUP"))
        .add_card(make_test_card("UNIQ", "UNIQ"))
        .hand(0, &["SRC"])
        .build();

    // Two copies of "DUP" + one "UNIQ" in opponent's trash.
    push_to_trash(&mut runner, 1, "DUP");
    push_to_trash(&mut runner, 1, "DUP");
    push_to_trash(&mut runner, 1, "UNIQ");

    let src_card = runner.game.players[0].hand[0].handle();

    let steps = vec![CompiledStep::SelectCountCappedMulti {
        clamp_to_available: false,
        of: CompiledPlayerRef::Opponent,
        zone: CompiledZone::Trash,
        min: 0,
        max: digimon_dsl::compiled::CompiledCountBound::Literal(3),
        filter: CompiledPredicate::default(),
        bind_as: Some("picks".to_string()),
        prompt: "Pick distinct".to_string(),
        prompt_key: None,
        optional_zero: false,
        distinct_by: Some(CompiledDistinctBy::CardNumber),
    }];

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        let mut bindings = Bindings::new();
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    // Step 1: 3 candidates (the two DUPs + UNIQ).
    let pending = runner.game.pending_selection.as_ref().unwrap();
    assert_eq!(pending.valid_action_ids.len(), 3);
    let (action_id, selecting_player) = (pending.valid_action_ids[0], pending.selecting_player);
    runner
        .game
        .resolve_selection(selecting_player, action_id)
        .expect("first pick");

    // Step 2: only UNIQ should remain — both DUP indices are filtered out
    // because the picked DUP shares its card_id with the other DUP.
    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("pending must re-arm after first pick");
    assert_eq!(
        pending.valid_action_ids.len(),
        1,
        "after picking a DUP, the other DUP must be filtered by distinct_by=card_number"
    );

    let (action_id, selecting_player) = (pending.valid_action_ids[0], pending.selecting_player);
    runner
        .game
        .resolve_selection(selecting_player, action_id)
        .expect("second pick");

    // No more candidates: trampoline auto-commits.
    assert!(runner.game.pending_selection.is_none());
}

// ── G-ASSEMBLY-DISTINCT-BY-COLOR: `distinct_by: color` ─────────────────────
//
// "w/different colors" is a SET-level constraint: the picks must admit an
// injective card → printed-color assignment (a multicolor card represents
// exactly one of its colors). A pairwise "shares a color" rule would wrongly
// reject a Red/Blue card after a Red pick (it can stand for Blue).

fn colored(card_id: &str, colors: &[digimon_engine::enums::CardColor]) -> digimon_engine::card_data::CardData {
    let mut d = make_test_card(card_id, card_id);
    d.colors = colors.to_vec();
    d
}

fn color_steps(max: u8) -> Vec<CompiledStep> {
    vec![CompiledStep::SelectCountCappedMulti {
        clamp_to_available: false,
        of: CompiledPlayerRef::Opponent,
        zone: CompiledZone::Trash,
        min: 0,
        max: digimon_dsl::compiled::CompiledCountBound::Literal(max),
        filter: CompiledPredicate::default(),
        bind_as: Some("picks".to_string()),
        prompt: "Pick w/different colors".to_string(),
        prompt_key: None,
        optional_zero: false,
        distinct_by: Some(CompiledDistinctBy::Color),
    }]
}

fn trash_action(runner: &DebugRunner, player: usize, card_id: &str) -> u16 {
    let idx = runner.game.players[player]
        .trash
        .iter()
        .position(|c| runner.game.card_data[c.data_index].card_id == card_id)
        .unwrap_or_else(|| panic!("{card_id} not in trash"));
    digimon_engine::action::space::TRASH_EFFECT_START + idx as u16
}

#[test]
fn distinct_by_color_keeps_a_multicolor_card_that_can_represent_another_color() {
    use digimon_engine::enums::CardColor::{Blue, Red};
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .add_card(colored("R1", &[Red]))
        .add_card(colored("R2", &[Red]))
        .add_card(colored("RB", &[Red, Blue]))
        .hand(0, &["SRC"])
        .build();
    for id in ["R1", "R2", "RB"] {
        push_to_trash(&mut runner, 1, id);
    }
    let src_card = runner.game.players[0].hand[0].handle();
    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        let mut bindings = Bindings::new();
        run_steps(&color_steps(3), &mut ctx, &mut bindings);
    }
    let r1 = trash_action(&runner, 1, "R1");
    let r2 = trash_action(&runner, 1, "R2");
    let rb = trash_action(&runner, 1, "RB");
    let pending = runner.game.pending_selection.as_ref().unwrap();
    assert_eq!(pending.valid_action_ids.len(), 3);
    let sp = pending.selecting_player;
    runner.game.resolve_selection(sp, r1).expect("pick R1");
    let pending = runner.game.pending_selection.as_ref().expect("re-arms");
    assert!(
        pending.valid_action_ids.contains(&rb),
        "Red/Blue can stand for Blue next to a Red pick"
    );
    assert!(
        !pending.valid_action_ids.contains(&r2),
        "a second mono-Red card has no free color"
    );
}

#[test]
fn distinct_by_color_rejects_a_card_once_its_colors_are_all_claimed() {
    use digimon_engine::enums::CardColor::{Blue, Red};
    // RB picked first, then R: RB must stand for Blue, R for Red. A third
    // mono-Blue card would need Blue too → no perfect assignment.
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .add_card(colored("RB", &[Red, Blue]))
        .add_card(colored("R1", &[Red]))
        .add_card(colored("B1", &[Blue]))
        .hand(0, &["SRC"])
        .build();
    for id in ["RB", "R1", "B1"] {
        push_to_trash(&mut runner, 1, id);
    }
    let src_card = runner.game.players[0].hand[0].handle();
    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        let mut bindings = Bindings::new();
        run_steps(&color_steps(3), &mut ctx, &mut bindings);
    }
    let rb = trash_action(&runner, 1, "RB");
    let r1 = trash_action(&runner, 1, "R1");
    let sp = runner.game.pending_selection.as_ref().unwrap().selecting_player;
    runner.game.resolve_selection(sp, rb).expect("pick RB");
    runner.game.resolve_selection(sp, r1).expect("pick R1");
    // B1 is the only remaining card and it is inadmissible → auto-commit.
    assert!(
        runner.game.pending_selection.is_none(),
        "{{RB, R1, B1}} has only 2 colors for 3 cards; B1 must be filtered and the pick commit"
    );
}

#[test]
fn distinct_by_color_never_offers_a_colorless_card() {
    use digimon_engine::enums::CardColor::Red;
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .add_card(colored("R1", &[Red]))
        .add_card(colored("NONE", &[]))
        .hand(0, &["SRC"])
        .build();
    for id in ["R1", "NONE"] {
        push_to_trash(&mut runner, 1, id);
    }
    let src_card = runner.game.players[0].hand[0].handle();
    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        let mut bindings = Bindings::new();
        run_steps(&color_steps(2), &mut ctx, &mut bindings);
    }
    let none = trash_action(&runner, 1, "NONE");
    let pending = runner.game.pending_selection.as_ref().unwrap();
    assert!(!pending.valid_action_ids.contains(&none));
    assert_eq!(pending.valid_action_ids.len(), 1);
}

#[test]
fn distinct_by_color_yaml_parses_and_lowers() {
    let yaml = r#"
card: TEST-COLOR
name: Color Test
kind: digimon
level: 7
color: [white]
cost: 16
dp: 16000
traits: [ADVENTURE]
alt_paths:
  - kind: assembly
    cost: 8
    materials:
      - filter: { kind: digimon, trait_has: ADVENTURE }
        zones: [trash]
        repeat: { min: 6, max: 6 }
        distinct_by: color
        stack_under: true
"#;
    let spec: digimon_dsl::spec::CardSpec = serde_yml::from_str(yaml).expect("parse");
    let compiled = digimon_dsl::compile::compile(&spec).expect("compile");
    let m = &compiled.alt_paths[0].materials[0];
    assert_eq!(m.distinct_by, Some(CompiledDistinctBy::Color));
}
