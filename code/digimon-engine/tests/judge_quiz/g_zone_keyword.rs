//! Cluster G — zone/keyword scoping.
//!
//! Questions (see `card-resolution.md`):
//!   Q3  Puppetmon (EX10-020) `[All Turns]` doesn't function in the breeding
//!       area, so it can digivolve into Quartzmon (BT12-057) — judge: YES.
//!   Q4  Aldamon (AD1-002) given `<Security A. +1>` by Atomic Inferno (BT4-098)
//!       and `<Security A. −1>` by Holy Flame (ST3-15): base 1 + 1 − 1 = 1
//!       check; one done ⇒ no more — judge: NO another check.  (Extends the
//!       `mid_attack_security_attack_recompute.rs` live-net-strike rule.)
//!
//! Scenarios authored under tasks §9.

#![allow(unused_imports)]

/// Q3 — Puppetmon (EX10-020) [All Turns] doesn't function in the breeding area, so
/// it can digivolve into Quartzmon (BT12-057). Judge: YES.
#[test]
#[ignore = "BLOCKED-CARD: needs EX10-020 (Puppetmon), BT12-057 (Quartzmon)."]
fn q3_breeding_area_effect_inactive_allows_digivolve() {}

/// Q4 — Aldamon (AD1-002) given `<Security A. +1>` by Atomic Inferno (BT4-098) and
/// `<Security A. −1>` by Holy Flame (ST3-15): base 1 + 1 − 1 = 1 check; one done ⇒
/// no more. Judge: NO another check. (Extends mid_attack_security_attack_recompute.rs.)
#[test]
#[ignore = "BLOCKED-CARD: needs AD1-002 (Aldamon), BT4-098 (Atomic Inferno), ST3-15 (Holy Flame)."]
fn q4_security_attack_net_modifiers_one_check() {}
