//! G-MULTI-SELECT-OPP-PLAY-COST-SUM: DSL play-cost-budget multi-select.
//!
//! Play-cost analog of `phase2g_dp_budget.rs`. `select_opponent_play_cost_budget`
//! lets the player pick 0..N opponent permanents whose running printed
//! play-cost sum never exceeds a budget; any single permanent costing more
//! than the budget is excluded outright. The optional `filter` restricts the
//! candidate set (e.g. `kind: digimon`).

use digimon_dsl::compiled::{CompiledPredicate, CompiledStep, CompiledZone};
use digimon_engine::action::space::{encode_attack, PASS, TRASH_EFFECT_START};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::{run_steps, RunOutcome};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::CardKind;

/// A Digimon test card with an explicit printed play cost.
fn digimon_with_cost(id: &str, name: &str, play_cost: u16) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(4000);
    card.play_cost = play_cost;
    card
}

/// A Tamer test card with an explicit printed play cost.
fn tamer_with_cost(id: &str, name: &str, play_cost: u16) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Tamer;
    card.play_cost = play_cost;
    card
}

fn budget_step(then: Vec<CompiledStep>) -> CompiledStep {
    CompiledStep::SelectOpponentPlayCostBudget {
        play_cost_budget: 6,
        min_picks: 0,
        filter: CompiledPredicate::default(),
        bind_as: Some("targets".to_string()),
        prompt: "Choose opponents within play-cost budget".to_string(),
        then,
    }
}

fn trash_budget_step(then: Vec<CompiledStep>) -> CompiledStep {
    CompiledStep::SelectPlayCostBudget {
        of: digimon_dsl::compiled::CompiledPlayerRef::You,
        zone: CompiledZone::Trash,
        play_cost_budget: 6,
        min_picks: 0,
        filter: CompiledPredicate::default(),
        bind_as: Some("picks".to_string()),
        prompt: "Play trash cards within play-cost budget".to_string(),
        optional_zero: true,
        then,
    }
}

fn push_to_trash(runner: &mut DebugRunner, player: usize, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("{card_id} not found in card_data"));
    let next = runner.game.next_card_index();
    runner.game.players[player]
        .trash
        .push(CardSource::new(data_idx, player as u8, next));
}

fn player_trash_ids(runner: &DebugRunner, player: usize) -> Vec<String> {
    runner.game.players[player]
        .trash
        .iter()
        .map(|card| card.card_id(&runner.game.card_data).to_string())
        .collect()
}

/// Two opponent Digimon, costs 3 and 4 — running sum 7 > 6. After picking the
/// cost-4 Digimon the remaining budget is 2, so the cost-3 Digimon no longer
/// fits and the selection auto-resolves with just the one pick (the budget
/// trampoline ends when no candidate fits the remaining budget).
#[test]
fn play_cost_budget_caps_running_sum() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "Source"))
        .add_card(digimon_with_cost("C3", "Cost3", 3))
        .add_card(digimon_with_cost("C4", "Cost4", 4))
        .start();
    let source = runner.place_on_field(0, "SRC", Some(0));
    let source_card = runner.top_card(source);
    let _c3 = runner.place_on_field(1, "C3", Some(0));
    let c4 = runner.place_on_field(1, "C4", Some(0));

    let steps = vec![budget_step(vec![CompiledStep::DeleteBoundPermanents {
        binding: "targets".to_string(),
    }])];

    let mut bindings = Bindings::new();
    let outcome = {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(source), 0);
        run_steps(&steps, &mut ctx, &mut bindings)
    };
    assert_eq!(outcome, RunOutcome::Parked);

    // Before any pick, both Digimon are eligible (each individually ≤ 6).
    let view = runner.pending_selection().expect("budget selection open");
    assert_eq!(
        view.valid_action_ids.iter().filter(|a| **a != PASS).count(),
        2,
        "both opponent Digimon eligible before any pick"
    );

    // Pick C4 (cost 4) — remaining budget drops to 2; C3 (cost 3) no longer
    // fits, so the trampoline auto-resolves the selection with the single pick.
    runner
        .game
        .resolve_selection(0, encode_attack(0, c4.index as u16))
        .expect("pick C4");
    assert!(
        runner.game.pending_selection.is_none(),
        "selection auto-resolves once no candidate fits the remaining budget"
    );

    // Only C4 was deleted; C3 survives (running sum cap honored).
    let surviving: Vec<_> = runner
        .game
        .player(1)
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&runner.game.card_data).to_string())
        .collect();
    assert_eq!(
        surviving,
        vec!["C3".to_string()],
        "only the cost-4 Digimon deleted; cost-3 survives the budget cap"
    );
}

/// A single opponent Digimon costing 8 — above the budget of 6 — must be
/// ineligible outright (per-card cap, no selection installs).
#[test]
fn play_cost_budget_excludes_single_over_budget_target() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "Source"))
        .add_card(digimon_with_cost("C8", "Cost8", 8))
        .start();
    let source = runner.place_on_field(0, "SRC", Some(0));
    let source_card = runner.top_card(source);
    runner.place_on_field(1, "C8", Some(0));

    let steps = vec![budget_step(vec![CompiledStep::DeleteBoundPermanents {
        binding: "targets".to_string(),
    }])];

    let mut bindings = Bindings::new();
    let outcome = {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(source), 0);
        run_steps(&steps, &mut ctx, &mut bindings)
    };
    // No eligible candidate → step no-ops and the dispatcher continues.
    assert_eq!(outcome, RunOutcome::Synchronous);
    assert!(
        runner.game.pending_selection.is_none(),
        "no selection installs when the only candidate is over budget"
    );
    assert_eq!(
        runner.game.player(1).battle_area.len(),
        1,
        "the over-budget Digimon must survive (never selectable)"
    );
}

/// The optional `filter` restricts the candidate set — a `kind: digimon`
/// filter excludes an opponent Tamer even when its play cost is within budget.
#[test]
fn play_cost_budget_filter_excludes_non_matching_kind() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "Source"))
        .add_card(digimon_with_cost("DGM", "OppDigimon", 3))
        .add_card(tamer_with_cost("TMR", "OppTamer", 2))
        .start();
    let source = runner.place_on_field(0, "SRC", Some(0));
    let source_card = runner.top_card(source);
    let dgm = runner.place_on_field(1, "DGM", Some(0));
    let tmr = runner.place_on_field(1, "TMR", Some(0));

    let mut digimon_only = CompiledPredicate::default();
    digimon_only.kind = Some(digimon_dsl::compiled::CompiledCardKind::Digimon);

    let steps = vec![CompiledStep::SelectOpponentPlayCostBudget {
        play_cost_budget: 6,
        min_picks: 0,
        filter: digimon_only,
        bind_as: Some("targets".to_string()),
        prompt: "Delete opponent Digimon within budget".to_string(),
        then: vec![CompiledStep::DeleteBoundPermanents {
            binding: "targets".to_string(),
        }],
    }];

    let mut bindings = Bindings::new();
    let outcome = {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(source), 0);
        run_steps(&steps, &mut ctx, &mut bindings)
    };
    assert_eq!(outcome, RunOutcome::Parked);

    let view = runner.pending_selection().expect("selection installed");
    assert!(
        view.valid_action_ids
            .contains(&encode_attack(0, dgm.index as u16)),
        "opponent Digimon (cost 3) must be eligible"
    );
    assert!(
        !view
            .valid_action_ids
            .contains(&encode_attack(0, tmr.index as u16)),
        "opponent Tamer must be excluded by the kind: digimon filter; valid={:?}",
        view.valid_action_ids
    );

    runner.game.resolve_selection(0, PASS).expect("decline");
}

/// Visible-zone aggregate budget selection: the controller may stop early,
/// and only the selected trash cards are played from their true origin.
#[test]
fn trash_play_cost_budget_can_stop_early_and_play_selected_cards() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "Source"))
        .add_card(digimon_with_cost("C2", "Cost2", 2))
        .add_card(digimon_with_cost("C3", "Cost3", 3))
        .add_card(digimon_with_cost("C4", "Cost4", 4))
        .start();
    let source = runner.place_on_field(0, "SRC", Some(0));
    let source_card = runner.top_card(source);
    push_to_trash(&mut runner, 0, "C2");
    push_to_trash(&mut runner, 0, "C3");
    push_to_trash(&mut runner, 0, "C4");

    let steps = vec![trash_budget_step(vec![CompiledStep::PlayFromTrashFree {
        of: digimon_dsl::compiled::CompiledPlayerRef::You,
        trash_index: digimon_dsl::compiled::CompiledBindingRef::Named("picks".to_string()),
        suppress_on_play: false,
    }])];

    let mut bindings = Bindings::new();
    let outcome = {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(source), 0);
        run_steps(&steps, &mut ctx, &mut bindings)
    };
    assert_eq!(outcome, RunOutcome::Parked);

    let view = runner
        .pending_selection()
        .expect("trash budget selection open");
    assert!(
        view.valid_action_ids.contains(&(TRASH_EFFECT_START + 1)),
        "C3 starts eligible; valid={:?}",
        view.valid_action_ids
    );

    runner
        .game
        .resolve_selection(0, TRASH_EFFECT_START + 1)
        .expect("pick C3");

    let view = runner
        .pending_selection()
        .expect("selection remains open after one pick");
    assert!(
        view.is_optional,
        "controller can stop early after picking fewer than the budget"
    );
    runner.game.resolve_selection(0, PASS).expect("stop early");

    let played_ids: Vec<_> = runner.game.players[0]
        .battle_area
        .iter()
        .map(|perm| perm.top_card().card_id(&runner.game.card_data).to_string())
        .collect();
    assert!(
        played_ids.contains(&"C3".to_string()),
        "selected C3 should be played from trash"
    );
    assert_eq!(
        player_trash_ids(&runner, 0),
        vec!["C2".to_string(), "C4".to_string()],
        "unselected trash cards stay in their origin zone"
    );
}

/// After spending part of the budget, candidates above the remaining budget
/// must disappear from the mask while affordable unselected cards remain legal.
#[test]
fn trash_play_cost_budget_masks_candidates_over_remaining_budget() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "Source"))
        .add_card(digimon_with_cost("C2", "Cost2", 2))
        .add_card(digimon_with_cost("C3", "Cost3", 3))
        .add_card(digimon_with_cost("C4", "Cost4", 4))
        .add_card(digimon_with_cost("C8", "Cost8", 8))
        .start();
    let source = runner.place_on_field(0, "SRC", Some(0));
    let source_card = runner.top_card(source);
    push_to_trash(&mut runner, 0, "C2");
    push_to_trash(&mut runner, 0, "C3");
    push_to_trash(&mut runner, 0, "C4");
    push_to_trash(&mut runner, 0, "C8");

    let steps = vec![trash_budget_step(vec![CompiledStep::PlayFromTrashFree {
        of: digimon_dsl::compiled::CompiledPlayerRef::You,
        trash_index: digimon_dsl::compiled::CompiledBindingRef::Named("picks".to_string()),
        suppress_on_play: false,
    }])];

    let mut bindings = Bindings::new();
    {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(source), 0);
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    let view = runner
        .pending_selection()
        .expect("trash budget selection open");
    assert!(
        !view.valid_action_ids.contains(&(TRASH_EFFECT_START + 3)),
        "over-budget C8 should not be eligible initially; valid={:?}",
        view.valid_action_ids
    );
    runner
        .game
        .resolve_selection(0, TRASH_EFFECT_START + 1)
        .expect("pick C3");

    let view = runner
        .pending_selection()
        .expect("selection remains open after one pick");
    assert!(
        view.valid_action_ids.contains(&(TRASH_EFFECT_START)),
        "C2 fits remaining budget 3; valid={:?}",
        view.valid_action_ids
    );
    assert!(
        !view.valid_action_ids.contains(&(TRASH_EFFECT_START + 2)),
        "C4 exceeds remaining budget 3 and must be masked out; valid={:?}",
        view.valid_action_ids
    );

    runner.game.resolve_selection(0, PASS).expect("stop early");
    assert_eq!(
        player_trash_ids(&runner, 0),
        vec!["C2".to_string(), "C4".to_string(), "C8".to_string()],
        "all unselected cards stay in trash"
    );
}
