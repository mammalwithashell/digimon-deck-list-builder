"""Global mechanics-readiness artifact for the training gate.

Reads the rules-conformance ledger ``qa/rules/cr_obligations.json`` and writes
``data/mechanics_readiness.json``: the counts of obligations in the *mechanics
set* (those carrying at least one mechanics-set tag) by status, the list of
rules whose open gap blocks training, and ``ready``.

    ready = counts.gap_blocking == 0 and counts.proven > 0

``proven > 0`` keeps an empty or all-not-testable mechanics set from reading as
ready. The artifact records the sha256 of the ledger file it was built from;
``--check`` rebuilds from the current ledger and fails if the committed file
differs (including a stale recorded sha).

    python -m tools.rules.mechanics_readiness [--check]

Stdlib only.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
DEFAULT_OBLIGATIONS = REPO_ROOT / "qa/rules/cr_obligations.json"
DEFAULT_OUTPUT = REPO_ROOT / "data/mechanics_readiness.json"

# Verbatim from spec §5b.
MECHANICS_SET = (
    "effects-and-ordering",
    "keywords",
    "attack-and-timing",
    "security",
    "battle",
    "blocking",
    "rule-checks",
    "turn-flow",
    "digivolution-and-costs",
    "play-and-costs",
    "card-use-and-costs",
    "link",
)


def ledger_sha256(path: Path) -> str:
    """sha256 of the ledger file bytes (CRLF -> LF so autocrlf checkouts agree)."""
    data = Path(path).read_bytes().replace(b"\r\n", b"\n")
    return hashlib.sha256(data).hexdigest()


def build(ledger: dict, obligations_sha256: str) -> dict:
    mech = set(MECHANICS_SET)
    counts = {"proven": 0, "not_testable": 0, "gap_blocking": 0, "gap_nonblocking": 0, "total": 0}
    blocking: list[dict] = []
    for rule_id, ob in ledger["obligations"].items():
        mechanisms = list(ob.get("mechanisms", []))
        if not mech.intersection(mechanisms):
            continue
        counts["total"] += 1
        status = ob["status"]
        if status == "proven":
            counts["proven"] += 1
        elif status == "not-testable":
            counts["not_testable"] += 1
        elif ob.get("blocks_training"):
            counts["gap_blocking"] += 1
            blocking.append(
                {"rule": rule_id, "gap_id": ob.get("gap_id"), "mechanisms": mechanisms}
            )
        else:
            counts["gap_nonblocking"] += 1
    blocking.sort(key=lambda b: b["rule"])
    return {
        "version": 1,
        "source": {
            "obligations_sha256": obligations_sha256,
            "rules_version": ledger.get("source", {}).get("rules_version", "4.3"),
        },
        "mechanics_set": list(MECHANICS_SET),
        "counts": counts,
        "blocking": blocking,
        "ready": counts["gap_blocking"] == 0 and counts["proven"] > 0,
    }


def serialise(payload: dict) -> str:
    return json.dumps(payload, sort_keys=True, indent=2, ensure_ascii=False) + "\n"


def summary_line(payload: dict) -> str:
    c = payload["counts"]
    return (
        f"{c['total']} obligations in the mechanics set: "
        f"{c['proven']} proven / {c['not_testable']} not-testable / "
        f"{c['gap_blocking']} gap(blocking) / {c['gap_nonblocking']} gap(non-blocking) "
        f"=> ready: {str(payload['ready']).lower()}"
    )


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--obligations", type=Path, default=DEFAULT_OBLIGATIONS)
    ap.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    ap.add_argument(
        "--check",
        action="store_true",
        help="exit 1 if the committed artifact differs from a fresh build of the ledger",
    )
    args = ap.parse_args(argv)

    ledger = json.loads(args.obligations.read_text(encoding="utf-8"))
    payload = build(ledger, ledger_sha256(args.obligations))
    fresh = serialise(payload)

    if args.check:
        if not args.output.exists():
            print(f"error: {args.output} is missing; run without --check to create it", file=sys.stderr)
            return 1
        existing = args.output.read_bytes().decode("utf-8").replace("\r\n", "\n")
        if existing != fresh:
            try:
                recorded = json.loads(existing).get("source", {}).get("obligations_sha256")
            except ValueError:
                recorded = None
            why = (
                "recorded ledger sha256 differs from the current ledger"
                if recorded != payload["source"]["obligations_sha256"]
                else "contents differ from a fresh build"
            )
            print(
                f"error: {args.output} is stale ({why}); run "
                "`python -m tools.rules.mechanics_readiness` to refresh",
                file=sys.stderr,
            )
            return 1
        print(f"mechanics readiness check: ok. {summary_line(payload)}")
        return 0

    args.output.parent.mkdir(parents=True, exist_ok=True)
    with open(args.output, "w", encoding="utf-8", newline="\n") as f:
        f.write(fresh)
    print(f"wrote {args.output}")
    print(summary_line(payload))
    for b in payload["blocking"]:
        print(f"  BLOCKING {b['rule']} ({b['gap_id']}): {', '.join(b['mechanisms'])}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
