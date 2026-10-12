"""Complete-revision planning, atomic publication, and rollback recovery."""

from __future__ import annotations

import json
import os
import shutil
import time
import uuid
from dataclasses import dataclass
from pathlib import Path, PurePosixPath
from typing import Callable, Iterator, Mapping, Sequence

from .errors import ValidationError, WriteError
from .locking import RepositoryWriterLock
from .serialization import canonical_json_bytes, content_digest, sha256_hex
from .sync_plan import (
    SourcePlan,
    composite_sync_plan_digest,
    source_plan_components,
)
from .transaction import (
    EMPTY_REVISION,
    ArtifactMutation,
    PlanToken,
    RecoveryRequiredError,
    RevisionMarker,
    TransactionJournal,
    TransactionState,
    current_revision,
    load_revision_marker,
    load_transaction_journal,
    prepare_transaction,
    read_consistent_revision,
    write_revision_marker,
    write_transaction_journal,
)


FaultHook = Callable[[str, str | None], None]

CANDIDATE_INVENTORY_SCHEMA_VERSION = 1
CANDIDATE_INVENTORY_NAME = ".card-data-candidate.json"
TRANSACTION_LAYOUT_SCHEMA_VERSION = 1
TRANSACTION_STORAGE_ROOT = PurePosixPath("data/.card_data_tx")
TRANSACTION_STORAGE_KEY_LENGTH = 16
TRANSACTION_ORDINAL_WIDTH = 8
# One journal fsync per bounded prefix avoids O(N^2) journal I/O for the full
# 21k-artifact publication while keeping the recovery intent conservative.
TRANSACTION_PROGRESS_BATCH_SIZE = 512
# Windows can transiently deny an otherwise valid atomic replacement while a
# virus scanner, indexer, or another reader is releasing its handle.  Keep the
# retry window short and bounded: persistent permission problems must still
# stop the transaction and leave its recovery evidence intact.
WINDOWS_REPLACE_RETRY_ATTEMPTS = 8
WINDOWS_REPLACE_RETRY_INITIAL_DELAY_SECONDS = 0.01
WINDOWS_REPLACE_RETRY_MAX_DELAY_SECONDS = 0.1
WINDOWS_TRANSIENT_REPLACE_WINERRORS = frozenset({5, 32})


@dataclass(frozen=True, slots=True)
class ArtifactDiff:
    artifact_path: str
    prior_digest: str | None
    new_digest: str

    @property
    def changed(self) -> bool:
        return self.prior_digest != self.new_digest


@dataclass(frozen=True, slots=True)
class _TransactionStorageLayout:
    """Short private paths mapped to the transaction's logical artifacts.

    Logical artifact paths stay in the journal and revision marker.  Private
    staging and rollback files use fixed-width ordinals so a deeply nested
    observation path is never duplicated below a transaction directory on
    Windows.  The on-disk layout manifest binds the shortened directory key
    back to the complete plan token and exact path mapping.
    """

    root: Path
    manifest: Path
    manifest_bytes: bytes
    marker_backup: Path
    staged: Mapping[str, Path]
    rollback: Mapping[str, Path]


@dataclass(frozen=True, slots=True)
class PublicationPlan:
    token: PlanToken
    marker: RevisionMarker
    artifact_bytes: Mapping[str, bytes]
    artifact_diffs: tuple[ArtifactDiff, ...]
    source_plan_digest: str
    source_plan_component_digests: tuple[str, ...]
    source_diff_digest: str | None
    source_plan_requested_scope: str


def candidate_inventory_payload(
    artifact_bytes: Mapping[str, bytes],
) -> dict[str, object]:
    """Describe the exact publishable files in one candidate directory.

    The inventory is a candidate-control sidecar, not a fixed-path artifact. It
    lets plan/apply reject both stale leftovers and missing candidate files
    instead of treating every recursively discovered file as publishable.
    """

    normalized: dict[str, str] = {}
    for raw_path, body in artifact_bytes.items():
        relative = _relative_path(raw_path)
        if relative == CANDIDATE_INVENTORY_NAME:
            raise ValidationError(
                "candidate inventory cannot list itself as a publication artifact"
            )
        if not isinstance(body, bytes):
            raise ValidationError("candidate artifact body must be bytes", path=relative)
        if relative in normalized:
            raise ValidationError("candidate inventory contains duplicate normalized paths")
        normalized[relative] = sha256_hex(body)
    if not normalized:
        raise ValidationError("candidate inventory must declare at least one artifact")
    return {
        "schema_version": CANDIDATE_INVENTORY_SCHEMA_VERSION,
        "artifact_digests": dict(sorted(normalized.items())),
    }


def load_candidate_artifacts(candidate_dir: Path) -> dict[str, bytes]:
    """Load exactly the files declared by a digest-checked candidate inventory."""

    root = candidate_dir.resolve()
    inventory_path = root / CANDIDATE_INVENTORY_NAME
    try:
        payload = json.loads(inventory_path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise ValidationError(
            "publication candidate inventory is unavailable or invalid",
            path=inventory_path,
            details={"error": str(exc)},
        ) from exc
    if not isinstance(payload, Mapping) or set(payload) != {
        "schema_version",
        "artifact_digests",
    }:
        raise ValidationError(
            "publication candidate inventory has missing or unknown fields",
            path=inventory_path,
        )
    if payload["schema_version"] != CANDIDATE_INVENTORY_SCHEMA_VERSION:
        raise ValidationError(
            "unsupported publication candidate inventory schema version",
            path=inventory_path,
        )
    raw_digests = payload["artifact_digests"]
    if not isinstance(raw_digests, Mapping) or not raw_digests:
        raise ValidationError(
            "publication candidate inventory requires artifact digests",
            path=inventory_path,
        )
    declared: dict[str, str] = {}
    for raw_path, raw_digest in raw_digests.items():
        relative = _relative_path(str(raw_path))
        if relative == CANDIDATE_INVENTORY_NAME:
            raise ValidationError(
                "candidate inventory cannot list itself as a publication artifact",
                path=inventory_path,
            )
        if relative in declared:
            raise ValidationError(
                "publication candidate inventory contains duplicate normalized paths",
                path=inventory_path,
            )
        declared[relative] = _require_digest(
            raw_digest, "publication candidate artifact digest"
        )

    actual_paths = {
        path.relative_to(root).as_posix()
        for path in root.rglob("*")
        if path.is_file() and path.resolve() != inventory_path.resolve()
    }
    missing = sorted(set(declared) - actual_paths)
    unexpected = sorted(actual_paths - set(declared))
    if missing or unexpected:
        raise ValidationError(
            "publication candidate files do not match the reviewed inventory",
            path=inventory_path,
            details={"missing": missing, "unexpected": unexpected},
        )

    bodies: dict[str, bytes] = {}
    for relative, expected in sorted(declared.items()):
        path = root / PurePosixPath(relative)
        try:
            body = path.read_bytes()
        except OSError as exc:
            raise ValidationError(
                "publication candidate artifact is unavailable", path=path
            ) from exc
        actual = sha256_hex(body)
        if actual != expected:
            raise ValidationError(
                "publication candidate artifact digest is stale",
                path=path,
                details={"expected_digest": expected, "actual_digest": actual},
            )
        bodies[relative] = body
    from .compatibility_corrections import validate_correction_artifacts
    validate_correction_artifacts(bodies)
    return bodies


def _publication_binding_digest(
    *,
    source_plan_digest: str,
    source_plan_component_digests: Sequence[str],
    source_diff_digest: str,
    artifact_diffs: Sequence[ArtifactDiff],
) -> str:
    return content_digest(
        {
            "source_plan_digest": source_plan_digest,
            "source_plan_component_digests": tuple(source_plan_component_digests),
            "source_diff_digest": source_diff_digest,
            "artifact_diffs": tuple(artifact_diffs),
        }
    )


def _require_digest(value: object, label: str) -> str:
    digest = str(value)
    if len(digest) != 64 or any(
        character not in "0123456789abcdef" for character in digest
    ):
        raise ValidationError(f"{label} is invalid")
    return digest


def publication_plan_payload(plan: PublicationPlan) -> dict[str, object]:
    payload: dict[str, object] = {
        "schema_version": 2 if plan.source_diff_digest is not None else 1,
        "token": plan.token.to_dict(),
        "marker": plan.marker.to_dict(),
        "artifact_diffs": [
            {
                "artifact_path": item.artifact_path,
                "prior_digest": item.prior_digest,
                "new_digest": item.new_digest,
            }
            for item in plan.artifact_diffs
        ],
        "source_plan_digest": plan.source_plan_digest,
    }
    if plan.source_diff_digest is not None:
        payload.update(
            {
                "source_plan_component_digests": list(
                    plan.source_plan_component_digests
                ),
                "source_diff_digest": plan.source_diff_digest,
                "source_plan_requested_scope": plan.source_plan_requested_scope,
            }
        )
    return payload


def load_publication_plan(
    payload: Mapping[str, object],
    *,
    candidate_dir: Path,
) -> PublicationPlan:
    schema_version = payload.get("schema_version")
    if schema_version not in {1, 2}:
        raise ValidationError("unsupported publication plan schema version")
    token = PlanToken.from_dict(payload["token"])
    marker = RevisionMarker.from_dict(payload["marker"])
    if token.staged_digest != marker.revision or marker.plan_token_id != token.token_id:
        raise ValidationError("publication plan token and marker do not match")
    diffs = tuple(
        ArtifactDiff(
            artifact_path=item["artifact_path"],
            prior_digest=item.get("prior_digest"),
            new_digest=item["new_digest"],
        )
        for item in payload["artifact_diffs"]
    )
    if {item.artifact_path for item in diffs} != set(marker.file_digests):
        raise ValidationError("publication plan diffs do not cover the complete marker")
    bodies = load_candidate_artifacts(candidate_dir)
    if set(bodies) != set(marker.file_digests):
        raise ValidationError(
            "publication plan marker does not match the candidate inventory",
            details={
                "missing_from_candidate": sorted(set(marker.file_digests) - set(bodies)),
                "unexpected_in_candidate": sorted(set(bodies) - set(marker.file_digests)),
            },
        )
    for relative, expected in marker.file_digests.items():
        actual = sha256_hex(bodies[relative])
        if actual != expected:
            raise ValidationError(
                "publication candidate artifact digest is stale",
                path=candidate_dir / PurePosixPath(relative),
                details={"expected_digest": expected, "actual_digest": actual},
            )
    source_plan_digest = _require_digest(
        payload["source_plan_digest"], "publication source plan digest"
    )
    if schema_version == 1:
        component_digests = (source_plan_digest,)
        source_diff_digest = None
        source_plan_requested_scope = token.requested_scope
    else:
        raw_component_digests = payload.get("source_plan_component_digests")
        if not isinstance(raw_component_digests, list) or not raw_component_digests:
            raise ValidationError(
                "publication source plan component digests must be a non-empty array"
            )
        component_digests = tuple(
            _require_digest(item, "publication source plan component digest")
            for item in raw_component_digests
        )
        if len(set(component_digests)) != len(component_digests):
            raise ValidationError(
                "publication source plan component digests must be unique"
            )
        expected_source_plan_digest = (
            component_digests[0]
            if len(component_digests) == 1
            else composite_sync_plan_digest(component_digests)
        )
        if source_plan_digest != expected_source_plan_digest:
            raise ValidationError(
                "publication source plan digest does not match its component digests"
            )
        source_diff_digest = _require_digest(
            payload.get("source_diff_digest"), "publication source diff digest"
        )
        source_plan_requested_scope = str(
            payload.get("source_plan_requested_scope", "")
        )
        if not source_plan_requested_scope:
            raise ValidationError("publication source plan requested scope is missing")
        if token.requested_scope != source_plan_requested_scope:
            raise ValidationError(
                "publication token requested scope does not match its source plan"
            )
        expected_diff_digest = _publication_binding_digest(
            source_plan_digest=source_plan_digest,
            source_plan_component_digests=component_digests,
            source_diff_digest=source_diff_digest,
            artifact_diffs=diffs,
        )
        if token.diff_digest != expected_diff_digest:
            raise ValidationError(
                "publication token does not bind its source plan components"
            )
    return PublicationPlan(
        token=token,
        marker=marker,
        artifact_bytes=bodies,
        artifact_diffs=diffs,
        source_plan_digest=source_plan_digest,
        source_plan_component_digests=component_digests,
        source_diff_digest=source_diff_digest,
        source_plan_requested_scope=source_plan_requested_scope,
    )


def _relative_path(value: str) -> str:
    path = PurePosixPath(value)
    if not value or path.is_absolute() or any(part in {"", ".", ".."} for part in path.parts):
        raise ValidationError("publication artifact path must be repository-relative")
    return path.as_posix()


def _artifact_path(root: Path, relative: str) -> Path:
    relative = _relative_path(relative)
    path = (root / PurePosixPath(relative)).resolve()
    if not path.is_relative_to(root.resolve()):
        raise ValidationError("publication artifact escapes repository", path=path)
    return path


def _transaction_storage_layout(
    root: Path, plan: PublicationPlan
) -> _TransactionStorageLayout:
    """Build one collision-checked short-path storage manifest for a plan."""

    storage_key = plan.token.token_id[:TRANSACTION_STORAGE_KEY_LENGTH]
    relative_root = TRANSACTION_STORAGE_ROOT / storage_key
    entries: list[dict[str, object]] = []
    staged: dict[str, Path] = {}
    rollback: dict[str, Path] = {}
    for ordinal, diff in enumerate(
        sorted(plan.artifact_diffs, key=lambda item: item.artifact_path)
    ):
        file_name = f"{ordinal:0{TRANSACTION_ORDINAL_WIDTH}x}"
        staged_relative = (relative_root / "s" / file_name).as_posix()
        rollback_relative = (
            (relative_root / "r" / file_name).as_posix()
            if diff.prior_digest is not None
            else None
        )
        staged[diff.artifact_path] = _artifact_path(root, staged_relative)
        if rollback_relative is not None:
            rollback[diff.artifact_path] = _artifact_path(root, rollback_relative)
        entries.append(
            {
                "artifact_path": diff.artifact_path,
                "staged_path": staged_relative,
                "rollback_path": rollback_relative,
            }
        )
    marker_backup_relative = (relative_root / "m").as_posix()
    identity = {
        "schema_version": TRANSACTION_LAYOUT_SCHEMA_VERSION,
        "plan_token_id": plan.token.token_id,
        "target_revision": plan.marker.revision,
        "entries": entries,
        "marker_backup_path": marker_backup_relative,
    }
    payload = {
        **identity,
        "layout_revision": content_digest(identity),
    }
    layout_root = _artifact_path(root, relative_root.as_posix())
    return _TransactionStorageLayout(
        root=layout_root,
        manifest=layout_root / "layout.json",
        manifest_bytes=canonical_json_bytes(payload),
        marker_backup=_artifact_path(root, marker_backup_relative),
        staged=staged,
        rollback=rollback,
    )


def _prepare_transaction_storage(layout: _TransactionStorageLayout) -> None:
    """Create or validate the private ordinal-path transaction directory."""

    layout.root.mkdir(parents=True, exist_ok=True)
    if layout.manifest.exists():
        try:
            actual = layout.manifest.read_bytes()
        except OSError as exc:
            raise WriteError(
                "transaction storage manifest is unreadable",
                path=layout.manifest,
                details={"error": str(exc)},
            ) from exc
        if actual != layout.manifest_bytes:
            raise WriteError(
                "short transaction storage key collides with another plan",
                path=layout.manifest,
            )
    else:
        unexpected = sorted(
            path.name for path in layout.root.iterdir() if path != layout.manifest
        )
        if unexpected:
            raise WriteError(
                "transaction storage exists without its path manifest",
                path=layout.root,
                details={"entries": unexpected},
            )
        last_error: OSError | None = None
        for _ in range(8):
            # Place the temporary beside, rather than inside, the keyed root.
            # A process death before replace therefore leaves that root empty
            # and the same reviewed plan remains safely retryable.
            temporary = layout.root.parent / f".l{uuid.uuid4().hex[:16]}"
            try:
                handle = temporary.open("xb")
            except FileExistsError as exc:
                last_error = exc
                continue
            try:
                with handle:
                    handle.write(layout.manifest_bytes)
                    handle.flush()
                    os.fsync(handle.fileno())
                os.replace(temporary, layout.manifest)
                last_error = None
                break
            except OSError as exc:
                last_error = exc
                try:
                    temporary.unlink(missing_ok=True)
                except OSError:
                    pass
                break
        if last_error is not None:
            raise WriteError(
                "failed to write transaction storage manifest",
                path=layout.manifest,
                details={"error": str(last_error)},
            ) from last_error
    (layout.root / "s").mkdir(exist_ok=True)
    (layout.root / "r").mkdir(exist_ok=True)


def _progress_batches(
    values: Sequence[ArtifactMutation],
) -> Iterator[tuple[ArtifactMutation, ...]]:
    for offset in range(0, len(values), TRANSACTION_PROGRESS_BATCH_SIZE):
        yield tuple(values[offset : offset + TRANSACTION_PROGRESS_BATCH_SIZE])


def _require_preserved_correction_history(root, artifacts, marker_path, journal_path):
    """A complete publication cannot silently downgrade a versioned ledger."""
    from .compatibility_corrections import (
        CORRECTIONS_PATH, CorrectionDocument, correction_document_from_bytes,
        load_correction_document, require_document_extension,
    )
    if load_revision_marker(marker_path) is not None:
        committed = read_consistent_revision(root, marker_path=marker_path, journal_path=journal_path)
        raw = committed.values.get(CORRECTIONS_PATH)
        prior = correction_document_from_bytes(raw) if raw is not None else CorrectionDocument()
    else:
        prior = load_correction_document(root / CORRECTIONS_PATH)
    raw = artifacts.get(CORRECTIONS_PATH)
    proposed = correction_document_from_bytes(raw) if raw is not None else CorrectionDocument()
    require_document_extension(prior, proposed)


def build_publication_plan(
    *,
    repo_root: Path,
    source_plan: SourcePlan,
    artifact_bytes: Mapping[str, bytes],
    marker_path: Path,
    journal_path: Path,
    reviewed_adjudication_digests: Sequence[str] = (),
) -> PublicationPlan:
    """Bind one complete candidate revision without writing any files."""

    root = repo_root.resolve()
    from .compatibility_corrections import validate_correction_artifacts
    validate_correction_artifacts(artifact_bytes)
    _require_preserved_correction_history(root, artifact_bytes, marker_path, journal_path)
    if not artifact_bytes:
        raise ValidationError("publication candidate must contain at least one artifact")
    journal = load_transaction_journal(journal_path)
    if journal is not None and journal.state not in {
        TransactionState.COMMITTED,
        TransactionState.ROLLED_BACK,
    }:
        raise RecoveryRequiredError("an incomplete transaction must be recovered before planning")
    marker = load_revision_marker(marker_path)
    if marker is not None:
        read_consistent_revision(
            root,
            marker_path=marker_path,
            journal_path=journal_path,
        )
    base_revision = marker.revision if marker else EMPTY_REVISION
    components = source_plan_components(source_plan)
    blocked_components = tuple(
        item.requested_scope for item in components if not item.can_apply
    )
    if blocked_components:
        raise ValidationError(
            "every source sync plan component must be applicable before publication",
            details={"blocked_requested_scopes": blocked_components},
        )
    if source_plan.base_committed_revision != base_revision:
        raise ValidationError(
            "source sync plan base revision does not match committed revision",
            details={
                "source_plan_base": source_plan.base_committed_revision,
                "committed_revision": base_revision,
            },
        )

    normalized: dict[str, bytes] = {}
    for raw_path, body in artifact_bytes.items():
        relative = _relative_path(raw_path)
        if not isinstance(body, bytes):
            raise ValidationError("publication artifact body must be bytes", path=relative)
        normalized[relative] = body
    if len(normalized) != len(artifact_bytes):
        raise ValidationError("publication candidate contains duplicate normalized paths")
    if marker is not None:
        omitted = sorted(set(marker.file_digests) - set(normalized))
        if omitted:
            raise ValidationError(
                "publication candidate omits previously committed artifacts",
                details={"artifact_paths": omitted},
            )
    new_digests = {path: sha256_hex(body) for path, body in sorted(normalized.items())}
    target_marker = RevisionMarker.create(new_digests)
    diffs: list[ArtifactDiff] = []
    for relative, new_digest in new_digests.items():
        path = _artifact_path(root, relative)
        prior_digest = sha256_hex(path.read_bytes()) if path.exists() else None
        diffs.append(ArtifactDiff(relative, prior_digest, new_digest))
    component_digests = tuple(item.draft_digest for item in components)
    diff_digest = _publication_binding_digest(
        source_plan_digest=source_plan.draft_digest,
        source_plan_component_digests=component_digests,
        source_diff_digest=source_plan.diff_digest,
        artifact_diffs=diffs,
    )
    token = PlanToken.create(
        requested_scope=source_plan.requested_scope,
        base_revision=base_revision,
        staged_digest=target_marker.revision,
        diff_digest=diff_digest,
        reviewed_adjudication_digests=reviewed_adjudication_digests,
    )
    target_marker = RevisionMarker.create(new_digests, plan_token_id=token.token_id)
    return PublicationPlan(
        token=token,
        marker=target_marker,
        artifact_bytes=dict(sorted(normalized.items())),
        artifact_diffs=tuple(sorted(diffs, key=lambda item: item.artifact_path)),
        source_plan_digest=source_plan.draft_digest,
        source_plan_component_digests=component_digests,
        source_diff_digest=source_plan.diff_digest,
        source_plan_requested_scope=source_plan.requested_scope,
    )


def _replace_with_windows_retry(source: Path, destination: Path) -> None:
    """Replace once, retrying only transient Windows access/share failures."""

    delay = WINDOWS_REPLACE_RETRY_INITIAL_DELAY_SECONDS
    for attempt in range(WINDOWS_REPLACE_RETRY_ATTEMPTS):
        try:
            os.replace(source, destination)
            return
        except OSError as exc:
            transient_windows_error = (
                getattr(exc, "winerror", None)
                in WINDOWS_TRANSIENT_REPLACE_WINERRORS
            )
            if (
                not transient_windows_error
                or attempt + 1 >= WINDOWS_REPLACE_RETRY_ATTEMPTS
            ):
                raise
            time.sleep(delay)
            delay = min(delay * 2, WINDOWS_REPLACE_RETRY_MAX_DELAY_SECONDS)


def _atomic_replace_from(staged: Path, destination: Path) -> None:
    destination.parent.mkdir(parents=True, exist_ok=True)
    _replace_with_windows_retry(staged, destination)


def _atomic_copy_replace_from(source: Path, destination: Path) -> None:
    """Atomically restore a file without consuming the rollback evidence."""

    destination.parent.mkdir(parents=True, exist_ok=True)
    last_error: OSError | None = None
    for _ in range(8):
        # Keep the sibling name shorter than long observation filenames.  The
        # old destination-derived temporary name could itself cross MAX_PATH.
        temporary = destination.parent / f".r{uuid.uuid4().hex[:16]}"
        try:
            handle = temporary.open("xb")
        except FileExistsError as exc:
            last_error = exc
            continue
        try:
            with source.open("rb") as source_handle, handle:
                shutil.copyfileobj(source_handle, handle)
                handle.flush()
                os.fsync(handle.fileno())
            _replace_with_windows_retry(temporary, destination)
            return
        except OSError as exc:
            last_error = exc
            try:
                temporary.unlink(missing_ok=True)
            except OSError:
                pass
            break
    raise WriteError(
        "failed to atomically restore card-data rollback artifact",
        path=destination,
        details={"source": str(source), "error": str(last_error)},
    ) from last_error


def _invoke(hook: FaultHook | None, event: str, path: str | None = None) -> None:
    if hook is not None:
        hook(event, path)


def apply_publication(
    plan: PublicationPlan,
    *,
    repo_root: Path,
    marker_path: Path,
    journal_path: Path,
    lock_path: Path | None = None,
    recover_stale_lock: bool = False,
    fault_hook: FaultHook | None = None,
) -> TransactionJournal:
    """Apply a reviewed plan under lock, writing the revision marker last."""

    root = repo_root.resolve()
    from .compatibility_corrections import validate_correction_artifacts
    validate_correction_artifacts(plan.artifact_bytes)
    lock = RepositoryWriterLock(
        root,
        lock_path=lock_path,
        recover_stale=recover_stale_lock,
    )
    with lock:
        _require_preserved_correction_history(root, plan.artifact_bytes, marker_path, journal_path)
        if current_revision(marker_path) != plan.token.base_revision:
            raise ValidationError("publication plan is stale before staging")
        for diff in plan.artifact_diffs:
            destination = _artifact_path(root, diff.artifact_path)
            actual = sha256_hex(destination.read_bytes()) if destination.exists() else None
            if actual != diff.prior_digest:
                raise ValidationError(
                    "publication artifact changed after planning",
                    path=destination,
                    details={"planned_digest": diff.prior_digest, "actual_digest": actual},
                )
        try:
            layout = _transaction_storage_layout(root, plan)
            _prepare_transaction_storage(layout)
            mutations: list[ArtifactMutation] = []
            for diff in plan.artifact_diffs:
                staged = layout.staged[diff.artifact_path]
                staged.write_bytes(plan.artifact_bytes[diff.artifact_path])
                if sha256_hex(staged.read_bytes()) != diff.new_digest:
                    raise WriteError(
                        "staged publication artifact failed digest validation",
                        path=staged,
                    )
                rollback_relative = None
                if diff.prior_digest is not None:
                    destination = _artifact_path(root, diff.artifact_path)
                    rollback = layout.rollback[diff.artifact_path]
                    shutil.copyfile(destination, rollback)
                    rollback_relative = rollback.relative_to(root).as_posix()
                mutations.append(
                    ArtifactMutation(
                        artifact_path=diff.artifact_path,
                        staged_path=staged.relative_to(root).as_posix(),
                        new_digest=diff.new_digest,
                        prior_digest=diff.prior_digest,
                        rollback_path=rollback_relative,
                    )
                )
            prior_marker_relative = None
            if marker_path.exists():
                shutil.copyfile(marker_path, layout.marker_backup)
                prior_marker_relative = layout.marker_backup.relative_to(root).as_posix()
            _invoke(fault_hook, "staged", None)
            journal = prepare_transaction(
                plan.token,
                mutations,
                marker_path=marker_path,
                journal_path=journal_path,
                writer_lock=lock,
                prior_marker_backup_path=prior_marker_relative,
            )
            journal = journal.start_publishing()
            write_transaction_journal(journal_path, journal)
            _invoke(fault_hook, "publishing", None)
            for batch in _progress_batches(journal.mutations):
                # Persist the entire bounded intent prefix before the first
                # replacement. Recovery conservatively restores every entry
                # in the batch, including any not reached before a crash.
                journal = journal.mark_replaced_batch(
                    tuple(mutation.artifact_path for mutation in batch)
                )
                write_transaction_journal(journal_path, journal)
                for mutation in batch:
                    _invoke(
                        fault_hook,
                        "replacement_intent_recorded",
                        mutation.artifact_path,
                    )
                    staged = _artifact_path(root, mutation.staged_path)
                    destination = _artifact_path(root, mutation.artifact_path)
                    _atomic_replace_from(staged, destination)
                    _invoke(fault_hook, "replaced", mutation.artifact_path)
            write_revision_marker(marker_path, plan.marker)
            _invoke(fault_hook, "marker_written", None)
            journal = journal.commit()
            write_transaction_journal(journal_path, journal)
            return journal
        except BaseException as exc:
            existing = load_transaction_journal(journal_path)
            if existing is None:
                raise
            if existing.state in {TransactionState.COMMITTED, TransactionState.ROLLED_BACK}:
                raise
            journal = existing.start_rollback(str(exc) or type(exc).__name__)
            write_transaction_journal(journal_path, journal)
            intended = tuple(
                reversed(journal.mutations[: len(journal.replaced_paths)])
            )
            pending = intended[len(journal.rolled_back_paths) :]
            for batch in _progress_batches(pending):
                for mutation in batch:
                    destination = _artifact_path(root, mutation.artifact_path)
                    if mutation.rollback_path is None:
                        destination.unlink(missing_ok=True)
                    else:
                        rollback = _artifact_path(root, mutation.rollback_path)
                        _atomic_copy_replace_from(rollback, destination)
                journal = journal.mark_rolled_back_batch(
                    tuple(mutation.artifact_path for mutation in batch)
                )
                write_transaction_journal(journal_path, journal)
            if journal.prior_marker_backup_path is None:
                marker_path.unlink(missing_ok=True)
            else:
                backup = _artifact_path(root, journal.prior_marker_backup_path)
                if not backup.exists():
                    raise RecoveryRequiredError(
                        "transaction marker rollback artifact is unavailable",
                        path=backup,
                    )
                _atomic_copy_replace_from(backup, marker_path)
            journal = journal.finish_rollback()
            write_transaction_journal(journal_path, journal)
            raise


def recover_publication(
    *,
    repo_root: Path,
    marker_path: Path,
    journal_path: Path,
    lock_path: Path | None = None,
    recover_stale_lock: bool = True,
) -> TransactionJournal | None:
    """Roll back any interrupted publication before later consumers proceed."""

    root = repo_root.resolve()
    with RepositoryWriterLock(
        root, lock_path=lock_path, recover_stale=recover_stale_lock
    ):
        journal = load_transaction_journal(journal_path)
        if journal is None or journal.state in {
            TransactionState.COMMITTED,
            TransactionState.ROLLED_BACK,
        }:
            return journal
        if journal.state is not TransactionState.ROLLING_BACK:
            journal = journal.start_rollback("recovered interrupted publication")
            write_transaction_journal(journal_path, journal)
        intended = tuple(
            reversed(journal.mutations[: len(journal.replaced_paths)])
        )
        pending = intended[len(journal.rolled_back_paths) :]
        for batch in _progress_batches(pending):
            for mutation in batch:
                destination = _artifact_path(root, mutation.artifact_path)
                if mutation.rollback_path is None:
                    destination.unlink(missing_ok=True)
                else:
                    rollback = _artifact_path(root, mutation.rollback_path)
                    if not rollback.exists():
                        raise RecoveryRequiredError(
                            "transaction rollback artifact is unavailable",
                            path=rollback,
                        )
                    _atomic_copy_replace_from(rollback, destination)
            journal = journal.mark_rolled_back_batch(
                tuple(mutation.artifact_path for mutation in batch)
            )
            write_transaction_journal(journal_path, journal)
        if journal.prior_marker_backup_path is None:
            marker_path.unlink(missing_ok=True)
        else:
            backup = _artifact_path(root, journal.prior_marker_backup_path)
            if not backup.exists():
                raise RecoveryRequiredError(
                    "transaction marker rollback artifact is unavailable", path=backup
                )
            _atomic_copy_replace_from(backup, marker_path)
        journal = journal.finish_rollback()
        write_transaction_journal(journal_path, journal)
        return journal


def rollback_committed_publication(
    *,
    repo_root: Path,
    marker_path: Path,
    journal_path: Path,
    expected_current_revision: str,
    reason: str,
    lock_path: Path | None = None,
    recover_stale_lock: bool = False,
    fault_hook: FaultHook | None = None,
) -> TransactionJournal:
    """Explicitly restore the revision preceding the latest successful publish.

    This is deliberately separate from interrupted-publication recovery.  The
    operator must pin the currently committed revision and provide a reason;
    every current and rollback digest is checked before the journal enters a
    rollback state.  Rollback backups are copied, not consumed, so an
    interrupted explicit rollback can be resumed by ``recover_publication``.
    """

    if (
        len(expected_current_revision) != 64
        or any(character not in "0123456789abcdef" for character in expected_current_revision)
    ):
        raise ValueError("expected_current_revision must be a lowercase sha256 digest")
    if not reason or not reason.strip():
        raise ValueError("rollback reason must not be empty")

    root = repo_root.resolve()
    with RepositoryWriterLock(
        root, lock_path=lock_path, recover_stale=recover_stale_lock
    ):
        journal = load_transaction_journal(journal_path)
        if journal is None:
            raise RecoveryRequiredError(
                "no committed card-data transaction is available for explicit rollback",
                path=journal_path,
            )
        if journal.state is not TransactionState.COMMITTED:
            raise RecoveryRequiredError(
                "explicit rollback requires the latest transaction to be committed",
                path=journal_path,
                details={"journal_state": journal.state.value},
            )

        current = read_consistent_revision(
            root,
            marker_path=marker_path,
            journal_path=journal_path,
        )
        if current.revision != expected_current_revision:
            raise ValidationError(
                "explicit rollback revision pin does not match the committed revision",
                details={
                    "expected_current_revision": expected_current_revision,
                    "actual_current_revision": current.revision,
                },
            )
        if journal.target_revision != expected_current_revision:
            raise RecoveryRequiredError(
                "committed transaction target does not match the pinned revision",
                path=journal_path,
                details={
                    "transaction_target_revision": journal.target_revision,
                    "expected_current_revision": expected_current_revision,
                },
            )

        mutations = {item.artifact_path: item for item in journal.mutations}
        for mutation in journal.mutations:
            marker_digest = current.marker.file_digests.get(mutation.artifact_path)
            if marker_digest != mutation.new_digest:
                raise RecoveryRequiredError(
                    "committed marker does not match transaction mutation digest",
                    path=_artifact_path(root, mutation.artifact_path),
                    details={
                        "marker_digest": marker_digest,
                        "transaction_digest": mutation.new_digest,
                    },
                )
            if mutation.prior_digest is None:
                continue
            rollback = _artifact_path(root, mutation.rollback_path or "")
            if not rollback.exists():
                raise RecoveryRequiredError(
                    "committed rollback artifact is unavailable", path=rollback
                )
            actual_digest = sha256_hex(rollback.read_bytes())
            if actual_digest != mutation.prior_digest:
                raise RecoveryRequiredError(
                    "committed rollback artifact failed digest validation",
                    path=rollback,
                    details={
                        "expected_digest": mutation.prior_digest,
                        "actual_digest": actual_digest,
                    },
                )

        prior_marker: RevisionMarker | None = None
        if journal.prior_marker_backup_path is None:
            if journal.base_revision != EMPTY_REVISION:
                raise RecoveryRequiredError(
                    "committed rollback is missing its prior revision marker",
                    path=journal_path,
                    details={"base_revision": journal.base_revision},
                )
        else:
            marker_backup = _artifact_path(root, journal.prior_marker_backup_path)
            if not marker_backup.exists():
                raise RecoveryRequiredError(
                    "committed rollback marker artifact is unavailable", path=marker_backup
                )
            prior_marker = load_revision_marker(marker_backup)
            if prior_marker is None or prior_marker.revision != journal.base_revision:
                raise RecoveryRequiredError(
                    "committed rollback marker does not match the base revision",
                    path=marker_backup,
                    details={
                        "expected_base_revision": journal.base_revision,
                        "actual_base_revision": (
                            prior_marker.revision if prior_marker is not None else None
                        ),
                    },
                )

            for relative_path, prior_digest in prior_marker.file_digests.items():
                mutation = mutations.get(relative_path)
                if mutation is not None:
                    if mutation.prior_digest != prior_digest:
                        raise RecoveryRequiredError(
                            "prior marker does not match transaction rollback digest",
                            path=_artifact_path(root, relative_path),
                            details={
                                "marker_digest": prior_digest,
                                "transaction_digest": mutation.prior_digest,
                            },
                        )
                    continue
                artifact = _artifact_path(root, relative_path)
                try:
                    actual_digest = sha256_hex(artifact.read_bytes())
                except OSError as exc:
                    raise RecoveryRequiredError(
                        "unchanged prior-revision artifact is unavailable",
                        path=artifact,
                        details={"error": str(exc)},
                    ) from exc
                if actual_digest != prior_digest:
                    raise RecoveryRequiredError(
                        "unchanged prior-revision artifact failed digest validation",
                        path=artifact,
                        details={
                            "expected_digest": prior_digest,
                            "actual_digest": actual_digest,
                        },
                    )

        journal = journal.start_committed_rollback(reason)
        write_transaction_journal(journal_path, journal)
        _invoke(fault_hook, "rollback_started", None)
        pending = tuple(reversed(journal.mutations))[
            len(journal.rolled_back_paths) :
        ]
        for batch in _progress_batches(pending):
            for mutation in batch:
                destination = _artifact_path(root, mutation.artifact_path)
                if mutation.rollback_path is None:
                    destination.unlink(missing_ok=True)
                else:
                    rollback = _artifact_path(root, mutation.rollback_path)
                    _atomic_copy_replace_from(rollback, destination)
            journal = journal.mark_rolled_back_batch(
                tuple(mutation.artifact_path for mutation in batch)
            )
            write_transaction_journal(journal_path, journal)
            for mutation in batch:
                _invoke(fault_hook, "rolled_back", mutation.artifact_path)

        if prior_marker is None:
            marker_path.unlink(missing_ok=True)
        else:
            marker_backup = _artifact_path(root, journal.prior_marker_backup_path or "")
            _atomic_copy_replace_from(marker_backup, marker_path)
        _invoke(fault_hook, "rollback_marker_restored", None)
        journal = journal.finish_rollback()
        write_transaction_journal(journal_path, journal)
        _invoke(fault_hook, "rollback_finished", None)
        return journal
