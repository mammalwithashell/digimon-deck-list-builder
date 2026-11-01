use super::*;
use crate::{debug_runner::{make_test_card, DebugRunner}, trigger_context::TriggerContext};
use std::{panic::{catch_unwind, AssertUnwindSafe}, sync::{Arc, atomic::{AtomicUsize, Ordering}}};

fn fixture(marked: bool, nested: bool) -> (Game, PermanentHandle) {
    let mut r = DebugRunner::builder().add_card(make_test_card("FG-S", "Source"))
        .add_card(make_test_card("FG-T", "Top"))
        .hand(0, &["FG-S"; 3]).security(0, &["FG-S"]).memory(5).start();
    let host = r.place_stack(0, &["FG-S", "FG-T"]);
    let breeding = r.place_stack(0, &["FG-T"]);
    r.game.players[0].breeding_area = Some(r.game.players[0].battle_area.remove(breeding.index as usize));
    let trash = r.game.players[0].hand.pop().unwrap();
    r.game.players[0].put_in_trash(trash);
    let revealed = r.game.players[0].hand.pop().unwrap();
    r.game.revealed_cards.push(revealed);
    let mut ctx = TriggerContext::default(); ctx.projected_reference_reads_guarded = marked;
    if nested { let mut parent = TriggerContext::default(); parent.simultaneous_play_members.push(ctx); ctx = parent; }
    r.game.current_trigger_context = Some(ctx);
    (r.game, host)
}

fn initial(path: u8) {
    for nested in [false, true] { for marked in [false, true] { for allowed in [false, true] {
        let (mut game, host) = fixture(marked, nested);
        let source = game.players[0].battle_area[0].top_card().handle();
        let calls = Arc::new(AtomicUsize::new(0)); let counter = calls.clone();
        let before_phase = game.current_phase;
        let result = catch_unwind(AssertUnwindSafe(|| {
            let mut ctx = EffectContext::new(&mut game, source, Some(host), 0);
            macro_rules! filter { () => { move |_, _| { counter.fetch_add(1, Ordering::SeqCst); allowed } }; }
            match path {
                0 => ctx.select_hand(0, "guard", false, filter!(), |_,_| {}),
                1 => ctx.select_trash(0, "guard", false, filter!(), |_,_| {}),
                2 => ctx.select_material(host, "guard", false, filter!(), |_,_| {}),
                3 => { ctx.select_reveal("guard", false, filter!(), |_,_| {}); },
                4 => ctx.select_security(0, "guard", false, filter!(), |_,_| {}),
                5 => ctx.search_own_security_stack("guard", false, filter!(), |_,_| {}),
                6 => ctx.select_own_permanent("guard", false, filter!(), |_,_| {}),
                7 => ctx.select_own_breeding_permanent("guard", false, filter!(), |_,_| {}),
                8 => ctx.select_count_capped_multi_min(0, CountCappedZone::Hand, 1, 1, "guard", false, None, filter!(), |_,_| {}),
                _ => unreachable!()
            }
        }));
        if marked {
            assert!(result.is_err(), "path {path} must reject");
            assert_eq!(calls.load(Ordering::SeqCst), 0, "path {path} body must not run");
            assert!(game.pending_selection.is_none());
            assert_eq!(game.current_phase, before_phase);
        } else {
            assert!(result.is_ok(), "path {path}");
            assert!(calls.load(Ordering::SeqCst) > 0);
            // Security intentionally retains inspection/Done even without eligible cards.
            if path != 4 && path != 5 { assert_eq!(game.pending_selection.is_some(), allowed); }
            else if allowed { assert!(game.pending_selection.is_some()); }
        }
    } } }
}
macro_rules! initial_test { ($name:ident, $path:expr) => { #[test] fn $name() { initial($path); } }; }
initial_test!(hand_filter_checks_before_body, 0);
initial_test!(trash_filter_checks_before_body, 1);
initial_test!(material_filter_checks_before_body, 2);
initial_test!(reveal_filter_checks_before_body, 3);
initial_test!(initial_security_filter_checks_before_body, 4);
initial_test!(security_search_filter_checks_before_body, 5);
initial_test!(field_filter_checks_before_body, 6);
initial_test!(breeding_filter_checks_before_body, 7);
initial_test!(count_capped_filter_checks_before_body, 8);

#[test]
fn deferred_candidate_filters_check_current_context_at_invocation() {
    for path in 0..3 { for nested in [false, true] { for marked in [false, true] { for allowed in [false, true] {
        let (game, _) = fixture(marked, nested);
        let calls = Arc::new(AtomicUsize::new(0)); let counter = calls.clone();
        let result = catch_unwind(AssertUnwindSafe(|| match path {
            0 => { let filter: SourceFilter = Arc::new(move |_,_| { counter.fetch_add(1, Ordering::SeqCst); allowed });
                source_multi_candidates(&game, 0, &[], &filter).len() },
            1 => { let filter: DpBudgetFilter = Arc::new(move |_,_| { counter.fetch_add(1, Ordering::SeqCst); allowed });
                dp_budget_candidates(&game, 0, 100000, &[], &filter).len() },
            _ => { let filter: PlayCostBudgetFilter = Arc::new(move |_,_| { counter.fetch_add(1, Ordering::SeqCst); allowed });
                play_cost_budget_candidates(&game, 0, 100, &[], &filter).len() }
        }));
        if marked { assert!(result.is_err()); assert_eq!(calls.load(Ordering::SeqCst), 0); }
        else { assert_eq!(result.unwrap(), usize::from(allowed)); assert_eq!(calls.load(Ordering::SeqCst), 1); }
    } } } }
}

#[test]
fn union_card_and_carrier_filters_check_before_body() {
    use crate::selection::UnionZoneSet;
    for zone in [UnionZoneSet::HAND, UnionZoneSet::TRASH, UnionZoneSet::MATERIAL] {
      for carrier_probe in [false, true] { if carrier_probe && zone != UnionZoneSet::MATERIAL { continue; }
       for nested in [false, true] { for marked in [false, true] { for allowed in [false, true] {
        let (game, _) = fixture(marked, nested);
        let calls = Arc::new(AtomicUsize::new(0)); let count = calls.clone(); let other = calls.clone();
        let filter = move |_: &Game, _: &CardSource, _: UnionZoneSet| {
            if !carrier_probe { count.fetch_add(1, Ordering::SeqCst); } allowed
        };
        let carrier = move |_: &Game, _: PermanentHandle| { other.fetch_add(1, Ordering::SeqCst); allowed };
        let result = catch_unwind(AssertUnwindSafe(|| union_zone_candidates_physical(
            &game, 0, zone, None, if carrier_probe { Some(&carrier) } else { None }, &filter)));
        if marked { assert!(result.is_err()); assert_eq!(calls.load(Ordering::SeqCst), 0); }
        else { assert_eq!(result.unwrap().len(), usize::from(allowed)); assert_eq!(calls.load(Ordering::SeqCst), 1); }
       } } }
      }
    }
}
