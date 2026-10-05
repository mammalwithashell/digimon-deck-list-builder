"""Resolve card-loop inputs into one work set, and freeze it as a run plan.

Design D3 (openspec/changes/add-card-authoring-loop/design.md):

| Input                         | Pool                    | Core                          | Ranking                    |
|-------------------------------|-------------------------|-------------------------------|----------------------------|
| `--cards a,b` / `--cards @f`  | as given                | all                           | given order                |
| `--decklists f...`            | union of list cards     | in >= core_fraction of lists  | greedy decklist completion |
| `--archetype NAME`            | deck-library lists      | as decklists                  | as decklists               |
| `--set PREFIX`                | the set's card ids      | set cards in any known list;  | completion over the known  |
|                               |                         | all if none                   | lists, else card id        |

Forms combine. `--cards` and `--set` define the pool explicitly; `--decklists`
and `--archetype` contribute lists. When lists are given they define core and
ranking (core restricted to the pool), so `--set BT27 --decklists ...` takes the
pool from the set and core/ranking from the lists. "Known lists" for a bare
`--set` are every `data/deck_library.json` list that contains a set card.

Decklist files go through the engine's own parser (`digimon_engine.parse_deck`,
the PyO3 binding over `deck_tools::parse_deck`), so every format the desktop
app accepts works and there is no loop-specific parser.

`cli_plan` freezes the resolved set, the per-card DCGO script presence and the
preflight results into `<runs_dir>/<run-id>/plan.json`.
"""
from __future__ import annotations

import argparse
import glob
import hashlib
import json
import re
import subprocess
import sys
from collections import Counter
from dataclasses import asdict, dataclass, field, replace
from datetime import datetime, timezone
from difflib import get_close_matches
from pathlib import Path
from typing import Callable, Iterable, Sequence

from data_paths import ARCHETYPE_ALIASES, CARDS_JSON, DATA_DIR, DECK_LIBRARY, REPO_ROOT
from tools.author_set.set_resolver import normalize_prefix, resolve_set
from tools.clause_coverage import archetype as archetype_mod

from .config import LoopConfig, load_config

PLAN_VERSION = 1
CARD_OFFICIAL_JSON = DATA_DIR / "card_official.json"
YAML_CARDS_DIR = REPO_ROOT / "code" / "digimon-engine" / "cards"

#: A printed card id: set token, hyphen, number (`BT7-056`, `P-001`, `ST1-03`).
CARD_ID_RE = re.compile(r"^[A-Z]+\d*-\d+$")
_RUN_ID_RE = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._-]*$")


# ---------------------------------------------------------------------------
# the work set
# ---------------------------------------------------------------------------


@dataclass
class WorkSet:
    """What a run is about. `ranking` is always a permutation of `pool`;
    `core` is a subset of `pool` in pool order; `decklists` are the lists that
    drove core and ranking (one id per copy), or None when none did."""

    pool: list[str]
    core: list[str]
    ranking: list[str]
    decklists: list[list[str]] | None = None
    sources: dict = field(default_factory=dict)

    def to_dict(self) -> dict:
        return asdict(self)

    @classmethod
    def from_dict(cls, data: dict) -> "WorkSet":
        return cls(
            pool=list(data["pool"]),
            core=list(data["core"]),
            ranking=list(data["ranking"]),
            decklists=[list(x) for x in data["decklists"]] if data.get("decklists") is not None else None,
            sources=dict(data.get("sources") or {}),
        )


def _dedupe(ids: Iterable[str]) -> list[str]:
    seen: set[str] = set()
    out = []
    for c in ids:
        if c not in seen:
            seen.add(c)
            out.append(c)
    return out


# ---------------------------------------------------------------------------
# --cards
# ---------------------------------------------------------------------------


def parse_cards_arg(value: str) -> list[str]:
    """`--cards`: comma-separated ids, or `@file` (ids split on commas /
    whitespace / newlines, `#` comments). Uppercased, de-duplicated, order kept.
    A malformed id is an error rather than a silently empty plan entry."""
    value = value.strip()
    if value.startswith("@"):
        text = Path(value[1:]).read_text(encoding="utf-8-sig")
        tokens: list[str] = []
        for line in text.splitlines():
            tokens += re.split(r"[,\s]+", line.split("#", 1)[0])
    else:
        tokens = value.split(",")
    ids, bad = [], []
    for token in tokens:
        token = token.strip().upper()
        if not token:
            continue
        (ids if CARD_ID_RE.match(token) else bad).append(token)
    if bad:
        raise ValueError(f"malformed card id(s): {', '.join(bad)} (expected e.g. BT7-056)")
    if not ids:
        raise ValueError(f"--cards {value!r} resolved to no card ids")
    return _dedupe(ids)


# ---------------------------------------------------------------------------
# --decklists
# ---------------------------------------------------------------------------


def _engine_parse_deck() -> Callable[[str], list[str]]:
    """The engine's deck parser (indirection so tests can stand in for it)."""
    try:
        from digimon_engine import parse_deck
    except ImportError as e:  # pragma: no cover - depends on the local build
        raise ValueError(
            "--decklists needs the digimon_engine PyO3 binding "
            "(cd code/digimon-engine-py && maturin develop)"
        ) from e
    return parse_deck


def expand_decklist_paths(paths: Sequence[str | Path]) -> list[Path]:
    """Expand glob patterns ourselves: PowerShell does not, and a literal
    `lists/*.txt` reaching `open()` would read as a missing file."""
    out: list[Path] = []
    for p in paths:
        s = str(p)
        if any(ch in s for ch in "*?[") and not Path(s).exists():
            matches = sorted(glob.glob(s))
            if not matches:
                raise FileNotFoundError(f"no decklist files match {s}")
            out += [Path(m) for m in matches]
        else:
            out.append(Path(s))
    return out


def load_decklists(paths: Sequence[str | Path]) -> list[list[str]]:
    """Parse each file with the engine's `parse_deck` (one id per copy)."""
    parse = _engine_parse_deck()
    lists = []
    for path in expand_decklist_paths(paths):
        raw = Path(path).read_text(encoding="utf-8-sig")
        try:
            cards = [str(c) for c in parse(raw)]
        except ValueError as e:
            raise ValueError(f"{path}: {e}") from e
        if not cards:
            raise ValueError(f"{path}: parsed to an empty decklist")
        lists.append(cards)
    return lists


# ---------------------------------------------------------------------------
# --archetype
# ---------------------------------------------------------------------------


def load_aliases(path: str | Path) -> dict:
    """`archetype_aliases.json`: `{canonical: [alias, ...]}` plus `_comment`."""
    data = json.loads(Path(path).read_text(encoding="utf-8"))
    return {k: v for k, v in data.items() if not k.startswith("_") and isinstance(v, list)}


def resolve_archetype_name(name: str, library: dict, aliases: dict) -> str:
    """Deck-library key for `name`, case-insensitively, through the alias map.

    An unknown name raises `LookupError` with near-misses over library names
    AND aliases, because a campaign aimed at a misspelled archetype would
    otherwise resolve to nothing and read as finished.
    """
    by_lower = {k.lower(): k for k in library}
    if name.lower() in by_lower:
        return by_lower[name.lower()]
    alias_to_canonical = {
        a.lower(): canonical
        for canonical, alts in aliases.items()
        if not canonical.startswith("_")
        for a in alts
        if isinstance(a, str)
    }
    canonical = alias_to_canonical.get(name.lower())
    if canonical is not None:
        if canonical.lower() in by_lower:
            return by_lower[canonical.lower()]
        raise LookupError(
            f"{name!r} is an alias of {canonical!r}, which has no lists in the deck library"
        )
    display: dict[str, str] = {k.lower(): k for k in library}
    for canonical, alts in aliases.items():
        for a in alts:
            if isinstance(a, str):
                display.setdefault(a.lower(), f"{a} (alias of {canonical})")
    close = get_close_matches(name.lower(), list(display), n=5, cutoff=0.6)
    hint = f" Did you mean: {', '.join(display[c] for c in close)}?" if close else ""
    raise LookupError(f"no archetype named {name!r} in the deck library or the alias map.{hint}")


def library_lists(library: dict) -> list[list[str]]:
    """Every decklist of every archetype in the deck library."""
    out: list[list[str]] = []
    for entry in library.values():
        out += archetype_mod.decklists(entry)
    return out


# ---------------------------------------------------------------------------
# ranking: greedy decklist completion
# ---------------------------------------------------------------------------


def decklist_completion_order(
    decklists: Iterable[Iterable[str]], outstanding: Iterable[str]
) -> list[tuple[str, int]]:
    """Greedy decklist completion, with the number of lists each pick completes.

    THE single implementation of this ranking. It is the algorithm of the
    oracle-readiness design §4.8 (`readiness --plan`, branch
    `worktree-dcgo-effect-translation`) and of this change's design D3;
    readiness should import it from here rather than re-derive it.

    `outstanding` are the cards that still need work; every other card is
    treated as already done. Repeatedly pick the outstanding card whose
    adjudication completes the most decklists that are otherwise complete
    (every other card of the list is done or already picked), tie-broken by
    the number of decklists containing the card, then by card id. Copies
    within a list count once. Outstanding cards in no list rank last, by id.
    Runs in O(|outstanding| * (lists touched)) rather than re-scanning every
    list per pick, so a whole-library ranking stays cheap.
    """
    outstanding = set(outstanding)
    remaining: list[set[str]] = []
    containing: dict[str, list[int]] = {c: [] for c in outstanding}
    for cards in decklists:
        rem = set(cards) & outstanding
        if not rem:
            continue  # complete already; it can never be unblocked by a pick
        idx = len(remaining)
        remaining.append(rem)
        for c in rem:
            containing[c].append(idx)
    freq = {c: len(containing[c]) for c in outstanding}

    # completes[c] = lists whose ONLY outstanding, unpicked card is c.
    completes: Counter[str] = Counter()
    for rem in remaining:
        if len(rem) == 1:
            completes[next(iter(rem))] += 1

    fallback = sorted(outstanding, key=lambda c: (-freq[c], c))
    fb = 0
    picked: set[str] = set()
    order: list[tuple[str, int]] = []
    while len(order) < len(outstanding):
        if completes:
            best = min(completes, key=lambda c: (-completes[c], -freq[c], c))
        else:
            while fallback[fb] in picked:
                fb += 1
            best = fallback[fb]
        order.append((best, completes.pop(best, 0)))
        picked.add(best)
        for idx in containing[best]:
            rem = remaining[idx]
            rem.discard(best)
            if len(rem) == 1:
                completes[next(iter(rem))] += 1
    return order


def rank_by_decklist_completion(
    decklists: Iterable[Iterable[str]], outstanding: Iterable[str]
) -> list[str]:
    """Card ids in greedy-decklist-completion order (see `decklist_completion_order`)."""
    return [c for c, _ in decklist_completion_order(decklists, outstanding)]


# ---------------------------------------------------------------------------
# resolution
# ---------------------------------------------------------------------------


def load_cards_json(path: str | Path = CARDS_JSON) -> dict:
    """`cards.json` as `{card_id: card}` (tolerates the legacy array form)."""
    data = json.loads(Path(path).read_text(encoding="utf-8"))
    if isinstance(data, list):
        return {c["card_id"]: c for c in data}
    return data


def resolve_work_set(
    *,
    cards: Sequence[str] | None = None,
    set_prefix: str | None = None,
    decklist_files: Sequence[str | Path] | None = None,
    archetype: str | None = None,
    core_fraction: float = archetype_mod.DEFAULT_CORE_FRACTION,
    cards_data: dict | None = None,
    library_path: str | Path = DECK_LIBRARY,
    aliases_path: str | Path = ARCHETYPE_ALIASES,
) -> WorkSet:
    """Resolve any combination of the four input forms (module docstring table).

    At plan time every pool card is treated as outstanding for the ranking;
    a driver that knows what is already adjudicated re-ranks with
    `rank_by_decklist_completion(work_set.decklists, outstanding)`.
    """
    cards = list(cards or [])
    if not (cards or set_prefix or decklist_files or archetype):
        raise ValueError("nothing to resolve: give --cards, --set, --decklists and/or --archetype")
    if not 0.0 <= core_fraction <= 1.0:
        raise ValueError("core_fraction must be within [0, 1]")

    sources: dict = {
        "cards": cards or None,
        "set": None,
        "decklist_files": None,
        "archetype": None,
        "core_fraction": core_fraction,
    }
    _library: list[dict] = []

    def library() -> dict:
        if not _library:
            _library.append(archetype_mod.load_archetypes(library_path))
        return _library[0]

    set_ids: list[str] = []
    if set_prefix:
        prefix = normalize_prefix(set_prefix)
        data = cards_data if cards_data is not None else load_cards_json()
        set_ids = resolve_set(prefix, data)
        if not set_ids:
            raise ValueError(
                f"no {prefix} cards in cards.json; ingest the set first: "
                f"python code/tools/ingest_cards.py --set {prefix}"
            )
        sources["set"] = prefix

    lists: list[list[str]] = []
    if decklist_files:
        files = expand_decklist_paths(decklist_files)
        lists += load_decklists(files)
        sources["decklist_files"] = [str(f) for f in files]
    if archetype:
        canonical = resolve_archetype_name(archetype, library(), load_aliases(aliases_path))
        arch_lists = archetype_mod.decklists(library()[canonical])
        if not arch_lists:
            raise ValueError(f"archetype {canonical!r} has no decklists in the deck library")
        lists += arch_lists
        sources["archetype"] = canonical
        sources["archetype_query"] = archetype

    explicit = _dedupe(cards + set_ids)

    if lists:
        pool = explicit or sorted(archetype_mod.list_frequency(lists))
        info = archetype_mod.core_of_lists(lists, core_fraction)
        core_cards = set(info["cards"])
        core = [c for c in pool if c in core_cards]
        ranking = rank_by_decklist_completion(lists, pool)
        sources.update(
            list_count=info["list_count"],
            core_threshold=info["threshold"],
            core_rule=f">= {info['threshold']} of {info['list_count']} lists "
                      f"(core_fraction {core_fraction})" + (", within the pool" if explicit else ""),
            ranking_rule="greedy decklist completion",
        )
        return WorkSet(pool, core, ranking, lists, sources)

    if set_ids:
        set_id_set = set(set_ids)
        try:
            known = [lst for lst in library_lists(library()) if set_id_set.intersection(lst)]
        except FileNotFoundError:
            known = []
        sources["known_lists"] = len(known)
        if known:
            in_known = set().union(*(set(lst) for lst in known))
            given = set(cards)
            core = [c for c in explicit if c in given or c in in_known]
            ranking = rank_by_decklist_completion(known, explicit)
            sources.update(
                core_rule="set cards in any known deck-library list" + (" + --cards" if cards else ""),
                ranking_rule="greedy decklist completion over the known lists",
            )
            return WorkSet(explicit, core, ranking, known, sources)
        sources.update(
            core_rule="whole pool (no known deck-library list contains a set card)",
            ranking_rule="--cards order, then card id" if cards else "card id",
        )
        return WorkSet(explicit, list(explicit), list(explicit), None, sources)

    sources.update(core_rule="all given cards", ranking_rule="given order")
    return WorkSet(list(explicit), list(explicit), list(explicit), None, sources)


# ---------------------------------------------------------------------------
# shared CLI plumbing (plan + preflight)
# ---------------------------------------------------------------------------


def add_input_arguments(parser: argparse.ArgumentParser) -> None:
    """The input forms plus the data-path overrides both `plan` and
    `preflight` accept (defaults are the repo's committed data)."""
    g = parser.add_argument_group("inputs (combinable)")
    g.add_argument("--cards", help="comma-separated card ids, or @file")
    g.add_argument("--set", dest="set_prefix", metavar="PREFIX", help="release-set prefix, e.g. BT27")
    g.add_argument("--decklists", nargs="+", metavar="FILE",
                   help="decklist files in any format the engine parser accepts (globs ok)")
    g.add_argument("--archetype", help="deck-library archetype name or alias")
    g.add_argument("--core-fraction", type=float, default=None,
                   help="core = cards in >= this fraction of the lists (default: config)")
    g.add_argument("--config", help="loop config TOML (defaults from config.py)")
    d = parser.add_argument_group("data paths (defaults: committed repo data)")
    d.add_argument("--cards-json", type=Path, default=None)
    d.add_argument("--official-json", type=Path, default=None)
    d.add_argument("--library", type=Path, default=None, help="deck_library.json")
    d.add_argument("--aliases", type=Path, default=None, help="archetype_aliases.json")
    d.add_argument("--yaml-dir", type=Path, default=None, help="DSL card specs root")
    d.add_argument("--dcgo-root", type=Path, default=None,
                   help="DCGO checkout (default: the base repo's, rule 29)")


def effective_config(args: argparse.Namespace) -> LoopConfig:
    """The config file (or defaults) with the CLI's `--core-fraction` applied."""
    config = load_config(args.config)
    if args.core_fraction is not None:
        config = replace(config, core_fraction=args.core_fraction)
    return config


def work_set_from_args(
    args: argparse.Namespace, config: LoopConfig, *, cards_data: dict | None = None
) -> WorkSet:
    if cards_data is None and args.set_prefix:
        cards_data = load_cards_json(args.cards_json or CARDS_JSON)
    return resolve_work_set(
        cards=parse_cards_arg(args.cards) if args.cards else None,
        set_prefix=args.set_prefix,
        decklist_files=args.decklists,
        archetype=args.archetype,
        core_fraction=config.core_fraction,
        cards_data=cards_data,
        library_path=args.library or DECK_LIBRARY,
        aliases_path=args.aliases or ARCHETYPE_ALIASES,
    )


INPUT_ERRORS = (ValueError, LookupError, OSError)


# ---------------------------------------------------------------------------
# plan
# ---------------------------------------------------------------------------


def config_sha256(config: LoopConfig) -> str:
    """sha256 of the effective config, JSON-dumped with sorted keys."""
    return hashlib.sha256(json.dumps(asdict(config), sort_keys=True).encode("utf-8")).hexdigest()


def _base_sha() -> str | None:
    """HEAD of the checkout the plan was made in (worktrees pin to it, D10)."""
    try:
        out = subprocess.run(["git", "rev-parse", "HEAD"], cwd=REPO_ROOT, capture_output=True,
                             text=True, timeout=30)
    except (OSError, subprocess.SubprocessError):
        return None
    if out.returncode != 0:
        return None
    return out.stdout.strip() or None


def default_run_id(inputs: dict) -> str:
    stamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    digest = hashlib.sha256(json.dumps(inputs, sort_keys=True).encode("utf-8")).hexdigest()[:6]
    return f"{stamp}-{digest}"


def build_plan_document(
    *,
    run_id: str,
    work_set: WorkSet,
    config: LoopConfig,
    inputs: dict,
    dcgo_root: str | None,
    dcgo_scripts: dict[str, str | None],
    preflight: dict,
    base_sha: str | None = None,
    created_at: str | None = None,
) -> dict:
    """The frozen `plan.json`. `dcgo.unavailable` lists the pool cards with no
    DCGO script; their clauses and interactions are planned `unavailable`, and
    `resume` must re-check them (they re-enter as outstanding once DCGO gains
    the script — see `preflight.newly_available`)."""
    return {
        "version": PLAN_VERSION,
        "run_id": run_id,
        "created_at": created_at or datetime.now(timezone.utc).isoformat(timespec="seconds"),
        "base_sha": base_sha,
        "inputs": inputs,
        "config": json.loads(json.dumps(asdict(config))),
        "config_sha256": config_sha256(config),
        "work_set": work_set.to_dict(),
        "dcgo": {
            "root": dcgo_root,
            "scripts": dcgo_scripts,
            "unavailable": [c for c, p in dcgo_scripts.items() if p is None],
        },
        "preflight": preflight,
    }


def load_plan(path: str | Path) -> dict:
    return json.loads(Path(path).read_text(encoding="utf-8"))


def cli_plan(argv: list[str] | None = None) -> int:
    """`python -m tools.card_loop plan ...` — exit 0 GO, 1 NO-GO (plan still
    written, so the remedies are on disk), 2 bad input / plan already exists."""
    from . import preflight as pf  # lazy: preflight imports this module

    parser = argparse.ArgumentParser(
        prog="python -m tools.card_loop plan",
        description="Resolve inputs into a frozen run plan (runs/card-loop/<run-id>/plan.json).",
    )
    add_input_arguments(parser)
    parser.add_argument("--run-id", help="default: UTC timestamp + input digest")
    parser.add_argument("--runs-dir", type=Path, default=None, help="default: config runs_dir")
    parser.add_argument("--skip-preflight", action="store_true",
                        help="record the plan without the gate (DCGO presence is still recorded)")
    args = parser.parse_args(argv)

    try:
        config = effective_config(args)
        cards_data = load_cards_json(args.cards_json or CARDS_JSON)
        work_set = work_set_from_args(args, config, cards_data=cards_data)
    except INPUT_ERRORS as e:
        print(f"plan: {e}", file=sys.stderr)
        return 2

    inputs = {
        "cards": work_set.sources.get("cards"),
        "set": args.set_prefix,
        "decklists": work_set.sources.get("decklist_files"),
        "archetype": args.archetype,
        "core_fraction": config.core_fraction,
        "config_path": args.config,
        "skip_preflight": bool(args.skip_preflight),
    }
    run_id = args.run_id or default_run_id(inputs)
    if not _RUN_ID_RE.match(run_id):
        print(f"plan: invalid --run-id {run_id!r} (letters, digits, . _ -)", file=sys.stderr)
        return 2
    plan_path = Path(args.runs_dir or config.runs_dir) / run_id / "plan.json"
    if plan_path.exists():
        print(f"plan: {plan_path} already exists; plans are frozen -- pick another --run-id",
              file=sys.stderr)
        return 2

    dcgo_root = pf.resolve_dcgo_root(args.dcgo_root)
    if args.skip_preflight:
        scripts = pf.dcgo_script_presence(work_set.pool, dcgo_root)
        preflight_doc: dict = {"skipped": True}
        go, table = True, None
    else:
        report = pf.preflight_from_args(args, config, work_set.pool, cards_data=cards_data,
                                        dcgo_root=dcgo_root)
        scripts, preflight_doc, go = report.dcgo_scripts, report.to_dict(), report.go
        table = report.format_table()

    doc = build_plan_document(
        run_id=run_id, work_set=work_set, config=config, inputs=inputs,
        dcgo_root=str(dcgo_root) if dcgo_root else None, dcgo_scripts=scripts,
        preflight=preflight_doc, base_sha=_base_sha(),
    )
    plan_path.parent.mkdir(parents=True, exist_ok=True)
    try:
        with open(plan_path, "x", encoding="utf-8") as f:
            json.dump(doc, f, indent=2, sort_keys=False)
            f.write("\n")
    except FileExistsError:
        print(f"plan: {plan_path} already exists; plans are frozen -- pick another --run-id",
              file=sys.stderr)
        return 2

    if table:
        print(table)
    unavailable = doc["dcgo"]["unavailable"]
    print(f"plan {run_id}: pool {len(work_set.pool)}, core {len(work_set.core)}, "
          f"unavailable (no DCGO script) {len(unavailable)}; "
          f"preflight {'skipped' if args.skip_preflight else ('GO' if go else 'NO-GO')}")
    print(f"  ranking head: {', '.join(work_set.ranking[:10])}"
          + (" ..." if len(work_set.ranking) > 10 else ""))
    print(f"  wrote {plan_path}")
    if not go:
        print("NO-GO: no worker may be invoked for this plan until the failing checks pass "
              "(re-plan with a new --run-id after fixing).", file=sys.stderr)
    return 0 if go else 1
