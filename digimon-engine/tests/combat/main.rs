//! Combat + security test binary — covers attack flow, interrupts,
//! security pipeline, Rush exemption, Force-Attack mask, and Overclock.

mod alliance_interrupt;
mod block_interrupt;
mod collision_mandatory;
mod counter_hand_play;
mod counter_interrupt;
mod force_attack_mask;
mod overclock;
mod raid_retarget;
mod redirect_and_cancel;
mod rush_exemption;
mod scenarios;
mod security_effects;
mod would_attack_replacements;
