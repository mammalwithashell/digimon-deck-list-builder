//! Combat + security test binary — covers attack flow, interrupts,
//! security pipeline, Rush exemption, Force-Attack mask, and Overclock.

mod alliance_interrupt;
mod block_interrupt;
mod collision_mandatory;
mod counter_hand_play;
mod counter_interrupt;
mod deletion_cause_observer;
mod effect_battle;
mod effect_granted_attack;
mod force_attack_mask;
mod group6_keywords;
mod group6_overclock;
mod on_ally_opponent_attack;
mod on_block_observer;
mod overclock;
mod phase9_end_to_end;
mod piercing_security;
mod progress_mutation_gates;
mod progress_partial;
mod raid_retarget;
mod reboot_unsuspend;
mod redirect_and_cancel;
mod rush_exemption;
mod scenarios;
mod security_attack_keyword;
mod security_effects;
mod track_c_deferred_modifiers;
mod track_c_modifiers;
mod until_condition_controller;
mod would_attack_replacements;
