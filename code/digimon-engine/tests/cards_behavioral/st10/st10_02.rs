//! ST10-02 Salamon
//!
//! Inherited Effect [End of Your Turn] This Digimon and any of your other
//! Digimon may DNA digivolve into a Digimon card in the hand.

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::debug_runner::DebugRunner;

#[test]
fn st10_02_compiles_inherited_end_turn_dna_alt_path_registration() {
    let runner = DebugRunner::builder()
        .dsl_card("ST10-02")
        .expect("ST10-02 YAML loads")
        .start();
    let compiled = runner
        .compiled_card("ST10-02")
        .expect("ST10-02 compiled card present");

    let registrations: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|clause| match clause {
            CompiledClause::Declarative(CompiledDeclarativeClause::AltPathRegistration {
                scope,
                trigger,
                registers,
                ..
            }) => Some((*scope, *trigger, registers.kind)),
            _ => None,
        })
        .collect();

    assert_eq!(registrations.len(), 1);
    assert_eq!(
        registrations[0],
        (
            CompiledScope::Inherited,
            CompiledTiming::EndOfYourTurn,
            CompiledAltPathKind::DnaDigivolve,
        )
    );
}
