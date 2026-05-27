## Context

The current reward function (`DigimonEnv._compute_reward`, `code/digimon_gym/digimon_gym.py:389–483`) returns a single float per step from a fixed formula:

- Terminal: `+10.0` win base + up to `+5.0` fast-win bonus, `−10.0` loss, `−1.0` draw.
- Dense (event-based, fires on security count change): `+1.5` per opp security removed, `−0.5` per own security lost.
- Optional asymmetric digivolve shaping (gated by `digivolve_shaping=False` default): `+0.1` per regular digivolve, `+3.9` additional per DNA digivolve.
- Step penalty: `−0.001`.

The shaping knobs are flat fields on `TrainingConfig` (`digivolve_reward`, `dna_digivolve_bonus`). Adding a new dense signal today means editing both `_compute_reward` and `TrainingConfig`, then re-running. There is no per-archetype variation, no telemetry on individual reward components, and no way for two side-by-side training runs to use different reward shapes from configuration.

The codebase already has the right plumbing for the new design:

- `info["deck1_archetype"]` is set by `GeneralistDeckPoolWrapper.reset()` (`code/digimon_gym/agents/gauntlet.py:820`) — the natural lookup key for agent-archetype-keyed profiles.
- `WinRateCallback` already maintains per-archetype tally dicts (`_archetype_*`, `_agent_archetype_*`, `_matchup_*`) and emits per-archetype TB scalars. Per-component telemetry can ride the same rails.
- The Rust engine exposes `Game::drain_events()` (`code/digimon-engine/src/game.rs:1537`) and `get_rl_state()` (`code/digimon-engine-py/src/lib.rs:682`); the env already consumes `get_rl_state` for terminal detection and digivolve-counter deltas.
- `code/digimon-engine/src/events.rs` defines `GameEvent::Attack`, `Trash`, `Mill`, and `SecurityReveal` variants but explicitly notes they are not yet emitted. The reward-profiles change provides the forcing function to wire them.

The closest existing precedent is `docs/superpowers/specs/2026-05-23-digivolve-reward-shaping-design.md` (the asymmetric agent-only digivolve bonus) and `openspec/specs/per-archetype-digivolve-telemetry/spec.md` (the per-archetype callback aggregation pattern). This change generalizes both.

## Goals / Non-Goals

**Goals:**

- Composable reward components, each a small unit with a uniform interface, so adding a new dense signal is "write a class, register it, reference it in YAML."
- Named profiles in version-controlled YAML, with optional inheritance so common terminal/step components don't need to be re-declared in every profile.
- Agent-archetype-keyed assignment via `info["deck1_archetype"]`, with a required `_default` fallback for runs where archetype is absent (gauntlet, fixed-deck smoke tests).
- Per-component, per-profile, and per-(agent_archetype × component) telemetry. TB gets the first two; the third lands in the `evals.jsonl` sidecar.
- Run reproducibility: the resolved profile name and a content hash of the profiles file live in `models/<run>/metadata.json`. Resuming with a different profiles file fails loudly unless overridden.
- Byte-identical behavior for existing runs: the shipped `_default` profile matches today's recipe exactly so `pytest`'s reward-sensitive tests and existing trained-model evaluations don't shift.
- Wire `GameEvent::Attack`, `GameEvent::Trash`, and `GameEvent::SecurityReveal` emission in the Rust engine — unlocks block / deletion / security-reveal components and is independently useful for replay/UI consumers.

**Non-Goals:**

- `ModifierApplied` engine events and `apply_status_*` components (cannot_attack / no_blocking / no_digivolve). The modifier-event surface has its own design question ("what counts as one application?") that deserves a dedicated proposal.
- Opponent-archetype-keyed profiles. The reward function should depend on the agent's win condition, not the opponent's identity (which is observable from the board).
- Matchup (N×N) profiles. Adds combinatorial profile maintenance with little learning benefit.
- Cross-profile reward normalization. Different profiles will have different reward magnitudes; we treat per-profile reward as diagnostic only and rely on profile-independent signals (win_rate, mean_episode_length) for cross-run comparison.
- TB-tier per-(agent_archetype × component) scalars. Would multiply scalar count by `archetypes × components` and overwhelm the TB UI. The sidecar carries this drilldown instead.
- Removing the deprecated flat `digivolve_reward` / `dna_digivolve_bonus` fields. v1 keeps them as no-op with a `DeprecationWarning`; v2 removes them.

## Decisions

### D1: Component model is "occurrences in, float out" — not "engine state in"

Each `RewardComponent` exposes:

```python
class RewardComponent(Protocol):
    name: str
    def compute(self,
                occurrences: list[Occurrence],
                episode_state: MutableMapping[str, Any]) -> float: ...
```

`Occurrence` is a typed sum (`PlayedCard`, `DnaDigivolved`, `Blocked`, `OppDeleted`, `OwnDeleted`, `SecurityRemoved`, `SecurityLost`, `MemoryShifted`, `Digivolved`, `TerminalOutcome`, `StepElapsed`) carrying the minimum fields each component needs (player id, card id, delta, terminal reason, step count at termination).

**Why occurrences and not raw engine state**: occurrences decouple components from engine-internal representations. A component that rewards "Omnimon played" doesn't need to know how `Game::events` is structured, doesn't need PyO3 wrapper types, and is trivially testable in pure Python without the engine running. The `RewardEventBus` is the single place that knows how to translate `drain_events()` + `get_rl_state()` deltas into occurrences; components are pure.

**Alternative considered: pass raw `info_after` and `info_before` dicts.** Rejected because every component would re-derive deltas independently (security_remove and security_lost would both subtract counters), making test fixtures and component composition harder. Occurrences amortize the engine-state-to-event work once.

**Alternative considered: components subscribe to engine `GameEvent` variants directly.** Rejected because some signals (security count change, digivolve counter, terminal outcome) come from `get_rl_state` counters, not events. A single uniform stream is simpler than a dual API.

### D2: Single YAML file, two top-level keys (`profiles` and `assignments`)

```yaml
# code/digimon_gym/agents/reward/profiles.yaml
profiles:
  _base_terminal:
    components:
      - kind: terminal_outcome
        win_base: 10.0
        fast_win_bonus_max: 5.0
        fast_win_par_steps: 200
        loss: -10.0
        draw: -1.0
      - kind: step_penalty
        weight: -0.001

  _default:
    inherits: _base_terminal
    components:
      - kind: security_remove
        weight: 1.5
      - kind: security_lost
        weight: -0.5

  dna_omnimon_combo_v1:
    inherits: _default
    components:
      - kind: dna_digivolve
        weight: 5.0
      - kind: play_named_card
        match: { card_name: "Omnimon" }
        weight: 3.0
        once_per_episode: true

  rocks_aggro_v1:
    inherits: _default
    components:
      - kind: security_remove
        weight: 3.0           # override parent's 1.5
      - kind: step_penalty
        weight: -0.003        # encourage shorter games

assignments:
  _default: _default
  "DNA Omnimon": dna_omnimon_combo_v1
  "Rocks": rocks_aggro_v1
```

**Why single file**: profile authoring is a coordinated act (you usually touch the assignment when adding a profile). One file means one git diff and trivial schema validation. Per-profile files would be tidier on paper but would scatter the assignment map.

**Why two keys, not nested**: keeps the assignment map ungrep-confusable from component definitions. An archetype name appearing as a profile key is obviously wrong; one appearing as an assignment key is correct.

**Profile names beginning with `_` are private** — usable as `inherits:` targets but cannot appear in `assignments`. Enforced at load time.

### D3: Inheritance semantics — replace, not sum

When child and parent both declare a component with the same `kind` (and, for parameterized kinds like `play_named_card`, the same key parameters), the child's declaration **replaces** the parent's wholesale.

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

Multiple instances of the same `kind` with different key parameters coexist:

```yaml
combo:
  inherits: _default
  components:
    - kind: play_named_card
      match: { card_name: "Omnimon" }
      weight: 3.0
    - kind: play_named_card
      match: { card_name: "Imperialdramon" }
      weight: 2.0
```

**Why replace not sum**: sum is unintuitive when the parent is shared across many children (you can't override a value, only add to it). Replace matches how class inheritance overrides methods.

**Key parameters** for de-duplication are documented per-component: `play_named_card` keys on (`card_name`, `card_id`, `trait`); `terminal_outcome` keys on nothing (only one allowed per profile); most others key on nothing (single instance per profile).

### D4: Wrapper position — `DigimonEnv -> RewardProfileWrapper -> OpponentWrapper -> ...`

`RewardProfileWrapper` wraps `DigimonEnv` directly, before `OpponentWrapper`. Reasons:

- The wrapper needs to read `info["deck1_archetype"]` to resolve the profile. `GeneralistDeckPoolWrapper` (which sets that field) wraps outer; on first `reset()` from outside-in, the deck-pool wrapper sets the field, then inner wrappers see it via the returned info. Wrapping closest to env means the profile choice is fixed before `OpponentWrapper` runs opponent decisions.
- `OpponentWrapper` already discards opponent dense rewards (CLAUDE.md rule 7) — the agent-only nature of components is preserved naturally without extra plumbing.
- The wrapper replaces the env's scalar reward; downstream wrappers see only the post-profile reward, matching what SB3 logs.

**Episode-state lifetime**: `episode_state` is a dict created in `reset()` and threaded through every `compute()` call until the next `reset()`. Holds per-episode memory like "Omnimon already counted once" or "frames since last block." Components that don't need it ignore the parameter.

**BO3 interaction**: in `match_format=bo3`, one Gym episode = one match (3 games). `episode_state` persists across the 3 games (matching CLAUDE.md rule 26's LSTM-state carryover). Components like `play_named_card(once_per_episode=true)` therefore fire once per **match**, not per game. This is intentional — episodic identity is what `once_per_episode` should track.

### D5: Telemetry tiers

| Tier | What | Where | Cardinality |
|------|------|-------|------------|
| Per-component global | `pilot/reward/<component>/mean_per_game` | TensorBoard | `O(components)` ≈ 11 |
| Per-profile aggregate | `pilot/profile/<profile>/mean_reward` and `pilot/profile/<profile>/share_<component>` | TensorBoard | `O(profiles × components)` ≈ 50 |
| Per-archetype × component | `evals.jsonl` row's `by_agent_archetype[X].component_means[<component>]` | sidecar JSONL | `O(archetypes × components)` ≈ 200 |

The TB tier deliberately caps at `O(profiles × components)`. The per-archetype drilldown lives in the sidecar where it can be queried by tooling (`digimon-training-mcp`) without bloating the TB UI.

The `WinRateCallback` accumulates `_component_totals: dict[str, float]`, `_profile_component_totals: dict[(profile, component), float]`, and `_agent_archetype_component_totals: dict[(archetype, component), float]`, all reset to zero at `__init__` and incremented in the same post-game hook that already updates `_archetype_wins`. They follow the existing precedent (no cross-resume persistence; fresh tallies on `WinRateCallback` reconstruction).

### D6: Reproducibility — content hash, not file path

`metadata.json` records:

```json
{
  "reward_profiles_path": "code/digimon_gym/agents/reward/profiles.yaml",
  "reward_profiles_hash": "sha256:<hex>",
  "reward_profile_override": null,
  "reward_assignments_snapshot": { "_default": "_default", "DNA Omnimon": "dna_omnimon_combo_v1", ... }
}
```

The hash is computed over the parsed-and-canonicalized profile data (sorted keys, normalized floats) so whitespace and key-order changes don't trigger false mismatches. On resume, if the loaded profiles' hash differs from the checkpoint's hash, training fails with:

```
Reward profiles changed since checkpoint.
  Checkpoint hash: sha256:abc...
  Current hash:    sha256:def...
Pass --reward-profiles-override-mismatch to proceed anyway.
```

The full `assignments` snapshot is captured so even if the file is later edited, the run's effective assignment table is reconstructible from `metadata.json` alone.

### D7: Default profile is byte-identical to today

The shipped `_default` profile encodes the current recipe exactly:

```yaml
_default:
  inherits: _base_terminal
  components:
    - kind: security_remove
      weight: 1.5
    - kind: security_lost
      weight: -0.5

_base_terminal:
  components:
    - kind: terminal_outcome
      win_base: 10.0
      fast_win_bonus_max: 5.0
      fast_win_par_steps: 200
      loss: -10.0
      draw: -1.0
    - kind: step_penalty
      weight: -0.001
```

A new pytest, `code/tests/rl/test_default_profile_byte_identical.py`, runs 100 seeded episodes with the legacy `_compute_reward` path and 100 with the profile path and asserts the per-step reward sequences are equal float-for-float. This is the gate that defends "existing runs are unchanged."

### D8: Engine event emission — minimal, additive, plus field extensions for arrival-aware components

Three new emissions land in `code/digimon-engine/src/game.rs`:

- `GameEvent::Attack` at the attack-declaration site (within the combat state machine, before any block resolution).
- `GameEvent::Trash` at every call site that moves a card into a player's trash zone — must cover `Game::delete_permanents_batch` (per CLAUDE.md rule 25), hand-discard paths, security-loss paths, and effect-driven trashing.
- `GameEvent::SecurityReveal` at the security-check resolution site, emitted once per revealed card.

In addition, two already-emitted variants gain payload fields needed by arrival-aware components (`digivolve_into_named_card`, cost-aware `play_named_card`):

- `GameEvent::Play` gains `cost_paid: i16`, `cost_printed: i16`, and `via_alt_path: Option<String>`. The `via_alt_path` value is the canonical alt-path key from `CompiledAltPathKind::as_key()` (one of `"digivolve"`, `"dna_digivolve"`, `"blast_dna_digivolve"`, `"digixros"`, `"burst_digivolve"`, `"app_fusion"`, `"assembly"`, `"activated_digivolve"`) when the card was played through an alt-path that bypassed the printed cost, otherwise `None`. `cost_paid` reflects the actual memory paid (after tamer cost reductions, alt-path discounts, etc.). `cost_printed` is the card's printed `play_cost` from `CardData`.
- `GameEvent::Digivolve` gains `was_dna: bool`, `was_blast_dna: bool`, and `memory_paid: i16`. `was_dna` is true for any DNA-style digivolve including end-of-turn DNA, registered alt-path DNA, and the standard `dna_costs` path. `was_blast_dna` is the narrower flag for Blast DNA (alt-path with `CompiledAltPathKind::BlastDnaDigivolve`). The result card identity is already carried by the existing `top_card_id` field; `result_traits` and `result_level` are **not** added to the engine event payload — the `RewardEventBus` looks them up via the card registry and enriches the `Digivolved` occurrence before passing it to components. This keeps the engine event surface narrow, avoids duplicating `CardData` into events, and preserves component purity (components never call into the registry themselves).

Each emission uses `Game::next_event_seq()` for the monotonic seq, matching the existing emission discipline. The field additions are non-breaking for downstream consumers because `GameEvent` serializes via `#[serde(tag = "type")]` and consumers ignore unknown fields by convention (verified in `code/digimon-engine-mcp/` and replay viewer). Five new integration tests under `code/digimon-engine/tests/event_emission/` exercise each variant via `DebugRunner` and assert the event shape (including the extended fields).

**Rust no-approximations policy applies** (CLAUDE.md rule 17): emissions must not be conditional on whether the env consumes them. Replay/UI consumers benefit independently — the `via_alt_path` field is particularly valuable for replay forensics because it surfaces "this Omnimon entered via DNA, not hardcast" in human-readable form.

### D10: Arrival-aware components distinguish digivolve-into from hardcast

A central observation from the DNA Omnimon exploration: the same target card (e.g., BT17-078 Omnimon, AD1-025 Omnimon) is a **good play** when arrived at via digivolve / DNA digivolve / xros_req alt-path, and a **bad play** when arrived at via full-cost hardcast from hand. Rewarding `play_named_card { card_name: "Omnimon" }` without arrival awareness conflates these two events.

Two components carry the load:

**`digivolve_into_named_card`** — fires when the agent digivolves into a matching card. Match keys mirror `play_named_card`: `card_id`, `card_name`, `trait`. Adds three optional gating keys specific to digivolve semantics:

- `was_dna: true | false | null` — restrict to (only DNA) / (only non-DNA) / (any) digivolves. `null` (default) matches both.
- `was_blast_dna: true | false | null` — same shape, narrower flag.
- `min_result_level: u8 | null` — restrict to digivolves whose result level is at least N. Useful for "reward climbing to Lv6+" without naming specific cards.

```yaml
- kind: digivolve_into_named_card
  match: { card_id: ["BT17-078", "AD1-025"] }   # both DNA-Omnimon variants
  was_dna: true                                  # only via DNA path
  weight: 4.0
  budget: { max_fires_per_episode: 1 }
```

**`play_named_card`** (extended) — gains optional cost-paid matchers:

- `cost_paid_lt: int | null` — fires only when `cost_paid < N`.
- `cost_paid_gte: int | null` — fires only when `cost_paid >= N`.
- `cost_paid_eq: int | null` — fires only when `cost_paid == N`.
- `cost_paid_lt_printed: bool` — convenience for "anything below printed cost" (true when `cost_paid < cost_printed`).
- `cost_paid_gte_printed: bool` — convenience for "at full cost or higher" (true when `cost_paid >= cost_printed`).
- `via_alt_path: string | list[string] | null` — fires only when the `via_alt_path` event field matches one of the listed alt-path keys.

```yaml
# Penalize hardcast Omnimon at full cost
- kind: play_named_card
  match: { card_id: ["BT17-078", "AD1-025"] }
  cost_paid_gte_printed: true
  weight: -1.5
  budget: { max_fires_per_episode: 1 }

# Reward AD1-025 played via the "assembly" alt-path (free re-play through Partition)
- kind: play_named_card
  match: { card_id: ["AD1-025"] }
  via_alt_path: ["assembly"]
  cost_paid_eq: 0
  weight: 2.0
```

**Why both components, not a unified one**: a digivolve produces an event with both `top_card_id` (the result) and `from_stack_top` (the material), while a hardcast Play event produces just `card_id` and the `via_alt_path` indicator. Semantically distinct enough that splitting components keeps each one's matcher fields tight and unambiguous.

**Boss-card derivation for telemetry**: at profile load, the boss-card set is computed as `{ every card matched by any digivolve_into_named_card entry whose effective weight ≥ BOSS_REWARD_THRESHOLD }`. The threshold defaults to `2.0` and is overridable per profile via top-level `boss_reward_threshold: float`. This drives the new sidecar columns (`digivolves_into_boss_agent`, `hardcasts_of_boss_full_cost_agent`, etc.) without forcing the operator to declare boss cards twice.

### D11: Per-component budget controls

Today's `once_per_episode: bool` flag is a single point on a spectrum. The combined design replaces it with a structured `budget:` sub-key per component, of which `once_per_episode` becomes shorthand:

```yaml
- kind: block_event
  weight: 0.15
  budget:
    max_fires_per_episode: 4          # hard cap on count
    max_total_per_episode: 0.5        # hard cap on magnitude (sign-respecting)
    diminishing_returns_factor: 0.6   # nth fire = weight × factor^(n-1)
```

Resolution rules:

- Both `max_fires_per_episode` and `max_total_per_episode` may be set; the effective limit is whichever binds first.
- `diminishing_returns_factor` applies multiplicatively per fire BEFORE budget caps. A component with `weight: 1.0, factor: 0.5` emits `1.0, 0.5, 0.25, …`.
- For **negative** weights, `max_total_per_episode` is interpreted as a floor: a `weight: -1.5, max_total_per_episode: -1.5` component fires once and then stops.
- `once_per_episode: true` is a deprecated alias for `max_fires_per_episode: 1` accepted by the YAML loader (warning emitted at load), removed in v2.

Budget state lives in `episode_state` (one entry per component instance, keyed by the canonical component key tuple). BO3 episodes carry the budget across all three games (matching D4 — one match = one episode).

**Why on each component**: per-component budget is the most direct expression of "reward DNA Omnimon arrival once, but allow climbing-material rewards to fire twice." Tying budget to component avoids cross-component coupling that a profile-only cap would impose.

### D12: Per-profile budget cap and floor

Per-component budgets bound individual signals; a per-profile budget bounds their sum. Both are useful — per-profile insures against unforeseen interactions where individually-bounded components compound into a profile that dwarfs terminal magnitude.

```yaml
dna_omnimon_combo_v1:
  inherits: _default
  budget:
    per_episode_cap: 12.0           # sum of POSITIVE shaped reward ≤ 12.0
    per_episode_floor: -3.0         # sum of NEGATIVE shaped reward ≥ -3.0
  components:
    ...
```

Resolution: every component's per-step contribution is summed into a running `profile_positive_total` and `profile_negative_total`. If a contribution would push the relevant total past its cap/floor, the contribution is clamped to the remaining budget (possibly to zero), and the clamped amount is recorded in `info["reward_breakdown_clamped"]` for telemetry.

The cap/floor apply ONLY to shaped components from this profile. The terminal_outcome and step_penalty components are NOT counted against the cap — they are part of the underlying recipe, not shaping. (The byte-identical `_default` requirement still holds because `_default` has no per-profile budget set.)

**Why both `_cap` and `_floor`**: hardcast penalty (negative) and DNA reward (positive) live in different sign halves; one shared budget would force a tradeoff between "how much can shape pull?" and "how much can shape push?" that has no natural answer.

### D13: Hot-reload semantics — mtime-check at reset

Operator iteration on profile YAML benefits from not having to restart training to test a new weight. The loader stat()s the YAML file at every `env.reset()` and re-parses when mtime advances:

```python
class ProfileLoader:
    def reload_if_changed(self) -> bool:
        st = os.stat(self.path)
        if st.st_mtime_ns > self._last_mtime_ns:
            self._reload()
            self._last_mtime_ns = st.st_mtime_ns
            return True
        return False
```

The active profile for an episode is locked at `RewardProfileWrapper.reset()` time and immutable for the rest of that episode — mid-episode YAML edits never apply mid-game. This preserves the within-episode reward shape's integrity.

**Sidecar repro under hot-reload**: the per-game `evals.jsonl` row (introduced by `add-per-game-eval-log`) records the profile hash actually used for that game in a new field `reward_profile_hash`. This makes runs reproducible even when the file changed mid-run — the sidecar is the ground truth for "what shape was applied per game."

**Run-metadata hash semantics under hot-reload**: `models/<run>/metadata.json::reward_profiles_hash` records the hash AT RUN-START. The hash-mismatch resume check (D6) compares against the run-start hash, not the latest hash. If hot-reload changed the file during the run, resume after the run will still see a mismatch — which is the intended behavior (resume should pick up where the run left off, not start with whatever the YAML now says).

**Disable**: `TrainingConfig.reward_profiles_hot_reload: bool = True` toggles the mtime check. Off-by-default would be safer for reproducibility-first workflows; on-by-default is friendlier for the user's stated iteration use case. We default to ON and provide the flag for off-when-needed.

### D14: Shipped archetype profiles bake in the DNA-deck design

Two non-private archetype profiles ship in `profiles.yaml`. They serve as living examples of the schema (in particular the `key_cards:` declaration from §D15) and as the seed shapes for the DNA archetypes the user is training today.

```yaml
dna_omnimon_combo_v1:
  inherits: _default
  budget:
    per_episode_cap: 15.0           # accommodates 10.0 key-card cap + headroom
    per_episode_floor: -3.0
  key_cards:
    # Omnimon family — the win condition
    - cards: [BT17-078, AD1-025, BT13-112, BT22-015, BT1-084]
      reward: 6.0                   # first arrival ≈ 60% of a win
      # diminishing_factor: 0.4     # default; asymptotic cap = 10.0
      # max_per_episode: null       # default; decay handles bounding
      hardcast_penalty: -1.5
      # hardcast_max_per_episode: 1 # default
      alt_path_reward: 2.0          # AD1-025 via Partition / Assembly
      alt_paths: [assembly]
  components:
    # Climb the Greymon/Garurumon ladder (NOT a key card — small reward, diminishing fast)
    - kind: digivolve_into_named_card
      match: { card_id: [BT17-015, BT17-027] }    # WG, MG
      min_result_level: 6
      weight: 0.5
      budget: { max_fires_per_episode: 2, diminishing_returns_factor: 0.7 }

    # Tamer / enabler plays
    - kind: play_named_card
      match: { card_id: [BT17-081, BT22-017] }    # Tai&Matt, Gabumon BT22
      weight: 0.3
      budget: { max_fires_per_episode: 2 }

    # Generic disruption signal
    - kind: block_event
      weight: 0.15
      budget: { max_fires_per_episode: 4, diminishing_returns_factor: 0.6 }

bg_imperialdramon_combo_v1:
  inherits: _default
  budget:
    per_episode_cap: 12.0
    per_episode_floor: -2.0
  key_cards:
    - cards: [BT12-XXX]             # Paildramon ID — confirm against data/cards.json
      reward: 6.0
      hardcast_penalty: -1.2
      # No alt_path_reward — BG Imperialdramon has no Partition equivalent on Paildramon
  components:
    # Climb to Lv4 materials (Stingmon / ExVeemon — IDs confirmed in task 5.3)
    - kind: digivolve_into_named_card
      match: { card_id: [BT12-031, BT12-032] }
      min_result_level: 4
      weight: 0.4
      budget: { max_fires_per_episode: 2, diminishing_returns_factor: 0.7 }

    # Tamer play (Davis & Ken)
    - kind: play_named_card
      match: { card_id: [BT16-085] }
      weight: 0.3
      budget: { max_fires_per_episode: 1 }

    # Reward free Lv3 plays via Davis & Ken's Start-of-Main alt-path
    - kind: play_named_card
      match: { card_name: [Veemon, Wormmon] }
      cost_paid_eq: 0
      weight: 0.4
      budget: { max_fires_per_episode: 2 }
```

**Why ship these and not just the schema**: the user is training DNA archetypes today. Shipping concrete profiles means the v1 land is immediately useful; subsequent profile authoring becomes copy-and-modify rather than design-from-scratch. Compared to the pre-D15 hand-written form, each profile loses ~12 lines of `digivolve_into_named_card` / `play_named_card` / hardcast triplets in favor of a single `key_cards:` entry. The exact card IDs for BG Imperialdramon Stingmon/ExVeemon/Paildramon are TBD in this proposal — task 5.3 confirms them against `data/cards.json` before shipping.

### D15: Key-cards declaration

A profile MAY include a top-level `key_cards:` list that declaratively expresses "these cards are the win condition; reward getting them, penalize wasting them." Each entry expands at load time into 1–3 concrete components:

```yaml
key_cards:
  - cards: [BT17-078, AD1-025, BT13-112, BT22-015, BT1-084]
    reward: 6.0
    diminishing_factor: 0.4         # optional; default 0.4
    max_per_episode: null           # optional; default null (decay handles bounding)
    hardcast_penalty: -1.5          # optional; omit for "no penalty on hardcast"
    hardcast_max_per_episode: 1     # optional; default 1
    alt_path_reward: 2.0            # optional; requires alt_paths to also be set
    alt_paths: [assembly]
```

**Expansion rules.** At profile-load time, each `key_cards:` entry expands into:

| Sub-field set | Expanded component |
|---|---|
| `reward` (required) | `digivolve_into_named_card { match: {card_id: <cards>}, weight: <reward>, budget: {max_fires_per_episode: <max_per_episode>, diminishing_returns_factor: <diminishing_factor>} }` |
| `hardcast_penalty` | `play_named_card { match: {card_id: <cards>}, cost_paid_gte_printed: true, weight: <hardcast_penalty>, budget: {max_fires_per_episode: <hardcast_max_per_episode>} }` |
| `alt_path_reward` + `alt_paths` | `play_named_card { match: {card_id: <cards>}, via_alt_path: <alt_paths>, weight: <alt_path_reward>, budget: {max_fires_per_episode: <max_per_episode>, diminishing_returns_factor: <diminishing_factor>} }` |

Expanded components participate in inheritance and override the same way hand-written ones do. A child profile can suppress a parent's `key_cards:` entry by redeclaring `key_cards:` (replace, matching D3 semantics).

**Defaults and rationale.**

| Default | Value | Rationale |
|---|---|---|
| `diminishing_factor` for `reward` and `alt_path_reward` | `0.4` | Fire-1 = full reward, fire-2 = 40%, fire-3 = 16% → asymptotic cap `reward / 0.6`. Big initial, sharp taper. |
| `max_per_episode` for `reward` and `alt_path_reward` | `null` (unlimited) | Decay bounds the total naturally; a hard cap would interact awkwardly with decay. |
| `max_per_episode` for `hardcast_penalty` | `1` | Decaying a penalty teaches "second mistake is cheaper" — wrong lesson. Penalties are hard-capped. |
| `diminishing_factor` for `hardcast_penalty` | implicit `1.0` (no decay) | Same rationale as above. Not exposed as a knob; if the operator wants it decayed, they should use a hand-written `play_named_card` instead. |

**Sizing `reward` against terminal.** A first-time key-card arrival of `reward: 6.0` is approximately 60% of a base win (+10). The asymptotic cap (`6.0 / (1 - 0.4) = 10.0`) deliberately equals a base win — so the agent can never aggregate more shaped reward from "always assemble the combo" than from "actually win the game." Operators who want a smaller signal can drop `reward` (cap scales linearly); operators who want a more aggressive signal can raise `reward` AND must also raise the profile's `per_episode_cap` to leave the new cap unclamped.

**Per-match decay semantics.** Diminishing returns apply across the entire Gym episode. Under `match_format: bo3` (one episode = one BO3 match), this means getting the key card in game 1 reduces the reward for getting it in game 2 and game 3 of the same match. The operator's stated intent: "the first time I assemble Omnimon in a match is the most valuable; subsequent reassemblies still register, just smaller." If per-game decay is wanted instead, the operator should switch to `match_format: single` or hand-write a `digivolve_into_named_card` component with bespoke budget — `key_cards:` is opinionated on per-episode decay.

**Boss-cards set derivation.** The profile's boss-cards set (consumed by the arrival-aware sidecar columns from D14 — `digivolves_into_boss_agent`, `hardcasts_of_boss_full_cost_agent`, etc.) is the **union of every `key_cards:` entry's `cards` list**. The pre-D15 threshold-based derivation (`BOSS_REWARD_THRESHOLD` = 2.0 over `digivolve_into_named_card` weights) is **dropped**. If a profile declares no `key_cards:`, its boss-cards set is empty and the boss-arrival columns emit zero for that profile's games.

**Why a high-level declaration rather than hand-written components.** Two reasons:

1. **Single source of truth for "what is the win condition?"** Declaring `key_cards:` once expresses operator intent unambiguously — the cards listed are *both* what to reward AND what defines boss-card telemetry. Hand-written components could express the same shape (reward + hardcast penalty + alt-path bonus) but would scatter the intent across 3 entries plus a separate boss-cards declaration, with risk of drift.
2. **Operator iteration speed.** Tuning a key card's signal is the most common edit during training experiments. A one-line `reward: 6.0 → 8.0` edit is easier to reason about than three separate component weights that must be kept in proportion.

**Alternative considered: a `key_card` *component kind* instead of a profile-level block.** Rejected because (a) the expansion produces *multiple* components from one declaration, which doesn't fit the component model cleanly; (b) the boss-cards set is profile-level state, not component-level; (c) the declaration is naturally read as "this profile's win condition is X," not as "this profile includes a component that..."

### D9: Deprecation path for `digivolve_reward` / `dna_digivolve_bonus`

The TrainingConfig fields stay in v1 with these mechanics:

- If a user sets either field to a non-default value, `TrainingConfig._validate` emits a `DeprecationWarning` directing them to define a custom profile.
- The fields are no longer read by the env — the active profile drives all shaping.
- `digivolve_shaping=True` with default reward values silently maps to a built-in `_digivolve_shaped` profile (which the shipped YAML defines) and is treated as `reward_profile_override="_digivolve_shaped"`. Existing scripted runs that set this flag keep working.
- v2 (separate proposal) removes the fields.

## Risks / Trade-offs

- **[Risk] Profile drift across archetypes makes `pilot/mean_eval_reward` incomparable across runs.** → Mitigation: the spec explicitly designates per-profile reward as diagnostic only; cross-run comparison uses `pilot/win_rate` and `pilot/mean_episode_length`, which are profile-independent. Documented in `docs/REWARD_PROFILES.md` and called out in the TB scalar's tooltip via a `# diagnostic` suffix in the scalar name.
- **[Risk] Wiring three new events in the Rust engine triggers `GameEvent` consumers downstream (UI animation, replay viewer) to render previously-silent events.** → Mitigation: emission lands in a single PR; UI/replay consumers that pattern-match on event type already use a default-skip branch (verified in `code/digimon-engine-mcp/` and replay viewer). The three new variants opt in by being explicitly listed where consumers care.
- **[Risk] Per-component telemetry cardinality grows TB log size meaningfully on long runs.** → Mitigation: scalar count is bounded (~60 total). Empirical TB log growth is dominated by per-step metrics already logged (`pilot/explained_variance` etc.) — adding 60 per-eval scalars is <1% incremental.
- **[Risk] BO3 `episode_state` semantics surprise users — `once_per_episode` fires once per match, not per game.** → Mitigation: documented in `docs/REWARD_PROFILES.md` with an explicit BO3 callout. The `match_format=single` legacy path naturally degrades to per-game and matches the obvious-reading.
- **[Risk] Hash-mismatch on resume blocks legitimate edits (typo fix, comment change) and creates friction.** → Mitigation: hash is over canonicalized parsed data, not raw bytes — whitespace, key order, and trailing comments don't trigger mismatch. Override flag is documented.
- **[Risk] Component implementations couple to engine event field names (e.g., `Trash.card_id`); a future engine rename breaks every component.** → Mitigation: occurrences are a translation layer; the `RewardEventBus` is the only file that imports engine event types, and it's intentionally small. Engine event renames touch one file.
- **[Trade-off] `inherits` is single-parent, not multiple.** Profile authors who want "aggro + combo" hybrid must duplicate components. Multiple inheritance would add merge-order ambiguity; single inheritance keeps the merge deterministic. Revisit if archetype matrix demands it.
- **[Risk] Hot-reload silently changes shape mid-run, breaking the operator's mental model of "I'm training with profile X."** → Mitigation: per-game `reward_profile_hash` in the sidecar (D13) makes the actual shape per game queryable post-hoc. Run-metadata records the run-start hash for resume integrity. `reward_profiles_hot_reload: bool` config toggle gives operators an off-switch for reproducibility-first workflows.
- **[Risk] Per-profile `per_episode_cap` clamping creates a non-linear reward surface where the marginal value of a component depends on what has already fired this episode.** → Mitigation: clamping is recorded in `info["reward_breakdown_clamped"]` and rolled up into a TB scalar `pilot/profile/<p>/clamp_share`. If clamping fires >5% of steps in eval, the cap is too tight and should be raised. Documented in `docs/REWARD_PROFILES.md`.
- **[Risk] Negative shaping (hardcast penalty) makes the agent learn to never hold hardcastable boss cards — pitching them via discard effects rather than holding them.** → Mitigation: cap negative-shape per episode with `per_episode_floor: -1.5` to bound the worst case. Eval telemetry shows `hardcasts_of_boss_full_cost_agent` so the user can see whether the penalty is firing too often (suggesting magnitude too high) or never (suggesting magnitude too low).
- **[Risk] Boss-card derivation via `BOSS_REWARD_THRESHOLD` could miss intent if a profile uses lower weights for sound reasons (e.g., once_per_episode big cards with `weight: 1.5`).** → Mitigation: threshold is overridable per profile (`boss_reward_threshold: 1.0`). If neither default nor explicit threshold matches an archetype's natural boss tier, profile authors can pin it explicitly via a top-level `boss_cards:` list that supersedes derivation.
- **[Risk] Event field extensions (cost_paid on Play, was_dna on Digivolve) require touching every emission site — risk of an emitter forgetting to populate the new fields.** → Mitigation: Rust struct field additions become compile errors at every emit site if the field has no default. Use non-Option non-defaulted fields (`cost_paid: i16`, not `Option<i16>`) so the compiler forces the operator through every emitter. `via_alt_path: Option<String>` stays Option because it's genuinely absent in the common case.

## Migration Plan

This is a new capability with a byte-identical default — no migration required for existing runs or checkpoints.

For users who want to adopt profiles:

1. Run training with no config changes — `_default` profile is selected, reward is identical to pre-change behavior.
2. Optionally set `reward_profile_override: dna_omnimon_combo_v1` (or another shipped profile) in the run YAML to use a non-default shape for a fixed-deck run.
3. For generalist runs, edit `profiles.yaml` `assignments` map to bind archetype names to profile names.
4. New profiles are added by editing `profiles.yaml`; no code changes needed unless a new component kind is introduced (which requires a Python class + registry entry).

Rollback: revert the change set. Existing checkpoints continue to work either way (no checkpoint format change).

## Open Questions

None blocking. Items that may surface during implementation and have an obvious default:

- **Q: When a profile references a component kind that doesn't exist (typo), should loading fail fast or fall back to default?** → Default: fail fast at load with a clear error listing valid kinds. Silent fallback would mask configuration bugs.
- **Q: Should the `_digivolve_shaped` legacy-compat profile be in the shipped YAML or derived in Python from the deprecated flags?** → Default: shipped in YAML, so the YAML stays the single source of truth and the deprecation path is "switch from flag to profile name" not "lose access to the recipe."
- **Q: Where in the wrapper chain does `DeckPoolWrapper` (own-deck variants) sit relative to `RewardProfileWrapper`?** → Default: `DeckPoolWrapper` wraps outer (it varies the deck; the profile reads the resulting archetype from info). Verified compatible because both `GeneralistDeckPoolWrapper` and `DeckPoolWrapper` set `info["deck1_archetype"]` on `reset()`.
