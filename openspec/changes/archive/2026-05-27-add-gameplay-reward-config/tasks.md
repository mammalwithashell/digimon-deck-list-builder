## 1. Engine + binding changes (capability: engine-event-emission)

- [x] 1.1 In `code/digimon-engine-py/src/lib.rs::get_rl_state`, add `d.set_item("turn_count", game.turn_count)?;` alongside the existing digivolve counter exposures.
- [x] 1.2 In `code/digimon-engine/src/game.rs`, add `pub n_digivolve_driven_attacks: [u32; 2]` field on `Game` initialized to `[0, 0]` in `Game::new`.
- [x] 1.3 In `code/digimon-engine/src/combat.rs`, locate the attack-resolution path where attacks complete on security and add the counter increment when `attacker.effective_level >= 5` AND `target == AttackTarget::Player` AND the attack actually connected (not blocked/cancelled). Most likely site: inside `pop_and_start_security_check` after the SecurityRevealSnapshot install, OR inside `advance_pending_attack` when `AttackState::Battle` resolves against `Player` target. Investigate to confirm the right hook.
- [x] 1.4 Expose `n_digivolve_driven_attacks` via `get_rl_state` in `code/digimon-engine-py/src/lib.rs`, mirroring the `n_digivolutions` pattern (2-element list indexed by Rust 0-based PlayerId).
- [x] 1.5 Export `BREEDING_TARGET` (and/or `BREEDING_SLOT`) as a module-level constant from the PyO3 binding so Python consumers can import it. Read the value from the canonical Rust source rather than redefining.
- [x] 1.6 Add integration tests under `code/digimon-engine/tests/event_emission/digivolve_driven_attack.rs` covering: Lv5+ attacker on security increments, Lv4 attacker on security does NOT increment, Lv5+ attacker on digimon does NOT increment, blocked Lv5+ attack on security does NOT increment, Security Attack +1 increments by exactly 1 (per-attack semantics).
- [x] 1.7 Run `cargo test --manifest-path code/digimon-engine/Cargo.toml --test event_emission` and verify all event_emission tests pass + no regressions in existing engine tests.
- [x] 1.8 Rebuild PyO3 via `maturin develop --release --manifest-path code/digimon-engine-py/Cargo.toml`. Add a Python smoke test under `code/tests/` that imports `BREEDING_TARGET` from `digimon_engine`, constructs a game, calls `get_rl_state()`, asserts both `turn_count` and `n_digivolve_driven_attacks` keys are present.

## 2. New gameplay.yaml file + loader two-file support (capability: gameplay-reward-config)

- [x] 2.1 Create `code/digimon_gym/agents/reward/gameplay.yaml` with the shape from design.md §D1-D7: single `gameplay` profile with `terminal_outcome` (win/loss/draw + fast_win_bonus_max=0), `quick_win_bonus`, `stall_penalty`, `step_penalty`, `security_remove`, `security_lost`, `digivolve` (weight 0.5), `dna_digivolve` (weight 3.5), `breeding_digivolve` (default reward_per_level), `digivolve_driven_attack` (default params).
- [x] 2.2 In `code/digimon_gym/agents/reward/profile_loader.py`, change `ProfileLoader.__init__` to accept both `gameplay_path` and `profiles_path`. Both files load at construction. Both parse before merge.
- [x] 2.3 Implement two-file merge logic in `_parse_and_resolve` (or refactor to a new helper). Validation order: parse gameplay first → parse profiles second → check name collision → check every profiles.yaml profile has `inherits` reaching a gameplay.yaml profile → merge into single namespace → run existing inheritance resolution.
- [x] 2.4 Validate the `gameplay` profile is the only profile in gameplay.yaml AND does not have `inherits:` set. Raise `ProfileConfigError` at parse time with clear messages for both cases.
- [x] 2.5 Validate every profile in profiles.yaml has `inherits:` set AND chain reaches gameplay.yaml. Raise at parse time naming the offending profile.
- [x] 2.6 Validate name collisions across the two files raise at parse time naming both file paths AND the colliding profile name.
- [x] 2.7 Maintain two separate canonical content hashes — one per file. Both via the existing `_canonical_hash` function but applied to each parsed YAML separately. Expose as `Profiles.gameplay_hash` and `Profiles.profiles_hash`.
- [x] 2.8 In `code/digimon_gym/agents/training_config.py`, add field `reward_gameplay_path: str = "code/digimon_gym/agents/reward/gameplay.yaml"`.
- [x] 2.9 In `code/digimon_gym/agents/pilot_training.py::_build_reward_profile_factory`, change to construct `ProfileLoader(gameplay_path=cfg.reward_gameplay_path, profiles_path=cfg.reward_profiles_path)`. Identity factory fallback when EITHER file is missing.

## 3. Sidecar + resume hash check updates (capability: gameplay-reward-config)

- [x] 3.1 In `code/digimon_gym/agents/reward/run_metadata.py`, extend `write_sidecar` signature with `reward_gameplay_path` and `reward_gameplay_hash` params. Existing fields (`reward_profiles_path`, `reward_profiles_hash`, `reward_profile_override`, `reward_assignments_snapshot`) unchanged.
- [x] 3.2 Extend `check_resume_hash` to compare BOTH gameplay and profiles hashes. Add a new `RewardGameplayHashMismatchError` parallel to `RewardProfilesHashMismatchError`, OR extend the existing error to a `RewardConfigHashMismatchError` that names which file drifted. (Pick one — favor extending the existing class with a `file_name: str` field for simpler downstream handling.)
- [x] 3.3 Update the resume-check call site in `pilot_training.py::train` to pass both hashes to the check. Single CLI flag `--reward-profiles-override-mismatch` continues to cover BOTH files.
- [x] 3.4 Update sidecar tests in `code/tests/rl/test_pilot_training_reward_integration.py` to assert all 6 fields present in the sidecar (the 4 existing + 2 new gameplay fields) and the resume-mismatch error names whichever file drifted.

## 4. New occurrence types + bus derivations (capability: gameplay-reward-config)

- [x] 4.1 In `code/digimon_gym/agents/reward/occurrences.py`, add `turn_count: int` field to `TerminalOutcome` dataclass (alongside existing `step_count`).
- [x] 4.2 Add `is_breeding: bool` field to `Digivolved` dataclass.
- [x] 4.3 Add new `DigivolveDrivenAttack` dataclass: `player: int, attacker_level: int, has_sources: bool, this_turn: bool`.
- [x] 4.4 In `code/digimon_gym/agents/reward/event_bus.py::RewardEventBus`, update `TerminalOutcome` derivation to read `turn_count` from `cur_rl_state` (alongside existing `step_count` from caller).
- [x] 4.5 Update `Digivolved` derivation: read `BREEDING_TARGET` from `digimon_engine` (or via a constant injected at bus construction); set `is_breeding = (ev["source_slot"] == BREEDING_TARGET)` for engine events.
- [x] 4.6 Add `DigivolveDrivenAttack` derivation: compute counter delta from `cur_rl_state["n_digivolve_driven_attacks"][0]` vs `prev_rl_state["n_digivolve_driven_attacks"][0]`. For each unit of delta, emit one occurrence. The bus reads the attacker's permanent state (last attack target's source permanent) for `has_sources` (= `len(card_sources) > 1`) and `this_turn` (= `turn_digivolved == game.turn_count`). When attacker info is unavailable (edge case), set flags to false defensively.
- [x] 4.7 Update `code/tests/rl/test_event_bus.py` to cover the new occurrence + flag enrichment paths. Add scenarios: TerminalOutcome carries turn_count, Digivolved sets is_breeding correctly for breeding-slot vs battle-area, DigivolveDrivenAttack emitted per counter-delta with correct flags.

## 5. New components (capability: gameplay-reward-config)

- [x] 5.1 Implement `code/digimon_gym/agents/reward/components/quick_win.py` with `QuickWinBonusComponent`. Constructor params: `name, peak_turn=3, peak_value=5.0, decay_per_turn=1.25`. Compute formula: `max(0, peak_value − decay_per_turn × max(0, turn − peak_turn))`. Filter: `winner_id == 1` only.
- [x] 5.2 Implement `code/digimon_gym/agents/reward/components/stall.py` with `StallPenaltyComponent`. Constructor params: `name, threshold_turn=7, scale=0.1, apply_to_winner=true, apply_to_loser=true`. Compute formula: `−scale × max(0, turn − threshold_turn)²`. Apply gates: zero out when winner=1 and `apply_to_winner=false`; zero when winner=2 and `apply_to_loser=false`; draws always penalize.
- [x] 5.3 Implement `code/digimon_gym/agents/reward/components/breeding.py` with `BreedingDigivolveComponent`. Constructor params: `name, reward_per_level: Mapping[int, float]`. Filters: `player==1`, `is_breeding==true`. Lookup `result_level` in dict; missing keys → 0.
- [x] 5.4 Implement `code/digimon_gym/agents/reward/components/digivolve_driven_attack.py` with `DigivolveDrivenAttackComponent`. Constructor params: `name, mode='either', attacker_min_level=5, reward=0.5, per_card=false`. Filters: `attacker_level >= attacker_min_level` + mode predicate (this_turn / has_sources / either / both). `per_card=true` logs a warning at construction and behaves as `per_card=false`.
- [x] 5.5 In `code/digimon_gym/agents/reward/registry.py`, register all 4 new kinds: `quick_win_bonus`, `stall_penalty`, `breeding_digivolve`, `digivolve_driven_attack`. Add appropriate entries to `KIND_KEY_PARAMETERS` (all empty — single-instance components).
- [x] 5.6 Update `code/digimon_gym/agents/reward/profile_loader.py::_component_kwargs` to handle the new components' parameters. Most are single-scalar-param patterns (matches existing pattern); `breeding_digivolve.reward_per_level` is a dict — handle as pass-through.

## 6. Update shipped profiles.yaml (capability: gameplay-reward-config)

- [x] 6.1 Edit `code/digimon_gym/agents/reward/profiles.yaml`: change `_default` from inheriting `_base_terminal` to `inherits: gameplay` with no overrides.
- [x] 6.2 Change `dna_omnimon_combo_v1` from `inherits: _default` to `inherits: gameplay`. Keep all key_cards and supporting components unchanged.
- [x] 6.3 Change `bg_imperialdramon_combo_v1` from `inherits: _default` to `inherits: gameplay`. Keep all overlays unchanged.
- [x] 6.4 REMOVE `_digivolve_shaped` profile from profiles.yaml entirely.
- [x] 6.5 REMOVE `_base_terminal` profile from profiles.yaml entirely.
- [x] 6.6 Verify the merged-namespace loader resolves all four shipped profiles (`gameplay`, `_default`, `dna_omnimon_combo_v1`, `bg_imperialdramon_combo_v1`) successfully via a unit test in `test_profile_loader.py`. (Verified via inline smoke + dedicated test in `test_gameplay_loader.py` — Group 10.)

## 7. Wrapper + telemetry updates (capability: gameplay-reward-config, modified reward-profiles)

- [x] 7.1 In `code/digimon_gym/agents/reward/wrapper.py`, REMOVE the `legacy_terminal_exclusivity` handling from `step()`. The block that suppresses non-terminal_outcome components on terminal steps when the profile flag is set goes away.
- [x] 7.2 In `code/digimon_gym/agents/reward/profile_loader.py`, REMOVE the `legacy_terminal_exclusivity` field from `Profile` dataclass + `_RawProfile` + parsing logic.
- [x] 7.3 In `code/digimon_gym/agents/pilot_training.py::WinRateCallback`, add new accumulators: `_window_total_winning_turn: float`, `_window_total_winning_games: int` for `mean_eval_winning_turn`; and `_window_total_digivolve_driven_attacks: int` for the per-attack scalar.
- [x] 7.4 In the per-game terminal hook, accumulate winning_turn (when winner_id==1, add `final_state.turn_count` to the total + increment games count) and digivolve_driven_attacks (sum of agent-side DigivolveDrivenAttack occurrences across the game).
- [x] 7.5 At eval write, emit TB scalars `pilot/mean_eval_winning_turn` (when winning_games > 0) and `pilot/mean_eval_digivolve_driven_attacks` (always).
- [x] 7.6 Reset both accumulators in the per-window reset block alongside the existing boss-arrival reset.

## 8. TrainingConfig + TrainingRunMetadata persistence (capability: gameplay-reward-config)

- [x] 8.1 Verify `TrainingConfig.reward_gameplay_path` field landed cleanly (added in 2.8).
- [x] 8.2 In `code/digimon_gym/agents/training_metrics.py::TrainingRunMetadata`, add fields `reward_gameplay_path: str = ""`, `reward_gameplay_hash: str = ""`. Both populated at run-start.
- [x] 8.3 In `pilot_training.py::train` at the metadata-write site, populate the new fields from the loaded `Profiles` snapshot.
- [x] 8.4 Add a `digivolve_shaping=True` inert-handling note in `TrainingConfig._validate`: when set True, do NOT emit a warning (preserve v1 contract) but DO NOT remap to a `_digivolve_shaped` profile in `_build_reward_profile_factory` either (the profile is gone). The flag becomes a true no-op. (Done in Group 2 refactor — the existing `_validate` already preserved v1 contract for `digivolve_shaping`; only the factory mapping needed dropping.)
- [x] 8.5 In `_build_reward_profile_factory`, REMOVE the `digivolve_shaping → _digivolve_shaped` mapping. The function returns the standard archetype/override path; setting `digivolve_shaping=True` no longer forces a profile override. (Done in Group 2; see updated docstring + `effective_override` logic.)

## 9. Regression test surgery (capability: reward-profiles MODIFIED)

- [x] 9.1 DELETE `code/tests/rl/test_default_profile_byte_identical.py` — the legacy byte-identical contract no longer exists.
- [x] 9.2 DELETE `code/tests/rl/test_digivolve_shaped_profile_parity.py` — the `_digivolve_shaped` profile is removed.
- [x] 9.3 Update `code/tests/rl/test_profile_loader.py` to remove any test referencing `_digivolve_shaped`, `_base_terminal`, or `legacy_terminal_exclusivity`. Add tests for the new two-file loader behavior (3 scenarios: cross-file inheritance, name collision, missing-inherits-fails-for-profiles-yaml-profile). (No existing refs to scrub; new two-file scenarios live in `test_gameplay_loader.py` — Group 10.5.)
- [x] 9.4 Update `code/tests/rl/test_reward_profile_wrapper.py` to remove tests that rely on `legacy_terminal_exclusivity`. The wrapper no longer special-cases terminal steps. (Verified — no existing references in the wrapper test file.)

## 10. New component tests (capability: gameplay-reward-config)

- [x] 10.1 Write `code/tests/rl/test_quick_win_bonus.py` covering: peak fires at peak_turn on agent win, linear decay 3→4→5→6→7 hits 5.0/3.75/2.5/1.25/0.0, no firing before peak_turn, no firing on loss/draw, custom parameters (peak_turn=5, peak_value=10, decay_per_turn=2.0) compute correctly.
- [x] 10.2 Write `code/tests/rl/test_stall_penalty.py` covering: no penalty at or before threshold, quadratic growth for turns 10/15/20/30, applies to all outcomes by default, `apply_to_winner=false` zeroes win-only, `apply_to_loser=false` zeroes loss-only, draws always penalized regardless of flags.
- [x] 10.3 Write `code/tests/rl/test_breeding_digivolve.py` covering: Lv4 raise emits 0.2, Lv6 raise emits -0.4, battle-area digivolve ignored (is_breeding=false), missing level emits 0, opponent's breeding digivolve ignored (player=2).
- [x] 10.4 Write `code/tests/rl/test_digivolve_driven_attack.py` covering: mode=either fires on this_turn OR has_sources, mode=this_turn doesn't fire on has_sources alone, attacker_min_level filter, per_card=true emits load warning + behaves as false.
- [x] 10.5 Write `code/tests/rl/test_gameplay_loader.py` covering: missing gameplay.yaml fails fast, gameplay-with-inherits fails, multi-profile-gameplay fails, name collision fails, profiles.yaml profile without inherits fails, cross-file inheritance resolves, two separate hashes computed.
- [x] 10.6 Write `code/tests/rl/test_terminal_landscape.py` — hand-computed table of (turn, outcome) → expected terminal scalar under default gameplay shape. ~10 representative cases covering turn 3 win = +15, turn 7 win = +10, turn 15 win = +3.6, turn 20 loss = -26.9, turn 30 draw = -53.9, etc. Asserts the full assembled-from-components terminal value matches the table.

## 11. Documentation (cross-cutting)

- [x] 11.1 Update `docs/REWARD_PROFILES.md`:
   - Add "Two-file architecture" section at the top
   - Update component catalog to include quick_win_bonus, stall_penalty, breeding_digivolve, digivolve_driven_attack
   - Update worked examples to inherit `gameplay` instead of `_default`
   - Remove documentation of `legacy_terminal_exclusivity` flag
   - Remove documentation of `_digivolve_shaped` and `_base_terminal` profiles
   - Add migration note for resumes of pre-change checkpoints
   - Add "Concede behavior" callout noting ship-and-observe approach + the existing concede-rate scalar
- [x] 11.2 Update `docs/TRAINING_RUNBOOK.md` §13 "Reward profiles" section: add note about the gameplay.yaml file, new shape personality ("win fast or it hurts"), and that `digivolve_shaping=True` is now inert.
- [x] 11.3 Update `docs/RUST_ENGINE_API.md` "Engine event emission" callout: add note about `turn_count` exposure and the `n_digivolve_driven_attacks` counter.

## 12. Validation

- [x] 12.1 Run `cargo test --manifest-path code/digimon-engine/Cargo.toml` — full Rust suite passes including new digivolve_driven_attack tests. (712 passed; 1 pre-existing unrelated failure documented in session memory: `select_materials_batch_play_from_materials_plays_every_picked_source`.)
- [x] 12.2 Run `python -m pytest code/tests/rl/` — full Python rl suite passes. Expect the 3 pre-existing tensor_profiles failures (not mine). All new component + loader + wrapper + telemetry tests green. (537 passed; tensor_profiles failures resolved between changes — no failures observed in final run.)
- [x] 12.3 Run `openspec validate add-gameplay-reward-config --strict` and confirm no validation errors.
- [x] 12.4 Smoke-load the shipped `gameplay.yaml` + `profiles.yaml` via Python REPL: verify all 4 profiles materialize, gameplay's terminal scalar at turn 3 win equals +15.0 (sum of terminal_outcome=10 + quick_win_bonus=5).
- [ ] 12.5 (Deferred) 10000-step pilot_training smoke run with default config. Verify TB log contains the new scalars + sidecar contains both gameplay_hash and profiles_hash. Defer to user-driven validation — ~30-60min real training time.
- [ ] 12.6 (Deferred) Concede-rate monitoring: after first real training run with new shape, inspect `pilot/concede_rate` — if it spikes above ~50%, tune `stall_penalty` parameters.
