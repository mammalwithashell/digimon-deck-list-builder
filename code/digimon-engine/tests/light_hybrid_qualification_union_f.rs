//! Canonical late card physical choice qualification.
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
#[path="support/dsl_card_data.rs"] mod dsl_card_data;
#[path="cards_behavioral/bt23/bt23_072.rs"] mod bt23_072;
#[path="cards_behavioral/bt23/bt23_076.rs"] mod bt23_076;
#[path="cards_behavioral/bt24/bt24_007.rs"] mod bt24_007;
#[path="cards_behavioral/bt24/bt24_009.rs"] mod bt24_009;
