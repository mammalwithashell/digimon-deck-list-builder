import pytest
from pydantic import ValidationError

from server.routers.schemas import GameActionRequest


def test_page_token_remains_exact_at_transport_boundary():
    token = {"prompt_id": "18446744073709551615", "page_generation": "9007199254740993"}
    request = GameActionRequest(action=59, expected_card_choice=token)
    assert request.expected_card_choice.model_dump() == token


@pytest.mark.parametrize("token", [
    {"prompt_id": 123, "page_generation": "1"},
    {"prompt_id": "123", "page_generation": 1},
    {"prompt_id": "123"},
    {"prompt_id": "123", "page_generation": "1", "card_id": "not-a-token"},
])
def test_malformed_page_token_is_not_coerced_or_silently_reinterpreted(token):
    with pytest.raises(ValidationError):
        GameActionRequest(action=30, expected_card_choice=token)


def test_serial_legacy_action_shape_has_no_synthetic_page_token():
    assert GameActionRequest(action=1).expected_card_choice is None
