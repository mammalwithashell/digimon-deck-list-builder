//! Full canonical EX13 late source/union fixture modules.
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
#[path = "support/dsl_card_data.rs"]
mod dsl_card_data;
#[path = "cards_behavioral/ex13/chronicle_support.rs"]
mod chronicle_support;
#[path = "cards_behavioral/ex13/ex13_067.rs"]
mod ex13_067;
