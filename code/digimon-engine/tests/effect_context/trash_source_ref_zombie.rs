//! Regression test for `G-PERMANENT-EMPTY-DURING-MATERIAL-EXTRACTION` —
//! `Game::trash_source_ref` sibling. This is the agent-selected
//! "trash 1 of your digivolution sources" action surface (e.g. Rocks
//! archetype's self-trash effects).
//!
//! When the selected `SourceSelectionRef` resolves to the carrier's only
//! remaining card, the trash leaves the carrier with empty `card_sources`
//! and the slot intact in `battle_area` — a zombie. Downstream trigger
//! fan-out then panics with `Permanent must have at least one card`.
//!
//! Post-fix: the carrier slot is soft-removed in the same step that
//! pushes the source to trash.

use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::selection::SourceSelectionRef;

#[test]
fn trash_source_ref_emptying_carrier_removes_slot() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("T-ONLY", "OnlyTest"))
        .start();

    let perm = runner.place_on_field(0, "T-ONLY", None);
    let only_handle = runner.game.players[0].battle_area[perm.index as usize].card_sources[0]
        .handle();
    assert_eq!(
        runner.game.players[0].battle_area[perm.index as usize]
            .card_sources
            .len(),
        1,
        "precondition: carrier has exactly 1 source"
    );

    let ok = runner.game.trash_source_ref(SourceSelectionRef {
        permanent: perm,
        field_index: perm.index,
        source_index: 0,
        card: only_handle,
    });
    assert!(ok, "trash_source_ref should succeed");

    // No zombies.
    for (pid, player) in runner.game.players.iter().enumerate() {
        for (slot, perm) in player.battle_area.iter().enumerate() {
            assert!(
                !perm.card_sources.is_empty(),
                "zombie permanent at p{}.battle_area[{}]: card_sources is empty",
                pid,
                slot
            );
        }
    }

    // Carrier slot removed; source is in trash.
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        0,
        "carrier slot must be soft-removed"
    );
    assert_eq!(
        runner.game.players[0].trash.len(),
        1,
        "trashed source must be in trash"
    );
}
