"""LoopMerger (add-card-authoring-loop tasks 6.5, 6.7; design D13): worker diffs
merge one at a time through merge_wave semantics; card fixes commit onto the run
branch, engine fixes onto their own `card-loop/<run>/engine-<gap>` branch in a
separate worktree, never touching the run branch."""
import json
import subprocess
import sys
from pathlib import Path
from types import SimpleNamespace

import pytest

from tools.author_set import merge_wave as mw
from tools.card_loop.driver_contracts import MergeRequest
from tools.card_loop.gaps import RunGapLane
from tools.card_loop.merge import LoopMerger, engine_branch_name, is_engine_path
from tools.card_loop.workers.base import capture_artifacts

ENG = "code/digimon-engine"
CB = f"{ENG}/tests/cards_behavioral"
RUN_BRANCH = "card-loop/run1/run"
BASE_FILES = {
    f"{ENG}/Cargo.toml": b'[[test]]\nname = "cards_behavioral"\npath = "tests/cards_behavioral/main.rs"\n',
    f"{ENG}/src/lib.rs": b"pub fn rule() -> u8 { 1 }\n",
    f"{CB}/main.rs": b"mod bt21;\n",
    f"{CB}/bt21/mod.rs": b"mod bt21_001;\n",
    f"{CB}/bt21/bt21_001.rs": b"#[test]\nfn plays() {}\n",
    f"{ENG}/cards/bt21/BT21-001.yaml": b"id: BT21-001\n",
    "qa/dsl-vocab-gaps.md": b"# DSL\n",
}
CARD_029 = {
    f"{ENG}/cards/bt21/BT21-029.yaml": b"id: BT21-029\n",
    f"{CB}/bt21/bt21_029.rs": b"#[test]\nfn declines() {}\n",
    f"{CB}/bt21/mod.rs": b"mod bt21_001;\nmod bt21_029;\n",
}
ENGINE_FIX = {
    f"{ENG}/src/lib.rs": b"pub fn rule() -> u8 { 2 }\n",
    f"{CB}/bt21/bt21_030.rs": b"#[test]\nfn rule_is_two() {}\n",
    f"{CB}/bt21/mod.rs": b"mod bt21_001;\nmod bt21_030;\n",
}


def git(cwd, *args):
    return subprocess.run(["git", *args], cwd=cwd, check=True, capture_output=True,
                          text=True, encoding="utf-8").stdout.strip()


@pytest.fixture
def gitenv(tmp_path, monkeypatch):
    cfg = tmp_path / "gitconfig"
    cfg.write_text("[user]\n\tname = t\n\temail = t@example.invalid\n[core]\n\tautocrlf = false\n",
                   encoding="utf-8")
    monkeypatch.setenv("GIT_CONFIG_GLOBAL", str(cfg))
    monkeypatch.setenv("GIT_CONFIG_NOSYSTEM", "1")


def make_repo(tmp_path, branch=RUN_BRANCH):
    root = tmp_path / "repo"
    root.mkdir()
    git(root, "init", "-q", "-b", branch)
    for p, data in BASE_FILES.items():
        (root / p).parent.mkdir(parents=True, exist_ok=True)
        (root / p).write_bytes(data)
    git(root, "add", "-A")
    git(root, "commit", "-qm", "base")
    return root, git(root, "rev-parse", "HEAD")


@pytest.fixture
def repo(tmp_path, gitenv):
    return make_repo(tmp_path)


class FakeRunner:
    """`RunContext.run_command`: records (env, bare argv, cwd); impact_scope
    answers `scope`; a cargo argv matching a `fail` predicate exits 101."""

    def __init__(self, scope=None, fail=()):
        self.scope = scope if scope is not None else {"full_suite_required": False,
                                                      "cards": ["BT21-029"], "side_binaries": []}
        self.fail = list(fail)
        self.calls = []

    def __call__(self, argv, cwd, timeout):
        env, bare = mw.split_env(argv)
        self.calls.append({"env": env, "argv": bare, "cwd": str(cwd)})
        if len(bare) > 1 and bare[1] == mw.IMPACT_SCOPE_SCRIPT:
            return 0, json.dumps(self.scope), ""
        for pred in self.fail:
            if pred(bare):
                return 101, "test bt21::bt21_029::declines ... FAILED\ntest result: FAILED. 0 passed; 1 failed", ""
        return 0, "test result: ok. 1 passed; 0 failed", ""

    def cargo(self):
        return [c for c in self.calls if c["argv"][:1] == ["cargo"]]


def ctx_for(root, base, tmp_path, runner):
    return SimpleNamespace(run_id="run1", run_dir=str(tmp_path / "run"), repo=str(root),
                           base_sha=base, plan={}, pool=None, run_command=runner,
                           config=SimpleNamespace(cargo_target_base=str(tmp_path / "tgt")))


def artifacts(root, base, tmp_path, name, edits):
    wt = tmp_path / f"wt-{name}"
    git(root, "worktree", "add", "-q", "--detach", str(wt), base)
    for p, data in edits.items():
        (wt / p).parent.mkdir(parents=True, exist_ok=True)
        (wt / p).write_bytes(data)
    return capture_artifacts(wt, base, tmp_path / f"art-{name}")


def request(art, *, engine=False, gap_id=None, attempt="20261006T120000Z-run1-a1"):
    return MergeRequest(attempt_id=attempt, family="claude", model="sonnet", artifacts=art,
                        engine=engine, gap_id=gap_id, subject="")


def merger(tmp_path, **kw):
    return LoopMerger(worktree_root=str(tmp_path / "wt"), **kw)


# --- card merges onto the run branch -------------------------------------------------------


def test_card_merge_commits_on_the_run_branch(repo, tmp_path):
    root, base = repo
    (root / "scratch.txt").write_text("driver scratch\n")          # someone else's dirty file
    runner = FakeRunner()
    ctx = ctx_for(root, base, tmp_path, runner)

    res = merger(tmp_path).merge(ctx, request(artifacts(root, base, tmp_path, "a", CARD_029)))

    assert res.ok, res.errors
    assert res.sha == git(root, "rev-parse", "HEAD") and res.branch == RUN_BRANCH
    assert res.touched == sorted(CARD_029)
    msg = git(root, "log", "-1", "--format=%B")
    assert "Loop-Attempt: 20261006T120000Z-run1-a1" in msg and "Loop-Model: claude/sonnet" in msg
    assert "Loop-Run: run1" in msg
    assert sorted(git(root, "show", "--name-only", "--format=", "HEAD").splitlines()) == sorted(CARD_029)
    assert git(root, "status", "--porcelain") == "?? scratch.txt"

    pack, test = runner.cargo()
    assert pack["argv"] == ["cargo", "build", "-p", "digimon-engine"]
    assert pack["env"] == {"RUST_MIN_STACK": "268435456"} and pack["cwd"] == str(root)
    assert test["argv"] == ["cargo", "test", "--manifest-path", f"{ENG}/Cargo.toml", "--test",
                            "cards_behavioral", "--", "bt21_029", "bt21::bt21_029", "bt21::",
                            "--test-threads=8"]
    impact = [c for c in runner.calls if c["argv"][1:2] == [mw.IMPACT_SCOPE_SCRIPT]][0]
    assert impact["argv"][:3] == [sys.executable, mw.IMPACT_SCOPE_SCRIPT, "--json"]
    assert res.scope["cards"] == ["BT21-029"] and res.scope["suite"]["green"] is True
    assert res.scope["suite"]["commands"][0]["command"].startswith("RUST_MIN_STACK=268435456 cargo test")


def test_red_suite_leaves_the_tree_as_it_was(repo, tmp_path):
    root, base = repo
    (root / "scratch.txt").write_text("driver scratch\n")
    runner = FakeRunner(fail=[lambda a: a[:2] == ["cargo", "test"]])
    ctx = ctx_for(root, base, tmp_path, runner)

    res = merger(tmp_path).merge(ctx, request(artifacts(root, base, tmp_path, "a", CARD_029)))

    assert not res.ok and res.sha is None
    assert any("scoped suite failed" in e for e in res.errors)
    assert git(root, "rev-parse", "HEAD") == base
    assert git(root, "status", "--porcelain") == "?? scratch.txt"


def test_full_suite_scope_runs_the_whole_binary(repo, tmp_path):
    root, base = repo
    runner = FakeRunner(scope={"full_suite_required": True, "cards": [], "side_binaries": ["dsl"]})
    res = merger(tmp_path).merge(ctx_for(root, base, tmp_path, runner),
                                 request(artifacts(root, base, tmp_path, "a", CARD_029)))
    assert res.ok
    tests = [c["argv"] for c in runner.cargo() if c["argv"][1] == "test"]
    assert tests == [["cargo", "test", "--manifest-path", f"{ENG}/Cargo.toml", "--test",
                      "cards_behavioral", "--", "--test-threads=8"],
                     ["cargo", "test", "--manifest-path", f"{ENG}/Cargo.toml", "--test", "dsl",
                      "--", "--test-threads=8"]]


def test_remerging_the_same_attempt_is_a_noop(repo, tmp_path):
    root, base = repo
    art = artifacts(root, base, tmp_path, "a", CARD_029)
    first = merger(tmp_path).merge(ctx_for(root, base, tmp_path, FakeRunner()), request(art))
    runner = FakeRunner()

    again = merger(tmp_path).merge(ctx_for(root, base, tmp_path, runner), request(art))

    assert again.ok and again.sha == first.sha and runner.cargo() == []
    assert git(root, "rev-parse", "HEAD") == first.sha


def test_dead_module_is_refused_and_rolled_back(repo, tmp_path):
    root, base = repo
    edits = {k: v for k, v in CARD_029.items() if not k.endswith("mod.rs")}
    runner = FakeRunner()
    res = merger(tmp_path).merge(ctx_for(root, base, tmp_path, runner),
                                 request(artifacts(root, base, tmp_path, "a", edits)))
    assert not res.ok and any("dead module" in e for e in res.errors)
    assert runner.cargo() == [] and git(root, "status", "--porcelain") == ""


def test_engine_code_without_engine_flag_is_refused(repo, tmp_path):
    root, base = repo
    res = merger(tmp_path).merge(ctx_for(root, base, tmp_path, FakeRunner()),
                                 request(artifacts(root, base, tmp_path, "a", ENGINE_FIX)))
    assert not res.ok and "engine=True" in res.errors[0]
    assert git(root, "status", "--porcelain") == ""


def test_tracker_edits_in_a_worker_diff_are_refused(repo, tmp_path):
    root, base = repo
    res = merger(tmp_path).merge(ctx_for(root, base, tmp_path, FakeRunner()), request(
        artifacts(root, base, tmp_path, "a", {"qa/dsl-vocab-gaps.md": b"# DSL\n## mine\n"})))
    assert not res.ok and "orchestrator-only" in res.errors[0]


def test_run_tree_on_main_is_refused(tmp_path, gitenv):
    root, base = make_repo(tmp_path, branch="main")
    res = merger(tmp_path).merge(ctx_for(root, base, tmp_path, FakeRunner()),
                                 request(artifacts(root, base, tmp_path, "a", CARD_029)))
    assert not res.ok and "main" in res.errors[0]
    assert git(root, "status", "--porcelain") == "" and git(root, "rev-parse", "HEAD") == base


def test_missing_artifacts_are_an_error(repo, tmp_path):
    root, base = repo
    res = merger(tmp_path).merge(ctx_for(root, base, tmp_path, FakeRunner()), request({}))
    assert not res.ok and "artifacts" in res.errors[0]


# --- engine merges on their own branch -----------------------------------------------------


def test_engine_merge_lands_on_its_own_branch_and_leaves_the_run_branch(repo, tmp_path):
    root, base = repo
    (root / "scratch.txt").write_text("driver scratch\n")
    runner = FakeRunner(scope={"full_suite_required": True, "cards": [], "side_binaries": []})
    ctx = ctx_for(root, base, tmp_path, runner)
    lane = RunGapLane(tmp_path / "run", {}, repo=root)
    lane.park("card:BT21-030", "G-ENGINE-RULE", is_core=True)
    art = artifacts(root, base, tmp_path, "a", ENGINE_FIX)

    res = LoopMerger(worktree_root=str(tmp_path / "wt"), gap_lane=lane).merge(
        ctx, request(art, engine=True, gap_id="G-ENGINE-RULE"))

    assert res.ok, res.errors
    branch = "card-loop/run1/engine-G-ENGINE-RULE"
    assert res.branch == branch and res.sha == git(root, "rev-parse", branch)
    assert git(root, "rev-parse", f"{branch}~1") == base
    assert "Loop-Gap: G-ENGINE-RULE" in git(root, "log", "-1", "--format=%B", branch)
    # the run branch is untouched: same HEAD, same tree, still checked out
    assert git(root, "rev-parse", "HEAD") == base
    assert git(root, "rev-parse", "--abbrev-ref", "HEAD") == RUN_BRANCH
    assert git(root, "status", "--porcelain") == "?? scratch.txt"
    assert (root / ENG / "src/lib.rs").read_bytes() == BASE_FILES[f"{ENG}/src/lib.rs"]
    # cargo ran in the engine worktree with its own target dir (rule 31)
    wt = Path(runner.cargo()[0]["cwd"])
    assert wt.parent == tmp_path / "wt" and wt != root
    assert all(c["env"]["CARGO_TARGET_DIR"] == str(tmp_path / "tgt" / wt.name)
               for c in runner.cargo())
    # the gap lane knows where the fix is; the branch is free for a human to check out
    assert lane.get("G-ENGINE-RULE")["branch"] == branch
    assert lane.get("G-ENGINE-RULE")["branch_sha"] == res.sha
    assert git(wt, "rev-parse", "--abbrev-ref", "HEAD") == "HEAD"
    assert lane.closed(ctx) == []
    git(root, "merge", "-q", "--no-ff", "-m", "human merge", branch)
    assert lane.closed(ctx) == ["G-ENGINE-RULE"]


def test_second_engine_fix_for_a_gap_reuses_its_branch(repo, tmp_path):
    root, base = repo
    m = merger(tmp_path)
    ctx = ctx_for(root, base, tmp_path, FakeRunner())
    first = m.merge(ctx, request(artifacts(root, base, tmp_path, "a", ENGINE_FIX), engine=True,
                                 gap_id="G-X"))
    more = {f"{ENG}/src/extra.rs": b"pub fn extra() {}\n",
            f"{CB}/bt21/bt21_031.rs": b"#[test]\nfn extra() {}\n",
            f"{CB}/bt21/mod.rs": b"mod bt21_001;\nmod bt21_031;\n"}
    second = m.merge(ctx, request(artifacts(root, base, tmp_path, "b", more), engine=True,
                                  gap_id="G-X", attempt="20261006T130000Z-run1-b2"))
    assert first.ok and second.ok, second.errors
    assert second.branch == first.branch
    assert git(root, "rev-parse", f"{second.branch}~1") == first.sha
    assert git(root, "rev-parse", "HEAD") == base


def test_failed_engine_merge_deletes_the_branch_it_created(repo, tmp_path):
    root, base = repo
    runner = FakeRunner(fail=[lambda a: a[:2] == ["cargo", "test"]])
    res = merger(tmp_path).merge(ctx_for(root, base, tmp_path, runner),
                                 request(artifacts(root, base, tmp_path, "a", ENGINE_FIX),
                                         engine=True, gap_id="G-X"))
    assert not res.ok
    assert git(root, "branch", "--list", "card-loop/run1/engine-G-X") == ""
    assert git(root, "rev-parse", "HEAD") == base and git(root, "status", "--porcelain") == ""


def test_run_branch_named_like_the_run_prefix_is_reported(tmp_path, gitenv):
    root, base = make_repo(tmp_path, branch="card-loop/run1")
    res = merger(tmp_path).merge(ctx_for(root, base, tmp_path, FakeRunner()),
                                 request(artifacts(root, base, tmp_path, "a", ENGINE_FIX),
                                         engine=True, gap_id="G-X"))
    assert not res.ok and "card-loop/run1/run" in res.errors[0]


def test_engine_branch_name_and_paths():
    assert engine_branch_name("run1", "G-ENGINE-RULE") == "card-loop/run1/engine-G-ENGINE-RULE"
    assert engine_branch_name("run1", "weird id/..x") == "card-loop/run1/engine-weird-id-x"
    assert is_engine_path(f"{ENG}/src/game.rs") and is_engine_path("code/digimon-dsl/src/x.rs")
    assert not is_engine_path(f"{ENG}/cards/bt21/BT21-001.yaml")
    assert not is_engine_path(f"{CB}/bt21/bt21_001.rs")


def test_a_diff_against_the_run_heads_newer_base_merges(repo, tmp_path):
    # Re-authoring: the second worker started from the run branch's HEAD (after
    # the first merge), so its manifest base is newer than the plan's base.
    root, base = repo
    ctx = ctx_for(root, base, tmp_path, FakeRunner())
    assert merger(tmp_path).merge(ctx, request(artifacts(root, base, tmp_path, "a", CARD_029))).ok
    head = git(root, "rev-parse", "HEAD")
    assert head != base
    yaml = next(p for p in CARD_029 if p.endswith(".yaml"))
    art = artifacts(root, head, tmp_path, "b", {yaml: CARD_029[yaml] + b"# re-authored\n"})
    res = merger(tmp_path).merge(ctx, request(art, attempt="20261006T120001Z-run1-a2"))
    assert res.ok, res.errors
    assert (root / yaml).read_bytes().endswith(b"# re-authored\n")
