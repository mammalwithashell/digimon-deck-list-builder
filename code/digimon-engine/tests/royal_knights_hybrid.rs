//! Canonical tests for the remaining established cards in the newly admitted hybrid list.
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
#[path = "cards_behavioral/ex13/orphan_support.rs"] mod orphan_support;
#[path = "cards_behavioral/bt16/bt16_102.rs"] mod bt16_102;
#[path = "cards_behavioral/bt20/bt20_021.rs"] mod bt20_021;
#[path = "cards_behavioral/bt4/bt4_104.rs"] mod bt4_104;
#[path = "cards_behavioral/bt8/bt8_095.rs"] mod bt8_095;
#[path = "cards_behavioral/ex13/ex13_070.rs"] mod ex13_070;
#[path = "cards_behavioral/lm/lm_034.rs"] mod lm_034;
#[path = "cards_behavioral/lm/lm_056.rs"] mod lm_056;
#[path = "cards_behavioral/lm/lm_057.rs"] mod lm_057;
#[path = "cards_behavioral/lm/lm_058.rs"] mod lm_058;
#[path = "cards_behavioral/p/p_104.rs"] mod p_104;
#[path = "cards_behavioral/p/p_117.rs"] mod p_117;
#[path = "cards_behavioral/st21/st21_15.rs"] mod st21_15;
#[path = "cards_behavioral/st22/st22_08.rs"] mod st22_08;
