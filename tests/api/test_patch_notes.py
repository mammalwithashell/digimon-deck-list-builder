"""Tests for the patch notes endpoints (releases + known issues)."""

from __future__ import annotations

import pytest
from httpx import ASGITransport, AsyncClient
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession, async_sessionmaker, create_async_engine

from digimon_gym.db.auth import ROLE_ADMIN, assign_role_to_user
from digimon_gym.db.database import get_db
from digimon_gym.db.models import Base, User


@pytest.fixture
async def db_engine():
    engine = create_async_engine(
        "sqlite+aiosqlite:///:memory:",
        echo=False,
        connect_args={"check_same_thread": False},
    )
    async with engine.begin() as conn:
        await conn.run_sync(Base.metadata.create_all)
    yield engine
    async with engine.begin() as conn:
        await conn.run_sync(Base.metadata.drop_all)
    await engine.dispose()


@pytest.fixture
async def session_factory(db_engine):
    return async_sessionmaker(db_engine, class_=AsyncSession, expire_on_commit=False)


@pytest.fixture
async def client(session_factory, monkeypatch):
    from digimon_gym.api import app, ai_task_worker, training_job_worker

    async def override_get_db():
        async with session_factory() as session:
            yield session

    # Disable background workers for deterministic endpoint tests.
    # monkeypatch auto-reverts after the test so global state is restored.
    async def _noop():
        return None

    monkeypatch.setattr(ai_task_worker, "start", _noop)
    monkeypatch.setattr(ai_task_worker, "stop", _noop)
    monkeypatch.setattr(training_job_worker, "start", _noop)
    monkeypatch.setattr(training_job_worker, "stop", _noop)

    app.dependency_overrides[get_db] = override_get_db
    transport = ASGITransport(app=app)
    async with AsyncClient(transport=transport, base_url="http://test") as c:
        yield c
    app.dependency_overrides.clear()


async def _register_and_login(client: AsyncClient, username: str) -> str:
    await client.post(
        "/auth/register",
        json={
            "username": username,
            "email": f"{username}@example.com",
            "password": "secure-password-123",
        },
    )
    login = await client.post(
        "/auth/login",
        json={"username": username, "password": "secure-password-123"},
    )
    assert login.status_code == 200
    return login.json()["access_token"]


async def _grant_admin(session_factory, username: str) -> None:
    async with session_factory() as db:
        user = (
            await db.execute(select(User).where(User.username == username))
        ).scalar_one()
        await assign_role_to_user(db, user.id, ROLE_ADMIN)
        await db.commit()


class TestPatchNotesPublicRead:
    async def test_get_empty(self, client: AsyncClient):
        resp = await client.get("/patch-notes")
        assert resp.status_code == 200
        body = resp.json()
        assert body == {"known_issues": [], "releases": []}

    async def test_get_does_not_require_auth(self, client: AsyncClient):
        # No Authorization header.
        resp = await client.get("/patch-notes")
        assert resp.status_code == 200


class TestKnownIssueAdminFlow:
    async def test_non_admin_cannot_create(self, client: AsyncClient):
        token = await _register_and_login(client, "plainuser")
        resp = await client.post(
            "/patch-notes/known-issues",
            headers={"Authorization": f"Bearer {token}"},
            json={"title": "Nope", "description": ""},
        )
        assert resp.status_code == 403

    async def test_admin_create_and_public_read(self, client: AsyncClient, session_factory):
        token = await _register_and_login(client, "adminone")
        await _grant_admin(session_factory, "adminone")

        create = await client.post(
            "/patch-notes/known-issues",
            headers={"Authorization": f"Bearer {token}"},
            json={"title": "Replays intermittently crash", "description": "Happens during turn 2."},
        )
        assert create.status_code == 201
        issue = create.json()
        assert issue["title"] == "Replays intermittently crash"
        assert issue["description"] == "Happens during turn 2."
        assert "id" in issue

        # Public read (no auth) sees it.
        public = await client.get("/patch-notes")
        assert public.status_code == 200
        body = public.json()
        assert len(body["known_issues"]) == 1
        assert body["known_issues"][0]["id"] == issue["id"]

    async def test_admin_update(self, client: AsyncClient, session_factory):
        token = await _register_and_login(client, "admintwo")
        await _grant_admin(session_factory, "admintwo")
        headers = {"Authorization": f"Bearer {token}"}

        create = await client.post(
            "/patch-notes/known-issues",
            headers=headers,
            json={"title": "Original", "description": "d1"},
        )
        issue_id = create.json()["id"]

        patch = await client.patch(
            f"/patch-notes/known-issues/{issue_id}",
            headers=headers,
            json={"title": "Updated"},
        )
        assert patch.status_code == 200
        assert patch.json()["title"] == "Updated"
        assert patch.json()["description"] == "d1"

    async def test_admin_delete(self, client: AsyncClient, session_factory):
        token = await _register_and_login(client, "adminthree")
        await _grant_admin(session_factory, "adminthree")
        headers = {"Authorization": f"Bearer {token}"}

        create = await client.post(
            "/patch-notes/known-issues",
            headers=headers,
            json={"title": "Gone soon", "description": ""},
        )
        issue_id = create.json()["id"]

        delete = await client.delete(
            f"/patch-notes/known-issues/{issue_id}",
            headers=headers,
        )
        assert delete.status_code == 204

        public = await client.get("/patch-notes")
        assert public.json()["known_issues"] == []

    async def test_update_nonexistent_returns_404(self, client: AsyncClient, session_factory):
        token = await _register_and_login(client, "adminfour")
        await _grant_admin(session_factory, "adminfour")
        resp = await client.patch(
            "/patch-notes/known-issues/nonexistent",
            headers={"Authorization": f"Bearer {token}"},
            json={"title": "x"},
        )
        assert resp.status_code == 404

    async def test_delete_nonexistent_returns_404(self, client: AsyncClient, session_factory):
        token = await _register_and_login(client, "adminfive")
        await _grant_admin(session_factory, "adminfive")
        resp = await client.delete(
            "/patch-notes/known-issues/nonexistent",
            headers={"Authorization": f"Bearer {token}"},
        )
        assert resp.status_code == 404

    async def test_non_admin_cannot_update(self, client: AsyncClient, session_factory):
        admin_token = await _register_and_login(client, "adminsix")
        await _grant_admin(session_factory, "adminsix")
        create = await client.post(
            "/patch-notes/known-issues",
            headers={"Authorization": f"Bearer {admin_token}"},
            json={"title": "Seed", "description": ""},
        )
        issue_id = create.json()["id"]

        plain_token = await _register_and_login(client, "plainupdater")
        resp = await client.patch(
            f"/patch-notes/known-issues/{issue_id}",
            headers={"Authorization": f"Bearer {plain_token}"},
            json={"title": "Hijack"},
        )
        assert resp.status_code == 403

    async def test_non_admin_cannot_delete(self, client: AsyncClient, session_factory):
        admin_token = await _register_and_login(client, "adminseven")
        await _grant_admin(session_factory, "adminseven")
        create = await client.post(
            "/patch-notes/known-issues",
            headers={"Authorization": f"Bearer {admin_token}"},
            json={"title": "Seed2", "description": ""},
        )
        issue_id = create.json()["id"]

        plain_token = await _register_and_login(client, "plaindeleter")
        resp = await client.delete(
            f"/patch-notes/known-issues/{issue_id}",
            headers={"Authorization": f"Bearer {plain_token}"},
        )
        assert resp.status_code == 403

    async def test_ordered_by_created_at_desc(
        self, client: AsyncClient, session_factory
    ):
        token = await _register_and_login(client, "adminorder")
        await _grant_admin(session_factory, "adminorder")
        headers = {"Authorization": f"Bearer {token}"}

        titles = ["first", "second", "third"]
        for title in titles:
            resp = await client.post(
                "/patch-notes/known-issues",
                headers=headers,
                json={"title": title, "description": ""},
            )
            assert resp.status_code == 201

        public = await client.get("/patch-notes")
        assert public.status_code == 200
        returned = [issue["title"] for issue in public.json()["known_issues"]]
        # Newest (last inserted) first.
        assert returned == list(reversed(titles))


class TestReleaseAdminFlow:
    async def test_admin_create_and_order_desc(self, client: AsyncClient, session_factory):
        token = await _register_and_login(client, "adminrel1")
        await _grant_admin(session_factory, "adminrel1")
        headers = {"Authorization": f"Bearer {token}"}

        first = await client.post(
            "/patch-notes/releases",
            headers=headers,
            json={
                "version": "0.1.0",
                "release_date": "2026-01-01T00:00:00Z",
                "title": "First",
                "added": ["Patch notes page."],
                "changed": [],
                "fixed": [],
            },
        )
        assert first.status_code == 201, first.text

        second = await client.post(
            "/patch-notes/releases",
            headers=headers,
            json={
                "version": "0.2.0",
                "release_date": "2026-02-01T00:00:00Z",
                "title": "Second",
                "added": ["New feature."],
                "changed": ["Tweaked layout."],
                "fixed": ["Squashed bug."],
            },
        )
        assert second.status_code == 201, second.text

        public = await client.get("/patch-notes")
        assert public.status_code == 200
        releases = public.json()["releases"]
        assert len(releases) == 2
        # newest first
        assert releases[0]["version"] == "0.2.0"
        assert releases[1]["version"] == "0.1.0"
        assert releases[0]["added"] == ["New feature."]
        assert releases[0]["changed"] == ["Tweaked layout."]
        assert releases[0]["fixed"] == ["Squashed bug."]

    async def test_duplicate_version_conflicts(self, client: AsyncClient, session_factory):
        token = await _register_and_login(client, "adminrel2")
        await _grant_admin(session_factory, "adminrel2")
        headers = {"Authorization": f"Bearer {token}"}

        payload = {
            "version": "1.0.0",
            "release_date": "2026-03-01T00:00:00Z",
            "title": None,
            "added": [],
            "changed": [],
            "fixed": [],
        }
        first = await client.post("/patch-notes/releases", headers=headers, json=payload)
        assert first.status_code == 201

        second = await client.post("/patch-notes/releases", headers=headers, json=payload)
        assert second.status_code == 409

    async def test_update_release(self, client: AsyncClient, session_factory):
        token = await _register_and_login(client, "adminrel3")
        await _grant_admin(session_factory, "adminrel3")
        headers = {"Authorization": f"Bearer {token}"}

        create = await client.post(
            "/patch-notes/releases",
            headers=headers,
            json={
                "version": "0.3.0",
                "release_date": "2026-03-10T00:00:00Z",
                "title": "Pre-edit",
                "added": ["a"],
                "changed": [],
                "fixed": [],
            },
        )
        release_id = create.json()["id"]

        patch = await client.patch(
            f"/patch-notes/releases/{release_id}",
            headers=headers,
            json={"title": "Post-edit", "added": ["a", "b"]},
        )
        assert patch.status_code == 200
        body = patch.json()
        assert body["title"] == "Post-edit"
        assert body["added"] == ["a", "b"]
        # Unmodified fields remain.
        assert body["version"] == "0.3.0"

    async def test_delete_release(self, client: AsyncClient, session_factory):
        token = await _register_and_login(client, "adminrel4")
        await _grant_admin(session_factory, "adminrel4")
        headers = {"Authorization": f"Bearer {token}"}

        create = await client.post(
            "/patch-notes/releases",
            headers=headers,
            json={
                "version": "0.4.0",
                "release_date": "2026-04-01T00:00:00Z",
                "title": None,
                "added": [],
                "changed": [],
                "fixed": [],
            },
        )
        release_id = create.json()["id"]

        delete = await client.delete(
            f"/patch-notes/releases/{release_id}",
            headers=headers,
        )
        assert delete.status_code == 204

        public = await client.get("/patch-notes")
        assert public.json()["releases"] == []

    async def test_non_admin_cannot_create_release(self, client: AsyncClient):
        token = await _register_and_login(client, "plainreleaseuser")
        resp = await client.post(
            "/patch-notes/releases",
            headers={"Authorization": f"Bearer {token}"},
            json={
                "version": "9.9.9",
                "release_date": "2026-09-09T00:00:00Z",
                "title": None,
                "added": [],
                "changed": [],
                "fixed": [],
            },
        )
        assert resp.status_code == 403

    async def test_delete_nonexistent_release_returns_404(
        self, client: AsyncClient, session_factory
    ):
        token = await _register_and_login(client, "adminrel5")
        await _grant_admin(session_factory, "adminrel5")
        resp = await client.delete(
            "/patch-notes/releases/does-not-exist",
            headers={"Authorization": f"Bearer {token}"},
        )
        assert resp.status_code == 404

    async def test_update_nonexistent_release_returns_404(
        self, client: AsyncClient, session_factory
    ):
        token = await _register_and_login(client, "adminrel6")
        await _grant_admin(session_factory, "adminrel6")
        resp = await client.patch(
            "/patch-notes/releases/does-not-exist",
            headers={"Authorization": f"Bearer {token}"},
            json={"title": "nope"},
        )
        assert resp.status_code == 404

    async def test_update_to_duplicate_version_conflicts(
        self, client: AsyncClient, session_factory
    ):
        token = await _register_and_login(client, "adminrel7")
        await _grant_admin(session_factory, "adminrel7")
        headers = {"Authorization": f"Bearer {token}"}

        r1 = await client.post(
            "/patch-notes/releases",
            headers=headers,
            json={
                "version": "1.0.0",
                "release_date": "2026-01-01T00:00:00Z",
                "title": None,
                "added": [],
                "changed": [],
                "fixed": [],
            },
        )
        assert r1.status_code == 201

        r2 = await client.post(
            "/patch-notes/releases",
            headers=headers,
            json={
                "version": "1.1.0",
                "release_date": "2026-01-02T00:00:00Z",
                "title": None,
                "added": [],
                "changed": [],
                "fixed": [],
            },
        )
        assert r2.status_code == 201
        r2_id = r2.json()["id"]

        # Attempt to rename r2 to r1's version → 409.
        conflict = await client.patch(
            f"/patch-notes/releases/{r2_id}",
            headers=headers,
            json={"version": "1.0.0"},
        )
        assert conflict.status_code == 409

        # Public payload still shows both rows with their original versions
        # (the failed commit was rolled back).
        public = await client.get("/patch-notes")
        versions = sorted(r["version"] for r in public.json()["releases"])
        assert versions == ["1.0.0", "1.1.0"]

    async def test_non_admin_cannot_update_release(
        self, client: AsyncClient, session_factory
    ):
        admin_token = await _register_and_login(client, "adminrel8")
        await _grant_admin(session_factory, "adminrel8")
        create = await client.post(
            "/patch-notes/releases",
            headers={"Authorization": f"Bearer {admin_token}"},
            json={
                "version": "2.0.0",
                "release_date": "2026-02-01T00:00:00Z",
                "title": None,
                "added": [],
                "changed": [],
                "fixed": [],
            },
        )
        release_id = create.json()["id"]

        plain_token = await _register_and_login(client, "plainreleaseupdater")
        resp = await client.patch(
            f"/patch-notes/releases/{release_id}",
            headers={"Authorization": f"Bearer {plain_token}"},
            json={"title": "Hijack"},
        )
        assert resp.status_code == 403

    async def test_non_admin_cannot_delete_release(
        self, client: AsyncClient, session_factory
    ):
        admin_token = await _register_and_login(client, "adminrel9")
        await _grant_admin(session_factory, "adminrel9")
        create = await client.post(
            "/patch-notes/releases",
            headers={"Authorization": f"Bearer {admin_token}"},
            json={
                "version": "3.0.0",
                "release_date": "2026-03-01T00:00:00Z",
                "title": None,
                "added": [],
                "changed": [],
                "fixed": [],
            },
        )
        release_id = create.json()["id"]

        plain_token = await _register_and_login(client, "plainreleasedeleter")
        resp = await client.delete(
            f"/patch-notes/releases/{release_id}",
            headers={"Authorization": f"Bearer {plain_token}"},
        )
        assert resp.status_code == 403
