//! Complete canonical late source-selection fixture modules.
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
#[path = "support/dsl_card_data.rs"]
mod dsl_card_data;
#[path = "cards_behavioral/bt26/support.rs"]
mod support;
#[path = "cards_behavioral/bt24/bt24_098.rs"]
mod bt24_098;
#[path = "cards_behavioral/bt25/bt25_092.rs"]
mod bt25_092;
#[path = "cards_behavioral/bt26/bt26_066.rs"]
mod bt26_066;
#[path = "cards_behavioral/bt26/bt26_069.rs"]
mod bt26_069;
