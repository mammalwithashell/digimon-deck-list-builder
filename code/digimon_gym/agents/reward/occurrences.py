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
    `terminal_outcome` component's fast-win bonus.
    """

    winner_id: Optional[int]
    step_count: int
    reason: Optional[str]  # game.terminal_outcome_reason.as_str()


@dataclass(frozen=True)
class StepElapsed:
    """One env step elapsed. Always emitted exactly once per step.
    Consumed by `step_penalty`.
    """

    pass
