//! Canonical late source/security consumers of the owned choice contract.
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[path = "cards_behavioral/bt26/support.rs"]
mod support;
#[path = "cards_behavioral/bt23/bt23_032.rs"]
mod bt23_032;
#[path = "cards_behavioral/bt25/bt25_085.rs"]
mod bt25_085;
#[path = "cards_behavioral/bt26/bt26_009.rs"]
mod bt26_009;
#[path = "cards_behavioral/bt26/bt26_082.rs"]
mod bt26_082;
#[path = "cards_behavioral/bt4/bt4_072.rs"]
mod bt4_072;
