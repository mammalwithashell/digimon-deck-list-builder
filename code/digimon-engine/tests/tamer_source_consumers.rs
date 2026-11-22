//! Existing consumers of the captured source-host predicate.
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
#[path = "support/dsl_card_data.rs"]
mod dsl_card_data;
#[path = "cards_behavioral/bt26/support.rs"]
mod support;
#[path = "cards_behavioral/bt25/bt25_029.rs"]
mod bt25_029;
#[path = "cards_behavioral/st24/st24_11.rs"]
mod st24_11;
#[path = "cards_behavioral/bt26/bt26_002.rs"]
mod bt26_002;
#[path = "cards_behavioral/bt26/bt26_044.rs"]
mod bt26_044;
#[path = "cards_behavioral/bt26/bt26_049.rs"]
mod bt26_049;
#[path = "cards_behavioral/bt26/bt26_057.rs"]
mod bt26_057;
#[path = "cards_behavioral/bt26/bt26_076.rs"]
mod bt26_076;
