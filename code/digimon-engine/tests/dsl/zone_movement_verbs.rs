use digimon_dsl::compiled::{CompiledClause, CompiledStep};
use digimon_engine::action::space::TRASH_EFFECT_START;
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::CardKind;
use digimon_engine::permanent::{Permanent, PermanentHandle};
use digimon_engine::selection::{
    OptionResolutionPhase, OptionSubtype, OptionUseSource, PendingOption,
};

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
        subtype: OptionSubtype::Standard,
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

// ─── S8 `move_trash_card_to_deck_top` (G-ZONE-SELECTED-TRASH-TO-DECK-TOP) ──────

#[test]
fn move_trash_card_to_deck_top_routes_selected_card_to_owner_deck_top() {
    // LM-030 clause B shape: a select_trash binds one card, then
    // move_trash_card_to_deck_top places it on TOP of its OWNER's deck.
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "Source"))
        .add_card(make_test_card("T0", "Trash 0"))
        .add_card(make_test_card("T1", "Trash 1"))
        .add_card(make_test_card("DECK0", "Deck Bottom"))
        .start();
    let source = runner.place_on_field(0, "SRC", Some(0));
    let source_card = runner.game.players[0].battle_area[0].top_card().handle();
    // Seed P0's deck with one card; the returned card must land on TOP (Vec end).
    let deck_seed = add_to_trash(&mut runner, 0, 0, "DECK0");
    {
        let card = runner.game.players[0]
            .trash
            .iter()
            .position(|c| c.handle() == deck_seed)
            .expect("seed");
        let cs = runner.game.players[0].trash.remove(card);
        runner.game.players[0].deck.push(cs);
    }
    // P0's trash holds two cards; T1 is OWNED by P1.
    add_to_trash(&mut runner, 0, 0, "T0");
    let owned_by_p1 = add_to_trash(&mut runner, 0, 1, "T1");

    run_compiled_steps(
        &mut runner,
        source_card,
        Some(source),
        compile_steps(
            r#"      - select_trash:
          of: you
          bind_as: to_return
          filter: { kind: digimon }
          prompt: "Return 1 card from trash to the top of the deck"
      - move_trash_card_to_deck_top:
          of: you
          card: to_return
"#,
        ),
    );

    // Resolve the select_trash selection — pick the P1-owned card.
    let (player, action) = {
        let pending = runner
            .game
            .pending_selection
            .as_ref()
            .expect("select_trash should park a selection");
        let want = TRASH_EFFECT_START
            + runner.game.players[0]
                .trash
                .iter()
                .position(|c| c.handle() == owned_by_p1)
                .expect("p1 card still in p0 trash") as u16;
        assert!(pending.valid_action_ids.contains(&want));
        (pending.selecting_player, want)
    };
    runner
        .execute_action(player, action)
        .expect("select_trash resolves");

    assert!(runner.game.pending_selection.is_none());
    // The P1-owned card returned to P1's deck (owner routing), on TOP.
    assert_eq!(
        runner.deck_size(1),
        1,
        "P1-owned trash card returns to P1's deck"
    );
    assert_eq!(
        runner.game.players[1].deck.last().map(|c| c.handle()),
        Some(owned_by_p1),
        "returned card must be on TOP (Vec end) of its owner's deck"
    );
    // P0's deck is untouched — only the un-picked T0 remains in P0's trash.
    assert_eq!(runner.deck_size(0), 1, "P0 deck untouched");
    assert_eq!(
        runner.trash_size(0),
        1,
        "only the picked card leaves P0's trash"
    );
}

#[test]
fn move_trash_card_to_deck_top_noop_when_card_absent() {
    // Negative: a binding pointing at a card not in trash is a silent no-op.
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "Source"))
        .add_card(make_test_card("PHANTOM", "Phantom"))
        .start();
    let source = runner.place_on_field(0, "SRC", Some(0));
    let source_card = runner.game.players[0].battle_area[0].top_card().handle();
    let phantom = CardHandle(9999);

    let steps = vec![CompiledStep::MoveTrashCardToDeckTop {
        of: digimon_dsl::compiled::CompiledPlayerRef::You,
        card: digimon_dsl::compiled::CompiledBindingRef::Named("ghost".into()),
    }];
    let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(source), 0);
    let mut bindings = Bindings::new();
    bindings.insert_card("ghost", phantom);
    run_steps(&steps, &mut ctx, &mut bindings);

    assert_eq!(runner.deck_size(0), 0, "no card moved");
    assert!(
        bindings.result_log().returned_to_deck.is_empty(),
        "no return recorded for a no-op"
    );
}

// ─── S9 `returned_card_matching` (G-ANY-RETURNED-CARD-PREDICATE) ──────────────

#[test]
fn return_all_trash_to_deck_bottom_records_returned_cards_in_result_log() {
    // The bulk return now records every moved card identity so a later
    // `returned_card_matching` / `effect_returned_any_card` predicate can
    // inspect this effect's returns.
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "Source"))
        .add_card(make_test_card("R0", "Returned 0"))
        .add_card(make_test_card("R1", "Returned 1"))
        .start();
    let source = runner.place_on_field(0, "SRC", Some(0));
    let source_card = runner.game.players[0].battle_area[0].top_card().handle();
    let r0 = add_to_trash(&mut runner, 0, 0, "R0");
    let r1 = add_to_trash(&mut runner, 0, 0, "R1");

    let steps = compile_steps(r#"      - return_all_trash_to_deck_bottom: { of: you }"#);
    let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(source), 0);
    let mut bindings = Bindings::new();
    run_steps(&steps, &mut ctx, &mut bindings);

    let logged = &bindings.result_log().returned_to_deck;
    assert!(logged.contains(&r0) && logged.contains(&r1));
    assert_eq!(logged.len(), 2);
}

#[test]
fn return_to_hand_records_returned_card_in_result_log() {
    // Bounce effects are still "returned" effects. Keep that separate from
    // generic add-to-hand/search logging so result riders do not overmatch.
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "Source"))
        .add_card(make_test_card("TARGET", "Target"))
        .start();
    let source = runner.place_on_field(0, "SRC", Some(0));
    let target = runner.place_on_field(1, "TARGET", Some(1));
    let target_card = runner.game.player(1).battle_area[target.index as usize]
        .top_card()
        .handle();
    let source_card = runner.game.players[0].battle_area[source.index as usize]
        .top_card()
        .handle();

    let steps = compile_steps(r#"      - return_to_hand: { target: target }"#);
    let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(source), 0);
    let mut bindings = Bindings::new();
    bindings.insert_permanent("target", target);
    run_steps(&steps, &mut ctx, &mut bindings);

    assert!(
        bindings.result_log().returned_cards.contains(&target_card),
        "return_to_hand must feed effect_returned_any_card / returned_card_matching"
    );
    assert!(
        bindings.result_log().added_to_hand.contains(&target_card),
        "return_to_hand also remains visible to add-to-hand result predicates"
    );
}

#[test]
fn for_each_merges_body_result_log_into_parent_bindings() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "Source"))
        .add_card(make_test_card("A", "Target A"))
        .add_card(make_test_card("B", "Target B"))
        .start();
    let source = runner.place_on_field(0, "SRC", Some(0));
    let a = runner.place_on_field(1, "A", Some(1));
    let b = runner.place_on_field(1, "B", Some(1));
    let a_card = runner.game.player(1).battle_area[a.index as usize]
        .top_card()
        .handle();
    let b_card = runner.game.player(1).battle_area[b.index as usize]
        .top_card()
        .handle();
    let source_card = runner.game.players[0].battle_area[source.index as usize]
        .top_card()
        .handle();

    let steps = compile_steps(
        r#"      - for_each:
          over: { of: opponent, zone: [battle_area], kind: digimon }
          bind_as: target
          body:
            - return_to_hand: { target: target }
"#,
    );
    let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(source), 0);
    let mut bindings = Bindings::new();
    run_steps(&steps, &mut ctx, &mut bindings);

    let logged = &bindings.result_log().returned_cards;
    assert!(logged.contains(&a_card) && logged.contains(&b_card));
    assert_eq!(logged.len(), 2);
}

#[test]
fn returned_card_matching_fires_when_a_matching_card_was_returned() {
    // BT17-077 clause 1c shape: return all trash, then "if this effect
    // returned a white level 7 card, gain 3 memory".
    let src = make_test_card("SRC", "Source");
    let mut white7 = make_test_card("WHITE7", "White Seven");
    white7.colors = vec![digimon_engine::enums::CardColor::White];
    white7.level = Some(7);
    let mut red3 = make_test_card("RED3", "Red Three");
    red3.colors = vec![digimon_engine::enums::CardColor::Red];
    red3.level = Some(3);

    let mut runner = DebugRunner::builder()
        .add_card(src)
        .add_card(white7)
        .add_card(red3)
        .start();
    let source = runner.place_on_field(0, "SRC", Some(0));
    let source_card = runner.game.players[0].battle_area[0].top_card().handle();
    add_to_trash(&mut runner, 0, 0, "WHITE7");
    add_to_trash(&mut runner, 0, 0, "RED3");

    let steps = compile_steps(
        r#"      - return_all_trash_to_deck_bottom: { of: you }
      - if:
          condition:
            returned_card_matching: { color_is: white, level_eq: 7 }
          then:
            - gain_memory: 3
"#,
    );
    let before = runner.memory();
    run_compiled_steps(&mut runner, source_card, Some(source), steps);

    assert_eq!(
        runner.memory(),
        before + 3,
        "returned_card_matching must fire — a white Lv.7 card was returned"
    );
}

#[test]
fn returned_card_matching_does_not_fire_without_a_matching_card() {
    // Negative: no white Lv.7 among the returned cards → predicate fails.
    let src = make_test_card("SRC", "Source");
    let mut red3 = make_test_card("RED3", "Red Three");
    red3.colors = vec![digimon_engine::enums::CardColor::Red];
    red3.level = Some(3);
    let mut white4 = make_test_card("WHITE4", "White Four");
    white4.colors = vec![digimon_engine::enums::CardColor::White];
    white4.level = Some(4);

    let mut runner = DebugRunner::builder()
        .add_card(src)
        .add_card(red3)
        .add_card(white4)
        .start();
    let source = runner.place_on_field(0, "SRC", Some(0));
    let source_card = runner.game.players[0].battle_area[0].top_card().handle();
    add_to_trash(&mut runner, 0, 0, "RED3");
    add_to_trash(&mut runner, 0, 0, "WHITE4");

    let steps = compile_steps(
        r#"      - return_all_trash_to_deck_bottom: { of: you }
      - if:
          condition:
            returned_card_matching: { color_is: white, level_eq: 7 }
          then:
            - gain_memory: 3
"#,
    );
    let before = runner.memory();
    run_compiled_steps(&mut runner, source_card, Some(source), steps);

    assert_eq!(
        runner.memory(),
        before,
        "returned_card_matching must not fire — no white Lv.7 was returned"
    );
}

#[test]
fn move_trash_card_to_deck_top_feeds_returned_card_matching() {
    // S8 + S9 compose: the selected-trash → deck-top move also records the
    // returned card so `returned_card_matching` can inspect it.
    let src = make_test_card("SRC", "Source");
    let mut green6 = make_test_card("GREEN6", "Green Six");
    green6.colors = vec![digimon_engine::enums::CardColor::Green];
    green6.level = Some(6);

    let mut runner = DebugRunner::builder()
        .add_card(src)
        .add_card(green6)
        .start();
    let source = runner.place_on_field(0, "SRC", Some(0));
    let source_card = runner.game.players[0].battle_area[0].top_card().handle();
    let green = add_to_trash(&mut runner, 0, 0, "GREEN6");

    let steps = vec![
        CompiledStep::MoveTrashCardToDeckTop {
            of: digimon_dsl::compiled::CompiledPlayerRef::You,
            card: digimon_dsl::compiled::CompiledBindingRef::Named("picked".into()),
        },
        CompiledStep::If {
            condition: digimon_dsl::compiled::CompiledPredicate {
                returned_card_matching: Some(Box::new(digimon_dsl::compiled::CompiledPredicate {
                    color_is: Some(digimon_dsl::compiled::CompiledColor::Green),
                    ..Default::default()
                })),
                ..Default::default()
            },
            then: vec![CompiledStep::GainMemory(2)],
            else_branch: vec![],
        },
    ];
    let before = runner.memory();
    let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(source), 0);
    let mut bindings = Bindings::new();
    bindings.insert_card("picked", green);
    run_steps(&steps, &mut ctx, &mut bindings);

    assert_eq!(
        runner.memory(),
        before + 2,
        "returned_card_matching must see the card moved by move_trash_card_to_deck_top"
    );
}

// ─── G-DSL-COST-RETURN-SELF-DIGI-CARD-BY-NAME ───────────────────────────────
//
// `return_selected_sources_to_hand` removes each `select_own_sources`-bound
// digivolution source card from its host stack and routes it to its OWNER's
// hand (mirror of `trash_selected_sources`, hand destination instead of trash;
// fires no `OnDigivolutionCardTrashed`).

/// POSITIVE: a selected own digivolution source returns to the controller's
/// own hand (owner == controller, the common case), leaving the host
/// permanent's top card and remaining stack intact.
#[test]
fn return_selected_sources_to_hand_routes_picked_source_to_owner_hand() {
    use digimon_dsl::compiled::CompiledPredicate;
    use digimon_engine::action::space::encode_source_select;
    use digimon_engine::dsl_cards::step::RunOutcome;
    use digimon_engine::selection::SelectionKind;

    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC-LOW", "Bottom Source"))
        .add_card(make_test_card("SRC-MID", "Dragon Mode"))
        .add_card(make_test_card("TOP", "Host Top"))
        .add_card(make_test_card("EFFECT", "Effect"))
        .hand(0, &["EFFECT"])
        .start();

    // Host stack owned entirely by P0: [SRC-LOW, SRC-MID, TOP] bottom→top.
    let host = runner.place_stack(0, &["SRC-LOW", "SRC-MID", "TOP"]);
    let source_card = runner.game.players[0].hand[0].handle();

    let hand_before = runner.game.players[0].hand.len();

    let mut name_filter = CompiledPredicate::default();
    name_filter.name_contains = Some("Dragon Mode".to_string());

    let steps = vec![CompiledStep::SelectOwnSources {
        target: None,
        filter: name_filter,
        min: 1,
        max: 1,
        bind_as: Some("picked".to_string()),
        prompt: "Choose Dragon Mode source".to_string(),
        then: vec![CompiledStep::ReturnSelectedSourcesToHand {
            source_refs: "picked".to_string(),
        }],
    }];

    let mut bindings = Bindings::new();
    let outcome = {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, None, 0);
        run_steps(&steps, &mut ctx, &mut bindings)
    };
    assert_eq!(outcome, RunOutcome::Parked);
    assert_eq!(
        runner.game.pending_selection.as_ref().map(|s| s.kind),
        Some(SelectionKind::SourceMulti {
            min: 1,
            max: 1,
            picked: 0,
        })
    );

    // SRC-MID is the digivolution source at source_index 1 (just under TOP).
    let pick = encode_source_select(host.index as u16, 1).expect("Dragon Mode source action");
    runner.execute_action(0, pick).expect("pick Dragon Mode");

    assert!(runner.game.pending_selection.is_none());

    // Host top card and the remaining below-top source are untouched.
    let host_perm = &runner.game.players[0].battle_area[host.index as usize];
    assert_eq!(
        host_perm.card_sources.len(),
        2,
        "host stack shrinks by exactly the one returned source"
    );
    assert_eq!(
        host_perm.top_card().card_id(&runner.game.card_data),
        "TOP",
        "host top card preserved"
    );
    assert_eq!(
        host_perm.card_sources[0].card_id(&runner.game.card_data),
        "SRC-LOW",
        "the un-picked below-top source remains, ordering preserved"
    );

    // The picked source returns to P0's (its owner's) hand, NOT to trash.
    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before + 1,
        "owner's hand grows by 1 (the returned Dragon Mode source)"
    );
    assert!(
        runner.game.players[0]
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "SRC-MID"),
        "the returned source is the Dragon Mode card"
    );
    assert!(
        runner.game.players[0].trash.is_empty(),
        "return-to-hand must NOT route the source to trash"
    );
}

/// OWNER-ROUTING: a digivolution source owned by P1 but sitting under a
/// P0-controlled host returns to P1's hand (the source's `CardSource.owner`),
/// not P0's — even though P0 is the controller running the effect.
#[test]
fn return_selected_sources_to_hand_routes_to_source_owner_not_controller() {
    use digimon_engine::action::space::encode_source_select;
    use digimon_engine::dsl_cards::step::RunOutcome;

    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC-OWNED-BY-P1", "Foreign Source"))
        .add_card(make_test_card("TOP", "Host Top"))
        .add_card(make_test_card("EFFECT", "Effect"))
        .hand(0, &["EFFECT"])
        .start();

    // P0 controls host [TOP]; push a P1-owned digivolution source under it.
    let host = runner.place_stack(0, &["TOP"]);
    runner.push_source_owned(host, "SRC-OWNED-BY-P1", 1);
    let source_card = runner.game.players[0].hand[0].handle();

    let p0_hand_before = runner.game.players[0].hand.len();
    let p1_hand_before = runner.game.players[1].hand.len();

    let steps = vec![CompiledStep::SelectOwnSources {
        target: None,
        filter: digimon_dsl::compiled::CompiledPredicate::default(),
        min: 1,
        max: 1,
        bind_as: Some("picked".to_string()),
        prompt: "Choose source".to_string(),
        then: vec![CompiledStep::ReturnSelectedSourcesToHand {
            source_refs: "picked".to_string(),
        }],
    }];

    let mut bindings = Bindings::new();
    let outcome = {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, None, 0);
        run_steps(&steps, &mut ctx, &mut bindings)
    };
    assert_eq!(outcome, RunOutcome::Parked);

    // The P1-owned source is at source_index 0 (below TOP).
    let pick = encode_source_select(host.index as u16, 0).expect("foreign source action");
    runner.execute_action(0, pick).expect("pick foreign source");

    assert!(runner.game.pending_selection.is_none());
    assert_eq!(
        runner.game.players[0].hand.len(),
        p0_hand_before,
        "controller P0's hand must NOT receive a source it does not own"
    );
    assert_eq!(
        runner.game.players[1].hand.len(),
        p1_hand_before + 1,
        "the source returns to its OWNER P1's hand"
    );
    assert!(
        runner.game.players[1]
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "SRC-OWNED-BY-P1"),
        "P1's hand holds the returned foreign source"
    );
}
