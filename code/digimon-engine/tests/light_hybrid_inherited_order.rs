#[path = "cards_behavioral/bt18/light_hybrid_inherited_order.rs"]
mod inherited_order;
#[test]
fn bt18_063_q2994_inherited_tamer_on_play_and_original_on_deletion_are_ordered_together() {
    inherited_order::prove_simultaneous_order("BT18-063");
}
#[test]
fn bt18_076_q3021_inherited_tamer_on_play_and_original_on_deletion_are_ordered_together() {
    inherited_order::prove_simultaneous_order("BT18-076");
}
