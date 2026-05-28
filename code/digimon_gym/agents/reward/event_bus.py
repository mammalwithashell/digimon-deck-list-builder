"""`RewardEventBus` — translates `RustHeadlessGame` per-step output into
the typed `Occurrence` stream that components consume.

Spec: `openspec/changes/add-reward-profiles/specs/reward-profiles/spec.md`
"Reward components are composable units with a uniform interface" —
"all signals SHALL be derived from the `occurrences` stream and from
`episode_state` ... Components SHALL NOT read engine state directly."

The bus is the single place that knows how to read engine state and
events. It runs inside `DigimonEnv.step` and writes the resulting
occurrence list to `info["reward_occurrences"]`, where
`RewardProfileWrapper` reads it.

Inputs per step:

- `prev_rl_state`: the `get_rl_state()` dict snapshotted just BEFORE
  the engine step ran. The bus tracks this internally.
- `cur_rl_state`: the `get_rl_state()` dict after the step.
- `events`: list of event dicts drained via
  `get_events_since_last_step()`. Each dict shape matches the PyO3
  binding's `event_to_pydict` serialization (top-level `type`, `seq`,
  `player`, `source_card_id`, `source_slot`, `target_card_id`,
  `target_slot`, `meta`).

Outputs: `list[Occurrence]` (typed dataclass instances).

Card-data enrichment: `Digivolved` occurrences need `result_level` and
`result_traits` for `digivolve_into_named_card` component matching.
The bus accepts an optional `card_data_lookup` callable that maps
`card_id -> (level, traits)` — typically a thin adapter over
`digimon_engine.CardDatabase`. When unset, enrichment returns
`(None, [])` and matchers gracefully fail-closed.

Known v1 limitations (documented in module + tracked as follow-ups):

- `Blocked` occurrences are never emitted: the engine has no
  `GameEvent::Block` variant. `BlockEventComponent` will fire zero
  in v1.
- `OppDeleted`/`OwnDeleted` emit on ALL `Trash` events for the
  respective player, NOT just battle-area-origin events. Hand
  discards and security trashes are counted as "deletions" — an
  overcount relative to the spec's "battle-area only" requirement.
  Profile authors who need tighter filtering should wait for the
  engine to expose `source_zone` on the Trash event variant.
"""

from __future__ import annotations

from typing import Any, Callable, List, Mapping, Optional, Sequence, Tuple

from .occurrences import (
    Blocked,
    Digivolved,
    DigivolveDrivenAttack,
    DnaDigivolved,
    MemoryShifted,
    OppDeleted,
    OwnDeleted,
    PlayedCard,
    SecurityLost,
    SecurityRemoved,
    StepElapsed,
    TerminalOutcome,
)


# Lookup type: card_id -> (level, traits). None when card unknown.
CardDataLookup = Callable[[str], Optional[Tuple[Optional[int], Sequence[str]]]]


# Inspector type for `digivolve_driven_attack` enrichment. Maps
# (player, field_index) -> dict with keys `level: int`, `has_sources:
# bool`, `this_turn: bool`. Returns None when the permanent is gone
# (e.g. deleted during security resolution). The bus falls back to
# conservative defaults (level=0, both flags False) in that case.
PermanentInspector = Callable[[int, int], Optional[Mapping[str, Any]]]


def _null_lookup(_card_id: str) -> Optional[Tuple[Optional[int], Sequence[str]]]:
    """Default no-op lookup: returns None for every card. Components
    that depend on enrichment (e.g., `digivolve_into_named_card` with
    `min_result_level` or `match_trait`) will fail-closed.
    """
    return None


def _null_inspector(_player: int, _field_index: int) -> Optional[Mapping[str, Any]]:
    """Default no-op permanent inspector. Returns None — the bus then
    falls back to conservative (level=0, has_sources=False,
    this_turn=False) defaults when emitting `DigivolveDrivenAttack`.
    """
    return None


class RewardEventBus:
    """Stateful translator from engine state+events to Occurrences.

    `step_count` is the env-level step counter (passed by the caller —
    typically `DigimonEnv._step_count`). The bus uses it to populate
    `TerminalOutcome.step_count` for the fast-win bonus.

    `breeding_target` is the field index that marks the breeding
    area (`digimon_engine.BREEDING_TARGET`, value 14). Used to set
    `Digivolved.is_breeding`. Defaults to `None` — if `None`, the
    bus attempts to read the constant from `digimon_engine` at
    construction, falling back to the hard-coded 14 when the import
    fails (tests without the binding installed).
    """

    def __init__(
        self,
        card_data_lookup: Optional[CardDataLookup] = None,
        *,
        breeding_target: Optional[int] = None,
        permanent_inspector: Optional[PermanentInspector] = None,
    ) -> None:
        self._card_data_lookup = card_data_lookup or _null_lookup
        self._permanent_inspector = permanent_inspector or _null_inspector
        if breeding_target is None:
            try:
                from digimon_engine import BREEDING_TARGET as _bt  # type: ignore
                breeding_target = int(_bt)
            except Exception:
                breeding_target = 14  # canonical value from action::space
        self._breeding_target = int(breeding_target)
        # Snapshot of rl_state at last derive() call (the "previous step").
        # None until the first derive — first step has nothing to delta against.
        self._prev_rl_state: Optional[Mapping[str, Any]] = None

    def reset(self) -> None:
        """Clear inter-step state. Called by `RewardProfileWrapper.reset`
        so a fresh episode doesn't inherit deltas from a prior one.
        """
        self._prev_rl_state = None

    def derive(
        self,
        cur_rl_state: Mapping[str, Any],
        events: Sequence[Mapping[str, Any]],
        step_count: int,
    ) -> List[Any]:
        """Build the per-step occurrence list from engine output.

        Always emits exactly one `StepElapsed` per call. Subsequent
        derivation order is canonical (terminal, security delta,
        memory, plays, digivolves, deletions) so component tests see
        a deterministic stream when constructing fixtures.

        After this call, `cur_rl_state` becomes the new `prev_rl_state`
        for the next derive.
        """
        occurrences: List[Any] = []

        # Step marker — exactly one per env.step(). Components like
        # `step_penalty` rely on this firing per step regardless of
        # what events occurred.
        occurrences.append(StepElapsed())

        # Terminal outcome — driven by current state, not events. The
        # `GameOver` event variant exists in the engine but its
        # PyO3-side payload doesn't carry winner_id consistently;
        # rl_state is the authoritative source for terminal detection
        # in the rest of DigimonEnv.
        if cur_rl_state.get("game_over"):
            winner = cur_rl_state.get("winner_id")
            reason = cur_rl_state.get("terminal_reason")
            turn_count = int(cur_rl_state.get("turn_count", 0))
            occurrences.append(
                TerminalOutcome(
                    winner_id=int(winner) if winner is not None else None,
                    step_count=int(step_count),
                    reason=str(reason) if reason is not None else None,
                    turn_count=turn_count,
                )
            )

        # Security delta (asymmetric — agent-side perspective):
        # opp removal = drop in p2_security count from prev → cur.
        # own loss   = drop in p1_security count.
        if self._prev_rl_state is not None:
            prev_p1 = int(self._prev_rl_state.get("p1_security", 0))
            prev_p2 = int(self._prev_rl_state.get("p2_security", 0))
            cur_p1 = int(cur_rl_state.get("p1_security", 0))
            cur_p2 = int(cur_rl_state.get("p2_security", 0))

            opp_removed = prev_p2 - cur_p2
            if opp_removed > 0:
                occurrences.append(SecurityRemoved(count=int(opp_removed)))
            own_lost = prev_p1 - cur_p1
            if own_lost > 0:
                occurrences.append(SecurityLost(count=int(own_lost)))

            # `DigivolveDrivenAttack` derivation (per-attack counter delta).
            # Engine-side counter increments once per qualifying attack
            # (Lv5+ attacker, target = security, attack actually
            # reaches security). We emit one occurrence per delta unit.
            # The attacker permanent is identified by walking the step's
            # Attack events with target_player set + target_field_index
            # is None (i.e., security-target attacks). When an inspector
            # is wired, we enrich with `attacker_level` + flags;
            # otherwise conservative defaults (level=0, both flags
            # False) keep components from over-firing.
            prev_driven = self._prev_rl_state.get("n_digivolve_driven_attacks") or [0, 0]
            cur_driven = cur_rl_state.get("n_digivolve_driven_attacks") or [0, 0]
            for player_idx in range(2):
                try:
                    delta = int(cur_driven[player_idx]) - int(prev_driven[player_idx])
                except (TypeError, IndexError, ValueError):
                    delta = 0
                if delta <= 0:
                    continue
                # Find the security-target attack(s) attributed to this
                # player in the current step. The engine emits one
                # Attack event per attack declaration; for Player-target
                # attacks, target_field_index is None.
                player_attacks = [
                    e for e in events
                    if e.get("type") == "Attack"
                    and int(e.get("player", -1)) == player_idx
                    and e.get("target_field_index") is None
                    and e.get("target_player") is not None
                ]
                # Emit `delta` occurrences. Pair each with the next
                # unused attack event when available; fall back to
                # defaults for any extras (engine counter and event
                # stream can drift in rare multi-attack edge cases).
                for i in range(delta):
                    attack_ev = player_attacks[i] if i < len(player_attacks) else None
                    info: Mapping[str, Any] = {}
                    if attack_ev is not None:
                        attacker_field = attack_ev.get("attacker_field_index")
                        if attacker_field is not None:
                            inspected = self._permanent_inspector(
                                player_idx, int(attacker_field)
                            )
                            if inspected is not None:
                                info = inspected
                    # Per spec: `player` is Python 1/2 convention.
                    py_player = player_idx + 1
                    occurrences.append(
                        DigivolveDrivenAttack(
                            player=py_player,
                            attacker_level=int(info.get("level", 0) or 0),
                            has_sources=bool(info.get("has_sources", False)),
                            this_turn=bool(info.get("this_turn", False)),
                        )
                    )

        # Event-driven occurrences. Per-event order preserved from the
        # engine's drained seq order so consumers can correlate (e.g.,
        # Attack-then-Trash heuristics in future profile authoring).
        memory_delta_total = 0
        for ev in events:
            ev_type = ev.get("type")

            if ev_type == "MemoryChange":
                # Aggregate into a single MemoryShifted per step rather
                # than one per event — components want the net delta, not
                # the per-event history. The aggregation also makes
                # `memory_swing` less noisy on multi-effect turns.
                delta = int(ev.get("meta", {}).get("delta", 0))
                memory_delta_total += delta
                continue

            if ev_type == "Play":
                meta = ev.get("meta", {}) or {}
                occurrences.append(
                    PlayedCard(
                        player=int(ev.get("player", 0)),
                        card_id=str(ev.get("source_card_id") or ""),
                        field_index=int(ev.get("source_slot") or 0),
                        cost_paid=int(meta.get("cost_paid", 0)),
                        cost_printed=int(meta.get("cost_printed", 0)),
                        via_alt_path=meta.get("via_alt_path"),
                    )
                )
                continue

            if ev_type == "Digivolve":
                meta = ev.get("meta", {}) or {}
                top_card_id = str(ev.get("source_card_id") or "")
                lookup_result = self._card_data_lookup(top_card_id)
                if lookup_result is not None:
                    result_level, result_traits = lookup_result
                    traits_list = list(result_traits)
                else:
                    result_level, traits_list = None, []
                was_dna = bool(meta.get("was_dna", False))
                field_index = int(ev.get("source_slot") or 0)
                digivolved = Digivolved(
                    player=int(ev.get("player", 0)),
                    top_card_id=top_card_id,
                    field_index=field_index,
                    from_stack_top=str(meta.get("from_stack_top", "")),
                    was_dna=was_dna,
                    was_blast_dna=bool(meta.get("was_blast_dna", False)),
                    memory_paid=int(meta.get("memory_paid", 0)),
                    result_level=result_level,
                    result_traits=traits_list,
                    is_breeding=(field_index == self._breeding_target),
                )
                occurrences.append(digivolved)
                # Spec decision 5 (digivolve shaping precedent): emit a
                # DnaDigivolved marker alongside any DNA event so the
                # simple `dna_digivolve` component fires without
                # re-checking the flag on every Digivolved.
                if was_dna:
                    occurrences.append(
                        DnaDigivolved(player=int(ev.get("player", 0)))
                    )
                continue

            if ev_type == "Trash":
                # v1 limitation: no zone filtering. Treats every Trash
                # as a "deletion" from the owner's perspective. Profile
                # authors who need battle-area-only signal should wait
                # for the engine to add `source_zone` on the Trash
                # variant.
                owner = int(ev.get("player", 0))
                card_id = str(ev.get("source_card_id") or "")
                if owner == 1:
                    occurrences.append(OwnDeleted(owner_player=1, card_id=card_id))
                elif owner == 2:
                    occurrences.append(OppDeleted(owner_player=2, card_id=card_id))
                continue

            # Other event types (TurnStart, PhaseChange, SecurityReveal,
            # GameOver, Concede, EffectFizzled, Attack, Mill) don't
            # currently drive a Reward occurrence. Attack emission
            # would feed a future Blocked-derivation pass once the
            # engine adds GameEvent::Block.

        if memory_delta_total != 0:
            occurrences.append(MemoryShifted(delta=int(memory_delta_total)))

        # Advance prev_rl_state for next derive.
        self._prev_rl_state = dict(cur_rl_state)

        return occurrences
