from copy import deepcopy

from server.state_filter import filter_state_for_player, filter_state_for_spectator


def state(selecting_player):
    return {
        "player1": {"handIds": [], "securityIds": []},
        "player2": {"handIds": [], "securityIds": []},
        "pendingSelection": {
            "selectingPlayer": selecting_player,
            "cardChoice": {
                "prompt_id": "18446744073709551615",
                "page_generation": "123",
                "rows": [
                    {"zone": "security", "owner": 0, "card_id": "SECRET",
                     "card_registry_index": 42, "identity_visible": True},
                    {"zone": "battle_source", "owner": 1, "card_id": "PUBLIC",
                     "card_registry_index": 43, "identity_visible": True},
                ],
            },
        },
    }


def test_search_permission_does_not_leak_to_other_seat_or_hidden_spectator():
    for selecting in (1, 2):
        raw = state(selecting)
        before = deepcopy(raw)
        assert filter_state_for_player(raw, selecting)["pendingSelection"]["cardChoice"]["rows"][0]["card_id"] == "SECRET"
        for view in (filter_state_for_player(raw, 3 - selecting), filter_state_for_spectator(raw)):
            choice = view["pendingSelection"]["cardChoice"]
            hidden, public = choice["rows"]
            assert hidden["card_id"] is None
            assert hidden["card_registry_index"] is None
            assert hidden["identity_visible"] is False
            assert public["card_id"] == "PUBLIC"
            assert choice["prompt_id"] == "18446744073709551615"
        assert raw == before


def test_second_seat_remaps_typed_zero_based_zone_owners_without_mutation():
    raw = state(2)
    view = filter_state_for_player(raw, 2)
    assert [r["owner"] for r in view["pendingSelection"]["cardChoice"]["rows"]] == [1, 0]
    assert [r["owner"] for r in raw["pendingSelection"]["cardChoice"]["rows"]] == [0, 1]


def test_open_spectator_retains_explicit_open_visibility():
    assert filter_state_for_spectator(state(1), "open")["pendingSelection"]["cardChoice"]["rows"][0]["card_id"] == "SECRET"

def test_union_hand_rows_are_private_to_owner_and_trash_rows_remain_public():
    for owner in (0, 1):
        raw = state(owner + 1)
        raw["pendingSelection"]["cardChoice"]["rows"] = [
            {"zone": "hand", "owner": owner, "card_id": "PRIVATE-HAND", "card_registry_index": 41, "identity_visible": True},
            {"zone": "trash", "owner": owner, "card_id": "PUBLIC-TRASH", "card_registry_index": 42, "identity_visible": True},
        ]
        before = deepcopy(raw)
        mine = filter_state_for_player(raw, owner + 1)["pendingSelection"]["cardChoice"]["rows"]
        assert mine[0]["card_id"] == "PRIVATE-HAND"
        for view in (filter_state_for_player(raw, 2 - owner), filter_state_for_spectator(raw)):
            hand, trash = view["pendingSelection"]["cardChoice"]["rows"]
            assert hand["card_id"] is None
            assert hand["card_registry_index"] is None
            assert hand["identity_visible"] is False
            assert trash["card_id"] == "PUBLIC-TRASH"
        assert raw == before
