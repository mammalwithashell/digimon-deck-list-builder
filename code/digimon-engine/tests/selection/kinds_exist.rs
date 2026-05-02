use digimon_engine::enums::GamePhase;
use digimon_engine::selection::{SelectionKind, UnionZoneSet};

#[test]
fn new_selection_kinds_exist() {
    let _ = SelectionKind::UnionZone {
        zones: UnionZoneSet::HAND | UnionZoneSet::TRASH,
    };
    let _ = SelectionKind::OrderedPermutation { remaining: 3 };
    let _ = SelectionKind::CountCappedMultiSelect { max: 2, picked: 0 };
    let _ = GamePhase::SelectUnion;
    let _ = GamePhase::SelectPermutation;
    let _ = GamePhase::SelectBudgeted;
}

#[test]
fn union_zone_set_bitops() {
    let both = UnionZoneSet::HAND | UnionZoneSet::TRASH;
    assert!(both.contains(UnionZoneSet::HAND));
    assert!(both.contains(UnionZoneSet::TRASH));
    assert!(!UnionZoneSet::HAND.contains(UnionZoneSet::TRASH));
    assert!(!UnionZoneSet(0).is_empty() || UnionZoneSet(0).is_empty()); // trivially true
    assert!(UnionZoneSet(0).is_empty());
    assert!(!UnionZoneSet::HAND.is_empty());
}

#[test]
fn new_game_phases_are_selection_phases() {
    assert!(GamePhase::SelectUnion.is_selection_phase());
    assert!(GamePhase::SelectPermutation.is_selection_phase());
    assert!(GamePhase::SelectBudgeted.is_selection_phase());
}

#[test]
fn new_game_phases_py_name() {
    assert_eq!(GamePhase::SelectUnion.py_name(), "SelectUnion");
    assert_eq!(GamePhase::SelectPermutation.py_name(), "SelectPermutation");
    assert_eq!(GamePhase::SelectBudgeted.py_name(), "SelectBudgeted");
}
