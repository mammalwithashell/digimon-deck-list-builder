# Reward Profiles

Operator guide for composable reward shaping in pilot training.

Spec: `openspec/changes/add-reward-profiles/specs/reward-profiles/spec.md`.

---

## Quick start

The shipped `code/digimon_gym/agents/reward/profiles.yaml` defines five
profiles out of the box. Default training picks `_default` (byte-identical
to the legacy reward) — no config change is needed for legacy behavior.

To train DNA Omnimon with archetype-aware shaping:

```yaml
# training_config.yaml
generalist: true                # archetype sampling drives profile pick
# reward_profile_override left unset
```

The runner reads `info["deck1_archetype"]` per episode, looks it up in
`profiles.yaml::assignments`, and applies the matching profile. Unknown
archetypes fall back to `_default`.

To force a specific profile regardless of archetype (e.g. fixed-deck
training of a single archetype):

```yaml
reward_profile_override: dna_omnimon_combo_v1
```

---

## File layout

`profiles.yaml` has two top-level keys:

```yaml
profiles:
  <profile_name>:
    inherits: <parent_name>          # optional
    legacy_terminal_exclusivity: bool # optional, default false
    budget:                          # optional, per-profile cap/floor
      per_episode_cap: float
      per_episode_floor: float
    key_cards:                       # optional, declarative shorthand
      - cards: [card_id, ...]
        reward: float
        # ... see "key_cards declaration" below
    components:                      # the component list
      - kind: <component_kind>
        # ... kind-specific params + optional budget

assignments:
  _default: <profile_name>           # REQUIRED — fallback target
  "<archetype name>": <profile_name>
```

Profile names starting with `_` are **private** — they can be referenced
via `inherits:` but cannot be assigned to real archetypes. The `_default`
assignment key is the exception: it's allowed to target a private profile
(typically `_default: _default`).

---

## Component catalog (v1)

12 components are registered. Each has a stable `kind` string used in
YAML.

### Foundation (every profile inherits these via `_base_terminal`)

| Kind | Effect |
|---|---|
| `terminal_outcome` | Win/loss/draw scalar + fast-win bonus curve at termination. |
| `step_penalty` | Per-step penalty (`weight` per step). |

**`terminal_outcome` parameters** — `win_base`, `fast_win_bonus_max`,
`fast_win_par_steps`, `loss`, `draw`. The fast-win bonus is linear in
`max(0, par_steps - step_count) / par_steps × bonus_max`, clamped at
zero for slow wins.

Example (legacy defaults):

```yaml
- kind: terminal_outcome
  win_base: 10.0
  fast_win_bonus_max: 5.0
  fast_win_par_steps: 200
  loss: -10.0
  draw: -1.0
- kind: step_penalty
  weight: -0.001
```

### Dense — state-counter-derived

| Kind | Fires on | Notes |
|---|---|---|
| `security_remove` | step where opp security count dropped | `weight × n_removed` |
| `security_lost` | step where own security count dropped | conventionally negative |
| `digivolve` | step where the agent's `n_digivolutions` counter bumped | DNA also bumps this (stacks) |
| `dna_digivolve` | step where the agent's `n_dna_digivolutions` counter bumped | additive over `digivolve` |
| `memory_swing` | step with any memory change | aggregated net delta × weight |

### Dense — event-driven (requires engine event wiring)

| Kind | Fires on | Notes |
|---|---|---|
| `block_event` | `GameEvent::Attack` blocked by the agent | **v1: fires zero** — no `Block` event yet |
| `opp_deletion` | opponent card moved to trash | v1: counts ALL trash (incl. hand discard) |
| `own_deletion` | own card moved to trash | same caveat |

### Match-anything

| Kind | Match keys | Gating keys |
|---|---|---|
| `play_named_card` | `card_id`, `card_name`, `trait` | `cost_paid_lt`, `cost_paid_eq`, `cost_paid_gte`, `cost_paid_lt_printed`, `cost_paid_gte_printed`, `via_alt_path` |
| `digivolve_into_named_card` | `card_id`, `card_name`, `trait` | `was_dna`, `was_blast_dna`, `min_result_level` |

**Match semantics**: at least one match key MUST be set. Multiple match
keys AND together. `card_id` / `card_name` accept string OR list (any-of).
`trait` accepts string OR list (membership in `result_traits`).

**Gating semantics**: all set gating keys AND with the match. Unset keys
are ignored.

**v1 limitations**:

- `match_card_name` and `match_trait` on `play_named_card` fail-closed
  (the bus doesn't yet enrich `PlayedCard` with name/traits). Use
  `card_id` instead. Trait matching on `digivolve_into_named_card`
  works — `Digivolved.result_traits` is bus-enriched.
- `via_alt_path` keys come from `CompiledAltPathKind::as_key()`:
  `"digivolve"`, `"dna_digivolve"`, `"blast_dna_digivolve"`,
  `"digixros"`, `"burst_digivolve"`, `"app_fusion"`, `"assembly"`,
  `"activated_digivolve"`. Generic effect-initiated free plays
  (e.g., Davis & Ken) surface as `via_alt_path: null` — match on
  `cost_paid_eq: 0` instead.

---

## key_cards declaration

The `key_cards:` block is shorthand for declaring "these cards are the
win condition" with a single high-level entry. Each entry expands to
1–3 synthetic components at load time AND populates the profile's
**boss-cards set** for telemetry.

```yaml
key_cards:
  - cards: [BT17-078, AD1-025, BT13-112, BT22-015, BT1-084]
    reward: 6.0                  # required — digivolve-into reward
    diminishing_factor: 0.4      # default; fire-n value = reward × factor^(n-1)
    max_per_episode: null        # default; decay handles bounding
    hardcast_penalty: -1.5       # optional — full-cost hardcast penalty
    hardcast_max_per_episode: 1  # default
    alt_path_reward: 2.0         # optional — bonus via cost-reducing alt-path
    alt_paths: [assembly]        # required when alt_path_reward set
```

**Expansion** (defaults applied):

| Sub-field | Expands to |
|---|---|
| `reward` (required) | `digivolve_into_named_card { match.card_id=<cards>, weight=reward, budget.diminishing_returns_factor=diminishing_factor, budget.max_fires_per_episode=max_per_episode }` |
| `hardcast_penalty` | `play_named_card { match.card_id=<cards>, cost_paid_gte_printed=true, weight=hardcast_penalty, budget.max_fires_per_episode=hardcast_max_per_episode }` |
| `alt_path_reward` + `alt_paths` | `play_named_card { match.card_id=<cards>, via_alt_path=<alt_paths>, weight=alt_path_reward, budget = <same as reward> }` |

**Default decay schedule** (`reward: 6.0, diminishing_factor: 0.4`):

| Fire # | Contribution | Cumulative |
|---|---|---|
| 1 | +6.0 | 6.0 |
| 2 | +2.4 | 8.4 |
| 3 | +0.96 | 9.36 |
| ∞ | — | **+10.0** (asymptote = `reward / (1 - factor)`) |

The asymptotic cap (10.0) equals a base win — so the agent can never
aggregate more shaped reward from "always assemble the combo" than
from "actually win the game". Operators who want a smaller signal
drop `reward`; operators who want a more aggressive signal raise
`reward` AND must also raise the profile's `per_episode_cap` to leave
room.

**Boss-cards set**: union of all `key_cards[].cards` lists. Consumed
by the arrival-aware sidecar columns + window-mean TB scalars (see
[Telemetry](#telemetry)).

**Per-match semantics under BO3** (`match_format: bo3`): decay applies
across the entire match. Getting the key card in game 1 reduces the
reward for getting it in games 2 and 3 of the same match. Per-game
fresh decay requires `match_format: single`.

---

## Inheritance + override

Profiles support single-parent inheritance via `inherits: <parent>`.
Cycles fail at load with a clear error.

**Component override**: when a child declares a component with the same
`(kind, key-params)` tuple as a parent component, the child's wholly
replaces the parent's. Multiple instances of the same kind with
different key-params coexist:

```yaml
parent:
  components:
    - kind: security_remove
      weight: 1.5
child:
  inherits: parent
  components:
    - kind: security_remove
      weight: 3.0       # final weight is 3.0, NOT 4.5
```

**Key-params per kind** (from `registry.py::KIND_KEY_PARAMETERS`):

- `play_named_card`: `match`, `cost_paid_*`, `via_alt_path` (distinct
  instances per gating combination coexist).
- `digivolve_into_named_card`: `match`, `was_dna`, `was_blast_dna`,
  `min_result_level`.
- Everything else: empty set (at most one per profile).

**key_cards inheritance**: when a child declares its own `key_cards:`
block, the parent's `key_cards:` is replaced **wholesale** (not merged).
This is the same intent as component override but at the
key_cards-block level — operators who want to extend the parent's boss
list should copy-and-modify.

---

## Budget engine

Two layers of budget clamping, each opt-in per-component / per-profile.

### Per-component budget

```yaml
- kind: block_event
  weight: 0.15
  budget:
    max_fires_per_episode: 4          # hard cap on fire count
    max_total_per_episode: 0.5        # hard cap on cumulative magnitude
    diminishing_returns_factor: 0.6   # nth fire = weight × factor^(n-1)
```

Resolution order per fire:

1. Compute base emission `weight × event_multiplier` (component dependent).
2. Apply `diminishing_returns_factor` based on prior fire count.
3. Apply `max_total_per_episode` clamp (sign-respecting).
4. Apply `max_fires_per_episode` gate (zero emission past the cap).
5. Apply per-profile cap/floor (next section).

Zero-raw fires don't consume budget — a step where the component had
no event to react to doesn't count as one of the budgeted fires.

### Per-profile budget

```yaml
budget:
  per_episode_cap: 12.0      # max cumulative POSITIVE shaping reward
  per_episode_floor: -3.0    # max cumulative NEGATIVE shaping reward
```

Bounds the **sum** of all shaping components in an episode.
`terminal_outcome` and `step_penalty` are **exempt** — they're foundation,
not shaping. Clamped amounts surface in `info["reward_breakdown_clamped"]`
and roll up to the `pilot/profile/<p>/clamp_share` TB scalar so
operators can detect over-tight budgets.

### `once_per_episode` shorthand (deprecated)

```yaml
- kind: play_named_card
  ...
  once_per_episode: true     # equivalent to budget.max_fires_per_episode: 1
```

Emits `DeprecationWarning` at load. Use `budget.max_fires_per_episode: 1`
in new YAML.

---

## Hot reload

`reward_profiles_hot_reload: true` (default) makes the loader stat the
file at every `env.reset()` and re-parse when mtime advances.

- The active profile for an episode is **locked at `reset()`** — mid-episode
  edits never apply mid-game.
- Reload failures preserve the previously-loaded profiles + log a warning.
  A subsequent fix re-loads cleanly at the next reset.
- The per-game `reward_profile_hash` recorded in the eval-game-log sidecar
  reflects the hash **for that game** — runs that reload mid-eval show
  different hashes across rows.
- The run-start `metadata.json::reward_profiles_hash` is the run-start
  value, not the latest reload hash. Resume checks compare against the
  run-start value.

Disable for reproducibility-first workflows:

```yaml
reward_profiles_hot_reload: false
```

---

## Resume hash check

At run-start, the runner writes `<run_dir>/reward_profiles.meta.json`
with 4 fields: `reward_profiles_path`, `reward_profiles_hash`,
`reward_profile_override`, `reward_assignments_snapshot`.

On resume, the recorded hash is compared against the current
canonicalized profile hash. Mismatches fail with both hashes named:

```
Reward profiles changed since checkpoint.
  Checkpoint hash: sha256:abc...
  Current hash:    sha256:def...
Pass --reward-profiles-override-mismatch to proceed anyway.
```

The canonical hash is over parsed-and-sorted YAML with normalized
floats — **whitespace, key reordering, and comments do NOT trigger
mismatch**. Re-saving with a YAML formatter is a no-op.

Use `--reward-profiles-override-mismatch` only when you intentionally
want to switch reward shape mid-run. The override silently writes a
fresh sidecar reflecting the new hash; future resumes reference that.

---

## Legacy compatibility

### Byte-identical `_default`

The shipped `_default` profile matches legacy `DigimonEnv._compute_reward`
float-for-float. A regression test (`test_default_profile_byte_identical.py`)
runs 10 seeded episodes through both paths and asserts equality.

The match relies on a profile-level flag `legacy_terminal_exclusivity:
true` which suppresses all non-`terminal_outcome` components on the
terminal step — replicating the legacy short-circuit. **Set this flag
to false (the default) on custom profiles** unless you specifically
want to replicate the legacy quirk.

### `digivolve_shaping: true` → `_digivolve_shaped`

Setting `TrainingConfig.digivolve_shaping=True` (no explicit override)
maps to `reward_profile_override = "_digivolve_shaped"` — the shipped
profile that recreates legacy digivolve shaping (`+0.1` regular, `+3.9`
DNA additive). No deprecation warning fires for `digivolve_shaping`
itself in v1.

### Deprecated flat fields

| Field | Default | Behavior in v1 | Removal |
|---|---|---|---|
| `digivolve_shaping` | `False` | Maps to `_digivolve_shaped` profile when True; no warning | v2 |
| `digivolve_reward` | `0.1` | Unread; `DeprecationWarning` if non-default | v2 |
| `dna_digivolve_bonus` | `3.9` | Unread; `DeprecationWarning` if non-default | v2 |

To customize digivolve reward weights, define a custom profile that
inherits `_digivolve_shaped` and overrides the `digivolve` /
`dna_digivolve` components, then set
`reward_profile_override: <your_profile>`.

---

## Telemetry

### TensorBoard scalars

**Tier 1 — per-component global** (one per component kind seen):

```
pilot/reward/<component>/mean_per_game     e.g., pilot/reward/security_remove/mean_per_game
```

**Tier 2 — per-profile aggregate** (one per profile seen):

```
pilot/profile/<profile_name>/mean_reward
pilot/profile/<profile_name>/share_<component>      (fraction of profile reward from this component)
pilot/profile/<profile_name>/clamp_share            (fraction of steps where per-profile budget clamped)
```

**Tier 3 — window-mean boss-arrival** (always emitted; zero when no
boss-cards declared):

```
pilot/mean_eval_digivolves_into_boss_per_game
pilot/mean_eval_hardcasts_of_boss_per_game
pilot/mean_eval_digivolve_discipline      (digivolves / (digivolves + hardcasts))
```

### `evals.jsonl` sidecar columns

Per-opponent-archetype + per-agent-archetype slice (when archetype info
present). `evals.jsonl[N].by_archetype[X]` gains:

```json
{
  "wins": ..., "draws": ..., "games": ..., "win_rate": ...,
  "digivolves": ..., "dna_digivolves": ...,
  "opponent_digivolves": ..., "opponent_dna_digivolves": ...,
  "component_means": {
    "security_remove": 1.2,
    "step_penalty": -0.04,
    ...
  },
  "digivolves_into_boss": 12,
  "digivolves_into_boss_dna": 8,
  "hardcasts_of_boss": 2,
  "hardcasts_of_boss_full_cost": 1
}
```

`by_agent_archetype` mirrors this shape, keyed on the AGENT's
archetype (generalist mode only).

Top-level window means:

```json
{
  "mean_eval_digivolves_into_boss_per_game": 0.5,
  "mean_eval_hardcasts_of_boss_per_game": 0.1,
  "mean_eval_digivolve_discipline": 0.833      // null when no boss interaction
}
```

---

## Worked examples

### DNA Omnimon — `dna_omnimon_combo_v1`

```yaml
dna_omnimon_combo_v1:
  inherits: _default
  budget:
    per_episode_cap: 15.0          # 10 from key_cards + headroom for climb + block
    per_episode_floor: -3.0
  key_cards:
    - cards: [BT17-078, AD1-025, BT13-112, BT22-015, BT1-084]
      reward: 6.0                  # ~60% of a win; asymptotic cap 10.0
      hardcast_penalty: -1.5       # full-cost hardcast = wasted memory
      alt_path_reward: 2.0
      alt_paths: [assembly]        # AD1-025 via Partition re-play
  components:
    - kind: digivolve_into_named_card           # material climb
      match: { card_id: [BT17-015, BT17-027] }  # WG, MG
      min_result_level: 6
      weight: 0.5
      budget: { max_fires_per_episode: 2, diminishing_returns_factor: 0.7 }
    - kind: play_named_card                     # tamer/enabler
      match: { card_id: [BT17-081, BT22-017] }
      weight: 0.3
      budget: { max_fires_per_episode: 2 }
    - kind: block_event                         # generic disruption
      weight: 0.15
      budget: { max_fires_per_episode: 4, diminishing_returns_factor: 0.6 }
```

### BG Imperialdramon — `bg_imperialdramon_combo_v1`

```yaml
bg_imperialdramon_combo_v1:
  inherits: _default
  budget:
    per_episode_cap: 12.0
    per_episode_floor: -2.0
  key_cards:
    - cards: [BT12-028]            # Paildramon
      reward: 6.0
      hardcast_penalty: -1.2
      # no alt-path: Paildramon has no Partition-style replay
  components:
    - kind: digivolve_into_named_card           # Lv4 material climb
      match: { card_id: [BT12-022, BT12-050] }  # ExVeemon, Stingmon
      min_result_level: 4
      weight: 0.4
      budget: { max_fires_per_episode: 2, diminishing_returns_factor: 0.7 }
    - kind: play_named_card                     # Davis & Ken
      match: { card_id: [BT16-085] }
      weight: 0.3
      budget: { max_fires_per_episode: 1 }
```

---

## Authoring a new archetype profile

1. **Identify the win-condition cards.** Card IDs from `data/cards.json`
   are the simplest matcher.
2. **Inherit from `_default`** so terminal + step + security signals
   come for free.
3. **Add `key_cards:`** if the archetype has a clearly identifiable
   win-condition card. This single block usually does 80% of the work.
4. **Add supporting components** for material climb (small reward,
   diminishing fast) and enablers (small flat reward).
5. **Set a profile-level `budget:` cap** ≈ asymptotic key-card cap +
   ~50% headroom. Floor ≈ ½ cap, negative.
6. **Add the assignment** to the `assignments:` map using the EXACT
   archetype name from `data/deck_library.json`.
7. **Verify**: load the file via `ProfileLoader`, check the resolved
   component list + boss-cards set. The pytest in
   `test_profile_loader.py` exercises the loader end-to-end.

---

## Troubleshooting

| Symptom | Likely cause | Fix |
|---|---|---|
| `_default` profile reward differs from legacy | `legacy_terminal_exclusivity` not inherited | Set explicitly on your profile, OR inherit from `_default` |
| `play_named_card` with `match.card_name` doesn't fire | Bus enrichment for card names is v2 work | Use `match.card_id` instead |
| `block_event` always fires zero | Engine has no `GameEvent::Block` yet | Wait for v2 engine wiring |
| `opp_deletion` overcounts | v1 doesn't filter Trash by source zone | Use only when archetype-relevant; tighter signal in v2 |
| Hot reload reverts on save | Parse failure — check warning logs | Fix YAML syntax; next reset re-loads |
| Resume fails with hash mismatch | YAML changed since checkpoint | `--reward-profiles-override-mismatch` if intentional |
| `pilot/profile/<p>/clamp_share` high | Budget too tight | Raise `per_episode_cap` / `per_episode_floor` |

---

## See also

- `openspec/changes/add-reward-profiles/` — full design + spec + tasks
- `docs/TRAINING_RUNBOOK.md` — selecting reward profiles in a run
- `code/digimon_gym/agents/reward/` — implementation + registry +
  budget engine
- `code/digimon_gym/agents/reward/profiles.yaml` — shipped profiles
