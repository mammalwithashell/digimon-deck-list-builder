"""Parametrized pytest integration for YAML scenario files.

Discovers all .yaml files under tests/scenarios/ and runs them via ScenarioRunner.
Skills generate archetype-specific scenarios to tests/scenarios/{archetype}/ for
automatic pytest discovery.
"""
import pytest
from pathlib import Path

from digimon_gym.engine.runners.scenario_runner import ScenarioRunner

SCENARIO_DIR = Path(__file__).parent.parent / "scenarios"


def discover_scenarios():
    """Find all .yaml/.yml files under tests/scenarios/."""
    if not SCENARIO_DIR.exists():
        return []
    files = sorted(SCENARIO_DIR.glob("**/*.yaml")) + sorted(SCENARIO_DIR.glob("**/*.yml"))
    return [(f, f.stem) for f in files]


SCENARIOS = discover_scenarios()


@pytest.mark.scenario
@pytest.mark.parametrize("scenario_path,name", SCENARIOS, ids=[n for _, n in SCENARIOS])
def test_yaml_scenario(scenario_path, name):
    """Run a YAML scenario file and verify all assertions pass."""
    result = ScenarioRunner.from_yaml(scenario_path).run()
    if not result.passed:
        pytest.fail(result.summary)
