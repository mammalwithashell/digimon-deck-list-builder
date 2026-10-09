"""`/decks/card-database`: the deck builder's local card text (mirrors the engine's CardMeta)."""
from server.routers import deck_tools


def _card(card_id):
    deck_tools._card_database_payload.cache_clear()
    return next(c for c in deck_tools._card_database_payload() if c["card_id"] == card_id)


def test_a_link_box_is_served_as_the_link_effect():
    # general_rule.pdf 2-3-12: BT21-009 Gatchmon prints <Raid> in its link box and has no
    # inherited effect.
    gatchmon = _card("BT21-009")
    assert gatchmon["inherited_effect"] == ""
    assert gatchmon["link_effect"].startswith("＜Raid＞")


def test_a_card_without_a_link_box_has_an_empty_link_effect():
    agumon = _card("ST1-03")
    assert agumon["inherited_effect"] == "[Your Turn] This Digimon gets +1000 DP."
    assert agumon["link_effect"] == ""
