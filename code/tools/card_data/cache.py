"""Transport retry and payload-addressed response caching.

The response cache is an acquisition aid, not a canonical source of truth.  It
stores successful response bodies under their SHA-256 digest and never turns a
parser/schema failure into an offline cache fallback.
"""

from __future__ import annotations

import os
import socket
import time
import urllib.error
import urllib.request
from dataclasses import dataclass
from pathlib import Path
from typing import Callable, Protocol

from .errors import HttpError, NetworkError, ValidationError, WriteError
from .models import SHA256_RE
from .serialization import sha256_hex


DEFAULT_RETRYABLE_HTTP_STATUSES = frozenset({408, 425, 429, 500, 502, 503, 504})


class ResponseLike(Protocol):
    """Minimal response surface used by :func:`fetch_with_retry`."""

    status: int

    def read(self) -> bytes: ...

    def close(self) -> None: ...


OpenUrl = Callable[..., ResponseLike]
Sleep = Callable[[float], None]


@dataclass(frozen=True, slots=True)
class RetryPolicy:
    """Bounded retry policy for transient transport and HTTP failures."""

    max_attempts: int = 3
    initial_backoff_seconds: float = 0.25
    backoff_multiplier: float = 2.0
    retryable_http_statuses: frozenset[int] = DEFAULT_RETRYABLE_HTTP_STATUSES

    def __post_init__(self) -> None:
        if self.max_attempts < 1:
            raise ValueError("max_attempts must be at least 1")
        if self.initial_backoff_seconds < 0:
            raise ValueError("initial_backoff_seconds cannot be negative")
        if self.backoff_multiplier < 1:
            raise ValueError("backoff_multiplier must be at least 1")


@dataclass(frozen=True, slots=True)
class FetchedPayload:
    """One successfully received response and its content identity."""

    source_url: str
    body: bytes
    payload_digest: str
    attempts: int
    cache_path: Path | None = None


class ContentAddressedResponseCache:
    """Store raw response bytes by payload digest.

    There is intentionally no mutable URL-to-latest pointer here.  A caller may
    reuse a known digest explicitly, but a failed live request is never silently
    relabeled as a successful current observation.
    """

    def __init__(self, root: str | os.PathLike[str]) -> None:
        self.root = Path(root)

    def path_for(self, digest: str) -> Path:
        if not isinstance(digest, str) or not SHA256_RE.fullmatch(digest):
            raise ValidationError(
                "response-cache key must be a lowercase sha256 digest",
                field_path="payload_digest",
            )
        return self.root / digest[:2] / f"{digest}.bin"

    def put(self, body: bytes) -> tuple[str, Path]:
        if not isinstance(body, bytes):
            raise TypeError("cached response body must be bytes")
        digest = sha256_hex(body)
        destination = self.path_for(digest)
        try:
            destination.parent.mkdir(parents=True, exist_ok=True)
            if destination.exists():
                existing = destination.read_bytes()
                if sha256_hex(existing) != digest:
                    raise ValidationError(
                        "content-addressed response cache contains corrupt bytes",
                        path=destination,
                        details={"expected_digest": digest},
                    )
                return digest, destination

            temporary = destination.with_name(
                f".{destination.name}.{os.getpid()}.tmp"
            )
            temporary.write_bytes(body)
            os.replace(temporary, destination)
        except ValidationError:
            raise
        except OSError as exc:
            raise WriteError(
                "failed to write content-addressed response cache",
                path=destination,
                details={"error": str(exc)},
            ) from exc
        return digest, destination

    def get(self, digest: str) -> bytes:
        path = self.path_for(digest)
        try:
            body = path.read_bytes()
        except OSError as exc:
            raise ValidationError(
                "content-addressed response payload is unavailable",
                path=path,
                details={"payload_digest": digest},
            ) from exc
        actual = sha256_hex(body)
        if actual != digest:
            raise ValidationError(
                "content-addressed response payload failed digest validation",
                path=path,
                details={"expected_digest": digest, "actual_digest": actual},
            )
        return body


def _network_error(exc: BaseException, *, url: str, attempt: int) -> NetworkError:
    return NetworkError(
        "source request failed before an HTTP response was received",
        source=url,
        details={"attempt": attempt, "error": str(exc), "source_url": url},
    )


def _http_error(
    *, url: str, status_code: int, attempt: int, reason: str | None = None
) -> HttpError:
    return HttpError(
        f"source request returned HTTP {status_code}",
        status_code=status_code,
        source_url=url,
        source=url,
        details={"attempt": attempt, **({"reason": reason} if reason else {})},
    )


def fetch_with_retry(
    url: str,
    *,
    cache: ContentAddressedResponseCache | None = None,
    policy: RetryPolicy = RetryPolicy(),
    timeout: float = 30.0,
    opener: OpenUrl = urllib.request.urlopen,
    sleep: Sleep = time.sleep,
    user_agent: str = "digimon-deck-list-builder/card-data",
) -> FetchedPayload:
    """Fetch ``url`` with bounded retries and exact failure classification.

    Only transport errors and explicitly retryable HTTP statuses are retried.
    Parsing and source-schema validation occur after this function and retain
    their own typed error categories.
    """

    request = urllib.request.Request(url, headers={"User-Agent": user_agent})
    for attempt in range(1, policy.max_attempts + 1):
        response: ResponseLike | None = None
        error: NetworkError | HttpError | None = None
        retryable = False
        try:
            response = opener(request, timeout=timeout)
            status = int(getattr(response, "status", 200) or 200)
            if status < 200 or status >= 300:
                error = _http_error(url=url, status_code=status, attempt=attempt)
                retryable = status in policy.retryable_http_statuses
            else:
                body = response.read()
                if not isinstance(body, bytes):
                    raise TypeError("source response read() must return bytes")
                payload_digest = sha256_hex(body)
                cache_path = None
                if cache is not None:
                    cached_digest, cache_path = cache.put(body)
                    if cached_digest != payload_digest:  # defensive invariant
                        raise ValidationError(
                            "response cache returned an inconsistent payload digest"
                        )
                return FetchedPayload(
                    source_url=url,
                    body=body,
                    payload_digest=payload_digest,
                    attempts=attempt,
                    cache_path=cache_path,
                )
        except urllib.error.HTTPError as exc:
            error = _http_error(
                url=url,
                status_code=int(exc.code),
                attempt=attempt,
                reason=str(exc.reason) if exc.reason is not None else None,
            )
            retryable = int(exc.code) in policy.retryable_http_statuses
        except urllib.error.URLError as exc:
            error = _network_error(exc, url=url, attempt=attempt)
            retryable = True
        except (socket.timeout, TimeoutError, ConnectionError) as exc:
            error = _network_error(exc, url=url, attempt=attempt)
            retryable = True
        except OSError as exc:
            # TLS, socket read, connection reset, and DNS implementations may
            # surface as plain OSError subclasses rather than URLError.
            error = _network_error(exc, url=url, attempt=attempt)
            retryable = True
        finally:
            if response is not None:
                close = getattr(response, "close", None)
                if callable(close):
                    try:
                        close()
                    except OSError:
                        # The body/status already define the acquisition result;
                        # a best-effort close failure must not reclassify it.
                        pass

        if error is None:
            raise RuntimeError("source fetch ended without a result or diagnostic")
        if not retryable or attempt == policy.max_attempts:
            raise error
        delay = policy.initial_backoff_seconds * (
            policy.backoff_multiplier ** (attempt - 1)
        )
        sleep(delay)

    raise AssertionError("retry loop exhausted without returning or raising")
