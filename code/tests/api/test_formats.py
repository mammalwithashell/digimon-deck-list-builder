from fastapi.testclient import TestClient

from server.api import app


def test_formats_catalog_uses_engine_registry_playable_formats() -> None:
    client = TestClient(app)
    response = client.get("/formats")
    assert response.status_code == 200
    body = response.json()
    assert body[0]["id"] == "standard"
    assert body[0]["enabled"] is True
    assert [item["id"] for item in body if item["enabled"]] == [
        "standard",
        "no_restriction",
        "pauper",
        "eden",
        "eden_singleton",
    ]
