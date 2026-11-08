"""Public security revelation preserves identities without disclosing private inspection."""
from copy import deepcopy
from server.state_filter import filter_state_for_player, filter_state_for_spectator


def state():
    return {
        "player1": {"id": 1, "handIds": ["SECRET1"], "handCards": [{"cardId": "SECRET1"}],
                    "securityIds": ["HIDDEN1", "PUBLIC1"], "securityIdentityPublic": [False, True]},
        "player2": {"id": 2, "handIds": ["SECRET2"], "handCards": [{"cardId": "SECRET2"}],
                    "securityIds": ["PUBLIC2", "HIDDEN2"], "securityIdentityPublic": [True, False]},
        "pendingSelection": {"selectingPlayer": 1, "cardChoice": {"rows": [
            {"owner": 1, "zone": "security", "position": "0", "card_id": "PUBLIC2", "card_registry_index": 11, "identity_visible": True},
            {"owner": 1, "zone": "security", "position": "1", "card_id": "HIDDEN2", "card_registry_index": 12, "identity_visible": True},
        ]}},
    }


def test_public_security_survives_both_seat_views_and_hidden_spectators():
    original = state(); saved = deepcopy(original)
    for seat in [1, 2]:
        view = filter_state_for_player(original, seat)
        own = [None, "PUBLIC1"] if seat == 1 else ["PUBLIC2", None]
        opp = ["PUBLIC2", None] if seat == 1 else [None, "PUBLIC1"]
        assert view["player1"]["securityIds"] == own
        assert view["player2"]["securityIds"] == opp
        assert view["player2"]["handIds"] == []
        assert view["player2"]["handCards"] == []
    spectator = filter_state_for_spectator(original)
    assert spectator["player1"]["securityIds"] == [None, "PUBLIC1"]
    assert spectator["player2"]["securityIds"] == ["PUBLIC2", None]
    assert original == saved


def test_private_search_visibility_does_not_become_public():
    for view in [filter_state_for_player(state(), 2), filter_state_for_spectator(state())]:
        rows = view["pendingSelection"]["cardChoice"]["rows"]
        assert rows[0]["card_id"] == "PUBLIC2"
        assert rows[0]["identity_visible"] is True
        assert rows[1]["card_id"] is None
        assert rows[1]["card_registry_index"] is None
        assert rows[1]["identity_visible"] is False


def test_missing_or_malformed_public_mask_fails_closed():
    for mask in [None, [], [True], [1, "true"]]:
        full = state()
        full["player1"]["securityIdentityPublic"] = mask
        full["player2"].pop("securityIdentityPublic")
        view = filter_state_for_spectator(full)
        assert view["player1"]["securityIds"] == []
        assert view["player2"]["securityIds"] == []
        assert all(row["card_id"] is None for row in view["pendingSelection"]["cardChoice"]["rows"])


def test_shuffle_revocation_and_stale_public_position_hide_old_search_identity():
    full = state()
    full["player2"]["securityIdentityPublic"] = [False, False]
    view = filter_state_for_spectator(full)
    assert view["player2"]["securityIds"] == []
    assert all(row["card_id"] is None for row in view["pendingSelection"]["cardChoice"]["rows"])
    full["player2"]["securityIdentityPublic"] = [True, False]
    full["player2"]["securityIds"][0] = "OTHER-PUBLIC"
    rows = filter_state_for_spectator(full)["pendingSelection"]["cardChoice"]["rows"]
    assert rows[0]["card_id"] is None


def test_missing_selecting_player_never_discloses_private_rows_to_spectators():
    full = state()
    full["pendingSelection"].pop("selectingPlayer")
    rows = filter_state_for_spectator(full)["pendingSelection"]["cardChoice"]["rows"]
    assert rows[0]["card_id"] == "PUBLIC2"
    assert rows[1]["card_id"] is None
