## Context

The just-shipped `add-reward-profiles` change (94/102 tasks, 156 reward-stack tests green) established a profile-driven reward framework in `code/digimon_gym/agents/reward/`. Its single shipped baseline (`_default`) is byte-identical to legacy `DigimonEnv._compute_reward` to preserve existing-run behavior during rollout. That baseline includes:

- Terminal: `+10 win` + linear fast-win bonus (cap `+5` at step 0, zero by step 200), `−10 loss`, `−1 draw`.
- Dense: `+1.5 / -0.5` per security removed/lost.
- Optional digivolve shaping: `+0.1 / +3.9` (regular + DNA additive) via the `_digivolve_shaped` profile.
- Step penalty: `−0.001`.
- Profile-level `legacy_terminal_exclusivity` flag suppresses every non-terminal component on the terminal step — required for byte-identical behavior because the legacy code short-circuits on terminal.

Operators want to retire that legacy shape and adopt a new aggression-shaped default — quick wins rewarded heavily, slow games penalized quadratically (regardless of outcome), richer digivolve signal, breeding-area shaping, and a digivolve-driven-attack reward.

The framework already permits all of this via custom profiles. What it does NOT cleanly express:

- A separate FILE for universal game-mechanic shaping (vs archetype overlays). The current single `profiles.yaml` mixes baseline + overlays.
- Components for non-linear terminal curves (quick-win-bonus, stall-penalty).
- A component that reads `turn_count` (engine binding currently exposes only `step_count`).
- A breeding-area digivolve signal distinguished from battle-area.
- A digivolve-driven-attack signal (needs a new engine counter).

This change adds the file split + four new components + the engine plumbing they need. It drops the legacy baseline entirely — no `_legacy_default` carryover — so resumes against old checkpoints hit the existing reward-profiles hash mismatch and require the operator to explicitly accept the new shape via `--reward-profiles-override-mismatch`.

## Goals / Non-Goals

**Goals:**

- Separate FILE for universal gameplay reward shape (`gameplay.yaml`) distinct from archetype overlays (`profiles.yaml`). Loader merges both into one profile namespace; profile inheritance crosses files.
- Four new components: `quick_win_bonus`, `stall_penalty`, `breeding_digivolve`, `digivolve_driven_attack`. Each parameterized for tunability.
- Engine plumbing: expose `turn_count` in `get_rl_state`; new `n_digivolve_driven_attacks` counter incremented in combat resolution.
- Bus enrichment: `TerminalOutcome.turn_count`, `Digivolved.is_breeding`, new `DigivolveDrivenAttack` occurrence.
- Drop legacy default entirely. `_default` becomes a thin pass-through to `gameplay`. `_digivolve_shaped` + `_base_terminal` private profiles removed. `legacy_terminal_exclusivity` flag removed from spec + code.
- Sidecar persists gameplay-hash AND profiles-hash separately so resume-mismatch error names which file drifted.
- Telemetry: `pilot/mean_eval_winning_turn`, `pilot/mean_eval_digivolve_driven_attacks`, plus per-component TB scalars via existing Group 10 infrastructure.

**Non-Goals:**

- Tuning concede behavior. The new shape may shift concede rates; we ship + observe via the existing `pilot/concede_rate` scalar and tune from real training data rather than pre-tuning.
- Removing the deprecated `digivolve_shaping` / `digivolve_reward` / `dna_digivolve_bonus` TrainingConfig fields. They're already deprecated-warned in v1; removal is v2 work.
- Per-archetype overrides of gameplay-level signals (e.g., a control archetype that disables stall penalty). Already possible via component overrides in `profiles.yaml` inheritance — documented as a recipe in `docs/REWARD_PROFILES.md`, no new mechanism needed.
- Caps / clamps on the stall penalty. The penalty grows unboundedly with turn count; the agent's job is to never reach those turn counts.
- A `_legacy_default` carryover profile that recreates the old shape for resume continuity. Operators with old checkpoints either accept the new shape on resume (via the override flag) or fine-tune fresh.

## Decisions

### D1: Two-file split — `gameplay.yaml` and `profiles.yaml`

`gameplay.yaml` defines a SINGLE profile (`gameplay`) containing the universal game-mechanic shaping. `profiles.yaml` defines archetype overlays. Both files are loaded by `ProfileLoader`; their profile namespaces are merged into one map.

```
code/digimon_gym/agents/reward/
├── gameplay.yaml      ← universal game-mechanic shaping
│                        defines a single profile: `gameplay`
└── profiles.yaml      ← archetype overlays
                         every profile inherits from `gameplay`
                         (transitively — directly or via a private base)
```

**Why two files**:

- **Conceptual separation.** Universal shaping ("winning fast is good", "Lv6 in breeding is wasted") is categorically different from archetype-specific shaping ("DNA Omnimon is the win condition"). Mixing them invites edits that change archetype shape while intending to tune the baseline.
- **Edit churn isolation.** Universal tuning is the most frequent edit during training experiments. Keeping it in its own file means archetype profiles don't churn when the operator tunes the baseline.
- **Telemetry & resume**. Two separate content hashes let the resume-mismatch error name which file drifted. Operators see "you edited gameplay.yaml mid-run" vs "you edited profiles.yaml mid-run" directly.

**Why not a single file with conventions**: tried-and-true file-level separation is more enforceable than "use the `_gameplay_*` private-profile convention". The loader's mandatory-inheritance-from-gameplay rule makes the split structural.

**Alternative considered: one file, gameplay as a private profile (`_gameplay`).** Rejected because:
- Edits to the gameplay base are commingled with archetype edits in the same git diff.
- No hash separation — operator can't tell from the resume-mismatch error which signal changed.
- The mandatory-inheritance rule has nothing to anchor against ("inherit from a `_*` profile" is too loose).

### D2: Cross-file inheritance via merged namespace

`ProfileLoader(gameplay_path, profiles_path)` loads both files in parallel:

1. Parse `gameplay.yaml` → temporary profile map A.
2. Parse `profiles.yaml` → temporary profile map B.
3. Validate name collision: profiles in B SHALL NOT shadow profiles in A. Loader fails at parse time if any name appears in both.
4. Merge into a single map: `A ∪ B`.
5. Validate every profile in B has `inherits:` set AND inheritance chain (transitively) reaches a profile defined in A. Fails at parse time otherwise — message names the offending profile.
6. Resolve inheritance using the merged map (same algorithm as today).

**Why mandatory inheritance from a gameplay profile**: enforces the architectural split. Without it, an archetype profile could redeclare every component, accidentally creating a parallel baseline that drifts from gameplay.

**Why parse-time validation (not load-time)**: errors surface immediately on file edits via the existing hot-reload pathway; operators don't ship a misconfigured run that fails only after the first eval.

### D3: Two content hashes, two sidecar fields

The Group 8 `reward_profiles.meta.json` sidecar gains two new fields:

```json
{
  "reward_gameplay_path":    "code/digimon_gym/agents/reward/gameplay.yaml",
  "reward_gameplay_hash":    "sha256:<hex>",
  "reward_profiles_path":    "code/digimon_gym/agents/reward/profiles.yaml",
  "reward_profiles_hash":    "sha256:<hex>",
  "reward_profile_override": <str | null>,
  "reward_assignments_snapshot": { ... }
}
```

On resume, the check compares BOTH hashes. Mismatch error names which file drifted:

```
Reward gameplay shape changed since checkpoint.
  Checkpoint hash: sha256:abc...
  Current hash:    sha256:def...
Pass --reward-profiles-override-mismatch to proceed anyway.
```

The `--reward-profiles-override-mismatch` CLI flag covers BOTH file hashes — operators don't need separate flags. If only one file drifted, the error message identifies which.

### D4: `quick_win_bonus` — turn-based not step-based

The legacy `terminal_outcome.fast_win_bonus` was step-based (`fast_win_par_steps: 200`). Steps don't map cleanly to game progress — a 7-turn game with many selection steps can have 100+ env steps. Turn count is the natural unit.

**Component contract**:

```python
@dataclass
class QuickWinBonusComponent:
    name: str
    peak_turn: int                # default 3 (earliest possible P1 win)
    peak_value: float             # default +5.0
    decay_per_turn: float         # default 1.25
    # Formula:
    #   bonus = max(0, peak_value − decay_per_turn × max(0, turn − peak_turn))
    # Fires only when winner_id == 1 (agent won).
```

Default values produce the curve:

```
  turn  bonus       total win (with base +10)
  ────────────────────────────────────────────
    3   +5.00       +15.00         ← peak
    4   +3.75       +13.75
    5   +2.50       +12.50
    6   +1.25       +11.25
    7    0          +10.00         ← clean seam with stall start
    8+   0          +10.00 (then stall penalty applies)
```

The `decay_per_turn=1.25` choice is specifically calibrated so the bonus reaches exactly 0 at `turn = 7`, where `stall_penalty` starts. No overlap, no gap.

**Why fire only on win**: a fast LOSS is still a loss; there's no "quick-loss bonus" interpretation. The signal explicitly rewards "you won, and you won fast."

**Reads `turn_count`, not `step_count`**: per the engine change, `TerminalOutcome.turn_count` is bus-enriched from `rl_state.turn_count`. The existing `terminal_outcome` component's `fast_win_par_steps` keeps reading `step_count` for backward-compat with custom profiles that use it; `gameplay.yaml` sets `fast_win_bonus_max: 0` to disable that path while keeping the variant available.

### D5: `stall_penalty` — symmetric quadratic on every terminal

```python
@dataclass
class StallPenaltyComponent:
    name: str
    threshold_turn: int           # default 7
    scale: float                  # default 0.1
    apply_to_winner: bool         # default true
    apply_to_loser: bool          # default true
    # Formula:
    #   penalty = −scale × max(0, turn − threshold_turn)²
    # Fires on EVERY TerminalOutcome (win/loss/draw) when the respective
    # apply_to_* flag is true. Draws always get the penalty (apply_to_*
    # gates only wins/losses).
```

The penalty grows **unboundedly** with turn count. By design:

```
  turn   penalty
  ─────────────
    7     0
   10    -0.9
   15    -6.4
   20   -16.9
   30   -52.9
   50  -184.9
```

Total terminal under default gameplay shape (win + stall):

```
  turn   bonus  stall    win total   loss total   draw total
  ───────────────────────────────────────────────────────────
    3   +5.00    0       +15.00      −10.00       −1.00
    7    0       0       +10.00      −10.00       −1.00
   10    0     -0.9       +9.10      −10.90       −1.90
   20    0    -16.9       −6.90      −26.90      −17.90      ← draw worse than loss
   30    0    -52.9      −42.90      −62.90      −53.90      ← draw still worst
```

**Asymmetric implication**: a forced step-limit win at turn 30 nets `-42.9`, vs being-forced-to-lose at `-62.9`. The 20-point spread still rewards "win" — but BOTH outcomes are catastrophic. The agent's correct policy is "never play 30-turn games."

**No cap, by design**: caps create discontinuities and let the agent treat all turn-N+ games as equivalently bad. Unbounded growth keeps the gradient meaningful at every turn.

**Concede implication**: concede at turn 8 = `−10 − 0.1 = −10.1`, vs lose-naturally at turn 20 = `−26.9`. Delta of `+16.8` favors concede. We ship + observe via the existing `pilot/concede_rate` scalar; if concede rate exceeds operator threshold (e.g., >50%) the operator tunes — likely by raising `threshold_turn` or lowering `scale` for losses specifically.

### D6: `breeding_digivolve` — per-level lookup

```python
@dataclass
class BreedingDigivolveComponent:
    name: str
    reward_per_level: Mapping[int, float]
    # Default: {3: 0.4, 4: 0.2, 5: 0.1, 6: -0.4}
    # Halves each step; Lv6 same magnitude as Lv3 but negative.
```

Default rationale:

- `Lv2 → Lv3`: most eggs hatch to Lv2 (Babies). The first productive raise; cleans the slot for the next egg.
- `Lv3 → Lv4`: half — still useful, less critical.
- `Lv4 → Lv5`: half again — getting close to the "move to battle" decision; small reward.
- `Lv5 → Lv6`: same magnitude as Lv3 but **negative**. Digivolving to Lv6 in breeding almost always wrong — it locks the slot and the Lv6 can't be played onto the field with proper memory cost. Should move to battle area first.

The component consumes `Digivolved` occurrences where `is_breeding == true`. Lookup is exact-match: a result level not in the dict produces zero. Operators tuning the values can add or remove level entries.

**Why a dict, not a continuous formula**: per-level rewards are discrete and small. A formula adds complexity without expressiveness — the dict is read like a table, matches the discrete game logic, and is easy to tune by hand.

**Alternative considered: linear / exponential decay parameters.** Rejected for v1. If operators ask for it, the component can grow a `formula:` field that overrides `reward_per_level`. Not in scope here.

### D7: `digivolve_driven_attack` — engine counter + Python component filter

The signal: "a Lv5+ digi connected with security, having either been just-digivolved this turn OR having `card_sources` under it." Mode parameter on the component selects between "this_turn", "has_sources", "either", "both".

Engine work (Rust):

- New counter `n_digivolve_driven_attacks: [u32; 2]` on `Game`.
- Incremented in `combat.rs` attack-resolution path (the `pop_and_start_security_check` site or its precursor) when `attacker.effective_level >= attacker_min_level (default 5)` AND `attack.target == AttackTarget::Player` (i.e., security stack). NOT incremented on blocked attacks or digi-vs-digi battles.
- Per-attack semantics: the counter bumps once per attack regardless of `Security Attack +N` (i.e., per-card revelations don't double-count).
- The engine increments unconditionally on qualifying attacks; the **mode predicate lives in the Python component**, not the engine. The bus enriches the occurrence with `has_sources` and `this_turn` flags; the component filters.

Bus derivation:

- New `DigivolveDrivenAttack(player: int, attacker_level: int, has_sources: bool, this_turn: bool)` occurrence.
- Derived from `n_digivolve_driven_attacks[player]` counter delta. The bus pairs each delta with the attacker's `card_sources.len() > 1` and `turn_digivolved == current_turn` flags read from the post-step state. (Edge case: if multiple qualifying attacks fire in one env step, the bus emits one occurrence per attack; flags reflect the attacker for each.)

Component contract:

```python
@dataclass
class DigivolveDrivenAttackComponent:
    name: str
    mode: str                     # one of {this_turn, has_sources, either, both}
    attacker_min_level: int       # default 5
    reward: float                 # default +0.5
    per_card: bool                # default false (per attack, not per card)
    # `attacker_min_level` is RE-CHECKED by the component (engine already
    # filtered) so the component is meaningful when set higher than the
    # engine's filter; engine's filter is the loose lower bound.
```

Mode predicate:

| mode | matches if |
|---|---|
| `this_turn` | event.this_turn |
| `has_sources` | event.has_sources |
| `either` | event.this_turn OR event.has_sources (broad) |
| `both` | event.this_turn AND event.has_sources (narrow) |

Default `mode: either` per operator decision; broad signal that fires on most aggressive Lv5+ attacks.

**Why mode in component, not engine**: lets operators tune the signal scope without rebuilding the engine. The engine just counts qualifying attacks; the component filters down to the operator's preferred shape.

**Per-card NOT supported in v1**: would require the bus to read `SecurityReveal` events per attack and correlate with the counter delta. Adds complexity for marginal value. The component defaults `per_card: false` and ignores the parameter for now — documented as a v2 extension point.

### D8: Drop legacy entirely — no `_legacy_default` carryover

The `_default` profile becomes:

```yaml
_default:
  inherits: gameplay
  # No overrides — _default IS gameplay.
```

`_digivolve_shaped` and `_base_terminal` are REMOVED from the shipped `profiles.yaml`. Existing YAML configs that `inherits: _digivolve_shaped` will fail to load with a clear "profile not defined" error.

Operators with old checkpoints resuming under the new code hit the reward-profiles hash mismatch (both gameplay-hash and profiles-hash differ from the checkpoint's pre-change recorded hashes). They must pass `--reward-profiles-override-mismatch` to continue, acknowledging the reward shape change.

**Why no `_legacy_default` carryover**: the entire point of the framework is to retire the legacy fork. Carrying a parallel "byte-identical to OLD legacy" profile is the same maintenance burden the framework was supposed to eliminate. Operators who want continuity should evaluate the old checkpoint under the new shape (forfeit byte-identical reward) or fine-tune fresh.

**`legacy_terminal_exclusivity` flag REMOVED**: the legacy quirk it patched (legacy `_compute_reward` short-circuiting on terminal) doesn't exist under the new shape. `quick_win_bonus` and `stall_penalty` fire on terminal naturally — no carve-out needed.

### D9: `digivolve_shaping` config flag becomes inert

Existing v1 contract: `digivolve_shaping=True` maps to `reward_profile_override = "_digivolve_shaped"`.

New contract: `digivolve_shaping=True` is **silently no-op**. The flag is still accepted by `TrainingConfig._validate` (no error) but maps to nothing — the gameplay default already includes digivolve weights universally. Existing YAML configs that set `digivolve_shaping: true` continue to load and train; they just train under the new gameplay default rather than the old `_digivolve_shaped` profile.

**No new deprecation warning**: the v1 deprecation timeline for this flag was "no warning in v1, removal in v2." This change preserves that — v2 still removes it.

### D10: TrainingRunMetadata persists shaping config

New fields on `TrainingRunMetadata`:

- `reward_gameplay_path: str`
- `reward_gameplay_hash: str`
- `reward_gameplay_overrides: dict[str, Any]` — flattened component params from the active gameplay profile (e.g., `{"quick_win_bonus.peak_value": 5.0, "stall_penalty.scale": 0.1, ...}`). Lets downstream tooling distinguish paired/baseline runs without re-loading the YAML.

The existing `hyperparameters` dict already includes everything from `cfg.to_dict()`. The new fields are TOP-LEVEL for ease of querying.

### D11: Telemetry additions

New TB scalars in `WinRateCallback`:

- `pilot/mean_eval_winning_turn` — mean `turn_count` at terminal across eval-window wins. Lets operators see the typical winning game length.
- `pilot/mean_eval_digivolve_driven_attacks` — mean count of qualifying attacks per eval game.

Per-component TB scalars for the 4 new components (`pilot/reward/quick_win_bonus/mean_per_game`, etc.) surface automatically via the Group 10 callback infrastructure — no new wiring needed.

The `pilot/concede_rate` scalar already exists from BO3 match training; we'll watch it during the first training run to confirm concede rate doesn't spike past acceptable thresholds under the new stall penalty.

## Risks / Trade-offs

- **[Risk] Concede rate spikes under unbounded stall penalty.** → Mitigation: ship + observe via `pilot/concede_rate`. If it exceeds operator threshold (~50%), tune `stall_penalty.scale` or `stall_penalty.threshold_turn` for losses specifically. The component's `apply_to_winner` / `apply_to_loser` flags already support asymmetric tuning.
- **[Risk] Step-limit forced wins become net-negative under new shape.** → Mitigation: this is by design — agent shouldn't play 30-turn games. The force-step-limit-winner engine mechanism still fires; it just produces a negative-reward "win" that the agent learns to avoid. Documented as intended behavior in `docs/REWARD_PROFILES.md`.
- **[Risk] Old checkpoints become incompatible without `--reward-profiles-override-mismatch`.** → Mitigation: the flag exists from v1 and is documented. Resume-mismatch error message clearly explains the situation + names which file drifted. Operators can also evaluate old models without resuming (eval doesn't compare hashes).
- **[Risk] `_digivolve_shaped` / `_base_terminal` removal breaks YAML configs that `inherits:` them.** → Mitigation: loader fails fast at parse time with a clear "profile not defined" error. v1-era configs that used these profile names are uncommon (introduced same day). Migration is a 1-line change to `inherits: gameplay` in the affected YAML.
- **[Risk] Two-file loader complicates the existing single-file mental model.** → Mitigation: documented in `docs/REWARD_PROFILES.md` with a clear "edit gameplay.yaml for baseline, edit profiles.yaml for archetype" rule. Mandatory-inheritance-from-gameplay enforces the architectural split structurally.
- **[Risk] Engine counter `n_digivolve_driven_attacks` increment site is in attack-resolution; subtle bugs in the predicate (lv ≥ 5, target == security) could over- or under-count.** → Mitigation: TDD-style. Write integration tests under `code/digimon-engine/tests/event_emission/` asserting counter increments only for the matching attack shape; cover Lv4 attacker (no increment), Lv5+ attacker on security (increment), Lv5+ attacker on digimon (no increment), blocked Lv5+ attack (counter behavior TBD — likely doesn't increment since the attack didn't connect).
- **[Risk] Bus-side mode-filter divergence from engine-side counter.** → Mitigation: spec scenarios assert the bus-emitted `DigivolveDrivenAttack.this_turn` / `has_sources` flags match the engine state at the moment of attack. Bus reads attacker permanent state immediately after counter delta detection.
- **[Trade-off] Stall penalty applies on draws (`apply_to_winner` / `apply_to_loser` flags don't gate draws).** → Draws are extremely rare under force-step-limit-wins. The penalty applying on draws is a small additional incentive against "let it ride to step limit" agents; the rarity makes the trade-off mostly cosmetic.
- **[Trade-off] No `_legacy_default` carryover means no graceful migration for existing trained models.** → Operators with checkpointed models must either accept the new reward shape on resume or fine-tune fresh. Acceptable given v1 just shipped — existing models aren't deeply invested in the legacy shape yet.

## Migration Plan

1. **Ship the change.** Existing training YAML configs that don't explicitly reference removed profiles continue to work — `_default` still exists, just now inherits `gameplay` instead of `_base_terminal`. The new shape is the new universal default.
2. **Update YAML configs that explicitly reference removed profiles.** Configs that `inherits: _digivolve_shaped` or `inherits: _base_terminal` need to change to `inherits: gameplay`. Loader fails fast on the missing profile; no silent breakage.
3. **For resumes of old checkpoints**: pass `--reward-profiles-override-mismatch` on the resume command. Accepts that the reward signal has changed since the checkpoint was created.
4. **For fresh training under the new shape**: no action required. Default `TrainingConfig.reward_gameplay_path` points at the shipped `gameplay.yaml`; default `reward_profiles_path` points at the shipped `profiles.yaml`. The new shape is active.

Rollback: revert this change set. The v1 `add-reward-profiles` framework remains intact; only the gameplay-vs-profile split + the 4 new components + the engine plumbing revert. Existing checkpoints still load and evaluate either way.

## Open Questions

These will likely surface during implementation; each has an obvious default:

- **Q: Should `stall_penalty` cap at some maximum?** → Default: no cap. Unbounded growth keeps the gradient meaningful. Operators who want a cap can wrap the component or set `apply_to_loser=false` to fully disable on losses. If empirical concede rate spikes, revisit.
- **Q: Should the `breeding_digivolve` component accept a continuous formula in addition to the dict?** → Default: no — dict is sufficient for the 4 discrete levels.
- **Q: Should `digivolve_driven_attack` track per-attack (current plan) or per-attacker (would let operators count "first attack vs subsequent attacks" differently)?** → Default: per-attack. Per-attacker semantics can be added later as a `dedupe_per_episode` flag if requested.
- **Q: Should the engine counter `n_digivolve_driven_attacks` increment on blocked attacks?** → Default: no — the spec says "connects with security," which a blocked attack doesn't. Integration tests will lock this in.
- **Q: When `digivolve_driven_attack.per_card` is requested by a profile but the v1 implementation doesn't support it, should the loader warn or silently ignore?** → Default: warn at load time; ignore the parameter at runtime. v2 makes it functional.
