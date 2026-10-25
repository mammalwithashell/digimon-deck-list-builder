//! Canonical EX4/EX7 owned-source and union fixture qualification.
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
#[path = "support/dsl_card_data.rs"]
mod dsl_card_data;
#[path = "cards_behavioral/ex4/ex4_073.rs"]
mod ex4_073;
#[path = "cards_behavioral/ex7/ex7_011.rs"]
mod ex7_011;
