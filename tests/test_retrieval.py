"""Tests for source-type-aware chunking in digimon_gym.ai.retrieval."""

from __future__ import annotations

import json
import textwrap
from pathlib import Path

import pytest

from digimon_gym.ai.retrieval import (
    Chunk,
    LocalRAGIndex,
    chunk_cards_json,
    chunk_markdown_by_section,
    chunk_python_by_ast,
    chunk_text,
    _infer_python_source_type,
)


# ---------------------------------------------------------------------------
# Chunk dataclass tests
# ---------------------------------------------------------------------------


class TestChunkDataclass:
    def test_default_fields(self):
        c = Chunk(chunk_id="test:0", source="test.py", text="hello", embedding=None)
        assert c.source_type == "rules"
        assert c.function_name is None
        assert c.class_name is None

    def test_as_dict_minimal(self):
        c = Chunk(chunk_id="test:0", source="test.py", text="hello", embedding=None)
        d = c.as_dict()
        assert d["source_type"] == "rules"
        assert "function_name" not in d
        assert "class_name" not in d

    def test_as_dict_with_function(self):
        c = Chunk(
            chunk_id="game.py:Game.decode_action",
            source="digimon_gym/engine/game.py",
            text="def decode_action(self): pass",
            embedding=None,
            source_type="engine_api",
            function_name="decode_action",
            class_name="Game",
        )
        d = c.as_dict()
        assert d["source_type"] == "engine_api"
        assert d["function_name"] == "decode_action"
        assert d["class_name"] == "Game"
        assert d["chunk_id"] == "game.py:Game.decode_action"

    def test_as_dict_function_only(self):
        c = Chunk(
            chunk_id="game.py:helper",
            source="game.py",
            text="def helper(): pass",
            embedding=None,
            source_type="engine_api",
            function_name="helper",
        )
        d = c.as_dict()
        assert d["function_name"] == "helper"
        assert "class_name" not in d


# ---------------------------------------------------------------------------
# chunk_python_by_ast tests
# ---------------------------------------------------------------------------


class TestChunkPythonByAst:
    def test_top_level_function(self):
        source = textwrap.dedent("""\
            def greet(name):
                \"\"\"Say hello.\"\"\"
                return f"Hello {name}"
        """)
        chunks = chunk_python_by_ast(source, "digimon_gym/engine/game.py")
        assert len(chunks) == 1
        assert chunks[0]["function_name"] == "greet"
        assert chunks[0]["class_name"] is None
        assert chunks[0]["source_type"] == "engine_api"
        assert "def greet" in chunks[0]["text"]

    def test_class_methods(self):
        source = textwrap.dedent("""\
            class MyClass:
                def method_a(self):
                    pass

                def method_b(self):
                    return 42
        """)
        chunks = chunk_python_by_ast(source, "digimon_gym/engine/core/permanent.py")
        assert len(chunks) == 2
        names = {c["function_name"] for c in chunks}
        assert names == {"method_a", "method_b"}
        for c in chunks:
            assert c["class_name"] == "MyClass"
            assert c["source_type"] == "engine_api"

    def test_transpiler_source_type(self):
        source = "def pattern_match(x):\n    return x\n"
        chunks = chunk_python_by_ast(source, "tools/transpiler/patterns.py")
        assert chunks[0]["source_type"] == "transpiler"

    def test_syntax_error_fallback(self):
        source = "def broken(\n  this is not valid python"
        chunks = chunk_python_by_ast(source, "digimon_gym/engine/game.py")
        assert len(chunks) >= 1
        for c in chunks:
            assert c["source_type"] == "engine_api"
            assert c["function_name"] is None

    def test_empty_module_fallback(self):
        source = "# Just a comment\nIMPORT_CONSTANT = 42\n"
        chunks = chunk_python_by_ast(source, "digimon_gym/engine/game.py")
        # Falls back to text chunking since no functions/classes with methods
        assert len(chunks) >= 1
        for c in chunks:
            assert c["source_type"] == "engine_api"

    def test_async_function(self):
        source = textwrap.dedent("""\
            async def fetch_data():
                return await get()
        """)
        chunks = chunk_python_by_ast(source, "digimon_gym/engine/game.py")
        assert len(chunks) == 1
        assert chunks[0]["function_name"] == "fetch_data"

    def test_mixed_top_level_and_class(self):
        source = textwrap.dedent("""\
            def helper():
                pass

            class Engine:
                def run(self):
                    pass

                def stop(self):
                    pass
        """)
        chunks = chunk_python_by_ast(source, "digimon_gym/engine/game.py")
        assert len(chunks) == 3
        names = [(c["function_name"], c["class_name"]) for c in chunks]
        assert ("helper", None) in names
        assert ("run", "Engine") in names
        assert ("stop", "Engine") in names


# ---------------------------------------------------------------------------
# chunk_markdown_by_section tests
# ---------------------------------------------------------------------------


class TestChunkMarkdownBySection:
    def test_basic_sections(self):
        md = textwrap.dedent("""\
            # Title

            Some intro text.

            ## Section One

            Content of section one.

            ## Section Two

            Content of section two.
        """)
        chunks = chunk_markdown_by_section(md, "RULES.md")
        # Should have: intro (no title), Section One, Section Two
        assert len(chunks) >= 2
        titles = [c["section_title"] for c in chunks]
        assert "Section One" in titles
        assert "Section Two" in titles
        for c in chunks:
            assert c["source_type"] == "rules"

    def test_long_section_sub_chunked(self):
        # Create a section > 2000 chars
        long_content = "word " * 500  # ~2500 chars
        md = f"## Long Section\n\n{long_content}\n"
        chunks = chunk_markdown_by_section(md, "SPEC.md")
        assert len(chunks) >= 2  # Should be sub-chunked
        for c in chunks:
            assert c["section_title"] == "Long Section"
            assert c["source_type"] == "rules"

    def test_short_section_not_split(self):
        md = "## Short\n\nJust a few words.\n"
        chunks = chunk_markdown_by_section(md, "SPEC.md")
        assert len(chunks) == 1
        assert chunks[0]["section_title"] == "Short"

    def test_empty_text(self):
        chunks = chunk_markdown_by_section("", "empty.md")
        assert chunks == []

    def test_no_headers(self):
        md = "Just some plain text without any headers.\n"
        chunks = chunk_markdown_by_section(md, "plain.md")
        # Should produce one chunk with empty title
        assert len(chunks) == 1
        assert chunks[0]["section_title"] == ""


# ---------------------------------------------------------------------------
# chunk_cards_json tests
# ---------------------------------------------------------------------------


class TestChunkCardsJson:
    def test_dict_format(self):
        data = {
            "BT1-001": {"card_id": "BT1-001", "name": "Agumon", "level": 3},
            "BT1-002": {"card_id": "BT1-002", "name": "Greymon", "level": 4},
        }
        source_text = json.dumps(data)
        chunks = chunk_cards_json(source_text, "cards.json")
        assert len(chunks) == 2
        ids = {c["card_id"] for c in chunks}
        assert ids == {"BT1-001", "BT1-002"}
        for c in chunks:
            assert c["source_type"] == "card_metadata"
            # text should be valid JSON
            parsed = json.loads(c["text"])
            assert isinstance(parsed, dict)

    def test_list_format(self):
        data = [
            {"card_id": "BT1-001", "name": "Agumon"},
            {"card_id": "BT1-002", "name": "Greymon"},
        ]
        source_text = json.dumps(data)
        chunks = chunk_cards_json(source_text, "cards.json")
        assert len(chunks) == 2
        assert chunks[0]["card_id"] == "BT1-001"
        assert chunks[1]["card_id"] == "BT1-002"

    def test_invalid_json_fallback(self):
        chunks = chunk_cards_json("not valid json{{{", "cards.json")
        assert len(chunks) >= 1
        for c in chunks:
            assert c["source_type"] == "card_metadata"
            assert c["card_id"] is None

    def test_empty_dict(self):
        chunks = chunk_cards_json("{}", "cards.json")
        assert chunks == []

    def test_empty_list(self):
        chunks = chunk_cards_json("[]", "cards.json")
        assert chunks == []


# ---------------------------------------------------------------------------
# _infer_python_source_type tests
# ---------------------------------------------------------------------------


class TestInferPythonSourceType:
    def test_engine_path(self):
        assert _infer_python_source_type("digimon_gym/engine/game.py") == "engine_api"

    def test_engine_subpath(self):
        assert _infer_python_source_type("digimon_gym/engine/core/permanent.py") == "engine_api"

    def test_transpiler_path(self):
        assert _infer_python_source_type("tools/transpiler/patterns.py") == "transpiler"

    def test_other_path(self):
        assert _infer_python_source_type("some/other/module.py") == "rules"

    def test_windows_backslash(self):
        assert _infer_python_source_type("digimon_gym\\engine\\game.py") == "engine_api"
        assert _infer_python_source_type("tools\\transpiler\\patterns.py") == "transpiler"


# ---------------------------------------------------------------------------
# build_local_index integration test (using tmp_path)
# ---------------------------------------------------------------------------


class TestBuildLocalIndex:
    def test_builds_v2_index(self, tmp_path: Path):
        """Build an index from minimal synthetic sources and verify v2 format."""
        from digimon_gym.ai.retrieval import build_local_index

        # Create a small Python file
        engine_dir = tmp_path / "digimon_gym" / "engine"
        engine_dir.mkdir(parents=True)
        py_file = engine_dir / "sample.py"
        py_file.write_text("def decode():\n    pass\n", encoding="utf-8")

        # Create a small markdown file
        md_file = tmp_path / "RULES.md"
        md_file.write_text("## Overview\n\nSome rules.\n", encoding="utf-8")

        # Create a small cards.json
        cards_file = tmp_path / "cards.json"
        cards_file.write_text(
            json.dumps({"C-001": {"card_id": "C-001", "name": "TestCard"}}),
            encoding="utf-8",
        )

        output = tmp_path / "index.json"
        result = build_local_index(
            output_path=output,
            source_paths=[py_file, md_file, cards_file],
        )

        assert result["version"] == 2
        assert result["chunk_count"] > 0

        # Verify source types
        source_types_found = {c["source_type"] for c in result["chunks"]}
        assert "engine_api" in source_types_found
        assert "rules" in source_types_found
        assert "card_metadata" in source_types_found

        # Verify Python chunk has function_name
        py_chunks = [c for c in result["chunks"] if c["source_type"] == "engine_api"]
        assert any(c.get("function_name") == "decode" for c in py_chunks)

        # Verify card chunk has correct card_id in chunk_id
        card_chunks = [c for c in result["chunks"] if c["source_type"] == "card_metadata"]
        assert any("C-001" in c["chunk_id"] for c in card_chunks)

        # Verify file was written
        assert output.exists()
        written = json.loads(output.read_text(encoding="utf-8"))
        assert written["version"] == 2


# ---------------------------------------------------------------------------
# LocalRAGIndex v1 backward compatibility test
# ---------------------------------------------------------------------------


class TestLocalRAGIndexLoad:
    def test_load_v1_defaults_source_type(self, tmp_path: Path):
        """Loading a v1 index should default source_type to 'rules'."""
        index_file = tmp_path / "index.json"
        v1_data = {
            "version": 1,
            "chunk_count": 1,
            "sources": ["RULES.md"],
            "chunks": [
                {
                    "chunk_id": "RULES.md:0",
                    "source": "RULES.md",
                    "text": "some rules content",
                    "embedding": None,
                }
            ],
        }
        index_file.write_text(json.dumps(v1_data), encoding="utf-8")

        idx = LocalRAGIndex(path=index_file)
        idx.load()
        assert idx.loaded
        assert len(idx.chunks) == 1
        assert idx.chunks[0]["source_type"] == "rules"
        assert idx.chunks[0]["function_name"] is None
        assert idx.chunks[0]["class_name"] is None

    def test_load_v2_preserves_fields(self, tmp_path: Path):
        """Loading a v2 index should preserve all new fields."""
        index_file = tmp_path / "index.json"
        v2_data = {
            "version": 2,
            "chunk_count": 1,
            "sources": ["digimon_gym/engine/game.py"],
            "chunks": [
                {
                    "chunk_id": "game.py:Game.run",
                    "source": "digimon_gym/engine/game.py",
                    "source_type": "engine_api",
                    "function_name": "run",
                    "class_name": "Game",
                    "text": "def run(self): pass",
                    "embedding": None,
                }
            ],
        }
        index_file.write_text(json.dumps(v2_data), encoding="utf-8")

        idx = LocalRAGIndex(path=index_file)
        idx.load()
        assert idx.loaded
        c = idx.chunks[0]
        assert c["source_type"] == "engine_api"
        assert c["function_name"] == "run"
        assert c["class_name"] == "Game"


# ---------------------------------------------------------------------------
# retrieve() source_types filter test
# ---------------------------------------------------------------------------


class TestRetrieveSourceTypesFilter:
    def test_filter_by_source_type(self, tmp_path: Path):
        """retrieve() with source_types should only return matching chunks."""
        index_file = tmp_path / "index.json"
        v2_data = {
            "version": 2,
            "chunk_count": 3,
            "sources": ["game.py", "RULES.md", "cards.json"],
            "chunks": [
                {
                    "chunk_id": "game.py:decode",
                    "source": "game.py",
                    "source_type": "engine_api",
                    "text": "decode action game engine method",
                    "embedding": None,
                },
                {
                    "chunk_id": "RULES.md:0",
                    "source": "RULES.md",
                    "source_type": "rules",
                    "text": "rules for the game engine actions",
                    "embedding": None,
                },
                {
                    "chunk_id": "cards.json:BT1-001",
                    "source": "cards.json",
                    "source_type": "card_metadata",
                    "text": "card BT1-001 Agumon level 3",
                    "embedding": None,
                },
            ],
        }
        index_file.write_text(json.dumps(v2_data), encoding="utf-8")

        idx = LocalRAGIndex(path=index_file)
        idx.load()

        # Filter to engine_api only
        results = idx.retrieve("game engine decode", source_types=["engine_api"])
        assert all(r["source_type"] == "engine_api" for r in results)
        assert len(results) >= 1

        # Filter to card_metadata only
        results = idx.retrieve("Agumon", source_types=["card_metadata"])
        assert all(r["source_type"] == "card_metadata" for r in results)

        # Filter to rules only
        results = idx.retrieve("game engine rules", source_types=["rules"])
        assert all(r["source_type"] == "rules" for r in results)

    def test_no_filter_returns_all_types(self, tmp_path: Path):
        """retrieve() without source_types should search all chunks."""
        index_file = tmp_path / "index.json"
        v2_data = {
            "version": 2,
            "chunk_count": 2,
            "sources": ["game.py", "RULES.md"],
            "chunks": [
                {
                    "chunk_id": "game.py:decode",
                    "source": "game.py",
                    "source_type": "engine_api",
                    "text": "decode action method for game",
                    "embedding": None,
                },
                {
                    "chunk_id": "RULES.md:0",
                    "source": "RULES.md",
                    "source_type": "rules",
                    "text": "decode action rules for game",
                    "embedding": None,
                },
            ],
        }
        index_file.write_text(json.dumps(v2_data), encoding="utf-8")

        idx = LocalRAGIndex(path=index_file)
        idx.load()

        results = idx.retrieve("decode action game")
        source_types = {r["source_type"] for r in results}
        assert len(source_types) >= 2  # both types returned

    def test_retrieve_returns_source_type_field(self, tmp_path: Path):
        """retrieve() results should include source_type."""
        index_file = tmp_path / "index.json"
        v2_data = {
            "version": 2,
            "chunk_count": 1,
            "sources": ["game.py"],
            "chunks": [
                {
                    "chunk_id": "game.py:run",
                    "source": "game.py",
                    "source_type": "engine_api",
                    "text": "run the game engine",
                    "embedding": None,
                },
            ],
        }
        index_file.write_text(json.dumps(v2_data), encoding="utf-8")

        idx = LocalRAGIndex(path=index_file)
        idx.load()

        results = idx.retrieve("run game")
        assert len(results) == 1
        assert results[0]["source_type"] == "engine_api"


# ---------------------------------------------------------------------------
# chunk_text preserved behavior
# ---------------------------------------------------------------------------


class TestChunkTextPreserved:
    def test_basic_chunking(self):
        text = "word " * 500
        chunks = chunk_text(text, chunk_size=100, overlap=20)
        assert len(chunks) > 1
        for c in chunks:
            assert len(c) <= 100

    def test_empty_text(self):
        assert chunk_text("") == []
        assert chunk_text("   ") == []
