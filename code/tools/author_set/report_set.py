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
from dataclasses import dataclass
from typing import Optional

from .clusterer import Partition, cluster
from .ingest_diff import SetDiff
from .keyword_gate import KeywordGateReport, triage_set_from_artifacts
from .set_resolver import normalize_prefix, resolve_set


@dataclass
class SetReport:
    set_prefix: str
    card_ids: list[str]
    diff: Optional[SetDiff]
    keywords: KeywordGateReport
    partition: Partition

    @property
    def blocks_full_run(self) -> bool:
        return self.keywords.blocks_authoring

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
        lines.append(f"  FLAG_FOR_HUMAN:       {sorted(self.keywords.flag_for_human) or '-'}")
        if self.keywords.lexicon_misses:
            lines.append(f"  lexicon-miss patches suggested: {sorted(self.keywords.lexicon_misses)}")
        lines.append("")
        lines.append("Phase 3 — slice partition:")
        lines.append(self.partition.format())
        lines.append("")
        if self.blocks_full_run:
            lines.append("RESULT: a full run is BLOCKED — resolve flagged keywords "
                         "(provide context/direction) before authoring.")
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
    return SetReport(set_prefix=pre, card_ids=ids, diff=diff, keywords=kw, partition=part)


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
