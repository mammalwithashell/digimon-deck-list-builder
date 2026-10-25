//! Canonical union-card fixture qualification.
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
#[path="support/dsl_card_data.rs"] mod dsl_card_data;
#[path="cards_behavioral/bt20/bt20_095.rs"] mod bt20_095;
#[path="cards_behavioral/bt21/bt21_062.rs"] mod bt21_062;
