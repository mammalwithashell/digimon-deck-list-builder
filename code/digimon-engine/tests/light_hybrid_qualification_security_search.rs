//! Canonical printed-search card qualification, plus Apocalymon material controls.
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
#[path = "support/dsl_card_data.rs"]
mod dsl_card_data;
#[path = "cards_behavioral/bt13/bt13_012.rs"]
mod bt13_012;
#[path = "cards_behavioral/bt14/bt14_033.rs"]
mod bt14_033;
#[path = "cards_behavioral/bt15/bt15_092.rs"]
mod bt15_092;
#[path = "cards_behavioral/bt15/bt15_102.rs"]
mod bt15_102;
