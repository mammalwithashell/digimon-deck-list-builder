from fastapi.testclient import TestClient

from server.api import app


def test_formats_catalog_marks_standard_enabled() -> None:
    client = TestClient(app)
    response = client.get("/formats")
    assert response.status_code == 200
    body = response.json()
    assert body[0]["id"] == "standard"
    assert body[0]["enabled"] is True
    assert [item["id"] for item in body if item["enabled"]] == ["standard"]
