"""Checked source-lane classification for direct DSL card directories."""

from __future__ import annotations

import json
from dataclasses import dataclass
from enum import StrEnum
from pathlib import Path
from typing import Mapping

from .errors import ValidationError


class SourceLane(StrEnum):
    PRODUCTION = "production"
    EXAMPLE = "example"
    TEST_FIXTURE = "test_fixture"


@dataclass(frozen=True, slots=True)
class SourceLaneManifest:
    schema_version: int
    lanes: Mapping[SourceLane, tuple[str, ...]]

    def __post_init__(self) -> None:
        if self.schema_version != 1:
            raise ValidationError("unsupported source-lane manifest schema version")
        if set(self.lanes) != set(SourceLane):
            raise ValidationError(
                "source-lane manifest must declare every lane exactly once",
                details={"lanes": sorted(str(lane) for lane in self.lanes)},
            )
        owners: dict[str, SourceLane] = {}
        for lane, directories in self.lanes.items():
            for directory in directories:
                if not directory or Path(directory).name != directory:
                    raise ValidationError(
                        "source-lane entries must be direct directory names",
                        details={"lane": lane.value, "directory": directory},
                    )
                prior = owners.get(directory)
                if prior is not None:
                    raise ValidationError(
                        "source directory appears in overlapping lanes",
                        details={
                            "directory": directory,
                            "lanes": [prior.value, lane.value],
                        },
                    )
                owners[directory] = lane

    @property
    def directory_lanes(self) -> dict[str, SourceLane]:
        return {
            directory: lane
            for lane, directories in self.lanes.items()
            for directory in directories
        }

    def validate_tree(self, cards_root: Path) -> None:
        actual = {path.name for path in cards_root.iterdir() if path.is_dir()}
        declared = set(self.directory_lanes)
        if actual != declared:
            raise ValidationError(
                "DSL source directories do not reconcile with source-lane manifest",
                path=cards_root,
                details={
                    "unclassified_directories": sorted(actual - declared),
                    "missing_directories": sorted(declared - actual),
                },
            )

    def lane_for(self, path: Path, *, cards_root: Path) -> SourceLane:
        try:
            relative = path.resolve().relative_to(cards_root.resolve())
        except ValueError as exc:
            raise ValidationError("DSL path is outside the card source tree", path=path) from exc
        if len(relative.parts) < 2:
            raise ValidationError("DSL file must belong to a classified direct directory", path=path)
        directory = relative.parts[0]
        try:
            return self.directory_lanes[directory]
        except KeyError as exc:
            raise ValidationError("DSL path belongs to an unclassified source directory", path=path) from exc

    def yaml_paths(self, cards_root: Path, lane: SourceLane) -> tuple[Path, ...]:
        self.validate_tree(cards_root)
        paths = [
            path
            for directory in self.lanes[lane]
            for path in (cards_root / directory).rglob("*.yaml")
        ]
        paths.extend(
            path
            for directory in self.lanes[lane]
            for path in (cards_root / directory).rglob("*.yml")
        )
        return tuple(sorted(set(paths)))


def load_source_lane_manifest(path: Path) -> SourceLaneManifest:
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise ValidationError("source-lane manifest cannot be loaded", path=path) from exc
    raw_lanes = payload.get("lanes")
    if not isinstance(raw_lanes, Mapping):
        raise ValidationError("source-lane manifest lanes must be an object", path=path)
    try:
        lanes = {
            SourceLane(key): tuple(value)
            for key, value in raw_lanes.items()
        }
    except (ValueError, TypeError) as exc:
        raise ValidationError("source-lane manifest contains an unknown lane or shape", path=path) from exc
    return SourceLaneManifest(schema_version=payload.get("schema_version"), lanes=lanes)
