//! Focused rerun for the four cards completed by the remaining meta gap work.
//! The same modules are registered in the full cards_behavioral suite.
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[path = "cards_behavioral/bt20/bt20_018.rs"]
mod bt20_018;
#[path = "cards_behavioral/bt20/bt20_056.rs"]
mod bt20_056;
#[path = "cards_behavioral/bt20/bt20_087.rs"]
mod bt20_087;
#[path = "cards_behavioral/ex6/ex6_072.rs"]
mod ex6_072;
