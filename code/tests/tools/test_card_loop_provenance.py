"""Provenance (tasks 4.2): produced_by stamps, Loop-* trailers, blame -> attempt."""
import subprocess

import pytest

from tools.card_loop.provenance import (
    add_trailers,
    blame_attempts,
    blame_lines,
    commit_attempts,
    commit_trailers,
    loop_commit_message,
    produced_by,
    stamp,
    stamp_yaml_text,
    yaml_produced_by,
)


# --- records ------------------------------------------------------------------------------

def test_stamp_top_level_and_nested_without_mutating():
    verdict = {"verdict": "confirmed"}
    out = stamp(verdict, "A1")
    assert out == {"verdict": "confirmed", "produced_by": "A1"} and verdict == {"verdict": "confirmed"}
    scen = {"card": "BT7-056", "meta": {"author": "x"}}
    s2 = stamp(scen, "A2", path=("meta", "produced_by"))
    assert s2["meta"] == {"author": "x", "produced_by": "A2"} and "produced_by" not in scen["meta"]
    assert produced_by(s2, path=("meta", "produced_by")) == "A2"
    assert produced_by(scen, path=("meta", "produced_by")) is None
    assert stamp({}, "A3", path=("meta", "produced_by")) == {"meta": {"produced_by": "A3"}}
    with pytest.raises(ValueError):
        stamp({"meta": "scalar"}, "A4", path=("meta", "produced_by"))
    with pytest.raises(ValueError):
        stamp({}, "has space")


def test_yaml_header_is_added_once_and_replaced():
    doc = "id: BT7-056\r\neffects: []\r\n"
    once = stamp_yaml_text(doc, "A1", "claude", "sonnet")
    assert once.startswith("# produced_by: A1 (claude/sonnet)\r\nid: BT7-056")
    twice = stamp_yaml_text(once, "A2", "codex", None)
    assert twice == "# produced_by: A2 (codex/default)\r\nid: BT7-056\r\neffects: []\r\n"
    assert yaml_produced_by(twice) == "A2"
    assert yaml_produced_by(doc) is None and yaml_produced_by("") is None


# --- trailers -----------------------------------------------------------------------------

def test_commit_trailers_shape():
    assert commit_trailers("A1", "codex", None) == ["Loop-Attempt: A1", "Loop-Model: codex/default"]
    assert commit_trailers("A1", "claude", "sonnet")[1] == "Loop-Model: claude/sonnet"
    with pytest.raises(ValueError):
        commit_trailers("", "claude", "sonnet")


def test_add_trailers_joins_an_existing_block_or_starts_one():
    plain = add_trailers("cards: fix BT7-056\n\nBody text.", ["Loop-Attempt: A1"])
    assert plain == "cards: fix BT7-056\n\nBody text.\n\nLoop-Attempt: A1\n"
    signed = "subj\n\nbody\n\nCo-Authored-By: X <x@example.invalid>\n"
    joined = add_trailers(signed, ["Loop-Attempt: A1", "Co-Authored-By: X <x@example.invalid>"])
    assert joined == "subj\n\nbody\n\nCo-Authored-By: X <x@example.invalid>\nLoop-Attempt: A1\n"
    # a lone subject line that looks like "Key: value" is a subject, not a trailer block
    assert add_trailers("cards: fix", ["Loop-Attempt: A1"]) == "cards: fix\n\nLoop-Attempt: A1\n"


def test_loop_commit_message():
    msg = loop_commit_message("cards: implement BT7-056", "why", attempt_id="A1",
                              family="claude", model="sonnet",
                              extra_trailers=["Co-Authored-By: C <c@example.invalid>"])
    assert msg == ("cards: implement BT7-056\n\nwhy\n\nLoop-Attempt: A1\n"
                   "Loop-Model: claude/sonnet\nCo-Authored-By: C <c@example.invalid>\n")


# --- blame in a real temp repo ------------------------------------------------------------

@pytest.fixture
def repo(tmp_path, monkeypatch):
    cfg = tmp_path / "gitconfig"
    cfg.write_text("[user]\n\tname = t\n\temail = t@example.invalid\n[core]\n\tautocrlf = false\n",
                   encoding="utf-8")
    # isolate from the operator's global git config (hooks, signing, autocrlf);
    # provenance's own git subprocesses inherit this environment too
    monkeypatch.setenv("GIT_CONFIG_GLOBAL", str(cfg))
    monkeypatch.setenv("GIT_CONFIG_NOSYSTEM", "1")
    root = tmp_path / "repo"
    root.mkdir()

    def git(*args):
        return subprocess.run(["git", *args], cwd=root, check=True, capture_output=True,
                              text=True, encoding="utf-8").stdout.strip()

    def commit(text, message):
        (root / "card.yaml").write_bytes(text.encode("utf-8"))
        git("add", "card.yaml")
        msg = root.parent / "msg.txt"
        msg.write_bytes(message.encode("utf-8"))
        git("commit", "-q", "-F", str(msg))
        return git("rev-parse", "HEAD")

    git("init", "-q", "-b", "main")
    return root, commit


V1 = "id: BT7-056\nname: X\neffects:\n  - on_play\n  - draw: 1\n"


def test_blame_resolves_lines_to_attempts_through_trailers(repo):
    root, commit = repo
    c1 = commit(V1, loop_commit_message("cards: implement BT7-056", None, attempt_id="A-impl",
                                        family="claude", model="sonnet",
                                        extra_trailers=["Co-Authored-By: C <c@example.invalid>"]))
    commit(V1 + "notes: human\nmore: human\n", "human edit, no trailers\n")
    v3 = V1.replace("  - draw: 1", "  - draw: 2") + "notes: human\nmore: human\n"
    c3 = commit(v3, loop_commit_message("cards: fix BT7-056", None, attempt_id="A-fix",
                                        family="codex", model=None))

    assert commit_attempts(root, c1) == ("A-impl",)
    got = blame_attempts(root, "card.yaml", [(1, 7)])
    assert got == {1: "A-impl", 2: "A-impl", 3: "A-impl", 4: "A-impl", 5: "A-fix",
                   6: None, 7: None}
    # Spec scenario "Blame resolves to an attempt": the fix edited line 5; blaming that
    # line at the fix's parent resolves it to the attempt that originally wrote it.
    assert blame_attempts(root, "card.yaml", [5], rev=f"{c3}^") == {5: "A-impl"}
    # several ranges in one call
    assert blame_attempts(root, "card.yaml", [1, (6, 7)]) == {1: "A-impl", 6: None, 7: None}


def test_blame_is_ambiguous_for_multi_attempt_commits_and_none_for_uncommitted(repo):
    root, commit = repo
    msg = add_trailers("merge wave\n", ["Loop-Attempt: A1", "Loop-Attempt: A2",
                                       "Loop-Model: claude/sonnet"])
    commit(V1, msg)
    detail = blame_lines(root, "card.yaml", [(1, 1)])
    assert detail[0].attempts == ("A1", "A2")
    assert blame_attempts(root, "card.yaml", [1]) == {1: None}
    (root / "card.yaml").write_bytes(("changed\n" + V1.split("\n", 1)[1]).encode("utf-8"))
    wt = blame_lines(root, "card.yaml", [1, 2], rev=None)
    assert wt[0].commit is None and wt[0].attempts == ()
    assert wt[1].attempts == ("A1", "A2")


def test_bad_ranges_rejected():
    with pytest.raises(ValueError):
        blame_attempts(".", "x", [(5, 2)])
    with pytest.raises(ValueError):
        blame_attempts(".", "x", [0])
    assert blame_attempts(".", "x", []) == {}
