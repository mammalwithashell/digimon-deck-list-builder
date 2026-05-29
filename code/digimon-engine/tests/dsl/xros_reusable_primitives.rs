use digimon_dsl::{compile::compile, spec::CardSpec};
use digimon_engine::action::space::{
    encode_attack, encode_source_select, PASS, PLAY_HAND_START, TRASH_EFFECT_START,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::{run_steps, RunOutcome};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::PermanentHandle;

fn card(id: &str, kind: CardKind, traits: &[&str], play_cost: i32) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: kind,
        level: matches!(kind, CardKind::Digimon).then_some(4),
        dp: matches!(kind, CardKind::Digimon).then_some(4000),
        play_cost: play_cost as u16,
        colors: vec![CardColor::Red],
        traits: traits.iter().map(|s| s.to_string()).collect(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        dual: None,
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
        also_treated_as: Vec::new(),
    }
}

fn evo_card(id: &str, traits: &[&str], from_level: u8) -> CardData {
    let mut data = card(id, CardKind::Digimon, traits, 4);
    data.level = Some(from_level + 1);
    data.evo_costs = vec![EvoCost {
        card_color: 0,
        level: from_level,
        memory_cost: 0,
    }];
    data
}

fn compile_steps(yaml: &str) -> Vec<digimon_dsl::compiled::CompiledStep> {
    let spec: CardSpec = serde_yml::from_str(yaml).expect("yaml parses");
    let compiled = compile(&spec).expect("yaml compiles");
    let digimon_dsl::compiled::CompiledClause::Triggered(clause) = &compiled.effects[0] else {
        panic!("expected triggered clause");
    };
    clause.process.clone()
}

fn compile_errors(yaml: &str) -> String {
    let spec: CardSpec = serde_yml::from_str(yaml).expect("yaml parses");
    let err = compile(&spec).expect_err("yaml should fail validation");
    format!("{err:?}")
}

fn insert_source(runner: &mut DebugRunner, host: PermanentHandle, card_id: &str) -> CardHandle {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|data| data.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown test card {card_id}"));
    let source = CardSource::new(data_idx, host.player, runner.game.next_card_index());
    let handle = source.handle();
    let sources = &mut runner.game.players[host.player as usize].battle_area[host.index as usize]
        .card_sources;
    let insert_at = sources.len().saturating_sub(1);
    sources.insert(insert_at, source);
    handle
}

fn push_to_hand(runner: &mut DebugRunner, player: u8, card_id: &str) -> CardHandle {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|data| data.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown test card {card_id}"));
    let source = CardSource::new(data_idx, player, runner.game.next_card_index());
    let handle = source.handle();
    runner.game.players[player as usize].hand.push(source);
    handle
}

fn push_to_trash(runner: &mut DebugRunner, player: u8, card_id: &str) -> CardHandle {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|data| data.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown test card {card_id}"));
    let source = CardSource::new(data_idx, player, runner.game.next_card_index());
    let handle = source.handle();
    runner.game.players[player as usize].trash.push(source);
    handle
}

fn source_ids(runner: &DebugRunner, host: PermanentHandle) -> Vec<String> {
    runner.game.players[host.player as usize].battle_area[host.index as usize]
        .card_sources
        .iter()
        .map(|card| card.card_id(&runner.game.card_data).to_string())
        .collect()
}

#[test]
fn select_under_tamer_sources_can_play_origin_preserved_source_for_free() {
    let steps = compile_steps(
        r#"
card: TEST-UNDER-TAMER-PLAY
name: Under Tamer Play
kind: tamer
color: [red]
cost: 3
effects:
  - when: main
    process:
      - select_under_tamer_sources:
          min: 1
          max: 1
          filter: { trait_has: Xros Heart }
          bind_as: chosen
          prompt: Choose a card under your Tamers
          then:
            - play_under_tamer_source:
                source_refs: chosen
                cost_delta: free
"#,
    );

    let mut runner = DebugRunner::builder()
        .add_card(card("TAMER", CardKind::Tamer, &[], 3))
        .add_card(card("DIGI-HOST", CardKind::Digimon, &[], 3))
        .add_card(card("PLAY-ME", CardKind::Digimon, &["Xros Heart"], 7))
        .memory(0)
        .start();
    let tamer = runner.place_on_field(0, "TAMER", None);
    let digimon = runner.place_on_field(0, "DIGI-HOST", None);
    let play_me = insert_source(&mut runner, tamer, "PLAY-ME");
    insert_source(&mut runner, digimon, "PLAY-ME");
    let source_card = runner.top_card(tamer);

    let outcome = {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(tamer), 0);
        let mut bindings = Bindings::new();
        run_steps(&steps, &mut ctx, &mut bindings)
    };

    assert_eq!(outcome, RunOutcome::Parked);
    let pending = runner.game.pending_selection.as_ref().unwrap();
    let tamer_source = encode_source_select(tamer.index as u16, 0).unwrap();
    let digimon_source = encode_source_select(digimon.index as u16, 0).unwrap();
    assert!(pending.valid_action_ids.contains(&tamer_source));
    assert!(!pending.valid_action_ids.contains(&digimon_source));

    runner
        .game
        .resolve_selection(pending.selecting_player, tamer_source)
        .expect("resolve under-Tamer source selection");

    assert!(runner.game.players[0]
        .battle_area
        .iter()
        .any(|perm| perm.top_card().handle() == play_me));
    assert_eq!(source_ids(&runner, tamer), vec!["TAMER"]);
}

#[test]
fn select_under_tamer_source_can_feed_effect_initiated_digivolve() {
    let steps = compile_steps(
        r#"
card: TEST-UNDER-TAMER-DIGIVOLVE
name: Under Tamer Digivolve
kind: digimon
color: [red]
level: 3
cost: 3
dp: 2000
effects:
  - when: when_attacking
    process:
      - select_under_tamer_sources:
          min: 1
          max: 1
          filter:
            name_contains: OmniShoutmon
          bind_as: evo_source
          prompt: Choose an under-Tamer card to digivolve into
          then:
            - effect_initiated_digivolve:
                target: source
                source: evo_source
                cost: free
"#,
    );

    let mut base_card = card("BASE3", CardKind::Digimon, &["Xros Heart"], 3);
    base_card.level = Some(3);
    let mut runner = DebugRunner::builder()
        .add_card(base_card)
        .add_card(card("TAMER", CardKind::Tamer, &[], 3))
        .add_card(evo_card("OmniShoutmon", &["Xros Heart"], 3))
        .memory(0)
        .start();
    let base = runner.place_on_field(0, "BASE3", None);
    let tamer = runner.place_on_field(0, "TAMER", None);
    let evo_handle = insert_source(&mut runner, tamer, "OmniShoutmon");
    let source_card = runner.top_card(base);

    {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(base), 0);
        let mut bindings = Bindings::new();
        assert_eq!(
            run_steps(&steps, &mut ctx, &mut bindings),
            RunOutcome::Parked
        );
    }

    let action = encode_source_select(tamer.index as u16, 0).unwrap();
    let pending = runner.game.pending_selection.as_ref().unwrap();
    assert!(pending.valid_action_ids.contains(&action));
    runner
        .game
        .resolve_selection(pending.selecting_player, action)
        .expect("choose under-Tamer evolution card");

    assert_eq!(
        runner.game.players[0].battle_area[base.index as usize]
            .top_card()
            .handle(),
        evo_handle
    );
    assert_eq!(source_ids(&runner, tamer), vec!["TAMER"]);
}

#[test]
fn under_tamer_source_digivolve_with_no_candidate_installs_no_hidden_selection() {
    let steps = compile_steps(
        r#"
card: TEST-UNDER-TAMER-NO-CANDIDATE
name: Under Tamer No Candidate
kind: digimon
color: [red]
level: 3
cost: 3
dp: 2000
effects:
  - when: when_attacking
    process:
      - select_under_tamer_sources:
          min: 1
          max: 1
          filter:
            name_contains: OmniShoutmon
          bind_as: evo_source
          prompt: Choose an under-Tamer card to digivolve into
          then:
            - effect_initiated_digivolve:
                target: source
                source: evo_source
                cost: free
"#,
    );

    let mut base_card = card("BASE3", CardKind::Digimon, &["Xros Heart"], 3);
    base_card.level = Some(3);
    let mut runner = DebugRunner::builder()
        .add_card(base_card)
        .add_card(card("TAMER", CardKind::Tamer, &[], 3))
        .add_card(evo_card("RaptorSparrowmon", &["Xros Heart"], 3))
        .memory(0)
        .start();
    let base = runner.place_on_field(0, "BASE3", None);
    let tamer = runner.place_on_field(0, "TAMER", None);
    let unrelated = insert_source(&mut runner, tamer, "RaptorSparrowmon");
    let source_card = runner.top_card(base);

    let outcome = {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(base), 0);
        let mut bindings = Bindings::new();
        run_steps(&steps, &mut ctx, &mut bindings)
    };

    assert_eq!(outcome, RunOutcome::Synchronous);
    assert!(
        runner.game.pending_selection.is_none(),
        "no legal under-Tamer source should not create a hidden selection"
    );
    assert_eq!(
        runner.game.players[0].battle_area[base.index as usize]
            .top_card()
            .card_id(&runner.game.card_data),
        "BASE3"
    );
    assert_eq!(
        runner.game.players[0].battle_area[tamer.index as usize].card_sources[0].handle(),
        unrelated,
        "non-matching source must remain under the Tamer"
    );
}

#[test]
fn under_tamer_source_digivolve_can_be_declined_before_source_moves() {
    let steps = compile_steps(
        r#"
card: TEST-UNDER-TAMER-DECLINE
name: Under Tamer Decline
kind: digimon
color: [red]
level: 3
cost: 3
dp: 2000
effects:
  - when: when_attacking
    process:
      - select_under_tamer_sources:
          min: 0
          max: 1
          filter:
            name_contains: OmniShoutmon
          bind_as: evo_source
          prompt: Choose an under-Tamer card to digivolve into
          then:
            - effect_initiated_digivolve:
                target: source
                source: evo_source
                cost: free
"#,
    );

    let mut base_card = card("BASE3", CardKind::Digimon, &["Xros Heart"], 3);
    base_card.level = Some(3);
    let mut runner = DebugRunner::builder()
        .add_card(base_card)
        .add_card(card("TAMER", CardKind::Tamer, &[], 3))
        .add_card(evo_card("OmniShoutmon", &["Xros Heart"], 3))
        .memory(0)
        .start();
    let base = runner.place_on_field(0, "BASE3", None);
    let tamer = runner.place_on_field(0, "TAMER", None);
    let evo_handle = insert_source(&mut runner, tamer, "OmniShoutmon");
    let source_card = runner.top_card(base);

    {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(base), 0);
        let mut bindings = Bindings::new();
        assert_eq!(
            run_steps(&steps, &mut ctx, &mut bindings),
            RunOutcome::Parked
        );
    }

    let pending = runner.game.pending_selection.as_ref().unwrap();
    assert!(pending.is_optional);
    assert!(pending.valid_action_ids.contains(&PASS));
    runner
        .game
        .resolve_selection(pending.selecting_player, PASS)
        .expect("decline under-Tamer digivolve");

    assert_eq!(
        runner.game.players[0].battle_area[base.index as usize]
            .top_card()
            .card_id(&runner.game.card_data),
        "BASE3"
    );
    assert_eq!(
        runner.game.players[0].battle_area[tamer.index as usize].card_sources[0].handle(),
        evo_handle,
        "declining must leave the selected card under the Tamer"
    );
}

#[test]
fn under_tamer_source_digivolve_offers_matching_cards_under_each_tamer() {
    let steps = compile_steps(
        r#"
card: TEST-UNDER-TAMER-MULTI
name: Under Tamer Multi
kind: digimon
color: [red]
level: 3
cost: 3
dp: 2000
effects:
  - when: when_attacking
    process:
      - select_under_tamer_sources:
          min: 1
          max: 1
          filter:
            name_contains: OmniShoutmon
          bind_as: evo_source
          prompt: Choose an under-Tamer card to digivolve into
          then:
            - effect_initiated_digivolve:
                target: source
                source: evo_source
                cost: free
"#,
    );

    let mut base_card = card("BASE3", CardKind::Digimon, &["Xros Heart"], 3);
    base_card.level = Some(3);
    let mut runner = DebugRunner::builder()
        .add_card(base_card)
        .add_card(card("TAMER-A", CardKind::Tamer, &[], 3))
        .add_card(card("TAMER-B", CardKind::Tamer, &[], 3))
        .add_card(evo_card("OmniShoutmon", &["Xros Heart"], 3))
        .memory(0)
        .start();
    let base = runner.place_on_field(0, "BASE3", None);
    let tamer_a = runner.place_on_field(0, "TAMER-A", None);
    let tamer_b = runner.place_on_field(0, "TAMER-B", None);
    insert_source(&mut runner, tamer_a, "OmniShoutmon");
    insert_source(&mut runner, tamer_b, "OmniShoutmon");
    let source_card = runner.top_card(base);

    {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(base), 0);
        let mut bindings = Bindings::new();
        assert_eq!(
            run_steps(&steps, &mut ctx, &mut bindings),
            RunOutcome::Parked
        );
    }

    let pending = runner.game.pending_selection.as_ref().unwrap();
    let tamer_a_action = encode_source_select(tamer_a.index as u16, 0).unwrap();
    let tamer_b_action = encode_source_select(tamer_b.index as u16, 0).unwrap();
    assert!(pending.valid_action_ids.contains(&tamer_a_action));
    assert!(pending.valid_action_ids.contains(&tamer_b_action));
}

#[test]
fn selected_hand_or_trash_card_can_be_placed_under_source_tamer() {
    let steps = compile_steps(
        r#"
card: TEST-UNDER-TAMER-PLACE
name: Under Tamer Place
kind: tamer
color: [red]
cost: 3
effects:
  - when: main
    process:
      - select_union_zone:
          of: you
          zones: [hand, trash]
          filter: { trait_has: Xros Heart }
          bind_as: picked
          prompt: Pick a card to stash
          then:
            - place_selected_card_under_tamer:
                card: picked
                tamer: source
                face_down: true
"#,
    );

    let mut runner = DebugRunner::builder()
        .add_card(card("TAMER", CardKind::Tamer, &[], 3))
        .add_card(card("HAND-XH", CardKind::Digimon, &["Xros Heart"], 3))
        .add_card(card("TRASH-XH", CardKind::Digimon, &["Xros Heart"], 3))
        .memory(0)
        .start();
    let tamer = runner.place_on_field(0, "TAMER", None);
    push_to_hand(&mut runner, 0, "HAND-XH");
    let trash_card = push_to_trash(&mut runner, 0, "TRASH-XH");
    let source_card = runner.top_card(tamer);

    {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(tamer), 0);
        let mut bindings = Bindings::new();
        assert_eq!(
            run_steps(&steps, &mut ctx, &mut bindings),
            RunOutcome::Parked
        );
    }

    let action = TRASH_EFFECT_START;
    let pending = runner.game.pending_selection.as_ref().unwrap();
    assert!(pending.valid_action_ids.contains(&action));
    runner
        .game
        .resolve_selection(pending.selecting_player, action)
        .expect("resolve trash-origin stash");

    let sources = &runner.game.players[0].battle_area[tamer.index as usize].card_sources;
    assert_eq!(sources[0].handle(), trash_card);
    assert!(sources[0].face_down);
    assert!(runner.game.players[0].trash.is_empty());
}

#[test]
fn moved_source_count_can_reduce_followup_play_cost() {
    let steps = compile_steps(
        r#"
card: TEST-MOVE-COUNT
name: Move Count
kind: digimon
color: [red]
level: 4
cost: 4
dp: 4000
effects:
  - when: main
    process:
      - select_own_permanent:
          filter: { kind: tamer }
          bind_as: stash_tamer
          prompt: Choose a Tamer
      - move_matching_sources_under_tamer:
          from: source
          tamer: stash_tamer
          filter: { trait_has: Xros Heart }
          bind_count_as: moved_count
      - select_hand:
          of: you
          filter: { trait_has: Xros Heart }
          bind_as: play_me
          prompt: Choose a Digimon to play
      - play_from_hand:
          of: you
          hand_index: play_me
          cost_delta:
            reduce_fn:
              binding_value: moved_count
"#,
    );

    let mut runner = DebugRunner::builder()
        .add_card(card("HOST", CardKind::Digimon, &["Xros Heart"], 4))
        .add_card(card("TAMER", CardKind::Tamer, &[], 3))
        .add_card(card("SRC-A", CardKind::Digimon, &["Xros Heart"], 4))
        .add_card(card("SRC-B", CardKind::Digimon, &["Xros Heart"], 4))
        .add_card(card("PLAY-HAND", CardKind::Digimon, &["Xros Heart"], 5))
        .memory(5)
        .start();
    let host = runner.place_on_field(0, "HOST", None);
    let tamer = runner.place_on_field(0, "TAMER", None);
    insert_source(&mut runner, host, "SRC-A");
    insert_source(&mut runner, host, "SRC-B");
    push_to_hand(&mut runner, 0, "PLAY-HAND");
    let source_card = runner.top_card(host);

    {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(host), 0);
        let mut bindings = Bindings::new();
        assert_eq!(
            run_steps(&steps, &mut ctx, &mut bindings),
            RunOutcome::Parked
        );
    }

    let tamer_action = encode_attack(0, tamer.index as u16);
    let pending = runner.game.pending_selection.as_ref().unwrap();
    runner
        .game
        .resolve_selection(pending.selecting_player, tamer_action)
        .expect("choose Tamer destination");

    let hand_action = PLAY_HAND_START;
    let pending = runner.game.pending_selection.as_ref().unwrap();
    runner
        .game
        .resolve_selection(pending.selecting_player, hand_action)
        .expect("choose hand card to play");

    assert_eq!(
        runner.game.memory, 2,
        "PLAY-HAND cost 5 reduced by 2 moved sources pays 3"
    );
    assert_eq!(source_ids(&runner, host), vec!["HOST"]);
    assert_eq!(source_ids(&runner, tamer), vec!["SRC-A", "SRC-B", "TAMER"]);
}

#[test]
fn trash_top_stacked_sources_and_no_source_target_filter_are_authorable() {
    let steps = compile_steps(
        r#"
card: TEST-STACK-PAYOFF
name: Stack Payoff
kind: option
color: [red]
cost: 3
effects:
  - when: main_from_hand
    process:
      - select_opponent_permanent:
          filter:
            kind: digimon
            materials_count_gte: 1
          bind_as: stacked
          prompt: Choose stacked Digimon
      - trash_top_stacked_sources:
          target: stacked
          count: 2
      - select_opponent_permanent:
          filter:
            kind: digimon
            materials_count_lte: 0
          bind_as: no_sources
          prompt: Choose Digimon with no sources
      - return_to_deck:
          target: no_sources
          position: bottom
"#,
    );

    let mut runner = DebugRunner::builder()
        .add_card(card("SRC", CardKind::Option, &[], 3))
        .add_card(card("STACKED", CardKind::Digimon, &[], 3))
        .add_card(card("NO-SOURCE", CardKind::Digimon, &[], 3))
        .add_card(card("MAT-A", CardKind::Digimon, &[], 3))
        .add_card(card("MAT-B", CardKind::Digimon, &[], 3))
        .memory(0)
        .start();
    let stacked = runner.place_on_field(1, "STACKED", None);
    let no_source = runner.place_on_field(1, "NO-SOURCE", None);
    insert_source(&mut runner, stacked, "MAT-A");
    insert_source(&mut runner, stacked, "MAT-B");
    let source_card = push_to_hand(&mut runner, 0, "SRC");

    {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, None, 0);
        let mut bindings = Bindings::new();
        assert_eq!(
            run_steps(&steps, &mut ctx, &mut bindings),
            RunOutcome::Parked
        );
    }

    let stacked_action = encode_attack(0, stacked.index as u16);
    let no_source_action = encode_attack(0, no_source.index as u16);
    let pending = runner.game.pending_selection.as_ref().unwrap();
    assert!(pending.valid_action_ids.contains(&stacked_action));
    assert!(!pending.valid_action_ids.contains(&no_source_action));
    runner
        .game
        .resolve_selection(pending.selecting_player, stacked_action)
        .expect("choose stacked target");

    assert_eq!(source_ids(&runner, stacked), vec!["STACKED"]);

    let pending = runner.game.pending_selection.as_ref().unwrap();
    assert!(pending.valid_action_ids.contains(&no_source_action));
    runner
        .game
        .resolve_selection(pending.selecting_player, no_source_action)
        .expect("choose no-source target");

    assert!(runner.game.players[1]
        .battle_area
        .iter()
        .all(|perm| perm.top_card().card_id(&runner.game.card_data) != "NO-SOURCE"));
}

#[test]
fn unsupported_xros_primitive_fields_report_explicit_compile_errors() {
    let under_tamer_errors = compile_errors(
        r#"
card: TEST-BAD-UNDER-TAMER
name: Bad Under Tamer
kind: tamer
color: [red]
cost: 3
effects:
  - when: main
    process:
      - select_under_tamer_sources:
          min: 1
          max: 1
          filter: { host_kind_is: digimon }
          bind_as: chosen
          prompt: Choose incorrectly filtered source
"#,
    );
    assert!(
        under_tamer_errors.contains("select_under_tamer_sources requires host_kind_is: tamer"),
        "unexpected errors: {under_tamer_errors}"
    );

    let wildcard_errors = compile_errors(
        r#"
card: TEST-BAD-WILDCARD
name: Bad Wildcard
kind: digimon
color: [red]
level: 4
cost: 4
dp: 4000
effects:
  - when: main
    process:
      - register_digixros_wildcard_for_turn:
          card: source
          zone: deck
"#,
    );
    assert!(
        wildcard_errors.contains("digixros wildcard zone must be one of hand, trash, or material"),
        "unexpected errors: {wildcard_errors}"
    );
}
