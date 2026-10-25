//! Reuse canonical card cases while migrating physical selection contracts.
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[path = "cards_behavioral/bt1/bt1_087.rs"]
mod bt1_087;
#[path = "cards_behavioral/bt20/bt20_083.rs"]
mod bt20_083;
#[path = "cards_behavioral/bt21/bt21_074.rs"]
mod bt21_074;
