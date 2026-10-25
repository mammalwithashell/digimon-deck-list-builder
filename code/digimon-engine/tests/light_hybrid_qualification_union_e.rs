//! Canonical BT22-098 owned union fixture qualification.
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
#[path="support/dsl_card_data.rs"] mod dsl_card_data;
#[path="cards_behavioral/bt22/bt22_098.rs"] mod bt22_098;
