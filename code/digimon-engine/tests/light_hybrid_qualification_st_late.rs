//! Canonical starter-card physical choice qualification.
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
#[path="support/dsl_card_data.rs"] mod dsl_card_data;
#[path="cards_behavioral/st19/st19_08.rs"] mod st19_08;
#[path="cards_behavioral/st2/st2_cards.rs"] mod st2_cards;
#[path="cards_behavioral/st5/st5_effects.rs"] mod st5_effects;
#[path="cards_behavioral/st9/st9_06.rs"] mod st9_06;
