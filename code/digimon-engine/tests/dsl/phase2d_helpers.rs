//! Shared helpers for Phase 2d DSL tests.
//!
//! Mirrors the `auto_commits_at_max` pattern from
//! `digimon-engine/tests/selection/count_capped.rs`: each pick is a
//! separate `resolve_selection` call. After each pick, the engine re-arms
//! `pending_selection` with the remaining candidates (the picked index is
//! filtered out). When `picked == max` (or no candidates remain), the
//! engine fires the final callback and clears `pending_selection`
//! automatically — no submit / PASS action needed.

use digimon_engine::debug_runner::DebugRunner;

/// Resolve a `SelectCountCappedMulti` by picking the candidate at each of
/// the supplied **offsets within the current `pending_selection.valid_action_ids`**.
/// Offsets are NOT zone indices — they are positions in the live
/// candidate list at the moment of each pick (which shrinks after each
/// pick because the picked index is filtered out).
///
/// Example: `&[0, 0]` means "pick the first remaining candidate twice"
/// — after the first pick the candidate list shrinks, so offset 0 of the
/// re-armed list is the second card from the original zone.
pub fn resolve_count_capped_picks(runner: &mut DebugRunner, picks: &[usize]) {
    for (step, &offset) in picks.iter().enumerate() {
        let (action_id, selecting_player) = {
            let pending = runner.game.pending_selection.as_ref().unwrap_or_else(|| {
                panic!(
                    "resolve_count_capped_picks: pending_selection vanished before pick {step}"
                )
            });
            assert!(
                offset < pending.valid_action_ids.len(),
                "resolve_count_capped_picks: pick {step} offset {offset} out of range \
                 (valid_action_ids has {} entries)",
                pending.valid_action_ids.len()
            );
            (pending.valid_action_ids[offset], pending.selecting_player)
        };
        runner
            .game
            .resolve_selection(selecting_player, action_id)
            .unwrap_or_else(|e| {
                panic!("resolve_count_capped_picks: pick {step} resolve_selection failed: {e:?}")
            });
    }
}
