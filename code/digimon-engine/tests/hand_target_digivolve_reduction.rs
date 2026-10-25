#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[path = "dsl/constructs/hand_target_digivolve_reduction.rs"]
mod constructs;
