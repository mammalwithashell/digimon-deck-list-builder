//! Complete canonical late source-selection fixture modules.
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
#[path = "support/dsl_card_data.rs"]
mod dsl_card_data;
#[path = "cards_behavioral/bt26/support.rs"]
mod support;
#[path = "cards_behavioral/bt26/bt26_085.rs"]
mod bt26_085;
#[path = "cards_behavioral/bt26/bt26_096.rs"]
mod bt26_096;
#[path = "cards_behavioral/bt26/bt26_097.rs"]
mod bt26_097;
