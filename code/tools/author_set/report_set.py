"""Report-only dry run for a release set (task 8.3 — feeds the Phase 1-3 preview).

Composes the deterministic primitives into the preview a user inspects BEFORE a
full `/author-set` run dispatches any agents:

    Phase 1  resolve set + (optional) pull/diff vs cards.json
    Phase 2  keyword-gate triage (covered / auto_ingest / flag_for_human)
    Phase 3  cluster into slices + orphan bucket

No agents, no engine writes, no network unless ``do_pull=True``. The gate's
``flag_for_human`` list and the manifest's ``auto_ingest_candidates`` together
tell the user what substrate work a full run would require up front.

CLI:
    PYTHONPATH=code python -m tools.author_set.report_set BT17 [--pull]
"""

from __future__ import annotations

import json
import os
import subprocess
from dataclasses import dataclass, field
from typing import Optional

from .clusterer import Partition, cluster
from .gap_router import cards_using_keyword, cards_using_subsystem_keyword
from .ingest_diff import SetDiff, scoped_test_source_command
from .keyword_gate import KeywordGateReport, triage_set_from_artifacts
from .set_resolver import normalize_prefix, resolve_set


def _resolve_dcgo_root() -> str:
    """Best-effort base-repo DCGO checkout (rule 29: lives in the base repo, not
    a worktree). Env override wins; else the git common-dir parent + /DCGO; else
    "" (a nonexistent root, which makes the per-card classifier fall back to the
    text scanner — still correct, just not DCGO-verified)."""
    env = os.environ.get("BASE_DCGO")
    if env:
        return env
    try:
        common = subprocess.check_output(
            ["git", "rev-parse", "--path-format=absolute", "--git-common-dir"],
            text=True, stderr=subprocess.DEVNULL,
        ).strip()
        cand = os.path.join(os.path.dirname(common), "DCGO")
        if os.path.isdir(cand):
            return cand
    except Exception:
        pass
    return ""


@dataclass
class SetReport:
    set_prefix: str
    card_ids: list[str]
    diff: Optional[SetDiff]
    keywords: KeywordGateReport
    partition: Partition
    # Per-subsystem-keyword: the cards that ACTUALLY use it (DCGO-verified, text
    # fallback). A subsystem keyword excludes only THESE cards, not their whole
    # slice — the rest of the set is authorable. (The BT25 Aegiomon fix: [Link]
    # excludes 2 of 18 slice cards, not all 18.)
    subsystem_affected: dict[str, list[str]] = field(default_factory=dict)
    flagged_affected: dict[str, list[str]] = field(default_factory=dict)

    @property
    def blocks_full_run(self) -> bool:
        return self.keywords.blocks_authoring

    @property
    def excluded_cards(self) -> set[str]:
        """Union of cards excluded by any subsystem or flagged keyword."""
        out: set[str] = set()
        for v in self.subsystem_affected.values():
            out.update(v)
        for v in self.flagged_affected.values():
            out.update(v)
        return out

    def format(self) -> str:
        lines = [f"=== /author-set {self.set_prefix} — DRY RUN ===",
                 f"Cards in set: {len(self.card_ids)}"]
        if self.diff is not None:
            lines.append(f"Ingest diff: {self.diff.summary()}")
        else:
            lines.append("Ingest diff: (skipped — no pull)")
        lines.append("")
        lines.append("Phase 2 — keyword gate:")
        lines.append(f"  covered:              {len(self.keywords.covered)}")
        lines.append(f"  auto_ingest (simple): {sorted(self.keywords.auto_ingest) or '-'}")
        lines.append(f"  auto_ingest SUBSYSTEM (assess, don't auto-port): "
                     f"{sorted(self.keywords.auto_ingest_subsystem) or '-'}")
        # Per-subsystem-keyword affected cards (DCGO-verified). Only THESE are
        # excluded — NOT the whole archetype slice they belong to.
        for kw in sorted(self.subsystem_affected):
            aff = self.subsystem_affected[kw]
            lines.append(f"      `{kw}` excludes ONLY ({len(aff)}): {', '.join(aff) or '-'} "
                         f"— the rest of their slices are authorable")
        lines.append(f"  FLAG_FOR_HUMAN:       {sorted(self.keywords.flag_for_human) or '-'}")
        for kw in sorted(self.flagged_affected):
            aff = self.flagged_affected[kw]
            lines.append(f"      `{kw}` excludes ({len(aff)}): {', '.join(aff) or '-'}")
        if self.keywords.lexicon_misses:
            lines.append(f"  lexicon-miss patches suggested: {sorted(self.keywords.lexicon_misses)}")
        lines.append("")
        lines.append("Phase 3 — slice partition:")
        lines.append(self.partition.format())
        lines.append("")
        lines.append("Verification after authoring merge:")
        lines.append(f"  scoped-test source: `{scoped_test_source_command()}`")
        lines.append("  use `cards_behavioral_filter` and `side_binaries`; if "
                     "`full_suite_required` is true, run the broader suite.")
        lines.append("")
        excluded = self.excluded_cards
        if self.keywords.flag_for_human:
            lines.append("RESULT: HALT — an unknown keyword (flag_for_human) needs human "
                         "context/direction before authoring its cards. The cards NOT "
                         f"depending on it ({len(self.card_ids) - len(excluded)} of "
                         f"{len(self.card_ids)}) are still authorable.")
        elif self.keywords.auto_ingest_subsystem:
            lines.append(f"RESULT: PARTIAL — a subsystem keyword is present, but it excludes "
                         f"only {len(excluded)} card(s) ({', '.join(sorted(excluded)) or '-'}); "
                         f"author the remaining {len(self.card_ids) - len(excluded)} (skip the "
                         f"excluded cards, schedule the subsystem). Do NOT exclude whole slices.")
        else:
            lines.append("RESULT: clear to proceed (pending slice-partition approval).")
        return "\n".join(lines)


def _set_texts(card_ids, cards) -> list[str]:
    return [
        " ".join(str(cards[c].get(k, "") or "") for k in
                 ("effect_description_eng", "inherited_effect_description_eng",
                  "security_effect_description_eng"))
        for c in card_ids
    ]


def build_report(
    set_prefix: str,
    cards: dict,
    *,
    do_pull: bool = False,
    archetype_map: dict | None = None,
    dcgo_root: str | None = None,
) -> SetReport:
    """Assemble the Phase 1-3 dry-run report for a set.

    ``cards`` is the cards.json dict. When ``do_pull`` is set, pulls the live set
    and diffs (tolerant of network failure via ``ingest_diff.pull_and_diff``).
    """
    pre = normalize_prefix(set_prefix)
    diff = None
    if do_pull:
        from .ingest_diff import pull_and_diff

        diff, _pulled = pull_and_diff(pre, cards)

    ids = resolve_set(pre, cards)
    texts = _set_texts(ids, cards)
    kw = triage_set_from_artifacts(texts, set_prefix=pre)
    part = cluster({c: cards[c] for c in ids}, pre, archetype_map=archetype_map)

    set_cards = {c: cards[c] for c in ids}
    root = _resolve_dcgo_root() if dcgo_root is None else dcgo_root
    # Subsystem keywords exclude only their actual users (DCGO-verified) — never a
    # whole slice. Flagged (unknown) keywords have no DCGO spec, so text is the
    # only signal available.
    subsystem_affected = {
        kwname: cards_using_subsystem_keyword(kwname, set_cards, dcgo_root=root, set_prefix=pre)
        for kwname in kw.auto_ingest_subsystem
    }
    flagged_affected = {
        kwname: cards_using_keyword(kwname, set_cards) for kwname in kw.flag_for_human
    }
    return SetReport(
        set_prefix=pre, card_ids=ids, diff=diff, keywords=kw, partition=part,
        subsystem_affected=subsystem_affected, flagged_affected=flagged_affected,
    )


def _load_cards() -> dict:
    from data_paths import CARDS_JSON

    with open(CARDS_JSON, encoding="utf-8") as f:
        data = json.load(f)
    return {c["card_id"]: c for c in data} if isinstance(data, list) else data


def main(argv=None):
    import argparse

    ap = argparse.ArgumentParser(description="Release-set authoring dry run (Phase 1-3 preview).")
    ap.add_argument("set_prefix", help="e.g. BT17, EX12, ST3")
    ap.add_argument("--pull", action="store_true", help="pull live set + diff vs cards.json")
    args = ap.parse_args(argv)
    report = build_report(args.set_prefix, _load_cards(), do_pull=args.pull)
    print(report.format())
    return 1 if report.blocks_full_run else 0


if __name__ == "__main__":
    raise SystemExit(main())
