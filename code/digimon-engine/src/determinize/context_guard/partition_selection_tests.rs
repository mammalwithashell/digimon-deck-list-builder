use super::*;
use crate::{debug_runner::{make_test_card, DebugRunner}, trigger_context::TriggerContext};
use std::{panic::{catch_unwind, AssertUnwindSafe}, sync::{Arc, atomic::{AtomicUsize, Ordering}}};

fn fixture(marked: bool, nested: bool) -> (Game, PermanentHandle, SourceSelectionRef) {
    let mut r = DebugRunner::builder().add_card(make_test_card("GUARD-SOURCE", "Source"))
        .add_card(make_test_card("GUARD-TOP", "Top")).memory(5).start();
    let host = r.place_stack(0, &["GUARD-SOURCE", "GUARD-TOP"]);
    let source = SourceSelectionRef { permanent: host, field_index: host.index, source_index: 0,
        card: r.game.players[0].battle_area[0].card_sources[0].handle() };
    let mut context = TriggerContext::default(); context.projected_reference_reads_guarded = marked;
    if nested { let mut parent = TriggerContext::default(); parent.simultaneous_play_members.push(context); context = parent; }
    r.game.current_trigger_context = Some(context);
    (r.game, host, source)
}
fn requirement(expected: SourceSelectionRef, calls: Arc<AtomicUsize>, allowed: bool) -> PartitionRequirement {
    // Direct struct construction must not bypass engine invocation protection.
    PartitionRequirement { label: "guard probe", matches: Box::new(move |_, actual| {
        calls.fetch_add(1, Ordering::SeqCst); assert_eq!(actual, expected); allowed
    }) }
}

#[test]
fn partition_candidate_scan_checks_before_requirement_body() {
    for nested in [false, true] { for marked in [false, true] { for allowed in [false, true] {
        let (game, host, source) = fixture(marked, nested);
        let calls = Arc::new(AtomicUsize::new(0));
        let requirements = [requirement(source, calls.clone(), allowed)];
        let result = catch_unwind(AssertUnwindSafe(|| partition_source_refs(&game, host, &requirements)));
        if marked { assert!(result.is_err()); assert_eq!(calls.load(Ordering::SeqCst), 0); }
        else { assert_eq!(result.unwrap(), if allowed { vec![source] } else { vec![] }); assert_eq!(calls.load(Ordering::SeqCst), 1); }
    } } }
}

#[test]
fn partition_assignment_checks_before_requirement_body() {
    for nested in [false, true] { for marked in [false, true] { for allowed in [false, true] {
        let (game, _, source) = fixture(marked, nested);
        let calls = Arc::new(AtomicUsize::new(0));
        let requirements = [requirement(source, calls.clone(), allowed)];
        let mut used = [false];
        let result = catch_unwind(AssertUnwindSafe(|| partition_match_from(&game, &requirements, &[source], &mut used, 0)));
        if marked { assert!(result.is_err()); assert_eq!(calls.load(Ordering::SeqCst), 0); assert_eq!(used, [false]); }
        else { assert_eq!(result.unwrap(), allowed); assert_eq!(calls.load(Ordering::SeqCst), 1); }
    } } }
}
