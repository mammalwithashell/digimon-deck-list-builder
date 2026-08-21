"""Tests for `tools.clause_coverage.coverage` against synthetic DCGO recordings.

Uses small hand-written `.jsonl` fixtures shaped like real DCGO recordings
(`docs/DCGO_RECORDING_SCHEMA.md`) rather than the real (read-only, external)
`D:\\dcgo-build\\vb-corpus2\\` corpus, so these tests are self-contained and
don't depend on machine-local data.
"""

from __future__ import annotations

import json

from tools.clause_coverage.coverage import build_coverage_report

DECK_CARD_IDS = ["A-001", "A-002", "A-003"]


def _clauses(*ids_with_zones: tuple[str, str]) -> list[dict]:
    """Build a minimal synthetic clause list: [(card_id, zone), ...]."""
    return [
        {
            "id": f"{cid}#{zone}#0",
            "card_id": cid,
            "zone": zone,
            "label": "test",
            "kind": "untimed",
            "timings": [],
            "keyword": None,
            "text": "x",
            "source": "cards_json",
            "image_path": None,
        }
        for cid, zone in ids_with_zones
    ]


def _write_recording(path, rows: list[dict]) -> None:
    with open(path, "w", encoding="utf-8") as f:
        for row in rows:
            f.write(json.dumps(row) + "\n")


def test_clause_level_is_unknown_and_counts_add_up_to_denominator(tmp_path):
    clauses = _clauses(("A-001", "effect"), ("A-001", "security"), ("A-002", "effect"))
    _write_recording(
        tmp_path / "game-000.jsonl",
        [
            {"type": "game_start", "game_id": "g1"},
            {"type": "action", "step": 0, "actor": 0, "action_id": 1, "phase": "Main", "board_p0": [], "board_p1": []},
            {"type": "game_end", "winner": 0, "reason": "concede", "total_steps": 1},
        ],
    )

    report = build_coverage_report(clauses, DECK_CARD_IDS, tmp_path)

    assert report["clause_level"]["status"] == "UNKNOWN_FOR_ALL"
    by_status = report["clause_level"]["by_status"]
    assert by_status == {"UNKNOWN": len(clauses)}
    assert sum(by_status.values()) == report["denominator"]["total_clauses"] == len(clauses)
    assert set(report["clause_level"]["unknown_clause_ids"]) == {c["id"] for c in clauses}


def test_card_level_derives_on_board_from_board_snapshots(tmp_path):
    clauses = _clauses(("A-001", "effect"))
    _write_recording(
        tmp_path / "game-000.jsonl",
        [
            {"type": "game_start", "game_id": "g1"},
            {
                "type": "action",
                "step": 0,
                "actor": 0,
                "action_id": 1,
                "phase": "Main",
                "board_p0": ["A-001"],
                "board_p1": [],
            },
            {
                "type": "selection",
                "step": 1,
                "actor": 0,
                "prompt": "SelectPermanentEffect",
                "targets": [],
                "board_p0": ["A-001", "A-002"],
                "board_p1": [],
            },
            {"type": "game_end", "winner": 0, "reason": "concede", "total_steps": 2},
        ],
    )

    report = build_coverage_report(clauses, DECK_CARD_IDS, tmp_path)

    card_level = report["card_level"]
    assert card_level["on_board"] == ["A-001", "A-002"]
    assert card_level["never_on_board"] == ["A-003"]
    assert card_level["on_board_count"] == 2
    assert card_level["deck_cards_total"] == 3


def test_prompt_level_counts_selection_prompts(tmp_path):
    clauses = _clauses(("A-001", "effect"))
    _write_recording(
        tmp_path / "game-000.jsonl",
        [
            {"type": "game_start", "game_id": "g1"},
            {"type": "selection", "step": 0, "actor": 0, "prompt": "SelectHandEffect", "card_ids": []},
            {"type": "selection", "step": 1, "actor": 1, "prompt": "SelectHandEffect", "card_ids": []},
            {"type": "selection", "step": 2, "actor": 0, "prompt": "OptionalSkill", "bool_value": True},
            {"type": "game_end", "winner": 0, "reason": "concede", "total_steps": 3},
        ],
    )

    report = build_coverage_report(clauses, DECK_CARD_IDS, tmp_path)

    prompt_level = report["prompt_level"]
    assert prompt_level["counts"] == {"SelectHandEffect": 2, "OptionalSkill": 1}
    assert prompt_level["distinct_prompts"] == 2
    assert prompt_level["total_selection_rows"] == 3


def test_schema_note_detects_absence_of_card_id_and_action_detail(tmp_path):
    clauses = _clauses(("A-001", "effect"))
    _write_recording(
        tmp_path / "game-000.jsonl",
        [
            {"type": "game_start", "game_id": "g1"},
            {"type": "action", "step": 0, "actor": 0, "action_id": 1, "phase": "Main", "board_p0": [], "board_p1": []},
            {"type": "game_end", "winner": 0, "reason": "concede", "total_steps": 1},
        ],
    )

    report = build_coverage_report(clauses, DECK_CARD_IDS, tmp_path)
    assert report["corpus"]["action_rows_with_card_id"] == 0
    assert report["corpus"]["action_detail_rows"] == 0
    assert "predates" in report["corpus"]["schema_note"]


def test_schema_note_detects_presence_of_card_id_and_action_detail(tmp_path):
    clauses = _clauses(("A-001", "effect"))
    _write_recording(
        tmp_path / "game-000.jsonl",
        [
            {"type": "game_start", "game_id": "g1"},
            {
                "type": "action",
                "step": 0,
                "actor": 0,
                "action_id": 1,
                "phase": "Main",
                "card_id": "A-001",
                "board_p0": [],
                "board_p1": [],
            },
            {"type": "action_detail", "step": 0, "actor": 0, "card_id": "A-001", "cost_paid": 3},
            {"type": "game_end", "winner": 0, "reason": "concede", "total_steps": 1},
        ],
    )

    report = build_coverage_report(clauses, DECK_CARD_IDS, tmp_path)
    assert report["corpus"]["action_rows_with_card_id"] == 1
    assert report["corpus"]["action_detail_rows"] == 1
    assert "predates" not in report["corpus"]["schema_note"]


def test_malformed_lines_are_skipped_and_counted_not_fatal(tmp_path):
    clauses = _clauses(("A-001", "effect"))
    path = tmp_path / "game-000.jsonl"
    with open(path, "w", encoding="utf-8") as f:
        f.write(json.dumps({"type": "game_start", "game_id": "g1"}) + "\n")
        f.write("{not valid json\n")
        f.write(json.dumps({"type": "game_end", "winner": 0, "reason": "concede", "total_steps": 0}) + "\n")

    report = build_coverage_report(clauses, DECK_CARD_IDS, tmp_path)
    assert report["corpus"]["malformed_lines_skipped"] == 1
    assert report["corpus"]["games"] == 1


def test_multiple_recording_files_are_all_read(tmp_path):
    clauses = _clauses(("A-001", "effect"))
    for i in range(3):
        _write_recording(
            tmp_path / f"game-{i:03d}.jsonl",
            [
                {"type": "game_start", "game_id": f"g{i}"},
                {"type": "game_end", "winner": 0, "reason": "concede", "total_steps": 0},
            ],
        )
    report = build_coverage_report(clauses, DECK_CARD_IDS, tmp_path)
    assert report["corpus"]["games"] == 3
    assert len(report["corpus"]["files"]) == 3
