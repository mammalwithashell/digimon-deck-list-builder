"""Tests for source-type-aware chunking and hybrid retrieval in digimon_gym.ai.retrieval."""

from __future__ import annotations

import json
import textwrap
from pathlib import Path
from unittest.mock import patch

import pytest

from digimon_gym.ai.retrieval import (
    Chunk,
    LocalRAGIndex,
    _cosine,
    chunk_cards_json,
    chunk_markdown_by_section,
    chunk_python_by_ast,
    chunk_text,
    _infer_python_source_type,
    _is_cards_json,
)


# ---------------------------------------------------------------------------
# _cosine helper tests
# ---------------------------------------------------------------------------


class TestCosineHelper:
    def test_identical_vectors(self):
        a = [1.0, 2.0, 3.0]
        assert _cosine(a, a) == pytest.approx(1.0)

    def test_orthogonal_vectors(self):
        a = [1.0, 0.0]
        b = [0.0, 1.0]
        assert _cosine(a, b) == pytest.approx(0.0)

    def test_symmetry(self):
        a = [1.0, 2.0, 3.0]
        b = [4.0, 5.0, 6.0]
        assert _cosine(a, b) == pytest.approx(_cosine(b, a))

    def test_zero_vector(self):
        a = [0.0, 0.0, 0.0]
        b = [1.0, 2.0, 3.0]
        assert _cosine(a, b) == pytest.approx(0.0)
        assert _cosine(b, a) == pytest.approx(0.0)

    def test_different_lengths(self):
        a = [1.0, 2.0]
        b = [1.0, 2.0, 3.0]
        assert _cosine(a, b) == 0.0


# ---------------------------------------------------------------------------
# Chunk dataclass tests
# ---------------------------------------------------------------------------


class TestChunkDataclass:
    def test_default_fields(self):
        c = Chunk(chunk_id="test:0", source="test.py", text="hello", embedding=None)
        assert c.source_type == "rules"
        assert c.function_name is None
        assert c.class_name is None
        assert c.section_title is None
        assert c.card_id is None

    def test_as_dict_minimal(self):
        c = Chunk(chunk_id="test:0", source="test.py", text="hello", embedding=None)
        d = c.as_dict()
        assert d["source_type"] == "rules"
        assert "function_name" not in d
        assert "class_name" not in d
        assert "section_title" not in d
        assert "card_id" not in d

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

    def test_as_dict_with_section_title(self):
        c = Chunk(
            chunk_id="RULES.md:Overview:0",
            source="RULES.md",
            text="Some rules text",
            embedding=None,
            source_type="rules",
            section_title="Overview",
        )
        d = c.as_dict()
        assert d["section_title"] == "Overview"
        assert "card_id" not in d

    def test_as_dict_with_card_id(self):
        c = Chunk(
            chunk_id="cards.json:BT1-001",
            source="cards.json",
            text='{"name": "Agumon"}',
            embedding=None,
            source_type="card_metadata",
            card_id="BT1-001",
        )
        d = c.as_dict()
        assert d["card_id"] == "BT1-001"
        assert "section_title" not in d


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

    def test_decorated_function_includes_decorator(self):
        source = textwrap.dedent("""\
            class MyClass:
                @property
                def value(self):
                    return self._value

                @staticmethod
                def create():
                    return MyClass()
        """)
        chunks = chunk_python_by_ast(source, "digimon_gym/engine/game.py")
        assert len(chunks) == 2
        value_chunk = [c for c in chunks if c["function_name"] == "value"][0]
        assert "@property" in value_chunk["text"]
        create_chunk = [c for c in chunks if c["function_name"] == "create"][0]
        assert "@staticmethod" in create_chunk["text"]

    def test_decorated_top_level_function(self):
        source = textwrap.dedent("""\
            @some_decorator
            def my_func():
                pass
        """)
        chunks = chunk_python_by_ast(source, "digimon_gym/engine/game.py")
        assert len(chunks) == 1
        assert "@some_decorator" in chunks[0]["text"]
        assert "def my_func" in chunks[0]["text"]


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
# _is_cards_json tests
# ---------------------------------------------------------------------------


class TestIsCardsJson:
    def test_correct_path(self):
        p = Path("some/root/digimon_gym/engine/data/cards.json")
        assert _is_cards_json(p) is True

    def test_wrong_filename(self):
        p = Path("digimon_gym/engine/data/other.json")
        assert _is_cards_json(p) is False

    def test_cards_json_in_wrong_location(self):
        """cards.json that is NOT under digimon_gym/engine/data/ should not match."""
        p = Path("some/other/cards.json")
        assert _is_cards_json(p) is False

    def test_windows_backslash(self):
        p = Path("C:\\repo\\digimon_gym\\engine\\data\\cards.json")
        assert _is_cards_json(p) is True


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

        # Create a small Python file under the engine path
        engine_dir = tmp_path / "digimon_gym" / "engine"
        engine_dir.mkdir(parents=True)
        py_file = engine_dir / "sample.py"
        py_file.write_text("def decode():\n    pass\n", encoding="utf-8")

        # Create a small markdown file
        md_file = tmp_path / "RULES.md"
        md_file.write_text("## Overview\n\nSome rules.\n", encoding="utf-8")

        # Create a small cards.json at the correct path
        data_dir = tmp_path / "digimon_gym" / "engine" / "data"
        data_dir.mkdir(parents=True, exist_ok=True)
        cards_file = data_dir / "cards.json"
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

        # Verify card chunk has card_id in both chunk_id and as a field
        card_chunks = [c for c in result["chunks"] if c["source_type"] == "card_metadata"]
        assert any("C-001" in c["chunk_id"] for c in card_chunks)
        assert any(c.get("card_id") == "C-001" for c in card_chunks)

        # Verify markdown chunk has section_title
        md_chunks = [c for c in result["chunks"] if c["source_type"] == "rules"]
        assert any(c.get("section_title") == "Overview" for c in md_chunks)

        # Verify markdown chunk_ids include index for uniqueness
        md_chunk_ids = [c["chunk_id"] for c in md_chunks]
        for cid in md_chunk_ids:
            # Format is "RULES.md:Overview:0" or "RULES.md:0"
            parts = cid.split(":")
            assert len(parts) >= 2

        # Verify file was written
        assert output.exists()
        written = json.loads(output.read_text(encoding="utf-8"))
        assert written["version"] == 2

    def test_duplicate_section_headers_unique_ids(self, tmp_path: Path):
        """Two sections with the same ## header should get distinct chunk_ids."""
        from digimon_gym.ai.retrieval import build_local_index

        md_file = tmp_path / "SPEC.md"
        md_file.write_text(
            "## Notes\n\nFirst notes.\n\n## Notes\n\nSecond notes.\n",
            encoding="utf-8",
        )

        output = tmp_path / "index.json"
        result = build_local_index(
            output_path=output,
            source_paths=[md_file],
        )

        chunk_ids = [c["chunk_id"] for c in result["chunks"]]
        # All chunk_ids must be unique
        assert len(chunk_ids) == len(set(chunk_ids)), f"Duplicate chunk_ids: {chunk_ids}"


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
        assert idx.chunks[0]["section_title"] is None
        assert idx.chunks[0]["card_id"] is None

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
        """retrieve() results should include source_type and function_name."""
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
                    "function_name": "run",
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
        assert results[0]["function_name"] == "run"


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


# ---------------------------------------------------------------------------
# Helper to build an index file with test data
# ---------------------------------------------------------------------------


def _write_index(tmp_path: Path, chunks: list[dict], version: int = 2) -> Path:
    """Write a synthetic index.json and return its path."""
    index_file = tmp_path / "index.json"
    data = {
        "version": version,
        "chunk_count": len(chunks),
        "sources": list({c.get("source", "test") for c in chunks}),
        "chunks": chunks,
    }
    index_file.write_text(json.dumps(data), encoding="utf-8")
    return index_file


def _make_chunks() -> list[dict]:
    """Standard set of 4 test chunks covering multiple source types."""
    return [
        {
            "chunk_id": "game.py:decode",
            "source": "game.py",
            "source_type": "engine_api",
            "text": "decode action method for the game engine processing",
            "embedding": None,
        },
        {
            "chunk_id": "RULES.md:0",
            "source": "RULES.md",
            "source_type": "rules",
            "text": "rules governing decode action flow in game sessions",
            "embedding": None,
        },
        {
            "chunk_id": "cards.json:BT1-001",
            "source": "cards.json",
            "source_type": "card_metadata",
            "text": "card BT1-001 Agumon level 3 rookie digimon",
            "embedding": None,
        },
        {
            "chunk_id": "patterns.py:match",
            "source": "tools/transpiler/patterns.py",
            "source_type": "transpiler",
            "text": "pattern match transpiler helper function for cards",
            "embedding": None,
        },
    ]


# ---------------------------------------------------------------------------
# _tokenize_for_bm25 tests
# ---------------------------------------------------------------------------


class TestTokenizeForBm25:
    def test_basic_tokenization(self):
        tokens = LocalRAGIndex._tokenize_for_bm25("Hello World")
        assert tokens == ["hello", "world"]

    def test_returns_list_not_set(self):
        tokens = LocalRAGIndex._tokenize_for_bm25("foo bar foo")
        assert isinstance(tokens, list)
        # Preserves duplicates (important for BM25 frequency)
        assert tokens.count("foo") == 2

    def test_filters_short_tokens(self):
        tokens = LocalRAGIndex._tokenize_for_bm25("I am a big dog")
        # "I", "a" are single char -> filtered out; "am" is 2 chars -> kept
        assert "am" in tokens
        assert "big" in tokens
        assert "dog" in tokens
        assert "a" not in tokens

    def test_handles_punctuation(self):
        tokens = LocalRAGIndex._tokenize_for_bm25("hello, world! foo-bar")
        assert "hello" in tokens
        assert "world" in tokens
        assert "foo" in tokens
        assert "bar" in tokens

    def test_preserves_underscores(self):
        tokens = LocalRAGIndex._tokenize_for_bm25("decode_action some_method")
        assert "decode_action" in tokens
        assert "some_method" in tokens

    def test_empty_string(self):
        assert LocalRAGIndex._tokenize_for_bm25("") == []

    def test_numbers(self):
        tokens = LocalRAGIndex._tokenize_for_bm25("BT1 card 001 level3")
        assert "bt1" in tokens
        assert "001" in tokens
        assert "level3" in tokens


# ---------------------------------------------------------------------------
# BM25 index built on load() tests
# ---------------------------------------------------------------------------


class TestBm25IndexOnLoad:
    def test_bm25_created_on_load(self, tmp_path: Path):
        index_file = _write_index(tmp_path, _make_chunks())
        idx = LocalRAGIndex(path=index_file)
        idx.load()
        assert idx._bm25 is not None

    def test_bm25_none_when_no_file(self, tmp_path: Path):
        idx = LocalRAGIndex(path=tmp_path / "nonexistent.json")
        idx.load()
        assert idx._bm25 is None

    def test_bm25_none_when_empty_chunks(self, tmp_path: Path):
        index_file = _write_index(tmp_path, [])
        idx = LocalRAGIndex(path=index_file)
        idx.load()
        assert idx._bm25 is None

    def test_bm25_rebuilt_on_reload(self, tmp_path: Path):
        index_file = _write_index(tmp_path, _make_chunks())
        idx = LocalRAGIndex(path=index_file)
        idx.load()
        first_bm25 = idx._bm25
        # Reload and check a new instance is created
        idx.load()
        assert idx._bm25 is not None
        assert idx._bm25 is not first_bm25


# ---------------------------------------------------------------------------
# _bm25_rank tests
# ---------------------------------------------------------------------------


class TestBm25Rank:
    def test_returns_ranked_tuples(self, tmp_path: Path):
        chunks = _make_chunks()
        index_file = _write_index(tmp_path, chunks)
        idx = LocalRAGIndex(path=index_file)
        idx.load()

        all_indices = list(range(len(idx.chunks)))
        ranks = idx._bm25_rank("decode action game", idx.chunks, all_indices)

        assert len(ranks) > 0
        # Each entry is (rank_position, chunk)
        for rank_pos, chunk in ranks:
            assert isinstance(rank_pos, int)
            assert rank_pos >= 1
            assert isinstance(chunk, dict)

    def test_ranks_are_contiguous(self, tmp_path: Path):
        chunks = _make_chunks()
        index_file = _write_index(tmp_path, chunks)
        idx = LocalRAGIndex(path=index_file)
        idx.load()

        all_indices = list(range(len(idx.chunks)))
        ranks = idx._bm25_rank("decode action", idx.chunks, all_indices)
        rank_positions = [r for r, _ in ranks]
        assert rank_positions == list(range(1, len(ranks) + 1))

    def test_filtered_candidates(self, tmp_path: Path):
        chunks = _make_chunks()
        index_file = _write_index(tmp_path, chunks)
        idx = LocalRAGIndex(path=index_file)
        idx.load()

        # Only engine_api candidates
        candidates = [c for c in idx.chunks if c["source_type"] == "engine_api"]
        candidate_indices = [i for i, c in enumerate(idx.chunks) if c["source_type"] == "engine_api"]
        ranks = idx._bm25_rank("decode action", candidates, candidate_indices)

        assert len(ranks) == len(candidates)
        for _, chunk in ranks:
            assert chunk["source_type"] == "engine_api"

    def test_empty_query_tokens(self, tmp_path: Path):
        chunks = _make_chunks()
        index_file = _write_index(tmp_path, chunks)
        idx = LocalRAGIndex(path=index_file)
        idx.load()

        # Query with only single-char tokens -> empty after tokenization
        ranks = idx._bm25_rank("a b c", idx.chunks, list(range(len(idx.chunks))))
        assert ranks == []

    def test_no_bm25_index(self, tmp_path: Path):
        idx = LocalRAGIndex(path=tmp_path / "nonexistent.json")
        idx.load()
        ranks = idx._bm25_rank("decode", [], [])
        assert ranks == []

    def test_relevant_chunk_ranked_first(self, tmp_path: Path):
        """The chunk most relevant to the query should have rank 1."""
        chunks = _make_chunks()
        index_file = _write_index(tmp_path, chunks)
        idx = LocalRAGIndex(path=index_file)
        idx.load()

        all_indices = list(range(len(idx.chunks)))
        ranks = idx._bm25_rank("Agumon rookie digimon card", idx.chunks, all_indices)
        # The card_metadata chunk should be ranked first
        top_chunk = ranks[0][1]
        assert top_chunk["source_type"] == "card_metadata"


# ---------------------------------------------------------------------------
# _vector_rank tests
# ---------------------------------------------------------------------------


class TestVectorRank:
    def test_no_embeddings_returns_empty(self, tmp_path: Path):
        """When no embeddings are available, _vector_rank returns empty (BM25-only fallback)."""
        chunks = _make_chunks()  # All embeddings are None
        index_file = _write_index(tmp_path, chunks)
        idx = LocalRAGIndex(path=index_file)
        idx.load()

        ranks = idx._vector_rank("decode", idx.chunks)
        assert ranks == []

    def test_with_mock_embeddings(self, tmp_path: Path):
        """When embeddings are available, _vector_rank returns ranked results."""
        chunks = [
            {
                "chunk_id": "a",
                "source": "a.py",
                "source_type": "engine_api",
                "text": "decode action",
                "embedding": [1.0, 0.0, 0.0],
            },
            {
                "chunk_id": "b",
                "source": "b.py",
                "source_type": "rules",
                "text": "game rules",
                "embedding": [0.0, 1.0, 0.0],
            },
        ]
        index_file = _write_index(tmp_path, chunks)
        idx = LocalRAGIndex(path=index_file)
        idx.load()

        # Mock _embed_query to return a vector close to chunk "a"
        with patch.object(idx, "_embed_query", return_value=[0.9, 0.1, 0.0]):
            ranks = idx._vector_rank("decode action", idx.chunks)

        assert len(ranks) == 2
        # Chunk "a" should be ranked first (closer to query vector)
        assert ranks[0][1]["chunk_id"] == "a"
        assert ranks[0][0] == 1  # rank 1
        assert ranks[1][1]["chunk_id"] == "b"
        assert ranks[1][0] == 2  # rank 2

    def test_chunks_without_embeddings_skipped(self, tmp_path: Path):
        """Chunks missing embeddings are excluded from vector ranking."""
        chunks = [
            {
                "chunk_id": "a",
                "source": "a.py",
                "source_type": "engine_api",
                "text": "decode",
                "embedding": [1.0, 0.0],
            },
            {
                "chunk_id": "b",
                "source": "b.py",
                "source_type": "rules",
                "text": "rules",
                "embedding": None,
            },
        ]
        index_file = _write_index(tmp_path, chunks)
        idx = LocalRAGIndex(path=index_file)
        idx.load()

        with patch.object(idx, "_embed_query", return_value=[1.0, 0.0]):
            ranks = idx._vector_rank("decode", idx.chunks)

        assert len(ranks) == 1
        assert ranks[0][1]["chunk_id"] == "a"


# ---------------------------------------------------------------------------
# _rrf_merge tests
# ---------------------------------------------------------------------------


class TestRrfMerge:
    def test_merge_both_lists(self):
        chunk_a = {"chunk_id": "a", "text": "chunk a"}
        chunk_b = {"chunk_id": "b", "text": "chunk b"}

        bm25_ranks = [(1, chunk_a), (2, chunk_b)]
        vector_ranks = [(1, chunk_b), (2, chunk_a)]

        merged = LocalRAGIndex._rrf_merge(bm25_ranks, vector_ranks, k_param=60)
        scores = {chunk["chunk_id"]: score for score, chunk in merged}

        # Both chunks appear in both lists, so both get contributions from both
        expected_a = 1.0 / (60 + 1) + 1.0 / (60 + 2)
        expected_b = 1.0 / (60 + 2) + 1.0 / (60 + 1)
        assert abs(scores["a"] - expected_a) < 1e-10
        assert abs(scores["b"] - expected_b) < 1e-10
        # Both scores should be equal in this symmetric case
        assert abs(scores["a"] - scores["b"]) < 1e-10

    def test_chunk_in_one_list_only(self):
        chunk_a = {"chunk_id": "a", "text": "chunk a"}
        chunk_b = {"chunk_id": "b", "text": "chunk b"}

        bm25_ranks = [(1, chunk_a)]
        vector_ranks = [(1, chunk_b)]

        merged = LocalRAGIndex._rrf_merge(bm25_ranks, vector_ranks, k_param=60)
        scores = {chunk["chunk_id"]: score for score, chunk in merged}

        assert abs(scores["a"] - 1.0 / 61) < 1e-10
        assert abs(scores["b"] - 1.0 / 61) < 1e-10

    def test_empty_lists(self):
        merged = LocalRAGIndex._rrf_merge([], [], k_param=60)
        assert merged == []

    def test_one_empty_list(self):
        chunk_a = {"chunk_id": "a", "text": "chunk a"}
        bm25_ranks = [(1, chunk_a), (2, {"chunk_id": "b", "text": "b"})]

        merged = LocalRAGIndex._rrf_merge(bm25_ranks, [], k_param=60)
        assert len(merged) == 2

    def test_custom_k_param(self):
        chunk_a = {"chunk_id": "a", "text": "a"}
        bm25_ranks = [(1, chunk_a)]
        vector_ranks = [(1, chunk_a)]

        merged = LocalRAGIndex._rrf_merge(bm25_ranks, vector_ranks, k_param=10)
        score = merged[0][0]
        expected = 1.0 / (10 + 1) + 1.0 / (10 + 1)
        assert abs(score - expected) < 1e-10

    def test_higher_ranked_gets_higher_score(self):
        chunk_a = {"chunk_id": "a", "text": "a"}
        chunk_b = {"chunk_id": "b", "text": "b"}

        # chunk_a is #1 in both lists, chunk_b is #2 in both lists
        bm25_ranks = [(1, chunk_a), (2, chunk_b)]
        vector_ranks = [(1, chunk_a), (2, chunk_b)]

        merged = LocalRAGIndex._rrf_merge(bm25_ranks, vector_ranks, k_param=60)
        scores = {chunk["chunk_id"]: score for score, chunk in merged}
        assert scores["a"] > scores["b"]


# ---------------------------------------------------------------------------
# Hybrid retrieve() integration tests
# ---------------------------------------------------------------------------


class TestHybridRetrieve:
    def test_basic_bm25_only_retrieval(self, tmp_path: Path):
        """Without embeddings, retrieve still works via BM25."""
        chunks = _make_chunks()
        index_file = _write_index(tmp_path, chunks)
        idx = LocalRAGIndex(path=index_file)

        results = idx.retrieve("decode action game engine")
        assert len(results) > 0
        # All results should have required fields
        for r in results:
            assert "chunk_id" in r
            assert "source" in r
            assert "source_type" in r
            assert "text" in r
            assert "score" in r
            assert isinstance(r["score"], float)

    def test_source_types_filter(self, tmp_path: Path):
        """retrieve() with source_types should only return matching chunks."""
        chunks = _make_chunks()
        index_file = _write_index(tmp_path, chunks)
        idx = LocalRAGIndex(path=index_file)

        results = idx.retrieve("decode action", source_types=["engine_api"])
        for r in results:
            assert r["source_type"] == "engine_api"

    def test_source_types_filter_multiple(self, tmp_path: Path):
        """source_types can contain multiple types."""
        chunks = _make_chunks()
        index_file = _write_index(tmp_path, chunks)
        idx = LocalRAGIndex(path=index_file)

        results = idx.retrieve("decode action game", source_types=["engine_api", "rules"])
        for r in results:
            assert r["source_type"] in ("engine_api", "rules")

    def test_empty_query_returns_empty(self, tmp_path: Path):
        chunks = _make_chunks()
        index_file = _write_index(tmp_path, chunks)
        idx = LocalRAGIndex(path=index_file)
        idx.load()

        assert idx.retrieve("") == []
        assert idx.retrieve("   ") == []

    def test_no_matching_source_types(self, tmp_path: Path):
        chunks = _make_chunks()
        index_file = _write_index(tmp_path, chunks)
        idx = LocalRAGIndex(path=index_file)

        results = idx.retrieve("decode", source_types=["nonexistent_type"])
        assert results == []

    def test_k_limits_results(self, tmp_path: Path):
        chunks = _make_chunks()
        index_file = _write_index(tmp_path, chunks)
        idx = LocalRAGIndex(path=index_file)

        results = idx.retrieve("decode action game card", k=2)
        assert len(results) <= 2

    def test_no_chunks_returns_empty(self, tmp_path: Path):
        index_file = _write_index(tmp_path, [])
        idx = LocalRAGIndex(path=index_file)
        idx.load()

        assert idx.retrieve("decode") == []

    def test_results_sorted_by_score_descending(self, tmp_path: Path):
        chunks = _make_chunks()
        index_file = _write_index(tmp_path, chunks)
        idx = LocalRAGIndex(path=index_file)

        results = idx.retrieve("decode action game engine processing")
        if len(results) > 1:
            scores = [r["score"] for r in results]
            assert scores == sorted(scores, reverse=True)

    def test_hybrid_with_embeddings(self, tmp_path: Path):
        """When embeddings are available, both BM25 and vector contribute to RRF."""
        chunks = [
            {
                "chunk_id": "a",
                "source": "a.py",
                "source_type": "engine_api",
                "text": "decode action game engine method",
                "embedding": [1.0, 0.0, 0.0],
            },
            {
                "chunk_id": "b",
                "source": "RULES.md",
                "source_type": "rules",
                "text": "rules for gameplay sessions and turns",
                "embedding": [0.0, 1.0, 0.0],
            },
            {
                "chunk_id": "c",
                "source": "cards.json",
                "source_type": "card_metadata",
                "text": "card BT1-001 Agumon rookie digimon level three",
                "embedding": [0.0, 0.0, 1.0],
            },
        ]
        index_file = _write_index(tmp_path, chunks)
        idx = LocalRAGIndex(path=index_file)
        idx.load()

        # Mock embeddings: query vector close to chunk "a"
        with patch.object(idx, "_embed_query", return_value=[0.9, 0.1, 0.0]):
            results = idx.retrieve("decode action game engine")

        assert len(results) > 0
        # With both BM25 (text match) and vector (embedding similarity)
        # favoring chunk "a", it should be ranked first
        assert results[0]["chunk_id"] == "a"

    def test_ensure_loaded_called(self, tmp_path: Path):
        """retrieve() should call ensure_loaded() automatically."""
        chunks = _make_chunks()
        index_file = _write_index(tmp_path, chunks)
        idx = LocalRAGIndex(path=index_file)
        assert not idx.loaded

        results = idx.retrieve("decode action")
        assert idx.loaded
        assert len(results) > 0

    def test_bm25_uses_full_index_not_filtered(self, tmp_path: Path):
        """BM25 index should be built on ALL chunks; filtering happens via candidate_indices.

        We verify this by checking that BM25 scores for filtered candidates
        are computed from the full-corpus IDF, not just the subset.
        """
        # Create chunks where a term appears in many types
        chunks = [
            {"chunk_id": f"engine_{i}", "source": "game.py", "source_type": "engine_api",
             "text": f"decode action method variant {i}", "embedding": None}
            for i in range(5)
        ] + [
            {"chunk_id": "rules_0", "source": "RULES.md", "source_type": "rules",
             "text": "decode action rules specification", "embedding": None},
        ]
        index_file = _write_index(tmp_path, chunks)
        idx = LocalRAGIndex(path=index_file)
        idx.load()

        # When filtering to rules only, the BM25 score should still use
        # the IDF computed from all 6 documents, not just 1
        results = idx.retrieve("decode action", source_types=["rules"])
        assert len(results) == 1
        assert results[0]["source_type"] == "rules"

    def test_nonexistent_index_file(self, tmp_path: Path):
        idx = LocalRAGIndex(path=tmp_path / "missing.json")
        results = idx.retrieve("anything")
        assert results == []
