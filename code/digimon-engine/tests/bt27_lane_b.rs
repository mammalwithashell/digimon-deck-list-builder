//! Focused BT27 lane B behavior runner; no admission claim.
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
#[path = "cards_behavioral/bt27/mod.rs"]
mod bt27;
#[path = "cards_behavioral/ex4/ex4_019.rs"]
mod ex4_019;
