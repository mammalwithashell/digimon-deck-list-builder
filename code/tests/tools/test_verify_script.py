import json
import subprocess
import sys


def run_verify(*args: str) -> dict:
    result = subprocess.run(
        [sys.executable, "scripts/verify", *args, "--dry-run", "--json"],
        check=True,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    return json.loads(result.stdout)


def test_tier1_dry_run_is_cumulative_and_promotes_judge_quiz():
    payload = run_verify("--tier", "1")
    names = [suite["name"] for suite in payload["suites"]]

    assert payload["requested_tier"] == 1
    assert "impact_index_check" in names
    assert "judge_quiz" in names
    assert all(
        suite["env"].get("RUST_MIN_STACK") == "268435456"
        for suite in payload["suites"]
        if suite["kind"] == "cargo"
    )


def test_tier2_dry_run_wires_scope_goldens_and_fuzz():
    payload = run_verify(
        "--tier",
        "2",
        "--path",
        "code/digimon-engine/src/dsl_cards/step/draw.rs",
    )
    names = [suite["name"] for suite in payload["suites"]]

    assert "impact_scope" in names
    assert "replay_goldens_verify" in names
    assert "invariant_fuzz" in names
    assert any(name.startswith("cards_behavioral_") for name in names)
