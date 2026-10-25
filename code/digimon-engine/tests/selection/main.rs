//! Selection-prompt test binary — covers the `PendingSelection` state
//! machine: opponent-field targeting, DNA material picks, and the
//! selection-kind routing dispatch.

#[path = "../support/owned_card_choice.rs"]
mod owned_choices;

mod behavioral_end_to_end;
mod breeding_permanent;
mod count_capped;
mod dp_budget;
mod kinds;
mod kinds_exist;
mod material;
mod opponent_permanent;
mod opponent_selector;
mod ordered_permutation;
mod place_remainder;
mod reveal_buckets;
mod source_multi;
mod selfplay_phase_resume;
mod union_zone;
mod departed_source_serialization;
