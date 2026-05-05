//! BT13-007 King Drasil_7D6
//!
//! Implemented slice:
//! - Breeding floodgate that prevents your Digimon from digivolving.
//! - Breeding Royal Knight play cost reduction, via existing raw Rust gap stub.
//! - Start of main phase places the top Digi-Egg and Royal Knights under self.
//! - Inherited breeding observer gains memory when Royal Knight Options are placed.

use digimon_engine::debug_runner::DebugRunner;

#[test]
fn bt13_007_loads_from_embedded_dsl_pack() {
    DebugRunner::builder()
        .dsl_card("BT13-007")
        .expect("BT13-007 must load from embedded DSL pack")
        .start();
}

#[ignore = "pending: G-BREEDING-ROYAL-KNIGHT-COST-REDUCTION — existing example uses raw_rust amount_fn until formula can count this source's digivolution cards"]
#[test]
fn bt13_007_cost_reduction_counts_sources_under_king_drasil() {}
