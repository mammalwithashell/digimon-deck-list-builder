"""Network-free adapters joining campaign manifest input dimensions."""

from __future__ import annotations

import json
import hashlib
import re
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Iterable, Mapping

from .blockers import BlockerRegistry, blocker_registry_from_payload, load_blocker_registry
from .errors import ValidationError
from .models import CARD_ID_RE
from .pool_impact import find_deck_usage, load_yaml_card_ids
from .serialization import content_digest, natural_card_id_key, sha256_hex
from .source_lanes import SourceLane, load_source_lane_manifest


_IGNORE_RE = re.compile(r"^\s*#\s*\[\s*ignore(?:\s*=|\s*\])")


def is_real_card_id(card_id: str) -> bool:
    match = CARD_ID_RE.fullmatch(card_id.upper()) if isinstance(card_id, str) else None
    return bool(match and match.group("prefix") not in {"TEST", "TST", "TOKEN"})


def load_canonical_real_cards(
    path: Path, *, payload: Mapping[str, Any] | None = None
) -> dict[str, Mapping[str, Any]]:
    payload = payload if payload is not None else json.loads(path.read_text(encoding="utf-8"))
    raw_cards = payload.get("cards", payload)
    if not isinstance(raw_cards, Mapping):
        raise ValidationError("canonical cards artifact must contain a card mapping", path=path)
    cards: dict[str, Mapping[str, Any]] = {}
    for card_id, record in raw_cards.items():
        if not is_real_card_id(card_id):
            continue
        if not isinstance(record, Mapping):
            raise ValidationError("canonical card record must be an object", card_id=card_id)
        admission = record.get("admission", "admitted")
        if admission == "provisional":
            continue
        if admission not in {"admitted", "retired"}:
            raise ValidationError(
                "canonical card has unknown admission state",
                card_id=card_id,
                details={"admission": admission},
            )
        cards[card_id] = record
    return dict(sorted(cards.items(), key=lambda item: natural_card_id_key(item[0])))


@dataclass(frozen=True, slots=True)
class YamlInventory:
    production_paths: Mapping[str, tuple[str, ...]]
    example_paths: Mapping[str, tuple[str, ...]]
    test_fixture_paths: Mapping[str, tuple[str, ...]]
    path_digests: Mapping[str, str]

    @property
    def duplicate_production_ids(self) -> dict[str, tuple[str, ...]]:
        return {
            card_id: paths
            for card_id, paths in self.production_paths.items()
            if len(paths) > 1
        }


def load_yaml_inventory(cards_root: Path, lane_manifest_path: Path) -> YamlInventory:
    manifest = load_source_lane_manifest(lane_manifest_path)
    manifest.validate_tree(cards_root)
    by_lane: dict[SourceLane, dict[str, list[str]]] = {}
    digests: dict[str, str] = {}
    for lane in SourceLane:
        paths = manifest.yaml_paths(cards_root, lane)
        declarations = load_yaml_card_ids(paths)
        normalized: dict[str, list[str]] = {}
        for card_id, raw_paths in declarations.items():
            relative_paths = [Path(path).relative_to(cards_root).as_posix() for path in raw_paths]
            normalized[card_id] = relative_paths
            for relative in relative_paths:
                digests[relative] = sha256_hex((cards_root / relative).read_bytes())
        by_lane[lane] = normalized
    return YamlInventory(
        production_paths={key: tuple(value) for key, value in sorted(by_lane[SourceLane.PRODUCTION].items())},
        example_paths={key: tuple(value) for key, value in sorted(by_lane[SourceLane.EXAMPLE].items())},
        test_fixture_paths={key: tuple(value) for key, value in sorted(by_lane[SourceLane.TEST_FIXTURE].items())},
        path_digests=dict(sorted(digests.items())),
    )


def load_runtime_registry_ids(
    path: Path, *, payload: Any | None = None
) -> tuple[str, ...]:
    payload = payload if payload is not None else json.loads(path.read_text(encoding="utf-8"))
    if isinstance(payload, list):
        # ``digimon-engine-cli pool`` emits the raw snapshot as a JSON array.
        card_ids = payload
    elif isinstance(payload, Mapping):
        card_ids = payload.get("card_ids")
    else:
        card_ids = None
    if not isinstance(card_ids, list) or not all(isinstance(item, str) for item in card_ids):
        raise ValidationError(
            "runtime registry artifact must be an array or contain a card_ids array",
            path=path,
        )
    if len(card_ids) != len(set(card_ids)):
        raise ValidationError("runtime registry artifact contains duplicate card IDs", path=path)
    return tuple(sorted(card_ids, key=natural_card_id_key))


@dataclass(frozen=True, slots=True)
class VerdictInventory:
    rows: Mapping[str, Mapping[str, Any]]
    current_ids: frozenset[str]
    accepted_ids: frozenset[str]
    partial_ids: frozenset[str]


def load_verdict_inventory(
    path: Path, *, payload: Mapping[str, Any] | None = None
) -> VerdictInventory:
    payload = payload if payload is not None else json.loads(path.read_text(encoding="utf-8"))
    rows = payload.get("cards")
    if not isinstance(rows, Mapping):
        raise ValidationError("verdict ledger must contain a cards object", path=path)
    current = frozenset(rows)
    accepted = frozenset(
        card_id for card_id, row in rows.items() if row.get("status") in {"IMPLEMENTED", "AUDITED-OK"}
    )
    partial = frozenset(
        card_id for card_id, row in rows.items() if row.get("status") in {"PARTIAL", "INCOMPLETE", "BLOCKED-PARTIAL"}
    )
    return VerdictInventory(rows=rows, current_ids=current, accepted_ids=accepted, partial_ids=partial)


@dataclass(frozen=True, slots=True)
class EvidenceInventory:
    ledger_rows: tuple[Mapping[str, Any], ...]
    rows_by_card: Mapping[str, tuple[Mapping[str, Any], ...]]
    passing_current_ids: frozenset[str]
    stale_or_nonpassing_ids: frozenset[str]


def load_evidence_inventory(
    path: Path, *, payload: Mapping[str, Any] | None = None
) -> EvidenceInventory:
    payload = payload if payload is not None else json.loads(path.read_text(encoding="utf-8"))
    raw_rows = payload.get("evidence", [])
    if not isinstance(raw_rows, list):
        raise ValidationError("behavioral evidence must contain an evidence array", path=path)
    rows_by_card: dict[str, list[Mapping[str, Any]]] = {}
    evidence_ids: set[str] = set()
    passing: set[str] = set()
    stale: set[str] = set()
    # Array position is the append-only ledger order.  Preserve it in every
    # per-card view; the last applicable row is authoritative.
    for row in raw_rows:
        if not isinstance(row, Mapping):
            raise ValidationError("behavioral evidence row must be an object", path=path)
        evidence_id = row.get("evidence_id")
        if not isinstance(evidence_id, str) or not evidence_id:
            raise ValidationError("behavioral evidence row requires evidence_id", path=path)
        if evidence_id in evidence_ids:
            raise ValidationError(
                "behavioral evidence ledger contains a duplicate evidence_id",
                path=path,
                details={"evidence_id": evidence_id},
            )
        evidence_ids.add(evidence_id)
        card_ids = row.get("card_ids", [])
        if (
            not isinstance(card_ids, list)
            or not card_ids
            or not all(isinstance(card_id, str) for card_id in card_ids)
            or len(card_ids) != len(set(card_ids))
        ):
            raise ValidationError("behavioral evidence must map at least one card", path=path)
        test_targets = row.get("test_targets")
        if (
            not isinstance(test_targets, list)
            or not test_targets
            or not all(isinstance(target, str) and target for target in test_targets)
            or len(test_targets) != len(set(test_targets))
        ):
            raise ValidationError(
                "behavioral evidence requires unique test targets",
                path=path,
                details={"evidence_id": evidence_id},
            )
        if row.get("result") not in {"passed", "failed", "not-run"}:
            raise ValidationError(
                "behavioral evidence has an unsupported result",
                path=path,
                details={"evidence_id": evidence_id},
            )
        if not isinstance(row.get("current"), bool):
            raise ValidationError(
                "behavioral evidence current observation must be boolean",
                path=path,
                details={"evidence_id": evidence_id},
            )
        for field_name in (
            "engine_revision",
            "source_revision",
            "artifact_digest",
            "canonical_data_fingerprint",
            "yaml_fingerprint",
            "test_fingerprint",
        ):
            digest = row.get(field_name)
            if (
                not isinstance(digest, str)
                or len(digest) != 64
                or any(character not in "0123456789abcdef" for character in digest)
            ):
                raise ValidationError(
                    "behavioral evidence requires lowercase sha256 fingerprints",
                    path=path,
                    details={"evidence_id": evidence_id, "field": field_name},
                )
        for card_id in card_ids:
            rows_by_card.setdefault(card_id, []).append(row)
    for card_id, rows in rows_by_card.items():
        latest = rows[-1]
        row_passing = latest.get("result") == "passed" and latest.get("current") is True
        (passing if row_passing else stale).add(card_id)
    return EvidenceInventory(
        ledger_rows=tuple(raw_rows),
        rows_by_card={key: tuple(value) for key, value in sorted(rows_by_card.items())},
        passing_current_ids=frozenset(passing),
        stale_or_nonpassing_ids=frozenset(stale),
    )


def _tree_digest(root: Path, pattern: str) -> str:
    digest = hashlib.sha256()
    for path in sorted(root.rglob(pattern)):
        digest.update(path.relative_to(root).as_posix().encode("utf-8"))
        digest.update(b"\0")
        digest.update(path.read_bytes())
        digest.update(b"\0")
    return digest.hexdigest()


def _digest_file_set(repo_root: Path, paths: Iterable[Path]) -> str:
    """Digest a named set of repository inputs, not just their byte stream.

    Names are part of the binding so moving, adding, or removing a build/test
    input invalidates prior evidence even when another file has identical
    bytes.
    """

    resolved_root = repo_root.resolve()
    rows: dict[str, str] = {}
    for path in sorted({item.resolve() for item in paths if item.is_file()}):
        try:
            name = path.relative_to(resolved_root).as_posix()
        except ValueError:
            name = path.as_posix()
        rows[name] = sha256_hex(path.read_bytes())
    return content_digest(rows)


def engine_behavior_revision(
    repo_root: Path,
    *,
    engine_source_root: Path | None = None,
) -> str:
    """Fingerprint every code/build input that can change DSL card behavior.

    Card evidence is not current merely because ``digimon-engine/src`` is
    unchanged: DSL lowering, build-time pack assembly, dependency resolution,
    and source-lane/build baselines can all change the compiled behavior that
    the recorded test exercised.
    """

    root = repo_root.resolve()
    engine_source = (
        engine_source_root.resolve()
        if engine_source_root is not None
        else root / "code/digimon-engine/src"
    )
    engine_crate = engine_source.parent
    dsl_crate = root / "code/digimon-dsl"
    paths: set[Path] = set(engine_source.rglob("*.rs")) if engine_source.exists() else set()
    for candidate in (
        root / "Cargo.toml",
        root / "Cargo.lock",
        engine_crate / "Cargo.toml",
        engine_crate / "build.rs",
        engine_crate / "cards/source_lanes.json",
        engine_crate / "cards/core_validation_baseline.json",
        dsl_crate / "Cargo.toml",
        dsl_crate / "build.rs",
    ):
        if candidate.is_file():
            paths.add(candidate)
    if (dsl_crate / "src").is_dir():
        paths.update((dsl_crate / "src").rglob("*.rs"))
    return _digest_file_set(root, paths)


def _semantic_test_tree_files(test_root: Path) -> tuple[Path, ...]:
    if not test_root.is_dir():
        return ()
    return tuple(
        path
        for path in test_root.rglob("*")
        if path.is_file()
        and not any(part in {"target", "__pycache__"} for part in path.parts)
        and not path.name.startswith(".")
    )


def behavioral_test_tree_revision(repo_root: Path, test_root: Path) -> str:
    """Fingerprint the complete behavioral-test tree used by the campaign."""

    return _digest_file_set(repo_root, _semantic_test_tree_files(test_root))


def _test_support_files(test_root: Path) -> tuple[Path, ...]:
    """Return shared harness inputs that can alter an individual target.

    Rust integration targets depend on suite ``main.rs``/``mod.rs`` files and
    shared support/fixture/helper trees in addition to the leaf test file.
    Conservatively binding those shared inputs prevents a harness change from
    leaving every recorded leaf result marked current.
    """

    support_names = {"support", "fixtures", "helpers", "common"}
    return tuple(
        path
        for path in _semantic_test_tree_files(test_root)
        if path.name in {"main.rs", "mod.rs"}
        or any(part.lower() in support_names for part in path.relative_to(test_root).parts[:-1])
    )


def behavioral_test_support_revision(repo_root: Path, test_root: Path) -> str:
    """Fingerprint the shared harness once for reuse across evidence rows."""

    return _digest_file_set(repo_root, _test_support_files(test_root))


def behavioral_test_fingerprint(
    targets: tuple[str, ...],
    *,
    repo_root: Path,
    test_root: Path,
    support_revision: str | None = None,
) -> tuple[str, bool]:
    """Bind target bytes plus the shared test harness they compile through."""

    if not targets:
        return "0" * 64, False
    resolved_root = repo_root.resolve()
    target_rows: dict[str, str] = {}
    complete = True
    target_paths: set[Path] = set()
    for target in targets:
        path = (resolved_root / target).resolve()
        if not path.is_relative_to(resolved_root) or not path.is_file():
            complete = False
            target_rows[target] = "0" * 64
            continue
        target_rows[target] = sha256_hex(path.read_bytes())
        target_paths.add(path)
    if support_revision is None:
        support_paths = set(_test_support_files(test_root)) - target_paths
        support_revision = _digest_file_set(repo_root, support_paths)
    return (
        content_digest(
            {
                "targets": target_rows,
                "test_support_revision": support_revision,
            }
        ),
        complete,
    )


def _yaml_fingerprint(
    card_ids: tuple[str, ...],
    inventory: YamlInventory,
) -> tuple[str, bool]:
    rows: dict[str, str | None] = {}
    complete = True
    for card_id in card_ids:
        paths = inventory.production_paths.get(card_id, ())
        if not paths:
            rows[card_id] = None
            complete = False
        elif len(paths) == 1:
            rows[card_id] = inventory.path_digests[paths[0]]
        else:
            # Duplicate production YAML is rejected separately, but retain a
            # deterministic fingerprint for an actionable stale diagnostic.
            rows[card_id] = content_digest(
                {path: inventory.path_digests[path] for path in paths}
            )
            complete = False
    return content_digest(rows), complete


def _test_fingerprint(
    targets: tuple[str, ...],
    *,
    repo_root: Path,
    test_root: Path,
    support_revision: str | None = None,
) -> tuple[str, bool]:
    return behavioral_test_fingerprint(
        targets,
        repo_root=repo_root,
        test_root=test_root,
        support_revision=support_revision,
    )


def reconcile_evidence_currentness(
    evidence: EvidenceInventory,
    *,
    canonical_records: Mapping[str, Mapping[str, Any]],
    yaml_inventory: YamlInventory,
    repo_root: Path,
    engine_source_root: Path,
    test_root: Path,
    source_revision: str,
) -> EvidenceInventory:
    """Recompute evidence currentness from live repository fingerprints.

    The serialized ``current`` flag is only an observation made when a result
    was recorded.  Campaign generation never trusts it: canonical records,
    production YAML, test targets, engine source, and the generated card-data
    source are fingerprinted again.  Ledger array order remains authoritative,
    and only the last applicable row controls each card's pass/stale status.
    """

    engine_revision = engine_behavior_revision(
        repo_root,
        engine_source_root=engine_source_root,
    )
    support_revision = behavioral_test_support_revision(repo_root, test_root)
    normalized_by_id: dict[str, Mapping[str, Any]] = {}
    normalized_ledger: list[Mapping[str, Any]] = []
    for row in evidence.ledger_rows:
        card_ids = tuple(str(card_id) for card_id in row.get("card_ids", ()))
        targets = tuple(str(target) for target in row.get("test_targets", ()))
        canonical_complete = all(card_id in canonical_records for card_id in card_ids)
        canonical_fingerprint = content_digest(
            {card_id: canonical_records.get(card_id) for card_id in card_ids}
        )
        yaml_fingerprint, yaml_complete = _yaml_fingerprint(card_ids, yaml_inventory)
        test_fingerprint, tests_complete = _test_fingerprint(
            targets,
            repo_root=repo_root,
            test_root=test_root,
            support_revision=support_revision,
        )
        current = bool(
            canonical_complete
            and yaml_complete
            and tests_complete
            and row.get("canonical_data_fingerprint") == canonical_fingerprint
            and row.get("yaml_fingerprint") == yaml_fingerprint
            and row.get("test_fingerprint") == test_fingerprint
            and row.get("engine_revision") == engine_revision
            and row.get("source_revision") == source_revision
        )
        normalized = {**row, "current": current}
        normalized_by_id[str(row["evidence_id"])] = normalized
        normalized_ledger.append(normalized)

    rows_by_card = {
        card_id: tuple(normalized_by_id[str(row["evidence_id"])] for row in rows)
        for card_id, rows in evidence.rows_by_card.items()
    }
    passing: set[str] = set()
    stale: set[str] = set()
    for card_id, rows in rows_by_card.items():
        latest = rows[-1]
        row_passing = latest.get("result") == "passed" and latest.get("current") is True
        (passing if row_passing else stale).add(card_id)
    return EvidenceInventory(
        ledger_rows=tuple(normalized_ledger),
        rows_by_card=rows_by_card,
        passing_current_ids=frozenset(passing),
        stale_or_nonpassing_ids=frozenset(stale),
    )


def scan_ignored_tests(
    test_root: Path,
    *,
    evidence: EvidenceInventory,
    repo_root: Path,
) -> dict[str, tuple[str, ...]]:
    ignored_files: set[str] = set()
    for path in test_root.rglob("*.rs"):
        if any(_IGNORE_RE.match(line) for line in path.read_text(encoding="utf-8", errors="replace").splitlines()):
            ignored_files.add(path.relative_to(repo_root).as_posix())
    by_card: dict[str, list[str]] = {}
    for card_id, rows in evidence.rows_by_card.items():
        for row in rows:
            for target in row.get("test_targets", []):
                if target in ignored_files:
                    by_card.setdefault(card_id, []).append(target)
    return {card_id: tuple(sorted(set(paths))) for card_id, paths in sorted(by_card.items())}


def load_deck_usage(
    path: Path,
    card_ids: set[str],
    *,
    payload: Mapping[str, Any] | None = None,
) -> dict[str, list[dict[str, object]]]:
    payload = payload if payload is not None else json.loads(path.read_text(encoding="utf-8"))
    return find_deck_usage(payload, card_ids)


def load_blockers(
    path: Path,
    *,
    repo_root: Path,
    payload: Mapping[str, Any] | None = None,
    canonical_card_ids: set[str] | frozenset[str] | None = None,
) -> BlockerRegistry:
    registry = (
        blocker_registry_from_payload(payload, path=path)
        if payload is not None
        else load_blocker_registry(path)
    )
    registry.reconcile(repo_root, canonical_card_ids=canonical_card_ids)
    return registry
