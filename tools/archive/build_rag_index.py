#!/usr/bin/env python3
"""Build local-doc-first retrieval index for AI tasks."""

from __future__ import annotations

import argparse
from pathlib import Path
import sys

PROJECT_ROOT = Path(__file__).resolve().parents[1]
if str(PROJECT_ROOT) not in sys.path:
    sys.path.insert(0, str(PROJECT_ROOT))

from digimon_gym.ai.retrieval import DEFAULT_INDEX_PATH, DEFAULT_SOURCES, build_local_index


def main() -> None:
    parser = argparse.ArgumentParser(description="Build local RAG index")
    parser.add_argument(
        "--output",
        type=Path,
        default=DEFAULT_INDEX_PATH,
        help="Output index path (default: data/rag/index.json)",
    )
    parser.add_argument(
        "--source",
        action="append",
        default=[],
        help="Additional source file/directory paths",
    )
    parser.add_argument("--chunk-size", type=int, default=1200)
    parser.add_argument("--overlap", type=int, default=200)
    args = parser.parse_args()

    sources = list(DEFAULT_SOURCES)
    for src in args.source:
        sources.append(Path(src))

    payload = build_local_index(
        output_path=args.output,
        source_paths=sources,
        chunk_size=args.chunk_size,
        overlap=args.overlap,
    )
    print(f"Built RAG index at {args.output}")
    print(f"Chunks: {payload['chunk_count']}")
    print(f"Sources: {len(payload['sources'])}")


if __name__ == "__main__":
    main()
