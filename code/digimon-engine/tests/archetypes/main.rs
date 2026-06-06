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

mod callismon_dark_animal_bt25;
mod flaremon_beastkin;
mod gaogamon_beast_bt25;
mod machine_bt25;
mod mammal_bt25;
mod rocks;
mod thomas_data_squad_bt25;
mod titan_bt25;
