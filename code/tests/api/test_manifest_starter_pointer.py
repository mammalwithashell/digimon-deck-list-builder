"""The public manifest exposes an optional starter_ai_model_id pointer,
sourced from the STARTER_AI_MODEL_ID env var, so the desktop app knows which
model the AI-Starter mode should play. Defaults to None when unset."""
from server.db.schemas import ManifestResponse


def test_manifest_response_has_optional_starter_pointer():
    m = ManifestResponse(generated_at="2026-06-16T00:00:00Z", models=[])
    assert m.starter_ai_model_id is None

    m2 = ManifestResponse(
        generated_at="2026-06-16T00:00:00Z", models=[], starter_ai_model_id="abc-123"
    )
    assert m2.starter_ai_model_id == "abc-123"
