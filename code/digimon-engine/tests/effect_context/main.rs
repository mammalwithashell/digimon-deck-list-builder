//! Effect-context primitive tests.
//!
//! This binary covers selection-zone variants and effect-context primitives
//! introduced incrementally. Each test module corresponds to a distinct
//! feature or primitive; add new modules here as Phase D primitives land.

mod armor_purge_top;
mod battle_opponent_of;
mod breeding_zone_movement;
mod effect_attack_window;
mod effect_digivolve_from_zones;
mod effect_digivolve_union_zones;
mod effect_dna_trash_partner;
mod effect_initiated_dna_digivolve;
mod effect_refiring;
mod material_zone_select;
mod no_source_targets;
mod opponent_stack_trashing;
mod override_persistence;
mod place_as_bottom_source_zombie;
mod place_deck_top_under_permanent;
mod place_on_security_zombie;
mod place_under_permanent;
mod place_under_permanent_face_down;
mod play_from_hand_free;
mod play_from_materials;
mod play_from_security;
mod play_from_trash;
mod provenance_tokens;
mod schedule_delayed;
mod security_stack_operations;
mod source_move_under_tamer;
mod source_snapshot_rescue;
mod source_stack_operations;
mod trash_bottom_face_down_source;
mod trash_card_source;
mod trash_source_ref_zombie;
mod trash_top_source;
mod under_tamer_hand_placement;
mod under_tamer_play;
mod under_tamer_selectors;
mod under_tamer_trash_placement;
mod under_tamer_union_placement;
