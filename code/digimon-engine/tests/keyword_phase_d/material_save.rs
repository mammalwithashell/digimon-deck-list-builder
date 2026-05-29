//! Regression coverage for `<Material Save>` after the Xros Heart DigiXros
//! substrate moved it from a main-phase active skill to deletion/removal timing.
//!
//! Keep the historical Phase D target wired, but share the canonical tests with
//! the standalone Material Save keyword suite so stale `[Main]` expectations do
//! not drift back in.

include!("../material_save_keyword.rs");
