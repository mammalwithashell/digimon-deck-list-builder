"""Backfill meta_tier / meta_archetype for existing decks.

Reads every row in the `decks` table, runs the classifier on its card list,
and persists the tier. Safe to re-run — idempotent, re-classifies on every
invocation so that re-ingesting `deck_library.json` re-tags old decks.

Usage:
    python tools/backfill_deck_meta_tier.py                 # hosted DB (DATABASE_URL)
    python tools/backfill_deck_meta_tier.py --dry-run       # report only, no writes
    python tools/backfill_deck_meta_tier.py --limit 100     # process first N decks
"""
from __future__ import annotations

import argparse
import asyncio
import json
import logging
import sys
from pathlib import Path

# Make the repo root importable when run as a script.
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from sqlalchemy import select  # noqa: E402
from sqlalchemy.ext.asyncio import AsyncSession  # noqa: E402

from server.classifier.deck_tagger import get_library, tag_deck  # noqa: E402
from server.db.database import async_session  # noqa: E402
from server.db.models import Deck  # noqa: E402

logger = logging.getLogger("backfill_deck_meta_tier")


async def backfill(*, dry_run: bool, limit: int | None) -> None:
    lib = get_library()
    if lib is None:
        logger.error("Classifier library unavailable; aborting backfill.")
        return

    async with async_session() as session:  # type: AsyncSession
        query = select(Deck)
        if limit is not None:
            query = query.limit(limit)
        result = await session.execute(query)
        decks = result.scalars().all()

        counts: dict[str | None, int] = {"meta": 0, "rogue": 0, "jank": 0, None: 0}
        for deck in decks:
            main = json.loads(deck.main_deck) if deck.main_deck else []
            egg = json.loads(deck.egg_deck) if deck.egg_deck else []
            archetype, tier = tag_deck(main + egg)
            counts[tier] = counts.get(tier, 0) + 1
            if dry_run:
                continue
            deck.meta_archetype = archetype
            deck.meta_tier = tier

        if not dry_run:
            await session.commit()

        for tier, n in counts.items():
            logger.info("  %-6s %d", tier or "untagged", n)
        logger.info("processed %d decks (dry_run=%s)", len(decks), dry_run)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument("--limit", type=int, default=None)
    parser.add_argument("-v", "--verbose", action="store_true")
    args = parser.parse_args()

    logging.basicConfig(
        level=logging.DEBUG if args.verbose else logging.INFO,
        format="%(asctime)s %(levelname)s %(name)s: %(message)s",
    )
    asyncio.run(backfill(dry_run=args.dry_run, limit=args.limit))


if __name__ == "__main__":
    main()
