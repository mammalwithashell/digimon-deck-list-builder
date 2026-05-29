//! Cluster E — `<Partition>` / DigiXros departure semantics / sequential
//! de-digivolve with mid-sequence immunity.
//!
//! Questions (see `card-resolution.md`):
//!   Q15 LordKnightmon (X Antibody) (BT19-073) does `<De-Digivolve 1>` repeatedly
//!       on a stack (Omnimon X BT20-102 / Gallantmon X EX8-073 / Gallantmon
//!       BT17-016 / WarGrowlmon BT12-016 / Growlmon EX3-057 / Guilmon EX4-006);
//!       after the first, Gallantmon (X Antibody)'s immunity halts the rest —
//!       judge: Gallantmon (X Antibody) topmost.
//!   Q16 Lilithmon (EX6-057)-granted "[EoT] Delete this" on Paildramon
//!       (BT16-025) counts as leaving by its OWN effect — judge: `<Partition>`
//!       does NOT trigger.
//!   Q25 Miraculous Mega Knight (BT17-095) `[All Turns]` fires on DigiXros
//!       departure of WarGreymon (AD1-004) — judge: YES (DigiXros ≠ battle).
//!   Q29 Yuu Amano (BT10-093) + DigiXros (DarknessBagramon EX10-059 etc.):
//!       3 legal stack orderings — judge: placement order rules.
//!   Q30 (shared with cluster C) interruptive `<Partition>`.
//!
//! Scenarios authored under tasks §7.

#![allow(unused_imports)]

/// Q15 — LordKnightmon (X Antibody) (BT19-073) does `<De-Digivolve 1>` repeatedly;
/// after the first, Gallantmon (X Antibody) (EX8-073)'s [All Turns] immunity halts
/// the rest. Judge: Gallantmon (X Antibody) is the topmost card.
#[test]
#[ignore = "BLOCKED-CARD: needs BT19-073 (LordKnightmon X), BT17-016 (Gallantmon), BT12-016 (WarGrowlmon), EX3-057 (Growlmon). BT19-072, BT20-102, EX8-073, EX4-006 implemented."]
fn q15_sequential_de_digivolve_halted_by_x_antibody_immunity() {}

/// Q16 — Lilithmon (EX6-057)-granted "[EoT] Delete this" on Paildramon (BT16-025)
/// counts as leaving by its OWN effect. Judge: `<Partition>` does NOT trigger.
/// (One card away — Paildramon/ExVeemon/Stingmon all implemented.)
#[test]
#[ignore = "BLOCKED-CARD: needs EX6-057 (Lilithmon). BT16-025, BT12-022, BT12-050 implemented."]
fn q16_partition_not_triggered_when_leaving_by_own_granted_effect() {}

/// Q25 — Miraculous Mega Knight (BT17-095) [All Turns] fires on DigiXros departure
/// of WarGreymon (AD1-004) (departure ≠ battle). Judge: YES, triggers.
/// (One card away — WarGreymon/MetalGarurumon/Omnimon/MMK implemented.)
#[test]
#[ignore = "BLOCKED-CARD: needs EX3-014 (Dorbickmon, the DigiXros host). AD1-004, AD1-014, AD1-025, BT17-095 implemented."]
fn q25_all_turns_fires_on_digixros_departure_not_battle() {}

/// Q29 — Yuu Amano (BT10-093) top-placement (either order) + DigiXros bottom
/// placement (spec order): 3 legal DarknessBagramon (EX10-059) stacks. Judge:
/// the 3 specific orderings.
#[test]
#[ignore = "BLOCKED-CARD: needs BT10-093 (Yuu Amano), EX10-039 (ChuuChuumon), EX10-044 (Damemon), EX10-059 (DarknessBagramon), EX10-056 (Bagramon), EX10-031 (DarkKnightmon)."]
fn q29_legal_digixros_stack_orderings_with_yuu_amano() {}

// Q30 spans clusters C and E — its test lives in `c_declare_then_pay.rs`
// (`q30_partition_interruptive_suspends_both_with_cost_reduction`).
