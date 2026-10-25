//! Canonical source-choice fixture qualification; all original module tests included.
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
#[path = "support/dsl_card_data.rs"]
mod dsl_card_data;
#[path = "cards_behavioral/bt19/bt19_047.rs"]
mod bt19_047;
#[path = "cards_behavioral/bt19/bt19_057.rs"]
mod bt19_057;
#[path = "cards_behavioral/bt19/bt19_090.rs"]
mod bt19_090;
#[path = "cards_behavioral/bt21/bt21_023.rs"]
mod bt21_023;
#[path = "cards_behavioral/bt16/bt16_077.rs"]
mod bt16_077;

#[path = "cards_behavioral/bt21/bt21_073.rs"]
mod bt21_073;
