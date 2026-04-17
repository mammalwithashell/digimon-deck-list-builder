"""Desktop-side companion to the hosted `/models` catalog.

Responsibilities:

- Fetch the remote manifest from the hosted API and merge it with what's
  already bundled (shipped with the installer) or cached (downloaded by
  the user).
- Download a specific `(slug, version)` into the cache dir, resuming
  from a partial file if the last run was interrupted, and verifying
  the `sha256` against the manifest before committing the file.
- Resolve a model reference (slug OR legacy filename) to a local
  `.onnx` path the sidecar can load into `onnxruntime`.

Hosted API catalog lives in `digimon_gym/routers/models.py`. Keep the
JSON shapes aligned — the desktop client parses by name.
"""

from __future__ import annotations

import hashlib
import json
import logging
import os
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional

import httpx

logger = logging.getLogger(__name__)


MANIFEST_TTL_SECONDS = 5 * 60  # refetch no more than every 5 min


# ── Data shapes ─────────────────────────────────────────────────────────


@dataclass
class ModelVersionEntry:
    """Single downloadable version of a model. Mirrors the public
    `ModelVersionPublic` schema but adds client-side fields like
    `cache_path` and `is_cached`."""

    version: str
    size_bytes: int
    sha256: str
    url: str
    min_engine_version: Optional[str]
    onnx_input_shapes: Optional[Dict[str, Any]]
    changelog: str
    is_deprecated: bool
    released_at: str
    # Client-side derived:
    is_cached: bool = False
    cache_path: Optional[str] = None

    @classmethod
    def from_public(cls, data: Dict[str, Any]) -> "ModelVersionEntry":
        return cls(
            version=data["version"],
            size_bytes=data["size_bytes"],
            sha256=data["sha256"],
            url=data["url"],
            min_engine_version=data.get("min_engine_version"),
            onnx_input_shapes=data.get("onnx_input_shapes"),
            changelog=data.get("changelog", ""),
            is_deprecated=bool(data.get("is_deprecated")),
            released_at=data.get("released_at", ""),
        )


@dataclass
class ModelEntry:
    """One logical model as the desktop UI sees it: remote metadata
    fused with local cache/bundled state."""

    slug: str
    name: str
    arch: str
    description: str
    deck_pool: Optional[str]
    min_engine_version: str
    is_deprecated: bool
    versions: List[ModelVersionEntry] = field(default_factory=list)
    # Locally-bundled shipped files appear here too, so the UI can list
    # them in the same widget. A bundled model has no remote versions;
    # `bundled_filename` is set.
    bundled_filename: Optional[str] = None
    bundled_path: Optional[str] = None

    @property
    def latest_available(self) -> Optional[ModelVersionEntry]:
        candidates = [v for v in self.versions if not v.is_deprecated]
        return candidates[0] if candidates else None


# ── HTTP manifest fetch (with TTL cache) ────────────────────────────────


_manifest_cache: Dict[str, tuple[float, List[Dict[str, Any]]]] = {}


def fetch_remote_manifest(api_base: str, *, force: bool = False) -> List[Dict[str, Any]]:
    """GET `{api_base}/models`. Returns the raw list of records.

    Cached for `MANIFEST_TTL_SECONDS` to avoid hammering the hosted API
    on every `/games/models` sidecar lookup. Pass `force=True` from the
    "refresh" button in the UI.
    """
    now = time.time()
    cached = _manifest_cache.get(api_base)
    if cached and not force and now - cached[0] < MANIFEST_TTL_SECONDS:
        return cached[1]
    try:
        with httpx.Client(timeout=10.0) as http:
            resp = http.get(f"{api_base.rstrip('/')}/models")
            resp.raise_for_status()
            rows = resp.json()
            if not isinstance(rows, list):
                raise ValueError("manifest payload was not a list")
    except Exception as exc:
        logger.warning("failed to fetch remote manifest from %s: %s", api_base, exc)
        # Fall back to the stale cache if we have one so offline sessions
        # keep listing what they already know about.
        if cached:
            return cached[1]
        return []
    _manifest_cache[api_base] = (now, rows)
    return rows


def clear_manifest_cache() -> None:
    _manifest_cache.clear()


# ── Merge remote + cache + bundled ──────────────────────────────────────


def _version_cache_dir(cache_dir: Path, slug: str, version: str) -> Path:
    return cache_dir / slug / version


def _version_cache_path(cache_dir: Path, slug: str, version: str) -> Path:
    return _version_cache_dir(cache_dir, slug, version) / "model.onnx"


def merge_manifest(
    remote: List[Dict[str, Any]],
    bundled_dir: Optional[Path],
    cache_dir: Path,
) -> List[ModelEntry]:
    """Fuse the remote manifest with local bundled + cached state.

    Bundled models that don't appear in the remote manifest are still
    listed (e.g. a baseline greedy policy shipped with the installer).
    """
    cache_dir.mkdir(parents=True, exist_ok=True)
    entries: List[ModelEntry] = []
    seen_slugs: set[str] = set()

    for row in remote:
        slug = row.get("slug")
        if not slug:
            continue
        versions: List[ModelVersionEntry] = []
        for v_raw in row.get("versions", []):
            v = ModelVersionEntry.from_public(v_raw)
            cached = _version_cache_path(cache_dir, slug, v.version)
            if cached.exists() and cached.stat().st_size == v.size_bytes:
                v.is_cached = True
                v.cache_path = str(cached)
            versions.append(v)
        entries.append(
            ModelEntry(
                slug=slug,
                name=row.get("name", slug),
                arch=row.get("arch", "mlp"),
                description=row.get("description", ""),
                deck_pool=row.get("deck_pool"),
                min_engine_version=row.get("min_engine_version", "0.1.0"),
                is_deprecated=bool(row.get("is_deprecated")),
                versions=versions,
            )
        )
        seen_slugs.add(slug)

    # Surface bundled `.onnx` files as standalone entries so the UI can
    # select them. We key them by filename (no slug) and treat them as
    # always-cached.
    if bundled_dir and bundled_dir.exists():
        for bundled_file in sorted(bundled_dir.glob("*.onnx")):
            entries.append(
                ModelEntry(
                    slug=f"bundled:{bundled_file.stem}",
                    name=f"{bundled_file.stem} (bundled)",
                    arch="lstm" if "lstm" in bundled_file.stem.lower() else "mlp",
                    description="Shipped with this desktop build.",
                    deck_pool=None,
                    min_engine_version="0.1.0",
                    is_deprecated=False,
                    bundled_filename=bundled_file.name,
                    bundled_path=str(bundled_file),
                )
            )

    return entries


# ── Download + cache ────────────────────────────────────────────────────


ProgressCb = Callable[[int, int], None]  # (bytes_done, total_bytes)


class IntegrityError(RuntimeError):
    """Raised when a downloaded file's sha256 doesn't match the manifest."""


def download_model(
    entry: ModelEntry,
    version: ModelVersionEntry,
    cache_dir: Path,
    *,
    progress: Optional[ProgressCb] = None,
    chunk_size: int = 1 * 1024 * 1024,
) -> Path:
    """Download `version.url` into the cache dir, verify sha256, and
    return the final path. Resumes from a `.part` file if one exists
    from an interrupted prior run.

    Raises `IntegrityError` on sha256 mismatch; the partial file is
    discarded so the next attempt starts clean.
    """
    target_dir = _version_cache_dir(cache_dir, entry.slug, version.version)
    target_dir.mkdir(parents=True, exist_ok=True)
    final_path = target_dir / "model.onnx"
    if final_path.exists() and final_path.stat().st_size == version.size_bytes:
        # Already downloaded and size-correct. Verify hash before trusting.
        if _sha256_file(final_path) == version.sha256:
            return final_path
        # Size matched but hash didn't — treat as corruption.
        final_path.unlink()

    part_path = target_dir / "model.onnx.part"
    start_byte = part_path.stat().st_size if part_path.exists() else 0

    headers = {}
    if start_byte > 0:
        headers["Range"] = f"bytes={start_byte}-"

    hasher = hashlib.sha256()
    # If resuming, seed the hasher with the bytes already on disk.
    if start_byte > 0:
        with part_path.open("rb") as fh:
            while True:
                chunk = fh.read(chunk_size)
                if not chunk:
                    break
                hasher.update(chunk)

    bytes_written = start_byte
    with httpx.stream("GET", version.url, headers=headers, timeout=None) as resp:
        if resp.status_code == 200 and start_byte > 0:
            # Server ignored our Range request — restart from scratch.
            part_path.unlink(missing_ok=True)
            hasher = hashlib.sha256()
            bytes_written = 0
        elif resp.status_code not in (200, 206):
            raise RuntimeError(
                f"Download failed: HTTP {resp.status_code} from {version.url}"
            )
        mode = "ab" if bytes_written > 0 else "wb"
        with part_path.open(mode) as out:
            for chunk in resp.iter_bytes(chunk_size=chunk_size):
                out.write(chunk)
                hasher.update(chunk)
                bytes_written += len(chunk)
                if progress:
                    progress(bytes_written, version.size_bytes)

    digest = hasher.hexdigest()
    if digest != version.sha256:
        part_path.unlink(missing_ok=True)
        raise IntegrityError(
            f"sha256 mismatch for {entry.slug}@{version.version}: "
            f"expected {version.sha256}, got {digest}"
        )

    os.replace(part_path, final_path)
    return final_path


def delete_cached_version(cache_dir: Path, slug: str, version: str) -> bool:
    """Remove a cached version's files. Returns `True` if anything was
    actually deleted."""
    target = _version_cache_dir(cache_dir, slug, version)
    if not target.exists():
        return False
    for child in target.iterdir():
        child.unlink()
    target.rmdir()
    # Clean up empty parent slug dir so the cache doesn't accumulate
    # orphaned directory shells.
    try:
        target.parent.rmdir()
    except OSError:
        pass
    return True


def _sha256_file(path: Path, chunk_size: int = 1 * 1024 * 1024) -> str:
    h = hashlib.sha256()
    with path.open("rb") as fh:
        for chunk in iter(lambda: fh.read(chunk_size), b""):
            h.update(chunk)
    return h.hexdigest()


# ── Resolution for the sidecar's model loading ──────────────────────────


def resolve_model(
    reference: str,
    *,
    bundled_dir: Optional[Path],
    cache_dir: Optional[Path],
    prefer_version: Optional[str] = None,
) -> Optional[Path]:
    """Resolve a model `reference` (slug like `greedy-meta` or a legacy
    filename like `mlp_agent.onnx`) to a concrete `.onnx` path.

    Resolution order:
    1. If reference looks like a slug and `cache_dir` holds a matching
       cached version, return it (preferring `prefer_version`, then the
       newest by directory mtime).
    2. If reference is a filename that exists in `bundled_dir`, return
       that path.
    3. Otherwise return `None`.
    """
    if not reference:
        return None

    # Cached-slug path
    if cache_dir and cache_dir.exists():
        slug_dir = cache_dir / reference
        if slug_dir.is_dir():
            if prefer_version:
                candidate = slug_dir / prefer_version / "model.onnx"
                if candidate.exists():
                    return candidate
            # Fall back to most-recently-written version folder.
            version_dirs = [d for d in slug_dir.iterdir() if d.is_dir()]
            version_dirs.sort(key=lambda d: d.stat().st_mtime, reverse=True)
            for d in version_dirs:
                candidate = d / "model.onnx"
                if candidate.exists():
                    return candidate

    # Bundled-filename path (legacy `ONNX_MODELS_DIR` convention)
    if bundled_dir and bundled_dir.exists():
        safe = Path(reference).name
        candidate = bundled_dir / safe
        if candidate.exists():
            return candidate
        # Allow reference without `.onnx` extension.
        if not safe.endswith(".onnx"):
            candidate = bundled_dir / f"{safe}.onnx"
            if candidate.exists():
                return candidate

    return None
