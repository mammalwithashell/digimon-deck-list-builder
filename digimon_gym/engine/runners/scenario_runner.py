"""ScenarioRunner: execute YAML test scenarios declaratively.

Loads a YAML scenario file, creates a DebugRunner with the specified setup,
executes actions, and evaluates assertions against the final game state.

Usage:
    from digimon_gym.engine.runners.scenario_runner import ScenarioRunner

    result = ScenarioRunner.from_yaml("qa/scenarios/my-test.yaml").run()
    print(result.passed, result.summary)
"""

from __future__ import annotations

import json
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Dict, List, Optional

import yaml

from digimon_gym.engine.runners.debug_runner import DebugRunner, ActionResult


# ── Result dataclasses ───────────────────────────────────────────────────


@dataclass
class AssertionResult:
    """Result of a single assertion evaluation."""
    assertion: Dict[str, Any]
    passed: bool
    detail: str


@dataclass
class ScenarioResult:
    """Result of running a complete scenario."""
    name: str
    passed: bool
    assertions_passed: int
    assertions_failed: int
    assertion_details: List[AssertionResult]
    action_log: List[str]  # action descriptions executed
    error: Optional[str] = None

    @property
    def summary(self) -> str:
        status = "PASS" if self.passed else "FAIL"
        parts = [f"[{status}] {self.name}"]
        parts.append(f"  Assertions: {self.assertions_passed}/{self.assertions_passed + self.assertions_failed}")
        for ad in self.assertion_details:
            mark = "+" if ad.passed else "X"
            parts.append(f"  [{mark}] {ad.detail}")
        if self.error:
            parts.append(f"  ERROR: {self.error}")
        return "\n".join(parts)


# ── Deck resolution ─────────────────────────────────────────────────────


def _load_archetype_deck(archetype_name: str) -> List[str]:
    """Load the first decklist for an archetype from deck_library.json."""
    lib_path = Path(__file__).parent.parent / "data" / "deck_library.json"
    with open(lib_path, "r", encoding="utf-8") as f:
        library = json.load(f)
    arch = library["archetypes"].get(archetype_name)
    if not arch or not arch.get("decklists"):
        raise ValueError(f"No decklists for archetype: {archetype_name}")
    return json.loads(arch["decklists"][0]["decklist"])


# ── Assertion evaluation ────────────────────────────────────────────────


def _evaluate_assertion(runner: DebugRunner, assertion: Dict[str, Any]) -> AssertionResult:
    """Evaluate a single assertion against the current game state."""
    snap = runner.snapshot()

    # Each assertion is a dict with exactly one key (the assertion type)
    for atype, params in assertion.items():
        if atype == "zone_contains":
            return _assert_zone_contains(snap, params, expected=True)
        elif atype == "zone_not_contains":
            return _assert_zone_contains(snap, params, expected=False)
        elif atype == "zone_count":
            return _assert_zone_count(snap, params)
        elif atype == "field_count":
            return _assert_field_count(snap, params)
        elif atype == "memory_exact":
            return _assert_memory(snap, params, params)
        elif atype == "memory_range":
            return _assert_memory(snap, params.get("min", -100), params.get("max", 100))
        elif atype == "phase":
            actual = snap.phase
            expected = params if isinstance(params, str) else params.get("phase", params)
            ok = actual == expected
            return AssertionResult(assertion, ok, f"Phase: expected={expected}, actual={actual}")
        elif atype == "is_suspended":
            return _assert_suspended(snap, params, expected=True)
        elif atype == "not_suspended":
            return _assert_suspended(snap, params, expected=False)
        elif atype == "has_keyword":
            return _assert_keyword(runner, params, expected=True)
        elif atype == "no_keyword":
            return _assert_keyword(runner, params, expected=False)
        elif atype == "dp_range":
            return _assert_dp_range(snap, params)
        elif atype == "game_over":
            expected = params if isinstance(params, bool) else True
            ok = snap.is_game_over == expected
            return AssertionResult(assertion, ok,
                f"Game over: expected={expected}, actual={snap.is_game_over}")
        elif atype == "not_game_over":
            ok = not snap.is_game_over
            return AssertionResult(assertion, ok,
                f"Not game over: actual is_game_over={snap.is_game_over}")
        elif atype == "log_contains":
            substring = params if isinstance(params, str) else params.get("substring", "")
            logs = runner._logger.get_logs()
            found = any(substring.lower() in log.lower() for log in logs)
            return AssertionResult(assertion, found,
                f"Log contains '{substring}': {'found' if found else 'not found'}")
        elif atype == "winner":
            expected = params if isinstance(params, int) else params.get("player")
            ok = snap.winner == expected
            return AssertionResult(assertion, ok,
                f"Winner: expected=P{expected}, actual={'P' + str(snap.winner) if snap.winner else 'None'}")
        else:
            return AssertionResult(assertion, False, f"Unknown assertion type: {atype}")

    return AssertionResult(assertion, False, "Empty assertion")


def _get_zone_cards(snap, player: int, zone: str) -> List[str]:
    """Get card IDs from a zone in the snapshot."""
    if zone == "hand":
        return snap.p1_hand if player == 1 else snap.p2_hand
    elif zone == "trash":
        return snap.p1_trash if player == 1 else snap.p2_trash
    elif zone == "field":
        field_slots = snap.p1_field if player == 1 else snap.p2_field
        return [s.card_id for s in field_slots]
    else:
        return []


def _assert_zone_contains(snap, params: dict, expected: bool) -> AssertionResult:
    player = params.get("player", 1)
    zone = params.get("zone", "field")
    card = params.get("card", "")
    cards = _get_zone_cards(snap, player, zone)
    found = card in cards
    ok = found == expected
    verb = "contains" if expected else "does not contain"
    actual = "found" if found else "not found"
    return AssertionResult(
        {"zone_contains" if expected else "zone_not_contains": params}, ok,
        f"P{player} {zone} {verb} {card}: {actual} (zone={cards})",
    )


def _assert_zone_count(snap, params: dict) -> AssertionResult:
    player = params.get("player", 1)
    zone = params.get("zone", "field")
    expected = params.get("count", 0)
    cards = _get_zone_cards(snap, player, zone)
    actual = len(cards)
    ok = actual == expected
    return AssertionResult(
        {"zone_count": params}, ok,
        f"P{player} {zone} count: expected={expected}, actual={actual}",
    )


def _assert_field_count(snap, params: dict) -> AssertionResult:
    player = params.get("player", 1)
    expected = params.get("count", 0)
    field_slots = snap.p1_field if player == 1 else snap.p2_field
    actual = len(field_slots)
    ok = actual == expected
    return AssertionResult(
        {"field_count": params}, ok,
        f"P{player} field count: expected={expected}, actual={actual}",
    )


def _assert_memory(snap, min_val, max_val) -> AssertionResult:
    if isinstance(min_val, dict):
        min_v = min_val.get("min", -100)
        max_v = min_val.get("max", 100)
    else:
        min_v = min_val
        max_v = max_val
    actual = snap.memory
    ok = min_v <= actual <= max_v
    return AssertionResult(
        {"memory_range": {"min": min_v, "max": max_v}}, ok,
        f"Memory: expected [{min_v}, {max_v}], actual={actual}",
    )


def _assert_suspended(snap, params: dict, expected: bool) -> AssertionResult:
    player = params.get("player", 1)
    card = params.get("card", "")
    field_slots = snap.p1_field if player == 1 else snap.p2_field
    for slot in field_slots:
        if slot.card_id == card:
            ok = slot.is_suspended == expected
            state = "suspended" if expected else "unsuspended"
            return AssertionResult(
                {"is_suspended" if expected else "not_suspended": params}, ok,
                f"P{player} {card}: expected {state}, actual={'suspended' if slot.is_suspended else 'unsuspended'}",
            )
    return AssertionResult(
        {"is_suspended" if expected else "not_suspended": params}, False,
        f"P{player} {card}: not found on field",
    )


def _assert_keyword(runner: DebugRunner, params: dict, expected: bool) -> AssertionResult:
    player_id = params.get("player", 1)
    card = params.get("card", "")
    keyword = params.get("keyword", "")
    player = runner.game.player1 if player_id == 1 else runner.game.player2
    for perm in player.battle_area:
        top = perm.top_card
        if top and top.c_entity_base and top.c_entity_base.card_id == card:
            has_it = perm.has_keyword(f"_is_{keyword}")
            ok = has_it == expected
            verb = "has" if expected else "does not have"
            return AssertionResult(
                {"has_keyword" if expected else "no_keyword": params}, ok,
                f"P{player_id} {card} {verb} {keyword}: {'yes' if has_it else 'no'}",
            )
    return AssertionResult(
        {"has_keyword" if expected else "no_keyword": params}, False,
        f"P{player_id} {card}: not found on field",
    )


def _assert_dp_range(snap, params: dict) -> AssertionResult:
    player = params.get("player", 1)
    card = params.get("card", "")
    min_dp = params.get("min", 0)
    max_dp = params.get("max", 999999)
    field_slots = snap.p1_field if player == 1 else snap.p2_field
    for slot in field_slots:
        if slot.card_id == card:
            actual = slot.dp
            if actual is None:
                return AssertionResult({"dp_range": params}, False,
                    f"P{player} {card}: has no DP (tamer/egg)")
            ok = min_dp <= actual <= max_dp
            return AssertionResult({"dp_range": params}, ok,
                f"P{player} {card} DP: expected [{min_dp}, {max_dp}], actual={actual}")
    return AssertionResult({"dp_range": params}, False,
        f"P{player} {card}: not found on field")


# ── ScenarioRunner ───────────────────────────────────────────────────────


class ScenarioRunner:
    """Load and execute YAML scenario files."""

    def __init__(self, scenario: Dict[str, Any]):
        self.scenario = scenario
        self.name = scenario.get("name", "unnamed")

    @classmethod
    def from_yaml(cls, path: str | Path) -> ScenarioRunner:
        """Load a scenario from a YAML file."""
        path = Path(path)
        with open(path, "r", encoding="utf-8") as f:
            data = yaml.safe_load(f)
        return cls(data)

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> ScenarioRunner:
        """Create from a plain dict."""
        return cls(data)

    def run(self) -> ScenarioResult:
        """Execute the scenario and return pass/fail with details."""
        action_log = []
        assertion_details = []

        try:
            runner = self._create_runner()
            self._apply_setup(runner)
            action_log = self._execute_actions(runner)
            assertion_details = self._evaluate_assertions(runner)
        except Exception as e:
            return ScenarioResult(
                name=self.name,
                passed=False,
                assertions_passed=0,
                assertions_failed=0,
                assertion_details=[],
                action_log=action_log,
                error=f"{type(e).__name__}: {e}",
            )

        passed_count = sum(1 for a in assertion_details if a.passed)
        failed_count = sum(1 for a in assertion_details if not a.passed)

        return ScenarioResult(
            name=self.name,
            passed=failed_count == 0 and not any(
                a for a in assertion_details if not a.passed
            ),
            assertions_passed=passed_count,
            assertions_failed=failed_count,
            assertion_details=assertion_details,
            action_log=action_log,
        )

    def _create_runner(self) -> DebugRunner:
        """Create the DebugRunner from scenario setup."""
        setup = self.scenario.get("setup", {})

        # Resolve decks
        deck1 = setup.get("deck1")
        deck2 = setup.get("deck2")
        if not deck1:
            arch1 = setup.get("deck1_archetype")
            if arch1:
                deck1 = _load_archetype_deck(arch1)
            else:
                raise ValueError("Scenario must specify deck1 or deck1_archetype")
        if not deck2:
            arch2 = setup.get("deck2_archetype")
            if arch2:
                deck2 = _load_archetype_deck(arch2)
            else:
                raise ValueError("Scenario must specify deck2 or deck2_archetype")

        return DebugRunner(
            deck1=deck1,
            deck2=deck2,
            skip_shuffle=setup.get("skip_shuffle", True),
            auto_mulligan=setup.get("auto_mulligan", True),
            first_player=setup.get("first_player", 1),
            initial_memory=setup.get("memory", setup.get("initial_memory", 0)),
            starting_hand1=setup.get("starting_hand1"),
            starting_hand2=setup.get("starting_hand2"),
        )

    def _apply_setup(self, runner: DebugRunner):
        """Apply per-player zone setup from the scenario."""
        setup = self.scenario.get("setup", {})

        for pid, key in [(1, "player1"), (2, "player2")]:
            player_setup = setup.get(key, {})
            if not player_setup:
                continue

            # Hand injection
            for card_id in player_setup.get("hand", []):
                runner.inject_card(pid, card_id, "hand")

            # Field placement
            for entry in player_setup.get("field", []):
                if isinstance(entry, dict):
                    runner.place_on_field(
                        pid, entry.get("card_ids", []),
                        entry.get("is_suspended", False),
                        entry.get("turn_played", -1),
                    )
                elif isinstance(entry, list):
                    runner.place_on_field(pid, entry)

            # Security
            for card_id in player_setup.get("security", []):
                runner.inject_card(pid, card_id, "security_top")

            # Trash
            for card_id in player_setup.get("trash", []):
                runner.inject_card(pid, card_id, "trash")

            # Breeding
            breeding = player_setup.get("breeding", [])
            if breeding:
                runner.place_in_breeding(pid, breeding)

            # Library top
            for card_id in reversed(player_setup.get("library_top", [])):
                runner.inject_card(pid, card_id, "library_top")

        # Memory override (after player setup, in case setup affects it)
        if "memory" in setup:
            runner.set_memory(setup["memory"])

    def _execute_actions(self, runner: DebugRunner) -> List[str]:
        """Execute the action sequence from the scenario."""
        action_log = []
        actions = self.scenario.get("actions", [])

        # Auto-advance past breeding phase if first action is a Main phase action
        from digimon_gym.engine.data.enums import GamePhase
        if (actions and runner.game.current_phase == GamePhase.Breeding):
            pass_action = runner.find_action("Pass breeding")
            if pass_action is not None:
                result = runner.execute(pass_action)
                action_log.append(f"[auto] Pass breeding phase")

        for step in actions:
            if isinstance(step, dict):
                if "find" in step:
                    action_id = runner.find_action(step["find"])
                    if action_id is None:
                        available = runner.actions()
                        raise ValueError(
                            f"Action not found: '{step['find']}'. "
                            f"Available: {available}"
                        )
                    result = runner.execute(action_id)
                    action_log.append(f"[{action_id}] {result.action_description}")

                    # Check expected memory change
                    if "expect_memory_change" in step:
                        expected = step["expect_memory_change"]
                        actual = result.after.memory - result.before.memory
                        if actual != expected:
                            raise ValueError(
                                f"Memory change mismatch after '{step['find']}': "
                                f"expected={expected}, actual={actual}"
                            )

                elif "action" in step:
                    action_id = step["action"]
                    result = runner.execute(action_id)
                    action_log.append(f"[{action_id}] {result.action_description}")

                elif "auto_resolve" in step:
                    max_steps = step.get("max_steps", 20)
                    results = runner.auto_resolve(max_steps)
                    for r in results:
                        action_log.append(f"[auto:{r.action_id}] {r.action_description}")

                elif "set_memory" in step:
                    runner.set_memory(step["set_memory"])
                    action_log.append(f"[setup] set_memory={step['set_memory']}")

                elif "inject" in step:
                    inj = step["inject"]
                    runner.inject_card(
                        inj.get("player", 1),
                        inj["card"],
                        inj.get("zone", "hand"),
                    )
                    action_log.append(f"[setup] inject {inj['card']} to P{inj.get('player', 1)} {inj.get('zone', 'hand')}")

            elif isinstance(step, int):
                result = runner.execute(step)
                action_log.append(f"[{step}] {result.action_description}")

        return action_log

    def _evaluate_assertions(self, runner: DebugRunner) -> List[AssertionResult]:
        """Evaluate all assertions against the current state."""
        assertions = self.scenario.get("assertions", [])
        return [_evaluate_assertion(runner, a) for a in assertions]
