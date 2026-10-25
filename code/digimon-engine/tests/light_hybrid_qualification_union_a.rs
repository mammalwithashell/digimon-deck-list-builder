//! Canonical union-card fixture qualification.
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
#[path="support/dsl_card_data.rs"] mod dsl_card_data;
#[path="cards_behavioral/bt17/bt17_095.rs"] mod bt17_095;
#[path="cards_behavioral/bt17/bt17_097.rs"] mod bt17_097;
#[path="cards_behavioral/bt19/bt19_012.rs"] mod bt19_012;
#[path="cards_behavioral/bt19/bt19_061.rs"] mod bt19_061;
