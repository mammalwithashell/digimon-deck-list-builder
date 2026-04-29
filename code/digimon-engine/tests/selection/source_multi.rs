use std::sync::{Arc, Mutex};

use digimon_engine::action::space::{encode_source_select, PASS};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::{EffectContext, SourceSelectionRef};
use digimon_engine::enums::GamePhase;
use digimon_engine::selection::SelectionKind;
use digimon_engine::CardHandle;

fn ids(picks: &[SourceSelectionRef]) -> Vec<CardHandle> {
    picks.iter().map(|p| p.card).collect()
}

fn runner_with_source_cards() -> DebugRunner {
    DebugRunner::builder()
        .add_card(make_test_card("SRC-A", "Source A"))
        .add_card(make_test_card("SRC-B", "Source B"))
        .add_card(make_test_card("SRC-C", "Source C"))
        .add_card(make_test_card("TOP-A", "Top A"))
        .add_card(make_test_card("TOP-B", "Top B"))
        .start()
}

#[test]
fn exact_two_sources_can_be_selected_across_own_battle_area() {
    let mut r = runner_with_source_cards();
    let p0 = 0;
    let first = r.place_stack(p0, &["SRC-A", "SRC-B", "TOP-A"]);
    let second = r.place_stack(p0, &["SRC-C", "TOP-B"]);
    let first_top = r.top_card(first);
    let second_top = r.top_card(second);

    let picked = Arc::new(Mutex::new(Vec::new()));
    let picked_slot = Arc::clone(&picked);
    {
        let mut ctx = EffectContext::new(&mut r.game, first_top, Some(first), p0);
        ctx.select_own_sources(
            "choose two sources",
            2,
            2,
            move |_, source| source.card != first_top && source.card != second_top,
            move |ctx, sources| {
                for source in sources.iter() {
                    ctx.trash_card_source(source.permanent, source.card);
                }
                *picked_slot.lock().unwrap() = sources;
            },
        );
    }

    assert_eq!(r.game.current_phase, GamePhase::SelectSource);
    let sel = r.game.pending_selection.as_ref().expect("source selection");
    assert_eq!(
        sel.kind,
        SelectionKind::SourceMulti {
            min: 2,
            max: 2,
            picked: 0
        }
    );
    assert!(!sel.valid_action_ids.contains(&PASS));
    assert!(sel
        .valid_action_ids
        .contains(&encode_source_select(first.index as u16, 0).unwrap()));
    assert!(sel
        .valid_action_ids
        .contains(&encode_source_select(first.index as u16, 1).unwrap()));
    assert!(sel
        .valid_action_ids
        .contains(&encode_source_select(second.index as u16, 0).unwrap()));

    r.game
        .resolve_selection(p0, encode_source_select(first.index as u16, 1).unwrap())
        .expect("pick source B");
    assert_eq!(
        r.game.pending_selection.as_ref().unwrap().kind,
        SelectionKind::SourceMulti {
            min: 2,
            max: 2,
            picked: 1
        }
    );
    assert!(!r
        .game
        .pending_selection
        .as_ref()
        .unwrap()
        .valid_action_ids
        .contains(&PASS));

    r.game
        .resolve_selection(p0, encode_source_select(second.index as u16, 0).unwrap())
        .expect("pick source C");

    let chosen = picked.lock().unwrap().clone();
    assert_eq!(ids(&chosen).len(), 2);
    assert!(
        r.game
            .player(p0)
            .trash
            .iter()
            .any(|c| c.handle() == chosen[0].card),
        "first selected source was trashed by stable handle"
    );
    assert!(
        r.game
            .player(p0)
            .trash
            .iter()
            .any(|c| c.handle() == chosen[1].card),
        "second selected source was trashed by stable handle"
    );
}

#[test]
fn up_to_sources_enables_pass_only_after_minimum_is_met() {
    let mut r = runner_with_source_cards();
    let p0 = 0;
    let source_stack = r.place_stack(p0, &["SRC-A", "SRC-B", "TOP-A"]);
    let source_stack_top = r.top_card(source_stack);

    let picked = Arc::new(Mutex::new(Vec::new()));
    let picked_slot = Arc::clone(&picked);
    {
        let mut ctx = EffectContext::new(&mut r.game, source_stack_top, Some(source_stack), p0);
        ctx.select_own_sources(
            "choose up to two sources",
            1,
            2,
            move |_, source| source.card != source_stack_top,
            move |_, sources| {
                *picked_slot.lock().unwrap() = sources;
            },
        );
    }

    assert!(!r
        .game
        .pending_selection
        .as_ref()
        .unwrap()
        .valid_action_ids
        .contains(&PASS));
    r.game
        .resolve_selection(p0, encode_source_select(source_stack.index as u16, 0).unwrap())
        .expect("pick one");
    assert!(r
        .game
        .pending_selection
        .as_ref()
        .unwrap()
        .valid_action_ids
        .contains(&PASS));
    r.game.resolve_selection(p0, PASS).expect("commit early");
    assert_eq!(picked.lock().unwrap().len(), 1);
}
