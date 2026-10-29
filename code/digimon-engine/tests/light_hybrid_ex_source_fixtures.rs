//! Canonical EX11/EX12 physical-choice fixture qualification.
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
#[path = "support/dsl_card_data.rs"]
mod dsl_card_data;
#[path = "cards_behavioral/ex11/ex11_022.rs"]
mod ex11_022;
#[path = "cards_behavioral/ex11/ex11_038.rs"]
mod ex11_038;
#[path = "cards_behavioral/ex11/ex11_053.rs"]
mod ex11_053;
#[path = "cards_behavioral/ex11/ex11_057.rs"]
mod ex11_057;
#[path = "cards_behavioral/ex11/ex11_065.rs"]
mod ex11_065;
#[path = "cards_behavioral/ex12/ex12_031.rs"]
mod ex12_031;
#[path = "cards_behavioral/ex12/support.rs"]
mod support;
