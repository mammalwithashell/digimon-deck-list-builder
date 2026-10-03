//! **Archetype interaction tests** — multi-card / combo tests that exercise
//! several real implemented cards working together as the deck actually plays.
//!
//! This is the home the per-card `cards_behavioral/<set>/` tree lacks: combos
//! span sets and cards and have no single per-card owner. Each suite here is
//! one archetype (one file per archetype, mirroring its model doc at
//! `qa/archetype-qa/<archetype>-model.md`), and each `#[test]` maps 1:1 to a
//! named combo in that model, asserting the combo's claimed mechanical outcome.
//!
//! Authored by the `/archetype-interaction-test-author` skill (the capstone
//! that runs after the archetype's cards are implemented and per-card tests are
//! green). Run this binary alone with:
//!
//! ```bash
//! cargo test --manifest-path code/digimon-engine/Cargo.toml --test archetypes
//! ```
//!
//! Shared multi-card fixtures live in `support.rs`; richer board setup
//! (digivolution stacks, breeding, security/trash seeding) is already on
//! `DebugRunner` / `DebugRunnerBuilder`.

mod support;

// ── Fixture self-tests (prove the shared helpers) ────────────────────────────

#[cfg(test)]
mod fixture_smoke {
    use super::support::{dsl_builder, snapshot};

    /// `dsl_builder` loads multiple DSL cards and `snapshot` reads both
    /// players' zones — the minimum the interaction suites rely on.
    #[test]
    fn fixtures_build_a_multi_card_runner_and_snapshot_it() {
        // BT17-102 (Greymon) is a known implemented Rocks DSL card.
        let runner = dsl_builder(&["BT17-102"]).memory(3).start();
        let snap = snapshot(&runner);
        assert_eq!(snap.memory, 3, "snapshot reads the seeded memory");
        // A fresh debug game has empty battle areas until cards are placed.
        assert_eq!(snap.field, [0, 0]);
    }
}

// ── Per-archetype interaction suites ─────────────────────────────────────────

mod beatbreak_bt25;
mod bg_imperial;
mod callismon_dark_animal_bt25;
mod dna_omnimon;
mod dracomon_ex13;
mod veedramon_cs_ex13;
mod flaremon_beastkin;
mod gaogamon_beast_bt25;
mod ice_snow;
mod machine_bt25;
mod magneticdra;
mod mammal_bt25;
mod medusamon;
mod mutant_ex13;
mod nokia_alters;
mod omni_nokia;
mod omnimon_ace;
mod puppet_sister;
mod puppets;
mod richard_sampson_data_squad_ex13;
mod royal_knights_ex13;
mod rocks;
mod thomas_data_squad_bt25;
mod titan_bt25;

// Starter-deck interaction suites (ST-1 … ST-6), authored by the
// `/archetype-interaction-test-author` capstone run. Real-card / synthetic-free.
mod st1;
mod st2;
mod st3;
mod st4;
mod st5;
mod st6;

// EX13 "Sukamon / beast" slice (author-set workflow).
mod sukamon_beast_ex13;

// EX13 "Guilmon / reptile" slice (author-set workflow).
mod guilmon_reptile_ex13;

// EX13 "Chronicle" slice (author-set workflow).
mod chronicle_ex13;

// EX13 "Puppet (Sistermon Awakened DUALs)" slice (author-set workflow).
mod puppet_sistermon_ex13;

// EX13 "Witchelny" slice (author-set workflow).
mod witchelny_ex13;
