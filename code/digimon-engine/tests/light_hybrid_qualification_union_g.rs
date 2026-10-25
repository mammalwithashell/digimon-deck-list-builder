//! Canonical late card physical choice qualification.
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
#[path="support/dsl_card_data.rs"] mod dsl_card_data;
#[path="cards_behavioral/bt24/bt24_013.rs"] mod bt24_013;
#[path="cards_behavioral/bt24/bt24_021.rs"] mod bt24_021;
#[path="cards_behavioral/bt24/bt24_026.rs"] mod bt24_026;
#[path="cards_behavioral/bt24/bt24_042.rs"] mod bt24_042;
