//! Effect-queue drainer test binary. New effect-subsystem tests
//! (timing dispatch, observer fan-out, replacement framework) will
//! land here as the gap-closing roadmap lands.

mod dsl_archetype_slice;
#[path = "../support/dsl_card_data.rs"]
mod dsl_card_data;
mod queue_drainer;
