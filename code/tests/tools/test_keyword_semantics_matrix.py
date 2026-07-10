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

    assert rows["SecurityAttackPlus"]["rule"] == "16-3"
    assert rows["SecurityAttackMinus"]["rule"] == "16-3"
    assert rows["SecurityAttackPlus"]["kind"] == "persistent"
    assert rows["ArtsDigivolve"]["source"] == "engine_extension"
    assert rows["Ascension"]["optionality"] == "optional"


def test_matrix_covers_every_engine_keyword_variant():
    matrix = build_keyword_semantics_matrix(SOURCE_DOC)
    matrix_ids = {row["keyword_id"] for row in matrix["rows"] if row["engine_keyword"]}
    assert set(load_engine_keyword_variants(ENGINE_ENUM)) <= matrix_ids


def test_checked_in_keyword_semantics_matrix_is_current():
    expected = build_keyword_semantics_matrix(SOURCE_DOC)
    actual = json.loads(MATRIX_JSON.read_text(encoding="utf-8"))
    assert actual == expected
