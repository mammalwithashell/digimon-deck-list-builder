"""Typed `Occurrence` records derived from engine state + events.

The `RewardEventBus` (event_bus.py) translates `RustHeadlessGame`'s
per-step output (`get_rl_state` counter deltas + drained `GameEvent`s)
into a flat list of these typed occurrences. Components consume this
stream — they SHALL NOT read engine state directly.

Spec: `openspec/changes/add-reward-profiles/specs/reward-profiles/spec.md`
"Reward components are composable units with a uniform interface".

Design rationale:

- Occurrences amortize the engine-state-to-event translation once,
  rather than every component re-deriving deltas independently.
- The bus enriches occurrences with registry-looked-up data (e.g.,
  `Digivolved.result_level`, `Digivolved.result_traits`) so components
  stay pure — they only do numeric comparison + string matching, never
  card-database lookups.
- A typed sum (rather than a `dict[str, Any]`) catches typos at import
  time and lets components pattern-match by variant type.

All fields on these dataclasses are required positional. `from_` fields
are renamed where Python keyword collisions force it.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Optional, Sequence


# -- Card-arrival occurrences ----------------------------------------------


@dataclass(frozen=True)
class PlayedCard:
    """Agent or opponent played a card from hand (or via effect) onto the
    battle area. Maps from `GameEvent::Play`.

    Carries the cost-aware fields surfaced by the
    `engine-event-emission` spec so the `play_named_card` component
    can gate on cost / alt-path.
    """

    player: int            # Python 1/2 convention
    card_id: str
    field_index: int
    cost_paid: int
    cost_printed: int
    via_alt_path: Optional[str]  # `CompiledAltPathKind::as_key()` or None


@dataclass(frozen=True)
class Digivolved:
    """A digivolve resolved into a new top card. Maps from
    `GameEvent::Digivolve`. The bus enriches with registry data:
    `result_level` and `result_traits` are looked up from `CardData`
    via `top_card_id` so components don't need registry access.

    `is_breeding`: True iff the digivolve happened in the breeding
    area (field_index == BREEDING_TARGET). The `breeding_digivolve`
    component filters on this flag to score breeding-area raises
    separately from battle-area digivolves. See
    `add-gameplay-reward-config/specs/gameplay-reward-config/spec.md`
    `breeding_digivolve component`.
    """

    player: int
    top_card_id: str
    field_index: int
    from_stack_top: str
    was_dna: bool
    was_blast_dna: bool
    memory_paid: int
    # Registry-enriched (bus fills in; components consume read-only):
    result_level: Optional[int]
    result_traits: Sequence[str]
    is_breeding: bool = False


@dataclass(frozen=True)
class DnaDigivolved:
    """Convenience marker — emitted in addition to `Digivolved` when
    `was_dna=true`. Lets the simple `dna_digivolve` component fire
    without re-checking the flag on every `Digivolved`.

    Per spec decision 5 (digivolve shaping precedent), DNA stacks on
    regular: the bus emits BOTH `Digivolved` and `DnaDigivolved` for a
    single DNA digivolve. Components see both events.
    """

    player: int


# -- Combat occurrences ----------------------------------------------------


@dataclass(frozen=True)
class Blocked:
    """The agent blocked an incoming attack. Derived from
    `GameEvent::Attack` + the engine's subsequent block-resolution
    state (the bus inspects `pending_attack.is_blocked` + `blocker`).
    """

    blocker_player: int    # the player who declared the block
    attacker_player: int


@dataclass(frozen=True)
class OppDeleted:
    """An opponent's Digimon was removed from the battle area to trash.
    Derived from `GameEvent::Trash` filtered to battle-area-origin
    events whose owner is NOT the agent.
    """

    owner_player: int      # the trash zone receiving the card (Python 1/2)
    card_id: str


@dataclass(frozen=True)
class OwnDeleted:
    """The agent's own Digimon was removed from the battle area to
    trash. Mirror of `OppDeleted`.
    """

    owner_player: int
    card_id: str


# -- State-counter-derived occurrences ------------------------------------


@dataclass(frozen=True)
class SecurityRemoved:
    """The agent removed `count` of the opponent's security cards this
    step. Derived from `rl_state["p2_security"]` delta (positive
    direction = removal — opponent security count went down).
    """

    count: int             # always >= 1; the bus omits zero-deltas


@dataclass(frozen=True)
class SecurityLost:
    """The agent lost `count` of its own security cards this step.
    Mirror of `SecurityRemoved`.
    """

    count: int


@dataclass(frozen=True)
class MemoryShifted:
    """Memory moved by `delta` toward the agent's side this step.
    Positive = agent gained memory; negative = opponent gained.
    Derived from `GameEvent::MemoryChange` aggregated per step.
    """

    delta: int


# -- Lifecycle occurrences -------------------------------------------------


@dataclass(frozen=True)
class TerminalOutcome:
    """Game terminated. The `terminal_outcome` component consumes this
    to emit the win/loss/draw scalar + fast-win bonus curve.

    `winner_id` is Python 1/2 for a win, `None` for a draw.
    `step_count` is the env step counter at termination — feeds the
    legacy `terminal_outcome.fast_win_par_steps` bonus curve.
    `turn_count` is the engine's turn counter at termination — feeds
    the new turn-based `quick_win_bonus` and `stall_penalty` components
    introduced by `add-gameplay-reward-config`. Both available;
    components pick whichever unit matches their semantics.
    """

    winner_id: Optional[int]
    step_count: int
    reason: Optional[str]  # game.terminal_outcome_reason.as_str()
    turn_count: int = 0


@dataclass(frozen=True)
class MatchOutcome:
    """A BO3 match terminated. Fires ONCE per `MatchEnv` episode
    termination — not once per inner game. The `match_outcome` component
    consumes this to emit per-match terminal reward (separate from the
    per-game `terminal_outcome` reward).

    When a profile defines a `match_outcome` component, `MatchEnv` defers
    its hardcoded MATCH_TERMINAL_WIN/LOSS/etc. constants to the
    component AND skips its `(bo3_game_terminal − inner_game_terminal)`
    per-game adjustment — the profile owns all reward magnitudes. When
    no `match_outcome` is present, `MatchEnv` keeps its legacy
    calibration (back-compat).

    Fields:
    - `winner_id`: Python 1/2 for a match win, `None` for a draw (the
      rare 1-1-1 outcome that can only occur on the per-match step-limit
      truncation path).
    - `agent_game_wins` / `opp_game_wins`: per-side game tallies. Sum
      is `total_games` (1-3, plus the forfeit path can add extras).
    - `was_sweep`: True iff agent won 2-0 (and was the match winner).
    - `was_swept`: True iff opp won 2-0 (and the agent got 0 games).
    - `agent_conceded_any`: True iff agent submitted action 93 in any
      game of the match (used for smart-concede / scared-concede gating).
    - `match_step_count`: total outer-agent steps across all games in
      the match. Used for the fast-bonus / fast-win curve.
    - `total_games`: `agent_game_wins + opp_game_wins`. Forfeit paths
      (concede-during-play-order) bump opp's score artificially; this
      field reflects the FINAL score, not the games actually played.
    """

    winner_id: Optional[int]
    agent_game_wins: int
    opp_game_wins: int
    was_sweep: bool
    was_swept: bool
    agent_conceded_any: bool
    match_step_count: int
    total_games: int


@dataclass(frozen=True)
class DigivolveDrivenAttack:
    """A Lv5+ digivolved attacker connected with the opponent's security
    stack. Derived from `n_digivolve_driven_attacks[player]` counter
    delta on `cur_rl_state`. One occurrence per qualifying attack
    (per-attack semantics — Security Attack +N revealing multiple
    cards still emits exactly one). See
    `add-gameplay-reward-config/specs/gameplay-reward-config/spec.md`
    `digivolve_driven_attack component` for the consuming component's
    contract.

    `has_sources`: True iff the attacker's permanent has at least one
    card under it (`len(card_sources) > 1`). Indicates the attacker
    came from a digivolve chain.
    `this_turn`: True iff the attacker was just digivolved this turn
    (`turn_digivolved == game.turn_count`). The mode predicate
    (`this_turn` / `has_sources` / `either` / `both`) lives in the
    component, not the bus — the bus emits raw flags and lets the
    component filter.
    """

    player: int
    attacker_level: int
    has_sources: bool
    this_turn: bool


@dataclass(frozen=True)
class StepElapsed:
    """One env step elapsed. Always emitted exactly once per step.
    Consumed by `step_penalty`.
    """

    pass
