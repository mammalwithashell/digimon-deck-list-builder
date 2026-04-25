"""Local-doc-first retrieval index and query helpers."""

from __future__ import annotations

import ast
import json
import logging
import math
import os
import re
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable

from rank_bm25 import BM25Okapi

from digimon_gym.env import load_project_env

load_project_env()


PROJECT_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_INDEX_PATH = PROJECT_ROOT / "data" / "rag" / "index.json"
DEFAULT_SOURCES = [
    # Rules & Specs
    PROJECT_ROOT / "RULES_CONTEXT.md",
    PROJECT_ROOT / "ACTION_SPEC.md",
    PROJECT_ROOT / "TENSOR_SPEC.md",
    PROJECT_ROOT / "Digimon TCG resources",
    # Card Metadata
    PROJECT_ROOT / "data" / "cards.json",
    # Engine API
    PROJECT_ROOT / "digimon_gym" / "engine" / "game.py",
    PROJECT_ROOT / "digimon_gym" / "engine" / "interfaces" / "card_effect.py",
    PROJECT_ROOT / "digimon_gym" / "engine" / "core" / "permanent.py",
    PROJECT_ROOT / "digimon_gym" / "engine" / "core" / "card_source.py",
    PROJECT_ROOT / "digimon_gym" / "engine" / "core" / "card_script.py",
    # Transpiler
    PROJECT_ROOT / "tools" / "transpiler" / "patterns.py",
    PROJECT_ROOT / "tools" / "transpiler" / "generators.py",
    PROJECT_ROOT / "tools" / "transpiler" / "extractors.py",
]
SOURCE_EXTENSIONS = {".md", ".txt", ".json", ".py"}
PDF_EXTENSION = ".pdf"
SOURCE_EXTENSIONS.add(PDF_EXTENSION)

logger = logging.getLogger(__name__)


@dataclass
class Chunk:
    chunk_id: str
    source: str
    text: str
    embedding: list[float] | None
    source_type: str = "rules"
    function_name: str | None = None
    class_name: str | None = None
    section_title: str | None = None
    card_id: str | None = None

    def as_dict(self) -> dict:
        d = {
            "chunk_id": self.chunk_id,
            "source": self.source,
            "source_type": self.source_type,
            "text": self.text,
            "embedding": self.embedding,
        }
        if self.function_name is not None:
            d["function_name"] = self.function_name
        if self.class_name is not None:
            d["class_name"] = self.class_name
        if self.section_title is not None:
            d["section_title"] = self.section_title
        if self.card_id is not None:
            d["card_id"] = self.card_id
        return d


def discover_source_files(paths: Iterable[Path]) -> list[Path]:
    files: list[Path] = []
    for path in paths:
        if not path.exists():
            continue
        if path.is_file():
            if path.suffix.lower() in SOURCE_EXTENSIONS:
                files.append(path)
            continue
        for child in path.rglob("*"):
            if child.is_file() and child.suffix.lower() in SOURCE_EXTENSIONS:
                files.append(child)
    return sorted(set(files))


def chunk_text(text: str, chunk_size: int = 1200, overlap: int = 200) -> list[str]:
    clean = re.sub(r"\s+", " ", text).strip()
    if not clean:
        return []
    chunks = []
    step = max(1, chunk_size - overlap)
    for start in range(0, len(clean), step):
        part = clean[start : start + chunk_size]
        if part:
            chunks.append(part)
        if start + chunk_size >= len(clean):
            break
    return chunks


def _infer_python_source_type(file_path: str) -> str:
    """Infer source_type from a file path string."""
    normalized = file_path.replace("\\", "/")
    if "tools/transpiler" in normalized:
        return "transpiler"
    if "digimon_gym/engine" in normalized:
        return "engine_api"
    return "rules"


def _get_source_segment(source_text: str, node: ast.AST) -> str:
    """Extract source text for an AST node using line numbers.

    When the node has decorators, start from the first decorator line
    so that ``@property``, ``@staticmethod``, etc. are included.
    """
    lines = source_text.splitlines(keepends=True)
    # Use decorator start line when present
    if hasattr(node, "decorator_list") and node.decorator_list:
        start = node.decorator_list[0].lineno - 1  # 0-indexed
    else:
        start = node.lineno - 1
    end = node.end_lineno if node.end_lineno is not None else start + 1
    return "".join(lines[start:end])


def chunk_python_by_ast(
    source_text: str, file_path: str
) -> list[dict]:
    """Parse Python source with AST and extract one chunk per top-level
    function or class method. Falls back to text chunking on parse failure.

    Returns list of dicts with: text, source_type, function_name, class_name.
    """
    source_type = _infer_python_source_type(file_path)

    try:
        tree = ast.parse(source_text)
    except SyntaxError:
        logger.warning("AST parse failed for %s; falling back to text chunking", file_path)
        text_chunks = chunk_text(source_text)
        return [
            {
                "text": t,
                "source_type": source_type,
                "function_name": None,
                "class_name": None,
            }
            for t in text_chunks
        ]

    chunks: list[dict] = []

    for node in ast.iter_child_nodes(tree):
        if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
            text = _get_source_segment(source_text, node)
            chunks.append(
                {
                    "text": text,
                    "source_type": source_type,
                    "function_name": node.name,
                    "class_name": None,
                }
            )
        elif isinstance(node, ast.ClassDef):
            for item in ast.iter_child_nodes(node):
                if isinstance(item, (ast.FunctionDef, ast.AsyncFunctionDef)):
                    text = _get_source_segment(source_text, item)
                    chunks.append(
                        {
                            "text": text,
                            "source_type": source_type,
                            "function_name": item.name,
                            "class_name": node.name,
                        }
                    )

    # If parsing succeeded but produced no chunks (e.g. module-level code only),
    # fall back to text chunking.
    if not chunks:
        text_chunks = chunk_text(source_text)
        return [
            {
                "text": t,
                "source_type": source_type,
                "function_name": None,
                "class_name": None,
            }
            for t in text_chunks
        ]

    return chunks


def chunk_markdown_by_section(
    source_text: str, file_path: str
) -> list[dict]:
    """Split markdown on ## headers, keeping each section as one chunk
    (up to 2000 chars). Sections exceeding 2000 chars are sub-chunked
    with standard text chunking.

    Returns list of dicts with: text, source_type="rules", section_title.
    """
    # Split on lines that start with ##
    sections: list[tuple[str, str]] = []  # (title, body)
    current_title = ""
    current_lines: list[str] = []

    for line in source_text.splitlines(keepends=True):
        if line.lstrip().startswith("## "):
            # Save previous section
            if current_lines or current_title:
                sections.append((current_title, "".join(current_lines)))
            current_title = line.strip().lstrip("#").strip()
            current_lines = [line]
        else:
            current_lines.append(line)

    # Don't forget the last section
    if current_lines or current_title:
        sections.append((current_title, "".join(current_lines)))

    chunks: list[dict] = []
    for title, body in sections:
        if len(body) <= 2000:
            if body.strip():
                chunks.append(
                    {
                        "text": body,
                        "source_type": "rules",
                        "section_title": title,
                    }
                )
        else:
            # Sub-chunk with standard text chunking
            sub_chunks = chunk_text(body, chunk_size=1200, overlap=200)
            for sub in sub_chunks:
                chunks.append(
                    {
                        "text": sub,
                        "source_type": "rules",
                        "section_title": title,
                    }
                )

    return chunks


def chunk_cards_json(
    source_text: str, file_path: str
) -> list[dict]:
    """Parse cards.json and create one chunk per card entry.

    Returns list of dicts with: text (JSON string of card data),
    source_type="card_metadata", card_id.
    """
    try:
        data = json.loads(source_text)
    except json.JSONDecodeError:
        logger.warning("JSON parse failed for %s; falling back to text chunking", file_path)
        text_chunks = chunk_text(source_text)
        return [
            {
                "text": t,
                "source_type": "card_metadata",
                "card_id": None,
            }
            for t in text_chunks
        ]

    chunks: list[dict] = []
    if isinstance(data, dict):
        for card_id, card_data in data.items():
            text = json.dumps(card_data, indent=2)
            chunks.append(
                {
                    "text": text,
                    "source_type": "card_metadata",
                    "card_id": card_id,
                }
            )
    elif isinstance(data, list):
        for idx, card_data in enumerate(data):
            card_id = card_data.get("card_id", str(idx)) if isinstance(card_data, dict) else str(idx)
            text = json.dumps(card_data, indent=2)
            chunks.append(
                {
                    "text": text,
                    "source_type": "card_metadata",
                    "card_id": card_id,
                }
            )

    return chunks


def _read_text_file(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except UnicodeDecodeError:
        return path.read_text(encoding="latin-1")


def _read_pdf_file(path: Path) -> str:
    try:
        from pypdf import PdfReader
    except Exception:
        logger.warning("pypdf is not installed; skipping PDF source: %s", path)
        return ""

    try:
        reader = PdfReader(str(path))
        pages = [page.extract_text() or "" for page in reader.pages]
        return "\n".join(pages).strip()
    except Exception as exc:
        logger.warning("Failed reading PDF source %s: %s", path, exc)
        return ""


def _read_source_file(path: Path) -> str:
    if path.suffix.lower() == PDF_EXTENSION:
        return _read_pdf_file(path)
    return _read_text_file(path)


def _display_source_path(path: Path) -> str:
    try:
        return str(path.relative_to(PROJECT_ROOT)).replace("\\", "/")
    except ValueError:
        return str(path).replace("\\", "/")


def _embed_texts(texts: list[str], model: str = "text-embedding-3-small") -> list[list[float] | None]:
    api_key = os.getenv("OPENAI_API_KEY", "")
    if not api_key or not texts:
        return [None] * len(texts)
    try:
        from openai import OpenAI
    except Exception:
        return [None] * len(texts)

    client = OpenAI(api_key=api_key)
    vectors: list[list[float] | None] = []
    batch_size = 50
    for start in range(0, len(texts), batch_size):
        batch = texts[start : start + batch_size]
        try:
            resp = client.embeddings.create(model=model, input=batch)
            vectors.extend([list(item.embedding) for item in resp.data])
        except Exception:
            vectors.extend([None] * len(batch))
    return vectors


def _is_engine_python(path: Path) -> bool:
    """Check if a Python file is under digimon_gym/engine/."""
    normalized = str(path).replace("\\", "/")
    return "digimon_gym/engine" in normalized


def _is_transpiler_python(path: Path) -> bool:
    """Check if a Python file is under tools/transpiler/."""
    normalized = str(path).replace("\\", "/")
    return "tools/transpiler" in normalized


def _is_cards_json(path: Path) -> bool:
    """Check if a file is specifically data/cards.json."""
    normalized = str(path).replace("\\", "/")
    return normalized.endswith("data/cards.json")


def build_local_index(
    *,
    output_path: Path = DEFAULT_INDEX_PATH,
    source_paths: Iterable[Path] = DEFAULT_SOURCES,
    chunk_size: int = 1200,
    overlap: int = 200,
) -> dict:
    source_files = discover_source_files(source_paths)
    chunk_records: list[Chunk] = []

    texts_for_embeddings: list[str] = []
    for source_file in source_files:
        text = _read_source_file(source_file)
        display_path = _display_source_path(source_file)
        suffix = source_file.suffix.lower()

        if suffix == ".py" and (_is_engine_python(source_file) or _is_transpiler_python(source_file)):
            # AST-aware chunking for engine and transpiler Python files
            raw_chunks = chunk_python_by_ast(text, display_path)
            for cd in raw_chunks:
                fn = cd.get("function_name")
                cn = cd.get("class_name")
                if cn and fn:
                    chunk_id = f"{source_file.name}:{cn}.{fn}"
                elif fn:
                    chunk_id = f"{source_file.name}:{fn}"
                else:
                    chunk_id = f"{source_file.name}:{len(chunk_records)}"
                chunk_records.append(
                    Chunk(
                        chunk_id=chunk_id,
                        source=display_path,
                        text=cd["text"],
                        embedding=None,
                        source_type=cd["source_type"],
                        function_name=fn,
                        class_name=cn,
                    )
                )
                texts_for_embeddings.append(cd["text"])

        elif suffix == ".md":
            # Section-aware chunking for markdown files
            raw_chunks = chunk_markdown_by_section(text, display_path)
            for idx, cd in enumerate(raw_chunks):
                section = cd.get("section_title", "")
                if section:
                    chunk_id = f"{source_file.name}:{section}:{idx}"
                else:
                    chunk_id = f"{source_file.name}:{idx}"
                chunk_records.append(
                    Chunk(
                        chunk_id=chunk_id,
                        source=display_path,
                        text=cd["text"],
                        embedding=None,
                        source_type=cd["source_type"],
                        section_title=section or None,
                    )
                )
                texts_for_embeddings.append(cd["text"])

        elif _is_cards_json(source_file):
            # Per-card chunking for cards.json
            raw_chunks = chunk_cards_json(text, display_path)
            for cd in raw_chunks:
                card_id = cd.get("card_id", "unknown")
                chunk_id = f"{source_file.name}:{card_id}"
                chunk_records.append(
                    Chunk(
                        chunk_id=chunk_id,
                        source=display_path,
                        text=cd["text"],
                        embedding=None,
                        source_type=cd["source_type"],
                        card_id=card_id,
                    )
                )
                texts_for_embeddings.append(cd["text"])

        else:
            # Default: flat text chunking with source_type="rules"
            text_chunks = chunk_text(text, chunk_size=chunk_size, overlap=overlap)
            for idx, part in enumerate(text_chunks):
                chunk_id = f"{source_file.name}:{idx}"
                chunk_records.append(
                    Chunk(
                        chunk_id=chunk_id,
                        source=display_path,
                        text=part,
                        embedding=None,
                        source_type="rules",
                    )
                )
                texts_for_embeddings.append(part)

    embeddings = _embed_texts(texts_for_embeddings)
    for chunk, emb in zip(chunk_records, embeddings):
        chunk.embedding = emb

    output_path.parent.mkdir(parents=True, exist_ok=True)
    payload = {
        "version": 2,
        "chunk_count": len(chunk_records),
        "sources": [_display_source_path(p) for p in source_files],
        "chunks": [c.as_dict() for c in chunk_records],
    }
    output_path.write_text(json.dumps(payload, indent=2), encoding="utf-8")
    return payload


def _cosine(a: list[float], b: list[float]) -> float:
    if len(a) != len(b):
        return 0.0
    dot = sum(x * y for x, y in zip(a, b))
    na = math.sqrt(sum(x * x for x in a))
    nb = math.sqrt(sum(y * y for y in b))
    if na == 0 or nb == 0:
        return 0.0
    return dot / (na * nb)


class LocalRAGIndex:
    def __init__(self, path: Path = DEFAULT_INDEX_PATH) -> None:
        self.path = path
        self.loaded = False
        self.chunks: list[dict] = []
        self._has_embeddings = False
        self._bm25: BM25Okapi | None = None

    @staticmethod
    def _tokenize_for_bm25(text: str) -> list[str]:
        """Tokenize text for BM25: lowercase, split on whitespace/punctuation,
        keep tokens of 2+ chars. Returns a list (preserving frequency)."""
        return re.findall(r"[a-z0-9_]{2,}", text.lower())

    def load(self) -> None:
        if not self.path.exists():
            self.loaded = False
            self.chunks = []
            self._has_embeddings = False
            self._bm25 = None
            return
        data = json.loads(self.path.read_text(encoding="utf-8"))
        self.chunks = data.get("chunks", [])
        # Backward compatibility: v1 chunks lack source_type; default to "rules"
        for chunk in self.chunks:
            if "source_type" not in chunk:
                chunk["source_type"] = "rules"
            # Ensure optional fields exist for uniform access
            chunk.setdefault("function_name", None)
            chunk.setdefault("class_name", None)
            chunk.setdefault("section_title", None)
            chunk.setdefault("card_id", None)
        self._has_embeddings = any(chunk.get("embedding") for chunk in self.chunks)

        # Build BM25 index over all chunks
        if self.chunks:
            tokenized = [self._tokenize_for_bm25(c.get("text", "")) for c in self.chunks]
            self._bm25 = BM25Okapi(tokenized)
        else:
            self._bm25 = None

        self.loaded = True

    def ensure_loaded(self) -> None:
        if not self.loaded:
            self.load()

    def _embed_query(self, text: str) -> list[float] | None:
        api_key = os.getenv("OPENAI_API_KEY", "")
        if not api_key or not self._has_embeddings:
            return None
        try:
            from openai import OpenAI
        except Exception:
            return None
        client = OpenAI(api_key=api_key)
        try:
            resp = client.embeddings.create(model="text-embedding-3-small", input=text)
            return list(resp.data[0].embedding)
        except Exception:
            return None

    def _bm25_rank(
        self,
        query: str,
        candidates: list[dict],
        candidate_indices: list[int],
    ) -> list[tuple[int, dict]]:
        """Return (rank_position, chunk) tuples sorted by BM25 score descending.

        The BM25 index is built over ALL chunks. When source_types filtering is
        active we only look at the scores for *candidate_indices*.
        """
        if self._bm25 is None or not candidates:
            return []

        query_tokens = self._tokenize_for_bm25(query)
        if not query_tokens:
            return []

        all_scores = self._bm25.get_scores(query_tokens)

        # Pair each candidate with its BM25 score from the full index
        scored = [(all_scores[idx], chunk) for idx, chunk in zip(candidate_indices, candidates)]
        scored.sort(key=lambda x: x[0], reverse=True)

        # Convert to (rank_position, chunk) -- 1-based ranks
        return [(rank + 1, chunk) for rank, (_score, chunk) in enumerate(scored)]

    def _vector_rank(
        self,
        query: str,
        candidates: list[dict],
    ) -> list[tuple[int, dict]]:
        """Return (rank_position, chunk) tuples sorted by cosine similarity descending.

        If no query embedding is available, returns an empty list (BM25-only fallback).
        """
        query_embedding = self._embed_query(query)
        if not query_embedding:
            return []

        scored = []
        for chunk in candidates:
            emb = chunk.get("embedding")
            if not emb:
                continue
            sim = _cosine(query_embedding, emb)
            scored.append((sim, chunk))

        scored.sort(key=lambda x: x[0], reverse=True)

        return [(rank + 1, chunk) for rank, (_sim, chunk) in enumerate(scored)]

    @staticmethod
    def _rrf_merge(
        bm25_ranks: list[tuple[int, dict]],
        vector_ranks: list[tuple[int, dict]],
        k_param: int = 60,
    ) -> list[tuple[float, dict]]:
        """Reciprocal Rank Fusion of two ranked lists.

        For each unique chunk: rrf_score = 1/(k_param + bm25_rank) + 1/(k_param + vector_rank).
        If a chunk only appears in one list, only that term contributes.

        Returns list of (rrf_score, chunk) tuples.
        """
        # Use chunk_id as the merge key; map to (rrf_score_so_far, chunk_ref)
        merged: dict[str, tuple[float, dict]] = {}

        for rank, chunk in bm25_ranks:
            cid = chunk["chunk_id"]
            score = 1.0 / (k_param + rank)
            if cid in merged:
                prev_score, _ = merged[cid]
                merged[cid] = (prev_score + score, chunk)
            else:
                merged[cid] = (score, chunk)

        for rank, chunk in vector_ranks:
            cid = chunk["chunk_id"]
            score = 1.0 / (k_param + rank)
            if cid in merged:
                prev_score, _ = merged[cid]
                merged[cid] = (prev_score + score, chunk)
            else:
                merged[cid] = (score, chunk)

        return [(score, chunk) for score, chunk in merged.values()]

    def retrieve(
        self,
        query: str,
        k: int = 6,
        source_types: list[str] | None = None,
    ) -> list[dict]:
        self.ensure_loaded()
        if not self.chunks or not query.strip():
            return []

        # Filter chunks by source_type if specified
        if source_types:
            type_set = set(source_types)
            candidates = [c for c in self.chunks if c.get("source_type") in type_set]
            candidate_indices = [i for i, c in enumerate(self.chunks) if c.get("source_type") in type_set]
        else:
            candidates = self.chunks
            candidate_indices = list(range(len(self.chunks)))

        if not candidates:
            return []

        # Stage 1a: BM25 scores
        bm25_ranks = self._bm25_rank(query, candidates, candidate_indices)

        # Stage 1b: Vector scores
        vector_ranks = self._vector_rank(query, candidates)

        # Reciprocal Rank Fusion
        rrf_scores = self._rrf_merge(bm25_ranks, vector_ranks, k_param=60)

        # Stage 2: Take top k
        top_k = sorted(rrf_scores, key=lambda x: x[0], reverse=True)[:k]

        return [
            {
                "chunk_id": chunk.get("chunk_id"),
                "source": chunk.get("source"),
                "source_type": chunk.get("source_type", "rules"),
                "function_name": chunk.get("function_name"),
                "text": chunk.get("text"),
                "score": float(score),
            }
            for score, chunk in top_k
        ]
