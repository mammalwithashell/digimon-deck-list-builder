//! Synthetic test cards (TEST-001..022) used to exercise specific engine
//! code paths. These are not real Digimon cards — they're minimal fixtures
//! for the behavioral-test harness.
//!
//! Layout convention (locked in before production sets arrive): one file per
//! card, named after the card ID in lowercase with an underscore
//! (`test_001.rs` for `TEST-001`). The file exports one public struct whose
//! name is the card ID in PascalCase (`Test001`). `register()` below fans
//! out to all cards in this set.

use std::sync::Arc;

use crate::cards::CardEffectRegistry;
use crate::enums::ModifierType;

mod test_001;
mod test_002;
mod test_003;
mod test_004;
mod test_005;
mod test_006;
mod test_007;
mod test_008;
mod test_010;
mod test_011;
mod test_012;
mod test_013;
mod test_014;
mod test_020;
mod test_021;
mod test_022;
mod test_023;
mod test_024;
mod test_025;
mod test_phase7_cancel;
mod test_phase7_cancel2;
pub mod test_phase7_guard_sentinel;
mod test_phase7_handled;
mod test_phase7_optional;
mod test_phase7_recurse;
mod test_phase7_redirect;
mod test_phase7_substitute;
// Test cards distilled from ZXavier's Digi Rulings quiz — one card per
// isolated rule concept, named `test_q<NN>_<tag>.rs` where `NN` matches
// the quiz question number the card is drawn from.
mod test_q04_secatk;
mod test_q10_capmem;
mod test_q13_negdp;
mod test_q16_ondel;
mod test_q17_eotdel;

pub use test_phase7_guard_sentinel::{GUARD_SENTINEL_WWBD, GUARD_SENTINEL_WWLBA};

/// Register all test cards into the registry.
pub fn register(registry: &mut CardEffectRegistry) {
    registry.insert("TEST-001", Arc::new(test_001::Test001));
    registry.insert("TEST-002", Arc::new(test_002::Test002));
    registry.insert("TEST-003", Arc::new(test_003::Test003));
    registry.insert("TEST-004", Arc::new(test_004::Test004));
    registry.insert("TEST-005", Arc::new(test_005::Test005));
    registry.insert("TEST-006", Arc::new(test_006::Test006));
    registry.insert("TEST-007", Arc::new(test_007::Test007));
    registry.insert("TEST-008", Arc::new(test_008::Test008));
    registry.insert("TEST-010", Arc::new(test_010::Test010));
    registry.insert("TEST-011", Arc::new(test_011::Test011));
    registry.insert("TEST-012", Arc::new(test_012::Test012));
    registry.insert("TEST-013", Arc::new(test_013::Test013));
    registry.insert("TEST-014", Arc::new(test_014::Test014));
    registry.insert("TEST-020", Arc::new(test_020::Test020));
    registry.insert("TEST-021", Arc::new(test_021::Test021));
    registry.insert("TEST-022", Arc::new(test_022::Test022));
    registry.insert("TEST-023", Arc::new(test_023::Test023));
    registry.insert("TEST-024", Arc::new(test_024::Test024));
    registry.insert("TEST-025", Arc::new(test_025::Test025));
    registry.insert("TEST-P7-CANCEL", Arc::new(test_phase7_cancel::TestP7Cancel));
    registry.insert("TEST-P7-CANCEL2", Arc::new(test_phase7_cancel2::TestP7Cancel2));
    registry.insert(
        "TEST-P7-GUARD-SENTINEL",
        Arc::new(test_phase7_guard_sentinel::TestP7GuardSentinel),
    );
    registry.insert("TEST-P7-HANDLED", Arc::new(test_phase7_handled::TestP7Handled));
    registry.insert("TEST-P7-OPTIONAL", Arc::new(test_phase7_optional::TestP7Optional));
    registry.insert("TEST-P7-RECURSE", Arc::new(test_phase7_recurse::TestP7Recurse));
    registry.insert("TEST-P7-REDIRECT", Arc::new(test_phase7_redirect::TestP7Redirect));
    registry.insert("TEST-P7-SUBSTITUTE", Arc::new(test_phase7_substitute::TestP7Substitute));

    // Quiz-derived rule-concept cards.
    registry.insert("TEST-Q04-SECATK", Arc::new(test_q04_secatk::TestQ04Secatk));
    registry.insert("TEST-Q10-CAPMEM", Arc::new(test_q10_capmem::TestQ10Capmem));
    registry.insert("TEST-Q13-NEGDP", Arc::new(test_q13_negdp::TestQ13Negdp));
    registry.insert("TEST-Q16-ONDEL", Arc::new(test_q16_ondel::TestQ16Ondel));
    registry.insert("TEST-Q17-EOTDEL", Arc::new(test_q17_eotdel::TestQ17Eotdel));

    // Suppress unused warning on ModifierType (referenced via add_dp_modifier helper).
    let _ = ModifierType::ChangeDp;
}
