"""Tests for DigiXros engine feature.

Covers: parser, material validator, play flow, action mask, Assembly (trash).
"""
import pytest
from engine_py_legacy.engine.data.card_database import CardDatabase, parse_digixros_req
from engine_py_legacy.engine.data.evo_cost import DigiXrosCost, DigiXrosElement
from engine_py_legacy.engine.data.enums import GamePhase, EffectTiming
from engine_py_legacy.engine.game import Game
from engine_py_legacy.engine.game.action_mask import build_action_mask
from engine_py_legacy.engine.game.constants import (
    SEL_HAND_START, SEL_MY_FIELD_START, SEL_TRASH_START,
)
from engine_py_legacy.engine.core.permanent import Permanent
from engine_py_legacy.engine.validation.digixros_validator import (
    get_valid_digixros_materials, has_any_digixros_material,
)


@pytest.fixture(scope="module")
def db():
    return CardDatabase()


# ─── Parser Tests ─────────────────────────────────────────────────────


class TestParseDigiXrosReq:
    """Test parse_digixros_req covers all observed card text patterns."""

    def test_named_elements(self):
        text = (
            "[DigiXros\xa0-2] [Shoutmon] x [Ballistamon] x [Dorulumon] x [Starmons] "
            "\r\nWhen this"
        )
        result = parse_digixros_req(text)
        assert len(result) == 1
        xc = result[0]
        assert xc.reduce_cost_per_card == 2
        assert xc.max_materials == 4
        assert len(xc.elements) == 4
        names = [e.name_contains for e in xc.elements]
        assert names == ["Shoutmon", "Ballistamon", "Dorulumon", "Starmons"]

    def test_or_name_elements(self):
        text = "[DigiXros\xa0-1] [Agumon] or [Greymon] x [Gabumon] or [Garurumon]\r\nWhen this"
        result = parse_digixros_req(text)
        assert len(result) == 1
        xc = result[0]
        assert xc.reduce_cost_per_card == 1
        assert len(xc.elements) == 2
        # First element: Agumon or Greymon
        assert xc.elements[0].name_contains == "Agumon"
        assert "Greymon" in xc.elements[0].trait_alternatives
        # Second element: Gabumon or Garurumon
        assert xc.elements[1].name_contains == "Gabumon"
        assert "Garurumon" in xc.elements[1].trait_alternatives

    def test_trait_count(self):
        text = "[DigiXros\xa0-2] 1 Digimon card w/[Bagra Army]\xa0trait\r\nWhen this"
        result = parse_digixros_req(text)
        assert len(result) == 1
        xc = result[0]
        assert xc.reduce_cost_per_card == 2
        assert xc.max_materials == 1
        assert len(xc.elements) == 1
        assert xc.elements[0].trait_match == "Bagra Army"

    def test_count_name(self):
        text = "[DigiXros\xa0-1] 4 [Vemmon]\r\nWhen this"
        result = parse_digixros_req(text)
        assert len(result) == 1
        xc = result[0]
        assert xc.reduce_cost_per_card == 1
        assert xc.max_materials == 4
        assert xc.elements[0].name_contains == "Vemmon"
        assert xc.elements[0].count == 4

    def test_mixed_name_and_count(self):
        text = "[DigiXros\xa0-1] [Snatchmon] x 4 [Vemmon]\r\nWhen this"
        result = parse_digixros_req(text)
        assert len(result) == 1
        xc = result[0]
        assert len(xc.elements) == 2
        assert xc.elements[0].name_contains == "Snatchmon"
        assert xc.elements[0].count == 1
        assert xc.elements[1].name_contains == "Vemmon"
        assert xc.elements[1].count == 4

    def test_level_trait(self):
        text = (
            "[DigiXros\xa0-1] 5 Lv.5 or lower [Cyborg] or [Composite] trait "
            "Digimon cards w/different card numbers\r\nWhen this"
        )
        result = parse_digixros_req(text)
        assert len(result) == 1
        xc = result[0]
        assert xc.reduce_cost_per_card == 1
        assert xc.different_card_numbers is True
        assert xc.elements[0].level_max == 5
        assert xc.elements[0].trait_match == "Cyborg"
        assert "Composite" in xc.elements[0].trait_alternatives

    def test_infinity(self):
        text = (
            "[DigiXros\xa0-1] \u221e Digimon cards w/[Xros Heart] or [Blue Flare] "
            "trait & different card numbers\r\nWhen this"
        )
        result = parse_digixros_req(text)
        assert len(result) == 1
        xc = result[0]
        assert xc.max_materials == -1  # unlimited
        assert xc.different_card_numbers is True
        assert xc.elements[0].count == 99

    def test_assembly_trash_zone(self):
        text = "[Assembly -7] 7 level 4 [DM] trait Digimon cards w/different names\r\nWhen this"
        result = parse_digixros_req(text)
        assert len(result) == 1
        xc = result[0]
        assert xc.source_zones == ["trash"]
        assert xc.reduce_cost_per_card == 7
        assert xc.different_names is True
        assert xc.elements[0].trait_match == "DM"
        assert xc.elements[0].level_max == 4
        assert xc.elements[0].count == 7

    def test_assembly_named(self):
        text = "[Assembly -6] [Titamon] x [SkullBaluchimon] \r\nWhen this"
        result = parse_digixros_req(text)
        assert len(result) == 1
        xc = result[0]
        assert xc.source_zones == ["trash"]
        assert len(xc.elements) == 2

    def test_color_prefix(self):
        text = "[DigiXros\xa0-2] Blue [Greymon] x [MailBirdramon] \r\nWhen this"
        result = parse_digixros_req(text)
        assert len(result) == 1
        xc = result[0]
        from engine_py_legacy.engine.data.enums import CardColor
        assert xc.elements[0].color == CardColor.Blue
        assert xc.elements[0].name_contains == "Greymon"

    def test_has_text(self):
        text = "[DigiXros\xa0-2] 1 Digimon card with \uff1cSave\uff1e in text\r\nWhen this"
        result = parse_digixros_req(text)
        assert len(result) == 1
        xc = result[0]
        assert xc.has_text == "Save"

    def test_multi_trait_different_names(self):
        text = (
            "[DigiXros\xa0-2] 5 Digimon cards w/different names and [Dragon], "
            "[saur] or [Ceratopsian] in one of their traits\r\nWhen this"
        )
        result = parse_digixros_req(text)
        assert len(result) == 1
        xc = result[0]
        assert xc.different_names is True
        assert xc.elements[0].trait_match == "Dragon"
        assert "saur" in xc.elements[0].trait_alternatives

    def test_all_cards_parse(self, db):
        """Every card with DigiXros/Assembly xros_req parses to a non-empty DigiXrosCost."""
        count = sum(1 for c in db.cards.values() if c.digixros_costs)
        assert count == 60

    def test_round_trip_through_cards_json_format(self):
        """Parser output → JSON serializer → JSON → matches original shape.

        Locks in the `color` variant-name encoding + list/bool types so
        cards.json stays consumable by both engines.
        """
        import json as _json
        from tools.ingest_cards import _digixros_cost_to_json

        source = parse_digixros_req(
            "[DigiXros\xa0-2] [Shoutmon] x [Ballistamon] x [Dorulumon] x [Starmons] "
            "\r\nWhen this"
        )
        serialized = [_digixros_cost_to_json(dxc) for dxc in source]
        reloaded = _json.loads(_json.dumps(serialized))

        assert len(reloaded) == 1
        xc = reloaded[0]
        assert xc["reduce_cost_per_card"] == 2
        assert xc["max_materials"] == 4
        assert xc["source_zones"] == ["hand", "field"]
        names = [el["name_contains"] for el in xc["elements"]]
        assert names == ["Shoutmon", "Ballistamon", "Dorulumon", "Starmons"]
        # JSON must use bool/null/list primitives — not Python tuples/enums.
        for el in xc["elements"]:
            assert isinstance(el["is_digimon_only"], bool)
            assert el["color"] is None
            assert isinstance(el["trait_alternatives"], list)

    def test_assembly_source_zones_round_trip(self):
        """Assembly variant ([Assembly -N] block) keeps `source_zones=['trash']`."""
        import json as _json
        from tools.ingest_cards import _digixros_cost_to_json

        source = parse_digixros_req(
            "[Assembly\xa0-1] [Agumon] x [Gabumon]\r\nWhen this"
        )
        assert source, "Assembly parser produced no result"
        serialized = _digixros_cost_to_json(source[0])
        reloaded = _json.loads(_json.dumps(serialized))
        assert reloaded["source_zones"] == ["trash"]


# ─── Validator Tests ──────────────────────────────────────────────────


class TestDigiXrosValidator:

    def test_name_match(self, db):
        card = db.create_card_source("BT10-009")  # Shoutmon X4
        player = _make_player()
        # Add Shoutmon to hand
        shoutmon = db.create_card_source("BT10-008", player)
        player.hand_cards = [card, shoutmon]
        assert has_any_digixros_material(card, player)

    def test_no_match(self, db):
        card = db.create_card_source("BT10-009")  # Shoutmon X4
        player = _make_player()
        # Add unrelated card
        unrelated = db.create_card_source("BT1-001", player)
        player.hand_cards = [card, unrelated]
        assert not has_any_digixros_material(card, player)

    def test_field_material(self, db):
        card = db.create_card_source("BT10-009")
        player = _make_player()
        shoutmon = db.create_card_source("BT10-008", player)
        perm = Permanent([shoutmon])
        player.battle_area = [perm]
        player.hand_cards = [card]
        mats = get_valid_digixros_materials(card, player, [])
        assert SEL_MY_FIELD_START in mats

    def test_trash_material_for_assembly(self, db):
        card = db.create_card_source("BT24-081")  # Assembly: Titamon x SkullBaluchimon
        player = _make_player()
        player.hand_cards = [card]
        # Use BT1-080 Titamon (confirmed to exist)
        titamon = db.create_card_source("BT1-080", player)
        if titamon is None:
            pytest.skip("Titamon card not found")
        player.trash_cards = [titamon]
        mats = get_valid_digixros_materials(card, player, [])
        assert any(idx >= SEL_TRASH_START for idx in mats)

    def test_different_card_numbers(self, db):
        """Cards with same card_id should be rejected when different_card_numbers is set."""
        xros_cost = DigiXrosCost(
            elements=[DigiXrosElement(trait_match="Cyborg", count=3)],
            reduce_cost_per_card=1,
            max_materials=3,
            different_card_numbers=True,
            source_zones=["hand"],
        )
        player = _make_player()
        card = db.create_card_source("BT19-065", player)  # DigiXros card
        # Two cards with same card_id
        c1 = db.create_card_source("BT10-049", player)  # Ballistamon (Cyborg)
        c2 = db.create_card_source("BT10-049", player)
        player.hand_cards = [card, c1, c2]
        mats = get_valid_digixros_materials(card, player, [c1], xros_cost)
        # c2 has same card_id as c1, should be excluded
        assert not any((SEL_HAND_START + 2) == idx for idx in mats)

    def test_max_materials_limit(self, db):
        card = db.create_card_source("BT10-009")  # max_materials=4
        player = _make_player()
        # Already selected 4 materials
        already = [db.create_card_source("BT10-008") for _ in range(4)]
        player.hand_cards = [card, db.create_card_source("BT10-008", player)]
        mats = get_valid_digixros_materials(card, player, already)
        assert mats == []


# ─── Integration Tests ────────────────────────────────────────────────


class TestDigiXrosPlayFlow:

    def test_play_enters_selection(self, db):
        game, p1, _ = _setup_game(db)
        xros = db.create_card_source("BT10-009", p1)
        shoutmon = db.create_card_source("BT10-008", p1)
        p1.hand_cards = [xros, shoutmon]
        game.memory = 20

        game.decode_action(0, 1)
        assert game.current_phase == GamePhase.SelectMaterial
        assert game._pending_digixros is not None

    def test_select_and_decline(self, db):
        game, p1, _ = _setup_game(db)
        xros = db.create_card_source("BT10-009", p1)
        shoutmon = db.create_card_source("BT10-008", p1)
        ballistamon = db.create_card_source("BT10-049", p1)
        p1.hand_cards = [xros, shoutmon, ballistamon]
        game.memory = 20

        game.decode_action(0, 1)  # play DigiXros card
        assert game.current_phase == GamePhase.SelectMaterial

        game.decode_action(SEL_HAND_START + 1, 1)  # select Shoutmon
        game.decode_action(62, 1)  # decline further selection

        # Should have played with 1 material
        assert game.current_phase == GamePhase.Main
        assert len(p1.battle_area) == 1
        perm = p1.battle_area[0]
        assert len(perm.card_sources) == 2  # Shoutmon (bottom) + X4 (top)
        assert len(p1.hand_cards) == 1  # only Ballistamon left

    def test_cost_reduction(self, db):
        game, p1, _ = _setup_game(db)
        # BT10-009: cost 9, reduce 2 per material, 4 elements
        # Give 3 materials so after selecting 2, there's still 1 valid material
        # and we can explicitly decline
        xros = db.create_card_source("BT10-009", p1)
        shoutmon = db.create_card_source("BT10-008", p1)
        ballistamon = db.create_card_source("BT10-049", p1)
        dorulumon = db.create_card_source("BT10-034", p1)
        p1.hand_cards = [xros, shoutmon, ballistamon, dorulumon]
        game.memory = 20

        game.decode_action(0, 1)  # play
        game.decode_action(SEL_HAND_START + 1, 1)  # Shoutmon
        game.decode_action(SEL_HAND_START + 2, 1)  # Ballistamon
        # Still has Dorulumon available, so still in SelectMaterial
        assert game.current_phase == GamePhase.SelectMaterial
        game.decode_action(62, 1)  # decline further

        # Cost = 9 - 2*2 = 5, memory = 20 - 5 = 15
        assert game.memory == 15

    def test_field_material_only_top_card(self, db):
        """Per rules 7-2-2-7: only top card goes under, digi cards trashed."""
        game, p1, _ = _setup_game(db)
        xros = db.create_card_source("BT10-009", p1)

        # Create a field permanent with 2 sources (simulating a digivolved Shoutmon)
        shout_base = db.create_card_source("BT5-009", p1)  # Lv3 Shoutmon (bottom)
        shout_top = db.create_card_source("BT10-008", p1)  # Lv3 Shoutmon (top)
        field_perm = Permanent([shout_base, shout_top])
        field_perm._owner_game = game
        p1.battle_area = [field_perm]
        p1.hand_cards = [xros]
        p1.trash_cards = []
        game.memory = 20

        game.decode_action(0, 1)  # play
        game.decode_action(SEL_MY_FIELD_START, 1)  # select field perm
        game.decode_action(62, 1)  # done

        assert len(p1.battle_area) == 1
        new_perm = p1.battle_area[0]
        # Only top card (BT10-008) placed under, plus xros on top = 2 sources
        assert len(new_perm.card_sources) == 2
        assert new_perm.top_card.card_id == "BT10-009"  # xros card on top
        assert new_perm.card_sources[0].card_id == "BT10-008"  # top card at bottom

        # Digi card (shout_base) should be trashed
        assert len(p1.trash_cards) == 1
        assert p1.trash_cards[0].card_id == "BT5-009"

    def test_digixros_count_in_context(self, db):
        """OnEnterFieldAnyone context should include digixros_count."""
        game, p1, _ = _setup_game(db)
        xros = db.create_card_source("BT10-009", p1)
        shoutmon = db.create_card_source("BT10-008", p1)
        p1.hand_cards = [xros, shoutmon]
        game.memory = 20

        captured_ctx = {}
        original = game.execute_effects

        def capture(timing, ctx=None):
            if timing == EffectTiming.OnEnterFieldAnyone:
                captured_ctx.update(ctx or {})
            return original(timing, ctx)

        game.execute_effects = capture
        game.decode_action(0, 1)
        game.decode_action(SEL_HAND_START + 1, 1)
        game.decode_action(62, 1)

        assert captured_ctx.get("digixros_count") == 1

    def test_removal_cause_digixros(self, db):
        """WhenRemoveField should fire with removal_cause='digixros' for field materials."""
        game, p1, _ = _setup_game(db)
        xros = db.create_card_source("BT10-009", p1)
        shoutmon = db.create_card_source("BT10-008", p1)
        field_perm = Permanent([shoutmon])
        field_perm._owner_game = game
        p1.battle_area = [field_perm]
        p1.hand_cards = [xros]
        game.memory = 20

        removal_causes = []
        original = game.execute_effects

        def capture(timing, ctx=None):
            if timing == EffectTiming.WhenRemoveField:
                removal_causes.append((ctx or {}).get("removal_cause"))
            return original(timing, ctx)

        game.execute_effects = capture
        game.decode_action(0, 1)
        game.decode_action(SEL_MY_FIELD_START, 1)
        game.decode_action(62, 1)

        assert "digixros" in removal_causes

    def test_action_mask_allows_digixros(self, db):
        game, p1, _ = _setup_game(db)
        xros = db.create_card_source("BT10-009", p1)  # cost 9
        shoutmon = db.create_card_source("BT10-008", p1)
        p1.hand_cards = [xros, shoutmon]
        game.memory = 3  # Can't afford 9 normally, but with 4 mats => 9-8=1

        mask = build_action_mask(game, 1)
        assert mask[0] == 1.0  # Should be playable with DigiXros reduction

    def test_no_materials_play_at_full_cost(self, db):
        """DigiXros card with no valid materials should still be playable at full cost."""
        game, p1, _ = _setup_game(db)
        xros = db.create_card_source("BT10-009", p1)  # cost 9
        p1.hand_cards = [xros]
        game.memory = 10

        # No materials -> not has_any_digixros_material -> effective_cost = play_cost = 9
        mask = build_action_mask(game, 1)
        assert mask[0] == 1.0  # memory 10 >= cost 9

    def test_pending_cleared_on_game_over(self, db):
        game, p1, p2 = _setup_game(db)
        game._pending_digixros = {"some": "data"}
        game.declare_winner(p1)
        assert game._pending_digixros is None

    def test_assembly_from_trash(self, db):
        """Assembly cards should select materials from trash."""
        game, p1, _ = _setup_game(db)
        # EX9-055: Assembly -6, 4 [Negamon]
        assembly_card = db.create_card_source("EX9-055", p1)
        if assembly_card is None:
            pytest.skip("EX9-055 not found")
        p1.hand_cards = [assembly_card]

        # Find Negamon cards for trash
        negamon_ids = [
            cid for cid, c in db.cards.items()
            if "negamon" in c.card_name_eng.lower()
        ]
        if len(negamon_ids) < 4:
            pytest.skip("Not enough Negamon cards")
        for cid in negamon_ids[:4]:
            cs = db.create_card_source(cid, p1)
            p1.trash_cards.append(cs)

        game.memory = 30
        game.decode_action(0, 1)  # play

        if game.current_phase == GamePhase.SelectMaterial:
            # Select all 4 from trash
            for i in range(4):
                if game.current_phase != GamePhase.SelectMaterial:
                    break
                game.decode_action(SEL_TRASH_START + i, 1)

            if game.current_phase == GamePhase.SelectMaterial:
                game.decode_action(62, 1)

        assert len(p1.battle_area) >= 1
        assert len(p1.trash_cards) == 0  # all moved under card


# ─── Helpers ──────────────────────────────────────────────────────────


def _make_player():
    from engine_py_legacy.engine.core.player import Player
    return Player()


def _setup_game(db):
    game = Game()
    p1 = game.player1
    p2 = game.player2
    game.turn_player = p1
    game.opponent_player = p2
    game.current_phase = GamePhase.Main
    # Give P2 security so game doesn't end
    for _ in range(5):
        sec = db.create_card_source("BT1-001", p2)
        if sec:
            p2.security_cards.append(sec)
    return game, p1, p2
