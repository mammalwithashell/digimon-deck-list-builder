from __future__ import annotations

from tools.impact_scope import scope_for_changed_paths


def fixture_index() -> dict:
    return {
        "schema_version": 1,
        "cards": {
            "BT1-001": {
                "path": "bt1/BT1-001.yaml",
                "verbs": ["draw"],
                "predicates": ["name_contains"],
                "timings": ["on_play"],
            },
            "BT1-002": {
                "path": "bt1/BT1-002.yaml",
                "verbs": ["draw", "delete_permanent"],
                "predicates": [],
                "timings": ["on_deletion"],
            },
        },
        "verbs": {
            "delete_permanent": ["BT1-002"],
            "draw": ["BT1-001", "BT1-002"],
        },
        "predicates": {"name_contains": ["BT1-001"]},
        "timings": {"on_play": ["BT1-001"], "on_deletion": ["BT1-002"]},
    }


def test_scope_for_mapped_lowering_file_emits_consuming_cards():
    result = scope_for_changed_paths(
        ["code/digimon-engine/src/dsl_cards/step/draw.rs"],
        dsl_index=fixture_index(),
        lowering_map={
            "code/digimon-engine/src/dsl_cards/step/draw.rs": ["draw"],
        },
    )

    assert not result.full_suite_required
    assert result.verbs == ["draw"]
    assert result.cards == ["BT1-001", "BT1-002"]
    assert result.cards_behavioral_filter == "BT1-001|BT1-002"
    assert result.side_binaries == ["dsl"]


def test_scope_for_card_yaml_emits_the_changed_card():
    result = scope_for_changed_paths(
        ["code/digimon-engine/cards/bt1/BT1-001.yaml"],
        dsl_index=fixture_index(),
        lowering_map={},
    )

    assert not result.full_suite_required
    assert result.cards == ["BT1-001"]
    assert result.cards_behavioral_filter == "BT1-001"


def test_scope_for_unmapped_engine_file_escalates_to_full_suite():
    result = scope_for_changed_paths(
        ["code/digimon-engine/src/combat.rs"],
        dsl_index=fixture_index(),
        lowering_map={},
    )

    assert result.full_suite_required
    assert result.cards_behavioral_filter == ""
    assert result.reasons == [
        "code/digimon-engine/src/combat.rs is an unmapped engine/core file"
    ]
