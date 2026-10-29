//! Canonical union-card fixture qualification.
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
#[path="support/dsl_card_data.rs"] mod dsl_card_data;
#[path="cards_behavioral/bt19/bt19_099.rs"] mod bt19_099;
#[path="cards_behavioral/bt20/bt20_018.rs"] mod bt20_018;
#[path="cards_behavioral/bt20/bt20_056.rs"] mod bt20_056;
#[path="cards_behavioral/bt20/bt20_093.rs"] mod bt20_093;
