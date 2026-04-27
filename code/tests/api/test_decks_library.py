from __future__ import annotations

import pytest
from httpx import ASGITransport, AsyncClient
from sqlalchemy.ext.asyncio import AsyncSession, async_sessionmaker, create_async_engine

from server.db.database import get_db
from server.db.models import Base


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
async def client(db_engine, monkeypatch):
    from server.api import app
    from server.db.routers import decks as decks_router

    monkeypatch.setattr(decks_router, "out_of_set_cards", lambda _cards: [])
    session_factory = async_sessionmaker(db_engine, class_=AsyncSession, expire_on_commit=False)

    async def override_get_db():
        async with session_factory() as session:
            yield session

    app.dependency_overrides[get_db] = override_get_db
    transport = ASGITransport(app=app)
    async with AsyncClient(transport=transport, base_url="http://test") as c:
        yield c
    app.dependency_overrides.clear()


def _legal_deck() -> list[str]:
    ids = [f"BT14-{i:03d}" for i in range(10, 23)]
    deck: list[str] = []
    for cid in ids:
        for _ in range(4):
            deck.append(cid)
            if len(deck) == 50:
                return deck
    return deck


async def _register_login(client: AsyncClient, username: str) -> str:
    await client.post(
        "/auth/register",
        json={
            "username": username,
            "email": f"{username}@example.com",
            "password": "secure-password-123",
        },
    )
    resp = await client.post(
        "/auth/login",
        json={"username": username, "password": "secure-password-123"},
    )
    assert resp.status_code == 200, resp.text
    return resp.json()["access_token"]


async def _create_deck(client: AsyncClient, token: str, name: str = "Library Deck") -> dict:
    resp = await client.post(
        "/decks",
        headers={"Authorization": f"Bearer {token}"},
        json={
            "name": name,
            "description": "Test notes",
            "game_mode": "standard",
            "main_deck": _legal_deck(),
            "egg_deck": [],
            "tags": ["meta"],
        },
    )
    assert resp.status_code == 201, resp.text
    return resp.json()


@pytest.mark.asyncio
async def test_folder_crud_seeds_defaults_and_enforces_owner(client: AsyncClient):
    token_a = await _register_login(client, "library_a")
    token_b = await _register_login(client, "library_b")

    default_resp = await client.get("/decks/folders", headers={"Authorization": f"Bearer {token_a}"})
    assert default_resp.status_code == 200, default_resp.text
    assert [folder["name"] for folder in default_resp.json()] == ["Tournament", "Experimental", "Casual"]

    create_resp = await client.post(
        "/decks/folders",
        headers={"Authorization": f"Bearer {token_a}"},
        json={"name": "Locals", "sort_order": 10},
    )
    assert create_resp.status_code == 201, create_resp.text
    folder = create_resp.json()
    assert folder["name"] == "Locals"

    rename_resp = await client.put(
        f"/decks/folders/{folder['id']}",
        headers={"Authorization": f"Bearer {token_a}"},
        json={"name": "Regionals", "sort_order": 2},
    )
    assert rename_resp.status_code == 200, rename_resp.text
    assert rename_resp.json()["name"] == "Regionals"

    blocked_resp = await client.put(
        f"/decks/folders/{folder['id']}",
        headers={"Authorization": f"Bearer {token_b}"},
        json={"name": "Stolen"},
    )
    assert blocked_resp.status_code == 404

    delete_resp = await client.delete(
        f"/decks/folders/{folder['id']}",
        headers={"Authorization": f"Bearer {token_a}"},
    )
    assert delete_resp.status_code == 200, delete_resp.text


@pytest.mark.asyncio
async def test_deck_library_patch_updates_folder_pin_and_summary_fields(client: AsyncClient):
    token = await _register_login(client, "library_patch")
    deck = await _create_deck(client, token)
    folder_resp = await client.post(
        "/decks/folders",
        headers={"Authorization": f"Bearer {token}"},
        json={"name": "Testing"},
    )
    assert folder_resp.status_code == 201, folder_resp.text
    folder = folder_resp.json()

    patch_resp = await client.patch(
        f"/decks/{deck['id']}/library",
        headers={"Authorization": f"Bearer {token}"},
        json={"folder_id": folder["id"], "is_pinned": True},
    )
    assert patch_resp.status_code == 200, patch_resp.text
    patched = patch_resp.json()
    assert patched["folder_id"] == folder["id"]
    assert patched["is_pinned"] is True

    list_resp = await client.get("/decks", headers={"Authorization": f"Bearer {token}"})
    assert list_resp.status_code == 200, list_resp.text
    summary = list_resp.json()[0]
    assert summary["folder_id"] == folder["id"]
    assert summary["is_pinned"] is True
    assert summary["description"] == "Test notes"
    assert summary["tags"] == ["meta"]
    assert summary["main_count"] == 50
    assert summary["egg_count"] == 0
    assert summary["card_count"] == 50

    clear_resp = await client.patch(
        f"/decks/{deck['id']}/library",
        headers={"Authorization": f"Bearer {token}"},
        json={"folder_id": None, "is_pinned": False},
    )
    assert clear_resp.status_code == 200, clear_resp.text
    assert clear_resp.json()["folder_id"] is None
    assert clear_resp.json()["is_pinned"] is False


@pytest.mark.asyncio
async def test_deck_cannot_move_to_another_users_folder(client: AsyncClient):
    token_a = await _register_login(client, "library_owner")
    token_b = await _register_login(client, "library_other")
    deck = await _create_deck(client, token_a)
    folder_resp = await client.post(
        "/decks/folders",
        headers={"Authorization": f"Bearer {token_b}"},
        json={"name": "Other User Folder"},
    )
    assert folder_resp.status_code == 201, folder_resp.text
    folder = folder_resp.json()

    patch_resp = await client.patch(
        f"/decks/{deck['id']}/library",
        headers={"Authorization": f"Bearer {token_a}"},
        json={"folder_id": folder["id"]},
    )
    assert patch_resp.status_code == 404
