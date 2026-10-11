"""Consistent fixed-path reads for committed card-data artifacts.

Before the canonical producer cutover, repositories without a revision marker
remain readable in an explicit legacy-unversioned mode. A terminal rollback
to the empty pre-cutover revision also returns to that mode while retaining
its audit journal. Every other metadata-present state uses the strict journal /
hash / marker-before-and-after protocol and never falls back to loose files.
"""

from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Iterable, Mapping

from data_paths import (
    CARD_DATA_REVISION,
    CARD_TRANSACTION_JOURNAL,
    CARDS_JSON,
    REPO_ROOT,
)
from tools.card_data.errors import ValidationError
from tools.card_data.transaction import (
    RecoveryRequiredError,
    committed_revision_protocol_active,
    read_consistent_revision,
)


LEGACY_UNVERSIONED_REVISION = "legacy-unversioned"


@dataclass(frozen=True, slots=True)
class LoadedCardDataArtifact:
    value: Any
    revision: str
    relative_path: str
    raw_bytes: bytes


def _relative_to_repository(path: Path, repository_root: Path) -> str:
    try:
        return path.resolve().relative_to(repository_root.resolve()).as_posix()
    except ValueError as exc:
        raise ValidationError(
            "card-data artifact must stay within the repository",
            path=path,
        ) from exc


def load_json_artifact(
    path: Path,
    *,
    repository_root: Path = REPO_ROOT,
    marker_path: Path = CARD_DATA_REVISION,
    journal_path: Path = CARD_TRANSACTION_JOURNAL,
    require_committed_revision: bool = False,
) -> LoadedCardDataArtifact:
    """Load one JSON artifact without accepting a mixed publication.

    The legacy fallback is available before cutover and after a completed
    rollback to the empty pre-cutover revision. Any other journal without a
    marker is a recovery condition, not a reason to read loose files.
    """

    return load_json_artifacts(
        (path,),
        repository_root=repository_root,
        marker_path=marker_path,
        journal_path=journal_path,
        require_committed_revision=require_committed_revision,
    )[path]


def load_json_artifacts(
    paths: Iterable[Path],
    *,
    repository_root: Path = REPO_ROOT,
    marker_path: Path = CARD_DATA_REVISION,
    journal_path: Path = CARD_TRANSACTION_JOURNAL,
    require_committed_revision: bool = False,
) -> Mapping[Path, LoadedCardDataArtifact]:
    """Load several fixed paths from one before/after-verified revision."""

    ordered_paths = tuple(dict.fromkeys(Path(path) for path in paths))
    if not ordered_paths:
        raise ValidationError("at least one card-data artifact is required")
    relative_by_path = {
        path: _relative_to_repository(path, repository_root) for path in ordered_paths
    }
    if committed_revision_protocol_active(
        marker_path=marker_path,
        journal_path=journal_path,
    ):
        loaders = {
            relative: lambda raw: (json.loads(raw.decode("utf-8")), raw)
            for relative in relative_by_path.values()
        }
        result = read_consistent_revision(
            repository_root,
            marker_path=marker_path,
            journal_path=journal_path,
            loaders=loaders,
        )
        loaded: dict[Path, LoadedCardDataArtifact] = {}
        for path, relative in relative_by_path.items():
            if relative not in result.values:
                raise RecoveryRequiredError(
                    "committed card-data marker does not declare the requested artifact",
                    path=path,
                    details={"relative_path": relative, "revision": result.revision},
                )
            value, raw = result.values[relative]
            loaded[path] = LoadedCardDataArtifact(value, result.revision, relative, raw)
        return loaded

    if require_committed_revision:
        raise RecoveryRequiredError(
            "committed card-data revision marker is required",
            path=marker_path,
        )
    loaded = {}
    for path, relative in relative_by_path.items():
        try:
            raw = path.read_bytes()
            value = json.loads(raw.decode("utf-8"))
        except OSError as exc:
            raise ValidationError("card-data artifact is unavailable", path=path) from exc
        except (UnicodeDecodeError, json.JSONDecodeError) as exc:
            raise ValidationError("card-data artifact is not valid JSON", path=path) from exc
        loaded[path] = LoadedCardDataArtifact(value, LEGACY_UNVERSIONED_REVISION, relative, raw)
    return loaded


def load_cards_json(
    *,
    repository_root: Path = REPO_ROOT,
    path: Path = CARDS_JSON,
    marker_path: Path = CARD_DATA_REVISION,
    journal_path: Path = CARD_TRANSACTION_JOURNAL,
    require_committed_revision: bool = False,
) -> LoadedCardDataArtifact:
    loaded = load_json_artifact(
        path,
        repository_root=repository_root,
        marker_path=marker_path,
        journal_path=journal_path,
        require_committed_revision=require_committed_revision,
    )
    if not isinstance(loaded.value, dict):
        raise ValidationError("cards.json must be an object", path=path)
    return loaded
