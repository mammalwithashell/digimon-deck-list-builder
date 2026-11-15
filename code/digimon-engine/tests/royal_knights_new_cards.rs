//! Focused qualification for seven newly authored Royal Knights pool cards.
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
#[path = "cards_behavioral/royal_knights_new_cards.rs"] mod royal_knights_new_cards;
