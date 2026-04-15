"""Tests for the Rules domain object and GameService application layer."""

import pytest
from digimon_gym.engine.game.rules import Rules
from digimon_gym.engine.data.enums import GameMode, GamePhase
from digimon_gym.engine.data.deck_loader import validate_deck, RESTRICTED_LIST, CardRestriction
from digimon_gym.services.game_service import GameService


# ─── Test decks ──────────────────────────────────────────────────────
# Engine doesn't enforce copy limits — simple repetition is fine for game tests.
ENGINE_DECK = ["ST1-01"] * 5 + ["ST1-03"] * 45


class TestRulesDataclass:
    def test_standard_factory_has_banlist(self):
        rules = Rules.standard()
        assert rules.game_mode == GameMode.Standard
        assert rules.card_restrictions is not None
        assert rules.deck_size == 50
        assert rules.starting_security == 5
        assert rules.starting_hand_size == 5

    def test_no_banlist_factory(self):
        rules = Rules.no_banlist()
        assert rules.game_mode == GameMode.NoRestriction
        assert rules.card_restrictions is None

    def test_training_factory(self):
        rules = Rules.training()
        assert rules.game_mode == GameMode.NoRestriction
        assert rules.card_restrictions is None

    def test_frozen(self):
        rules = Rules.standard()
        with pytest.raises(AttributeError):
            rules.deck_size = 40  # type: ignore[misc]

    def test_custom_rules(self):
        rules = Rules(starting_security=3, memory_gauge_limit=15)
        assert rules.starting_security == 3
        assert rules.memory_gauge_limit == 15
        assert rules.deck_size == 50  # default preserved


class TestValidateDeckWithRules:
    def test_validate_with_standard_rules_bans(self):
        """Standard rules should enforce the banlist."""
        rules = Rules.standard()
        # BT2-090 is banned — put one in a 50-card deck
        deck = ["BT2-090"] + ENGINE_DECK[1:]
        result = validate_deck(deck, rules=rules)
        assert not result.is_valid
        assert any("banned" in e.lower() for e in result.errors)

    def test_validate_with_no_banlist_allows_banned(self):
        """No-banlist rules should skip all restriction checks."""
        rules = Rules.no_banlist()
        deck = ["BT2-090"] + ENGINE_DECK[1:]
        result = validate_deck(deck, rules=rules)
        assert not any("banned" in e.lower() for e in result.errors)

    def test_rules_restricted_list_param_takes_precedence(self):
        """Passing rules= should override restricted_list= param."""
        empty_restrictions = CardRestriction()
        rules = Rules(card_restrictions=empty_restrictions)
        deck = ["BT2-090"] + ENGINE_DECK[1:]
        # Even though BT2-090 is in the global RESTRICTED_LIST,
        # rules.card_restrictions (empty) takes precedence
        result = validate_deck(deck, rules=rules)
        assert not any("banned" in e.lower() for e in result.errors)

    def test_validate_default_uses_restricted_list(self):
        """Calling without rules should still use the default RESTRICTED_LIST."""
        # BT2-090 is banned under default
        deck = ["BT2-090"] + ENGINE_DECK[1:]
        result = validate_deck(deck)
        assert any("banned" in e.lower() for e in result.errors)


class TestGameService:
    def test_create_headless_game(self):
        svc = GameService()
        game_id, runner = svc.create_headless_game(ENGINE_DECK, ENGINE_DECK)
        assert game_id
        assert runner.game is not None
        assert svc.get(game_id) is runner

    def test_create_headless_with_custom_security(self):
        """Rules.starting_security is respected after mulligans complete."""
        svc = GameService()
        rules = Rules(starting_security=3)
        game_id, runner = svc.create_headless_game(ENGINE_DECK, ENGINE_DECK, rules=rules)
        game = runner.game
        # Game starts in mulligan — advance through it
        while game.current_phase == GamePhase.Mulligan:
            game.decode_action(0, game.current_player_id)
        assert len(game.player1.security_cards) == 3
        assert len(game.player2.security_cards) == 3

    def test_default_rules_give_5_security(self):
        """Default rules give 5 security after mulligans."""
        svc = GameService()
        game_id, runner = svc.create_headless_game(ENGINE_DECK, ENGINE_DECK)
        game = runner.game
        while game.current_phase == GamePhase.Mulligan:
            game.decode_action(0, game.current_player_id)
        assert len(game.player1.security_cards) == 5
        assert len(game.player2.security_cards) == 5

    def test_delete_game(self):
        svc = GameService()
        game_id, _ = svc.create_headless_game(ENGINE_DECK, ENGINE_DECK)
        svc.delete(game_id)
        assert svc.get(game_id) is None

    def test_evict_oldest_on_capacity(self):
        svc = GameService(max_games=2)
        id1, _ = svc.create_headless_game(ENGINE_DECK, ENGINE_DECK)
        id2, _ = svc.create_headless_game(ENGINE_DECK, ENGINE_DECK)
        id3, _ = svc.create_headless_game(ENGINE_DECK, ENGINE_DECK)
        assert svc.get(id1) is None
        assert svc.get(id2) is not None
        assert svc.get(id3) is not None

    def test_active_games_alias(self):
        svc = GameService()
        game_id, runner = svc.create_headless_game(ENGINE_DECK, ENGINE_DECK)
        assert svc.active_games[game_id] is runner

    def test_rules_stored_on_game(self):
        """The Rules object should be accessible on the Game instance."""
        svc = GameService()
        rules = Rules.no_banlist()
        game_id, runner = svc.create_headless_game(ENGINE_DECK, ENGINE_DECK, rules=rules)
        assert runner.game.rules is rules
        assert runner.game.rules.game_mode == GameMode.NoRestriction
