"""Regression tests for code/tools/publish_model.py — the three defects that
broke prod model uploads (2026-06-28):

1. The presigned Spaces PUT must send `x-amz-acl: public-read` (the URL signs
   ACL=public-read) or Spaces rejects the signature.
2. Admin auth must support the static ADMIN_API_KEY via the X-Admin-Key header
   (prod has no seeded admin user; /auth/login 401s).
3. It must be able to set `starter_deck` and to publish (PATCH published=true).

HTTP is fully mocked — no network.
"""
from __future__ import annotations

import pytest

from tools import publish_model as pm


class FakeResp:
    def __init__(self, status: int, payload: dict | None = None, text: str = ""):
        self.status_code = status
        self._payload = payload or {}
        self.text = text

    def json(self) -> dict:
        return self._payload


class FakeSession:
    """Records API calls; returns canned responses keyed by URL suffix."""

    def __init__(self):
        self.headers: dict[str, str] = {}
        self.calls: list[tuple[str, str, dict | None]] = []

    def post(self, url, json=None):
        self.calls.append(("POST", url, json))
        if url.endswith("/admin/models"):
            return FakeResp(201, {"id": "mid1", "upload_url": "https://spaces.example/put", "spaces_key": "k", "expires_in": 900})
        if url.endswith("/confirm"):
            return FakeResp(200, {
                "id": "mid1", "state": "uploaded", "file_sha256": "a" * 64,
                "file_size_bytes": 1, "tensor_size": 8850, "action_space_size": 2192,
            })
        if url.endswith("/auth/login"):
            return FakeResp(200, {"access_token": "tok"})
        raise AssertionError(f"unexpected POST {url}")

    def patch(self, url, json=None):
        self.calls.append(("PATCH", url, json))
        return FakeResp(200, {"id": "mid1"})


@pytest.fixture
def onnx_file(tmp_path):
    p = tmp_path / "m.onnx"
    p.write_bytes(b"\x00" * 1024)
    return p


@pytest.fixture
def wired(monkeypatch):
    """Patch the module's requests.Session + requests.put; capture PUTs."""
    session = FakeSession()
    put_calls: list[dict] = []

    def fake_put(url, data=None, headers=None, timeout=None):
        put_calls.append({"url": url, "headers": dict(headers or {}), "size": len(data or b"")})
        return FakeResp(200)

    monkeypatch.setattr(pm.requests, "Session", lambda: session)
    monkeypatch.setattr(pm.requests, "put", fake_put)
    return session, put_calls


def _create_payload(session: FakeSession) -> dict:
    for method, url, body in session.calls:
        if method == "POST" and url.endswith("/admin/models"):
            return body
    raise AssertionError("no create call recorded")


def test_put_sends_acl_and_content_type_headers(wired, onnx_file):
    session, put_calls = wired
    pm.publish_model(str(onnx_file), "m", "mlp", "https://api.test", admin_key="K")
    assert len(put_calls) == 1
    headers = put_calls[0]["headers"]
    # Both signed headers MUST be present (defect #1).
    assert headers.get("x-amz-acl") == "public-read"
    assert headers.get("Content-Type") == "application/octet-stream"


def test_admin_key_auth_uses_header_not_login(wired, onnx_file):
    session, _ = wired
    pm.publish_model(str(onnx_file), "m", "mlp", "https://api.test", admin_key="SECRET")
    # X-Admin-Key set, no JWT, no login round-trip (defect #2).
    assert session.headers.get("X-Admin-Key") == "SECRET"
    assert "Authorization" not in session.headers
    assert not any(url.endswith("/auth/login") for _, url, _ in session.calls)


def test_login_fallback_when_no_admin_key(wired, onnx_file):
    session, _ = wired
    pm.publish_model(str(onnx_file), "m", "lstm", "https://api.test", username="u", password="p")
    assert any(url.endswith("/auth/login") for _, url, _ in session.calls)
    assert session.headers.get("Authorization") == "Bearer tok"
    assert "X-Admin-Key" not in session.headers


def test_starter_deck_in_create_payload(wired, onnx_file):
    session, _ = wired
    pm.publish_model(
        str(onnx_file), "ST-3", "mlp", "https://api.test",
        admin_key="K", starter_deck="starter_st3_heavens_yellow",
    )
    assert _create_payload(session)["starter_deck"] == "starter_st3_heavens_yellow"


def test_starter_deck_omitted_when_unset(wired, onnx_file):
    session, _ = wired
    pm.publish_model(str(onnx_file), "m", "mlp", "https://api.test", admin_key="K")
    assert "starter_deck" not in _create_payload(session)


def test_publish_flag_patches_published_true(wired, onnx_file):
    session, _ = wired
    pm.publish_model(str(onnx_file), "m", "mlp", "https://api.test", admin_key="K", publish=True)
    patches = [(url, body) for method, url, body in session.calls if method == "PATCH"]
    assert patches and patches[0][1] == {"published": True}


def test_no_publish_by_default(wired, onnx_file):
    session, _ = wired
    pm.publish_model(str(onnx_file), "m", "mlp", "https://api.test", admin_key="K")
    assert not any(method == "PATCH" for method, _, _ in session.calls)


def test_put_retries_then_fails_with_exit(wired, onnx_file, monkeypatch):
    session, _ = wired

    def always_reset(url, data=None, headers=None, timeout=None):
        raise pm.requests.exceptions.ConnectionError("reset")

    monkeypatch.setattr(pm.requests, "put", always_reset)
    monkeypatch.setattr(pm.time, "sleep", lambda *_: None)  # don't actually back off
    with pytest.raises(SystemExit):
        pm.publish_model(str(onnx_file), "m", "mlp", "https://api.test", admin_key="K")
