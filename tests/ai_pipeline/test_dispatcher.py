"""Tests for extract_engine_calls and lookup_pinned_engine_methods in dispatcher."""

from __future__ import annotations

import json
import textwrap
from pathlib import Path

import pytest

from server.ai.dispatcher import extract_engine_calls, lookup_pinned_engine_methods
from server.ai.retrieval import LocalRAGIndex


# ---------------------------------------------------------------------------
# extract_engine_calls tests
# ---------------------------------------------------------------------------


class TestExtractEngineCalls:
    def test_game_dot_method(self):
        script = 'game.effect_digivolve_from_hand(player, perm, digi_filter)'
        result = extract_engine_calls(script)
        assert result == ["effect_digivolve_from_hand"]

    def test_player_dot_method(self):
        script = 'player.draw_cards(1)'
        result = extract_engine_calls(script)
        assert result == ["draw_cards"]

    def test_perm_dot_method(self):
        script = "perm.grant_keyword('_is_rush')"
        result = extract_engine_calls(script)
        assert result == ["grant_keyword"]

    def test_permanent_dot_method(self):
        script = "permanent.unsuspend()"
        result = extract_engine_calls(script)
        assert result == ["unsuspend"]

    def test_ctx_get_game_single_quotes(self):
        script = "ctx.get('game').effect_trash_cards(player, 1)"
        result = extract_engine_calls(script)
        assert result == ["effect_trash_cards"]

    def test_ctx_get_game_double_quotes(self):
        script = 'ctx.get("game").effect_play_from_zone(player, "hand")'
        result = extract_engine_calls(script)
        assert result == ["effect_play_from_zone"]

    def test_multiple_calls_deduped(self):
        script = textwrap.dedent("""\
            game.draw_cards(player, 1)
            game.draw_cards(player, 2)
            game.effect_trash_cards(player, 1)
        """)
        result = extract_engine_calls(script)
        assert result == ["draw_cards", "effect_trash_cards"]

    def test_multiple_different_receivers(self):
        script = textwrap.dedent("""\
            game.effect_select_opponent_permanent(player, on_delete, filter_fn=target_filter)
            player.draw_cards(1)
            perm.grant_keyword('_is_rush')
        """)
        result = extract_engine_calls(script)
        assert result == ["effect_select_opponent_permanent", "draw_cards", "grant_keyword"]

    def test_preserves_first_seen_order(self):
        script = textwrap.dedent("""\
            game.effect_b(player)
            game.effect_a(player)
            game.effect_c(player)
        """)
        result = extract_engine_calls(script)
        assert result == ["effect_b", "effect_a", "effect_c"]

    def test_empty_script(self):
        assert extract_engine_calls("") == []

    def test_no_matches(self):
        script = textwrap.dedent("""\
            effect0 = ICardEffect()
            effect0.set_effect_name("test")
            x = some_function()
        """)
        result = extract_engine_calls(script)
        assert result == []

    def test_realistic_bt13_script(self):
        script = textwrap.dedent("""\
            def process0(ctx):
                player = ctx.get('player')
                perm = ctx.get('permanent')
                game = ctx.get('game')
                if not (player and game):
                    return
                def target_filter(p):
                    if p.dp is None or p.dp > 2000:
                        return False
                    return p.is_digimon
                def on_delete(target_perm):
                    enemy = player.enemy if player else None
                    if enemy:
                        enemy.delete_permanent(target_perm)
                game.effect_select_opponent_permanent(
                    player, on_delete, filter_fn=target_filter, is_optional=False)
        """)
        result = extract_engine_calls(script)
        # "enemy" is not in the receiver list, so delete_permanent won't match
        # Only game.effect_select_opponent_permanent should match
        assert "effect_select_opponent_permanent" in result

    def test_multiline_method_call(self):
        """The regex should still match when method call spans multiple lines."""
        script = textwrap.dedent("""\
            game.effect_digivolve_from_hand(
                player, perm, digi_filter, is_optional=True)
        """)
        result = extract_engine_calls(script)
        assert result == ["effect_digivolve_from_hand"]

    def test_does_not_match_uppercase_methods(self):
        """Only lowercase method names should match per the regex."""
        script = 'game.SomeCapitalMethod(player)'
        result = extract_engine_calls(script)
        assert result == []

    def test_method_with_digits(self):
        script = 'game.effect_dp_plus_3000(player, perm)'
        result = extract_engine_calls(script)
        assert result == ["effect_dp_plus_3000"]

    def test_combined_script_with_play_and_draw(self):
        """Test a realistic script that calls both game and player methods."""
        script = textwrap.dedent("""\
            def process1(ctx):
                player = ctx.get('player')
                perm = ctx.get('permanent')
                game = ctx.get('game')
                if not (player and game):
                    return
                def play_filter(c):
                    return True
                game.effect_play_from_zone(
                    player, 'hand', play_filter, free=True, is_optional=True)
                if perm:
                    perm.grant_keyword('_is_rush')

            def process2(ctx):
                player = ctx.get('player')
                if player:
                    player.draw_cards(1)
        """)
        result = extract_engine_calls(script)
        assert result == ["effect_play_from_zone", "grant_keyword", "draw_cards"]


# ---------------------------------------------------------------------------
# Helper to create a test RAG index
# ---------------------------------------------------------------------------


def _write_test_index(tmp_path: Path, chunks: list[dict]) -> Path:
    """Write a minimal index.json and return its path."""
    index_file = tmp_path / "index.json"
    data = {
        "version": 2,
        "chunk_count": len(chunks),
        "sources": list({c.get("source", "test") for c in chunks}),
        "chunks": chunks,
    }
    index_file.write_text(json.dumps(data), encoding="utf-8")
    return index_file


# ---------------------------------------------------------------------------
# lookup_pinned_engine_methods tests
# ---------------------------------------------------------------------------


class TestLookupPinnedEngineMethods:
    def _make_engine_chunks(self) -> list[dict]:
        return [
            {
                "chunk_id": "game.py:Game.draw_cards",
                "source": "digimon_gym/engine/game.py",
                "source_type": "engine_api",
                "function_name": "draw_cards",
                "class_name": "Game",
                "text": "def draw_cards(self, player, count):\n    pass",
                "embedding": None,
            },
            {
                "chunk_id": "game.py:Game.effect_digivolve_from_hand",
                "source": "digimon_gym/engine/game.py",
                "source_type": "engine_api",
                "function_name": "effect_digivolve_from_hand",
                "class_name": "Game",
                "text": "def effect_digivolve_from_hand(self, player, permanent, filter_fn):\n    pass",
                "embedding": None,
            },
            {
                "chunk_id": "game.py:Game.effect_trash_cards",
                "source": "digimon_gym/engine/game.py",
                "source_type": "engine_api",
                "function_name": "effect_trash_cards",
                "class_name": "Game",
                "text": "def effect_trash_cards(self, player, count):\n    pass",
                "embedding": None,
            },
            {
                "chunk_id": "RULES.md:0",
                "source": "RULES.md",
                "source_type": "rules",
                "function_name": None,
                "text": "Rules for drawing cards in the game.",
                "embedding": None,
            },
        ]

    def test_exact_match_single(self, tmp_path: Path):
        chunks = self._make_engine_chunks()
        index_file = _write_test_index(tmp_path, chunks)
        idx = LocalRAGIndex(path=index_file)

        results = lookup_pinned_engine_methods(idx, ["draw_cards"])
        assert len(results) == 1
        assert results[0]["function_name"] == "draw_cards"
        assert "def draw_cards" in results[0]["text"]
        assert results[0]["source"] == "digimon_gym/engine/game.py"

    def test_exact_match_multiple(self, tmp_path: Path):
        chunks = self._make_engine_chunks()
        index_file = _write_test_index(tmp_path, chunks)
        idx = LocalRAGIndex(path=index_file)

        results = lookup_pinned_engine_methods(
            idx, ["draw_cards", "effect_digivolve_from_hand"]
        )
        assert len(results) == 2
        fn_names = [r["function_name"] for r in results]
        assert fn_names == ["draw_cards", "effect_digivolve_from_hand"]

    def test_deduplication(self, tmp_path: Path):
        chunks = self._make_engine_chunks()
        index_file = _write_test_index(tmp_path, chunks)
        idx = LocalRAGIndex(path=index_file)

        results = lookup_pinned_engine_methods(
            idx, ["draw_cards", "draw_cards", "draw_cards"]
        )
        assert len(results) == 1
        assert results[0]["function_name"] == "draw_cards"

    def test_retrieval_fallback(self, tmp_path: Path):
        """When no exact match, falls back to retrieval."""
        chunks = self._make_engine_chunks()
        index_file = _write_test_index(tmp_path, chunks)
        idx = LocalRAGIndex(path=index_file)

        # "nonexistent_method" won't have an exact function_name match,
        # but retrieval should still return something from engine_api chunks
        results = lookup_pinned_engine_methods(idx, ["nonexistent_method"])
        # May or may not find something depending on BM25, but should not error
        assert isinstance(results, list)

    def test_empty_method_names(self, tmp_path: Path):
        chunks = self._make_engine_chunks()
        index_file = _write_test_index(tmp_path, chunks)
        idx = LocalRAGIndex(path=index_file)

        results = lookup_pinned_engine_methods(idx, [])
        assert results == []

    def test_no_chunks_in_index(self, tmp_path: Path):
        index_file = _write_test_index(tmp_path, [])
        idx = LocalRAGIndex(path=index_file)

        results = lookup_pinned_engine_methods(idx, ["draw_cards"])
        assert results == []

    def test_ensures_loaded(self, tmp_path: Path):
        """Verifies that ensure_loaded() is called."""
        chunks = self._make_engine_chunks()
        index_file = _write_test_index(tmp_path, chunks)
        idx = LocalRAGIndex(path=index_file)
        assert not idx.loaded

        lookup_pinned_engine_methods(idx, ["draw_cards"])
        assert idx.loaded

    def test_exact_match_before_retrieval(self, tmp_path: Path):
        """Exact function_name match should be preferred over retrieval results."""
        chunks = [
            {
                "chunk_id": "game.py:Game.draw_cards",
                "source": "digimon_gym/engine/game.py",
                "source_type": "engine_api",
                "function_name": "draw_cards",
                "text": "EXACT MATCH def draw_cards(self, player, count): pass",
                "embedding": None,
            },
            {
                "chunk_id": "game.py:Game.draw_phase",
                "source": "digimon_gym/engine/game.py",
                "source_type": "engine_api",
                "function_name": "draw_phase",
                "text": "draw_cards mentioned in draw_phase method description",
                "embedding": None,
            },
        ]
        index_file = _write_test_index(tmp_path, chunks)
        idx = LocalRAGIndex(path=index_file)

        results = lookup_pinned_engine_methods(idx, ["draw_cards"])
        assert len(results) == 1
        assert "EXACT MATCH" in results[0]["text"]

    def test_result_dict_structure(self, tmp_path: Path):
        """Results should have exactly text, source, and function_name keys."""
        chunks = self._make_engine_chunks()
        index_file = _write_test_index(tmp_path, chunks)
        idx = LocalRAGIndex(path=index_file)

        results = lookup_pinned_engine_methods(idx, ["draw_cards"])
        assert len(results) == 1
        r = results[0]
        assert set(r.keys()) == {"text", "source", "function_name"}

    def test_mixed_exact_and_fallback(self, tmp_path: Path):
        """Some methods match exactly, others need retrieval fallback."""
        chunks = self._make_engine_chunks()
        index_file = _write_test_index(tmp_path, chunks)
        idx = LocalRAGIndex(path=index_file)

        results = lookup_pinned_engine_methods(
            idx, ["draw_cards", "totally_unknown_method"]
        )
        # draw_cards should always be found via exact match
        assert any(r["function_name"] == "draw_cards" for r in results)


class TestLLMTranspileDispatch:
    def test_rejects_missing_card_id(self):
        from server.ai.dispatcher import TaskDispatcher
        d = TaskDispatcher(rag_index=None, client=None)
        with pytest.raises(ValueError, match="card_id"):
            d.run("llm_transpile", {"set_id": "bt24", "module_name": "bt24_042"})

    def test_unknown_task_type_raises(self):
        from server.ai.dispatcher import TaskDispatcher
        d = TaskDispatcher(rag_index=None, client=None)
        with pytest.raises(ValueError, match="Unsupported"):
            d.run("nonexistent_type", {})
