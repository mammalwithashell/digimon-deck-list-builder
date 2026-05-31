"""Best-of-three match wrapper for RL pilot training.

One Gym episode = one BO3 match (up to 3 games). Sits between
`OpponentWrapper` (inside) and deck-pool wrappers (outside). Owns:

- Match state machine (game indices, score, concede history).
- Deck-pair persistence: decks sampled at match reset are held across
  all games of the match (real BO3 = same deck).
- BO3 `SelectPlayOrder` phase injected via the engine primitive
  `Game::request_play_order_selection`. The loser of the previous game
  picks first / second via action 94 / 95.
- LSTM hidden-state carry across games within a match. Achieved by
  calling `OpponentWrapper.reset_inner_only` (which resets the inner
  `DigimonEnv` without resetting `opponent_fn.reset_state`) at game
  boundaries rather than the full `OpponentWrapper.reset`.
- Per-game terminal reward adjusted to the BO3 calibration (`±12 +
  up to +3 fast`) by subtracting the inner `DigimonEnv` terminal
  (`±10 + up to +5 fast`) and adding the BO3 target.
- Match terminal reward synthesised at match end with sweep bonus,
  smart-concede bonus, scared-concede penalty, and fast-match bonus.
- Per-match hard step limit at 900 steps. On overrun, the current
  game is force-resolved and the match resolves on game count; tie
  → 1-1-1 draw with `-1` terminal.

See `openspec/changes/add-bo3-match-training/design.md` for the full
calibration rationale.
"""

from __future__ import annotations

import uuid
from typing import Any, Dict, List, Optional, Tuple

import gymnasium
import numpy as np

from digimon_gym.digimon_gym import DigimonEnv


class MatchEnv(gymnasium.Wrapper):
    """Wraps `OpponentWrapper(DigimonEnv(...))` to surface a BO3 match as
    one Gym episode.

    Constructor parameter `env` MUST be an `OpponentWrapper` (or
    equivalent that exposes `reset_inner_only` and the underlying
    `DigimonEnv` via `env.env`).
    """

    # ─── BO3 reward calibration (kept on the class for easy override) ───
    HARD_STEP_LIMIT_PER_MATCH = 900

    GAME_TERMINAL_WIN = 12.0
    GAME_TERMINAL_LOSS = -12.0
    GAME_FAST_BONUS_MAX = 3.0
    GAME_FAST_BONUS_PAR_STEPS = 150.0

    MATCH_TERMINAL_WIN = 30.0
    MATCH_TERMINAL_LOSS = -30.0
    MATCH_DRAW = -1.0
    MATCH_SWEEP_BONUS = 10.0
    MATCH_SMART_CONCEDE_BONUS = 5.0
    MATCH_SCARED_CONCEDE_PENALTY = -10.0
    MATCH_FAST_BONUS_MAX = 15.0
    MATCH_FAST_BONUS_PAR_STEPS = 450.0

    # Existing `DigimonEnv` terminal magnitudes — used to subtract its
    # contribution so we can replace with the BO3 values without
    # changing `DigimonEnv` itself ("DigimonEnv doesn't know about
    # matches", per design D11).
    INNER_GAME_TERMINAL_WIN = 10.0
    INNER_GAME_FAST_BONUS_MAX = 5.0
    INNER_GAME_FAST_BONUS_PAR_STEPS = 200.0
    INNER_GAME_TERMINAL_LOSS = -10.0
    INNER_GAME_DRAW = -1.0

    def __init__(self, env: gymnasium.Env, seed: Optional[int] = None) -> None:
        super().__init__(env)
        self._rng = np.random.default_rng(seed)
        self._inner_digimon_env: DigimonEnv = self._find_digimon_env(env)
        # Optional RewardProfileWrapper somewhere in the chain. When the
        # active profile defines a `match_outcome` component, MatchEnv
        # defers BOTH the per-game (bo3 − inner) calibration adjustment
        # AND the per-match terminal reward to the wrapper — the profile
        # owns the magnitudes end-to-end. See Path C in
        # `add-gameplay-reward-config/design.md`.
        self._reward_profile_wrapper: Optional[Any] = self._find_reward_profile_wrapper(env)
        # Decks held across games of the match.
        self._deck1: Optional[List[str]] = None
        self._deck2: Optional[List[str]] = None
        self._reset_match_state()

    def _find_digimon_env(self, env: gymnasium.Env) -> DigimonEnv:
        """Walk the wrapper chain to the inner `DigimonEnv`."""
        node: Any = env
        while node is not None:
            if isinstance(node, DigimonEnv):
                return node
            node = getattr(node, "env", None)
        raise TypeError(
            "MatchEnv requires an inner DigimonEnv at some depth in the "
            "wrapper chain; none found."
        )

    def _find_reward_profile_wrapper(self, env: gymnasium.Env) -> Optional[Any]:
        """Walk the wrapper chain looking for a `RewardProfileWrapper`.
        Returns None if absent — MatchEnv then keeps its legacy hardcoded
        calibration (back-compat).
        """
        # Local import to keep the engine-only routers safe from a circular
        # import through the reward package.
        from digimon_gym.agents.reward.wrapper import RewardProfileWrapper

        node: Any = env
        while node is not None:
            if isinstance(node, RewardProfileWrapper):
                return node
            node = getattr(node, "env", None)
        return None

    @property
    def _profile_owns_match_terminal(self) -> bool:
        """True iff the active reward profile defines a `match_outcome`
        component. Recomputed on access (cheap dict scan) so profile
        hot-reloads between matches take effect immediately.
        """
        if self._reward_profile_wrapper is None:
            return False
        return bool(self._reward_profile_wrapper.has_match_outcome_component)

    def _build_match_terminal_snapshot(self) -> Dict[str, Any]:
        """Construct the snapshot dict consumed by
        `RewardProfileWrapper.compute_match_terminal` and the
        `MatchOutcome` occurrence. Mirrors the fields the legacy
        `_calc_match_terminal` reads off `self`.
        """
        agent_wins = int(self.match_score[0])
        opp_wins = int(self.match_score[1])
        if agent_wins >= 2:
            winner_id: Optional[int] = 1
        elif opp_wins >= 2:
            winner_id = 2
        else:
            winner_id = None  # 1-1-1 draw via step-limit truncation
        return {
            "winner_id": winner_id,
            "agent_game_wins": agent_wins,
            "opp_game_wins": opp_wins,
            "was_sweep": agent_wins >= 2 and opp_wins == 0,
            "was_swept": opp_wins >= 2 and agent_wins == 0,
            "agent_conceded_any": any(self.concede_history),
            "match_step_count": int(self.match_step_count),
            "total_games": agent_wins + opp_wins,
        }

    # ─── State init ─────────────────────────────────────────────────────

    def _reset_match_state(self) -> None:
        self.match_id: Optional[str] = None
        # Match score indexed by Python pid - 1: [p1_wins, p2_wins].
        self.match_score: List[int] = [0, 0]
        # Set when a forced match end (step-limit) had no 2-win majority and
        # the match was decided by tiebreaker rather than a clean 2 wins.
        self._forced_match_tiebreak: bool = False
        self.match_step_count: int = 0
        self.match_step_count_in_current_game: int = 0
        self.current_game_index: int = 0
        # One slot per game in the match (max 3). `True` iff the agent
        # (P1) conceded that game via action 93.
        self.concede_history: List[bool] = [False, False, False]
        # When True, the next step is the play-order pick.
        self._pending_play_order_pick: bool = False
        # Python pid (1 or 2) of the chooser during the pending pick.
        self._pending_play_order_chooser: int = 1
        # Records each game's `play_order_choice` for the recording
        # artifact / info dict.
        self._play_order_history: List[Optional[Dict[str, Any]]] = [None, None, None]
        # On a game-terminating step, records the (p1_delta, p2_delta) so
        # `_info_with_match_metadata` can compute `match_score_before`
        # accurately by rolling back the just-applied delta.
        self._last_game_score_delta: List[int] = [0, 0]
        # Per-inner-game stats captured at game-termination time. Consumed
        # by `WinRateCallback._run_evaluation` to emit one per-game-eval-log
        # row per game in BO3 mode (since the callback's outer loop only
        # sees the match-level terminated=True). See
        # `openspec/changes/add-per-game-eval-log/`.
        self.match_game_history: List[Dict[str, Any]] = []

    # ─── Reset ─────────────────────────────────────────────────────────

    def reset(
        self,
        *,
        seed: Optional[int] = None,
        options: Optional[Dict[str, Any]] = None,
    ) -> Tuple[np.ndarray, Dict[str, Any]]:
        self._reset_match_state()
        self.match_id = str(uuid.uuid4())

        # Capture deck override for the whole match.
        options = dict(options) if options else {}
        explicit_deck1 = options.get("deck1")
        explicit_deck2 = options.get("deck2")

        # Coin-flip game-1 first player. Rust 0/1 convention; aligned
        # via the seed-parity trick (see `_align_seed_for_first_player`).
        first_player_rust = int(self._rng.integers(0, 2))

        game_seed = self._derive_game_seed(seed)
        game_seed = self._align_seed_for_first_player(game_seed, first_player_rust)

        obs, info = self.env.reset(seed=game_seed, options=options or None)

        # Lock in the decks used by inner DigimonEnv so we can rebuild
        # game 2/3 with the same pair. If a deck wasn't explicitly
        # provided, read what the inner env actually built with.
        self._deck1 = (
            list(explicit_deck1)
            if explicit_deck1 is not None
            else list(self._inner_digimon_env._deck1)
        )
        self._deck2 = (
            list(explicit_deck2)
            if explicit_deck2 is not None
            else list(self._inner_digimon_env._deck2)
        )

        info = self._info_with_match_metadata(info, game_terminated=False)
        return obs, info

    # ─── Step ──────────────────────────────────────────────────────────

    def step(self, action: int) -> Tuple[np.ndarray, float, bool, bool, Dict[str, Any]]:
        # If we're awaiting the play-order pick, that's the only action
        # we expect (other than concede). Route through the special path.
        if self._pending_play_order_pick:
            return self._step_during_play_order_pick(int(action))

        obs, reward, terminated, truncated, info = self.env.step(int(action))
        self.match_step_count += 1
        self.match_step_count_in_current_game += 1

        # Per-match hard step limit. If we cross it mid-game, force the
        # in-progress game to resolve via DigimonEnv's existing
        # `force_step_limit_winner` path. The terminated flag will be
        # set by that path on the next observation pull.
        if (
            not terminated
            and not truncated
            and self.match_step_count >= self.HARD_STEP_LIMIT_PER_MATCH
        ):
            force_winner = getattr(self._inner_digimon_env.runner, "force_step_limit_winner", None)
            if force_winner is not None:
                force_winner()
                obs = self._inner_digimon_env.runner.get_board_tensor(1)
                terminated = True
                info = dict(info)
                info["match_step_limit_truncated"] = True

        if not terminated:
            info = self._info_with_match_metadata(info, game_terminated=False)
            return obs, reward, False, truncated, info

        # ─── Game just ended. Decide whether the match ends too. ───
        return self._handle_game_terminal(obs, reward, truncated, info)

    def _step_during_play_order_pick(
        self, action: int
    ) -> Tuple[np.ndarray, float, bool, bool, Dict[str, Any]]:
        """Handle the agent / opponent submitting the play-order pick
        (action 94 or 95). On resolution, advance the engine to game N+1.

        Action 93 (concede) during this phase terminates the match —
        agent forfeits the remaining games. Any 94/95 routes through
        `resolve_selection`, writes the engine's `last_play_order_choice`
        slot, and triggers the inter-game transition.
        """
        # Action 93 = concede the whole match. The inner game is already
        # in `game_over` state, but conceding during the inter-game
        # phase is treated as a forfeit of all remaining games. Award the
        # opponent enough game wins to terminate the match as an agent
        # loss (real BO3 tournament rule: between-games concession is a
        # match forfeit). We award the minimum games needed so the score
        # reflects "agent stopped here," not "opp swept everything."
        if int(action) == 93:
            agent_wins = self.match_score[0]
            opp_wins = self.match_score[1]
            # Forfeit gives opp the games needed to reach 2.
            games_needed = max(0, 2 - opp_wins)
            self.match_score[1] += games_needed
            # Record the forfeit as a concede on the current game-index slot
            # so the scared-concede penalty fires when applicable (0-2 forfeit).
            if 0 <= self.current_game_index < len(self.concede_history):
                # Avoid double-counting if a real concede was already recorded.
                if not self.concede_history[self.current_game_index]:
                    self.concede_history[self.current_game_index] = True
            if self._profile_owns_match_terminal:
                snapshot = self._build_match_terminal_snapshot()
                match_reward = float(
                    self._reward_profile_wrapper.compute_match_terminal(snapshot)
                )
            else:
                match_reward = self._calc_match_terminal()
            info = self._info_with_match_metadata({}, game_terminated=True)
            info["match_terminated_via_concede_during_play_order"] = True
            obs = self._inner_digimon_env.runner.get_board_tensor(1)
            self.match_step_count += 1
            return obs, match_reward, True, False, info

        # Forward the action through the wrapper chain so OpponentWrapper
        # gets a chance to update its bookkeeping. The inner DigimonEnv
        # routes 94 / 95 through `resolve_selection`, which calls our
        # PlayOrder callback and writes `last_play_order_choice`. The
        # inner `terminated` value WILL be True (game_over carries over
        # from the prior game) — that's expected; we don't treat it as
        # a match-end signal.
        _obs, reward, _terminated, truncated, _info = self.env.step(int(action))
        self.match_step_count += 1

        # Read the chosen play order out of the engine.
        picked = self._inner_digimon_env.runner.take_play_order_choice()
        chooser_pid = self._pending_play_order_chooser
        if picked == "first":
            # The chooser plays first in the next game (Rust 0/1).
            first_player_rust = chooser_pid - 1
        elif picked == "second":
            first_player_rust = (1 - (chooser_pid - 1))
        else:
            # Defensive fallback — shouldn't happen if engine is healthy.
            first_player_rust = int(self._rng.integers(0, 2))

        # Record the pick for the recording artifact / info dict on
        # the upcoming game.
        next_game_index = self.current_game_index + 1
        if next_game_index < len(self._play_order_history):
            self._play_order_history[next_game_index] = {
                "chooser": chooser_pid,
                "picked": picked or "first",
            }

        # Transition to the next game. Reset inner env preserving decks
        # AND the LSTM hidden state in OpponentWrapper.
        self._pending_play_order_pick = False
        self.current_game_index = next_game_index
        self.match_step_count_in_current_game = 0

        seed = self._derive_game_seed(None)
        seed = self._align_seed_for_first_player(seed, first_player_rust)

        # `reset_inner_only` resets DigimonEnv (rebuilding the runner)
        # without calling `opponent_fn.reset_state` — LSTM h-state in
        # the OpponentWrapper persists across the game boundary.
        if hasattr(self.env, "reset_inner_only"):
            obs_new, info_new = self.env.reset_inner_only(
                seed=seed,
                options={"deck1": self._deck1, "deck2": self._deck2},
            )
        else:
            # Fallback for non-OpponentWrapper inner — full reset.
            obs_new, info_new = self.env.reset(
                seed=seed,
                options={"deck1": self._deck1, "deck2": self._deck2},
            )

        info_new = self._info_with_match_metadata(info_new, game_terminated=False)
        return obs_new, reward, False, truncated, info_new

    def _handle_game_terminal(
        self,
        obs: np.ndarray,
        reward: float,
        truncated: bool,
        info: Dict[str, Any],
    ) -> Tuple[np.ndarray, float, bool, bool, Dict[str, Any]]:
        """Called on the step where the inner game terminates. Updates
        match state, computes the per-game terminal adjustment, and
        either ends the match or transitions to the next game.
        """
        outcome = self._inner_digimon_env.terminal_outcome()
        winner_id_python = outcome.get("winner_id")  # 1 / 2 / None
        win_reason = outcome.get("win_reason") or outcome.get("reason") or "unknown"
        agent_conceded = bool(win_reason == "concede" and winner_id_python == 2)
        self.concede_history[self.current_game_index] = agent_conceded

        # Snapshot per-game stats BEFORE the inner env's reset (between
        # games or at match end). Engine counters reset on the next game's
        # reset(), so we must read them here.
        try:
            rl_state = self._inner_digimon_env._rl_state()
        except Exception:
            rl_state = {}
        self.match_game_history.append({
            "game_in_match_idx": int(self.current_game_index),
            "winner_id": winner_id_python,
            "win_reason": win_reason,
            "agent_conceded": agent_conceded,
            "digivolves_agent": int(rl_state.get("p1_digivolutions", 0)),
            "dna_digivolves_agent": int(rl_state.get("p1_dna_digivolutions", 0)),
            "digivolves_opponent": int(rl_state.get("p2_digivolutions", 0)),
            "dna_digivolves_opponent": int(rl_state.get("p2_dna_digivolutions", 0)),
            "steps": int(self.match_step_count_in_current_game),
        })

        # Per-game terminal adjustment: replace `DigimonEnv`'s ±10 (+ up
        # to +5 fast bonus, par 200) with BO3's ±12 (+ up to +3 fast
        # bonus, par 150). The dense reward accumulated before the
        # terminal step is unchanged.
        #
        # Path C: when the active reward profile defines a `match_outcome`
        # component, the profile owns all reward magnitudes — skip the
        # (bo3 − inner) calibration adjustment entirely. The per-game
        # terminal reward already flowed through the profile's
        # `terminal_outcome` component (DigimonEnv emitted 0 because
        # RewardProfileWrapper flips `legacy_reward=False`).
        steps_this_game = self.match_step_count_in_current_game
        if not self._profile_owns_match_terminal:
            inner_terminal = self._calc_inner_game_terminal(winner_id_python, steps_this_game)
            bo3_terminal = self._calc_bo3_game_terminal(winner_id_python, steps_this_game)
            reward = float(reward) + (bo3_terminal - inner_terminal)

        # Update match score and record the delta so
        # `_info_with_match_metadata` can compute match_score_before on
        # this same step.
        self._last_game_score_delta = [0, 0]
        if winner_id_python == 1:
            self.match_score[0] += 1
            self._last_game_score_delta[0] = 1
        elif winner_id_python == 2:
            self.match_score[1] += 1
            self._last_game_score_delta[1] = 1

        agent_wins = self.match_score[0]
        opp_wins = self.match_score[1]
        all_games_played = self.current_game_index >= 2  # zero-indexed
        match_step_limit_hit = self.match_step_count >= self.HARD_STEP_LIMIT_PER_MATCH

        match_decided = agent_wins >= 2 or opp_wins >= 2 or all_games_played or match_step_limit_hit

        # No-draw policy: a forced match end (step-limit / all games played)
        # must still produce a decisive winner. If neither side reached a
        # 2-win majority, award the tiebreak winner enough games to reach 2 —
        # mirroring the per-game `force_step_limit_winner` at the match tier.
        if match_decided and agent_wins < 2 and opp_wins < 2:
            self._force_match_decision()
            agent_wins = self.match_score[0]
            opp_wins = self.match_score[1]

        if match_decided:
            if self._profile_owns_match_terminal:
                snapshot = self._build_match_terminal_snapshot()
                match_reward = float(
                    self._reward_profile_wrapper.compute_match_terminal(snapshot)
                )
            else:
                match_reward = self._calc_match_terminal()
            reward = reward + match_reward
            info = self._info_with_match_metadata(info, game_terminated=True)
            info["outcome"] = info.get("outcome") or {}
            info["outcome"]["win_reason"] = win_reason
            info["outcome"]["winner_id"] = winner_id_python
            return obs, reward, True, truncated, info

        # ── Match continues. Install SelectPlayOrder for the loser. ──
        loser_pid = 2 if winner_id_python == 1 else 1
        self._pending_play_order_pick = True
        self._pending_play_order_chooser = loser_pid

        runner = self._inner_digimon_env.runner
        # PyO3 binding expects Python pid (1 or 2); does its own
        # Python→Rust translation internally.
        runner.request_play_order_selection(loser_pid)

        # Refresh obs to reflect the post-selection-install state.
        fresh_obs = runner.get_board_tensor(1)

        info = self._info_with_match_metadata(info, game_terminated=True)
        info["concede_this_game"] = agent_conceded
        info["outcome"] = info.get("outcome") or {}
        info["outcome"]["win_reason"] = win_reason
        info["outcome"]["winner_id"] = winner_id_python
        return fresh_obs, reward, False, truncated, info

    def _force_match_decision(self) -> None:
        """Award the tiebreak winner enough game-wins to reach 2 so a forced
        match end (step-limit / all games played) is always decisive — the BO3
        never reports a draw. Tiebreak order: game-win leader; on a 1-1 tie,
        the winner of the most recently completed game (which the engine's
        board-state tiebreaker decided if that game hit the step limit)."""
        a, o = self.match_score[0], self.match_score[1]
        if a >= 2 or o >= 2:
            return
        if a > o:
            tb = 0
        elif o > a:
            tb = 1
        else:
            # 1-1 tie: winner of the game that just ended.
            tb = 0 if self._last_game_score_delta[0] == 1 else 1
        self.match_score[tb] += 2 - self.match_score[tb]
        self._forced_match_tiebreak = True

    # ─── Match terminal calculator ─────────────────────────────────────

    def _calc_match_terminal(self) -> float:
        agent_wins = self.match_score[0]
        opp_wins = self.match_score[1]
        agent_conceded_any = any(self.concede_history)
        total_steps = float(self.match_step_count)

        if agent_wins >= 2:
            base = self.MATCH_TERMINAL_WIN
            if opp_wins == 0:
                base += self.MATCH_SWEEP_BONUS
            if agent_conceded_any:
                base += self.MATCH_SMART_CONCEDE_BONUS
            fast_bonus = (
                max(0.0, (self.MATCH_FAST_BONUS_PAR_STEPS - total_steps) / self.MATCH_FAST_BONUS_PAR_STEPS)
                * self.MATCH_FAST_BONUS_MAX
            )
            return base + fast_bonus
        if opp_wins >= 2:
            base = self.MATCH_TERMINAL_LOSS
            # Scared-concede penalty fires only on a 0-2 loss WITH any
            # agent concede in the match. Asymmetric: 1-2 with a smart
            # concede that didn't pan out is not extra-penalized (the
            # match terminal alone is enough).
            if agent_wins == 0 and agent_conceded_any:
                base += self.MATCH_SCARED_CONCEDE_PENALTY
            return base
        # Defensive fallback — unreachable now that `_force_match_decision`
        # resolves every forced match end to a 2-win majority (no draws).
        return self.MATCH_DRAW

    # ─── Game-terminal helpers ─────────────────────────────────────────

    def _calc_inner_game_terminal(
        self, winner_id_python: Optional[int], steps: int
    ) -> float:
        """Mirror of `DigimonEnv._compute_reward` terminal branch — what
        the inner env actually paid on the game-ending step. Used to
        subtract that contribution before re-adding the BO3 calibration.
        """
        if winner_id_python == 1:
            time_bonus = (
                max(0.0, (self.INNER_GAME_FAST_BONUS_PAR_STEPS - float(steps))
                / self.INNER_GAME_FAST_BONUS_PAR_STEPS)
                * self.INNER_GAME_FAST_BONUS_MAX
            )
            return self.INNER_GAME_TERMINAL_WIN + time_bonus
        if winner_id_python == 2:
            return self.INNER_GAME_TERMINAL_LOSS
        return self.INNER_GAME_DRAW

    def _calc_bo3_game_terminal(
        self, winner_id_python: Optional[int], steps: int
    ) -> float:
        """BO3 per-game terminal: ±12 + up to +3 fast (par 150)."""
        if winner_id_python == 1:
            time_bonus = (
                max(0.0, (self.GAME_FAST_BONUS_PAR_STEPS - float(steps))
                / self.GAME_FAST_BONUS_PAR_STEPS)
                * self.GAME_FAST_BONUS_MAX
            )
            return self.GAME_TERMINAL_WIN + time_bonus
        if winner_id_python == 2:
            return self.GAME_TERMINAL_LOSS
        return self.INNER_GAME_DRAW

    # ─── Helpers ───────────────────────────────────────────────────────

    def _derive_game_seed(self, requested: Optional[int]) -> int:
        if requested is not None:
            return int(requested)
        return int(self._rng.integers(0, 2**31 - 1))

    @staticmethod
    def _align_seed_for_first_player(base_seed: int, desired_first_player_rust: int) -> int:
        """`Game::new` uses `seed % player_count` (2 players → `seed % 2`)
        to choose the first player. Adjust the seed by +1 if needed so
        the chosen first player matches the BO3 play-order pick.
        """
        if base_seed % 2 == desired_first_player_rust:
            return base_seed
        return base_seed + 1

    def _info_with_match_metadata(
        self, info: Dict[str, Any], *, game_terminated: bool
    ) -> Dict[str, Any]:
        info = dict(info) if info else {}
        info["match_id"] = self.match_id
        info["game_index_in_match"] = self.current_game_index
        # `match_score_before` is the score at the START of the current
        # game. On non-terminating steps this equals match_score (no
        # update yet); on game-terminating steps the score was just
        # incremented in `_handle_game_terminal`, so we need to roll back
        # by the delta we just applied. The caller passes `game_terminated=True`
        # in that path and we use `_last_game_score_delta` for the roll-back.
        if game_terminated:
            info["match_score_before"] = {
                "p1_wins": max(0, self.match_score[0] - self._last_game_score_delta[0]),
                "p2_wins": max(0, self.match_score[1] - self._last_game_score_delta[1]),
            }
        else:
            info["match_score_before"] = {
                "p1_wins": self.match_score[0],
                "p2_wins": self.match_score[1],
            }
        # Play-order choice for the current game (None on game 1; populated
        # on games 2 / 3 once `_handle_play_order_pick` transitions in).
        if 0 <= self.current_game_index < len(self._play_order_history):
            info["play_order_choice"] = self._play_order_history[self.current_game_index]
        # `outcome.win_reason` is set on game-terminating steps by the
        # caller (`_handle_game_terminal`) via the inner `terminal_outcome()`
        # PyO3 dict. It's not stamped here because we don't have access to
        # the terminal-outcome reason in this helper.
        return info
