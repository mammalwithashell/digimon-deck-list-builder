"""Tests for tools.ingest_cards.normalize_misfiled_inherited.

The digimoncard.io API returns some cards' inherited effect inside
`main_effect`, prefixed with the printed "Inherited Effect" label, while
`source_effect` is empty (common on older sets' eggs/rookies, e.g. ST6-03
Gabumon). The ingest must file that text into
`inherited_effect_description_eng` or the engine's CardData.inherited_text
stays empty and the stack inspector shows no inherited effect.
"""
from __future__ import annotations

from tools.ingest_cards import normalize_misfiled_inherited


def make_card(effect: str, inherited: str = "") -> dict:
    return {
        "card_id": "ST6-03",
        "effect_description_eng": effect,
        "inherited_effect_description_eng": inherited,
        "security_effect_description_eng": "",
    }


def test_moves_prefixed_inherited_text_to_inherited_field():
    card = make_card(
        "Inherited Effect [When Attacking] ＜Draw 1＞ (Draw 1 card from "
        "your deck.) Then, trash 1 card in your hand."
    )
    changed = normalize_misfiled_inherited(card)
    assert changed is True
    assert card["effect_description_eng"] == ""
    assert card["inherited_effect_description_eng"] == (
        "[When Attacking] ＜Draw 1＞ (Draw 1 card from your deck.) "
        "Then, trash 1 card in your hand."
    )


def test_leaves_real_main_effects_alone():
    card = make_card("[When Digivolving] Draw 1.")
    assert normalize_misfiled_inherited(card) is False
    assert card["effect_description_eng"] == "[When Digivolving] Draw 1."
    assert card["inherited_effect_description_eng"] == ""


def test_does_not_clobber_an_existing_inherited_field():
    card = make_card(
        "Inherited Effect [Your Turn] +1000 DP.",
        inherited="[All Turns] Blocker.",
    )
    assert normalize_misfiled_inherited(card) is False
    assert card["effect_description_eng"] == "Inherited Effect [Your Turn] +1000 DP."
    assert card["inherited_effect_description_eng"] == "[All Turns] Blocker."


def test_skips_dual_cards():
    card = make_card("Inherited Effect [Your Turn] +1000 DP.")
    card["dual"] = {"digimon": {}, "option": {}}
    assert normalize_misfiled_inherited(card) is False
    assert card["effect_description_eng"] == "Inherited Effect [Your Turn] +1000 DP."


def test_requires_the_label_at_the_start():
    card = make_card("[When Digivolving] Gain 1 memory. Inherited Effect text mention.")
    assert normalize_misfiled_inherited(card) is False
