# Reward Profiles

Operator guide for composable reward shaping in pilot training.

Specs:
- `openspec/changes/add-reward-profiles/specs/reward-profiles/spec.md` — base framework
- `openspec/changes/add-gameplay-reward-config/specs/gameplay-reward-config/spec.md` — two-file split + new components

---

## Two-file architecture

Reward shaping lives in two sibling YAMLs under `code/digimon_gym/agents/reward/`:

| File | Owns | Profile count |
|---|---|---|
| `gameplay.yaml` | Universal game-mechanic shape — terminal, quick-win, stall, step, security, digivolve, breeding, digivolve-driven attack | Exactly one profile, named `gameplay`. The inheritance root — it MUST NOT itself `inherits:` anything. |
| `profiles.yaml` | Archetype overlays — `_default`, `dna_omnimon_combo_v1`, etc. | Many. Every profile here MUST `inherits: gameplay` (directly or transitively). |

`ProfileLoader` takes both paths (`gameplay_path=`, `profiles_path=`), loads
each file, then merges into a single name-keyed namespace. Name collisions
across the two files fail at load. The two files have **separate content
hashes** (`gameplay_hash`, `profiles_hash`) so the resume check can name
which file drifted.

Splitting gameplay shaping out of `profiles.yaml` keeps archetype overlays
small (most just `inherits: gameplay` and add a `key_cards:` block) and
lets the universal shape change without rewriting every archetype profile.

---

## Quick start

Default training uses the `_default` profile — a thin pass-through to
`gameplay`. The shipped gameplay shape has "win fast or it hurts"
personality: a `quick_win_bonus` peaks at +5 on turn 3 and decays to 0 by
turn 7; a `stall_penalty` starts at turn 8 and grows quadratically without
bound. See the file header in `gameplay.yaml` for the full terminal
landscape table.

To train DNA Omnimon with archetype-aware shaping:

```yaml
# training_config.yaml
generalist: true                # archetype sampling drives profile pick
# reward_profile_override left unset
```

The runner reads `info["deck1_archetype"]` per episode, looks it up in
`profiles.yaml::assignments`, and applies the matching profile. Unknown
archetypes fall back to `_default` (which is just `gameplay`).

To force a specific profile regardless of archetype (e.g. fixed-deck
training of a single archetype):

```yaml
reward_profile_override: dna_omnimon_combo_v1
```

---

## Migration note for resumes of pre-change checkpoints

The two-file split changes both the gameplay-hash and the profiles-hash
relative to any checkpoint trained before `add-gameplay-reward-config`.
Resuming such a checkpoint fails the resume hash check (twice — once per
file). Pass `--reward-profiles-override-mismatch` to proceed; a fresh
sidecar with both new hashes is then written. See [Resume hash check](#resume-hash-check).

The legacy `_digivolve_shaped` and `_base_terminal` profiles are gone —
their behavior is absorbed into the universal `gameplay` shape. The
`legacy_terminal_exclusivity` flag is removed; the loader now errors with
a migration message if YAML still sets it.

---

## Concede behavior

`stall_penalty` applies to losses by default — the longer a losing game
drags out, the larger the penalty. This makes conceding a hopeless game
attractive to the agent. We ship with this on and observe via the
existing `pilot/concede_rate` TB scalar. If concede rate spikes above
~50%, set `stall_penalty.apply_to_loser: false` on the relevant profile
(typically by overriding the component in an archetype profile that
inherits `gameplay`); draws are always penalized regardless of the apply
flags.

---

## File layout

Both `gameplay.yaml` and `profiles.yaml` share the same schema:

```yaml
profiles:
  <profile_name>:
    inherits: <parent_name>          # REQUIRED in profiles.yaml; FORBIDDEN
                                     # on the `gameplay` profile (it is the root)
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
```

`profiles.yaml` additionally has an `assignments:` top-level key:

```yaml
assignments:
  _default: <profile_name>           # REQUIRED — fallback target
  "<archetype name>": <profile_name>
```

`gameplay.yaml` does NOT carry `assignments:` — assignment is overlay-level
concern.

Profile names starting with `_` are **private** — they can be referenced
via `inherits:` but cannot be assigned to real archetypes. The `_default`
assignment key is the exception: it's allowed to target a private profile
(typically `_default: _default`).

---

## Component catalog

Each component has a stable `kind` string used in YAML. The 10 components
listed in [Foundation](#foundation-shipped-in-gameplayyaml) ship in
`gameplay.yaml` as the universal baseline; the remaining components are
opt-in per-profile (typically in `profiles.yaml` overlays).

### Foundation (shipped in `gameplay.yaml`)

| Kind | Effect |
|---|---|
| `terminal_outcome` | Win/loss/draw scalar + legacy fast-win bonus curve at termination. The shipped `gameplay` profile sets `fast_win_bonus_max=0` and delegates fast-win shaping to `quick_win_bonus`. |
| `quick_win_bonus` | Turn-based fast-win bonus on agent wins only — piecewise-linear peak/decay shape. |
| `stall_penalty` | Quadratic penalty for terminal at high turn counts. Fires on win, loss, AND draw by default. |
| `step_penalty` | Per-step penalty (`weight` per step). |
| `security_remove` | Step where opp security count dropped (`weight × n_removed`). |
| `security_lost` | Step where own security count dropped (`weight × n_lost`). |
| `digivolve` | Step where the agent's `n_digivolutions` counter bumped. |
| `dna_digivolve` | Step where the agent's `n_dna_digivolutions` counter bumped (additive over `digivolve`). |
| `breeding_digivolve` | Per-level reward for breeding-area digivolves (agent only). |
| `digivolve_driven_attack` | Reward for Lv5+ attacks that connect with security after a recent digivolve or with card_sources. |

**`terminal_outcome` parameters** — `win_base`, `fast_win_bonus_max`,
`fast_win_par_steps`, `loss`, `draw`. The legacy fast-win bonus is linear
in `max(0, par_steps - step_count) / par_steps × bonus_max`. In `gameplay`
the bonus is disabled (max=0) and `quick_win_bonus` takes over.

**`quick_win_bonus` parameters** — `peak_turn: int` (default 3),
`peak_value: float` (default 5.0), `decay_per_turn: float` (default 1.25).
Fires only on agent wins. Formula: `max(0, peak_value − decay_per_turn ×
max(0, turn − peak_turn))`. Uses `turn_count` from the terminal occurrence,
not `step_count`.

```yaml
- kind: quick_win_bonus
  peak_turn: 3
  peak_value: 5.0
  decay_per_turn: 1.25
# Emissions at turn 3/4/5/6/7 = +5.0 / +3.75 / +2.5 / +1.25 / 0.0
# Zero on loss, draw, and turns past the linear root.
```

**`stall_penalty` parameters** — `threshold_turn: int` (default 7),
`scale: float` (default 0.1), `apply_to_winner: bool` (default true),
`apply_to_loser: bool` (default true). Formula:
`−scale × max(0, turn − threshold_turn)²`. Unbounded. Draws are always
penalized regardless of the apply flags.

```yaml
- kind: stall_penalty
  threshold_turn: 7
  scale: 0.1
  apply_to_winner: true
  apply_to_loser: true
# Emissions at turn 7/10/15/20/30 = 0 / -0.9 / -6.4 / -16.9 / -52.9
```

**`breeding_digivolve` parameters** — `reward_per_level: Mapping[int, float]`
(required). Default shipped value `{3: 0.4, 4: 0.2, 5: 0.1, 6: -0.4}`.
Fires only when `is_breeding=true` and `player==1`. Missing keys produce
zero. The Lv6 negative entry is the explicit slot-lock anti-pattern.

```yaml
- kind: breeding_digivolve
  reward_per_level:
    3: 0.4
    4: 0.2
    5: 0.1
    6: -0.4
```

**`digivolve_driven_attack` parameters** — `mode: str` (one of
`"this_turn"`, `"has_sources"`, `"either"`, `"both"`; default `"either"`),
`attacker_min_level: int` (default 5), `reward: float` (default 0.5),
`per_card: bool` (default false). Per-attack semantics (one emission per
qualifying attack, regardless of Security Attack +N revealing multiple
cards). The `per_card=true` form is deferred to v2 and emits a load-time
warning.

```yaml
- kind: digivolve_driven_attack
  mode: either
  attacker_min_level: 5
  reward: 0.5
  per_card: false
```

Foundation YAML block from `gameplay.yaml`:

```yaml
- kind: terminal_outcome
  win_base: 10.0
  fast_win_bonus_max: 0.0          # delegated to quick_win_bonus
  fast_win_par_steps: 200
  loss: -10.0
  draw: -1.0
- kind: step_penalty
  weight: -0.001
```

Full scenario coverage for each new component lives in
`openspec/changes/add-gameplay-reward-config/specs/gameplay-reward-config/spec.md`.

### Dense — additional state-counter-derived (opt-in)

| Kind | Fires on | Notes |
|---|---|---|
| `memory_swing` | step with any memory change | aggregated net delta × weight |

(The foundation-shipped state-counter components — `security_remove`,
`security_lost`, `digivolve`, `dna_digivolve` — are listed above and
inherited by every profile via `gameplay`. DNA also bumps `digivolve`, so
the two emissions stack on a DNA fire.)

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
with 6 fields:

- `reward_gameplay_path` — path to `gameplay.yaml`
- `reward_gameplay_hash` — canonical sha256 of `gameplay.yaml`
- `reward_profiles_path` — path to `profiles.yaml`
- `reward_profiles_hash` — canonical sha256 of `profiles.yaml`
- `reward_profile_override` — the explicit override, if any
- `reward_assignments_snapshot` — frozen archetype→profile assignment map

On resume, BOTH file hashes are compared against the checkpoint's
recorded values. Mismatch in either file raises
`RewardProfilesHashMismatchError`. The error message names which file
drifted (`gameplay.yaml` vs `profiles.yaml`) so operators see "gameplay
shape changed" vs "archetype overlay changed" directly:

```
Reward profiles changed since checkpoint (gameplay.yaml drifted).
  Checkpoint hash: sha256:abc...
  Current hash:    sha256:def...
Pass --reward-profiles-override-mismatch to proceed anyway.
```

The canonical hash is over parsed-and-sorted YAML with normalized
floats — **whitespace, key reordering, and comments do NOT trigger
mismatch**. Re-saving with a YAML formatter is a no-op.

`--reward-profiles-override-mismatch` covers both file hashes — operators
do not need separate flags. The override silently writes a fresh sidecar
reflecting the new hashes for both files; future resumes reference those.

The `Profiles` dataclass exposes `gameplay_hash` and `profiles_hash`
fields. A backward-compat alias `content_hash` mirrors `profiles_hash` for
callers predating the split.

---

## Legacy compatibility

The `_digivolve_shaped` and `_base_terminal` private profiles, and the
`legacy_terminal_exclusivity` flag, have all been removed as of
`add-gameplay-reward-config` — their behavior is absorbed into the
universal `gameplay` shape (digivolve at +0.5 / +3.5 additive on DNA,
no terminal-exclusivity carve-out). The loader errors with a migration
message if YAML still sets `legacy_terminal_exclusivity`.

### Deprecated flat fields

| Field | Default | Behavior | Removal |
|---|---|---|---|
| `digivolve_shaping` | `False` | INERT — accepted, no warning, no effect on profile selection | v2 |
| `digivolve_reward` | `0.1` | Unread; `DeprecationWarning` if non-default | v2 |
| `dna_digivolve_bonus` | `3.9` | Unread; `DeprecationWarning` if non-default | v2 |

To customize digivolve reward weights, define a custom profile in
`profiles.yaml` that `inherits: gameplay` and overrides the `digivolve` /
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
  inherits: gameplay
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
  inherits: gameplay
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
2. **Inherit from `gameplay`** (in `profiles.yaml`) so terminal +
   step + security + digivolve + quick-win + stall signals come for free.
   Inheriting from `_default` works identically since `_default` is just
   `gameplay`.
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
| Load fails with `legacy_terminal_exclusivity` error | YAML still sets the removed flag | Delete the flag; the behavior is gone — see [Legacy compatibility](#legacy-compatibility) |
| Load fails: profiles.yaml profile has no `inherits:` | New rule — every `profiles.yaml` profile MUST inherit from a `gameplay.yaml` profile | Add `inherits: gameplay` |
| Load fails: name collision between files | A profile name appears in both `gameplay.yaml` and `profiles.yaml` | Rename one |
| `play_named_card` with `match.card_name` doesn't fire | Bus enrichment for card names is v2 work | Use `match.card_id` instead |
| `block_event` always fires zero | Engine has no `GameEvent::Block` yet | Wait for v2 engine wiring |
| `opp_deletion` overcounts | v1 doesn't filter Trash by source zone | Use only when archetype-relevant; tighter signal in v2 |
| Hot reload reverts on save | Parse failure — check warning logs | Fix YAML syntax; next reset re-loads |
| Resume fails with hash mismatch | YAML changed since checkpoint | `--reward-profiles-override-mismatch` if intentional; error message names which file drifted |
| Concede rate spikes above ~50% | `stall_penalty` makes losing-fast attractive | Override `stall_penalty.apply_to_loser: false` in the archetype overlay |
| `pilot/profile/<p>/clamp_share` high | Budget too tight | Raise `per_episode_cap` / `per_episode_floor` |

---

## See also

- `openspec/changes/add-reward-profiles/` — base framework design + spec + tasks
- `openspec/changes/add-gameplay-reward-config/` — two-file split + new components design + spec + tasks
- `docs/TRAINING_RUNBOOK.md` §13 — selecting reward profiles in a run
- `code/digimon_gym/agents/reward/` — implementation + registry +
  budget engine
- `code/digimon_gym/agents/reward/gameplay.yaml` — universal game-mechanic shape
- `code/digimon_gym/agents/reward/profiles.yaml` — shipped archetype overlays
