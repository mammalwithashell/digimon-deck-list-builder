## Why

`add-reward-profiles` shipped a profile-driven reward framework where the universal baseline (`_default` profile) is byte-identical to the legacy `DigimonEnv._compute_reward`. The legacy shape encodes a single design intent (sec-removal-dominant, mildly digivolve-shaped via opt-in flag) and was preserved for safe rollout. Operators want to evolve the universal baseline to a more aggressive, gameplay-mechanic-aware shape — quick-win bonus, stall penalty, richer digivolve rewards, breeding-area shaping, digivolve-driven attack signal — without conflating those universal changes with per-archetype shaping.

The framework already supports this via custom profiles, but two problems push toward a separate file:

1. **Conceptual conflation.** Universal game-mechanic rewards (e.g., "winning fast is good") are categorically different from archetype-specific overlays (e.g., "rewarding Omnimon arrival"). Mixing them in one file invites edits that change archetype shape while intending to tune the universal baseline.
2. **Tuning churn.** Universal-shape tuning is the most frequent reward-shape edit during training experiments. Confining it to its own file means archetype profiles don't drift when the operator tunes the baseline.

This change introduces `gameplay.yaml` as a separate file alongside `profiles.yaml`. Every profile in `profiles.yaml` inherits from a base defined in `gameplay.yaml` (typically the `gameplay` profile itself). Universal-baseline tuning happens in `gameplay.yaml`; archetype overlays stay in `profiles.yaml`.

Concretely, this change also evolves the universal baseline to a sharper "win fast or it hurts" shape, drops the legacy `_default` shape entirely (no `_legacy_default` carryover), and adds four new components (`quick_win_bonus`, `stall_penalty`, `breeding_digivolve`, `digivolve_driven_attack`) plus the engine plumbing they need.

## What Changes

- **New** `code/digimon_gym/agents/reward/gameplay.yaml`: universal game-mechanic reward shape defined as a single profile (`gameplay`). Every profile in `profiles.yaml` inherits from `gameplay`. Two-file loader merges namespaces.
- **New** components in the registry:
  - `quick_win_bonus` — fires on agent-win `TerminalOutcome`. Params: `peak_turn` (default 3 — earliest P1 win), `peak_value` (default +5.0), `decay_per_turn` (default 1.25). Formula: `max(0, peak_value − decay_per_turn × max(0, turn − peak_turn))`.
  - `stall_penalty` — fires on every `TerminalOutcome` (win OR loss OR draw). Params: `threshold_turn` (default 7), `scale` (default 0.1), `apply_to_winner` (default true), `apply_to_loser` (default true). Formula: `−scale × max(0, turn − threshold_turn)²`.
  - `breeding_digivolve` — fires on `Digivolved` occurrences flagged `is_breeding=true`. Params: `reward_per_level: dict[int, float]` (default `{3: 0.4, 4: 0.2, 5: 0.1, 6: -0.4}`). Each level halves the previous; Lv6 is same magnitude as Lv3 but negative (digivolving to Lv6 in breeding blocks the slot — almost always wrong).
  - `digivolve_driven_attack` — fires when a Lv5+ digi connects with security. Params: `mode` (default `"either"` — fires when attacker was just digivolved OR has `card_sources` under it), `attacker_min_level` (default 5), `reward` (default +0.5), `per_card` (default false — per attack, not per security card revealed).
- **Modified** `RewardEventBus` derivations:
  - `TerminalOutcome` gains `turn_count: int` (in addition to `step_count`). Components pick which they need.
  - `Digivolved` gains `is_breeding: bool`. Bus sets it from `field_index == BREEDING_TARGET`.
  - New `DigivolveDrivenAttack` occurrence derived from the new engine counter delta — carries `player`, `attacker_level`, `has_sources`, `this_turn`.
- **Modified** Rust engine + PyO3 binding:
  - `code/digimon-engine-py/src/lib.rs::get_rl_state` adds `turn_count`.
  - `code/digimon-engine/src/game.rs` gains `n_digivolve_driven_attacks: [u32; 2]`. Incremented in `combat.rs::pop_and_start_security_check` (or the attack-resolution path) when the attacker is Lv5+ AND the attack reached security. The mode predicate (just-digivolved vs has-sources) lives in the Python component; the engine just counts qualifying attack events.
  - `BREEDING_TARGET` / `BREEDING_SLOT` constant exposed from the binding so the bus + breeding_digivolve component identify breeding-area Digivolved events.
- **Modified** `ProfileLoader`: accepts both `gameplay_path` and `profiles_path`. Loads both YAML files, merges namespaces (gameplay's profiles + profiles.yaml's profiles live in one map). Validates that every profile in `profiles.yaml` inherits (transitively) from a profile defined in `gameplay.yaml`. Maintains two separate canonical content hashes.
- **Modified** `TrainingConfig`: adds `reward_gameplay_path` field. The existing `reward_profiles_path` stays. Both files contribute to the sidecar metadata.
- **Modified** reward-profile sidecar (`reward_profiles.meta.json`): now records `gameplay_path` + `gameplay_hash` + `profiles_path` + `profiles_hash` (two pairs). Resume-mismatch error names whichever file drifted.
- **Modified** shipped `profiles.yaml`:
  - `_default` is now `inherits: gameplay` with no overrides.
  - `dna_omnimon_combo_v1` and `bg_imperialdramon_combo_v1` now `inherits: gameplay` instead of `_default`.
  - `_digivolve_shaped` REMOVED (gameplay defines digivolve weights universally).
  - `_base_terminal` REMOVED (gameplay supersedes).
- **Removed** `legacy_terminal_exclusivity` profile-level flag. The new `quick_win_bonus` and `stall_penalty` components fire on terminal naturally — the carve-out exists only because the legacy `_compute_reward` short-circuited on terminal. With legacy dropped, the flag has no consumer.
- **Removed** byte-identical regression test (`test_default_profile_byte_identical.py`) and digivolve-shaped parity test (`test_digivolve_shaped_profile_parity.py`). The legacy reward contract those tests defend no longer exists.
- **Modified** `WinRateCallback` telemetry: adds `pilot/mean_eval_winning_turn` (mean turn_count at terminal for wins) and `pilot/mean_eval_digivolve_driven_attacks`. Per-component scalars for the new components surface automatically via the Group 10 infrastructure (`pilot/reward/quick_win_bonus/mean_per_game` etc.).
- **Modified** `TrainingRunMetadata`: persists `reward_gameplay_path`, `reward_gameplay_hash`, plus the new shaping defaults as runtime fields so paired/baseline runs are mechanically distinguishable (per the digivolve-shaping precedent).
- **Modified** `digivolve_shaping` config flag: still accepted (no error, no new warning); maps to the same `_default` profile (which now equals `gameplay`). The flag becomes inert. v2 removes.

## Capabilities

### New Capabilities

- `gameplay-reward-config`: universal game-mechanic reward shape defined in a YAML file separate from archetype overlays. Includes the 4 new components, two-file loader, gameplay-vs-profile hash separation, and the new aggression-shaped default.

### Modified Capabilities

- `reward-profiles`: profile loader gains the two-file-merge behavior; `_default` profile becomes a thin pass-through inheriting `gameplay`; the `legacy_terminal_exclusivity` flag is removed from the spec; the `_digivolve_shaped` and `_base_terminal` private profiles are removed. Profile inheritance now spans files (`profiles.yaml` profiles inherit from `gameplay.yaml` profiles).
- `engine-event-emission`: PyO3 `get_rl_state` adds `turn_count`. New `n_digivolve_driven_attacks` counter exposed alongside the digivolve counters.

## Impact

- **Affected code (Python)**: `code/digimon_gym/agents/reward/profile_loader.py` (two-file merge), `code/digimon_gym/agents/reward/wrapper.py` (legacy_terminal_exclusivity removal), `code/digimon_gym/agents/reward/components/` (4 new component files), `code/digimon_gym/agents/reward/occurrences.py` (TerminalOutcome.turn_count, Digivolved.is_breeding, new DigivolveDrivenAttack), `code/digimon_gym/agents/reward/event_bus.py` (3 new derivations), `code/digimon_gym/agents/reward/registry.py` (4 new kinds), `code/digimon_gym/agents/reward/run_metadata.py` (gameplay-hash addition), `code/digimon_gym/agents/training_config.py` (new field), `code/digimon_gym/agents/pilot_training.py` (loader wiring + telemetry).
- **Affected code (Rust)**: `code/digimon-engine/src/game.rs` (n_digivolve_driven_attacks counter), `code/digimon-engine/src/combat.rs` (increment site), `code/digimon-engine/src/action/space.rs` or similar (expose BREEDING_TARGET), `code/digimon-engine-py/src/lib.rs` (get_rl_state additions, constant export).
- **Affected data**: new `code/digimon_gym/agents/reward/gameplay.yaml`. Modified `code/digimon_gym/agents/reward/profiles.yaml` (3 profile inheritance updates + 2 private profile removals). No changes to `data/cards.json` / `data/deck_library.json`.
- **Affected configs**: `TrainingConfig` gains 1 new field (`reward_gameplay_path`). Existing field defaults unchanged.
- **Affected docs**: update `docs/REWARD_PROFILES.md` (two-file architecture, new components, dropped flags + profiles); update `docs/RUST_ENGINE_API.md` (new counter + binding addition); no new top-level doc.
- **Out of scope (deferred)**: concede-rate tuning (ship + observe via existing `pilot/concede_rate` scalar); removing the deprecated `digivolve_shaping` / `digivolve_reward` / `dna_digivolve_bonus` TrainingConfig fields (v2 work); per-archetype overrides of gameplay-level signals (already possible via component overrides in profile inheritance — call out as a recipe in docs).
- **Breaking changes**:
  - Resume against an old checkpoint hits the reward-profiles hash mismatch (both gameplay-hash AND profile-hash will differ from the checkpoint's prior single hash). Operator must pass `--reward-profiles-override-mismatch` to continue, accepting the new shape.
  - The byte-identical-to-legacy guarantee from `add-reward-profiles` is intentionally dropped. Any external consumer that asserted "default reward equals legacy" must update.
  - The `_digivolve_shaped` and `_base_terminal` profiles vanish; YAML configs that explicitly reference them in `inherits:` will fail to load.
- **No breaking changes** to: action space, observation tensor, replay format, saved checkpoint structure. Existing models load and evaluate; only the reward signal during inference is irrelevant (inference doesn't compute reward).
