//! Focused regression binary for the exact local Light Hybrid training deck.
//! Reuses the canonical per-card modules declared in cards_behavioral, keeping
//! targeted TDD independent of unrelated card-test compilation. Admission still
//! discovers the ordinary cards_behavioral module tree.
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[path = "support/dsl_card_data.rs"]
mod dsl_card_data;

#[path = "cards_behavioral/ex7/ex7_002.rs"]
mod ex7_002;

#[path = "cards_behavioral/bt6/bt6_022.rs"]
mod bt6_022;

#[path = "cards_behavioral/bt17/bt17_020.rs"]
mod bt17_020;

#[path = "cards_behavioral/bt17/bt17_023.rs"]
mod bt17_023;

#[path = "cards_behavioral/bt18/bt18_037.rs"]
mod bt18_037;

#[path = "cards_behavioral/bt18/bt18_067.rs"]
mod bt18_067;

#[path = "cards_behavioral/bt18/bt18_063.rs"]
mod bt18_063;

#[path = "cards_behavioral/bt18/bt18_077.rs"]
mod bt18_077;

#[path = "cards_behavioral/bt18/bt18_076.rs"]
mod bt18_076;

#[path = "cards_behavioral/ad1/ad1_015.rs"]
mod ad1_015;

#[path = "cards_behavioral/bt7/bt7_038.rs"]
mod bt7_038;

#[path = "cards_behavioral/bt18/bt18_081.rs"]
mod bt18_081;

#[path = "cards_behavioral/bt18/bt18_042.rs"]
mod bt18_042;

#[path = "cards_behavioral/p/p_153.rs"]
mod p_153;

#[path = "cards_behavioral/bt18/bt18_088.rs"]
mod bt18_088;

#[path = "cards_behavioral/bt18/bt18_094.rs"]
mod bt18_094;

#[path = "cards_behavioral/ad1/ad1_023.rs"]
mod ad1_023;

#[path = "cards_behavioral/bt18/bt18_097.rs"]
mod bt18_097;

#[path = "cards_behavioral/p/p_249.rs"]
mod p_249;
