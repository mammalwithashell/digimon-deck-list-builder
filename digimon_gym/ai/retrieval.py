"""Local-doc-first retrieval index and query helpers."""

from __future__ import annotations

import json
import logging
import math
import os
import re
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable, List

from digimon_gym.env import load_project_env

load_project_env()


PROJECT_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_INDEX_PATH = PROJECT_ROOT / "data" / "rag" / "index.json"
DEFAULT_SOURCES = [
    PROJECT_ROOT / "RULES_CONTEXT.md",
    PROJECT_ROOT / "ACTION_SPEC.md",
    PROJECT_ROOT / "TENSOR_SPEC.md",
    PROJECT_ROOT / "Digimon TCG resources",
    PROJECT_ROOT / "digimon_gym" / "engine" / "data" / "cards.json",
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

    def as_dict(self) -> dict:
        return {
            "chunk_id": self.chunk_id,
            "source": self.source,
            "text": self.text,
            "embedding": self.embedding,
        }


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
        chunks = chunk_text(text, chunk_size=chunk_size, overlap=overlap)
        for idx, part in enumerate(chunks):
            chunk_id = f"{source_file.name}:{idx}"
            chunk_records.append(
                Chunk(
                    chunk_id=chunk_id,
                    source=_display_source_path(source_file),
                    text=part,
                    embedding=None,
                )
            )
            texts_for_embeddings.append(part)

    embeddings = _embed_texts(texts_for_embeddings)
    for chunk, emb in zip(chunk_records, embeddings):
        chunk.embedding = emb

    output_path.parent.mkdir(parents=True, exist_ok=True)
    payload = {
        "version": 1,
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


def _tokenize(text: str) -> set[str]:
    return set(re.findall(r"[a-z0-9_]{2,}", text.lower()))


class LocalRAGIndex:
    def __init__(self, path: Path = DEFAULT_INDEX_PATH) -> None:
        self.path = path
        self.loaded = False
        self.chunks: list[dict] = []
        self._has_embeddings = False

    def load(self) -> None:
        if not self.path.exists():
            self.loaded = False
            self.chunks = []
            self._has_embeddings = False
            return
        data = json.loads(self.path.read_text(encoding="utf-8"))
        self.chunks = data.get("chunks", [])
        self._has_embeddings = any(chunk.get("embedding") for chunk in self.chunks)
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

    def retrieve(self, query: str, k: int = 6) -> list[dict]:
        self.ensure_loaded()
        if not self.chunks or not query.strip():
            return []

        query_embedding = self._embed_query(query)
        if query_embedding:
            scored = []
            for chunk in self.chunks:
                emb = chunk.get("embedding")
                if not emb:
                    continue
                score = _cosine(query_embedding, emb)
                scored.append((score, chunk))
            scored.sort(key=lambda row: row[0], reverse=True)
            return [
                {
                    "chunk_id": row[1].get("chunk_id"),
                    "source": row[1].get("source"),
                    "text": row[1].get("text"),
                    "score": float(row[0]),
                }
                for row in scored[:k]
            ]

        query_tokens = _tokenize(query)
        scored = []
        for chunk in self.chunks:
            tokens = _tokenize(chunk.get("text", ""))
            if not tokens:
                continue
            overlap = len(query_tokens.intersection(tokens))
            if overlap == 0:
                continue
            score = overlap / max(1, len(query_tokens))
            scored.append((score, chunk))
        scored.sort(key=lambda row: row[0], reverse=True)
        return [
            {
                "chunk_id": row[1].get("chunk_id"),
                "source": row[1].get("source"),
                "text": row[1].get("text"),
                "score": float(row[0]),
            }
            for row in scored[:k]
        ]
