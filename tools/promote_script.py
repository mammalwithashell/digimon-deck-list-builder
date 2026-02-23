#!/usr/bin/env python3
"""Promote a generated card script into frozen production lane."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

PROJECT_ROOT = Path(__file__).resolve().parents[1]
if str(PROJECT_ROOT) not in sys.path:
    sys.path.insert(0, str(PROJECT_ROOT))

from digimon_gym.engine.data.script_promotion import ScriptPromotionError, promote_script_from_generated


def main() -> int:
    parser = argparse.ArgumentParser(description="Promote generated script to frozen")
    parser.add_argument("--card-id", required=True)
    parser.add_argument("--set-id", required=True)
    parser.add_argument("--module-name", required=True)
    parser.add_argument("--expected-generated-hash", required=True)
    args = parser.parse_args()

    try:
        result = promote_script_from_generated(
            card_id=args.card_id,
            set_id=args.set_id,
            module_name=args.module_name,
            expected_generated_hash=args.expected_generated_hash,
        )
    except ScriptPromotionError as exc:
        print(f"Promotion failed: {exc}")
        return 1

    print(json.dumps(result, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
