"""New-set / node preflight for a card-loop run (design D3, task 2.5).

Every check returns `{name, status: ok|warn|fail, detail, remedy}` and every
`fail` names its remedy. The run is GO iff no check failed. Checks:

- `mirror_coverage`  every pool card is in `cards.json` AND in
  `card_official.json` with a `colors` key (an entry without `colors` is a
  failed-lookup skeleton, not data).
- `keyword_gate`     the author-set keyword gate (`tools.author_set.keyword_gate`)
  over the pool's printed text. A blocking keyword fails only when a card that
  uses it still needs implementing (no DSL spec); on already-authored cards it
  is a `warn` (the gate lags the engine: a keyword newly lowered through the DSL
  counts as covered only once the manifest's `dsl_lowered_keywords` is
  regenerated).
- `dcgo_scripts`     per-card DCGO `.cs` presence. A missing script is a `warn`:
  that card's clauses and interactions are `unavailable`, the rest proceeds. A
  missing DCGO checkout is a `fail` (it would silently make every card
  unavailable).
- `worker_cli.<family>`  the group-3 adapters' `resolve_*_exe()`; an adapter
  that is not built yet is a `warn`, an unresolved binary a `fail`.
- `oracle_node[.<check>]`  `dcgo-harness node status --build <player_dir>`,
  which carries the player's action-space-hash gate; each harness check is
  surfaced as its own row. No `player_dir` configured -> `warn`.

**Driver contract: NO-GO stops worker spend.** A driver MUST call
`run_preflight` (or read the plan's recorded result) and `ensure_go(report)`
before constructing any worker packet; `ensure_go` raises `PreflightNoGo`
listing each failing check and its remedy. `plan` still writes `plan.json` on a
NO-GO (so the remedies are on disk) and exits 1; nothing may run from it.
"""
from __future__ import annotations

import argparse
import importlib
import json
import re
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Callable, Iterable, Sequence

from data_paths import CARDS_JSON, DATA_DIR, REPO_ROOT
from tools.author_set.gap_router import cards_using_keyword
# `_dcgo_script_path` is private, but it is THE implementation of DCGO's
# `<SET>/<Colour>/<ID_>.cs` layout in Python; reuse it rather than add a fourth.
from tools.clause_coverage.card_sources import _dcgo_script_path, default_dcgo_root

from .config import LoopConfig
from .contracts import FAMILIES
from .workset import (
    CARD_OFFICIAL_JSON,
    INPUT_ERRORS,
    YAML_CARDS_DIR,
    add_input_arguments,
    effective_config,
    load_cards_json,
    work_set_from_args,
)

STATUSES = ("ok", "warn", "fail")

#: Seconds allowed for `cargo run -p dcgo-harness -- node status` (the first
#: invocation in a fresh target dir compiles the harness).
NODE_STATUS_TIMEOUT_S = 900

_TEXT_FIELDS = ("effect_description_eng", "inherited_effect_description_eng",
                "security_effect_description_eng")
_LIST_LIMIT = 15


def _fmt_ids(ids: Sequence[str], limit: int = _LIST_LIMIT) -> str:
    ids = list(ids)
    head = ", ".join(ids[:limit])
    return head + (f" (+{len(ids) - limit} more)" if len(ids) > limit else "")


@dataclass(frozen=True)
class Check:
    name: str
    status: str
    detail: str
    remedy: str | None = None

    def __post_init__(self):
        if self.status not in STATUSES:
            raise ValueError(f"unknown check status {self.status!r}; expected one of {STATUSES}")
        if self.status == "fail" and not self.remedy:
            raise ValueError(f"check {self.name!r} fails without naming a remedy")

    def to_dict(self) -> dict:
        return {"name": self.name, "status": self.status, "detail": self.detail,
                "remedy": self.remedy}


class PreflightNoGo(RuntimeError):
    """Raised by `ensure_go`: the run must not spend model budget."""


@dataclass
class PreflightReport:
    checks: list[Check]
    #: card id -> DCGO script path relative to the DCGO root, or None.
    dcgo_scripts: dict[str, str | None] = field(default_factory=dict)
    dcgo_root: str | None = None

    @property
    def go(self) -> bool:
        return not any(c.status == "fail" for c in self.checks)

    @property
    def failing(self) -> list[Check]:
        return [c for c in self.checks if c.status == "fail"]

    @property
    def unavailable(self) -> list[str]:
        """Pool cards with no DCGO script: plan their items as `unavailable`."""
        return [c for c, p in self.dcgo_scripts.items() if p is None]

    def to_dict(self) -> dict:
        return {"go": self.go, "checks": [c.to_dict() for c in self.checks]}

    def format_table(self) -> str:
        """ASCII-only (Windows consoles default to cp1252)."""
        width = max([len(c.name) for c in self.checks] + [5])
        lines = ["GO" if self.go else "NO-GO", f"{'STATUS':6}  {'CHECK'.ljust(width)}  DETAIL"]
        for c in self.checks:
            lines.append(f"{c.status:6}  {c.name.ljust(width)}  {_ascii(c.detail)}")
            if c.remedy and c.status != "ok":
                lines.append(f"{'':6}  {'':{width}}  -> {_ascii(c.remedy)}")
        return "\n".join(lines)


def _ascii(s: str) -> str:
    return s.encode("ascii", "replace").decode("ascii")


def ensure_go(report: PreflightReport) -> None:
    """The spend guard. Call before any worker is constructed or invoked."""
    if report.go:
        return
    reasons = "; ".join(f"{c.name}: {c.detail} -> {c.remedy}" for c in report.failing)
    raise PreflightNoGo(f"preflight NO-GO, no worker may run: {reasons}")


# ---------------------------------------------------------------------------
# mirror coverage
# ---------------------------------------------------------------------------


def check_mirror_coverage(pool: Sequence[str], cards: dict, official: dict) -> Check:
    off = official.get("cards", {}) if isinstance(official, dict) else {}
    not_in_cards = [c for c in pool if c not in cards]
    not_in_official = [c for c in pool if c not in off]
    skeletons = [c for c in pool if c in off and "colors" not in (off[c] or {})]
    if not (not_in_cards or not_in_official or skeletons):
        return Check("mirror_coverage", "ok",
                     f"{len(pool)}/{len(pool)} pool cards in cards.json and card_official.json")
    detail, remedy = [], []
    if not_in_cards:
        detail.append(f"{len(not_in_cards)} missing from cards.json: {_fmt_ids(not_in_cards)}")
        sets = sorted({c.partition("-")[0] for c in not_in_cards})
        remedy.append("; ".join(f"python code/tools/ingest_cards.py --set {s}" for s in sets))
    official_gap = not_in_official + skeletons
    if not_in_official:
        detail.append(f"{len(not_in_official)} missing from card_official.json: "
                      f"{_fmt_ids(not_in_official)}")
    if skeletons:
        detail.append(f"{len(skeletons)} failed-lookup skeletons (no colors) in "
                      f"card_official.json: {_fmt_ids(skeletons)}")
    if official_gap:
        remedy.append("rebuild the official mirror: python code/tools/build_card_bundles.py --ids "
                      + " ".join(official_gap[:_LIST_LIMIT])
                      + (" ..." if len(official_gap) > _LIST_LIMIT else ""))
    return Check("mirror_coverage", "fail", "; ".join(detail), "; then ".join(remedy))


# ---------------------------------------------------------------------------
# keyword gate
# ---------------------------------------------------------------------------


def _card_text(card: dict) -> str:
    return " ".join(str(card.get(k, "") or "") for k in _TEXT_FIELDS)


def _default_triage(texts, set_prefix: str = ""):
    from tools.author_set.keyword_gate import triage_set_from_artifacts

    return triage_set_from_artifacts(
        texts,
        manifest_path=str(DATA_DIR / "dcgo_keyword_manifest.json"),
        lexicons_path=str(DATA_DIR / "author_set_lexicons.json"),
        set_prefix=set_prefix,
    )


def check_keyword_gate(
    pool: Sequence[str],
    cards: dict,
    *,
    yaml_ids: set[str],
    triage: Callable | None = None,
) -> Check:
    """Run the author-set keyword gate over the pool's printed text.

    `flag_for_human` (nobody implements it) and `auto_ingest_subsystem`
    (DCGO has it, the engine needs a scheduled port) block authoring the
    cards that use them; `auto_ingest` (a cheap DCGO port) only warns.
    """
    triage = triage or _default_triage
    present = {c: cards[c] for c in pool if c in cards}
    report = triage([_card_text(card) for card in present.values()], set_prefix="card-loop")

    blocking: dict[str, list[str]] = {}   # "kind: keyword" -> cards still to implement
    authored: dict[str, list[str]] = {}   # "kind: keyword" -> cards that already have a spec
    for kind in ("flag_for_human", "auto_ingest_subsystem"):
        for kw in sorted(getattr(report, kind, {}) or {}):
            users = cards_using_keyword(kw, present)
            to_author = [c for c in users if c not in yaml_ids]
            label = f"{kind}: {kw}"
            if to_author or not users:  # unattributable -> conservative
                blocking[label] = to_author or ["(unattributed)"]
            else:
                authored[label] = users
    porting = {}
    for kw in sorted(getattr(report, "auto_ingest", {}) or {}):
        to_author = [c for c in cards_using_keyword(kw, present) if c not in yaml_ids]
        if to_author:
            porting[f"auto_ingest: {kw}"] = to_author

    def describe(groups: dict[str, list[str]]) -> str:
        return "; ".join(f"{k} ({_fmt_ids(v, 8)})" for k, v in groups.items())

    parts = []
    if blocking:
        parts.append(f"keywords block cards still to implement: {describe(blocking)}")
    if porting:
        parts.append(f"DCGO-implemented keywords to port before implementing: {describe(porting)}")
    if authored:
        parts.append(f"gate tokens on already-authored cards only (gate lexicon lag): "
                     f"{describe(authored)}")
    if blocking:
        return Check(
            "keyword_gate", "fail", "; ".join(parts),
            "flag_for_human: get human direction and log the gap "
            "(tools.author_set.gap_router.route_flagged_keyword -> docs/RUST_ENGINE_GAPS.md); "
            "auto_ingest_subsystem: schedule the engine port (/assess-archetype-rust); "
            "or drop those cards from the work set",
        )
    if parts:
        return Check("keyword_gate", "warn", "; ".join(parts),
                     "port auto_ingest keywords from DCGO (author-set Phase 2); for tokens "
                     "the engine already runs, regenerate data/dcgo_keyword_manifest.json "
                     "(python code/tools/author_set/dcgo_manifest.py; a DSL-lowered keyword "
                     "needs a DSL_KEYWORD_VOCAB entry) or teach the gate lexicon the token")
    return Check("keyword_gate", "ok", f"no blocking keyword in {len(present)} cards' text")


# ---------------------------------------------------------------------------
# DCGO script presence
# ---------------------------------------------------------------------------


def resolve_dcgo_root(override: str | Path | None = None) -> Path | None:
    """`override`, else the base repo's DCGO checkout (rule 29; honours
    `DIGIMON_DCGO_ROOT`). A worktree's own `./DCGO` is an empty placeholder."""
    if override:
        return Path(override)
    return default_dcgo_root()


def _card_effect_dir(root: Path) -> Path:
    return root / "Assets" / "Scripts" / "CardEffect"


def dcgo_script_presence(pool: Iterable[str], dcgo_root: str | Path | None) -> dict[str, str | None]:
    """`{card: "Assets/Scripts/CardEffect/<SET>/<Colour>/<ID_>.cs" | None}`.

    Never cached: `resume` calls this again so a card whose script DCGO has
    since gained re-enters the plan as outstanding.
    """
    root = Path(dcgo_root) if dcgo_root else None
    usable = root is not None and _card_effect_dir(root).is_dir()
    out: dict[str, str | None] = {}
    for card in pool:
        path = _dcgo_script_path(root, card) if usable else None
        out[card] = path.relative_to(root).as_posix() if path is not None else None
    return out


def newly_available(before: dict[str, str | None], after: dict[str, str | None]) -> list[str]:
    """Cards that had no DCGO script at plan time and have one now."""
    return [c for c, p in before.items() if p is None and after.get(c)]


def check_dcgo_scripts(
    pool: Sequence[str], dcgo_root: str | Path | None
) -> tuple[Check, dict[str, str | None]]:
    presence = dcgo_script_presence(pool, dcgo_root)
    root = Path(dcgo_root) if dcgo_root else None
    if root is None or not _card_effect_dir(root).is_dir():
        where = str(root) if root else "(unresolved)"
        return (
            Check("dcgo_scripts", "fail",
                  f"no DCGO checkout at {where}: every card would read as unavailable",
                  "initialise DCGO in the BASE repo, never a worktree (CLAUDE.md rule 29), "
                  "or pass --dcgo-root / set DIGIMON_DCGO_ROOT"),
            presence,
        )
    missing = [c for c, p in presence.items() if p is None]
    if missing:
        return (
            Check("dcgo_scripts", "warn",
                  f"{len(missing)} of {len(presence)} pool cards have no DCGO script "
                  f"(their clauses and interactions are planned unavailable): {_fmt_ids(missing)}",
                  "none needed to proceed; resume re-checks after a DCGO bump"),
            presence,
        )
    return Check("dcgo_scripts", "ok", f"{len(presence)}/{len(presence)} pool cards have a DCGO script"), presence


# ---------------------------------------------------------------------------
# worker CLIs (adapters are group 3's)
# ---------------------------------------------------------------------------

#: family -> (adapter module, resolver function). Each resolver is
#: `() -> str | None`: the CLI's executable path, or None when not installed.
_ADAPTERS = {
    "claude": ("tools.card_loop.workers.claude", "resolve_claude_exe"),
    "codex": ("tools.card_loop.workers.codex", "resolve_codex_exe"),
}

_INSTALL = {
    "claude": "install Claude Code and make sure `claude` is on PATH",
    "codex": "install the Codex app (Store package OpenAI.Codex*) so codex.exe resolves "
             "from its InstallLocation",
}


def _lazy_resolver(module: str, attr: str) -> Callable[[], str | None]:
    def resolve() -> str | None:
        fn = getattr(importlib.import_module(module), attr, None)
        if fn is None:
            raise ImportError(f"{module} has no {attr}")
        return fn()
    return resolve


def load_cli_resolvers() -> dict[str, Callable[[], str | None]]:
    """Resolvers that import their adapter only when called (ImportError ->
    'not built yet')."""
    return {family: _lazy_resolver(mod, attr) for family, (mod, attr) in _ADAPTERS.items()}


def check_worker_clis(resolvers: dict[str, Callable[[], str | None]] | None = None) -> list[Check]:
    resolvers = load_cli_resolvers() if resolvers is None else resolvers
    checks = []
    for family in FAMILIES:
        name = f"worker_cli.{family}"
        resolver = resolvers.get(family)
        if resolver is None:
            checks.append(Check(name, "warn", "no resolver registered", _INSTALL[family]))
            continue
        try:
            exe = resolver()
        except ImportError as e:
            checks.append(Check(name, "warn", f"adapter not built yet ({e})",
                                f"lands with tools/card_loop/workers/{family}.py (task group 3)"))
            continue
        except Exception as e:  # a resolver must not crash the preflight
            checks.append(Check(name, "fail", f"resolver raised {type(e).__name__}: {e}",
                                _INSTALL[family]))
            continue
        if not exe:
            checks.append(Check(name, "fail", f"{family} CLI not found", _INSTALL[family]))
        else:
            checks.append(Check(name, "ok", str(exe)))
    return checks


# ---------------------------------------------------------------------------
# oracle node
# ---------------------------------------------------------------------------

_NODE_LINE = re.compile(r"^\s*\[(ok|warn|fail)\]\s+([^:]+?):\s?(.*)$")
_REMEDY_LINE = re.compile(r"^\s*->\s?(.*)$")

Runner = Callable[[list[str], Path], tuple[int, str]]


def node_status_command(config: LoopConfig) -> list[str]:
    cmd = ["cargo", "run", "-q", "-p", "dcgo-harness", "--"]
    if config.harness_root:
        cmd += ["--root", str(config.harness_root)]
    return cmd + ["node", "status", "--build", str(config.player_dir)]


def default_runner(cmd: list[str], cwd: Path) -> tuple[int, str]:
    proc = subprocess.run(cmd, cwd=cwd, capture_output=True, text=True, encoding="utf-8",
                          errors="replace", timeout=NODE_STATUS_TIMEOUT_S)
    return proc.returncode, (proc.stdout or "") + (proc.stderr or "")


def parse_node_status(output: str) -> list[Check]:
    """`node status` prints `GO|NO-GO` then `[status] name: detail` rows, each
    optionally followed by a `-> remedy` row (dcgo-harness `node.rs`)."""
    rows: list[dict] = []
    for line in output.splitlines():
        m = _NODE_LINE.match(line)
        if m:
            rows.append({"status": m.group(1), "name": m.group(2).strip(),
                         "detail": m.group(3).strip(), "remedy": None})
            continue
        r = _REMEDY_LINE.match(line)
        if r and rows and rows[-1]["remedy"] is None:
            rows[-1]["remedy"] = r.group(1).strip()
    checks = []
    for row in rows:
        remedy = row["remedy"]
        if row["status"] == "fail" and not remedy:
            remedy = "see `dcgo-harness node status` output"
        checks.append(Check(f"oracle_node.{row['name']}", row["status"], row["detail"], remedy))
    return checks


def check_oracle_node(config: LoopConfig, *, runner: Runner | None = None,
                      cwd: Path | None = None) -> list[Check]:
    if not config.player_dir:
        return [Check("oracle_node", "warn",
                      "oracle not configured (player_dir unset): node, harness and the "
                      "player's action-space hash were not checked",
                      "set player_dir (and harness_root) in the loop config TOML")]
    runner = runner or default_runner
    cmd = node_status_command(config)
    try:
        rc, output = runner(cmd, cwd or REPO_ROOT)
    except (OSError, subprocess.SubprocessError) as e:
        return [Check("oracle_node", "fail", f"could not run `{' '.join(cmd)}`: {e}",
                      "make sure cargo is on PATH and `cargo build -p dcgo-harness` succeeds")]
    checks = parse_node_status(output)
    if rc != 0 and not any(c.status == "fail" for c in checks):
        tail = " | ".join(output.strip().splitlines()[-3:]) or "(no output)"
        remedy = ("set harness_root in the loop config (`node status` needs --root <DIR>)"
                  if not config.harness_root else
                  "fix the error above and re-run `dcgo-harness node status`")
        checks.append(Check("oracle_node", "fail", f"`node status` exited {rc}: {tail}", remedy))
    elif not checks:
        checks.append(Check("oracle_node", "warn", "`node status` printed no checks",
                            "run it by hand and compare with dcgo-harness node.rs"))
    return checks


# ---------------------------------------------------------------------------
# aggregate + CLI
# ---------------------------------------------------------------------------


def run_preflight(
    pool: Sequence[str],
    *,
    config: LoopConfig,
    cards: dict,
    official: dict,
    dcgo_root: str | Path | None,
    yaml_ids: set[str],
    runner: Runner | None = None,
    cli_resolvers: dict[str, Callable[[], str | None]] | None = None,
    triage: Callable | None = None,
) -> PreflightReport:
    """Every check, always (the next move depends on WHICH check failed)."""
    checks = [
        check_mirror_coverage(pool, cards, official),
        check_keyword_gate(pool, cards, yaml_ids=yaml_ids, triage=triage),
    ]
    dcgo_check, presence = check_dcgo_scripts(pool, dcgo_root)
    checks.append(dcgo_check)
    checks += check_worker_clis(cli_resolvers)
    checks += check_oracle_node(config, runner=runner)
    return PreflightReport(checks=checks, dcgo_scripts=presence,
                           dcgo_root=str(dcgo_root) if dcgo_root else None)


def yaml_card_ids(yaml_dir: str | Path = YAML_CARDS_DIR) -> set[str]:
    """Card ids with a DSL spec (`<set>/<ID>.yaml`) anywhere under `yaml_dir`."""
    root = Path(yaml_dir)
    return {p.stem for p in root.rglob("*.yaml")} if root.is_dir() else set()


def _load_json(path: Path) -> dict:
    return json.loads(Path(path).read_text(encoding="utf-8"))


def preflight_from_args(
    args: argparse.Namespace,
    config: LoopConfig,
    pool: Sequence[str],
    *,
    cards_data: dict | None = None,
    dcgo_root: Path | None = None,
) -> PreflightReport:
    """Load the data the checks need from the CLI's paths (or the defaults)."""
    cards = cards_data if cards_data is not None else load_cards_json(args.cards_json or CARDS_JSON)
    official = _load_json(args.official_json or CARD_OFFICIAL_JSON)
    root = dcgo_root if dcgo_root is not None else resolve_dcgo_root(args.dcgo_root)
    return run_preflight(
        pool, config=config, cards=cards, official=official, dcgo_root=root,
        yaml_ids=yaml_card_ids(args.yaml_dir or YAML_CARDS_DIR),
        cli_resolvers=load_cli_resolvers(),
    )


def cli_preflight(argv: list[str] | None = None) -> int:
    """`python -m tools.card_loop preflight ...` — exit 0 GO, 1 NO-GO, 2 bad input."""
    parser = argparse.ArgumentParser(
        prog="python -m tools.card_loop preflight",
        description="New-set / node readiness checks for a card-loop work set.",
    )
    add_input_arguments(parser)
    args = parser.parse_args(argv)
    try:
        config = effective_config(args)
        cards_data = load_cards_json(args.cards_json or CARDS_JSON)
        work_set = work_set_from_args(args, config, cards_data=cards_data)
        report = preflight_from_args(args, config, work_set.pool, cards_data=cards_data)
    except INPUT_ERRORS as e:
        print(f"preflight: {e}", file=sys.stderr)
        return 2
    print(report.format_table())
    print(f"pool {len(work_set.pool)} cards; unavailable (no DCGO script) {len(report.unavailable)}")
    return 0 if report.go else 1
