//! Canonical EX7 late source and union fixture qualification.
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
#[path = "support/dsl_card_data.rs"]
mod dsl_card_data;
#[path = "cards_behavioral/ex7/ex7_021.rs"]
mod ex7_021;
#[path = "cards_behavioral/ex7/ex7_043.rs"]
mod ex7_043;
#[path = "cards_behavioral/ex7/ex7_051.rs"]
mod ex7_051;
#[path = "cards_behavioral/ex7/ex7_073.rs"]
mod ex7_073;
