//! Remaining EX13 Royal Knights production-card qualification.
#[path = "cards_behavioral/bt13/bt13_102.rs"] mod bt13_102;
#[path = "cards_behavioral/bt8/bt8_094.rs"] mod bt8_094;
#[path = "cards_behavioral/st15/st15_16.rs"] mod st15_16;
#[path = "cards_behavioral/bt13/bt13_040.rs"] mod bt13_040;
#[path = "cards_behavioral/bt23/bt23_047.rs"] mod bt23_047;
#[path = "cards_behavioral/ex13/ex13_060.rs"] mod ex13_060;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
