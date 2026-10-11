"""Tests for the append-only global card-index registry manager."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from tools import build_registry as command
from tools.card_data.registry import (
    REGISTRY_CAPACITY,
    RegistryValidationError,
    StableCardRegistry,
    allocate_append_only,
    apply_compatibility_values,
    compatibility_values,
    read_registry,
    validate_append_only,
)


def _registry(assignments: dict[str, int]) -> StableCardRegistry:
    return StableCardRegistry(
        schema_version=1,
        registry_version=1,
        capacity=REGISTRY_CAPACITY,
        padding_index=0,
        assignments=assignments,
    )


def _card(card_id: str, **overrides) -> dict:
    row = {
        "card_id": card_id,
        "card_name_eng": card_id,
        "index": 0,
        "norm_id": 0.0,
        "card_index": int(card_id.rsplit("-", 1)[1]),
    }
    row.update(overrides)
    return row


def test_committed_registry_is_lossless_extraction_of_cards_json():
    root = Path(__file__).resolve().parents[3]
    cards = json.loads((root / "data/cards.json").read_text(encoding="utf-8"))
    registry = read_registry(root / "data/card_registry.json")

    assert registry.capacity == 20_000
    assert registry.registry_version == 1
    assert registry.padding_index == 0
    assert dict(registry.assignments) == {
        card_id: row["index"] for card_id, row in cards.items()
    }
    assert StableCardRegistry.from_cards_json(cards) == registry


def test_append_only_allocation_preserves_old_indices_and_uses_natural_order():
    previous = _registry({"BT1-001": 1, "BT1-003": 3})
    candidate = allocate_append_only(
        previous, ["BT1-001", "BT10-001", "BT2-001"]
    )

    assert candidate.assignments["BT1-001"] == 1
    assert candidate.assignments["BT1-003"] == 3
    assert candidate.assignments["BT2-001"] == 4
    assert candidate.assignments["BT10-001"] == 5


def test_allocation_rejects_duplicate_requests_and_new_test_ids():
    previous = _registry({"BT1-001": 1})
    with pytest.raises(RegistryValidationError, match="duplicate card ID"):
        allocate_append_only(previous, ["BT1-001", "BT1-001"])
    with pytest.raises(RegistryValidationError, match="synthetic/test"):
        allocate_append_only(previous, ["BT1-001", "TEST-999"])


def test_registry_rejects_duplicate_indices_and_capacity_overflow():
    with pytest.raises(RegistryValidationError, match="duplicate global index"):
        _registry({"BT1-001": 1, "BT1-002": 1})

    full = _registry({"BT1-001": REGISTRY_CAPACITY})
    with pytest.raises(RegistryValidationError, match="capacity"):
        allocate_append_only(full, ["BT1-001", "BT1-002"])


def test_registry_file_rejects_duplicate_card_id_keys(tmp_path: Path):
    path = tmp_path / "card_registry.json"
    path.write_text(
        '{"schema_version":1,"registry_version":1,"capacity":20000,'
        '"padding_index":0,"assignment_count":2,'
        '"assignments":{"BT1-001":1,"BT1-001":2}}',
        encoding="utf-8",
    )

    with pytest.raises(RegistryValidationError, match="duplicate JSON object key"):
        read_registry(path)


def test_append_only_validation_rejects_removal_reassignment_and_index_reuse():
    previous = _registry({"BT1-001": 1, "BT1-002": 2})
    with pytest.raises(RegistryValidationError, match="remove"):
        validate_append_only(previous, _registry({"BT1-001": 1}))
    with pytest.raises(RegistryValidationError, match="reassign"):
        validate_append_only(
            previous, _registry({"BT1-001": 3, "BT1-002": 2})
        )
    with pytest.raises(RegistryValidationError, match="contiguous"):
        validate_append_only(
            previous,
            _registry({"BT1-001": 1, "BT1-002": 2, "BT1-003": 4}),
        )
    with pytest.raises(RegistryValidationError, match="synthetic/test"):
        validate_append_only(_registry({}), _registry({"TEST-999": 1}))


def test_norm_id_and_non_unique_card_index_are_derived_independently():
    first = compatibility_values("BT1-001", 100)
    second = compatibility_values("EX1-001", 200)

    assert first == {"index": 100, "norm_id": 100 / 20_000, "card_index": 1}
    assert second == {"index": 200, "norm_id": 200 / 20_000, "card_index": 1}
    assert first["index"] != second["index"]
    assert first["card_index"] == second["card_index"]


def test_compatibility_projection_rejects_index_or_suffix_reassignment():
    registry = _registry({"BT1-001": 7})
    with pytest.raises(RegistryValidationError, match="reassign"):
        apply_compatibility_values(
            {"BT1-001": _card("BT1-001", index=8)}, registry
        )
    with pytest.raises(RegistryValidationError, match="numeric suffix"):
        apply_compatibility_values(
            {"BT1-001": _card("BT1-001", index=7, card_index=2)}, registry
        )


def test_legacy_build_registry_function_is_now_append_only_and_network_free():
    result = command.build_registry(
        ["BT1-001", "BT2-001"], {"BT1-001": 42}
    )
    assert result == {"BT1-001": 42, "BT2-001": 43}
    assert not hasattr(command, "fetch_set")
    assert not hasattr(command, "convert_card")
    assert command.card_sort_key("BT14-001") == ("BT", 14, 1)
    assert command.card_sort_key("P-001") == ("P", 0, 1)


def test_cli_creates_registry_and_only_populates_derived_values(tmp_path: Path):
    cards_path = tmp_path / "cards.json"
    registry_path = tmp_path / "card_registry.json"
    cards = {
        "BT1-001": _card("BT1-001", index=1, norm_id=0.5, card_name_eng="Keep Me"),
        "BT2-001": _card("BT2-001", index=2, norm_id=0.5, card_name_eng="Also Keep"),
    }
    cards_path.write_text(json.dumps(cards), encoding="utf-8")

    assert command.main(
        ["--cards-json", str(cards_path), "--registry", str(registry_path)]
    ) == 0
    written = json.loads(cards_path.read_text(encoding="utf-8"))
    assert written["BT1-001"]["norm_id"] == 1 / 20_000
    assert written["BT1-001"]["card_name_eng"] == "Keep Me"
    assert written["BT2-001"]["card_name_eng"] == "Also Keep"
    assert read_registry(registry_path).assignments == {
        "BT1-001": 1,
        "BT2-001": 2,
    }
    assert command.main(
        [
            "--check",
            "--cards-json",
            str(cards_path),
            "--registry",
            str(registry_path),
        ]
    ) == 0


def test_cli_allocates_new_local_card_after_registry_max(tmp_path: Path):
    cards_path = tmp_path / "cards.json"
    registry_path = tmp_path / "card_registry.json"
    cards = {
        "BT1-001": _card("BT1-001", index=1, norm_id=1 / 20_000),
        "BT2-001": _card("BT2-001", index=0, norm_id=0.0),
    }
    cards_path.write_text(json.dumps(cards), encoding="utf-8")
    command.write_registry(registry_path, _registry({"BT1-001": 1}))

    assert command.main(
        ["--cards-json", str(cards_path), "--registry", str(registry_path)]
    ) == 0
    assert read_registry(registry_path).assignments["BT2-001"] == 2
    written = json.loads(cards_path.read_text(encoding="utf-8"))
    assert written["BT2-001"]["index"] == 2
    assert written["BT2-001"]["norm_id"] == 2 / 20_000


def test_cli_dry_run_is_write_free(tmp_path: Path):
    cards_path = tmp_path / "cards.json"
    registry_path = tmp_path / "card_registry.json"
    original = json.dumps({"BT1-001": _card("BT1-001", index=1)})
    cards_path.write_text(original, encoding="utf-8")

    assert command.main(
        [
            "--dry-run",
            "--cards-json",
            str(cards_path),
            "--registry",
            str(registry_path),
        ]
    ) == 0
    assert cards_path.read_text(encoding="utf-8") == original
    assert not registry_path.exists()


def test_cli_preserves_offline_flag_but_rejects_retired_unsafe_modes(
    tmp_path: Path, capsys
):
    cards_path = tmp_path / "cards.json"
    registry_path = tmp_path / "card_registry.json"
    cards_path.write_text(
        json.dumps({"BT1-001": _card("BT1-001", index=1)}), encoding="utf-8"
    )

    assert command.main(
        [
            "--offline",
            "--cards-json",
            str(cards_path),
            "--registry",
            str(registry_path),
        ]
    ) == 0
    assert "always network-free" in capsys.readouterr().err
    assert command.main(["--sets", "BT1"]) == 2
    assert command.main(["--force"]) == 2
    assert command.main(["--capacity", "30000"]) == 2
