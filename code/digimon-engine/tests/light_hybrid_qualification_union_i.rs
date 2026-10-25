//! Canonical EX13 physical choice qualification.
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
#[path="support/dsl_card_data.rs"] mod dsl_card_data;
#[path="cards_behavioral/ex13/orphan_support.rs"] mod orphan_support;
#[path="cards_behavioral/ex13/ex13_013.rs"] mod ex13_013;
#[path="cards_behavioral/ex13/ex13_014.rs"] mod ex13_014;
#[path="cards_behavioral/ex13/ex13_031.rs"] mod ex13_031;
#[path="cards_behavioral/ex13/ex13_035.rs"] mod ex13_035;
