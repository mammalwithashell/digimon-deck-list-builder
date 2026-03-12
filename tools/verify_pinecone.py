"""Verify Pinecone index health and spot-check queries.

Usage:
    python tools/verify_pinecone.py
"""

from __future__ import annotations

import logging
import os
import sys

logging.basicConfig(level=logging.INFO, format="%(levelname)s: %(message)s")
logger = logging.getLogger(__name__)

INDEX_NAME = "digimon-engine"
EXPECTED_NAMESPACES = ["engine-api", "card-scripts", "card-metadata", "rules-docs"]


def get_client():
    api_key = os.environ.get("PINECONE_API_KEY")
    if not api_key:
        logger.error("PINECONE_API_KEY not set")
        sys.exit(1)
    from pinecone import Pinecone
    return Pinecone(api_key=api_key)


def check_namespace_stats(pc):
    """Print vector counts per namespace."""
    index = pc.Index(INDEX_NAME)
    stats = index.describe_index_stats()
    logger.info("Index '%s' stats:", INDEX_NAME)
    logger.info("  Total vectors: %s", stats.get("total_vector_count", "?"))
    ns_map = stats.get("namespaces", {})
    all_ok = True
    for ns in EXPECTED_NAMESPACES:
        count = ns_map.get(ns, {}).get("vector_count", 0)
        status = "OK" if count > 0 else "EMPTY"
        if count == 0:
            all_ok = False
        logger.info("  %-16s %5d vectors  [%s]", ns, count, status)
    return all_ok


def spot_check_search(pc, namespace: str, query: str, expected_substring: str):
    """Search a namespace and check that results contain expected content."""
    index = pc.Index(INDEX_NAME)
    try:
        results = index.search(
            namespace=namespace,
            query={"top_k": 3, "inputs": {"text": query}},
        )
        hits = results.get("result", {}).get("hits", [])
        if not hits:
            logger.warning("  FAIL: No results for query '%s' in '%s'", query, namespace)
            return False

        # Check if any result contains the expected substring
        found = False
        for hit in hits:
            fields = hit.get("fields", {})
            text = fields.get("text", "")
            # Also check metadata fields
            card_id = fields.get("card_id", "")
            if expected_substring.lower() in text.lower() or expected_substring.lower() in card_id.lower():
                found = True
                break

        if found:
            logger.info("  PASS: '%s' in '%s' — found '%s'", query, namespace, expected_substring)
        else:
            logger.warning("  FAIL: '%s' in '%s' — did not find '%s' in top 3",
                           query, namespace, expected_substring)
        return found
    except Exception as e:
        logger.error("  ERROR searching '%s': %s", namespace, e)
        return False


def main():
    pc = get_client()
    ok = True

    # 1. Check namespace stats
    logger.info("=" * 60)
    logger.info("Step 1: Namespace stats")
    logger.info("=" * 60)
    if not check_namespace_stats(pc):
        ok = False

    # 2. Spot-check queries
    logger.info("")
    logger.info("=" * 60)
    logger.info("Step 2: Spot-check queries")
    logger.info("=" * 60)

    checks = [
        ("engine-api", "action mask legal actions", "action_mask"),
        ("engine-api", "Blocker keyword check", "blocker"),
        ("card-scripts", "Rush grant keyword effect", "rush"),
        ("card-metadata", "DemiMeramon Digitama Red Flame", "DemiMeramon"),
        ("card-metadata", "Blocker Digimon", "Blocker"),
        ("rules-docs", "security attack", "security"),
    ]

    for namespace, query, expected in checks:
        if not spot_check_search(pc, namespace, query, expected):
            ok = False

    # 3. Summary
    logger.info("")
    logger.info("=" * 60)
    if ok:
        logger.info("All checks PASSED")
    else:
        logger.warning("Some checks FAILED — review output above")
    logger.info("=" * 60)

    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
