"""SIM (driver-only, no model): every authored scenario must lower and run in
our engine before any oracle time is spent (design D4, D13).

Each scenario runs `dcgo-harness exam --sim-only` against the deck books
`tools/exam_sim_all.candidates()` picks for it, best first, and passes on the
first book that lowers and asserts clean -- exactly the CI driver's rule.

    every scenario passes     -> ORACLE, data["deck_books"] = {path: book}
    any scenario fails        -> AUTHORING, data["sim_failure"] = {path: [lines]},
                                 and a `gate_fail` (gate "sim") correction of the
                                 authoring attempt (D11: an author's immediate correction).
                                 A Q&A scenario whose `expect_ruling:` is not the block
                                 both families agreed (ENCODE) fails the same way.
    the harness cannot run    -> StageDeferred (infrastructure, nobody's correction)
"""
from __future__ import annotations

from .. import corrections as corr
from ..driver_contracts import ItemRecord, StageOutcome
from . import base, harness


class SimExecutor:
    state = "SIM"

    def run(self, ctx, item: ItemRecord) -> StageOutcome:
        paths = list(item.data.get("scenario_paths") or [])
        books: dict[str, str] = {}
        notes: dict[str, list] = {}
        failures: dict[str, list] = {}
        if not paths:
            failures["(none)"] = ["no scenario was authored for this item"]
        for path in paths:
            if not base.repo_path(ctx, path).is_file():
                failures[path] = [f"scenario file {path} not found in the run tree"]
                continue
            try:
                candidates = harness.deck_books_for(ctx.repo, path)
            except Exception as e:  # a scenario YAML that does not parse
                failures[path] = [f"scenario {path} could not be read: {e}"]
                continue
            if not candidates:
                failures[path] = [f"no deck book resolves the `rest:` decks of {path}"]
                continue
            first_failure = None
            for book in candidates:
                rc, out, err = harness.run(ctx, harness.sim_argv(ctx, path, book), harness.SIM_TIMEOUT_S)
                report = harness.parse_sim_output(rc, out, err)
                if report.summary is None:
                    # No exam summary: the harness itself did not run (missing
                    # binary, bad arguments). Not an authoring failure.
                    raise base.StageDeferred(
                        f"dcgo-harness sim-only did not run for {path}: {report.failure_text()}",
                        StageOutcome(next_state=item.state, reason="harness unavailable"))
                if report.passed:
                    books[path] = book
                    notes[path] = report.notes
                    break
                first_failure = first_failure or report
            else:
                failures[path] = list(first_failure.failures) or [first_failure.failure_text()]

        # A Q&A exam's `expect_ruling:` is the value BOTH families agreed (D5/D7);
        # the author must carry it verbatim, so an altered block is the author's error.
        agreed = item.data.get("expect_ruling")
        if agreed:
            for path in paths:
                if path in failures or not base.repo_path(ctx, path).is_file():
                    continue
                found = (base.load_yaml(base.repo_path(ctx, path)) or {}).get("expect_ruling")
                if found != agreed:
                    failures[path] = [f"the scenario's expect_ruling {found!r} is not the block both "
                                      f"families agreed: {agreed!r}"]

        if failures:
            corrections = []
            author = item.data.get("author_attempt")
            if author:
                detail = "; ".join(f"{p}: {' | '.join(ls)}" for p, ls in failures.items())[:2000]
                corrections.append(corr.gate_fail(author, gate="sim",
                                                  stage=item.data.get("author_stage") or _author_stage(item),
                                                  item=item.item, detail=detail, ts=ctx.now()))
            return base.outcome("AUTHORING", item=item, corrections=corrections,
                                data={"sim_failure": failures, "deck_books": books or None},
                                reason=f"sim-only failed for {', '.join(failures)}")
        return base.outcome("ORACLE", item=item,
                            data={"deck_books": books, "sim_notes": notes, "sim_failure": None,
                                  "oracle_retry_paths": None, "oracle_results": None},
                            reason="sim-only clean; submitting to the oracle")


def _author_stage(item: ItemRecord) -> str:
    return "author_interaction" if item.kind == "interaction" else "author_clause"
