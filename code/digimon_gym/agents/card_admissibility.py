"""Light per-card admissibility for the mechanics training gate (Plan 2 Task 2).

A card may appear in a training decklist when it is:

1. **registered** -- the Rust engine registers it and the card database the game
   loads knows it (`gauntlet.load_playable_card_ids`);
2. **tested** -- listed in `data/tested_cards.json`;
3. free of a **failed ledger verdict** -- its status in
   `qa/qa-reports/validated_cards_dsl.json` or `validated_cards.json` is not in
   `FAILED_VERDICT_STATUSES` -- the ledger gate `NOT_READY_LEDGER_STATUSES`
   (`PARTIAL`, `BLOCKED`, `AUDITED-DRIFT`, `AUDITED-MISSING-TESTS`) plus the
   explicit failures `FAILED`, `FAIL`, `QA-FAIL` (ruling R4-revised). This
   module owns those sets; the gauntlet imports them;
4. free of an **untriaged divergence** -- no exam row of either family
   (`clauses`, or `interactions`: `qa:` rulings and `probe:` risk probes) is
   blocking under `row_blocks`, the single home of that rule.

When several rules fail, `reason()` names the first in `REASONS` order and
`reasons()` lists them all.

Fingerprint staleness (ruling R1', 2026-10-21): a row whose `text_sha256` no
longer matches the printed text is `unmeasured` when it was `confirmed`, and a
`confirmed` / `unmeasured` / `unreachable` / `unavailable` row never blocks, so
staleness cannot change a decision. It never voids a `diverged` row: a recorded
divergence stays a finding until it is triaged `dcgo_quirk` or `stale` with a
citation. That is why `load()` accepts `official` (the printed text the
readiness report fingerprints against) for interface parity but does not read
it, and why `from_repo()` does not load it.

Training images ship no `qa/` (ruling R5), so this module is also the generator
and checker of the committed artifact `data/card_admissibility.json`, which
covers every registered card id; `CardAdmissibility.from_artifact()` reads it
back at run time:

    PYTHONPATH=code python -m digimon_gym.agents.card_admissibility [--check] [--out PATH]

`--check` rebuilds from the current inputs and fails when the committed file
differs in anything but `generated_at`. Run `--check` (and any write of the
committed artifact) without input overrides: the input flags are refused unless
`--out` points elsewhere.

Importing the module, `load()` and every per-card decision need no engine
binding; only `load_registry_ids()` / `from_repo()` (the registry) import the
gauntlet and so `digimon_engine`.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import sys
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Iterable, Mapping, Optional

from data_paths import CARD_ADMISSIBILITY, QA_REPORTS_DIR, TESTED_CARDS

ARTIFACT_VERSION = 1

# Priority order: the first failing reason is `reason()`.
UNREGISTERED = "unregistered"
UNTESTED = "untested"
FAILED_VERDICT = "failed_verdict"
DIVERGED_NO_QUIRK = "diverged_no_quirk"
REASONS = (UNREGISTERED, UNTESTED, FAILED_VERDICT, DIVERGED_NO_QUIRK)

# The ledger gate: a PARTIAL / BLOCKED card has an unimplemented printed clause,
# an AUDITED-DRIFT / AUDITED-MISSING-TESTS card failed its audit. Owned here;
# `gauntlet._NOT_READY_DSL_STATUSES` is this set.
NOT_READY_LEDGER_STATUSES = frozenset({"PARTIAL", "BLOCKED", "AUDITED-DRIFT", "AUDITED-MISSING-TESTS"})
# Explicit failures: `FAILED`, and the hand-written ledger's `FAIL` / `QA-FAIL`
# (ruling R4-revised, 2026-10-21).
EXPLICIT_FAILED_STATUSES = frozenset({"FAILED", "FAIL", "QA-FAIL"})
# A ledger status in this set, in EITHER ledger, is a failed verdict.
FAILED_VERDICT_STATUSES = NOT_READY_LEDGER_STATUSES | EXPLICIT_FAILED_STATUSES

# A diverged row with this reason compared every step it could and found no
# divergence; the scenario just ran out of DCGO steps.
TRUNCATED_PREFIX = "TRUNCATED, no divergence found"
# Triage classes that clear a diverged row -- each only with a citation.
CLEARING_TRIAGES = frozenset({"dcgo_quirk", "stale"})

EXAM_FAMILIES = ("clauses", "interactions")
_REGENERATE = "PYTHONPATH=code python -m digimon_gym.agents.card_admissibility"


def _has_text(value: Any) -> bool:
    return bool(str(value or "").strip())


def row_blocks(row: Mapping[str, Any]) -> bool:
    """Whether one exam verdict row (clause or interaction) blocks its cards.

    Only a `diverged` row can block. In precedence order:

    1. triage `ours_wrong` blocks -- an explicit human finding outranks the
       automated truncation reason below;
    2. a reason starting `TRUNCATED, no divergence found` clears it;
    3. triage `dcgo_quirk` or `stale` with a non-empty citation clears it;
    4. anything else (no triage, an uncited or unknown triage) blocks.

    The text fingerprint plays no part (ruling R1').
    """
    if row.get("verdict") != "diverged":
        return False
    if row.get("triage") == "ours_wrong":
        return True
    if str(row.get("reason") or "").startswith(TRUNCATED_PREFIX):
        return False
    if row.get("triage") in CLEARING_TRIAGES and _has_text(row.get("citation")):
        return False
    return True


def _row_card_ids(
    family: str, row_id: str, row: Mapping[str, Any], file_card_id: Optional[str]
) -> list[str]:
    """The cards an exam row speaks for: a clause row its `card_id`, an
    interaction row every id in `card_ids` (a shared ruling counts for every
    card that prints it). Falls back to the verdict file's card, then to the
    clause id prefix (`<card>#<label>#<n>`)."""
    if family == "interactions":
        ids = [str(c) for c in row.get("card_ids") or [] if c]
    else:
        ids = [str(row["card_id"])] if row.get("card_id") else []
    if not ids and file_card_id:
        ids = [file_card_id]
    if not ids and family == "clauses" and "#" in row_id:
        ids = [row_id.split("#", 1)[0]]
    return ids


def _failed_ids(ledger: Mapping[str, Any]) -> set[str]:
    return {
        str(cid)
        for cid, entry in (ledger.get("cards") or {}).items()
        if str((entry or {}).get("status") or "").strip().upper() in FAILED_VERDICT_STATUSES
    }


@dataclass(frozen=True)
class DataPaths:
    """Inputs and output of the generator (`from_repo`, `main`)."""

    tested: Path = TESTED_CARDS
    dsl_ledger: Path = QA_REPORTS_DIR / "validated_cards_dsl.json"
    rust_ledger: Path = QA_REPORTS_DIR / "validated_cards.json"
    exam_verdicts: Path = QA_REPORTS_DIR / "exam-verdicts"
    out: Path = CARD_ADMISSIBILITY


def load_registry_ids() -> set[str]:
    """The registered (playable) card ids -- the loader the gauntlet uses for
    its `unregistered` rejections. Imported lazily: it needs the engine binding,
    and the gauntlet imports this module."""
    from digimon_gym.agents.gauntlet import load_playable_card_ids

    return set(load_playable_card_ids())


def _read_json(path: Path) -> Any:
    return json.loads(Path(path).read_text(encoding="utf-8"))


def _read_exam_verdicts(directory: Path) -> list[dict]:
    directory = Path(directory)
    if not directory.is_dir():
        raise FileNotFoundError(f"exam-verdict store {directory} is missing")
    out = []
    for path in sorted(directory.glob("*.json")):
        data = _read_json(path)
        out.append({**data, "card_id": path.stem})
    return out


def file_sha256(path: Path) -> str:
    """sha256 of a file's bytes (CRLF -> LF so autocrlf checkouts agree)."""
    return hashlib.sha256(Path(path).read_bytes().replace(b"\r\n", b"\n")).hexdigest()


def store_sha256(directory: Path) -> str:
    """sha256 over every `*.json` in the exam-verdict store, by name then bytes."""
    h = hashlib.sha256()
    for path in sorted(Path(directory).glob("*.json")):
        h.update(path.name.encode("utf-8") + b"\0")
        h.update(path.read_bytes().replace(b"\r\n", b"\n") + b"\0")
    return h.hexdigest()


def source_hashes(paths: DataPaths) -> dict[str, str]:
    return {
        "tested_cards_sha256": file_sha256(paths.tested),
        "validated_cards_dsl_sha256": file_sha256(paths.dsl_ledger),
        "validated_cards_sha256": file_sha256(paths.rust_ledger),
        "exam_verdicts_sha256": store_sha256(paths.exam_verdicts),
    }


@dataclass(frozen=True)
class CardAdmissibility:
    """Per-card admit/reject decisions. Build with `load`, `from_repo`, or
    `from_artifact`; ask with `admissible`, `reason`, `reasons`."""

    registered: frozenset[str]
    tested: frozenset[str]
    failed_verdict: frozenset[str]
    diverged_no_quirk: frozenset[str]

    @classmethod
    def load(
        cls,
        *,
        registry_ids: Iterable[str],
        tested_ids: Iterable[str],
        verdicts_dsl: Mapping[str, Any],
        verdicts_rust: Mapping[str, Any],
        official: Optional[Mapping[str, Any]] = None,
        exam_verdicts: Iterable[Mapping[str, Any]] = (),
    ) -> "CardAdmissibility":
        """Decide from in-memory inputs; needs no engine binding. `official` is
        accepted for interface parity and never read: staleness cannot change a
        decision (module doc)."""
        del official
        diverged: set[str] = set()
        for exam in exam_verdicts:
            file_card = exam.get("card_id")
            for family in EXAM_FAMILIES:
                for row_id, row in (exam.get(family) or {}).items():
                    if row_blocks(row):
                        diverged.update(_row_card_ids(family, str(row_id), row, file_card))
        return cls(
            registered=frozenset(str(c) for c in registry_ids),
            tested=frozenset(str(c) for c in tested_ids),
            failed_verdict=frozenset(_failed_ids(verdicts_dsl) | _failed_ids(verdicts_rust)),
            diverged_no_quirk=frozenset(diverged),
        )

    @classmethod
    def from_repo(cls, paths: DataPaths = DataPaths()) -> "CardAdmissibility":
        """Decide from the repo's sources (needs `qa/` and the engine binding)."""
        return cls.load(
            registry_ids=load_registry_ids(),
            tested_ids=_read_json(paths.tested)["card_ids"],
            verdicts_dsl=_read_json(paths.dsl_ledger),
            verdicts_rust=_read_json(paths.rust_ledger),
            exam_verdicts=_read_exam_verdicts(paths.exam_verdicts),
        )

    @classmethod
    def from_artifact_dict(cls, data: Mapping[str, Any]) -> "CardAdmissibility":
        """Rebuild from the committed artifact's payload. Every entry must carry
        `reasons` (known reasons, priority order) agreeing with its `admissible`
        and `reason`; anything else raises `ValueError` naming the card. An id
        the artifact does not list is unregistered."""
        if data.get("version") != ARTIFACT_VERSION:
            raise ValueError(
                f"card admissibility artifact version {data.get('version')!r} != {ARTIFACT_VERSION}; "
                f"regenerate it with `{_REGENERATE}`"
            )
        reasons: dict[str, list[str]] = {}
        for cid, entry in data["cards"].items():
            found = entry.get("reasons")
            ok = (
                isinstance(found, list)
                and all(r in REASONS for r in found)
                and len(set(found)) == len(found)
                and found == sorted(found, key=REASONS.index)
                and entry.get("admissible") is (not found)
                and entry.get("reason") == (found[0] if found else None)
            )
            if not ok:
                raise ValueError(
                    f"card admissibility artifact entry {cid} is inconsistent: {entry!r}; "
                    f"regenerate it with `{_REGENERATE}`"
                )
            reasons[str(cid)] = list(found)

        def having(reason: str) -> frozenset[str]:
            return frozenset(c for c, rs in reasons.items() if reason in rs)

        return cls(
            registered=frozenset(c for c, rs in reasons.items() if UNREGISTERED not in rs),
            tested=frozenset(c for c, rs in reasons.items() if UNTESTED not in rs),
            failed_verdict=having(FAILED_VERDICT),
            diverged_no_quirk=having(DIVERGED_NO_QUIRK),
        )

    @classmethod
    def from_artifact(cls, path: Path = CARD_ADMISSIBILITY) -> "CardAdmissibility":
        """Read `data/card_admissibility.json` -- the run-time path; no `qa/`."""
        return cls.from_artifact_dict(_read_json(path))

    def reasons(self, card_id: str) -> list[str]:
        """Every failing rule for `card_id`, in priority order (empty = admissible)."""
        out = []
        if card_id not in self.registered:
            out.append(UNREGISTERED)
        if card_id not in self.tested:
            out.append(UNTESTED)
        if card_id in self.failed_verdict:
            out.append(FAILED_VERDICT)
        if card_id in self.diverged_no_quirk:
            out.append(DIVERGED_NO_QUIRK)
        return out

    def reason(self, card_id: str) -> Optional[str]:
        """The highest-priority failing rule, or None when admissible."""
        found = self.reasons(card_id)
        return found[0] if found else None

    def admissible(self, card_id: str) -> bool:
        return not self.reasons(card_id)

    def build_artifact(self, *, sources: Mapping[str, str], generated_at: str) -> dict:
        """The `data/card_admissibility.json` payload over every registered id."""
        cards = {}
        counts = {"registered": len(self.registered), "admissible": 0}
        counts.update({r: 0 for r in REASONS})
        for cid in sorted(self.registered):
            found = self.reasons(cid)
            cards[cid] = {"admissible": not found, "reason": found[0] if found else None, "reasons": found}
            counts[found[0] if found else "admissible"] += 1
        return {
            "version": ARTIFACT_VERSION,
            "generated_at": generated_at,
            "sources": dict(sources),
            "counts": counts,
            "cards": cards,
        }


def serialise(payload: Mapping[str, Any]) -> str:
    return json.dumps(payload, sort_keys=True, indent=2, ensure_ascii=False) + "\n"


def _comparable(payload: Mapping[str, Any]) -> dict:
    return {k: v for k, v in payload.items() if k != "generated_at"}


def summary_line(payload: Mapping[str, Any]) -> str:
    c = payload["counts"]
    return (
        f"{c['registered']} registered cards: {c['admissible']} admissible / "
        + " / ".join(f"{c[r]} {r}" for r in REASONS)
    )


def main(argv: Optional[list[str]] = None) -> int:
    defaults = DataPaths()
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--out", type=Path, default=defaults.out)
    ap.add_argument("--tested", type=Path, default=defaults.tested)
    ap.add_argument("--dsl-ledger", type=Path, default=defaults.dsl_ledger)
    ap.add_argument("--rust-ledger", type=Path, default=defaults.rust_ledger)
    ap.add_argument("--exam-verdicts", type=Path, default=defaults.exam_verdicts)
    ap.add_argument(
        "--check",
        action="store_true",
        help="exit 1 if the committed artifact differs (ignoring generated_at) from a fresh build",
    )
    args = ap.parse_args(argv)
    overridden = [
        flag
        for flag, value, default in (
            ("--tested", args.tested, defaults.tested),
            ("--dsl-ledger", args.dsl_ledger, defaults.dsl_ledger),
            ("--rust-ledger", args.rust_ledger, defaults.rust_ledger),
            ("--exam-verdicts", args.exam_verdicts, defaults.exam_verdicts),
        )
        if Path(value).resolve() != Path(default).resolve()
    ]
    if overridden and Path(args.out).resolve() == Path(defaults.out).resolve():
        print(
            f"error: {', '.join(overridden)} would build the committed artifact "
            f"{defaults.out} from non-repo inputs; pass --out elsewhere",
            file=sys.stderr,
        )
        return 2
    paths = DataPaths(
        tested=args.tested,
        dsl_ledger=args.dsl_ledger,
        rust_ledger=args.rust_ledger,
        exam_verdicts=args.exam_verdicts,
        out=args.out,
    )

    try:
        adm = CardAdmissibility.from_repo(paths)
        sources = source_hashes(paths)
    except FileNotFoundError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1
    now = datetime.now(timezone.utc).isoformat(timespec="seconds")
    payload = adm.build_artifact(sources=sources, generated_at=now)

    existing: Optional[dict] = None
    corrupt = False
    if args.out.exists():
        try:
            existing = json.loads(args.out.read_text(encoding="utf-8"))
        except ValueError:
            corrupt = True
    unchanged = existing is not None and _comparable(existing) == _comparable(payload)

    if args.check:
        if not args.out.exists():
            print(f"error: {args.out} is missing; run `{_REGENERATE}` to create it", file=sys.stderr)
            return 1
        if corrupt:
            print(f"error: {args.out} is not valid JSON; run `{_REGENERATE}`", file=sys.stderr)
            return 1
        if not unchanged:
            recorded = (existing or {}).get("sources")
            why = (
                "recorded source hashes differ from the current inputs"
                if recorded != payload["sources"]
                else "contents differ from a fresh build"
            )
            print(f"error: {args.out} is stale ({why}); run `{_REGENERATE}`", file=sys.stderr)
            return 1
        print(f"card admissibility check: ok. {summary_line(payload)}")
        return 0

    if unchanged:
        payload["generated_at"] = existing.get("generated_at", now)
    args.out.parent.mkdir(parents=True, exist_ok=True)
    with open(args.out, "w", encoding="utf-8", newline="\n") as f:
        f.write(serialise(payload))
    print(f"wrote {args.out}")
    print(summary_line(payload))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
