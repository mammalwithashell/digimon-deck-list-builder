"""Tests for prompt builders in server.ai.prompts."""

from __future__ import annotations

import pytest

from server.ai.prompts import (
    _join_pinned_engine_chunks,
    build_script_autofix_messages,
)


# ---------------------------------------------------------------------------
# _join_pinned_engine_chunks tests
# ---------------------------------------------------------------------------


class TestJoinPinnedEngineChunks:
    def test_single_chunk(self):
        chunks = [
            {"function_name": "draw_cards", "text": "def draw_cards(self): pass"},
        ]
        result = _join_pinned_engine_chunks(chunks)
        assert "[Method 1] draw_cards" in result
        assert "def draw_cards(self): pass" in result

    def test_multiple_chunks(self):
        chunks = [
            {"function_name": "draw_cards", "text": "def draw_cards(self): pass"},
            {"function_name": "effect_trash_cards", "text": "def effect_trash_cards(self): pass"},
        ]
        result = _join_pinned_engine_chunks(chunks)
        assert "[Method 1] draw_cards" in result
        assert "[Method 2] effect_trash_cards" in result
        assert "def draw_cards" in result
        assert "def effect_trash_cards" in result

    def test_empty_list(self):
        result = _join_pinned_engine_chunks([])
        assert result == ""

    def test_missing_function_name(self):
        chunks = [{"text": "some method body"}]
        result = _join_pinned_engine_chunks(chunks)
        assert "[Method 1] unknown" in result
        assert "some method body" in result

    def test_chunks_separated_by_double_newline(self):
        chunks = [
            {"function_name": "a", "text": "body_a"},
            {"function_name": "b", "text": "body_b"},
        ]
        result = _join_pinned_engine_chunks(chunks)
        # Should be separated by double newline
        parts = result.split("\n\n")
        assert len(parts) == 2


# ---------------------------------------------------------------------------
# build_script_autofix_messages with pinned_engine_chunks tests
# ---------------------------------------------------------------------------


class TestBuildScriptAutofixMessagesWithPinnedChunks:
    def _base_kwargs(self, **overrides):
        defaults = {
            "card_id": "BT13-010",
            "issue_description": "Draw effect not triggering.",
            "scope_profile": "script",
            "allowed_paths": ["scripts/generated/bt13/bt13_010.py"],
            "file_contexts": [
                {
                    "path": "scripts/generated/bt13/bt13_010.py",
                    "hash": "abc123",
                    "content": "class BT13_010: pass",
                }
            ],
            "context_chunks": [
                {"source": "RULES.md", "text": "Draw rules context."},
            ],
        }
        defaults.update(overrides)
        return defaults

    def test_no_pinned_chunks_omits_section(self):
        system, user = build_script_autofix_messages(**self._base_kwargs())
        assert "Engine API methods used by this script" not in user

    def test_pinned_chunks_none_omits_section(self):
        system, user = build_script_autofix_messages(
            **self._base_kwargs(pinned_engine_chunks=None)
        )
        assert "Engine API methods used by this script" not in user

    def test_pinned_chunks_empty_list_omits_section(self):
        system, user = build_script_autofix_messages(
            **self._base_kwargs(pinned_engine_chunks=[])
        )
        assert "Engine API methods used by this script" not in user

    def test_pinned_chunks_included_in_prompt(self):
        pinned = [
            {
                "function_name": "draw_cards",
                "text": "def draw_cards(self, player, count):\n    pass",
                "source": "digimon_gym/engine/game.py",
            },
        ]
        system, user = build_script_autofix_messages(
            **self._base_kwargs(pinned_engine_chunks=pinned)
        )
        assert "Engine API methods used by this script:" in user
        assert "[Method 1] draw_cards" in user
        assert "def draw_cards(self, player, count):" in user

    def test_pinned_section_between_file_and_rag_context(self):
        pinned = [
            {
                "function_name": "draw_cards",
                "text": "def draw_cards(self): pass",
                "source": "game.py",
            },
        ]
        system, user = build_script_autofix_messages(
            **self._base_kwargs(pinned_engine_chunks=pinned)
        )
        # File context should come before pinned section
        file_pos = user.find("Current file context:")
        pinned_pos = user.find("Engine API methods used by this script:")
        rag_pos = user.find("Retrieved rules/engine context:")
        assert file_pos < pinned_pos < rag_pos

    def test_multiple_pinned_chunks(self):
        pinned = [
            {
                "function_name": "draw_cards",
                "text": "def draw_cards(self): pass",
                "source": "game.py",
            },
            {
                "function_name": "effect_trash_cards",
                "text": "def effect_trash_cards(self): pass",
                "source": "game.py",
            },
        ]
        system, user = build_script_autofix_messages(
            **self._base_kwargs(pinned_engine_chunks=pinned)
        )
        assert "[Method 1] draw_cards" in user
        assert "[Method 2] effect_trash_cards" in user

    def test_system_prompt_unchanged(self):
        """The system prompt should not be affected by pinned chunks."""
        pinned = [
            {"function_name": "draw_cards", "text": "body", "source": "game.py"},
        ]
        system_with, _ = build_script_autofix_messages(
            **self._base_kwargs(pinned_engine_chunks=pinned)
        )
        system_without, _ = build_script_autofix_messages(
            **self._base_kwargs()
        )
        assert system_with == system_without

    def test_rag_context_still_present(self):
        pinned = [
            {"function_name": "draw_cards", "text": "body", "source": "game.py"},
        ]
        _, user = build_script_autofix_messages(
            **self._base_kwargs(pinned_engine_chunks=pinned)
        )
        # RAG context should still be present
        assert "Draw rules context." in user
        assert "[Context 1]" in user
