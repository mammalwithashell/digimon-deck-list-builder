from __future__ import annotations

import hashlib

import pytest

from digimon_gym.ai.autofix_apply import (
    ApplyValidationError,
    apply_validated_edits,
    validate_edit_payload,
)


def _sha256_text(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def test_scope_script_allows_only_primary_script():
    payload = {
        "edits": [
            {
                "path": "digimon_gym/engine/data/scripts/generated/bt24/bt24_001.py",
                "expected_hash": "abc",
                "new_content": "x",
            }
        ]
    }
    edits = validate_edit_payload(
        result_payload=payload,
        scope_profile="script",
        set_id="bt24",
        module_name="bt24_001",
    )
    assert len(edits) == 1


def test_scope_script_rejects_engine_path():
    payload = {
        "edits": [
            {
                "path": "digimon_gym/engine/game.py",
                "expected_hash": "abc",
                "new_content": "x",
            }
        ]
    }
    with pytest.raises(ApplyValidationError):
        validate_edit_payload(
            result_payload=payload,
            scope_profile="script",
            set_id="bt24",
            module_name="bt24_001",
        )


def test_scope_global_deny_blocks_github():
    payload = {
        "edits": [
            {
                "path": ".github/workflows/test.yml",
                "expected_hash": "abc",
                "new_content": "x",
            }
        ]
    }
    with pytest.raises(ApplyValidationError):
        validate_edit_payload(
            result_payload=payload,
            scope_profile="script_engine",
            set_id="bt24",
            module_name="bt24_001",
        )


def test_apply_rejects_hash_mismatch(tmp_path):
    root = tmp_path
    target = root / "digimon_gym" / "engine" / "data" / "scripts" / "generated" / "bt24"
    target.mkdir(parents=True, exist_ok=True)
    file_path = target / "bt24_001.py"
    file_path.write_text("print('old')\n", encoding="utf-8")

    with pytest.raises(ApplyValidationError):
        apply_validated_edits(
            repo_root=root,
            edits=[
                {
                    "path": "digimon_gym/engine/data/scripts/generated/bt24/bt24_001.py",
                    "expected_hash": _sha256_text("print('different')\n"),
                    "new_content": "print('new')\n",
                }
            ],
        )
