//! Canonical late-set physical-choice fixture qualification; no copied card tests.
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
#[path = "support/dsl_card_data.rs"]
mod dsl_card_data;
#[path = "cards_behavioral/bt25/bt25_097.rs"]
mod bt25_097;
#[path = "cards_behavioral/ex10/ex10_003.rs"]
mod ex10_003;
#[path = "cards_behavioral/ex10/ex10_028.rs"]
mod ex10_028;
