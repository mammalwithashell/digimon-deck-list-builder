//! Verification of retained verdict blockers in the remaining EX13 hybrid list.
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
#[path = "cards_behavioral/bt9/bt9_109.rs"] mod bt9_109;
#[path = "cards_behavioral/lm/lm_028.rs"] mod lm_028;
