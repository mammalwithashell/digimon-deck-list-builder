"""Per-model `starter_deck` slug flows through the AI-model schemas so the
desktop AI-Starter mode can route each built-in starter deck to its trained
specialist. Unlike `deck_id` (an FK to a real `decks` row) this is a free-form
slug naming a *built-in* bundled deck (e.g. "starter_st3_heavens_yellow")."""
from server.db.schemas import (
    AIModelCreateRequest,
    AIModelUpdateRequest,
    ManifestModel,
)


def test_create_request_accepts_starter_deck():
    req = AIModelCreateRequest(
        name="ST-3 Specialist",
        model_type="mlp",
        starter_deck="starter_st3_heavens_yellow",
    )
    assert req.starter_deck == "starter_st3_heavens_yellow"
    # Optional — defaults to None for non-starter models.
    assert AIModelCreateRequest(name="x", model_type="lstm").starter_deck is None


def test_update_request_accepts_starter_deck():
    req = AIModelUpdateRequest(starter_deck="starter_st1_gaia_red")
    assert req.starter_deck == "starter_st1_gaia_red"
    assert AIModelUpdateRequest().starter_deck is None


def test_manifest_model_carries_starter_deck():
    m = ManifestModel(
        id="m1",
        name="ST-1 Specialist",
        model_type="mlp",
        tensor_size=8850,
        action_space_size=2192,
        engine_commit=None,
        trained_at=None,
        file_sha256="0" * 64,
        file_size_bytes=123,
        url="https://cdn/models/m1/policy.onnx",
        deck_id=None,
        deck_name=None,
        starter_deck="starter_st1_gaia_red",
        notes=None,
    )
    assert m.starter_deck == "starter_st1_gaia_red"
    # Older manifests without the field still parse (defaults to None).
    m2 = ManifestModel(
        id="m2",
        name="generalist",
        model_type="lstm",
        tensor_size=8850,
        action_space_size=2192,
        engine_commit=None,
        trained_at=None,
        file_sha256="0" * 64,
        file_size_bytes=123,
        url="https://cdn/models/m2/policy.onnx",
        deck_id=None,
        deck_name=None,
        notes=None,
    )
    assert m2.starter_deck is None
