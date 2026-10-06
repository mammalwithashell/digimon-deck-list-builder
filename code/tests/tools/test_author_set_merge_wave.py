"""merge_wave (harden-card-authoring-pipeline tasks 1.2, 1.4): manifest-filtered
3-way apply, registration assertion, pack check hook, idempotency, wave summary."""
import json
import os
import subprocess
import sys
from pathlib import Path

import pytest

from tools.author_set import merge_wave as mw
from tools.card_loop.workers.base import capture_artifacts

ENG = "code/digimon-engine"
CB = f"{ENG}/tests/cards_behavioral"
BASE_FILES = {
    f"{ENG}/Cargo.toml": b'[[test]]\nname = "cards_behavioral"\npath = "tests/cards_behavioral/main.rs"\n',
    f"{CB}/main.rs": b"mod bt21;\n",
    f"{CB}/bt21/mod.rs": b"mod bt21_001;\n",
    f"{CB}/bt21/bt21_001.rs": b"#[test]\nfn plays() {}\n",
    f"{ENG}/cards/bt21/BT21-001.yaml": b"id: BT21-001\n",
    "notes.txt": b"one\ntwo\nthree\n",
}


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
    git(root, "init", "-q", "-b", "run")
    for p, data in BASE_FILES.items():
        (root / p).parent.mkdir(parents=True, exist_ok=True)
        (root / p).write_bytes(data)
    git(root, "add", "-A")
    git(root, "commit", "-qm", "base")
    return root, git(root, "rev-parse", "HEAD")


def worker(root, base, tmp_path, name, edits):
    """A worker worktree at `base` with `edits` (path -> bytes, None deletes),
    captured exactly as card_loop workers capture it."""
    wt = tmp_path / f"wt-{name}"
    git(root, "worktree", "add", "-q", "--detach", str(wt), base)
    for p, data in edits.items():
        fp = wt / p
        if data is None:
            fp.unlink()
        else:
            fp.parent.mkdir(parents=True, exist_ok=True)
            fp.write_bytes(data)
    return capture_artifacts(wt, base, tmp_path / f"art-{name}")


CARD_029 = {
    f"{ENG}/cards/bt21/BT21-029.yaml": b"id: BT21-029\n",
    f"{CB}/bt21/bt21_029.rs": b"#[test]\nfn declines() {}\n",
    f"{CB}/bt21/mod.rs": b"mod bt21_001;\nmod bt21_029;\n",
}


def status(root):
    return git(root, "status", "--porcelain")


# --- apply ---------------------------------------------------------------------------------


def test_approved_card_applies_and_registers(repo, tmp_path):
    root, base = repo
    art = worker(root, base, tmp_path, "a", CARD_029)

    res = mw.apply_manifest_diff(root, art["diff"], art["manifest"], base)

    assert res.ok, res.errors
    assert res.applied == [f"{ENG}/cards/bt21/BT21-029.yaml", f"{CB}/bt21/bt21_029.rs"]
    assert res.registered == [f"{CB}/bt21/mod.rs"]          # mod-line union, not a 3-way apply
    assert res.out_of_manifest == []
    assert (root / CB / "bt21/mod.rs").read_bytes() == b"mod bt21_001;\nmod bt21_029;\n"
    assert mw.assert_registration(root, res) == []
    # staged: a later commit picks the merge up from the index
    assert set(git(root, "diff", "--cached", "--name-only").splitlines()) == set(CARD_029)


def test_out_of_manifest_paths_are_reported_never_applied(repo, tmp_path):
    root, base = repo
    art = worker(root, base, tmp_path, "a", {**CARD_029, "notes.txt": b"tampered\n"})
    manifest = json.loads(Path(art["manifest"]).read_text())
    manifest["files"] = [f for f in manifest["files"] if f["path"] != "notes.txt"]
    Path(art["manifest"]).write_text(json.dumps(manifest))

    res = mw.apply_manifest_diff(root, art["diff"], art["manifest"], base)

    assert res.ok, res.errors
    assert res.out_of_manifest == ["notes.txt"]
    assert (root / "notes.txt").read_bytes() == BASE_FILES["notes.txt"]
    assert "notes.txt" not in res.touched


def test_dead_module_is_reported_and_can_be_registered(repo, tmp_path):
    root, base = repo
    edits = {k: v for k, v in CARD_029.items() if not k.endswith("mod.rs")}
    art = worker(root, base, tmp_path, "a", edits)

    res = mw.apply_manifest_diff(root, art["diff"], art["manifest"], base)
    problems = mw.assert_registration(root, res)

    assert res.ok
    assert len(problems) == 1 and problems[0].startswith("dead module:")
    assert "bt21/mod.rs does not declare `mod bt21_029;`" in problems[0]

    assert mw.add_missing_registrations(root, res) == [f"{CB}/bt21/mod.rs"]
    assert mw.assert_registration(root, res) == []


def test_dead_module_up_the_chain_names_the_missing_set(repo, tmp_path):
    root, base = repo
    art = worker(root, base, tmp_path, "a", {
        f"{CB}/bt99/mod.rs": b"mod bt99_001;\n",
        f"{CB}/bt99/bt99_001.rs": b"#[test]\nfn x() {}\n",
    })
    res = mw.apply_manifest_diff(root, art["diff"], art["manifest"], base)
    problems = mw.assert_registration(root, res)
    assert any("main.rs does not declare `mod bt99;`" in p for p in problems)


def test_test_file_without_a_test_is_a_problem(repo, tmp_path):
    root, base = repo
    art = worker(root, base, tmp_path, "a", {
        f"{CB}/bt21/bt21_029.rs": b"fn helper() {}\n",
        f"{CB}/bt21/mod.rs": b"mod bt21_001;\nmod bt21_029;\n",
    })
    res = mw.apply_manifest_diff(root, art["diff"], art["manifest"], base)
    assert mw.assert_registration(root, res) == [
        f"{CB}/bt21/bt21_029.rs contains no #[test] -- a test module must contribute at least "
        f"one test to the `cards_behavioral` binary"]


def test_deleted_test_still_declared_is_a_problem(repo, tmp_path):
    root, base = repo
    art = worker(root, base, tmp_path, "a", {f"{CB}/bt21/bt21_001.rs": None})
    res = mw.apply_manifest_diff(root, art["diff"], art["manifest"], base)
    assert res.ok and res.applied == [f"{CB}/bt21/bt21_001.rs"]
    assert not (root / CB / "bt21/bt21_001.rs").exists()
    problems = mw.assert_registration(root, res)
    assert problems and "still declares `mod bt21_001;`" in problems[0]


def test_reapplying_the_same_diff_is_a_noop(repo, tmp_path):
    root, base = repo
    art = worker(root, base, tmp_path, "a", CARD_029)
    first = mw.apply_manifest_diff(root, art["diff"], art["manifest"], base)
    assert first.ok and not first.noop

    again = mw.apply_manifest_diff(root, art["diff"], art["manifest"], base)
    assert again.ok and again.noop, again.errors
    assert again.already_applied == sorted(CARD_029)

    git(root, "commit", "-qm", "merged")
    after_commit = mw.apply_manifest_diff(root, art["diff"], art["manifest"], base)
    assert after_commit.noop and status(root) == ""


def test_two_cards_of_one_set_union_their_registrations(repo, tmp_path):
    root, base = repo
    a = worker(root, base, tmp_path, "a", CARD_029)
    b = worker(root, base, tmp_path, "b", {
        f"{CB}/bt21/bt21_030.rs": b"#[test]\nfn x() {}\n",
        f"{CB}/bt21/mod.rs": b"mod bt21_001;\nmod bt21_030;\n",
    })
    assert mw.apply_manifest_diff(root, a["diff"], a["manifest"], base).ok
    git(root, "commit", "-qm", "a")

    res = mw.apply_manifest_diff(root, b["diff"], b["manifest"], base)

    assert res.ok, res.errors
    assert (root / CB / "bt21/mod.rs").read_bytes() == b"mod bt21_001;\nmod bt21_029;\nmod bt21_030;\n"
    assert mw.assert_registration(root, res) == []


def test_stale_base_is_refused(repo, tmp_path):
    root, base = repo
    art = worker(root, base, tmp_path, "a", CARD_029)
    (root / "notes.txt").write_bytes(b"moved on\n")
    git(root, "commit", "-qam", "next")
    newer = git(root, "rev-parse", "HEAD")

    res = mw.apply_manifest_diff(root, art["diff"], art["manifest"], newer)

    assert not res.ok and "stale worker base" in res.errors[0]
    assert status(root) == ""


def test_conflict_rolls_back_only_what_it_touched(repo, tmp_path):
    root, base = repo
    y001 = f"{ENG}/cards/bt21/BT21-001.yaml"
    art = worker(root, base, tmp_path, "a", {**CARD_029, y001: b"id: BT21-001\nname: W\n"})
    (root / y001).write_bytes(b"id: BT21-001\nname: R\n")
    git(root, "commit", "-qam", "conflicting edit")
    (root / "unrelated.txt").write_bytes(b"driver scratch\n")      # someone else's dirty file
    head_mod = (root / CB / "bt21/mod.rs").read_bytes()

    res = mw.apply_manifest_diff(root, art["diff"], art["manifest"], base)

    assert not res.ok and "git apply --3way failed" in res.errors[0]
    assert res.rolled_back and res.applied == [] and res.registered == []
    assert (root / y001).read_bytes() == b"id: BT21-001\nname: R\n"
    assert (root / CB / "bt21/mod.rs").read_bytes() == head_mod
    assert not (root / ENG / "cards/bt21/BT21-029.yaml").exists()
    assert not (root / CB / "bt21/bt21_029.rs").exists()
    assert status(root) == "?? unrelated.txt"


def test_transport_mismatch_is_an_error(repo, tmp_path):
    root, base = repo
    art = worker(root, base, tmp_path, "a", CARD_029)
    manifest = json.loads(Path(art["manifest"]).read_text())
    for f in manifest["files"]:
        if f["path"].endswith(".yaml"):
            f["sha256"] = f["sha256_lf"] = "0" * 64   # neither hash matches: not the worker's bytes
    Path(art["manifest"]).write_text(json.dumps(manifest))

    res = mw.apply_manifest_diff(root, art["diff"], art["manifest"], base)

    assert not res.ok and "transport mismatch" in res.errors[0]
    assert status(root) == ""


def test_mixed_line_endings_in_the_workers_file_are_not_a_transport_mismatch(repo, tmp_path):
    # Second pilot, BT26-005: the worker's YAML mixed CRLF and LF lines; under
    # core.autocrlf the diff carried LF-normalised text, and no single-convention
    # hash of the applied bytes matched the manifest's raw sha256. Three
    # attempts, three "transport mismatch" refusals.
    root, base = repo
    yaml = f"{ENG}/cards/bt21/BT21-029.yaml"
    mixed = b"id: BT21-029\r\nname: Medusamon\nlevel: 6\r\n"
    wt = tmp_path / "wt-mixed"
    git(root, "worktree", "add", "-q", "--detach", str(wt), base)
    for p, data in {**CARD_029, yaml: mixed}.items():
        (wt / p).parent.mkdir(parents=True, exist_ok=True)
        (wt / p).write_bytes(data)
    git(root, "config", "core.autocrlf", "true")       # the pilots' Windows checkouts
    art = capture_artifacts(wt, base, tmp_path / "art-mixed")
    manifest = json.loads(Path(art["manifest"]).read_text())
    entry = next(f for f in manifest["files"] if f["path"] == yaml)
    assert entry["sha256_lf"] == __import__("hashlib").sha256(mixed.replace(b"\r\n", b"\n")).hexdigest()

    res = mw.apply_manifest_diff(root, art["diff"], art["manifest"], base)
    assert res.ok, res.errors
    assert (root / yaml).read_bytes().replace(b"\r\n", b"\n") == mixed.replace(b"\r\n", b"\n")

    # Without the LF hash the old check still refuses it: the field is what carries it.
    git(root, "reset", "-q", "--hard", base)
    for f in manifest["files"]:
        f.pop("sha256_lf", None)
    Path(art["manifest"]).write_text(json.dumps(manifest))
    res2 = mw.apply_manifest_diff(root, art["diff"], art["manifest"], base)
    if not res2.ok:
        assert "transport mismatch" in res2.errors[0]


def test_skip_paths_are_dropped_like_scratch_not_refused(repo, tmp_path):
    root, base = repo
    tracker = "qa/dsl-vocab-gaps.md"
    art = worker(root, base, tmp_path, "a", {**CARD_029, tracker: b"# DSL\n## mine\n"})
    res = mw.apply_manifest_diff(root, art["diff"], art["manifest"], base, skip_paths=(tracker,))
    assert res.ok, res.errors
    assert res.skipped == [tracker] and not (root / tracker).exists()
    assert (root / ENG / "cards/bt21/BT21-029.yaml").exists()


def test_content_hashes_ignore_line_endings():
    import hashlib
    lf = b"a\nb\n"
    assert hashlib.sha256(lf).hexdigest() in mw.content_hashes(b"a\r\nb\r\n")
    assert hashlib.sha256(b"a\r\nb\r\n").hexdigest() in mw.content_hashes(lf)


def test_harden_style_manifest(repo, tmp_path):
    """harden D1's manifest: `files: [{path, sha256}]` beside card/verdict/... (no status)."""
    root, base = repo
    art = worker(root, base, tmp_path, "a", CARD_029)
    files = [{"path": f["path"], "sha256": f["sha256"]}
             for f in json.loads(Path(art["manifest"]).read_text())["files"]]
    Path(art["manifest"]).write_text(json.dumps({
        "card": "BT21-029", "verdict": "IMPLEMENTED", "files": files,
        "test_result_lines": ["test result: ok. 1 passed"], "notes": "", "gaps": []}))

    res = mw.apply_manifest_diff(root, art["diff"], art["manifest"], base)
    assert res.ok and res.touched == sorted(CARD_029)
    again = mw.apply_manifest_diff(root, art["diff"], art["manifest"], base)
    assert again.noop, again.errors


def test_manifest_entry_without_sha256_is_refused(repo, tmp_path):
    root, base = repo
    art = worker(root, base, tmp_path, "a", CARD_029)
    Path(art["manifest"]).write_text(json.dumps({"base_sha": base, "files": sorted(CARD_029)}))
    res = mw.apply_manifest_diff(root, art["diff"], art["manifest"], base)
    assert not res.ok and "sha256" in res.errors[0]
    assert status(root) == ""


# --- path safety (security review) ---------------------------------------------------------


def rewrite_manifest(art, mutate):
    manifest = json.loads(Path(art["manifest"]).read_text())
    mutate(manifest)
    Path(art["manifest"]).write_text(json.dumps(manifest))


def assert_untouched(root, tmp_path):
    assert status(root) == ""
    assert git(root, "diff", "--cached", "--name-only") == ""
    assert not (tmp_path / "outside.yaml").exists()


def test_manifest_path_traversal_is_refused(repo, tmp_path):
    root, base = repo
    art = worker(root, base, tmp_path, "a", CARD_029)
    rewrite_manifest(art, lambda m: m["files"].append(
        {"path": "../outside.yaml", "status": "A", "sha256": "0" * 64}))

    res = mw.apply_manifest_diff(root, art["diff"], art["manifest"], base)

    assert not res.ok
    assert any("../outside.yaml" in e and "'..'" in e for e in res.errors), res.errors
    assert_untouched(root, tmp_path)


@pytest.mark.parametrize("bad", ["C:/Windows/evil.yaml", "/etc/evil.yaml", "c:evil.yaml",
                                 "code\\digimon-engine\\cards\\x.yaml"])
def test_absolute_or_non_canonical_manifest_path_is_refused(repo, tmp_path, bad):
    root, base = repo
    art = worker(root, base, tmp_path, "a", CARD_029)
    rewrite_manifest(art, lambda m: m["files"].append({"path": bad, "status": "A", "sha256": "0" * 64}))
    res = mw.apply_manifest_diff(root, art["diff"], art["manifest"], base)
    assert not res.ok and any(bad in e for e in res.errors), res.errors
    assert_untouched(root, tmp_path)


def test_paths_outside_the_allowed_roots_are_skipped_and_the_rest_applied(repo, tmp_path):
    # A worker's scratch file (`scratch_q.py`, `.tmp_p.py`) or an edit to a file
    # only the orchestrator writes must not cost the whole diff: the first
    # pilot lost every authored scenario this way. Skipped, reported, the card
    # still lands; a structurally unsafe path still refuses everything.
    root, base = repo
    art = worker(root, base, tmp_path, "a", {**CARD_029, "notes.txt": b"rewritten\n",
                                             ".git-hooks/pre-commit": b"#!/bin/sh\n",
                                             "scratch_q.py": b"print(1)\n"})
    res = mw.apply_manifest_diff(root, art["diff"], art["manifest"], base)
    assert res.ok, res.errors
    assert sorted(res.skipped) == [".git-hooks/pre-commit", "notes.txt", "scratch_q.py"]
    assert all(p in res.touched for p in CARD_029)        # the card landed (mod.rs by union)
    assert (root / "notes.txt").read_bytes() == BASE_FILES["notes.txt"]
    assert not (root / "scratch_q.py").exists() and not (root / ".git-hooks").exists()


def test_a_structurally_unsafe_manifest_path_still_refuses_the_whole_diff(repo, tmp_path):
    root, base = repo
    art = worker(root, base, tmp_path, "a", {**CARD_029, "scratch_q.py": b"x\n"})
    import json
    m = json.loads(open(art["manifest"], encoding="utf-8").read())
    m["files"].append({"path": "../outside.yaml", "status": "A", "sha256": "0" * 64})
    open(art["manifest"], "w", encoding="utf-8").write(json.dumps(m))
    res = mw.apply_manifest_diff(root, art["diff"], art["manifest"], base)
    assert not res.ok and any("'..' segment" in e for e in res.errors)
    assert_untouched(root, tmp_path)


def test_path_policy_unit():
    ok = f"{ENG}/cards/bt21/BT21-029.yaml"
    assert mw.path_problem(ok) is None
    assert mw.path_problem("qa/dsl-vocab-gaps.md") is None
    assert "outside the allowed roots" in mw.path_problem("qa/dsl-vocab-gaps.md.bak")
    for bad in ("../x", f"{ENG}/cards/../../x.yaml", f"{ENG}/cards/./x.yaml", f"{ENG}/cards//x.yaml",
                f"{ENG}/cards/x.yaml:stream", f"{ENG}/cards/.git/config", f"{ENG}/cards/x.",
                "-rf", "", f"{ENG}/cards/x\0.yaml"):
        assert mw.path_problem(bad) is not None, bad
    assert mw.canonical("./code/x") == "code/x" and mw.canonical("../x") == "../x"


SYMLINK_DIFF = (
    "diff --git a/{p} b/{p}\n"
    "new file mode 120000\n"
    "index 0000000000000000000000000000000000000000..1111111111111111111111111111111111111111\n"
    "--- /dev/null\n"
    "+++ b/{p}\n"
    "@@ -0,0 +1 @@\n"
    "+../../../../../outside.yaml\n"
    "\\ No newline at end of file\n")


def test_symlink_creating_diff_is_refused(repo, tmp_path):
    root, base = repo
    p = f"{ENG}/cards/bt21/BT21-099.yaml"
    diff = tmp_path / "evil.diff"
    diff.write_bytes(SYMLINK_DIFF.format(p=p).encode())
    manifest = tmp_path / "evil.json"
    manifest.write_text(json.dumps({"base_sha": base, "files": [
        {"path": p, "status": "A", "sha256": "0" * 64}]}))

    res = mw.apply_manifest_diff(root, diff, manifest, base)

    assert not res.ok and any("symlink" in e and p in e for e in res.errors), res.errors
    assert not (root / p).exists() and not os.path.lexists(root / p)
    assert_untouched(root, tmp_path)


def make_link(link: Path, target: Path) -> None:
    """A directory symlink, or a junction where symlinks need privileges (Windows)."""
    try:
        os.symlink(target, link, target_is_directory=True)
        return
    except (OSError, NotImplementedError):
        pass
    try:
        import _winapi
        _winapi.CreateJunction(str(target), str(link))
    except (ImportError, OSError) as e:
        pytest.skip(f"cannot create a symlink or junction here: {e}")


def test_diff_under_an_existing_symlink_is_refused(repo, tmp_path):
    root, base = repo
    art = worker(root, base, tmp_path, "a", {f"{ENG}/cards/evil/BT21-099.yaml": b"id: BT21-099\n"})
    outside = tmp_path / "outside-dir"
    outside.mkdir()
    make_link(root / ENG / "cards" / "evil", outside)

    res = mw.apply_manifest_diff(root, art["diff"], art["manifest"], base)

    assert not res.ok and any("symlink" in e or "outside the repository" in e
                              for e in res.errors), res.errors
    assert list(outside.iterdir()) == []
    assert git(root, "diff", "--cached", "--name-only") == ""


# --- commands ------------------------------------------------------------------------------


def test_env_shim_round_trips_and_sets_the_environment(tmp_path):
    argv = mw.with_env({"FOO": "bar=baz"}, [sys.executable, "-c",
                                             "import os; print(os.environ['FOO'])"])
    assert mw.split_env(argv) == ({"FOO": "bar=baz"},
                                  [sys.executable, "-c", "import os; print(os.environ['FOO'])"])
    out = subprocess.run(argv, capture_output=True, text=True, check=True).stdout.strip()
    assert out == "bar=baz"
    assert mw.with_env({}, ["x"]) == ["x"]


def test_scoped_test_commands():
    T = "--test-threads=8"
    assert mw.scoped_test_commands({"full_suite_required": True, "side_binaries": ["dsl"]}) == [
        ["cargo", "test", "--manifest-path", f"{ENG}/Cargo.toml", "--test", "cards_behavioral", "--", T],
        ["cargo", "test", "--manifest-path", f"{ENG}/Cargo.toml", "--test", "dsl", "--", T]]
    scope = {"full_suite_required": False, "cards": ["BT21-029"],
             "side_binaries": ["cards_behavioral", "dsl"]}
    cmds = mw.scoped_test_commands(scope, [f"{CB}/bt21/bt21_029.rs", f"{CB}/bt21/mod.rs"])
    assert cmds[0][-4:] == ["bt21_029", "bt21::bt21_029", "bt21::", T]
    assert cmds[1][-3:] == ["dsl", "--", T]
    # a shared cards_behavioral file is not a set module: the whole binary runs
    whole = mw.scoped_test_commands({"cards": ["BT21-029"], "side_binaries": ["cards_behavioral"]},
                                    [f"{CB}/test_cards.rs"])
    assert whole == [["cargo", "test", "--manifest-path", f"{ENG}/Cargo.toml", "--test",
                      "cards_behavioral", "--", T]]
    assert mw.scoped_test_commands({"cards": [], "side_binaries": []}) == []


class FakeRunner:
    def __init__(self, responses=None):
        self.calls = []
        self.responses = responses or []   # [(predicate(bare argv) -> bool, (rc, out, err))]

    def __call__(self, argv, cwd, timeout):
        env, bare = mw.split_env(argv)
        self.calls.append({"env": env, "argv": bare, "cwd": cwd})
        for pred, resp in self.responses:
            if pred(bare):
                return resp
        return 0, "", ""


def is_impact(argv):
    return len(argv) > 1 and argv[1] == mw.IMPACT_SCOPE_SCRIPT


def test_compute_scope_parses_json_and_is_conservative(tmp_path):
    ok = FakeRunner([(is_impact, (0, json.dumps({"full_suite_required": False, "cards": ["BT21-029"],
                                                  "side_binaries": ["dsl"]}), ""))])
    scope, rec = mw.compute_scope(ok, str(tmp_path), [f"{ENG}/cards/bt21/BT21-029.yaml"])
    assert scope["cards"] == ["BT21-029"] and not scope["full_suite_required"]
    assert ok.calls[0]["argv"] == [sys.executable, mw.IMPACT_SCOPE_SCRIPT, "--json", "--path",
                                   f"{ENG}/cards/bt21/BT21-029.yaml"]
    assert rec["rc"] == 0 and "stdout" not in rec

    broken = FakeRunner([(is_impact, (1, "", "Traceback"))])
    scope, _ = mw.compute_scope(broken, str(tmp_path), ["x"])
    assert scope["full_suite_required"] and "impact_scope failed" in scope["reasons"][0]

    data = FakeRunner([(is_impact, (0, json.dumps({"full_suite_required": False, "cards": []}), ""))])
    scope, _ = mw.compute_scope(data, str(tmp_path), ["data/card_overrides.json"])
    assert scope["full_suite_required"]


def test_merge_wave_end_to_end_and_rerun(repo, tmp_path):
    root, base = repo
    a = worker(root, base, tmp_path, "a", CARD_029)
    b = worker(root, base, tmp_path, "b", {f"{ENG}/cards/bt21/BT21-030.yaml": b"id: BT21-030\n"})
    entries = [{"card": "BT21-029", **a}, {"card": "BT21-030", **b}]
    runner = FakeRunner([(is_impact, (0, json.dumps({"cards": ["BT21-029"], "side_binaries": []}), ""))])

    summary = mw.merge_wave(root, entries, base, runner=runner, approved=["BT21-029"], run_suite=True)

    assert summary["ok"] and summary["merged"] == ["BT21-029"] and summary["skipped"] == ["BT21-030"]
    pack = runner.calls[0]
    assert pack["argv"] == list(mw.PACK_CHECK_ARGV) and pack["env"]["RUST_MIN_STACK"] == "268435456"
    assert summary["pack_check"]["ok"]
    suite_argv = runner.calls[2]["argv"]
    assert suite_argv[:6] == ["cargo", "test", "--manifest-path", f"{ENG}/Cargo.toml", "--test",
                              "cards_behavioral"]
    assert "bt21_029" in suite_argv and "bt21::bt21_029" in suite_argv
    assert summary["suite"]["green"]
    assert "BT21-029 | merged" in mw.render_wave_summary(summary)

    rerun = FakeRunner()
    again = mw.merge_wave(root, entries, base, runner=rerun, approved=["BT21-029"])
    assert again["noop"] == ["BT21-029"] and again["merged"] == [] and again["ok"]
    assert rerun.calls == []                         # nothing changed: no pack build


def test_merge_wave_failing_entry_is_rolled_back(repo, tmp_path):
    root, base = repo
    dead = worker(root, base, tmp_path, "a",
                  {k: v for k, v in CARD_029.items() if not k.endswith("mod.rs")})
    summary = mw.merge_wave(root, [{"card": "BT21-029", **dead}], base, runner=FakeRunner())
    assert not summary["ok"] and summary["failed"] == ["BT21-029"]
    assert "dead module" in summary["rows"][0]["problems"][0]
    assert status(root) == ""

    fixed = mw.merge_wave(root, [{"card": "BT21-029", **dead}], base, runner=FakeRunner(),
                          register_missing=True)
    assert fixed["merged"] == ["BT21-029"]
    assert f"{CB}/bt21/mod.rs" in fixed["rows"][0]["registered"]


def test_pack_check_failure_fails_the_wave(repo, tmp_path):
    root, base = repo
    a = worker(root, base, tmp_path, "a", CARD_029)
    runner = FakeRunner([(lambda argv: argv[:2] == ["cargo", "build"], (101, "", "dsl parse errors"))])
    summary = mw.merge_wave(root, [{"card": "BT21-029", **a}], base, runner=runner)
    assert not summary["ok"] and summary["pack_check"]["ok"] is False
    assert "dsl parse errors" in summary["pack_check"]["tail"]
