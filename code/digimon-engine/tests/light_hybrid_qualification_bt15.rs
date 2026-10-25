//! Focused qualification of the unchanged canonical BT15-092 / BT15-102 card modules.
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
#[path = "cards_behavioral/bt15/bt15_092.rs"]
mod bt15_092;
#[path = "cards_behavioral/bt15/bt15_102.rs"]
mod bt15_102;
