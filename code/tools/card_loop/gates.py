"""The fix gate (design D5, D13; task 6.5; spec "Fixes pass the fix gate and
engine fixes stay off main").

`FixGate.check(ctx, item, merge)` judges a merged `fix_card` / `fix_engine`
result. The worker's validated result lives in `item.data["fix"]` (also read:
`item.data["fix_result"]`, or `item.data` itself when it carries the schema
keys `files, tests, test_result_lines, gaps, citation, notes`).

Cheap checks first; if any fails, nothing is built or run:
  * a fix result exists and the merge landed (`merge.ok`, `merge.sha`);
  * the fix reports no open `gaps` (non-empty means park, not gate);
  * engine code (`merge.is_engine_path`) is on a `card-loop/<run>/engine-*`
    branch, never the run branch (D13);
  * the citation is present and well-formed (`citation_problem`): `rule` = a
    `general_rule.pdf` section like `16-36` / `§16-36-1`, `ruling` =
    `qa:Q<number>`, `dcgo` = `<File>.cs[:<line>[-<line>]]` or a C# path;
  * at least one named test, each a Rust test path (`parse_test_name`).

Then, executed -- the worker's pasted `test_result_lines` are evidence of its
claim only, never of the outcome:
  * AFTER: a scratch worktree (`<root>/cl-<run>-gate`, reused and reset each
    time) is detached at `merge.sha`; the named tests run there and each must
    match >= 1 executed test, all `ok` (an ignored or ambiguous bare name
    fails).
  * the scoped suite: reused from `merge.scope["suite"]` when the merger ran
    it green; otherwise impact_scope + the scoped suite run in the same tree.
  * BEFORE: in that tree every NON-test path the fix touched (`merge.touched`
    minus anything under a crate's `tests/`) is restored to `merge.sha^` --
    `git checkout <sha>^ -- <path>`, or removed when the fix added it -- while
    the fix's tests and their `mod` wiring stay. The named tests run again and
    each must be red: a FAILED result, or a build that fails to compile (a test
    that needs the fix's new API is red by construction). A test green here
    proves nothing about the fix.
Every command, its rc and the tail of its output go into `evidence`; every
failed check's reason names its remedy.

Cargo argv: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test
<binary> -- <filter>... --test-threads=8` with `RUST_MIN_STACK=268435456` and the
scratch tree's own `CARGO_TARGET_DIR` (rules 31, 33), via `ctx.run_command`.
"""
from __future__ import annotations

import os
import re
from pathlib import Path
from typing import Any, Iterable, Mapping, Sequence

from ..author_set import merge_wave as mw
from .driver_contracts import GateResult, ItemRecord, MergeResult, RunContext
from .merge import (cargo_env_for, ensure_worktree, git, is_engine_path, reset_worktree, rev,
                    scratch_name, worktree_root)

FIX_KEYS = ("files", "tests", "test_result_lines", "gaps", "citation", "notes")
CITATION_KINDS = ("rule", "ruling", "dcgo")

_RULE_REF = re.compile(r"^(?:general[_ ]rules?(?:\.pdf)?\s*[,:]?\s*)?§?\s*\d+(?:-\d+)+(?:\s+.*)?$",
                       re.IGNORECASE)
_RULING_REF = re.compile(r"^qa:Q\d+$")
# A C# file, bare or under any path -- including the ABSOLUTE base-repo path
# the prompts themselves hand the worker (`C:/.../DCGO/Assets/.../BT13_060.cs:218`
# failed a landed card fix in the second pilot) -- with an optional :line or :a-b.
_DCGO_REF = re.compile(r"^(?:[A-Za-z]:)?(?:[^\s:]+/)*[A-Za-z0-9_.-]+\.cs(?::\d+(?:-\d+)?)?$")
_RUST_PATH = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)*$")
_BINARY = re.compile(r"^[A-Za-z0-9_]+$")
_RESULT_LINE = re.compile(r"^test (\S+) \.\.\. (ok|FAILED|ignored)\b", re.MULTILINE)
_RUNNING = re.compile(r"^running \d+ tests?$", re.MULTILINE)
_TESTS_DIR_PREFIXES = ("code/digimon-engine/tests/", "tests/")
_TEST_NAME_IN_CARGO = re.compile(r'\[\[test\]\]\s*\n\s*name\s*=\s*"([^"]+)"')

REMEDY_CITATION = ("cite a general_rule.pdf section (e.g. 16-36), an official ruling (qa:Q1234) "
                   "or a DCGO file:line (BT21_029.cs:45); a fix that cannot cite a source is a "
                   "finding -- escalate the item")


# ---------------------------------------------------------------------------
# pure helpers
# ---------------------------------------------------------------------------


def fix_result(item: ItemRecord) -> dict | None:
    data = item.data or {}
    for key in ("fix", "fix_result"):
        if isinstance(data.get(key), Mapping):
            return dict(data[key])
    if all(k in data for k in ("citation", "tests")):
        return {k: data[k] for k in FIX_KEYS if k in data}
    return None


def citation_problem(citation: Any) -> str | None:
    """None when well-formed, else what is wrong."""
    if not isinstance(citation, Mapping) or not citation.get("kind") or not citation.get("ref"):
        return "citation missing"
    kind, ref = citation["kind"], str(citation["ref"]).strip()
    if kind not in CITATION_KINDS:
        return f"citation kind {kind!r} is not one of {', '.join(CITATION_KINDS)}"
    ok = {"rule": lambda r: bool(_RULE_REF.match(r)),
          "ruling": lambda r: bool(_RULING_REF.match(r)),
          "dcgo": lambda r: bool(_DCGO_REF.match(r.replace("\\", "/")))}[kind](ref)
    if not ok:
        shape = {"rule": "a general_rule.pdf section like 16-36", "ruling": "qa:Q<number>",
                 "dcgo": "<File>.cs:<line> or a C# path"}[kind]
        return f"citation malformed: {kind} ref {ref!r} is not {shape}"
    return None


def parse_test_name(name: str, binaries: Iterable[str] = ("cards_behavioral",)) -> tuple[str, str]:
    """(test binary, libtest filter) for a named test. Accepted forms:
    `<module>::<fn>` (cards_behavioral), `<binary>::<module>::<fn>`, and a file
    path `[code/digimon-engine/]tests/<binary>/<a>/<b>.rs[::<fn>]`. Anything
    that is not a plain Rust path is refused (it becomes a process argument)."""
    bins = set(binaries) | {"cards_behavioral"}
    raw = (name or "").strip()
    path = raw.replace("\\", "/")
    for prefix in _TESTS_DIR_PREFIXES:
        if path.startswith(prefix):
            file_part, _, fn = path[len(prefix):].partition(".rs")
            parts = file_part.split("/")
            if len(parts) < 2 or not _BINARY.match(parts[0]):
                raise ValueError(f"{name!r}: not a test file under tests/<binary>/")
            mods = [p for p in parts[1:] if p not in ("main", "mod")]
            tail = fn[2:] if fn.startswith("::") else fn
            if tail:
                mods.append(tail)
            filt = "::".join(mods)
            if not filt or not _RUST_PATH.match(filt):
                raise ValueError(f"{name!r}: cannot derive a test path from it")
            return parts[0], filt
    if not _RUST_PATH.match(raw):
        raise ValueError(f"{name!r} is not a Rust test path")
    head, sep, rest = raw.partition("::")
    if sep and head in bins and rest:
        return head, rest
    return "cards_behavioral", raw


def test_binaries(tree: str | os.PathLike) -> set[str]:
    """`[[test]]` names in the engine's Cargo.toml (plus cards_behavioral)."""
    try:
        text = (Path(tree) / mw.ENGINE_CARGO_MANIFEST).read_text(encoding="utf-8")
    except OSError:
        text = ""
    return set(_TEST_NAME_IN_CARGO.findall(text)) | {"cards_behavioral"}


def _matches(executed: str, filt: str) -> bool:
    return executed == filt or executed.endswith("::" + filt) or executed.startswith(filt + "::")


def classify(results: Mapping[str, str], filt: str) -> tuple[str, list[str]]:
    """(outcome, matched names) of one named test within a run's results.
    Outcomes: pass | fail | ignored | missing | ambiguous."""
    matched = sorted(t for t in results if _matches(t, filt))
    if not matched:
        return "missing", matched
    if "::" not in filt and len(matched) > 1:
        return "ambiguous", matched
    statuses = {results[t] for t in matched}
    if "FAILED" in statuses:
        return "fail", matched
    if "ignored" in statuses:
        return "ignored", matched
    return "pass", matched


# ---------------------------------------------------------------------------
# the gate
# ---------------------------------------------------------------------------


class FixGate:
    """`driver_contracts.Gate` for `fix_card` / `fix_engine` results."""

    def __init__(self, *, worktree_root: str | os.PathLike | None = None,
                 test_threads: int = mw.DEFAULT_TEST_THREADS, timeout: float = 3600.0,
                 scope_timeout: float = 300.0):
        self.worktree_root = worktree_root
        self.test_threads = test_threads
        self.timeout = timeout
        self.scope_timeout = scope_timeout

    def check(self, ctx: RunContext, item: ItemRecord, merge: MergeResult) -> GateResult:
        reasons: list[str] = []
        evidence: dict = {}
        fix = fix_result(item)
        if fix is None:
            return GateResult(False, [
                "no fix result in item.data['fix'] -- the fix stage must store the worker's "
                "validated fix_card/fix_engine result there"], evidence)
        evidence["worker_claims"] = list(fix.get("test_result_lines") or [])
        evidence["merge"] = {"ok": merge.ok, "sha": merge.sha, "branch": merge.branch,
                             "touched": list(merge.touched or [])}

        if not merge.ok or not merge.sha:
            reasons.append(f"the merge did not land ({'; '.join(merge.errors) or 'no sha'}) -- "
                           f"nothing to gate; re-run the fix")
        gaps = fix.get("gaps") or []
        if gaps:
            ids = ", ".join(str(g.get("id")) for g in gaps if isinstance(g, Mapping))
            reasons.append(f"the fix reports open gaps ({ids}) -- park the item on the gap "
                           f"instead of gating the fix")
        engine_paths = sorted(p for p in merge.touched or [] if is_engine_path(p))
        engine_prefix = f"card-loop/{ctx.run_id}/engine-"
        if engine_paths and not str(merge.branch or "").startswith(engine_prefix):
            reasons.append(f"engine code ({', '.join(engine_paths)}) landed on "
                           f"{merge.branch or 'an unknown branch'}, not an {engine_prefix}* branch "
                           f"(D13) -- re-merge it with engine=True")
        cite = citation_problem(fix.get("citation"))
        citation = fix.get("citation") if isinstance(fix.get("citation"), Mapping) else {}
        evidence["citation"] = {"kind": citation.get("kind"), "ref": citation.get("ref"),
                                "ok": cite is None}
        if cite:
            reasons.append(f"{cite} -- {REMEDY_CITATION}")
        names = [str(t) for t in fix.get("tests") or []]
        if not names:
            reasons.append("no regression test named -- name at least one test that fails "
                           "before the fix and passes after it")
        if reasons:
            evidence["skipped"] = "cheap checks failed; no test was run"
            return GateResult(False, reasons, evidence)

        root = worktree_root(ctx, self.worktree_root)
        wt = root / scratch_name(ctx, "gate")
        try:
            ensure_worktree(ctx.repo, wt, merge.sha)
            reset_worktree(wt, merge.sha)
        except RuntimeError as e:
            return GateResult(False, [f"gate worktree: {e} -- free or remove {wt}"], evidence)

        binaries = test_binaries(wt)
        targets: list[tuple[str, str, str]] = []
        for n in names:
            try:
                b, f = parse_test_name(n, binaries)
                targets.append((n, b, f))
            except ValueError as e:
                reasons.append(f"{e} -- name tests by their module path, e.g. "
                               f"bt21::bt21_029::declines_optional")
        evidence["tests"] = [{"name": n, "binary": b, "filter": f} for n, b, f in targets]
        if reasons:
            return GateResult(False, reasons, evidence)
        env = cargo_env_for(ctx, wt)

        # AFTER: at the merged sha
        after = self._run_named(ctx, wt, env, targets)
        evidence["after"] = after
        for n, _, _ in targets:
            r = after["results"][n]
            if r["status"] != "pass":
                reasons.append(self._after_reason(n, r, merge.sha))
        if reasons:
            evidence["before"] = {"skipped": "the tests do not pass with the fix"}
            return GateResult(False, reasons, evidence)

        # the scoped suite, still at the merged sha
        suite = self._scoped_suite(ctx, wt, env, merge)
        evidence["scoped_suite"] = suite
        if not suite.get("green"):
            last = (suite.get("commands") or [{}])[-1]
            reasons.append(f"scoped suite red ({last.get('command', '?')}, rc {last.get('rc')}) -- "
                           f"the fix regresses other tests; fix the regression or revise the fix")

        # BEFORE: the fix's non-test paths restored to the parent
        before = self._run_before(ctx, wt, env, targets, merge)
        evidence["before"] = before
        for n, _, _ in targets:
            r = before["results"].get(n, {"status": "missing"})
            if r["status"] not in ("red", "compile_error"):
                reasons.append(self._before_reason(n, r, before.get("at") or "the parent"))
        return GateResult(not reasons, reasons, evidence)

    # ------------------------------------------------------------------ runs

    def _run_named(self, ctx, wt: Path, env: dict, targets) -> dict:
        runs, results = [], {}
        by_binary: dict[str, list[tuple[str, str]]] = {}
        for n, b, f in targets:
            by_binary.setdefault(b, []).append((n, f))
        for binary, items in by_binary.items():
            filters = list(dict.fromkeys(f for _, f in items))
            argv = mw.with_env(env, mw.cargo_test_argv(binary, filters,
                                                       test_threads=self.test_threads))
            rec = mw.run_logged(ctx.run_command, argv, str(wt), self.timeout)
            executed = {m.group(1): m.group(2) for m in _RESULT_LINE.finditer(rec["stdout"])}
            compiled = bool(_RUNNING.search(rec["stdout"]))
            runs.append({**mw.strip_output(rec), "compiled": compiled})
            for n, f in items:
                outcome, matched = classify(executed, f)
                if not compiled and rec["rc"] != 0:
                    outcome = "compile_error"
                elif outcome == "missing" and rec["rc"] != 0:
                    outcome = "inconclusive"
                results[n] = {"status": outcome, "matched": matched,
                              "outcomes": {t: executed[t] for t in matched}}
        return {"runs": runs, "results": results}

    def _scoped_suite(self, ctx, wt: Path, env: dict, merge: MergeResult) -> dict:
        scope = dict(merge.scope or {})
        prior = scope.get("suite")
        if isinstance(prior, Mapping) and prior.get("green") is not None:
            return {"reused": True, "green": bool(prior.get("green")),
                    "commands": list(prior.get("commands") or [])}
        touched = list(merge.touched or [])
        out: dict = {"reused": False}
        if not any(k in scope for k in ("full_suite_required", "cards", "side_binaries")):
            scope, out["impact_scope"] = mw.compute_scope(ctx.run_command, str(wt), touched,
                                                          timeout=self.scope_timeout)
        suite = mw.run_scoped_suite(ctx.run_command, str(wt), scope, touched, env=env,
                                    timeout=self.timeout, test_threads=self.test_threads)
        out.update(green=suite["green"], ran=suite["ran"], commands=suite["commands"],
                   scope={k: v for k, v in scope.items() if k != "suite"})
        return out

    def _run_before(self, ctx, wt: Path, env: dict, targets, merge: MergeResult) -> dict:
        parent = rev(wt, f"{merge.sha}^")
        if parent is None:
            return {"at": None, "reverted": [], "results": {},
                    "error": f"{merge.sha} has no parent to compare against"}
        fix_paths = sorted(p for p in (merge.touched or [])
                           if not mw.is_test_path(p) and mw.path_problem(p, None) is None)
        restore = [p for p in fix_paths
                   if git(wt, "cat-file", "-e", f"{parent}:{p}").returncode == 0]
        added = [p for p in fix_paths if p not in restore]
        if restore:
            git(wt, "checkout", "-q", parent, "--", *restore, check=True)
        for p in added:
            git(wt, "rm", "-q", "-f", "--ignore-unmatch", "--", p, check=True)
        run = self._run_named(ctx, wt, env, targets)
        results = {}
        for n, r in run["results"].items():
            status = {"fail": "red", "pass": "green"}.get(r["status"], r["status"])
            results[n] = {**r, "status": status}
        reset_worktree(wt, merge.sha)
        return {"at": parent, "reverted": fix_paths, "runs": run["runs"], "results": results}

    # ------------------------------------------------------------------ reasons

    @staticmethod
    def _after_reason(name: str, r: Mapping, sha: str) -> str:
        status = r["status"]
        if status == "missing":
            return (f"{name}: no test matched in the merged tree -- name it by its full module "
                    f"path (e.g. bt21::bt21_029::declines) and make sure its file is registered")
        if status == "ambiguous":
            return (f"{name!r} is ambiguous: it matches {len(r['matched'])} tests "
                    f"({', '.join(r['matched'])}) -- give its full module path")
        if status == "ignored":
            return f"{name}: the test is #[ignore]d -- an ignored test proves nothing"
        if status == "compile_error":
            return (f"{name}: the merged tree at {sha[:12]} does not compile -- the fix is "
                    f"incomplete")
        if status == "inconclusive":
            return (f"{name}: the run aborted before reporting it (rule 33 environmental abort?) "
                    f"-- rerun the gate")
        return f"{name}: fails with the fix (at {sha[:12]}) -- the fix does not make it pass"

    @staticmethod
    def _before_reason(name: str, r: Mapping, parent: str) -> str:
        status = r["status"]
        if status == "green":
            return (f"{name}: passes without the fix (non-test changes reverted to "
                    f"{parent[:12]}) -- it does not reproduce the divergence; write a test that "
                    f"is red without the fix")
        if status == "inconclusive":
            return (f"{name}: the before-run aborted before reporting it (rule 33 environmental "
                    f"abort?) -- rerun the gate")
        if status == "ignored":
            return f"{name}: the test is #[ignore]d -- an ignored test proves nothing"
        return f"{name}: could not establish that it fails without the fix ({status}) -- rerun"
