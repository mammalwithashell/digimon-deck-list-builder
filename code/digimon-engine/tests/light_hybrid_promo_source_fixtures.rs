//! Canonical promo physical-choice fixture qualification.
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
#[path = "support/dsl_card_data.rs"]
mod dsl_card_data;
#[path = "cards_behavioral/p/p_170.rs"]
mod p_170;
#[path = "cards_behavioral/p/p_189.rs"]
mod p_189;
#[path = "cards_behavioral/p/p_224.rs"]
mod p_224;
#[path = "cards_behavioral/p/p_244.rs"]
mod p_244;
