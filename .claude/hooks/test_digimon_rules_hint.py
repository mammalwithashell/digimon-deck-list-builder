"""Tests for the UserPromptSubmit rules-hint hook.

Run explicitly (not in the default pytest testpaths):
    python -m pytest .claude/hooks/test_digimon_rules_hint.py -v
"""
import json
import os
import subprocess
import sys
from pathlib import Path

HOOK = Path(__file__).resolve().parent / "digimon_rules_hint.py"


def _run(prompt: str) -> str:
    env = dict(os.environ, DIGIMON_RULES_PDF_DIR=r"C:/FAKE/Digimon TCG resources")
    p = subprocess.run(
        [sys.executable, str(HOOK)],
        input=json.dumps({"prompt": prompt}),
        capture_output=True,
        text=True,
        env=env,
    )
    assert p.returncode == 0, p.stderr
    return p.stdout


def test_keyword_match_emits_pointer():
    out = _run("How does Blocker interact with an attack?")
    assert "general_rule.pdf" in out
    assert "Blocker" in out
    assert "digimon-rules" in out


def test_rule_number_match_emits_pointer():
    out = _run("Explain rule 16-36 please")
    assert "general_rule.pdf" in out
    assert "16" in out


def test_no_rules_vocab_is_silent():
    assert _run("Refactor the deck builder pagination component").strip() == ""


def test_bare_common_verb_does_not_fire():
    # conservative: 'attack' alone (no keyword name / rule number) stays silent
    assert _run("make the attack button bigger in the UI").strip() == ""


def test_common_word_keyword_requires_brackets():
    # bare 'save' must NOT fire (RL/codebase noise); bracketed <Save> must fire
    assert _run("save the training checkpoint to disk").strip() == ""
    assert "general_rule.pdf" in _run("is <Save> optional or mandatory?")


def test_bracketed_value_form_matches():
    # bare-name matching also catches the bracketed-with-value printed form
    assert "general_rule.pdf" in _run("does <Security Attack +1> stack?")
