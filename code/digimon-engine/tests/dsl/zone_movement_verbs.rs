use digimon_dsl::compiled::{CompiledClause, CompiledStep};
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::CardKind;
use digimon_engine::permanent::{Permanent, PermanentHandle};
use digimon_engine::selection::{OptionResolutionPhase, OptionUseSource, PendingOption};

fn compile_steps(step_yaml: &str) -> Vec<CompiledStep> {
    let yaml = format!(
        r#"
card: DSL-ZONE
name: Zone Verb Harness
kind: digimon
level: 6
color: [blue]
cost: 10
dp: 11000
effects:
  - when: main_on_field
    process:
{step_yaml}
"#
    );
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(&yaml).expect("YAML parses");
    let compiled = digimon_dsl::compile::compile(&spec).expect("YAML compiles");
    match &compiled.effects[0] {
        CompiledClause::Triggered(t) => t.process.clone(),
        other => panic!("expected triggered clause, got {other:?}"),
    }
}

fn run_compiled_steps(
    runner: &mut DebugRunner,
    source_card: CardHandle,
    source_permanent: Option<PermanentHandle>,
    steps: Vec<CompiledStep>,
) {
    let mut ctx = EffectContext::new(&mut runner.game, source_card, source_permanent, 0);
    let mut bindings = Bindings::new();
    run_steps(&steps, &mut ctx, &mut bindings);
}

fn add_to_trash(runner: &mut DebugRunner, player: u8, owner: u8, card_id: &str) -> CardHandle {
    let game = &mut runner.game;
    let data_idx = game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .expect("card data");
    let idx = game.next_card_index();
    let card = CardSource::new(data_idx, owner, idx);
    let handle = card.handle();
    game.players[player as usize].trash.push(card);
    handle
}

fn seed_stack(
    runner: &mut DebugRunner,
    player: u8,
    cards_bottom_to_top: &[(&str, u8)],
) -> PermanentHandle {
    let game = &mut runner.game;
    let turn = game.turn_count;
    let mut sources = Vec::new();
    for (id, owner) in cards_bottom_to_top {
        let data_idx = game
            .card_data
            .iter()
            .position(|c| c.card_id == *id)
            .expect("card data");
        let idx = game.next_card_index();
        sources.push(CardSource::new(data_idx, *owner, idx));
    }
    let bottom = sources.remove(0);
    let mut permanent = Permanent::new(bottom, turn);
    permanent.card_sources.extend(sources);
    game.players[player as usize].battle_area.push(permanent);
    PermanentHandle {
        player,
        index: (game.players[player as usize].battle_area.len() - 1) as u8,
    }
}

fn card_ids(cards: &[CardSource], data: &[digimon_engine::card_data::CardData]) -> Vec<String> {
    cards.iter().map(|c| c.card_id(data).to_string()).collect()
}

#[test]
fn bounce_self_and_place_self_at_security_lower_into_self_helpers() {
    let mut bounce_runner = DebugRunner::builder()
        .add_card(make_test_card("SELF", "Self"))
        .start();
    let source = bounce_runner.place_on_field(0, "SELF", Some(0));
    let source_card = bounce_runner.game.players[0].battle_area[0]
        .top_card()
        .handle();

    run_compiled_steps(
        &mut bounce_runner,
        source_card,
        Some(source),
        compile_steps("      - bounce_self: {}\n"),
    );

    assert_eq!(bounce_runner.battle_area_size(0), 0);
    assert_eq!(bounce_runner.hand_size(0), 1);

    let mut security_runner = DebugRunner::builder()
        .add_card(make_test_card("SECSELF", "Security Self"))
        .start();
    let source = security_runner.place_on_field(0, "SECSELF", Some(0));
    let source_card = security_runner.game.players[0].battle_area[0]
        .top_card()
        .handle();

    run_compiled_steps(
        &mut security_runner,
        source_card,
        Some(source),
        compile_steps(
            "      - place_self_at_security:\n          position: top\n          face: up\n",
        ),
    );

    assert_eq!(security_runner.battle_area_size(0), 0);
    assert_eq!(security_runner.security_count(0), 1);
    let placed = security_runner.game.players[0].security[0].handle();
    assert!(security_runner.game.players[0]
        .face_up_security
        .contains(&placed.0));
}

#[test]
fn place_self_option_at_security_consumes_pending_option() {
    let mut option = make_test_card("OPT", "Option");
    option.card_kind = CardKind::Option;
    let mut runner = DebugRunner::builder().add_card(option).start();
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "OPT")
        .unwrap();
    let card = CardSource::new(data_idx, 0, runner.game.next_card_index());
    let handle = card.handle();
    runner.game.pending_option = Some(PendingOption {
        owner: 0,
        card,
        source_kind: OptionUseSource::Hand,
        resolution_phase: OptionResolutionPhase::MainEffectDrain,
    });

    run_compiled_steps(
        &mut runner,
        handle,
        None,
        compile_steps(
            "      - place_self_option_at_security:\n          position: top\n          face: up\n",
        ),
    );

    assert!(runner.game.pending_option.is_none());
    assert_eq!(runner.security_count(0), 1);
    assert_eq!(
        runner.game.players[0].security[0].card_id(&runner.game.card_data),
        "OPT"
    );
}

#[test]
fn permanent_and_stacked_card_security_verbs_move_expected_cards() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("BOTTOM", "Bottom"))
        .add_card(make_test_card("MID", "Mid"))
        .add_card(make_test_card("TOP", "Top"))
        .add_card(make_test_card("CARRIER", "Carrier"))
        .start();
    let target = seed_stack(&mut runner, 0, &[("BOTTOM", 0), ("MID", 1), ("TOP", 0)]);
    let source_card = runner.game.players[0].battle_area[0].top_card().handle();
    run_compiled_steps(
        &mut runner,
        source_card,
        Some(target),
        compile_steps(
            "      - place_permanent_on_security_observed:\n          target: source\n          position: top\n          face: down\n          include_sources: true\n",
        ),
    );
    assert_eq!(runner.battle_area_size(0), 0);
    assert_eq!(runner.security_count(0), 1);
    assert_eq!(runner.trash_size(0), 1, "P0-owned bottom source trashed");
    assert_eq!(
        runner.trash_size(1),
        1,
        "P1-owned middle source owner-routed"
    );

    let carrier = seed_stack(&mut runner, 0, &[("BOTTOM", 0), ("MID", 0), ("CARRIER", 0)]);
    let carrier_top = runner.game.players[0].battle_area[0].top_card().handle();
    run_compiled_steps(
        &mut runner,
        carrier_top,
        Some(carrier),
        compile_steps(
            "      - security_place_stacked_card:\n          carrier: source\n          source_index_from_top: 0\n          of: you\n          position: top\n          face: up\n",
        ),
    );
    assert_eq!(runner.security_count(0), 2);
    assert_eq!(
        runner.game.players[0]
            .security
            .last()
            .unwrap()
            .card_id(&runner.game.card_data),
        "MID"
    );

    run_compiled_steps(
        &mut runner,
        carrier_top,
        Some(carrier),
        compile_steps(
            "      - security_place_top_stacked_card:\n          carrier: source\n          of: you\n          position: top\n          face: down\n",
        ),
    );
    assert_eq!(
        runner.game.players[0]
            .security
            .last()
            .unwrap()
            .card_id(&runner.game.card_data),
        "BOTTOM"
    );
}

#[test]
fn bulk_trash_and_hand_reduction_verbs_call_helpers() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "Source"))
        .add_card(make_test_card("T0", "Trash 0"))
        .add_card(make_test_card("T1", "Trash 1"))
        .add_card(make_test_card("BASE", "Base"))
        .add_card(make_test_card("UNDER", "Under"))
        .add_card(make_test_card("TOP", "Top"))
        .add_card(make_test_card("H1", "H1"))
        .add_card(make_test_card("H2", "H2"))
        .add_card(make_test_card("H3", "H3"))
        .hand(1, &["H1", "H2", "H3"])
        .start();
    let source = runner.place_on_field(0, "SRC", Some(0));
    let source_card = runner.game.players[0].battle_area[0].top_card().handle();
    add_to_trash(&mut runner, 0, 0, "T0");
    add_to_trash(&mut runner, 0, 1, "T1");
    seed_stack(&mut runner, 1, &[("BASE", 1), ("UNDER", 1), ("TOP", 1)]);

    run_compiled_steps(
        &mut runner,
        source_card,
        Some(source),
        compile_steps(
            r#"      - return_all_trash_to_deck_bottom: { of: you }
      - trash_top_n_digivolution_cards_of_each:
          of: opponent
          n: 1
      - trash_opponent_hand_to_count:
          opponent: opponent
          target_count: 1
"#,
        ),
    );

    assert_eq!(
        runner.trash_size(0),
        0,
        "own trash drained before stack peel"
    );
    assert_eq!(
        runner.deck_size(0),
        1,
        "P0-owned trash card returned to P0 deck"
    );
    assert_eq!(
        runner.deck_size(1),
        1,
        "P1-owned trash card returned to P1 deck"
    );
    assert_eq!(
        card_ids(&runner.game.players[1].trash, &runner.game.card_data),
        vec!["UNDER".to_string()]
    );
    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("opponent hand reduction selection");
    assert_eq!(pending.selecting_player, 1);
}

#[test]
fn search_own_security_stack_runs_select_or_no_match_body() {
    let mut wanted = make_test_card("WANTED", "Wanted");
    wanted.traits = vec!["Olympos XII".to_string()];
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "Source"))
        .add_card(wanted)
        .add_card(make_test_card("FILLER", "Filler"))
        .security(0, &["FILLER", "WANTED"])
        .start();
    let source = runner.place_on_field(0, "SRC", Some(0));
    let source_card = runner.game.players[0].battle_area[0].top_card().handle();
    run_compiled_steps(
        &mut runner,
        source_card,
        Some(source),
        compile_steps(
            r#"      - search_own_security_stack:
          filter: { trait_has: "Olympos XII" }
          prompt: "Choose an Olympos card"
          bind_as: picked
          on_select:
            - add_to_hand_from_security:
                of: you
                card: picked
"#,
        ),
    );
    let (player, action) = {
        let pending = runner
            .game
            .pending_selection
            .as_ref()
            .expect("security search");
        assert_eq!(pending.valid_action_ids.len(), 1);
        (pending.selecting_player, pending.valid_action_ids[0])
    };
    runner
        .game
        .resolve_selection(player, action)
        .expect("resolve search");
    assert_eq!(runner.hand_size(0), 1);
    assert_eq!(runner.security_count(0), 1);

    run_compiled_steps(
        &mut runner,
        source_card,
        Some(source),
        compile_steps(
            r#"      - search_own_security_stack:
          filter: { trait_has: Missing }
          prompt: "Choose a missing card"
          bind_as: picked
          on_select:
            - add_to_hand_from_security:
                of: you
                card: picked
          on_no_match:
            - gain_memory: 1
"#,
        ),
    );
    assert!(runner.game.pending_selection.is_none());
    assert_eq!(runner.memory(), 1);
}
