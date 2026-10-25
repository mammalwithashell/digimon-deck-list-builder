//! Additional canonical union menu fixtures.
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
#[path="support/dsl_card_data.rs"] mod dsl_card_data;
#[path="cards_behavioral/bt21/bt21_091.rs"] mod bt21_091;
#[path="cards_behavioral/bt21/bt21_098.rs"] mod bt21_098;
