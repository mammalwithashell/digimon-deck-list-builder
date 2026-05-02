//! Effect-context primitive tests.
//!
//! This binary covers selection-zone variants and effect-context primitives
//! introduced incrementally. Each test module corresponds to a distinct
//! feature or primitive; add new modules here as Phase D primitives land.

mod armor_purge_top;
mod battle_opponent_of;
mod breeding_zone_movement;
mod effect_digivolve_from_zones;
mod effect_initiated_dna_digivolve;
mod material_zone_select;
mod override_persistence;
mod place_under_permanent;
mod play_from_hand_free;
mod play_from_materials;
mod play_from_security;
mod play_from_trash;
mod schedule_delayed;
mod security_stack_operations;
mod source_stack_operations;
mod trash_card_source;
mod trash_top_source;
