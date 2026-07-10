import json
from pathlib import Path


TIER1_MANIFEST = Path("qa/verification/tier1_suites.json")


def test_tier1_manifest_promotes_judge_quiz_as_required_gate():
    manifest = json.loads(TIER1_MANIFEST.read_text(encoding="utf-8"))
    suites = {suite["name"]: suite for suite in manifest["suites"]}

    judge_quiz = suites["judge_quiz"]
    assert judge_quiz["required"] is True
    assert "--test" in judge_quiz["command"]
    assert "judge_quiz" in judge_quiz["command"]


def test_tier1_manifest_includes_generated_rules_canaries():
    manifest = json.loads(TIER1_MANIFEST.read_text(encoding="utf-8"))
    names = {suite["name"] for suite in manifest["suites"]}

    assert {"keyword_semantics_matrix", "rules_faq"} <= names
