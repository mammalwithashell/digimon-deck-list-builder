import json
from pathlib import Path

from tools.keyword_semantics_matrix import (
    build_keyword_semantics_matrix,
    load_engine_keyword_variants,
)


SOURCE_DOC = Path("docs/digimon-rules/keyword-semantics.md")
MATRIX_JSON = Path("qa/rules/keyword_semantics_matrix.json")
ENGINE_ENUM = Path("code/digimon-engine/src/enums.rs")


def test_matrix_derives_split_security_rows_and_engine_extensions():
    matrix = build_keyword_semantics_matrix(SOURCE_DOC)
    rows = {row["keyword_id"]: row for row in matrix["rows"]}

    # Comprehensive Rules Ver. 4.3 numbering: the new §16-3 ("specified cards")
    # shifted every keyword up by one (Security A. 16-3 -> 16-4).
    assert rows["SecurityAttackPlus"]["rule"] == "16-4"
    assert rows["SecurityAttackMinus"]["rule"] == "16-4"
    assert rows["SecurityAttackPlus"]["kind"] == "persistent"
    assert rows["ArtsDigivolve"]["source"] == "engine_extension"
    assert rows["ArtsDigivolve"]["rule"] == "4-20"
    # 4.x keywords are real §16 rules now, sourced from the table, not extensions.
    assert rows["Ascension"]["optionality"] == "optional"
    assert rows["Ascension"]["source"] == "rules" and rows["Ascension"]["rule"] == "16-43"
    assert rows["Engage"]["rule"] == "16-44" and rows["Engage"]["optionality"] == "optional"
    assert rows["Guard"]["rule"] == "16-45"
    assert rows["Guard"]["optionality"] == "optional_cost_then_mandatory"
    assert rows["Detach"]["rule"] == "16-46" and rows["Detach"]["engine_keyword"] is False
    assert rows["Succession"]["rule"] == "16-47" and rows["Succession"]["kind"] == "persistent"
    assert rows["UseReq"]["rule"] == "16-42" and rows["UseReq"]["kind"] == "persistent"


def test_matrix_covers_every_engine_keyword_variant():
    matrix = build_keyword_semantics_matrix(SOURCE_DOC)
    matrix_ids = {row["keyword_id"] for row in matrix["rows"] if row["engine_keyword"]}
    assert set(load_engine_keyword_variants(ENGINE_ENUM)) <= matrix_ids


def test_checked_in_keyword_semantics_matrix_is_current():
    expected = build_keyword_semantics_matrix(SOURCE_DOC)
    actual = json.loads(MATRIX_JSON.read_text(encoding="utf-8"))
    assert actual == expected
