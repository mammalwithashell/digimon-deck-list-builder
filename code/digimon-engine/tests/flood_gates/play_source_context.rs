//! Compile + sanity tests for `PlaySource` enum.
//!
//! Behavioral enforcement (checking that ByHand vs ByEffect is correctly
//! threaded) lands in Task 4 when `CannotPlayDigimonByEffect` is wired.

#[test]
fn play_source_enum_exists_and_has_expected_variants() {
    use digimon_engine::enums::PlaySource;
    let _ = PlaySource::ByHand;
    let _ = PlaySource::ByEffect;
    let _ = PlaySource::ByDigivolve;
}

#[test]
fn play_source_variants_are_distinct() {
    use digimon_engine::enums::PlaySource;
    assert_ne!(PlaySource::ByHand, PlaySource::ByEffect);
    assert_ne!(PlaySource::ByEffect, PlaySource::ByDigivolve);
    assert_ne!(PlaySource::ByHand, PlaySource::ByDigivolve);
}
