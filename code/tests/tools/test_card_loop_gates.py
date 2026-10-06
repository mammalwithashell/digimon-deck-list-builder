"""Fix gate (add-card-authoring-loop task 6.5; spec "Fixes pass the fix gate and
engine fixes stay off main"): a citation, named tests that are red without the
fix and green with it (executed, never taken from the worker's pasted lines),
and a green scoped suite."""
import json
import subprocess
from pathlib import Path
from types import SimpleNamespace

import pytest

from tools.author_set import merge_wave as mw
from tools.card_loop.driver_contracts import ItemRecord, MergeResult
from tools.card_loop.gates import FixGate, citation_problem, fix_result, parse_test_name

ENG = "code/digimon-engine"
CB = f"{ENG}/tests/cards_behavioral"
YAML = f"{ENG}/cards/bt21/BT21-029.yaml"
TEST = f"{CB}/bt21/bt21_029.rs"
MOD = f"{CB}/bt21/mod.rs"
LIB = f"{ENG}/src/lib.rs"
RUN_BRANCH = "card-loop/run1/run"
BASE_FILES = {
    f"{ENG}/Cargo.toml": (b'[[test]]\nname = "cards_behavioral"\npath = "tests/cards_behavioral/main.rs"\n\n'
                          b'[[test]]\nname = "dsl"\npath = "tests/dsl/main.rs"\n'),
    LIB: b"pub fn rule() -> u8 { 1 }\n",
    f"{CB}/main.rs": b"mod bt21;\n",
    MOD: b"mod bt21_001;\n",
    f"{CB}/bt21/bt21_001.rs": b"#[test]\nfn plays() {}\n",
    YAML: b"id: BT21-029\noptional: false\n",
}
SUITE_GREEN = {"green": True, "ran": True, "commands": [
    {"command": "RUST_MIN_STACK=268435456 cargo test ... -- bt21_029 --test-threads=8", "rc": 0,
     "tail": "test result: ok. 3 passed"}]}


def git(cwd, *args):
    return subprocess.run(["git", *args], cwd=cwd, check=True, capture_output=True,
                          text=True, encoding="utf-8").stdout.strip()


@pytest.fixture
def repo(tmp_path, monkeypatch):
    cfg = tmp_path / "gitconfig"
    cfg.write_text("[user]\n\tname = t\n\temail = t@example.invalid\n[core]\n\tautocrlf = false\n",
                   encoding="utf-8")
    monkeypatch.setenv("GIT_CONFIG_GLOBAL", str(cfg))
    monkeypatch.setenv("GIT_CONFIG_NOSYSTEM", "1")
    root = tmp_path / "repo"
    root.mkdir()
    git(root, "init", "-q", "-b", RUN_BRANCH)
    for p, data in BASE_FILES.items():
        (root / p).parent.mkdir(parents=True, exist_ok=True)
        (root / p).write_bytes(data)
    git(root, "add", "-A")
    git(root, "commit", "-qm", "base")
    return root, git(root, "rev-parse", "HEAD")


def commit_fix(root, edits, msg="fix"):
    for p, data in edits.items():
        (root / p).parent.mkdir(parents=True, exist_ok=True)
        (root / p).write_bytes(data)
    git(root, "add", "-A")
    git(root, "commit", "-qm", msg)
    return git(root, "rev-parse", "HEAD")


CARD_FIX = {
    YAML: b"id: BT21-029\noptional: true\n",
    TEST: b"#[test]\nfn declines() {}\n#[test]\nfn plays() {}\n",
    MOD: b"mod bt21_001;\nmod bt21_029;\n",
}


def yaml_fixed(tree: Path) -> bool:
    return b"optional: true" in (tree / YAML).read_bytes()


class TreeRunner:
    """Simulates libtest from the tree it is run in: each known test's verdict
    is a predicate of that tree; `compiles` gates the build."""

    def __init__(self, tests, *, compiles=lambda tree: True, scope=None, abort=False):
        self.tests = tests
        self.compiles = compiles
        self.scope = scope or {"full_suite_required": False, "cards": ["BT21-029"], "side_binaries": []}
        self.abort = abort
        self.calls = []

    def __call__(self, argv, cwd, timeout):
        env, bare = mw.split_env(argv)
        tree = Path(cwd)
        self.calls.append({"env": env, "argv": bare, "cwd": str(cwd),
                           "yaml": (tree / YAML).read_bytes() if (tree / YAML).exists() else None})
        if len(bare) > 1 and bare[1] == mw.IMPACT_SCOPE_SCRIPT:
            return 0, json.dumps(self.scope), ""
        if bare[:2] != ["cargo", "test"]:
            return 0, "", ""
        if not self.compiles(tree):
            return 101, "", "error[E0425]: cannot find function `rule2`\nerror: could not compile"
        filters = [a for a in bare[bare.index("--") + 1:] if not a.startswith("--")]
        sel = [t for t in self.tests if not filters or any(f in t for f in filters)]
        out = [f"running {len(sel)} tests"]
        if self.abort:
            return 1, "\n".join(out), "error: test failed, to rerun pass `--test cards_behavioral`"
        ok = True
        for t in sel:
            good = self.tests[t](tree)
            ok &= good
            out.append(f"test {t} ... {'ok' if good else 'FAILED'}")
        out.append(f"test result: {'ok' if ok else 'FAILED'}.")
        return (0 if ok else 101), "\n".join(out), ""

    def cargo_tests(self):
        return [c for c in self.calls if c["argv"][:2] == ["cargo", "test"]]


def item_with(fix):
    return ItemRecord(item="clause:BT21-029#effect#0", state="GATE", data={"fix": fix})


def fix(**over):
    base = {"files": [YAML, TEST, MOD], "tests": ["bt21::bt21_029::declines"],
            "test_result_lines": ["test result: ok. 1 passed; 0 failed"], "gaps": [],
            "citation": {"kind": "rule", "ref": "16-36-1"}, "notes": ""}
    return {**base, **over}


def ctx_for(root, base, tmp_path, runner):
    return SimpleNamespace(run_id="run1", run_dir=str(tmp_path / "run"), repo=str(root),
                           base_sha=base, plan={}, pool=None, run_command=runner,
                           config=SimpleNamespace(cargo_target_base=str(tmp_path / "tgt")))


def merged(sha, touched=(YAML, TEST, MOD), scope=None, branch=RUN_BRANCH, ok=True):
    return MergeResult(ok=ok, sha=sha, branch=branch, touched=list(touched),
                       scope=scope if scope is not None else {"cards": ["BT21-029"],
                                                              "suite": SUITE_GREEN})


def gate(tmp_path):
    return FixGate(worktree_root=str(tmp_path / "wt"))


# --- the happy path ------------------------------------------------------------------------


def test_red_before_green_after_passes(repo, tmp_path):
    root, base = repo
    sha = commit_fix(root, CARD_FIX)
    runner = TreeRunner({"bt21::bt21_029::declines": yaml_fixed,
                         "bt21::bt21_029::plays": lambda t: True,
                         "bt21::bt21_001::plays": lambda t: True})

    res = gate(tmp_path).check(ctx_for(root, base, tmp_path, runner), item_with(fix()), merged(sha))

    assert res.passed, res.reasons
    after, before = runner.cargo_tests()
    assert after["yaml"] == CARD_FIX[YAML] and before["yaml"] == BASE_FILES[YAML]
    for run in (after, before):
        assert run["argv"] == ["cargo", "test", "--manifest-path", f"{ENG}/Cargo.toml", "--test",
                               "cards_behavioral", "--", "bt21::bt21_029::declines",
                               "--test-threads=8"]
        assert Path(run["cwd"]) != root and Path(run["cwd"]).parent == tmp_path / "wt"
        assert run["env"]["RUST_MIN_STACK"] == "268435456"
        assert run["env"]["CARGO_TARGET_DIR"] == str(tmp_path / "tgt" / Path(run["cwd"]).name)
    ev = res.evidence
    assert ev["citation"] == {"kind": "rule", "ref": "16-36-1", "ok": True}
    assert ev["after"]["results"]["bt21::bt21_029::declines"]["status"] == "pass"
    assert ev["before"]["results"]["bt21::bt21_029::declines"]["status"] == "red"
    assert ev["before"]["reverted"] == [YAML] and ev["before"]["at"] == base
    assert ev["after"]["runs"][0]["command"].startswith("CARGO_TARGET_DIR=")
    assert ev["scoped_suite"]["reused"] is True
    assert ev["worker_claims"] == ["test result: ok. 1 passed; 0 failed"]
    # the run tree itself was never touched
    assert git(root, "status", "--porcelain") == "" and git(root, "rev-parse", "HEAD") == sha


def test_a_test_that_passes_without_the_fix_proves_nothing(repo, tmp_path):
    root, base = repo
    sha = commit_fix(root, CARD_FIX)
    runner = TreeRunner({"bt21::bt21_029::declines": lambda t: True})
    res = gate(tmp_path).check(ctx_for(root, base, tmp_path, runner), item_with(fix()), merged(sha))
    assert not res.passed
    assert any("passes without the fix" in r and base[:12] in r for r in res.reasons), res.reasons


def test_the_workers_pasted_lines_are_not_trusted(repo, tmp_path):
    root, base = repo
    sha = commit_fix(root, CARD_FIX)
    runner = TreeRunner({"bt21::bt21_029::declines": lambda t: False})
    res = gate(tmp_path).check(ctx_for(root, base, tmp_path, runner),
                               item_with(fix(test_result_lines=["test result: ok. 9 passed"])),
                               merged(sha))
    assert not res.passed
    assert any("fails with the fix" in r for r in res.reasons), res.reasons


def test_a_test_that_does_not_compile_without_the_fix_counts_as_red(repo, tmp_path):
    root, base = repo
    sha = commit_fix(root, {LIB: b"pub fn rule() -> u8 { 1 }\npub fn rule2() -> u8 { 2 }\n",
                            TEST: b"#[test]\nfn declines() {}\n", MOD: CARD_FIX[MOD]})
    runner = TreeRunner({"bt21::bt21_029::declines": lambda t: True},
                        compiles=lambda t: b"rule2" in (t / LIB).read_bytes())
    m = merged(sha, touched=(LIB, TEST, MOD), branch="card-loop/run1/engine-G-RULE")
    res = gate(tmp_path).check(ctx_for(root, base, tmp_path, runner), item_with(fix()), m)
    assert res.passed, res.reasons
    assert res.evidence["before"]["results"]["bt21::bt21_029::declines"]["status"] == "compile_error"


# --- named tests ---------------------------------------------------------------------------


def test_unknown_and_ambiguous_test_names(repo, tmp_path):
    root, base = repo
    sha = commit_fix(root, CARD_FIX)
    runner = TreeRunner({"bt21::bt21_029::declines": yaml_fixed, "bt21::bt21_029::plays": yaml_fixed,
                         "bt21::bt21_001::plays": yaml_fixed})
    res = gate(tmp_path).check(ctx_for(root, base, tmp_path, runner),
                               item_with(fix(tests=["bt21::bt21_029::nope", "plays"])), merged(sha))
    assert not res.passed
    assert any("bt21::bt21_029::nope" in r and "no test matched" in r for r in res.reasons)
    assert any("'plays' is ambiguous" in r for r in res.reasons)


def test_an_aborted_run_is_inconclusive(repo, tmp_path):
    root, base = repo
    sha = commit_fix(root, CARD_FIX)
    runner = TreeRunner({"bt21::bt21_029::declines": yaml_fixed}, abort=True)
    res = gate(tmp_path).check(ctx_for(root, base, tmp_path, runner), item_with(fix()), merged(sha))
    assert not res.passed and any("rerun" in r for r in res.reasons)


def test_parse_test_name():
    bins = {"cards_behavioral", "dsl"}
    assert parse_test_name("bt21::bt21_029::declines", bins) == ("cards_behavioral",
                                                                 "bt21::bt21_029::declines")
    assert parse_test_name("cards_behavioral::bt21::bt21_029::declines", bins) == (
        "cards_behavioral", "bt21::bt21_029::declines")
    assert parse_test_name("dsl::lowering::x", bins) == ("dsl", "lowering::x")
    assert parse_test_name(f"{CB}/bt21/bt21_029.rs::declines", bins) == (
        "cards_behavioral", "bt21::bt21_029::declines")
    assert parse_test_name("tests/cards_behavioral/bt21/bt21_029.rs", bins) == (
        "cards_behavioral", "bt21::bt21_029")
    for bad in ("x; rm -rf /", "--nocapture", "bt21::", "a b", "", "$(boom)"):
        with pytest.raises(ValueError):
            parse_test_name(bad, bins)


# --- citation ------------------------------------------------------------------------------


@pytest.mark.parametrize("citation", [
    {"kind": "rule", "ref": "16-36"}, {"kind": "rule", "ref": "§16-36-1"},
    {"kind": "rule", "ref": "general_rule.pdf §5-2 (Save)"},
    {"kind": "ruling", "ref": "qa:Q1601"},
    {"kind": "dcgo", "ref": "BT21_029.cs:45"},
    {"kind": "dcgo", "ref": "Assets/Scripts/CardEffect/BT21/Red/BT21_029.cs:120-131"},
    {"kind": "dcgo", "ref": "Assets\\Scripts\\CardEffect\\BT21\\Red\\BT21_029.cs"},
])
def test_well_formed_citations(citation):
    assert citation_problem(citation) is None


@pytest.mark.parametrize("citation", [
    None, {}, {"kind": "rule", "ref": ""}, {"kind": "rule", "ref": "see the rules"},
    {"kind": "rule", "ref": "16"}, {"kind": "ruling", "ref": "Q1601"},
    {"kind": "ruling", "ref": "qa:1601"}, {"kind": "dcgo", "ref": "BT21_029"},
    {"kind": "dcgo", "ref": "BT21_029.cs:abc"}, {"kind": "wiki", "ref": "x"},
])
def test_missing_or_malformed_citations(citation):
    assert citation_problem(citation)


def test_missing_citation_fails_without_running_anything(repo, tmp_path):
    root, base = repo
    sha = commit_fix(root, CARD_FIX)
    runner = TreeRunner({"bt21::bt21_029::declines": yaml_fixed})
    res = gate(tmp_path).check(ctx_for(root, base, tmp_path, runner),
                               item_with(fix(citation={"kind": "rule", "ref": "trust me"})),
                               merged(sha))
    assert not res.passed and runner.calls == []
    assert any("citation" in r and "escalate" in r for r in res.reasons)


# --- the other preconditions ---------------------------------------------------------------


def test_scoped_suite_is_run_when_the_merge_did_not(repo, tmp_path):
    root, base = repo
    sha = commit_fix(root, CARD_FIX)
    runner = TreeRunner({"bt21::bt21_029::declines": yaml_fixed,
                         "bt21::bt21_001::plays": lambda t: False})    # a regression elsewhere
    runner.scope = {"full_suite_required": True, "cards": [], "side_binaries": []}
    res = gate(tmp_path).check(ctx_for(root, base, tmp_path, runner), item_with(fix()),
                               merged(sha, scope={}))
    assert not res.passed and any("scoped suite" in r for r in res.reasons)
    suite = res.evidence["scoped_suite"]
    assert suite["reused"] is False
    assert suite["commands"][0]["command"].endswith(
        "cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- "
        "--test-threads=8")


def test_red_merge_suite_fails_the_gate(repo, tmp_path):
    root, base = repo
    sha = commit_fix(root, CARD_FIX)
    runner = TreeRunner({"bt21::bt21_029::declines": yaml_fixed})
    red = {"green": False, "ran": True, "commands": [{"command": "cargo test", "rc": 101, "tail": "x"}]}
    res = gate(tmp_path).check(ctx_for(root, base, tmp_path, runner), item_with(fix()),
                               merged(sha, scope={"cards": [], "suite": red}))
    assert not res.passed and any("scoped suite" in r for r in res.reasons)


def test_engine_code_on_the_run_branch_fails(repo, tmp_path):
    root, base = repo
    sha = commit_fix(root, {LIB: b"pub fn rule() -> u8 { 2 }\n", **CARD_FIX})
    runner = TreeRunner({"bt21::bt21_029::declines": yaml_fixed})
    res = gate(tmp_path).check(ctx_for(root, base, tmp_path, runner), item_with(fix()),
                               merged(sha, touched=(LIB, YAML, TEST, MOD)))
    assert not res.passed and any("D13" in r for r in res.reasons)


def test_fix_reporting_gaps_should_park(repo, tmp_path):
    root, base = repo
    sha = commit_fix(root, CARD_FIX)
    res = gate(tmp_path).check(
        ctx_for(root, base, tmp_path, TreeRunner({})),
        item_with(fix(gaps=[{"kind": "dsl", "id": "G-DSL-X", "summary": "verb"}])), merged(sha))
    assert not res.passed and any("park" in r and "G-DSL-X" in r for r in res.reasons)


def test_no_fix_result_or_failed_merge(repo, tmp_path):
    root, base = repo
    ctx = ctx_for(root, base, tmp_path, TreeRunner({}))
    res = gate(tmp_path).check(ctx, ItemRecord(item="clause:BT21-029#effect#0", state="GATE"),
                               merged(base))
    assert not res.passed and "item.data['fix']" in res.reasons[0]
    res = gate(tmp_path).check(ctx, item_with(fix()), merged(None, ok=False))
    assert not res.passed and any("merge" in r for r in res.reasons)
    assert res.reasons and not ctx.run_command.calls


def test_no_named_tests(repo, tmp_path):
    root, base = repo
    sha = commit_fix(root, CARD_FIX)
    res = gate(tmp_path).check(ctx_for(root, base, tmp_path, TreeRunner({})),
                               item_with(fix(tests=[])), merged(sha))
    assert not res.passed and any("no regression test" in r for r in res.reasons)


# --- merger -> gate through the contract ---------------------------------------------------


@pytest.mark.parametrize("engine", [False, True])
def test_merger_then_gate(repo, tmp_path, engine):
    from tools.card_loop.driver_contracts import MergeRequest
    from tools.card_loop.merge import LoopMerger
    from tools.card_loop.workers.base import capture_artifacts

    root, base = repo
    edits = dict(CARD_FIX)
    if engine:
        edits[LIB] = b"pub fn rule() -> u8 { 2 }\n"
    wt = tmp_path / "worker"
    git(root, "worktree", "add", "-q", "--detach", str(wt), base)
    for p, data in edits.items():
        (wt / p).write_bytes(data)
    art = capture_artifacts(wt, base, tmp_path / "art")
    runner = TreeRunner({"bt21::bt21_029::declines": yaml_fixed,
                         "bt21::bt21_029::plays": lambda t: True,
                         "bt21::bt21_001::plays": lambda t: True})
    ctx = ctx_for(root, base, tmp_path, runner)

    merge = LoopMerger(worktree_root=str(tmp_path / "wt")).merge(ctx, MergeRequest(
        attempt_id="20261006T120000Z-run1-f1", family="claude", model="sonnet", artifacts=art,
        engine=engine, gap_id="G-ENGINE-RULE" if engine else None))
    assert merge.ok, merge.errors
    n_merge_calls = len(runner.cargo_tests())

    res = gate(tmp_path).check(ctx, item_with(fix()), merge)

    assert res.passed, res.reasons
    assert res.evidence["scoped_suite"]["reused"] is True
    assert len(runner.cargo_tests()) == n_merge_calls + 2          # after + before only
    assert git(root, "rev-parse", "HEAD") == (base if engine else merge.sha)
    if engine:
        assert merge.branch == "card-loop/run1/engine-G-ENGINE-RULE"
        assert sorted(res.evidence["before"]["reverted"]) == sorted([LIB, YAML])


def test_fix_result_lookup():
    f = fix()
    assert fix_result(item_with(f)) == f
    assert fix_result(ItemRecord(item="clause:X#effect#0", state="GATE", data={"fix_result": f})) == f
    assert fix_result(ItemRecord(item="clause:X#effect#0", state="GATE", data=dict(f))) == f
    assert fix_result(ItemRecord(item="clause:X#effect#0", state="GATE", data={})) is None
