import hashlib
import re
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
    assert normalise(t) == "do x. then see # and #."
    # cross-reference wrapped across lines / glued number differs from inner one
    t2 = 'Y (For details, refer to 4-7 "Link\nCards")4-9'
    assert normalise(t2) == "y"


def test_normalise_character_folding():
    assert normalise("It\u2019s \u201cA\u201d\u3000B") == 'it\'s "a" b'


def test_fingerprint_is_sha256_of_norm():
    assert fingerprint(normalise("x")) == hashlib.sha256(normalise("x").encode()).hexdigest()


def test_normalise_has_no_double_spaces_or_edge_whitespace():
    assert normalise('a (For details, refer to 1-2 "X") 1-2  b ') == "a b"
    for u in parse_rules(SRC):
        assert "  " not in u.text_norm and u.text_norm == u.text_norm.strip(), u.id


def test_normalise_tolerates_dropped_opening_paren_and_spaced_number():
    # opening paren lost by the extraction (8-1-2-1 shape)
    assert normalise('Pick it. For details, refer to 2-3-6 "Requirements")2-3-5') == "pick it."
    # number after the closing paren separated by a space (4-17-5 shape)
    assert normalise('Done.) (For details, refer to 15-4 "Effect States") 15-4') == "done.)"
    # case-insensitive, optional comma
    assert normalise('A. (for details refer to 3-1 "Z")3-1 B') == "a. b"


def test_cross_references_fully_removed_in_corpus():
    units = {u.id: u for u in parse_rules(SRC)}
    for rid in ("8-1-2-1", "9-1-1", "11-5-1-2-1", "16-14-2", "18-1-2", "4-17-5"):
        n = units[rid].text_norm
        assert "refer to" not in n and "#" not in n, (rid, n)


def test_chapter_headings_are_not_continuation_text():
    units = parse_rules(SRC)
    by_id = {u.id: u for u in units}
    assert not by_id["16-47-2"].text.endswith("17. Rule Checks")
    assert by_id["16-47-2"].text.endswith('"Persistent Effects")')
    tail = re.compile(r"\b\d{1,2}\. [A-Z][a-z]+$")
    heading_line = re.compile(r"(?:^|\s)(?:1[0-8]|[1-9])\. [A-Z][a-z]+(?: [A-Za-z]+)*$")
    for u in units:
        assert not tail.search(u.text), u.id
        assert not heading_line.search(u.text), u.id


def test_page_number_lines_are_dropped(tmp_path):
    ff = chr(12)
    src = tmp_path / "cr.txt"
    src.write_text(
        "1-1. First rule text\n"
        + ff + "37\n"
        + "continues here\n"
        + "1-2. <Alliance>\n"
        + ff + "38\n"
        + "1-3. Third " + ff + "39\n"
        + "rule\n"
        + "Update History\n",
        encoding="utf-8",
    )
    units = {u.id: u for u in parse_rules(src)}
    assert units["1-1"].text == "First rule text continues here"
    assert units["1-2"].text == "<Alliance>"
    assert units["1-3"].text == "Third rule"


def test_bare_digit_continuation_line_is_kept(tmp_path):
    # Without a form feed a lone digit line is rule text (wrapped cross-reference).
    src = tmp_path / "cr.txt"
    src.write_text("1-1. See 15-4-\n4\n1-2. Next\nUpdate History\n", encoding="utf-8")
    assert parse_rules(src)[0].text == "See 15-4- 4"
