"""Tests for `tools.clause_coverage.coverage` against synthetic DCGO recordings.

Uses small hand-written `.jsonl` fixtures shaped like real DCGO recordings
(`docs/DCGO_RECORDING_SCHEMA.md`) rather than the real (read-only, external)
`D:\\dcgo-build\\vb-corpus3\\` corpus, so these tests are self-contained and
don't depend on machine-local data. `effect_activation` row fixtures below
are shaped after that schema's "Effect-activation row" section, and the
matching behavior they exercise was validated against the real corpus (see
`.superpowers/sdd/clause-coverage-v2-report.md`) before being pinned here.
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


def _timing_clause(card_id: str, zone: str, idx: int, timings: list[str], text: str) -> dict:
    return {
        "id": f"{card_id}#{zone}#{idx}",
        "card_id": card_id,
        "zone": zone,
        "label": "test",
        "kind": "timing",
        "timings": timings,
        "keyword": None,
        "text": text,
        "source": "cards_json",
        "image_path": None,
    }


def _keyword_clause(card_id: str, zone: str, idx: int, keyword: str, text: str = "") -> dict:
    return {
        "id": f"{card_id}#{zone}#{idx}",
        "card_id": card_id,
        "zone": zone,
        "label": "test",
        "kind": "keyword",
        "timings": [],
        "keyword": keyword,
        "text": text,
        "source": "cards_json",
        "image_path": None,
    }


def _activation_row(
    step: int,
    actor: int,
    card_id: str,
    effect_description: str,
    *,
    executed: bool | None = True,
    is_optional: bool = False,
    phase: str = "Main",
) -> dict:
    return {
        "type": "effect_activation",
        "step": step,
        "actor": actor,
        "card_id": card_id,
        "effect_name": "Eff",
        "effect_description": effect_description,
        "is_optional": is_optional,
        "executed": executed,
        "phase": phase,
    }


def _write_recording(path, rows: list[dict]) -> None:
    with open(path, "w", encoding="utf-8") as f:
        for row in rows:
            f.write(json.dumps(row) + "\n")


def test_clause_level_all_not_fired_when_corpus_has_no_activation_rows(tmp_path):
    """With zero effect_activation rows, ordinary (non-blind-spot-kind)
    clauses are NOT_FIRED, not the retired UNKNOWN status -- and the
    by_status counts still sum to exactly the denominator, with no clause
    silently uncounted."""
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

    clause_level = report["clause_level"]
    assert clause_level["by_status"] == {
        "FIRED": 0,
        "OFFERED_ONLY": 0,
        "NOT_FIRED": len(clauses),
        "UNOBSERVABLE": 0,
    }
    assert sum(clause_level["by_status"].values()) == clause_level["total_clauses"] == len(clauses)
    assert set(clause_level["clause_ids_by_status"]["NOT_FIRED"]) == {c["id"] for c in clauses}


def test_activation_with_executed_true_marks_clause_fired(tmp_path):
    clauses = [_timing_clause("A-001", "effect", 0, ["On Play"], "Draw 1 card.")]
    _write_recording(
        tmp_path / "game-000.jsonl",
        [
            {"type": "game_start", "game_id": "g1"},
            _activation_row(0, 0, "A-001", "[On Play] Draw 1 card.", executed=True),
            {"type": "game_end", "winner": 0, "reason": "concede", "total_steps": 1},
        ],
    )

    report = build_coverage_report(clauses, ["A-001"], tmp_path)

    clause_level = report["clause_level"]
    assert clause_level["clause_ids_by_status"]["FIRED"] == ["A-001#effect#0"]
    assert clause_level["by_status"] == {"FIRED": 1, "OFFERED_ONLY": 0, "NOT_FIRED": 0, "UNOBSERVABLE": 0}


def test_clause_with_only_declined_activations_is_offered_only_not_fired(tmp_path):
    """A 'you may' effect offered and declined every time (is_optional:
    true, executed: false) must land in OFFERED_ONLY, never FIRED -- this
    distinction is the whole reason the recorder tracks both fields."""
    clauses = [_timing_clause("A-001", "effect", 0, ["On Deletion"], "You may play 1 card from your hand.")]
    _write_recording(
        tmp_path / "game-000.jsonl",
        [
            {"type": "game_start", "game_id": "g1"},
            _activation_row(
                0, 0, "A-001", "[On Deletion] You may play 1 card from your hand.", executed=False, is_optional=True
            ),
            _activation_row(
                1, 1, "A-001", "[On Deletion] You may play 1 card from your hand.", executed=False, is_optional=True
            ),
            {"type": "game_end", "winner": 0, "reason": "concede", "total_steps": 2},
        ],
    )

    report = build_coverage_report(clauses, ["A-001"], tmp_path)

    clause_level = report["clause_level"]
    assert clause_level["clause_ids_by_status"]["OFFERED_ONLY"] == ["A-001#effect#0"]
    assert clause_level["by_status"]["FIRED"] == 0


def test_unmatched_activation_is_counted_and_listed_not_dropped(tmp_path):
    """An activation whose printed text corresponds to no clause on its
    card is a denominator-bug signal and must be surfaced loudly, not
    silently discarded -- and must not spuriously mark an unrelated clause
    on the same card as fired."""
    clauses = [_timing_clause("A-001", "effect", 0, ["On Play"], "Draw 1 card.")]
    _write_recording(
        tmp_path / "game-000.jsonl",
        [
            {"type": "game_start", "game_id": "g1"},
            _activation_row(0, 0, "A-001", "[Security] Place this card in the battle area.", executed=True),
            {"type": "game_end", "winner": 0, "reason": "concede", "total_steps": 1},
        ],
    )

    report = build_coverage_report(clauses, ["A-001"], tmp_path)

    clause_level = report["clause_level"]
    assert clause_level["unmatched_activation_pair_count"] == 1
    assert clause_level["unmatched_activation_row_count"] == 1
    entry = clause_level["unmatched_activations"][0]
    assert entry["card_id"] == "A-001"
    assert entry["effect_description"] == "[Security] Place this card in the battle area."
    assert entry["occurrences"] == 1
    # The unrelated On Play clause is untouched -- NOT_FIRED, not FIRED.
    assert clause_level["clause_ids_by_status"]["NOT_FIRED"] == ["A-001#effect#0"]
    assert clause_level["clause_ids_by_status"]["FIRED"] == []


def test_repeated_unmatched_activation_rows_are_deduplicated_with_occurrence_count(tmp_path):
    clauses = [_timing_clause("A-001", "effect", 0, ["On Play"], "Draw 1 card.")]
    _write_recording(
        tmp_path / "game-000.jsonl",
        [
            {"type": "game_start", "game_id": "g1"},
            _activation_row(0, 0, "A-001", "[Security] Place this card in the battle area.", executed=True),
            _activation_row(1, 0, "A-001", "[Security] Place this card in the battle area.", executed=True),
            _activation_row(2, 0, "A-001", "[Security] Place this card in the battle area.", executed=True),
            {"type": "game_end", "winner": 0, "reason": "concede", "total_steps": 3},
        ],
    )

    report = build_coverage_report(clauses, ["A-001"], tmp_path)

    clause_level = report["clause_level"]
    assert clause_level["unmatched_activation_pair_count"] == 1
    assert clause_level["unmatched_activation_row_count"] == 3
    assert clause_level["unmatched_activations"][0]["occurrences"] == 3


def test_keyword_clause_is_unobservable_not_not_fired(tmp_path):
    """The hook does not observe angle-bracket keyword clauses (persistent/
    passive, not activated) -- absence of any matching activation must
    report UNOBSERVABLE, never the false claim NOT_FIRED."""
    clauses = [_keyword_clause("A-001", "effect", 0, "Piercing", "(reminder text)")]
    _write_recording(
        tmp_path / "game-000.jsonl",
        [
            {"type": "game_start", "game_id": "g1"},
            {"type": "game_end", "winner": 0, "reason": "concede", "total_steps": 0},
        ],
    )

    report = build_coverage_report(clauses, ["A-001"], tmp_path)

    clause_level = report["clause_level"]
    assert clause_level["clause_ids_by_status"]["UNOBSERVABLE"] == ["A-001#effect#0"]
    assert clause_level["clause_ids_by_status"]["NOT_FIRED"] == []
    assert clause_level["unobservable_breakdown"] == {"keyword_kind": 1}


def test_cost_line_timing_clause_is_unobservable(tmp_path):
    """[Digivolve] and friends are cost/condition lines, never dispatched
    as an activated effect -- same UNOBSERVABLE treatment as keywords."""
    clauses = [_timing_clause("A-001", "effect", 0, ["Digivolve"], "Lv.3: Cost 2")]
    _write_recording(
        tmp_path / "game-000.jsonl",
        [
            {"type": "game_start", "game_id": "g1"},
            {"type": "game_end", "winner": 0, "reason": "concede", "total_steps": 0},
        ],
    )

    report = build_coverage_report(clauses, ["A-001"], tmp_path)

    clause_level = report["clause_level"]
    assert clause_level["clause_ids_by_status"]["UNOBSERVABLE"] == ["A-001#effect#0"]
    assert clause_level["unobservable_breakdown"] == {"cost_line_kind": 1}


def test_keyword_clause_with_real_matched_evidence_reports_fired_not_unobservable(tmp_path):
    """UNOBSERVABLE is the fallback for a blind-spot-kind clause when there
    is no evidence -- but if this corpus happens to catch a genuine
    activation for one (confirmed in the real vb-corpus3 corpus, e.g.
    P-240's native <Piercing>), that real evidence must win rather than
    being discarded in favor of the coarser kind-based rule."""
    clauses = [_keyword_clause("A-001", "effect", 0, "Raid", "(reminder text)")]
    _write_recording(
        tmp_path / "game-000.jsonl",
        [
            {"type": "game_start", "game_id": "g1"},
            _activation_row(0, 0, "A-001", "<Raid> (reminder text)", executed=True),
            {"type": "game_end", "winner": 0, "reason": "concede", "total_steps": 1},
        ],
    )

    report = build_coverage_report(clauses, ["A-001"], tmp_path)

    clause_level = report["clause_level"]
    assert clause_level["clause_ids_by_status"]["FIRED"] == ["A-001#effect#0"]
    assert clause_level["clause_ids_by_status"]["UNOBSERVABLE"] == []


def test_normalization_matches_across_whitespace_and_unicode_punctuation(tmp_path):
    """DCGO's own rendering and this package's extracted text use
    different bullet glyphs (・ vs •), \\r\\n vs \\n, and incidental
    whitespace -- normalization must bridge those so the match still
    lands."""
    clauses = [
        _timing_clause(
            "A-001",
            "effect",
            0,
            ["On Play"],
            "Activate 1 of the effects below:\n\u30fbDraw a card.\n\u30fbGain 1 memory.",
        )
    ]
    _write_recording(
        tmp_path / "game-000.jsonl",
        [
            {"type": "game_start", "game_id": "g1"},
            _activation_row(
                0,
                0,
                "A-001",
                "[On Play] Activate 1 of the effects below:\r\n\u2022 Draw a card.\r\n\u2022  Gain 1 memory.",
                executed=True,
            ),
            {"type": "game_end", "winner": 0, "reason": "concede", "total_steps": 1},
        ],
    )

    report = build_coverage_report(clauses, ["A-001"], tmp_path)

    clause_level = report["clause_level"]
    assert clause_level["clause_ids_by_status"]["FIRED"] == ["A-001#effect#0"]
    assert clause_level["unmatched_activation_pair_count"] == 0


def test_status_counts_sum_to_denominator_across_all_four_statuses(tmp_path):
    clauses = [
        _timing_clause("A-001", "effect", 0, ["On Play"], "Draw 1 card."),  # -> FIRED
        _timing_clause("A-001", "effect", 1, ["On Deletion"], "You may play 1 card."),  # -> OFFERED_ONLY
        _timing_clause("A-002", "effect", 0, ["Main"], "Do a thing nobody observed."),  # -> NOT_FIRED
        _keyword_clause("A-002", "effect", 1, "Blocker"),  # -> UNOBSERVABLE (keyword)
        _timing_clause("A-002", "effect", 2, ["Digivolve"], "Lv.3: Cost 2"),  # -> UNOBSERVABLE (cost-line)
    ]
    _write_recording(
        tmp_path / "game-000.jsonl",
        [
            {"type": "game_start", "game_id": "g1"},
            _activation_row(0, 0, "A-001", "[On Play] Draw 1 card.", executed=True),
            _activation_row(1, 0, "A-001", "[On Deletion] You may play 1 card.", executed=False, is_optional=True),
            {"type": "game_end", "winner": 0, "reason": "concede", "total_steps": 2},
        ],
    )

    report = build_coverage_report(clauses, ["A-001", "A-002"], tmp_path)

    clause_level = report["clause_level"]
    assert clause_level["by_status"] == {"FIRED": 1, "OFFERED_ONLY": 1, "NOT_FIRED": 1, "UNOBSERVABLE": 2}
    assert sum(clause_level["by_status"].values()) == clause_level["total_clauses"] == len(clauses)


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
