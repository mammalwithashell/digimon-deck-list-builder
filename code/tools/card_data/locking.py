"""Repository-scoped writer locking for card-data publication.

The operating-system advisory lock is the exclusivity oracle.  Persisted JSON
metadata makes ownership inspectable and makes crash recovery explicit: an
``active`` record that can be locked is stale because the owning process no
longer holds the OS lock.  Recovery never breaks a lock that the OS still
reports as held.
"""

from __future__ import annotations

import json
import os
import platform
import socket
import time
import uuid
from dataclasses import dataclass
from pathlib import Path
from types import TracebackType
from typing import Any, BinaryIO, Callable

from .errors import WriteError
from .serialization import canonical_json_bytes


class WriterLockUnavailable(WriteError):
    """Raised when another process owns the repository writer lock."""


class StaleWriterLock(WriteError):
    """Raised when stale owner metadata requires explicit recovery."""


@dataclass(frozen=True, slots=True)
class LockOwner:
    schema_version: int
    lock_id: str
    repository_root: str
    pid: int
    hostname: str
    acquired_at: float
    status: str = "active"
    released_at: float | None = None
    recovered_lock_id: str | None = None

    def __post_init__(self) -> None:
        if self.schema_version != 1:
            raise ValueError("unsupported writer-lock schema version")
        if not self.lock_id:
            raise ValueError("writer-lock lock_id must not be empty")
        if self.pid <= 0:
            raise ValueError("writer-lock pid must be positive")
        if self.status not in {"active", "released"}:
            raise ValueError("writer-lock status must be active or released")
        if self.status == "released" and self.released_at is None:
            raise ValueError("released writer-lock metadata requires released_at")

    def to_dict(self) -> dict[str, Any]:
        return {
            "schema_version": self.schema_version,
            "lock_id": self.lock_id,
            "repository_root": self.repository_root,
            "pid": self.pid,
            "hostname": self.hostname,
            "acquired_at": self.acquired_at,
            "status": self.status,
            "released_at": self.released_at,
            "recovered_lock_id": self.recovered_lock_id,
        }

    @classmethod
    def from_dict(cls, payload: dict[str, Any]) -> "LockOwner":
        return cls(
            schema_version=payload["schema_version"],
            lock_id=payload["lock_id"],
            repository_root=payload["repository_root"],
            pid=payload["pid"],
            hostname=payload["hostname"],
            acquired_at=payload["acquired_at"],
            status=payload.get("status", "active"),
            released_at=payload.get("released_at"),
            recovered_lock_id=payload.get("recovered_lock_id"),
        )


def _try_platform_lock(handle: BinaryIO) -> bool:
    handle.seek(0)
    if os.name == "nt":
        import msvcrt

        try:
            msvcrt.locking(handle.fileno(), msvcrt.LK_NBLCK, 1)
        except OSError:
            return False
        return True

    import fcntl

    try:
        fcntl.flock(handle.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
    except BlockingIOError:
        return False
    return True


def _platform_unlock(handle: BinaryIO) -> None:
    handle.seek(0)
    if os.name == "nt":
        import msvcrt

        msvcrt.locking(handle.fileno(), msvcrt.LK_UNLCK, 1)
        return

    import fcntl

    fcntl.flock(handle.fileno(), fcntl.LOCK_UN)


class RepositoryWriterLock:
    """One exclusive card-data publication lock for a repository.

    ``recover_stale`` is intentionally opt-in.  A previous process that died
    leaves ``active`` metadata behind, but its OS lock is gone.  The next owner
    must explicitly acknowledge that stale record before overwriting it.
    """

    def __init__(
        self,
        repository_root: str | os.PathLike[str],
        *,
        lock_path: str | os.PathLike[str] | None = None,
        timeout: float = 0.0,
        poll_interval: float = 0.05,
        recover_stale: bool = False,
        clock: Callable[[], float] = time.time,
    ) -> None:
        if timeout < 0:
            raise ValueError("writer-lock timeout cannot be negative")
        if poll_interval <= 0:
            raise ValueError("writer-lock poll_interval must be positive")
        self.repository_root = Path(repository_root).resolve()
        candidate = (
            Path(lock_path)
            if lock_path is not None
            else Path("data/.card_data_writer.lock")
        )
        if not candidate.is_absolute():
            candidate = self.repository_root / candidate
        self.lock_path = candidate.resolve()
        if not self.lock_path.is_relative_to(self.repository_root):
            raise ValueError("writer-lock path must stay within the repository")
        self.timeout = timeout
        self.poll_interval = poll_interval
        self.recover_stale = recover_stale
        self._clock = clock
        self._handle: BinaryIO | None = None
        self._owner: LockOwner | None = None

    @property
    def is_held(self) -> bool:
        return self._handle is not None and self._owner is not None

    @property
    def owner(self) -> LockOwner | None:
        return self._owner

    def _open_lock_file(self) -> BinaryIO:
        try:
            self.lock_path.parent.mkdir(parents=True, exist_ok=True)
            handle = self.lock_path.open("a+b")
            handle.seek(0, os.SEEK_END)
            if handle.tell() == 0:
                handle.write(b"\0")
                handle.flush()
            return handle
        except OSError as exc:
            raise WriteError(
                "failed to open repository card-data writer lock",
                path=self.lock_path,
                details={"error": str(exc)},
            ) from exc

    @staticmethod
    def _read_prior_owner(handle: BinaryIO) -> tuple[LockOwner | None, bool]:
        handle.seek(0)
        raw = handle.read().strip(b"\0\r\n\t ")
        if not raw:
            return None, False
        try:
            payload = json.loads(raw.decode("utf-8"))
            if not isinstance(payload, dict):
                return None, True
            return LockOwner.from_dict(payload), False
        except (KeyError, TypeError, ValueError, UnicodeDecodeError, json.JSONDecodeError):
            return None, True

    @staticmethod
    def _write_owner(handle: BinaryIO, owner: LockOwner) -> None:
        handle.seek(0)
        handle.truncate()
        handle.write(canonical_json_bytes(owner.to_dict()))
        handle.flush()
        os.fsync(handle.fileno())

    def acquire(self) -> "RepositoryWriterLock":
        if self.is_held:
            raise RuntimeError("repository writer lock is already held by this object")

        handle = self._open_lock_file()
        deadline = time.monotonic() + self.timeout
        while not _try_platform_lock(handle):
            if time.monotonic() >= deadline:
                handle.close()
                raise WriterLockUnavailable(
                    "repository card-data writer lock is owned by another process",
                    path=self.lock_path,
                    details={"timeout_seconds": self.timeout},
                )
            time.sleep(min(self.poll_interval, max(0.0, deadline - time.monotonic())))

        prior_owner, malformed = self._read_prior_owner(handle)
        stale = malformed or (prior_owner is not None and prior_owner.status == "active")
        if stale and not self.recover_stale:
            try:
                _platform_unlock(handle)
            finally:
                handle.close()
            raise StaleWriterLock(
                "stale card-data writer-lock metadata requires explicit recovery",
                path=self.lock_path,
                details={
                    "malformed_metadata": malformed,
                    "stale_owner": prior_owner.to_dict() if prior_owner else None,
                },
            )

        now = self._clock()
        owner = LockOwner(
            schema_version=1,
            lock_id=str(uuid.uuid4()),
            repository_root=str(self.repository_root),
            pid=os.getpid(),
            hostname=socket.gethostname() or platform.node() or "unknown",
            acquired_at=now,
            recovered_lock_id=prior_owner.lock_id if stale and prior_owner else None,
        )
        try:
            self._write_owner(handle, owner)
        except OSError as exc:
            try:
                _platform_unlock(handle)
            finally:
                handle.close()
            raise WriteError(
                "failed to record repository writer-lock ownership",
                path=self.lock_path,
                details={"error": str(exc)},
            ) from exc

        self._handle = handle
        self._owner = owner
        return self

    def release(self) -> None:
        if not self.is_held:
            return
        assert self._handle is not None
        assert self._owner is not None
        handle = self._handle
        owner = LockOwner(
            **{
                **self._owner.to_dict(),
                "status": "released",
                "released_at": self._clock(),
            }
        )
        write_error: OSError | None = None
        try:
            self._write_owner(handle, owner)
        except OSError as exc:
            write_error = exc
        finally:
            try:
                _platform_unlock(handle)
            finally:
                handle.close()
                self._handle = None
                self._owner = None
        if write_error is not None:
            raise WriteError(
                "failed to record repository writer-lock release",
                path=self.lock_path,
                details={"error": str(write_error)},
            ) from write_error

    def __enter__(self) -> "RepositoryWriterLock":
        return self.acquire()

    def __exit__(
        self,
        exc_type: type[BaseException] | None,
        exc: BaseException | None,
        traceback: TracebackType | None,
    ) -> bool:
        try:
            self.release()
        except WriteError:
            if exc is None:
                raise
        return False
