"""`dcgo-harness` from the driver: the binary, the argv shapes, the output parsers,
the deck book for a scenario, and the prompt-mismatch router (design D4,
spec "Prompt-sequence failures are routed by who disagreed").

Every call goes through `ctx.run_command(argv, cwd, timeout) -> (rc, stdout,
stderr)` with `cwd = ctx.repo`, so all paths in argv are repo-relative:

    sim-only   <bin> [--root R] exam --scenario S --sim-only --cards-json data/cards.json
                     [--decks B] [--inspect N]
    oracle     <bin> [--root R] exam --oracle [--build P] --scenario S
                     --cards-json data/cards.json [--decks B] --verdicts
                     --clause-text-json C --backfill
                     [--interaction-denominator data/interaction_denominator.json]
                     --oracle-timeout T
    triage     <bin> verdict-triage --clause ID --triage CLASS --citation REF

`--decks` is omitted for the harness's own default book (`data/starter_decks.json`),
exactly as `tools/exam_sim_all.py` runs it.
"""
from __future__ import annotations

import glob
import json
import os
import re
import shutil
import subprocess
from dataclasses import dataclass, field
from pathlib import Path
from typing import Mapping, Sequence

from tools import exam_sim_all

CARDS_JSON = "data/cards.json"
INTERACTION_DENOMINATOR = "data/interaction_denominator.json"
SIM_TIMEOUT_S = 300.0
ORACLE_TIMEOUT_S = 300
VERDICT_TIMEOUT_S = 120.0

# --------------------------------------------------------------------------- binary


def _exe(name: str) -> str:
    return name + (".exe" if os.name == "nt" else "")


def harness_candidates(repo: str | os.PathLike, *, cargo_target_base: str | None = None,
                       env: Mapping[str, str] | None = None) -> list[Path]:
    """Where a built `dcgo-harness` may live, most specific first."""
    env = os.environ if env is None else env
    repo = Path(repo)
    exe = _exe("dcgo-harness")
    out: list[Path] = []
    if env.get("CARGO_TARGET_DIR"):
        out += [Path(env["CARGO_TARGET_DIR"]) / "debug" / exe, Path(env["CARGO_TARGET_DIR"]) / "release" / exe]
    out += [repo / "target" / "debug" / exe, repo / "target" / "release" / exe]
    bases = [b for b in (cargo_target_base, env.get("CARGO_TARGET_BASE"), r"D:\cargo-target") if b]
    names = [repo.name, Path(__file__).resolve().parents[4].name]
    for base in dict.fromkeys(bases):
        for name in dict.fromkeys(names):
            out += [Path(base) / name / "debug" / exe, Path(base) / name / "release" / exe]
    return list(dict.fromkeys(out))


def resolve_harness_bin(repo: str | os.PathLike, *, cargo_target_base: str | None = None,
                        env: Mapping[str, str] | None = None) -> str:
    """`DCGO_HARNESS_BIN`, else the first built binary found (per-worktree
    `D:/cargo-target/<worktree>/debug/dcgo-harness.exe`, `target/debug`, ...),
    else `dcgo-harness` on PATH."""
    env = os.environ if env is None else env
    if env.get("DCGO_HARNESS_BIN"):
        return env["DCGO_HARNESS_BIN"]
    for p in harness_candidates(repo, cargo_target_base=cargo_target_base, env=env):
        if p.is_file():
            return str(p)
    return shutil.which("dcgo-harness") or "dcgo-harness"


def harness_bin(ctx) -> str:
    cfg = getattr(ctx, "config", None)
    explicit = getattr(ctx, "harness_bin", None) or getattr(cfg, "harness_bin", None)
    if explicit:
        return str(explicit)
    return resolve_harness_bin(ctx.repo, cargo_target_base=getattr(cfg, "cargo_target_base", None))


def _root_args(ctx) -> list[str]:
    root = getattr(getattr(ctx, "config", None), "harness_root", None)
    return ["--root", str(root)] if root else []


# --------------------------------------------------------------------------- deck books


def load_deck_books(repo: str | os.PathLike) -> list[tuple[str, set]]:
    """`exam_sim_all.load_books`, rooted at `repo`: every pool JSON under
    `qa/dcgo-exams/` (absolute paths) plus the harness default book under its
    relative name `exam_sim_all.DEFAULT_BOOK`, last."""
    repo = Path(repo)
    books: list[tuple[str, set]] = []
    root = repo / "qa" / "dcgo-exams"
    for path in sorted(glob.glob(os.path.join(str(root), "**", "*.json"), recursive=True)):
        try:
            data = json.loads(Path(path).read_text(encoding="utf-8"))
        except (OSError, ValueError):
            continue
        if isinstance(data, dict) and isinstance(data.get("decks"), list):
            books.append((os.path.normpath(path),
                          {d.get("name") for d in data["decks"] if isinstance(d, dict)}))
    starter = repo / exam_sim_all.DEFAULT_BOOK
    if starter.is_file():
        data = json.loads(starter.read_text(encoding="utf-8"))
        books.append((exam_sim_all.DEFAULT_BOOK,
                      {d.get("id") or d.get("name") for d in data.get("starter_decks", [])}))
    return books


def deck_books_for(repo: str | os.PathLike, scenario: str) -> list[str]:
    """The books `exam_sim_all.candidates()` would try for `scenario`, best
    first, as repo-relative POSIX paths (the default book under its own name)."""
    repo = Path(repo)
    abs_scenario = os.path.normpath(str(repo / scenario))
    picked = exam_sim_all.candidates(abs_scenario, load_deck_books(repo))
    out = []
    for p in picked:
        if p == exam_sim_all.DEFAULT_BOOK:
            out.append(p)
        else:
            out.append(Path(os.path.relpath(p, repo)).as_posix())
    return out


def _decks_args(book: str | None) -> list[str]:
    if not book or book == exam_sim_all.DEFAULT_BOOK:
        return []
    return ["--decks", book]


# --------------------------------------------------------------------------- argv


def sim_argv(ctx, scenario: str, book: str | None, *, inspect: int | None = None) -> list[str]:
    argv = [harness_bin(ctx), *_root_args(ctx), "exam", "--scenario", scenario, "--sim-only",
            "--cards-json", CARDS_JSON, *_decks_args(book)]
    if inspect is not None:
        argv += ["--inspect", str(inspect)]
    return argv


def oracle_argv(ctx, scenario: str, book: str | None, *, clause_text_json: str) -> list[str]:
    cfg = getattr(ctx, "config", None)
    argv = [harness_bin(ctx), *_root_args(ctx), "exam", "--oracle"]
    if getattr(cfg, "player_dir", None):
        argv += ["--build", str(cfg.player_dir)]
    argv += ["--scenario", scenario, "--cards-json", CARDS_JSON, *_decks_args(book),
             "--verdicts", "--clause-text-json", clause_text_json, "--backfill"]
    if (Path(ctx.repo) / INTERACTION_DENOMINATOR).is_file():
        argv += ["--interaction-denominator", INTERACTION_DENOMINATOR]
    argv += ["--oracle-timeout", str(oracle_timeout(ctx))]
    return argv


def oracle_timeout(ctx) -> int:
    return int(getattr(getattr(ctx, "config", None), "oracle_timeout_s", None) or ORACLE_TIMEOUT_S)


# --------------------------------------------------------------------------- the player

NODE_TIMEOUT_S = 180.0


def heartbeat_job(ctx) -> str | None:
    """What the player's heartbeat names (`harness.heartbeat` under the harness
    root): a job id, `idle`, or None without a readable heartbeat."""
    root = getattr(getattr(ctx, "config", None), "harness_root", None)
    if not root:
        return None
    try:
        text = (Path(root) / "harness.heartbeat").read_text(encoding="utf-8").strip()
    except OSError:
        return None
    return text or None


def launch_detached(argv: Sequence[str], cwd: str, timeout: float) -> int | None:
    """Run a launcher whose child must outlive it (`node up` spawns the Unity
    player). No pipes: a captured stdout is inherited by the player (Rust's
    spawn on Windows inherits every inheritable handle), the launcher's EOF
    never comes, and the caller blocks until its timeout. Exit code, 124 on a
    timeout, None when the launcher could not start."""
    try:
        return subprocess.run(list(argv), cwd=cwd, stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
                              stderr=subprocess.DEVNULL, close_fds=True, timeout=timeout).returncode
    except OSError:
        return None
    except subprocess.TimeoutExpired:
        return 124


def restart_player(ctx) -> str:
    """`node down`, then `node up --build <player_dir>`, then `node status` to
    confirm: the only cure for a player wedged on a job (DCGO enforces no job
    timeout). Returns a note for the evidence; a failure is reported, never
    raised. `node up` goes through `ctx.launch_detached` when the context has
    one (tests), else `launch_detached`."""
    build = getattr(getattr(ctx, "config", None), "player_dir", None)
    if not build:
        return "not restarted: no player_dir configured"
    root = _root_args(ctx)
    rc, out, err = run(ctx, [harness_bin(ctx), *root, "node", "down"], NODE_TIMEOUT_S)
    if rc != 0:
        return f"restart failed at `node down` (exit {rc}): {(err or out).strip()[-300:]}"
    launcher = getattr(ctx, "launch_detached", None) or launch_detached
    rc = launcher([harness_bin(ctx), *root, "node", "up", "--build", str(build)], str(ctx.repo), NODE_TIMEOUT_S)
    if rc != 0:
        return f"restart failed at `node up` (exit {rc})"
    rc, out, err = run(ctx, [harness_bin(ctx), *root, "node", "status"], NODE_TIMEOUT_S)
    status = next((l.strip() for l in (out or "").splitlines() if "player:" in l), "")
    if "player: running" not in status:          # `[fail] player: not running` also says "running"
        return f"restart: `node up` returned 0 but `node status` says {status or (err or out).strip()[-200:]!r}"
    return f"restarted the player (`node down`, then `node up`; {status})"


def verdict_triage_argv(ctx, clause_id: str, triage: str, citation: str) -> list[str]:
    return [harness_bin(ctx), "verdict-triage", "--clause", clause_id, "--triage", triage,
            "--citation", citation]


def verdict_set_argv(ctx, *, verdict: str, reason: str, clause: str | None = None,
                     interaction: str | None = None, triage: str | None = None,
                     citation: str | None = None) -> list[str]:
    """`dcgo-harness verdict-set`: an ending the exam cannot produce (an
    unreachable clause, an interaction ending), with its triage."""
    if (clause is None) == (interaction is None):
        raise ValueError("verdict-set takes exactly one of clause / interaction")
    argv = [harness_bin(ctx), "verdict-set"]
    argv += ["--clause", clause] if clause is not None else ["--interaction", interaction]
    argv += ["--verdict", verdict, "--reason", reason]
    if triage:
        argv += ["--triage", triage]
        if citation:
            argv += ["--citation", citation]
    return argv


def run(ctx, argv: Sequence[str], timeout: float) -> tuple[int | None, str, str]:
    """`ctx.run_command` in the run's tree; a binary that cannot start is rc None."""
    try:
        rc, out, err = ctx.run_command(list(argv), str(ctx.repo), timeout)
    except OSError as e:
        return None, "", f"{argv[0]} could not be started: {e}"
    return rc, out or "", err or ""


# --------------------------------------------------------------------------- sim-only


@dataclass
class SimReport:
    passed: bool
    rc: int | None
    failures: list = field(default_factory=list)      # FAILED / ASSERT FAILED / stall / CONTRADICT lines
    notes: list = field(default_factory=list)          # `note:` lines
    summary: str | None = None                         # `exam: scenarios seen ...`
    assert_line: str | None = None                     # `assert: N check(s) ... M failed`
    line_ok: bool = False                              # the line lowered and ran clean (ruling aside)
    ruling_contradicted: bool = False                  # `RULING <q> CONTRADICTED` -- our engine vs the publisher

    def failure_text(self) -> str:
        lines = list(self.failures) or ([self.summary] if self.summary else [])
        return "\n".join(lines) if lines else f"sim-only exited {self.rc} with no failure line"

    def ruling_lines(self) -> list[str]:
        return [l for l in self.failures if "CONTRADICT" in l]


def parse_sim_output(rc: int | None, stdout: str, stderr: str = "") -> SimReport:
    """Pass = exit 0 and `/ failed 0` in the summary (`exam_sim_all.run_one`'s rule)."""
    text = (stdout or "") + "\n" + (stderr or "")
    failures, notes = [], []
    summary = assert_line = None
    for raw in text.splitlines():
        line = raw.strip()
        if not line:
            continue
        if line.startswith("exam: scenarios seen"):
            summary = line
        elif line.startswith("assert:"):
            assert_line = line
        elif line.startswith("note:"):
            notes.append(line[len("note:"):].strip())
        elif ("FAILED" in line or "CONTRADICT" in line or "did not run to completion" in line
              or line.startswith("Error")):
            # `RULING <q> CONTRADICTED: at N: ...` is how the harness reports an
            # `expect_ruling:` our engine does not meet -- exit 0, no FAILED
            # line. Without it the author re-authors blind (first pilot, Q2304).
            failures.append(line)
    passed = rc == 0 and summary is not None and "/ failed 0" in summary
    if rc is None:
        failures.append((stderr or "").strip() or "harness did not start")
    ruling = [l for l in failures if "CONTRADICT" in l]
    line_ok = rc == 0 and summary is not None and len(ruling) == len(failures)
    return SimReport(passed=passed, rc=rc, failures=failures, notes=notes, summary=summary,
                     assert_line=assert_line, line_ok=line_ok, ruling_contradicted=bool(ruling))


def parse_inspect(stdout: str) -> dict | None:
    """The pretty-printed `{snapshot, projection, complete, steps_run}` object
    `--inspect N` prints after the `lowered` line, or None."""
    lines = (stdout or "").splitlines()
    for i, line in enumerate(lines):
        if line.strip() != "{":
            continue
        dec = json.JSONDecoder()
        try:
            obj, _ = dec.raw_decode("\n".join(lines[i:]))
        except ValueError:
            continue
        if isinstance(obj, dict) and "snapshot" in obj:
            return obj
    return None


def inspect_error(stdout: str) -> str | None:
    for line in (stdout or "").splitlines():
        if line.strip().startswith("inspect:"):
            return line.strip()
    return None


# --------------------------------------------------------------------------- oracle


def parse_oracle_output(rc: int | None, stdout: str, stderr: str = "") -> list[dict]:
    """The JSON lines `exam --oracle` printed (one per scenario). Lines that do
    not start with `{` are notes and are skipped."""
    out = []
    for line in (stdout or "").splitlines():
        s = line.strip()
        if not s.startswith("{"):
            continue
        try:
            obj = json.loads(s)
        except ValueError:
            continue
        if isinstance(obj, dict):
            out.append(obj)
    return out


_MISMATCH = re.compile(r"prompt mismatch: step (\d+) expected prompt '([^']*)' but DCGO asked '([^']*)'")


@dataclass(frozen=True)
class PromptMismatch:
    row: int            # DCGO's scripted-input cursor (a lowered row, not a scenario step)
    expected: str       # the scenario's (or the emitter's) expect_prompt
    asked: str          # what DCGO asked


def prompt_mismatch(result: Mapping) -> PromptMismatch | None:
    """A DCGO-filed `failed` job whose message is a prompt mismatch."""
    if result.get("job_outcome") != "failed":
        return None
    m = _MISMATCH.search(str(result.get("reason") or ""))
    if not m:
        return None
    return PromptMismatch(row=int(m.group(1)), expected=m.group(2), asked=m.group(3))


_ACTOR = re.compile(r"prompt mismatch: step (\d+) expected actor (\d) but DCGO asked actor (\d)")


def actor_mismatch(result: Mapping) -> tuple[int, int] | None:
    """`(expected actor, actor DCGO asked)` from a failed job's actor mismatch."""
    if result.get("job_outcome") != "failed":
        return None
    m = _ACTOR.search(str(result.get("reason") or ""))
    return (int(m.group(2)), int(m.group(3))) if m else None


@dataclass(frozen=True)
class CandidateMismatch:
    """DCGO's prompt matched but offered other candidates than the pick the
    scenario (sim-only clean in our engine) makes: the engines disagree on
    what that selection offers."""
    prompt: str
    wanted: str
    pick: int
    wanted_list: list
    offered: list


# InputDriver.cs: `<Prompt>: wanted card 'X' (pick i of [..]) is not among the offered candidates [..]`
_CANDIDATES = re.compile(r"(\w+): wanted card '([^']*)' \(pick (\d+) of \[([^\]]*)\]\) is not among the "
                         r"offered candidates \[([^\]]*)\]")


def candidate_mismatch(result: Mapping) -> CandidateMismatch | None:
    if result.get("job_outcome") != "failed":
        return None
    m = _CANDIDATES.search(str(result.get("reason") or ""))
    if not m:
        return None

    def split(s: str) -> list:
        return [x.strip() for x in s.split(",") if x.strip()]

    return CandidateMismatch(prompt=m.group(1), wanted=m.group(2), pick=int(m.group(3)),
                             wanted_list=split(m.group(4)), offered=split(m.group(5)))


# DCGO's closed prompt vocabulary (InputDriver.cs); these three take an action id
# and our engine has no pending selection there.
ACTION_PROMPTS = ("main_phase", "breeding_action", "mulligan")
ACTION = "<action>"

# adapter.rs `dcgo_prompt_name`: the unconditional SelectionKind -> DCGO prompt
# class rows. Every other kind maps conditionally or not at all (None).
_KIND_TO_PROMPT = {
    "Hand": "SelectHandEffect",
    "OwnField": "SelectPermanentEffect",
    "OppField": "SelectPermanentEffect",
    "AnyField": "SelectPermanentEffect",
    "Trash": "SelectCardEffect",
    "Reveal": "SelectCardEffect",
    "RevealBucket": "SelectCardEffect",
    "Security": "SelectCardEffect",
    "OrderedPermutation": "SelectCardEffect",
    "Replacement": "OptionalSkill",
    "DpBudget": "SelectPermanentEffect",
}
# adapter.rs `kind_is_pick_shaped`: DCGO may fold an OptionalSkill gate into these.
_PICK_SHAPED = ("Hand", "OwnField", "OppField", "AnyField", "Trash", "Reveal", "Material")


def _kind_name(kind: str | None) -> str | None:
    if not kind:
        return None
    m = re.match(r"[A-Za-z_]+", kind)
    return m.group(0) if m else None


def ours_as_dcgo(snapshot: Mapping) -> str | None:
    """Our live prompt BEFORE a step, in DCGO's vocabulary: `ACTION` when no
    selection is pending (an action prompt), a DCGO class when the mapping is
    unambiguous, else None."""
    kind = _kind_name(snapshot.get("pending_kind"))
    if kind is None:
        return ACTION
    if kind == "TriggerOrder":
        n = len(snapshot.get("candidates") or [])
        if n >= 2:
            return "MultipleSkills"
        if n == 1 and snapshot.get("pending_optional"):
            return "OptionalSkill"
        return None
    return _KIND_TO_PROMPT.get(kind)


def _matches(ours: str | None, prompt: str, snapshot: Mapping) -> bool | None:
    """Does our prompt match DCGO prompt name `prompt`? None = cannot tell."""
    if prompt in ACTION_PROMPTS:
        return ours == ACTION
    if ours == ACTION:
        return False
    kind = _kind_name(snapshot.get("pending_kind"))
    if prompt == "OptionalSkill" and kind in _PICK_SHAPED and snapshot.get("pending_optional"):
        return True          # the declared OptionalSkill+pick fold (adapter.rs)
    if ours is None:
        return None
    return ours == prompt


@dataclass(frozen=True)
class PromptRoute:
    route: str           # scenario_wrong | engines_disagree | undetermined
    ours: str | None     # our prompt in DCGO vocabulary (ACTION for an action prompt)
    explanation: str


def decide_prompt_route(mismatch: PromptMismatch, snapshot: Mapping | None) -> PromptRoute:
    """Who disagreed with whom (D4).

    * our engine also contradicts the scenario's `expect` -> `scenario_wrong`
      (back to AUTHORING with the observed sequence);
    * our engine matches the scenario and DCGO does not -> `engines_disagree`
      (TRIAGE: a missing decline looks exactly like this);
    * we cannot map our prompt onto DCGO's vocabulary -> `undetermined`
      (TRIAGE, with the evidence: a model reads both sides).
    """
    if snapshot is None:
        return PromptRoute("undetermined", None, "our engine's prompt at that step could not be inspected")
    ours = ours_as_dcgo(snapshot)
    shown = "an action prompt (no pending selection)" if ours == ACTION else (
        f"{snapshot.get('pending_kind')} ({ours or 'no unambiguous DCGO prompt'})")
    m = _matches(ours, mismatch.expected, snapshot)
    if m is True:
        return PromptRoute("engines_disagree", ours,
                           f"the scenario expected '{mismatch.expected}' and our engine asked {shown}, "
                           f"but DCGO asked '{mismatch.asked}'")
    if m is False:
        return PromptRoute("scenario_wrong", ours,
                           f"the scenario expected '{mismatch.expected}' but our engine asked {shown} "
                           f"and DCGO asked '{mismatch.asked}'")
    return PromptRoute("undetermined", ours,
                       f"the scenario expected '{mismatch.expected}', DCGO asked '{mismatch.asked}', and "
                       f"our engine's {shown} has no unambiguous DCGO prompt mapping")


def scenario_step_for_row(steps: Sequence[Mapping], mismatch: PromptMismatch) -> tuple[int, str]:
    """Map DCGO's input-row cursor back to a scenario step.

    A scenario step lowers to zero or more DCGO rows and only a step's FIRST
    row carries its `expect.prompt`, so the row index is not the step index.
    Prefer the steps whose `expect.prompt` is the expected prompt (nearest to
    the row); otherwise the row index clamped to the line. Returns (step, how).
    """
    if not steps:
        return 0, "empty"
    want = [i for i, s in enumerate(steps)
            if isinstance(s, Mapping) and isinstance(s.get("expect"), Mapping)
            and s["expect"].get("prompt") == mismatch.expected]
    if len(want) == 1:
        return want[0], "expect"
    if want:
        best = min(want, key=lambda i: (abs(i - mismatch.row), i))
        return best, "expect-nearest"
    return min(mismatch.row, len(steps) - 1), "row"
