use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledCostDelta,
    CompiledDeclarativeClause, CompiledPredicate, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::enums::{CardColor, Keyword};
use digimon_engine::selection::SelectionKind;

use super::support::{
    field_contains, fire_when_digivolving, plain_digimon, select_first_non_pass, top_card_id,
    trash_contains, DebugRunner,
};

const CARD_ID: &str = "EX12-017";

#[test]
fn ex12_017_has_security_attack_plus_decode_counter_and_dna_paths() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-017 YAML loads")
        .start();
    let wargreymon = runner.place_on_field(0, CARD_ID, Some(0));
    let card = runner.compiled_card(CARD_ID).expect("compiled EX12-017");

    assert!(runner
        .game
        .has_keyword(wargreymon, Keyword::SecurityAttackPlus(1)));
    let dna_path = card
        .alt_paths
        .iter()
        .find(|path| {
            path.kind == CompiledAltPathKind::DnaDigivolve
                && path.cost == Some(CompiledCost::Literal(0))
        })
        .expect("printed Red/Yellow Lv.5 + Black/Purple Lv.5 DNA digivolve path");
    assert_eq!(dna_path.materials.len(), 2);
    assert_eq!(dna_path.materials[0].filter.level_eq, Some(5));
    assert!(
        dna_path.materials[0]
            .filter
            .color_only
            .as_ref()
            .is_some_and(|colors| {
                colors.contains(&CompiledColor::Red) && colors.contains(&CompiledColor::Yellow)
            }),
        "first DNA material is Red/Yellow Lv.5"
    );
    assert_eq!(dna_path.materials[1].filter.level_eq, Some(5));
    assert!(
        dna_path.materials[1]
            .filter
            .color_only
            .as_ref()
            .is_some_and(|colors| {
                colors.contains(&CompiledColor::Black) && colors.contains(&CompiledColor::Purple)
            }),
        "second DNA material is Black/Purple Lv.5"
    );

    let decode_replacements = card
        .effects
        .iter()
        .filter(|clause| {
            matches!(
                clause,
                CompiledClause::Declarative(CompiledDeclarativeClause::Replacement { .. })
            )
        })
        .count();
    assert_eq!(
        decode_replacements, 1,
        "WarGreymon has only its face-up Decode; the ingested inherited field is not printed text"
    );
    assert!(!card.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Declarative(CompiledDeclarativeClause::Replacement {
            scope: CompiledScope::Inherited,
            ..
        })
    )));

    let counter = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(triggered)
                if triggered.when.contains(&CompiledTiming::Counter) =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("Counter clause exists");
    assert!(counter.once_per_turn);
    assert!(counter.optional);
    assert!(counter
        .process
        .iter()
        .any(|step| matches!(step, CompiledStep::SelectHand { .. })));
    let counter_target_filter =
        find_select_hand_filter(&counter.process).expect("Counter chooses the DNA target in hand");
    assert!(
        predicate_has_name_is(counter_target_filter, "Omnimon"),
        "printed bracketed [Omnimon] target must be exact-name, not substring"
    );
    assert!(
        !predicate_has_name_contains(counter_target_filter, "Omnimon"),
        "`name_contains: Omnimon` would also match non-printed Omnimon variants"
    );
    assert!(walk_steps(
        &counter.process.iter().collect::<Vec<_>>(),
        |step| { matches!(step, CompiledStep::SelectDnaPair { .. }) }
    ));
    assert!(
        effect_dna_cost_is_printed(&counter.process),
        "counter DNA digivolve should pay the selected result's printed DNA cost"
    );
    assert!(walk_steps(
        &counter.process.iter().collect::<Vec<_>>(),
        |step| { matches!(step, CompiledStep::RedirectAttackTarget { .. }) }
    ));
}

#[test]
fn ex12_017_on_play_when_digivolving_when_attacking_clause_is_once_per_turn() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-017 YAML loads")
        .start();
    let card = runner.compiled_card(CARD_ID).expect("compiled EX12-017");

    let clause = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(triggered)
                if triggered.when.contains(&CompiledTiming::OnPlay)
                    && triggered.when.contains(&CompiledTiming::WhenDigivolving)
                    && triggered.when.contains(&CompiledTiming::WhenAttacking) =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("shared OP/WD/WA clause exists");

    assert!(clause.once_per_turn);
    assert!(
        !clause.optional,
        "printed de-digivolve/delete clause is mandatory"
    );
    assert!(clause
        .process
        .iter()
        .any(|step| matches!(step, CompiledStep::DeDigivolve { .. })));
}

#[test]
fn ex12_017_when_digivolving_de_digivolves_two_then_deletes_lowest_dp() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-017 YAML loads")
        .add_card(plain_digimon("OPP-BASE", CardColor::Red, 3, 8000))
        .add_card(plain_digimon("OPP-MID", CardColor::Red, 4, 5000))
        .add_card(plain_digimon("OPP-TOP", CardColor::Red, 5, 9000))
        .add_card(plain_digimon("OPP-LOW", CardColor::Red, 3, 1000))
        .memory(5)
        .start();
    let wargreymon = runner.place_on_field(0, CARD_ID, Some(0));
    let target = runner.place_stack(1, &["OPP-BASE", "OPP-MID", "OPP-TOP"]);
    runner.place_on_field(1, "OPP-LOW", Some(0));

    fire_when_digivolving(&mut runner, wargreymon);
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::OppField),
        "first prompt chooses the De-Digivolve target"
    );
    select_first_non_pass(&mut runner);
    // §16-12-1: the controller declares how many cards <De-Digivolve> trashes.
    assert!(runner.declare_max_count().expect("declare 2"));
    assert_eq!(
        top_card_id(&runner, 1, target.index as usize),
        "OPP-BASE",
        "De-Digivolve 2 should stop at the level 3 base"
    );

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::OppField),
        "then choose the lowest-DP Digimon to delete"
    );
    select_first_non_pass(&mut runner);
    runner.auto_resolve().expect("finish EX12-017 WD");

    assert!(trash_contains(&runner, 1, "OPP-TOP"));
    assert!(trash_contains(&runner, 1, "OPP-MID"));
    assert!(!field_contains(&runner, 1, "OPP-LOW"));
    assert!(field_contains(&runner, 1, "OPP-BASE"));
}

fn find_select_hand_filter(steps: &[CompiledStep]) -> Option<&CompiledPredicate> {
    for step in steps {
        if let CompiledStep::SelectHand { filter, .. } = step {
            return Some(filter);
        }
        if let Some(found) = match step {
            CompiledStep::If {
                then, else_branch, ..
            } => find_select_hand_filter(then).or_else(|| find_select_hand_filter(else_branch)),
            CompiledStep::Optional(body)
            | CompiledStep::AsSelectingPlayer { body, .. }
            | CompiledStep::ForEach { body, .. }
            | CompiledStep::PerSelected { body, .. }
            | CompiledStep::ScheduleDelayed { body, .. } => find_select_hand_filter(body),
            CompiledStep::SelectOwnPermanent { then, .. }
            | CompiledStep::SelectOpponentPermanent { then, .. }
            | CompiledStep::SelectAnyPermanent { then, .. }
            | CompiledStep::SelectTrash { then, .. }
            | CompiledStep::SelectOwnSources { then, .. }
            | CompiledStep::SelectOpponentSources { then, .. }
            | CompiledStep::SelectOpponentDpBudget { then, .. }
            | CompiledStep::SelectOpponentPlayCostBudget { then, .. }
            | CompiledStep::SelectOwnBreedingPermanent { then, .. }
            | CompiledStep::SelectReveal { then, .. }
            | CompiledStep::SelectSecurity { then, .. }
            | CompiledStep::SelectUnionZone { then, .. } => find_select_hand_filter(then),
            CompiledStep::SearchOwnSecurityStack {
                on_select,
                on_no_match,
                ..
            } => find_select_hand_filter(on_select)
                .or_else(|| on_no_match.as_deref().and_then(find_select_hand_filter)),
            _ => None,
        } {
            return Some(found);
        }
    }

    None
}

fn effect_dna_cost_is_printed(steps: &[CompiledStep]) -> bool {
    steps.iter().any(|step| match step {
        CompiledStep::EffectInitiatedDnaDigivolve { cost, .. } => {
            *cost == CompiledCostDelta::Printed
        }
        CompiledStep::If {
            then, else_branch, ..
        } => effect_dna_cost_is_printed(then) || effect_dna_cost_is_printed(else_branch),
        CompiledStep::Optional(body)
        | CompiledStep::AsSelectingPlayer { body, .. }
        | CompiledStep::ForEach { body, .. }
        | CompiledStep::PerSelected { body, .. }
        | CompiledStep::ScheduleDelayed { body, .. } => effect_dna_cost_is_printed(body),
        CompiledStep::SelectOwnPermanent { then, .. }
        | CompiledStep::SelectOpponentPermanent { then, .. }
        | CompiledStep::SelectAnyPermanent { then, .. }
        | CompiledStep::SelectHand { then, .. }
        | CompiledStep::SelectTrash { then, .. }
        | CompiledStep::SelectOwnSources { then, .. }
        | CompiledStep::SelectOpponentSources { then, .. }
        | CompiledStep::SelectOpponentDpBudget { then, .. }
        | CompiledStep::SelectOpponentPlayCostBudget { then, .. }
        | CompiledStep::SelectOwnBreedingPermanent { then, .. }
        | CompiledStep::SelectReveal { then, .. }
        | CompiledStep::SelectSecurity { then, .. }
        | CompiledStep::SelectUnionZone { then, .. } => effect_dna_cost_is_printed(then),
        CompiledStep::SearchOwnSecurityStack {
            on_select,
            on_no_match,
            ..
        } => {
            effect_dna_cost_is_printed(on_select)
                || on_no_match
                    .as_deref()
                    .is_some_and(effect_dna_cost_is_printed)
        }
        _ => false,
    })
}

fn predicate_has_name_is(predicate: &CompiledPredicate, needle: &str) -> bool {
    predicate.name_is.as_deref() == Some(needle)
        || predicate
            .all_of
            .iter()
            .any(|nested| predicate_has_name_is(nested, needle))
        || predicate
            .any_of
            .iter()
            .any(|nested| predicate_has_name_is(nested, needle))
}

fn predicate_has_name_contains(predicate: &CompiledPredicate, needle: &str) -> bool {
    predicate.name_contains.as_deref() == Some(needle)
        || predicate
            .all_of
            .iter()
            .any(|nested| predicate_has_name_contains(nested, needle))
        || predicate
            .any_of
            .iter()
            .any(|nested| predicate_has_name_contains(nested, needle))
}

fn walk_steps<F>(steps: &[&CompiledStep], pred: F) -> bool
where
    F: Fn(&CompiledStep) -> bool + Copy,
{
    for step in steps {
        if pred(step) {
            return true;
        }

        let nested: Vec<&CompiledStep> = match step {
            CompiledStep::If {
                then, else_branch, ..
            } => then.iter().chain(else_branch.iter()).collect(),
            CompiledStep::Optional(body)
            | CompiledStep::AsSelectingPlayer { body, .. }
            | CompiledStep::ForEach { body, .. }
            | CompiledStep::PerSelected { body, .. }
            | CompiledStep::ScheduleDelayed { body, .. } => body.iter().collect(),
            CompiledStep::SelectOwnPermanent { then, .. }
            | CompiledStep::SelectOpponentPermanent { then, .. }
            | CompiledStep::SelectAnyPermanent { then, .. }
            | CompiledStep::SelectHand { then, .. }
            | CompiledStep::SelectTrash { then, .. }
            | CompiledStep::SelectOwnSources { then, .. }
            | CompiledStep::SelectOpponentSources { then, .. }
            | CompiledStep::SelectOpponentDpBudget { then, .. }
            | CompiledStep::SelectOpponentPlayCostBudget { then, .. }
            | CompiledStep::SelectOwnBreedingPermanent { then, .. }
            | CompiledStep::SelectReveal { then, .. }
            | CompiledStep::SelectSecurity { then, .. }
            | CompiledStep::SelectUnionZone { then, .. } => then.iter().collect(),
            CompiledStep::SearchOwnSecurityStack {
                on_select,
                on_no_match,
                ..
            } => on_select
                .iter()
                .chain(on_no_match.iter().flatten())
                .collect(),
            _ => Vec::new(),
        };

        if !nested.is_empty() && walk_steps(&nested, pred) {
            return true;
        }
    }

    false
}

// ─── [Counter] in a real attack (audit 2026-11-10, user-confirmed) ───────────
//
// DCGO EX12_017.cs (Counter region):
// - the hand pick lists only cards that can DNA now (`CanPlayJogress(true)`);
// - the two Digimon must satisfy that card's DNA requirement;
// - the redirect is offered whether or not a DNA happened.

mod counter_runtime {
    use super::*;
    use digimon_engine::action::space::{HAND_EFFECT_START, PASS, PLAY_HAND_START};
    use digimon_engine::debug_runner::make_test_dna_card;
    use digimon_engine::enums::GamePhase;
    use digimon_engine::permanent::PermanentHandle;

    /// "Omnimon" that DNAs from two Lv.5 Digimon (cost 0).
    fn omnimon_lv5_pair() -> digimon_engine::card_data::CardData {
        let mut c = make_test_dna_card("OMNI-OK", "Omnimon", 5, 5, 0);
        c.card_kind = digimon_engine::enums::CardKind::Digimon;
        c.level = Some(7);
        c.dp = Some(15000);
        c
    }

    /// A [VB] Digimon whose DNA needs two Lv.6 Digimon: no legal pair here.
    fn vb_needs_lv6_pair() -> digimon_engine::card_data::CardData {
        let mut c = make_test_dna_card("VB-LV6PAIR", "Far Off", 6, 6, 0);
        c.card_kind = digimon_engine::enums::CardKind::Digimon;
        c.level = Some(7);
        c.dp = Some(15000);
        c.traits = vec!["VB".to_string()];
        c
    }

    struct Board {
        runner: DebugRunner,
        lv5_a: PermanentHandle,
        lv5_b: PermanentHandle,
        lv4: PermanentHandle,
    }

    /// Player 0 attacks player 1. Player 1 has WarGreymon, two Lv.5 Digimon
    /// and one Lv.4 Digimon, and `hand` in hand.
    fn attacked(hand: &[&str]) -> Board {
        let mut runner = DebugRunner::builder()
            .dsl_card(CARD_ID)
            .expect("EX12-017 YAML loads")
            .add_card(plain_digimon("ATK", CardColor::Blue, 5, 7000))
            .add_card(plain_digimon("LV5-A", CardColor::Red, 5, 7000))
            .add_card(plain_digimon("LV5-B", CardColor::Black, 5, 7000))
            .add_card(plain_digimon("LV4", CardColor::Green, 4, 5000))
            .add_card(omnimon_lv5_pair())
            .add_card(vb_needs_lv6_pair())
            .hand(1, hand)
            .memory(3)
            .start();
        let atk = runner.place_on_field(0, "ATK", Some(0));
        runner.place_on_field(1, CARD_ID, Some(0));
        let lv5_a = runner.place_on_field(1, "LV5-A", Some(0));
        let lv5_b = runner.place_on_field(1, "LV5-B", Some(0));
        let lv4 = runner.place_on_field(1, "LV4", Some(0));
        runner.attack_player(atk, 1, false);
        assert_eq!(runner.current_phase(), GamePhase::CounterTiming);
        Board { runner, lv5_a, lv5_b, lv4 }
    }

    fn hand_slot_actions(runner: &DebugRunner, id: &str) -> [u16; 2] {
        let slot = runner.game.players[1]
            .hand
            .iter()
            .position(|c| c.card_id(&runner.game.card_data) == id)
            .unwrap_or_else(|| panic!("{id} in player 1's hand")) as u16;
        [PLAY_HAND_START + slot, HAND_EFFECT_START + slot]
    }

    /// Take the Counter window and accept the optional effect, stopping at
    /// the first prompt of `kind`.
    fn advance_to(runner: &mut DebugRunner, kind: SelectionKind) -> digimon_engine::selection::PendingSelectionView {
        for _ in 0..6 {
            let view = runner.pending_selection_view().expect("a prompt is pending");
            // The Counter window is also a Hand-kind prompt; stop only at the
            // effect's own prompt.
            if view.kind == kind && !view.prompt.starts_with("Counter (") {
                return view;
            }
            let action = view
                .valid_action_ids
                .iter()
                .copied()
                .find(|&a| a != PASS)
                .expect("accept the counter / effect gate");
            runner.execute_action(view.selecting_player, action).expect("accept");
        }
        panic!("never reached a {kind:?} prompt");
    }

    fn offers(view: &digimon_engine::selection::PendingSelectionView, actions: &[u16]) -> bool {
        actions.iter().any(|a| view.valid_action_ids.contains(a))
    }

    #[test]
    fn hand_pick_offers_only_cards_that_can_dna_now() {
        let Board { mut runner, .. } = attacked(&["OMNI-OK", "VB-LV6PAIR"]);
        let card = runner.compiled_card(CARD_ID).expect("compiled EX12-017");
        let counter = card
            .effects
            .iter()
            .find_map(|clause| match clause {
                CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::Counter) => Some(t),
                _ => None,
            })
            .expect("Counter clause");
        let filter = find_select_hand_filter(&counter.process).expect("hand pick");
        assert!(
            filter.all_of.iter().any(|p| p.dna_field_pair.is_some()),
            "the hand pick carries the dna_field_pair legality filter"
        );
        let view = advance_to(&mut runner, SelectionKind::Hand);
        assert!(offers(&view, &hand_slot_actions(&runner, "OMNI-OK")));
        assert!(
            !offers(&view, &hand_slot_actions(&runner, "VB-LV6PAIR")),
            "no pair of Lv.6 Digimon: DCGO CanPlayJogress excludes it"
        );
    }

    #[test]
    fn pair_picks_offer_only_digimon_that_meet_the_dna_requirement() {
        use digimon_engine::action::space::encode_attack;
        let Board { mut runner, lv5_a, lv5_b, lv4 } = attacked(&["OMNI-OK"]);
        let view = advance_to(&mut runner, SelectionKind::Hand);
        let pick = hand_slot_actions(&runner, "OMNI-OK")
            .into_iter()
            .find(|a| view.valid_action_ids.contains(a))
            .unwrap();
        runner.execute_action(1, pick).expect("pick Omnimon");

        let left = runner.pending_selection_view().expect("first DNA material pick");
        let act = |h: PermanentHandle| encode_attack(h.player as u16, h.index as u16);
        assert!(left.valid_action_ids.contains(&act(lv5_a)));
        assert!(left.valid_action_ids.contains(&act(lv5_b)));
        assert!(
            !left.valid_action_ids.contains(&act(lv4)),
            "a Lv.4 Digimon is not a Lv.5 DNA material"
        );
        runner.execute_action(1, act(lv5_a)).expect("first material");
        let right = runner.pending_selection_view().expect("second DNA material pick");
        assert!(right.valid_action_ids.contains(&act(lv5_b)));
        assert!(!right.valid_action_ids.contains(&act(lv4)));
        assert!(!right.valid_action_ids.contains(&act(lv5_a)));
    }

    #[test]
    fn redirect_is_offered_even_without_a_dna() {
        let Board { mut runner, .. } = attacked(&["OMNI-OK"]);
        let view = advance_to(&mut runner, SelectionKind::Hand);
        assert!(view.is_optional);
        runner.execute_action(1, PASS).expect("decline the DNA");
        let next = runner
            .pending_selection_view()
            .expect("the redirect is still offered after declining the DNA");
        assert!(
            next.prompt.contains("attack target"),
            "expected the redirect prompt, got {:?}",
            next.prompt
        );
    }
}
