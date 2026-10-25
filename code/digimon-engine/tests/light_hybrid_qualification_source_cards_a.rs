//! Canonical source-choice fixture qualification; all original module tests included.
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
#[path = "support/dsl_card_data.rs"]
mod dsl_card_data;
#[path = "cards_behavioral/bt18/bt18_092.rs"]
mod bt18_092;
#[path = "cards_behavioral/bt19/bt19_008.rs"]
mod bt19_008;
#[path = "cards_behavioral/bt19/bt19_014.rs"]
mod bt19_014;
#[path = "cards_behavioral/bt19/bt19_033.rs"]
mod bt19_033;
