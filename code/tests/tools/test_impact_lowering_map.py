from __future__ import annotations

import json
from pathlib import Path

from tools.impact_lowering_map import check_lowering_map, load_lowering_map


def write_map(path: Path, files: dict[str, list[str]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps({"schema_version": 1, "files": files}, indent=2) + "\n",
        encoding="utf-8",
        newline="\n",
    )


def test_lowering_map_check_passes_for_complete_map(tmp_path: Path):
    repo = tmp_path
    (repo / "engine" / "memory.rs").parent.mkdir(parents=True)
    (repo / "engine" / "memory.rs").write_text("// lowering\n", encoding="utf-8")
    path = repo / "qa" / "impact" / "lowering_verb_map.json"
    write_map(path, {"engine/memory.rs": ["gain_memory", "lose_memory"]})

    result = check_lowering_map(
        load_lowering_map(path),
        authoritative_verbs={"gain_memory", "lose_memory"},
        repo=repo,
    )

    assert result.ok
    assert result.missing_verbs == []
    assert result.unknown_verbs == []


def test_lowering_map_check_fails_on_missing_unknown_and_missing_file(tmp_path: Path):
    repo = tmp_path
    path = repo / "qa" / "impact" / "lowering_verb_map.json"
    write_map(path, {"engine/missing.rs": ["gain_memory", "unknown_verb"]})

    result = check_lowering_map(
        load_lowering_map(path),
        authoritative_verbs={"gain_memory", "lose_memory"},
        repo=repo,
    )

    assert not result.ok
    assert result.missing_verbs == ["lose_memory"]
    assert result.unknown_verbs == ["unknown_verb"]
    assert result.missing_files == ["engine/missing.rs"]
