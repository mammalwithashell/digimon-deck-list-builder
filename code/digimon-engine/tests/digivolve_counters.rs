//! Integration tests for `Game::n_digivolutions` / `Game::n_dna_digivolutions`
//! counter instrumentation. These counters back the digivolve reward-shaping
//! signal in `DigimonEnv._compute_reward`; see
//! `docs/superpowers/specs/2026-05-23-digivolve-reward-shaping-design.md`.

use digimon_engine::debug_runner::DebugRunner;

#[test]
fn new_game_starts_with_zero_digivolution_counters() {
    let runner = DebugRunner::builder().start();
    assert_eq!(runner.game.n_digivolutions, [0u32, 0u32]);
    assert_eq!(runner.game.n_dna_digivolutions, [0u32, 0u32]);
}
