//! Canonical late card physical choice qualification.
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
#[path="support/dsl_card_data.rs"] mod dsl_card_data;
#[path="cards_behavioral/bt24/bt24_045.rs"] mod bt24_045;
#[path="cards_behavioral/bt24/bt24_089.rs"] mod bt24_089;
