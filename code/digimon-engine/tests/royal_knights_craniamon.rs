//! Craniamon and preserved Slayerdramon authority distinction.
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
#[path = "cards_behavioral/bt13/bt13_077.rs"]
mod bt13_077;
#[path = "cards_behavioral/ex3/ex3_024.rs"]
mod ex3_024;
// The final admitted exact list adds this already-implemented card to the union.
#[path = "cards_behavioral/bt22/bt22_041.rs"]
mod bt22_041;
