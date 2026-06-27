"""Operator API-key auth for the model-catalog admin routes.

Login-free model release: the admin gate accepts a static ADMIN_API_KEY (via
X-Admin-Key or Authorization: Bearer) and falls back to admin-JWT. These cover
the key path + the no-auth rejection without needing a DB (the key path and the
401 short-circuit before any DB access); the admin-JWT fallback is covered by the
catalog integration test.
"""
from __future__ import annotations

import pytest
from fastapi import HTTPException

from server.db.auth import admin_api_key, require_admin_or_api_key


def test_admin_api_key_reads_env(monkeypatch):
    monkeypatch.delenv("ADMIN_API_KEY", raising=False)
    assert admin_api_key() is None
    monkeypatch.setenv("ADMIN_API_KEY", "")
    assert admin_api_key() is None  # empty string => disabled
    monkeypatch.setenv("ADMIN_API_KEY", "s3cret")
    assert admin_api_key() == "s3cret"


async def test_key_via_x_admin_key_header(monkeypatch):
    monkeypatch.setenv("ADMIN_API_KEY", "s3cret")
    user = await require_admin_or_api_key(x_admin_key="s3cret", authorization=None, db=None)
    assert user is None  # operator authenticated, no user record


async def test_key_via_authorization_bearer(monkeypatch):
    monkeypatch.setenv("ADMIN_API_KEY", "s3cret")
    user = await require_admin_or_api_key(x_admin_key=None, authorization="Bearer s3cret", db=None)
    assert user is None


async def test_wrong_key_and_no_token_rejected(monkeypatch):
    monkeypatch.setenv("ADMIN_API_KEY", "s3cret")
    with pytest.raises(HTTPException) as exc:
        await require_admin_or_api_key(x_admin_key="nope", authorization=None, db=None)
    assert exc.value.status_code == 401


async def test_no_key_configured_falls_through_to_auth(monkeypatch):
    # With no ADMIN_API_KEY set, a stray X-Admin-Key buys nothing — still needs a token.
    monkeypatch.delenv("ADMIN_API_KEY", raising=False)
    with pytest.raises(HTTPException) as exc:
        await require_admin_or_api_key(x_admin_key="anything", authorization=None, db=None)
    assert exc.value.status_code == 401
