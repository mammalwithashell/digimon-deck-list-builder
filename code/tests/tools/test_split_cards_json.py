"""Tests for tools.split_cards_json CLI."""
from __future__ import annotations

import json
from pathlib import Path

from tools import split_cards_json as m


CARD_FIXTURE = {
    "BT24-001": {
        "card_id": "BT24-001",
        "card_name_eng": "Gigimon",
        "effect_description_eng": "",
        "inherited_effect_description_eng": "[Your Turn] Test inherited text.",
    },
    "BT24-008": {
        "card_id": "BT24-008",
        "card_name_eng": "Agumon",
        "effect_description_eng": "[On Play] Test effect.",
        "inherited_effect_description_eng": "",
    },
    "P-229": {
        "card_id": "P-229",
        "card_name_eng": "Promo Test",
        "effect_description_eng": "Promo effect.",
    },
    "PROMO123": {
        "card_id": "PROMO123",
        "card_name_eng": "Hyphenless Test",
        "effect_description_eng": "Misc effect.",
    },
}


def test_set_id_from_card_id_standard():
    assert m.set_id_from_card_id("BT24-001") == "bt24"
    assert m.set_id_from_card_id("ST2-13") == "st2"
    assert m.set_id_from_card_id("P-229") == "p"


def test_set_id_from_card_id_hyphenless_falls_back_to_misc():
    assert m.set_id_from_card_id("PROMO123") == "_misc"


def test_render_card_json_is_stable_pretty_json():
    rendered = m.render_card_json(CARD_FIXTURE["BT24-001"])
    assert rendered.endswith("\n")
    assert "\r\n" not in rendered
    assert rendered.startswith('{\n  "card_id": "BT24-001"')
    assert json.loads(rendered) == CARD_FIXTURE["BT24-001"]


def test_write_card_json_writes_beside_set_directory_with_lf_newlines(tmp_path: Path):
    out = m.write_card_json("BT24-001", CARD_FIXTURE["BT24-001"], tmp_path)
    assert out == tmp_path / "bt24" / "BT24-001.json"
    assert out.exists()
    raw = out.read_bytes()
    assert b"\r\n" not in raw
    assert json.loads(raw.decode("utf-8")) == CARD_FIXTURE["BT24-001"]


def test_cmd_build_writes_every_card_by_default(tmp_path: Path):
    rc = m.cmd_build(CARD_FIXTURE, tmp_path)
    assert rc == 0
    assert (tmp_path / "bt24" / "BT24-001.json").exists()
    assert (tmp_path / "bt24" / "BT24-008.json").exists()
    assert (tmp_path / "p" / "P-229.json").exists()
    assert (tmp_path / "_misc" / "PROMO123.json").exists()


def test_cmd_build_filters_by_card(tmp_path: Path):
    rc = m.cmd_build(CARD_FIXTURE, tmp_path, card_id="BT24-001")
    assert rc == 0
    assert (tmp_path / "bt24" / "BT24-001.json").exists()
    assert not (tmp_path / "bt24" / "BT24-008.json").exists()


def test_cmd_build_filters_by_set(tmp_path: Path):
    rc = m.cmd_build(CARD_FIXTURE, tmp_path, set_id="bt24")
    assert rc == 0
    assert (tmp_path / "bt24" / "BT24-001.json").exists()
    assert (tmp_path / "bt24" / "BT24-008.json").exists()
    assert not (tmp_path / "p" / "P-229.json").exists()


def test_cmd_build_rejects_unknown_card(tmp_path: Path, capsys):
    rc = m.cmd_build(CARD_FIXTURE, tmp_path, card_id="BT99-999")
    assert rc == 2
    err = capsys.readouterr().err
    assert "unknown card" in err.lower()


def test_cmd_build_rejects_empty_set(tmp_path: Path, capsys):
    rc = m.cmd_build(CARD_FIXTURE, tmp_path, set_id="ex99")
    assert rc == 2
    err = capsys.readouterr().err
    assert "no cards matched set" in err.lower()


def test_cmd_check_passes_when_tree_matches(tmp_path: Path):
    m.cmd_build(CARD_FIXTURE, tmp_path)
    rc = m.cmd_check(CARD_FIXTURE, tmp_path)
    assert rc == 0


def test_cmd_check_fails_on_missing_file(tmp_path: Path, capsys):
    rc = m.cmd_check({"BT24-001": CARD_FIXTURE["BT24-001"]}, tmp_path)
    assert rc == 1
    err = capsys.readouterr().err
    assert "missing" in err.lower()
    assert "BT24-001" in err


def test_cmd_check_fails_on_stale_file(tmp_path: Path, capsys):
    m.cmd_build({"BT24-001": CARD_FIXTURE["BT24-001"]}, tmp_path)
    stale = tmp_path / "bt24" / "BT24-001.json"
    stale.write_text("STALE\n", encoding="utf-8", newline="\n")
    rc = m.cmd_check({"BT24-001": CARD_FIXTURE["BT24-001"]}, tmp_path)
    assert rc == 1
    err = capsys.readouterr().err
    assert "differs" in err.lower()
    assert "BT24-001" in err
