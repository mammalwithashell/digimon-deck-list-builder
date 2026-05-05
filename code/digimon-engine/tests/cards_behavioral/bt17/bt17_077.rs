//! BT17-077 Imperialdramon: Paladin Mode

use digimon_engine::debug_runner::DebugRunner;

#[test]
fn bt17_077_loads_source_trash_slice() {
    DebugRunner::builder()
        .dsl_card("BT17-077")
        .expect("BT17-077 must load from embedded DSL pack")
        .start();
}

#[ignore = "pending: G-BLAST-DIGIVOLVE and G-BULK-TRASH-TO-DECK — Blast Digivolve, return all trash to deck, returned-white-level-7 memory, and sourceless bottom-deck cost"]
#[test]
fn bt17_077_blast_trash_to_deck_and_sourceless_bottom_deck_cost() {}
