"""Desktop-side `/models` catalog helpers.

Hits only the pure-logic surface (merge + resolve + sha integrity) —
network paths go through `httpx` and are covered end-to-end by the
hosted API's own test suite.
"""

import hashlib
from pathlib import Path

import pytest

from digimon_gym.engine.model_catalog import (
    IntegrityError,
    delete_cached_version,
    download_model,
    merge_manifest,
    resolve_model,
    ModelEntry,
    ModelVersionEntry,
)


def _write_bundled(models_dir: Path, filename: str, payload: bytes = b"bundled-bytes") -> Path:
    models_dir.mkdir(parents=True, exist_ok=True)
    p = models_dir / filename
    p.write_bytes(payload)
    return p


def _write_cached(cache_dir: Path, slug: str, version: str, payload: bytes) -> Path:
    target = cache_dir / slug / version
    target.mkdir(parents=True, exist_ok=True)
    p = target / "model.onnx"
    p.write_bytes(payload)
    return p


def _version_payload(
    slug: str, version: str, url: str, sha: str, size: int
) -> dict:
    return {
        "version": version,
        "size_bytes": size,
        "sha256": sha,
        "url": url,
        "min_engine_version": None,
        "onnx_input_shapes": None,
        "changelog": "",
        "is_deprecated": False,
        "released_at": "2026-04-17T00:00:00+00:00",
    }


class TestMergeManifest:
    def test_cached_version_marked_is_cached(self, tmp_path):
        cache = tmp_path / "cache"
        payload = b"cached-bytes"
        _write_cached(cache, "my-model", "1.0.0", payload)

        remote = [
            {
                "slug": "my-model",
                "name": "My Model",
                "arch": "mlp",
                "description": "",
                "deck_pool": None,
                "min_engine_version": "0.1.0",
                "is_deprecated": False,
                "versions": [
                    _version_payload(
                        "my-model",
                        "1.0.0",
                        "https://cdn.example/my-model/1.0.0/model.onnx",
                        hashlib.sha256(payload).hexdigest(),
                        len(payload),
                    )
                ],
            }
        ]
        entries = merge_manifest(remote, bundled_dir=None, cache_dir=cache)
        assert len(entries) == 1
        v = entries[0].versions[0]
        assert v.is_cached is True
        assert v.cache_path is not None

    def test_bundled_models_surface_as_standalone_entries(self, tmp_path):
        bundled = tmp_path / "bundled"
        _write_bundled(bundled, "mlp_agent.onnx")
        cache = tmp_path / "cache"
        entries = merge_manifest([], bundled_dir=bundled, cache_dir=cache)
        slugs = [e.slug for e in entries]
        assert any(s.startswith("bundled:") for s in slugs)
        bundled_entry = next(e for e in entries if e.slug.startswith("bundled:"))
        assert bundled_entry.bundled_filename == "mlp_agent.onnx"

    def test_remote_manifest_without_cache_marks_uncached(self, tmp_path):
        cache = tmp_path / "cache"
        remote = [
            {
                "slug": "fresh",
                "name": "Fresh",
                "arch": "lstm",
                "description": "",
                "deck_pool": None,
                "min_engine_version": "0.1.0",
                "is_deprecated": False,
                "versions": [
                    _version_payload("fresh", "2.0.0", "https://cdn.example/fresh/2.0.0/model.onnx", "abc", 42)
                ],
            }
        ]
        entries = merge_manifest(remote, bundled_dir=None, cache_dir=cache)
        assert entries[0].versions[0].is_cached is False


class TestResolveModel:
    def test_slug_prefers_cached_version(self, tmp_path):
        cache = tmp_path / "cache"
        _write_cached(cache, "greedy", "1.2.0", b"x")
        path = resolve_model("greedy", bundled_dir=None, cache_dir=cache)
        assert path is not None
        assert path.name == "model.onnx"
        assert "1.2.0" in str(path)

    def test_slug_picks_preferred_version(self, tmp_path):
        cache = tmp_path / "cache"
        _write_cached(cache, "greedy", "1.0.0", b"a")
        _write_cached(cache, "greedy", "2.0.0", b"b")
        path = resolve_model(
            "greedy", bundled_dir=None, cache_dir=cache, prefer_version="1.0.0"
        )
        assert "1.0.0" in str(path)

    def test_filename_resolves_against_bundled(self, tmp_path):
        bundled = tmp_path / "bundled"
        _write_bundled(bundled, "mlp_agent.onnx")
        path = resolve_model(
            "mlp_agent.onnx", bundled_dir=bundled, cache_dir=None
        )
        assert path is not None
        assert path.name == "mlp_agent.onnx"

    def test_filename_without_extension(self, tmp_path):
        bundled = tmp_path / "bundled"
        _write_bundled(bundled, "mlp_agent.onnx")
        path = resolve_model("mlp_agent", bundled_dir=bundled, cache_dir=None)
        assert path is not None

    def test_missing_returns_none(self, tmp_path):
        assert resolve_model(
            "nope", bundled_dir=tmp_path, cache_dir=tmp_path
        ) is None


class TestDownloadIntegrity:
    def test_download_refuses_on_sha_mismatch(self, tmp_path, monkeypatch):
        """Feed the downloader a mocked httpx.stream that delivers bytes
        whose hash doesn't match the manifest. Must raise IntegrityError
        and leave no `model.onnx` behind."""
        cache_dir = tmp_path / "cache"

        class _FakeResponse:
            status_code = 200

            def iter_bytes(self, chunk_size):
                yield b"actual-bytes-that-hash-to-something-else"

        class _FakeStreamCtx:
            def __enter__(self):
                return _FakeResponse()

            def __exit__(self, *args):
                return False

        monkeypatch.setattr(
            "digimon_gym.engine.model_catalog.httpx.stream",
            lambda *a, **kw: _FakeStreamCtx(),
        )

        entry = ModelEntry(
            slug="foo",
            name="Foo",
            arch="mlp",
            description="",
            deck_pool=None,
            min_engine_version="0.1.0",
            is_deprecated=False,
        )
        version = ModelVersionEntry(
            version="1.0.0",
            size_bytes=100,
            sha256="0" * 64,  # Will mismatch actual content
            url="https://example.com/foo.onnx",
            min_engine_version=None,
            onnx_input_shapes=None,
            changelog="",
            is_deprecated=False,
            released_at="2026-04-17T00:00:00+00:00",
        )

        with pytest.raises(IntegrityError):
            download_model(entry, version, cache_dir)

        # The quarantined partial should be deleted.
        final = cache_dir / "foo" / "1.0.0" / "model.onnx"
        assert not final.exists()


class TestDeleteCachedVersion:
    def test_removes_files_and_reports_true(self, tmp_path):
        _write_cached(tmp_path, "foo", "1.0.0", b"x")
        assert delete_cached_version(tmp_path, "foo", "1.0.0") is True
        assert not (tmp_path / "foo" / "1.0.0").exists()

    def test_missing_returns_false(self, tmp_path):
        assert delete_cached_version(tmp_path, "nope", "0.0.0") is False
