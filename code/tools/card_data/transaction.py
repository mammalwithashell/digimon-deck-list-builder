"""Generator-independent card-data transaction and consistent-read primitives."""

from __future__ import annotations

import json
import os
import uuid
from contextlib import contextmanager
from dataclasses import dataclass, replace
from enum import StrEnum
from pathlib import Path, PurePosixPath
from types import MappingProxyType
from typing import Any, Callable, Iterator, Mapping, Sequence

from .errors import ValidationError, WriteError
from .locking import RepositoryWriterLock
from .models import SCHEMA_VERSION, SHA256_RE
from .serialization import canonical_json_bytes, content_digest, sha256_hex


EMPTY_REVISION = "0" * 64


class StalePlanError(ValidationError):
    """Raised when a plan's base revision no longer matches committed state."""


class RecoveryRequiredError(ValidationError):
    """Raised when a journal/marker pair does not describe a complete revision."""


class ConcurrentRevisionError(ValidationError):
    """Raised when a reader cannot obtain one stable committed revision."""


def _require_digest(value: str, field_name: str) -> str:
    if not isinstance(value, str) or not SHA256_RE.fullmatch(value):
        raise ValueError(f"{field_name} must be a lowercase sha256 digest")
    return value


def _require_relative_path(value: str, field_name: str) -> str:
    if not isinstance(value, str) or not value or "\\" in value:
        raise ValueError(f"{field_name} must be a non-empty POSIX relative path")
    path = PurePosixPath(value)
    if path.is_absolute() or any(part in {"", ".", ".."} for part in path.parts):
        raise ValueError(f"{field_name} must stay within the repository")
    return path.as_posix()


@dataclass(frozen=True, slots=True)
class PlanToken:
    """Reviewed plan identity bound to its exact candidate and base revision."""

    schema_version: int
    requested_scope: str
    base_revision: str
    staged_digest: str
    diff_digest: str
    reviewed_adjudication_digests: tuple[str, ...] = ()
    token_id: str = ""

    def __post_init__(self) -> None:
        if self.schema_version != SCHEMA_VERSION:
            raise ValueError("unsupported plan-token schema version")
        if not isinstance(self.requested_scope, str) or not self.requested_scope.strip():
            raise ValueError("requested_scope must be a non-empty string")
        for name in ("base_revision", "staged_digest", "diff_digest"):
            _require_digest(getattr(self, name), name)
        for digest in self.reviewed_adjudication_digests:
            _require_digest(digest, "reviewed_adjudication_digests[]")
        if len(set(self.reviewed_adjudication_digests)) != len(
            self.reviewed_adjudication_digests
        ):
            raise ValueError("reviewed adjudication digests must be unique")
        expected = content_digest(self.binding_payload())
        if self.token_id and self.token_id != expected:
            raise ValueError("plan token_id does not match its bound inputs")
        object.__setattr__(self, "token_id", expected)

    def binding_payload(self) -> dict[str, Any]:
        return {
            "schema_version": self.schema_version,
            "requested_scope": self.requested_scope,
            "base_revision": self.base_revision,
            "staged_digest": self.staged_digest,
            "diff_digest": self.diff_digest,
            "reviewed_adjudication_digests": self.reviewed_adjudication_digests,
        }

    def to_dict(self) -> dict[str, Any]:
        return {**self.binding_payload(), "token_id": self.token_id}

    @classmethod
    def create(
        cls,
        *,
        requested_scope: str,
        base_revision: str,
        staged_digest: str,
        diff_digest: str,
        reviewed_adjudication_digests: Sequence[str] = (),
    ) -> "PlanToken":
        return cls(
            schema_version=SCHEMA_VERSION,
            requested_scope=requested_scope,
            base_revision=base_revision,
            staged_digest=staged_digest,
            diff_digest=diff_digest,
            reviewed_adjudication_digests=tuple(reviewed_adjudication_digests),
        )

    @classmethod
    def from_dict(cls, payload: Mapping[str, Any]) -> "PlanToken":
        return cls(
            schema_version=payload["schema_version"],
            requested_scope=payload["requested_scope"],
            base_revision=payload["base_revision"],
            staged_digest=payload["staged_digest"],
            diff_digest=payload["diff_digest"],
            reviewed_adjudication_digests=tuple(
                payload.get("reviewed_adjudication_digests", ())
            ),
            token_id=payload["token_id"],
        )


@dataclass(frozen=True, slots=True)
class RevisionMarker:
    """One committed content revision and hashes for every fixed-path file."""

    schema_version: int
    revision: str
    file_digests: Mapping[str, str]
    plan_token_id: str | None = None

    def __post_init__(self) -> None:
        if self.schema_version != SCHEMA_VERSION:
            raise ValueError("unsupported revision-marker schema version")
        if not self.file_digests:
            raise ValueError("revision marker must declare at least one artifact")
        normalized: dict[str, str] = {}
        for path, digest in self.file_digests.items():
            normalized[_require_relative_path(path, "file_digests path")] = _require_digest(
                digest, "file_digests digest"
            )
        expected = self.compute_revision(normalized)
        if self.revision != expected:
            raise ValueError("revision does not match declared file digests")
        if self.plan_token_id is not None:
            _require_digest(self.plan_token_id, "plan_token_id")
        object.__setattr__(
            self, "file_digests", MappingProxyType(dict(sorted(normalized.items())))
        )

    @staticmethod
    def compute_revision(file_digests: Mapping[str, str]) -> str:
        return content_digest(
            {
                "schema_version": SCHEMA_VERSION,
                "file_digests": dict(sorted(file_digests.items())),
            }
        )

    @classmethod
    def create(
        cls, file_digests: Mapping[str, str], *, plan_token_id: str | None = None
    ) -> "RevisionMarker":
        normalized = dict(sorted(file_digests.items()))
        return cls(
            schema_version=SCHEMA_VERSION,
            revision=cls.compute_revision(normalized),
            file_digests=normalized,
            plan_token_id=plan_token_id,
        )

    def to_dict(self) -> dict[str, Any]:
        return {
            "schema_version": self.schema_version,
            "revision": self.revision,
            "file_digests": dict(self.file_digests),
            "plan_token_id": self.plan_token_id,
        }

    @classmethod
    def from_dict(cls, payload: Mapping[str, Any]) -> "RevisionMarker":
        return cls(
            schema_version=payload["schema_version"],
            revision=payload["revision"],
            file_digests=payload["file_digests"],
            plan_token_id=payload.get("plan_token_id"),
        )


@dataclass(frozen=True, slots=True)
class ArtifactMutation:
    artifact_path: str
    staged_path: str
    new_digest: str
    prior_digest: str | None = None
    rollback_path: str | None = None

    def __post_init__(self) -> None:
        for name in ("artifact_path", "staged_path"):
            object.__setattr__(self, name, _require_relative_path(getattr(self, name), name))
        _require_digest(self.new_digest, "new_digest")
        if self.prior_digest is not None:
            _require_digest(self.prior_digest, "prior_digest")
            if self.rollback_path is None:
                raise ValueError("an existing artifact requires a rollback_path")
        elif self.rollback_path is not None:
            raise ValueError("a newly created artifact cannot have a rollback_path")
        if self.rollback_path is not None:
            object.__setattr__(
                self,
                "rollback_path",
                _require_relative_path(self.rollback_path, "rollback_path"),
            )

    def to_dict(self) -> dict[str, Any]:
        return {
            "artifact_path": self.artifact_path,
            "staged_path": self.staged_path,
            "new_digest": self.new_digest,
            "prior_digest": self.prior_digest,
            "rollback_path": self.rollback_path,
        }

    @classmethod
    def from_dict(cls, payload: Mapping[str, Any]) -> "ArtifactMutation":
        return cls(
            artifact_path=payload["artifact_path"],
            staged_path=payload["staged_path"],
            new_digest=payload["new_digest"],
            prior_digest=payload.get("prior_digest"),
            rollback_path=payload.get("rollback_path"),
        )


class TransactionState(StrEnum):
    PREPARED = "prepared"
    PUBLISHING = "publishing"
    ROLLING_BACK = "rolling_back"
    ROLLED_BACK = "rolled_back"
    COMMITTED = "committed"


@dataclass(frozen=True, slots=True)
class TransactionJournal:
    schema_version: int
    transaction_id: str
    plan_token_id: str
    requested_scope: str
    base_revision: str
    target_revision: str
    state: TransactionState
    mutations: tuple[ArtifactMutation, ...]
    replaced_paths: tuple[str, ...] = ()
    rolled_back_paths: tuple[str, ...] = ()
    prior_marker_backup_path: str | None = None
    rollback_reason: str | None = None

    def __post_init__(self) -> None:
        if self.schema_version != SCHEMA_VERSION:
            raise ValueError("unsupported transaction-journal schema version")
        for name in ("transaction_id", "plan_token_id", "base_revision", "target_revision"):
            _require_digest(getattr(self, name), name)
        if not self.requested_scope:
            raise ValueError("transaction requested_scope must not be empty")
        mutation_paths = [item.artifact_path for item in self.mutations]
        mutation_path_set = set(mutation_paths)
        if len(mutation_path_set) != len(mutation_paths):
            raise ValueError("transaction artifact paths must be unique")
        if any(path not in mutation_path_set for path in self.replaced_paths):
            raise ValueError("replaced_paths must refer to transaction artifacts")
        replaced_path_set = set(self.replaced_paths)
        if any(path not in replaced_path_set for path in self.rolled_back_paths):
            raise ValueError("rolled_back_paths must refer to replaced artifacts")
        if len(replaced_path_set) != len(self.replaced_paths):
            raise ValueError("replaced_paths must be unique")
        if len(set(self.rolled_back_paths)) != len(self.rolled_back_paths):
            raise ValueError("rolled_back_paths must be unique")
        if self.replaced_paths != tuple(mutation_paths[: len(self.replaced_paths)]):
            raise ValueError("replaced_paths must be a prefix of transaction order")
        rollback_order = tuple(reversed(self.replaced_paths))
        if self.rolled_back_paths != rollback_order[: len(self.rolled_back_paths)]:
            raise ValueError("rolled_back_paths must follow reverse replacement order")
        if self.prior_marker_backup_path is not None:
            object.__setattr__(
                self,
                "prior_marker_backup_path",
                _require_relative_path(
                    self.prior_marker_backup_path, "prior_marker_backup_path"
                ),
            )
        expected_transaction_id = content_digest(self.identity_payload())
        if self.transaction_id != expected_transaction_id:
            raise ValueError("transaction_id does not match journal identity inputs")
        if self.state in {TransactionState.ROLLING_BACK, TransactionState.ROLLED_BACK}:
            if not self.rollback_reason:
                raise ValueError("rollback state requires rollback_reason")
        elif self.rollback_reason is not None or self.rolled_back_paths:
            raise ValueError("non-rollback state cannot contain rollback progress")
        if self.state is TransactionState.PREPARED and self.replaced_paths:
            raise ValueError("prepared transaction cannot contain replacements")
        if self.state is TransactionState.COMMITTED and set(self.replaced_paths) != set(
            mutation_paths
        ):
            raise ValueError("committed transaction must replace every artifact")
        if self.state is TransactionState.ROLLED_BACK and set(
            self.rolled_back_paths
        ) != set(self.replaced_paths):
            raise ValueError("rolled-back transaction must restore every replaced artifact")

    @classmethod
    def begin(
        cls,
        plan: PlanToken,
        mutations: Sequence[ArtifactMutation],
        *,
        prior_marker_backup_path: str | None = None,
    ) -> "TransactionJournal":
        ordered = tuple(sorted(mutations, key=lambda item: item.artifact_path))
        identity = {
            "schema_version": SCHEMA_VERSION,
            "plan_token_id": plan.token_id,
            "requested_scope": plan.requested_scope,
            "base_revision": plan.base_revision,
            "target_revision": plan.staged_digest,
            "mutations": [item.to_dict() for item in ordered],
            "prior_marker_backup_path": prior_marker_backup_path,
        }
        return cls(
            schema_version=SCHEMA_VERSION,
            transaction_id=content_digest(identity),
            plan_token_id=plan.token_id,
            requested_scope=plan.requested_scope,
            base_revision=plan.base_revision,
            target_revision=plan.staged_digest,
            state=TransactionState.PREPARED,
            mutations=ordered,
            prior_marker_backup_path=prior_marker_backup_path,
        )

    def identity_payload(self) -> dict[str, Any]:
        return {
            "schema_version": self.schema_version,
            "plan_token_id": self.plan_token_id,
            "requested_scope": self.requested_scope,
            "base_revision": self.base_revision,
            "target_revision": self.target_revision,
            "mutations": [item.to_dict() for item in self.mutations],
            "prior_marker_backup_path": self.prior_marker_backup_path,
        }

    def start_publishing(self) -> "TransactionJournal":
        if self.state is not TransactionState.PREPARED:
            raise ValueError("only a prepared transaction can start publishing")
        return replace(self, state=TransactionState.PUBLISHING)

    def mark_replaced(self, artifact_path: str) -> "TransactionJournal":
        return self.mark_replaced_batch((artifact_path,))

    def mark_replaced_batch(
        self, artifact_paths: Sequence[str]
    ) -> "TransactionJournal":
        """Persist one write-ahead prefix before replacing its artifacts."""

        if self.state is not TransactionState.PUBLISHING:
            raise ValueError("artifact replacement requires publishing state")
        normalized = tuple(
            _require_relative_path(path, "artifact_path") for path in artifact_paths
        )
        if not normalized:
            raise ValueError("artifact replacement batch must not be empty")
        pending = tuple(item.artifact_path for item in self.mutations)[
            len(self.replaced_paths) :
        ]
        if not pending:
            raise ValueError("every transaction artifact is already replaced")
        expected = pending[: len(normalized)]
        if normalized != expected:
            raise ValueError(
                f"next artifact replacement batch must be {expected}"
            )
        return replace(
            self, replaced_paths=(*self.replaced_paths, *normalized)
        )

    def commit(self) -> "TransactionJournal":
        if self.state is not TransactionState.PUBLISHING:
            raise ValueError("only a publishing transaction can commit")
        return replace(self, state=TransactionState.COMMITTED)

    def start_rollback(self, reason: str) -> "TransactionJournal":
        if self.state not in {TransactionState.PREPARED, TransactionState.PUBLISHING}:
            raise ValueError("only an incomplete transaction can start rollback")
        if not reason:
            raise ValueError("rollback reason must not be empty")
        return replace(
            self, state=TransactionState.ROLLING_BACK, rollback_reason=reason
        )

    def start_committed_rollback(self, reason: str) -> "TransactionJournal":
        """Begin an explicit operator rollback of a successful publication.

        Interrupted-publication recovery intentionally cannot enter this path:
        callers must first prove that the committed journal still describes the
        current marker and supply an operator reason at the publication layer.
        """

        if self.state is not TransactionState.COMMITTED:
            raise ValueError("only a committed transaction can be explicitly rolled back")
        if not reason or not reason.strip():
            raise ValueError("rollback reason must not be empty")
        return replace(
            self,
            state=TransactionState.ROLLING_BACK,
            rollback_reason=reason.strip(),
        )

    def mark_rolled_back(self, artifact_path: str) -> "TransactionJournal":
        return self.mark_rolled_back_batch((artifact_path,))

    def mark_rolled_back_batch(
        self, artifact_paths: Sequence[str]
    ) -> "TransactionJournal":
        """Record one completely restored reverse-prefix batch."""

        if self.state is not TransactionState.ROLLING_BACK:
            raise ValueError("artifact rollback requires rolling_back state")
        normalized = tuple(
            _require_relative_path(path, "artifact_path") for path in artifact_paths
        )
        if not normalized:
            raise ValueError("artifact rollback batch must not be empty")
        expected_paths = tuple(reversed(self.replaced_paths))
        if len(self.rolled_back_paths) == len(expected_paths):
            raise ValueError("every replaced transaction artifact is already rolled back")
        expected = expected_paths[
            len(self.rolled_back_paths) : len(self.rolled_back_paths) + len(normalized)
        ]
        if normalized != expected:
            raise ValueError(f"next artifact rollback batch must be {expected}")
        return replace(
            self, rolled_back_paths=(*self.rolled_back_paths, *normalized)
        )

    def finish_rollback(self) -> "TransactionJournal":
        if self.state is not TransactionState.ROLLING_BACK:
            raise ValueError("only a rolling-back transaction can finish rollback")
        return replace(self, state=TransactionState.ROLLED_BACK)

    def to_dict(self) -> dict[str, Any]:
        return {
            "schema_version": self.schema_version,
            "transaction_id": self.transaction_id,
            "plan_token_id": self.plan_token_id,
            "requested_scope": self.requested_scope,
            "base_revision": self.base_revision,
            "target_revision": self.target_revision,
            "state": self.state.value,
            "mutations": [item.to_dict() for item in self.mutations],
            "replaced_paths": list(self.replaced_paths),
            "rolled_back_paths": list(self.rolled_back_paths),
            "prior_marker_backup_path": self.prior_marker_backup_path,
            "rollback_reason": self.rollback_reason,
        }

    @classmethod
    def from_dict(cls, payload: Mapping[str, Any]) -> "TransactionJournal":
        return cls(
            schema_version=payload["schema_version"],
            transaction_id=payload["transaction_id"],
            plan_token_id=payload["plan_token_id"],
            requested_scope=payload["requested_scope"],
            base_revision=payload["base_revision"],
            target_revision=payload["target_revision"],
            state=TransactionState(payload["state"]),
            mutations=tuple(
                ArtifactMutation.from_dict(item) for item in payload["mutations"]
            ),
            replaced_paths=tuple(payload.get("replaced_paths", ())),
            rolled_back_paths=tuple(payload.get("rolled_back_paths", ())),
            prior_marker_backup_path=payload.get("prior_marker_backup_path"),
            rollback_reason=payload.get("rollback_reason"),
        )


def _atomic_write_json(path: Path, payload: Mapping[str, Any]) -> None:
    try:
        path.parent.mkdir(parents=True, exist_ok=True)
    except OSError as exc:
        raise WriteError(
            "failed to create card-data transaction metadata directory",
            path=path.parent,
            details={"error": str(exc)},
        ) from exc
    body = canonical_json_bytes(payload)
    last_error: OSError | None = None
    for _ in range(8):
        # Marker and journal filenames are already stable contracts.  Keep the
        # sibling temporary name short instead of appending their full names,
        # a PID, and a UUID near Windows MAX_PATH.
        temporary = path.parent / f".j{uuid.uuid4().hex[:16]}"
        try:
            handle = temporary.open("xb")
        except FileExistsError as exc:
            last_error = exc
            continue
        try:
            with handle:
                handle.write(body)
                handle.flush()
                os.fsync(handle.fileno())
            os.replace(temporary, path)
            return
        except OSError as exc:
            last_error = exc
            try:
                temporary.unlink(missing_ok=True)
            except OSError:
                pass
            break
    raise WriteError(
        "failed to atomically write card-data transaction metadata",
        path=path,
        details={"error": str(last_error)},
    ) from last_error


def write_revision_marker(path: Path, marker: RevisionMarker) -> None:
    _atomic_write_json(path, marker.to_dict())


def _parse_revision_marker(raw: bytes, *, path: Path) -> RevisionMarker:
    try:
        payload = json.loads(raw.decode("utf-8"))
        return RevisionMarker.from_dict(payload)
    except (KeyError, TypeError, ValueError, UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise RecoveryRequiredError(
            "committed card-data revision marker is invalid",
            path=path,
            details={"error": str(exc)},
        ) from exc


def load_revision_marker(path: Path) -> RevisionMarker | None:
    if not path.exists():
        return None
    try:
        return _parse_revision_marker(path.read_bytes(), path=path)
    except OSError as exc:
        raise RecoveryRequiredError(
            "committed card-data revision marker cannot be read",
            path=path,
            details={"error": str(exc)},
        ) from exc


def write_transaction_journal(path: Path, journal: TransactionJournal) -> None:
    _atomic_write_json(path, journal.to_dict())


def load_transaction_journal(path: Path) -> TransactionJournal | None:
    if not path.exists():
        return None
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
        return TransactionJournal.from_dict(payload)
    except (OSError, KeyError, TypeError, ValueError, json.JSONDecodeError) as exc:
        raise RecoveryRequiredError(
            "card-data transaction journal is invalid and requires recovery",
            path=path,
            details={"error": str(exc)},
        ) from exc


def current_revision(marker_path: Path) -> str:
    marker = load_revision_marker(marker_path)
    return marker.revision if marker else EMPTY_REVISION


@contextmanager
def standalone_output_write_guard(
    repository_root: Path,
    output_paths: Sequence[Path],
    *,
    marker_path: Path,
    journal_path: Path,
    recover_stale_lock: bool = False,
) -> Iterator[None]:
    """Serialize legacy direct writes and reject publication-owned outputs.

    Before cutover, campaign/pool review outputs may still be written by their
    standalone commands.  Once a committed revision marker declares one of
    those paths, only the journaled publication path may replace it.  Holding
    the same repository writer lock across this ownership check and the write
    closes the race where cutover could otherwise begin between them.
    """

    root = repository_root.resolve()
    with RepositoryWriterLock(root, recover_stale=recover_stale_lock):
        marker = load_revision_marker(marker_path)
        journal = load_transaction_journal(journal_path)
        _assert_journal_matches_marker(journal, marker)
        if marker is not None:
            owned: list[str] = []
            for output_path in output_paths:
                resolved = output_path.resolve()
                try:
                    relative = resolved.relative_to(root).as_posix()
                except ValueError:
                    # Review artifacts outside the repository cannot be owned
                    # by its committed marker.
                    continue
                if relative in marker.file_digests:
                    owned.append(relative)
            if owned:
                raise WriteError(
                    "standalone write refused for publication-owned card-data artifacts",
                    details={
                        "paths": sorted(owned),
                        "committed_revision": marker.revision,
                        "required_writer": "sync publication plan/apply",
                    },
                )
        yield


def _assert_journal_matches_marker(
    journal: TransactionJournal | None, marker: RevisionMarker | None
) -> None:
    if journal is None:
        return
    if journal.state not in {TransactionState.COMMITTED, TransactionState.ROLLED_BACK}:
        raise RecoveryRequiredError(
            "incomplete card-data transaction journal requires recovery",
            details={
                "transaction_id": journal.transaction_id,
                "state": journal.state.value,
            },
        )
    expected = (
        journal.target_revision
        if journal.state is TransactionState.COMMITTED
        else journal.base_revision
    )
    actual = marker.revision if marker else EMPTY_REVISION
    if actual != expected:
        raise RecoveryRequiredError(
            "transaction journal does not match the committed revision marker",
            details={
                "transaction_id": journal.transaction_id,
                "journal_state": journal.state.value,
                "expected_revision": expected,
                "actual_revision": actual,
            },
        )


def committed_revision_protocol_active(
    *, marker_path: Path, journal_path: Path
) -> bool:
    """Return whether readers must use the committed-revision protocol.

    A completed rollback to ``EMPTY_REVISION`` deliberately restores the
    repository's pre-cutover state. Its terminal journal remains as audit and
    recovery evidence, so that one exact journal/marker combination is the
    only metadata-present state that still selects legacy-unversioned reads.
    """

    marker = load_revision_marker(marker_path)
    journal = load_transaction_journal(journal_path)
    if marker is None and journal is None:
        return False
    if (
        marker is None
        and journal is not None
        and journal.state is TransactionState.ROLLED_BACK
        and journal.base_revision == EMPTY_REVISION
    ):
        return False
    _assert_journal_matches_marker(journal, marker)
    if marker is None:
        # Only the terminal empty-base rollback above may omit its marker.
        raise RecoveryRequiredError(
            "committed card-data revision marker is unavailable",
            path=marker_path,
            details={
                "transaction_id": journal.transaction_id if journal else None,
                "journal_state": journal.state.value if journal else None,
            },
        )
    return True


def prepare_transaction(
    plan: PlanToken,
    mutations: Sequence[ArtifactMutation],
    *,
    marker_path: Path,
    journal_path: Path,
    writer_lock: RepositoryWriterLock,
    prior_marker_backup_path: str | None = None,
) -> TransactionJournal:
    """Atomically reserve a transaction after the optimistic base-revision CAS."""

    if not writer_lock.is_held:
        raise WriteError("transaction preparation requires the writer lock")
    marker_path = _require_repository_metadata_path(
        writer_lock.repository_root, marker_path, "revision marker"
    )
    journal_path = _require_repository_metadata_path(
        writer_lock.repository_root, journal_path, "transaction journal"
    )
    marker = load_revision_marker(marker_path)
    journal = load_transaction_journal(journal_path)
    _assert_journal_matches_marker(journal, marker)
    actual_revision = marker.revision if marker else EMPTY_REVISION
    if actual_revision != plan.base_revision:
        raise StalePlanError(
            "reviewed card-data plan is stale",
            details={
                "requested_scope": plan.requested_scope,
                "expected_base_revision": plan.base_revision,
                "actual_base_revision": actual_revision,
            },
        )
    prepared = TransactionJournal.begin(
        plan,
        mutations,
        prior_marker_backup_path=prior_marker_backup_path,
    )
    write_transaction_journal(journal_path, prepared)
    return prepared


@dataclass(frozen=True, slots=True)
class ConsistentRevision:
    revision: str
    values: Mapping[str, Any]
    marker: RevisionMarker


ArtifactLoader = Callable[[bytes], Any]


def _repository_artifact_path(repository_root: Path, relative_path: str) -> Path:
    relative_path = _require_relative_path(relative_path, "artifact path")
    root = repository_root.resolve()
    resolved = (root / PurePosixPath(relative_path)).resolve()
    if not resolved.is_relative_to(root):
        raise RecoveryRequiredError(
            "revision marker artifact escapes the repository",
            path=resolved,
        )
    return resolved


def _require_repository_metadata_path(
    repository_root: Path, path: Path, field_name: str
) -> Path:
    root = repository_root.resolve()
    resolved = path.resolve()
    if not resolved.is_relative_to(root):
        raise WriteError(f"{field_name} must stay within the locked repository", path=path)
    return resolved


def read_consistent_revision(
    repository_root: Path,
    *,
    marker_path: Path,
    journal_path: Path,
    loaders: Mapping[str, ArtifactLoader] | None = None,
    max_attempts: int = 2,
) -> ConsistentRevision:
    """Read and hash-check exactly one committed fixed-path revision.

    The marker and journal are checked both before and after artifact loading.
    A concurrent complete commit retries; an incomplete publication or hash
    mismatch fails immediately rather than returning mixed content.
    """

    if max_attempts < 1:
        raise ValueError("max_attempts must be at least 1")
    repository_root = repository_root.resolve()
    marker_path = _require_repository_metadata_path(
        repository_root, marker_path, "revision marker"
    )
    journal_path = _require_repository_metadata_path(
        repository_root, journal_path, "transaction journal"
    )
    loaders = loaders or {}

    for attempt in range(1, max_attempts + 1):
        journal_before = load_transaction_journal(journal_path)
        try:
            marker_before_raw = marker_path.read_bytes()
        except OSError as exc:
            raise RecoveryRequiredError(
                "committed card-data revision marker is unavailable",
                path=marker_path,
                details={"error": str(exc)},
            ) from exc
        marker_before = _parse_revision_marker(marker_before_raw, path=marker_path)
        _assert_journal_matches_marker(journal_before, marker_before)

        values: dict[str, Any] = {}
        for relative_path, expected_digest in marker_before.file_digests.items():
            path = _repository_artifact_path(repository_root, relative_path)
            try:
                raw = path.read_bytes()
            except OSError as exc:
                raise RecoveryRequiredError(
                    "committed card-data artifact is unavailable",
                    path=path,
                    details={"error": str(exc)},
                ) from exc
            actual_digest = sha256_hex(raw)
            if actual_digest != expected_digest:
                raise RecoveryRequiredError(
                    "committed card-data artifact failed digest validation",
                    path=path,
                    details={
                        "expected_digest": expected_digest,
                        "actual_digest": actual_digest,
                        "revision": marker_before.revision,
                    },
                )
            loader = loaders.get(relative_path, lambda value: value)
            values[relative_path] = loader(raw)

        journal_after = load_transaction_journal(journal_path)
        try:
            marker_after_raw = marker_path.read_bytes()
        except OSError as exc:
            raise RecoveryRequiredError(
                "committed card-data revision marker disappeared during read",
                path=marker_path,
                details={"error": str(exc)},
            ) from exc
        marker_after = _parse_revision_marker(marker_after_raw, path=marker_path)
        _assert_journal_matches_marker(journal_after, marker_after)

        if marker_after_raw == marker_before_raw:
            return ConsistentRevision(
                revision=marker_before.revision,
                values=MappingProxyType(values),
                marker=marker_before,
            )
        if attempt == max_attempts:
            raise ConcurrentRevisionError(
                "card-data revision changed while fixed-path artifacts were read",
                details={
                    "before_revision": marker_before.revision,
                    "after_revision": marker_after.revision,
                    "attempts": max_attempts,
                },
            )

    raise AssertionError("consistent revision loop exhausted")
