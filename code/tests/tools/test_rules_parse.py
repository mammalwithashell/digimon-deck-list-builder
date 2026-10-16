import hashlib
from pathlib import Path

from tools.rules.parse_rules import fingerprint, normalise, parse_rules

REPO = Path(__file__).resolve().parents[3]
SRC = REPO / "qa/rules/aegis/comprehensive-4.3.md"


def test_parses_first_and_last_rule():
    units = parse_rules(SRC)
    ids = [u.id for u in units]
    assert ids[0] == "1-1" and "18-3-3-3" in ids and len(units) == 1119


def test_toc_update_history_excluded():
    units = parse_rules(SRC)
    ids = [u.id for u in units]
    assert all(not u.text.startswith("Update History") for u in units)
    assert "1-1" in set(ids) and ids.count("1-1") == 1


def test_unit_fields():
    units = parse_rules(SRC)
    first = units[0]
    assert first.chapter == 1
    assert first.fingerprint == fingerprint(first.text_norm)
    assert first.text_norm == normalise(first.text)
    assert all(u.chapter == int(u.id.split("-")[0]) for u in units)


def test_normalise_is_stable_across_extractions():
    a = "15-4-3-5-1 The turn player chooses 1 of their effects to\nactivate from among the effects that\ntriggered simultaneously.26"
    b = "15-4-3-5-1 The turn player  chooses 1 of their effects to activate from among the effects that triggered simultaneously."
    assert normalise(a) == normalise(b)


def test_normalise_cross_references_and_rule_numbers():
    t = 'Do X. (For details, refer to 15-8-3 "Trigger-Type Effects")15-8-3 Then see 8-3 and 16-1-2.'
    # removal happens after whitespace collapse (binding order), so a double space remains
    assert normalise(t) == "do x.  then see # and #."
    # cross-reference wrapped across lines / glued number differs from inner one
    t2 = 'Y (For details, refer to 4-7 "Link\nCards")4-9'
    assert normalise(t2) == "y"


def test_normalise_character_folding():
    assert normalise("It\u2019s \u201cA\u201d\u3000B") == 'it\'s "a" b'


def test_fingerprint_is_sha256_of_norm():
    assert fingerprint(normalise("x")) == hashlib.sha256(normalise("x").encode()).hexdigest()
