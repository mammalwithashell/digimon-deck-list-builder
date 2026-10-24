"""Exercise the actual search CLI admission boundary before game generation."""
import json
import os
from pathlib import Path
import subprocess

import pytest

CLI = os.environ.get("DIGIMON_SEARCH_CLI")
ROOT = Path(__file__).resolve().parents[3]
pytestmark = pytest.mark.skipif(not CLI, reason="set DIGIMON_SEARCH_CLI to the freshly built native binary")


@pytest.mark.parametrize("card,reason", [("AD1-002","not admissible"), ("AD1-001","illegal")])
def test_native_search_rejects_blocked_and_illegal_decks_even_with_pool_all(tmp_path, card, reason):
    pool = tmp_path / "decks.json"
    pool.write_text(json.dumps([[card]]))
    output = tmp_path / "output"
    result = subprocess.run([CLI,"--pool","all","--cards-json",str(ROOT / "data/cards.json"),
        "selfplay","--uniform","--decks",str(pool),"--games","1","--out",str(output)],
        capture_output=True,text=True)
    assert result.returncode == 2, result.stderr
    assert reason in result.stderr
    assert not output.exists()


def test_native_search_refuses_missing_admission_artifacts(tmp_path):
    cards = tmp_path / "cards.json"
    cards.write_text("{}")
    pool = tmp_path / "decks.json"
    pool.write_text(json.dumps([["AD1-001"]]))
    output = tmp_path / "output"
    result = subprocess.run([CLI,"--pool","all","--cards-json",str(cards),
        "selfplay","--uniform","--decks",str(pool),"--games","1","--out",str(output)],
        capture_output=True,text=True)
    assert result.returncode == 2
    assert "mechanics_readiness.json" in result.stderr
    assert not output.exists()


@pytest.mark.parametrize("value", ["invalid", "-1"])
def test_native_search_rejects_invalid_cache_configuration_before_generation(tmp_path, value):
    pool = tmp_path / "decks.json"
    pool.write_text(json.dumps([["AD1-001"]]))
    output = tmp_path / "output"
    env = {**os.environ, "DIGIMON_SEARCH_EVAL_CACHE_ENTRIES": value}
    result = subprocess.run([CLI, "--pool", "all", "--cards-json", str(ROOT / "data/cards.json"),
        "selfplay", "--uniform", "--decks", str(pool), "--games", "1", "--out", str(output)],
        capture_output=True, text=True, env=env)
    assert result.returncode == 2
    assert "DIGIMON_SEARCH_EVAL_CACHE_ENTRIES" in result.stderr
    assert not output.exists()


def test_native_search_rejects_zero_evaluation_batch_size(tmp_path):
    pool = tmp_path / "decks.json"
    pool.write_text(json.dumps([["AD1-001"]]))
    output = tmp_path / "output"
    result = subprocess.run([CLI, "--pool", "all", "--cards-json", str(ROOT / "data/cards.json"),
        "selfplay", "--uniform", "--decks", str(pool), "--games", "1", "--out", str(output),
        "--eval-batch-size", "0"], capture_output=True, text=True)
    assert result.returncode == 2
    assert "--eval-batch-size must be positive" in result.stderr
    assert not output.exists()
