//! Focused exact-deck qualification without compiling the monolithic card suite.
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
#[path = "cards_behavioral/bt11/bt11_023.rs"]
mod bt11_023;
#[path = "cards_behavioral/bt11/bt11_112.rs"]
mod bt11_112;
#[path = "cards_behavioral/bt12/bt12_029.rs"]
mod bt12_029;
#[path = "cards_behavioral/bt22/bt22_019.rs"]
mod bt22_019;
#[path = "cards_behavioral/bt22/bt22_022.rs"]
mod bt22_022;
#[path = "cards_behavioral/bt22/bt22_023.rs"]
mod bt22_023;
#[path = "cards_behavioral/bt22/bt22_085.rs"]
mod bt22_085;
#[path = "dsl/can_unsuspend.rs"]
mod can_unsuspend;
#[path = "cards_behavioral/ex13/ex13_002.rs"]
mod ex13_002;
#[path = "cards_behavioral/ex13/ex13_017.rs"]
mod ex13_017;
#[path = "cards_behavioral/ex13/ex13_019.rs"]
mod ex13_019;
#[path = "cards_behavioral/ex13/ex13_022.rs"]
mod ex13_022;
#[path = "cards_behavioral/ex13/ex13_023.rs"]
mod ex13_023;
#[path = "cards_behavioral/ex13/ex13_069.rs"]
mod ex13_069;
#[path = "cards_behavioral/ulforce_exact_export.rs"]
mod exact_export;
#[path = "cards_behavioral/lm/lm_028.rs"]
mod lm_028;
#[path = "cards_behavioral/lm/lm_034.rs"]
mod lm_034;
#[path = "cards_behavioral/lm/lm_057.rs"]
mod lm_057;
#[path = "cards_behavioral/lm/lm_058.rs"]
mod lm_058;
#[path = "cards_behavioral/ex13/orphan_support.rs"]
mod orphan_support;
#[path = "cards_behavioral/p/p_104.rs"]
mod p_104;
#[path = "cards_behavioral/p/p_138.rs"]
mod p_138;
