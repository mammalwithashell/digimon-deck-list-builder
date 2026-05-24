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
    let first_source_a = encode_source_select(first.index as u16, 0).unwrap();
    let first_source_b = encode_source_select(first.index as u16, 1).unwrap();
    let first_top_action = encode_source_select(first.index as u16, 2).unwrap();
    let second_source_c = encode_source_select(second.index as u16, 0).unwrap();
    let second_top_action = encode_source_select(second.index as u16, 1).unwrap();

    assert!(sel.valid_action_ids.contains(&first_source_a));
    assert!(sel.valid_action_ids.contains(&first_source_b));
    assert!(sel.valid_action_ids.contains(&second_source_c));
    assert!(
        !sel.valid_action_ids.contains(&first_top_action),
        "top card of first stack must not be selectable as a source"
    );
    assert!(
        !sel.valid_action_ids.contains(&second_top_action),
        "top card of second stack must not be selectable as a source"
    );

    r.game
        .resolve_selection(p0, first_source_b)
        .expect("pick source B");
    let sel = r
        .game
        .pending_selection
        .as_ref()
        .expect("selection should continue after first source");
    assert_eq!(
        sel.kind,
        SelectionKind::SourceMulti {
            min: 2,
            max: 2,
            picked: 1
        }
    );
    assert!(!sel.valid_action_ids.contains(&PASS));
    assert!(
        !sel.valid_action_ids.contains(&first_source_b),
        "already-picked source must not remain legal"
    );
    assert!(
        !sel.valid_action_ids.contains(&first_top_action),
        "top card of first stack must remain illegal"
    );
    assert!(
        !sel.valid_action_ids.contains(&second_top_action),
        "top card of second stack must remain illegal"
    );

    r.game
        .resolve_selection(p0, second_source_c)
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
        .resolve_selection(
            p0,
            encode_source_select(source_stack.index as u16, 0).unwrap(),
        )
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

// ── Picker live revalidation on stale pick (G-DSL-TRASH-SOURCES-STALE-HANDLE) ─

/// Regression for `G-DSL-TRASH-SOURCES-STALE-HANDLE` /
/// `fix-trash-card-source-stale-handle`. `install_source_multi_selection`
/// snapshots candidate `SourceSelectionRef`s into `action_to_source` at
/// install time. If an intervening observer effect drains a candidate's
/// source between install and submit, the submit callback must NOT push the
/// stale ref onto `next_picked`; it must re-install with the prior valid
/// picks and refreshed candidates. Mirrors DCGO
/// `SelectCardEffect.SetUp(... customRootCardList: selectedPermanent.DigivolutionCards ...)`
/// (DCGO/Assets/Scripts/Script/CardEffectCommons/TrashDigivolutionCards.cs:125)
/// which reads the live list at display time.
///
/// Asserted: when min=1, max=1 and the only candidate vanishes, submit →
/// re-install → empty candidates → `picked.len() < min` → no-op (no
/// final-callback fires, no panic). When min=0, max=1 and the candidate
/// vanishes, submit → re-install → empty candidates → `picked.len() >= min`
/// → final-callback fires with empty picks.
#[test]
fn source_multi_picker_re_installs_on_stale_pick_min_one_no_op() {
    let mut r = runner_with_source_cards();
    let p0 = 0;
    let stack = r.place_stack(p0, &["SRC-A", "TOP-A"]);
    let stack_top = r.top_card(stack);

    let final_called = Arc::new(Mutex::new(false));
    let final_called_slot = Arc::clone(&final_called);
    {
        let mut ctx = EffectContext::new(&mut r.game, stack_top, Some(stack), p0);
        ctx.select_own_sources(
            "pick exactly one mineral source",
            1,
            1,
            move |_, source| source.card != stack_top,
            move |_, _sources| {
                *final_called_slot.lock().unwrap() = true;
            },
        );
    }

    // Picker offers exactly one candidate: slot 0, source 0 (SRC-A).
    let stale_action = encode_source_select(stack.index as u16, 0).unwrap();
    let sel = r.game.pending_selection.as_ref().expect("source picker open");
    assert_eq!(
        sel.kind,
        SelectionKind::SourceMulti {
            min: 1,
            max: 1,
            picked: 0
        }
    );
    assert!(sel.valid_action_ids.contains(&stale_action));

    // Simulate an intervening observer trashing SRC-A directly between install
    // and submit. After this, the picker's snapshot candidate is stale: it
    // still references SRC-A's CardHandle but SRC-A is no longer in the stack.
    r.game.players[p0 as usize].battle_area[stack.index as usize]
        .card_sources
        .remove(0);
    assert_eq!(
        r.game.players[p0 as usize].battle_area[stack.index as usize]
            .card_sources
            .len(),
        1,
        "only TOP-A remains in slot 0 (no source below top)"
    );

    // Agent submits the now-stale action_id.
    r.game
        .resolve_selection(p0, stale_action)
        .expect("submitting a stale-but-once-valid action must not error");

    // The picker should NOT have called final_callback (min=1, picked=0 after
    // refusing the stale pick, re-install finds no candidates, return without
    // final). The pending_selection should be None (cleared by
    // resolve_selection; not re-installed because empty-candidates branch
    // returned early).
    assert!(
        !*final_called.lock().unwrap(),
        "final callback must NOT fire (picked.len() < min, no fresh candidates)"
    );
    assert!(
        r.game.pending_selection.is_none(),
        "pending selection must clear after stale-pick re-install runs the empty-candidates branch"
    );
}

#[test]
fn source_multi_picker_re_installs_on_stale_pick_min_zero_finalizes_empty() {
    let mut r = runner_with_source_cards();
    let p0 = 0;
    let stack = r.place_stack(p0, &["SRC-A", "TOP-A"]);
    let stack_top = r.top_card(stack);

    let picked = Arc::new(Mutex::new(Vec::new()));
    let picked_slot = Arc::clone(&picked);
    let final_called = Arc::new(Mutex::new(false));
    let final_called_slot = Arc::clone(&final_called);
    {
        let mut ctx = EffectContext::new(&mut r.game, stack_top, Some(stack), p0);
        ctx.select_own_sources(
            "pick 0..=1 mineral sources",
            0,
            1,
            move |_, source| source.card != stack_top,
            move |_, sources| {
                *picked_slot.lock().unwrap() = sources;
                *final_called_slot.lock().unwrap() = true;
            },
        );
    }

    let stale_action = encode_source_select(stack.index as u16, 0).unwrap();
    assert!(r
        .game
        .pending_selection
        .as_ref()
        .expect("picker open")
        .valid_action_ids
        .contains(&stale_action));

    // Stale-mutate.
    r.game.players[p0 as usize].battle_area[stack.index as usize]
        .card_sources
        .remove(0);

    // Submit the stale action.
    r.game
        .resolve_selection(p0, stale_action)
        .expect("stale submit must not error");

    // min=0 path: re-install lands with empty candidates AND picked.len()=0 >= min=0,
    // so final_callback fires with an empty Vec.
    assert!(
        *final_called.lock().unwrap(),
        "final callback fires when picked.len() >= min, even with empty actuals"
    );
    assert!(
        picked.lock().unwrap().is_empty(),
        "final callback receives empty picks (no stale ref leaked through)"
    );
    assert!(
        r.game.pending_selection.is_none(),
        "pending selection cleared after final callback"
    );
}
