"""card-loop workers (group 3): contract, schemas, retry classes, fake worker,
vendor health, artifact capture. No test here calls a real CLI."""
from __future__ import annotations

import dataclasses
import hashlib
import json
import subprocess
from pathlib import Path

import pytest

from tools.card_loop.contracts import STAGES, TaskPacket, Usage, WorkerResult
from tools.card_loop.workers import base
from tools.card_loop.workers.base import (
    SCHEMAS_DIR,
    Worker,
    add_usage,
    capture_artifacts,
    classify_failure,
    parse_json_text,
    render_prompt,
    run_with_retry,
    schema_path,
    scrubbed_env,
    validate_result,
    with_validation_feedback,
)
from tools.card_loop.workers.fake import FakeWorker
from tools.card_loop.workers.health import VendorHealth

OWNED_STAGES = ("implement", "review", "author_clause", "author_interaction", "triage",
                "fix_card", "fix_engine")

VALID = {
    "implement": {"files": ["code/digimon-engine/cards/bt7/bt7_056.yaml"], "tests": ["bt7_056::play"],
                  "test_result_lines": ["test result: ok. 3 passed; 0 failed"],
                  "gaps": [{"kind": "dsl", "id": "dsl-gap-12", "summary": "needs X"}], "notes": ""},
    "review": {"verdict": "reject", "directives": [{"path": "a.yaml", "directive": "fix cost"},
                                                     {"path": None, "directive": "add a decline test"}],
               "summary": "two issues"},
    "author_clause": {"scenario_paths": ["qa/dcgo-exams/BT7/BT7-056.yaml"],
                      "covers": ["BT7-056#effect#0"], "notes": ""},
    "author_interaction": {"scenario_paths": [], "covers": ["BT7-056#effect#0"],
                           "notes": "no legal line reaches it"},
    "triage": {"classification": "dcgo_quirk", "citation": {"kind": "rule", "ref": "16-36"},
               "reasoning": "PDF says so"},
    "fix_card": {"files": ["a.yaml"], "tests": ["t"], "test_result_lines": [], "gaps": [],
                 "citation": {"kind": "ruling", "ref": "qa:Q1601"}, "notes": ""},
    "fix_engine": {"files": ["src/x.rs"], "tests": ["t"], "test_result_lines": [], "gaps": [],
                   "citation": {"kind": "dcgo", "ref": "BT7_056.cs:40"}, "notes": ""},
}

INVALID = {
    "implement": {"files": [], "tests": [], "test_result_lines": [], "gaps": [{"kind": "rust", "id": "x",
                                                                                "summary": ""}], "notes": ""},
    "review": {"verdict": "maybe", "directives": [], "summary": ""},
    "author_clause": {"scenario_paths": ["x"], "covers": []},
    "author_interaction": {"scenario_paths": "x", "covers": [], "notes": ""},
    "triage": {"classification": "ours_wrong", "citation": {"kind": "vibes", "ref": ""}, "reasoning": ""},
    "fix_card": {"files": [], "tests": [], "test_result_lines": [], "gaps": [], "citation": None,
                 "notes": ""},
    "fix_engine": {"files": [], "tests": [], "test_result_lines": [], "gaps": [],
                   "citation": {"kind": "rule", "ref": "1"}, "notes": "", "extra": 1},
}


def make_packet(tmp_path: Path | None = None, *, stage="triage", family="claude", item="clause:BT7-056#effect#0",
                attempt="att-1", prompt="do the thing", references=(), worktree=None) -> TaskPacket:
    return TaskPacket(stage=stage, family=family, item=item, attempt_id=attempt, prompt=prompt,
                      prompt_version="t@1", schema_path=str(schema_path(stage)),
                      worktree=str(worktree or tmp_path or "."), references=tuple(references))


def ok(result=None, cost=0.01, stage="triage") -> WorkerResult:
    return WorkerResult(status="ok", result=result if result is not None else VALID[stage],
                        usage=Usage(input_tokens=10, output_tokens=5, cost_usd=cost, wall_seconds=1.0))


def err(msg, cost=None) -> WorkerResult:
    return WorkerResult(status="error", error=msg, usage=Usage(cost_usd=cost, wall_seconds=0.5))


# ----------------------------------------------------------------------------- schemas


@pytest.mark.parametrize("stage", STAGES)
def test_stage_schema_exists_and_is_a_valid_schema(stage):
    import jsonschema

    schema = json.loads(schema_path(stage).read_text(encoding="utf-8"))
    jsonschema.Draft202012Validator.check_schema(schema)
    assert schema["description"].strip()


def _objects(node):
    if isinstance(node, dict):
        if node.get("type") == "object" or "properties" in node:
            yield node
        for v in node.values():
            yield from _objects(v)
    elif isinstance(node, list):
        for v in node:
            yield from _objects(v)


@pytest.mark.parametrize("stage", STAGES)
def test_stage_schema_is_strict_mode_compatible(stage):
    """Codex `--output-schema` uses OpenAI strict structured outputs: every object
    closed, every property required, root an object, no unsupported keywords."""
    schema = json.loads(schema_path(stage).read_text(encoding="utf-8"))
    assert schema["type"] == "object"
    for obj in _objects(schema):
        assert obj.get("additionalProperties") is False, obj
        assert sorted(obj.get("required", [])) == sorted(obj["properties"]), obj
    text = json.dumps(schema)
    for kw in ('"$ref"', '"format"', '"oneOf"', '"pattern"', '"$schema"'):
        assert kw not in text


def test_every_stage_has_a_schema():
    assert set(STAGES) <= {p.stem for p in SCHEMAS_DIR.glob("*.json")}


@pytest.mark.parametrize("stage", OWNED_STAGES)
@pytest.mark.parametrize("use_jsonschema", [True, False])
def test_valid_and_invalid_results(stage, use_jsonschema):
    schema = json.loads(schema_path(stage).read_text(encoding="utf-8"))
    assert validate_result(VALID[stage], schema, use_jsonschema=use_jsonschema) == []
    assert validate_result(INVALID[stage], schema, use_jsonschema=use_jsonschema)


def test_triage_citation_may_be_null():
    schema = json.loads(schema_path("triage").read_text(encoding="utf-8"))
    r = {"classification": "undetermined", "citation": None, "reasoning": "?"}
    for use in (True, False):
        assert validate_result(r, schema, use_jsonschema=use) == []


def test_minimal_validator_reports_paths():
    schema = json.loads(schema_path("review").read_text(encoding="utf-8"))
    errs = validate_result({"verdict": "accept", "directives": [{"path": 3, "directive": "x"}],
                            "summary": "s"}, schema, use_jsonschema=False)
    assert errs and errs[0].startswith("$.directives[0].path")


def test_parse_json_text_tolerates_fences():
    assert parse_json_text('```json\n{"ok": true}\n```') == {"ok": True}
    assert parse_json_text('{"ok": false}') == {"ok": False}
    assert parse_json_text("not json") is None
    assert parse_json_text(None) is None


# ----------------------------------------------------------------------------- failure classes


@pytest.mark.parametrize("text", [
    "Credit balance is too low",
    "Claude AI usage limit reached|1791170000",
    "You've hit your limit · resets 3pm",
    "You've hit your usage limit. Upgrade to Pro or try again later.",
    "Error code: 429 - {'error': {'type': 'insufficient_quota'}}",
    "You exceeded your current quota, please check your plan and billing details.",
])
def test_quota_texts(text):
    assert classify_failure(text) == "quota"


@pytest.mark.parametrize("text", [
    "API Error: 529 {\"type\":\"overloaded_error\"}",
    "rate_limit_error: Number of request tokens has exceeded your per-minute rate limit",
    "HTTP 503 Service Unavailable",
    "timeout: claude exceeded 3600s",
    "stream disconnected before completion",
    "Request failed with status 500",
])
def test_transient_texts(text):
    assert classify_failure(text) == "transient"


@pytest.mark.parametrize("text", ["Not logged in · Please run /login",
                                  "authentication_error: invalid x-api-key"])
def test_auth_texts(text):
    assert classify_failure(text) == "auth"


@pytest.mark.parametrize("text", [
    "card BT7-0502 not found",          # status codes only as whole words
    "used 15029 tokens",
    "error_max_budget_usd: Reached maximum budget ($0.25)",  # a budget stop is NOT quota
    "schema mismatch",
    "",
    None,
])
def test_permanent_or_unknown_texts(text):
    assert classify_failure(text) is None


# ----------------------------------------------------------------------------- retry


def test_ok_first_try(tmp_path):
    w = FakeWorker([ok()])
    res = run_with_retry(w, make_packet(tmp_path), sleep=lambda s: pytest.fail("slept"))
    assert res.status == "ok" and w.calls == 1


def test_transient_backs_off_exponentially_then_succeeds(tmp_path):
    slept = []
    w = FakeWorker([err("API Error: 529 overloaded", cost=0.002), err("rate limit"), ok(cost=0.01)])
    res = run_with_retry(w, make_packet(tmp_path), sleep=slept.append, base_delay=2.0)
    assert res.status == "ok"
    assert slept == [2.0, 4.0]
    assert w.calls == 3
    assert res.usage.cost_usd == pytest.approx(0.012)        # retries are billed to the attempt
    assert res.usage.wall_seconds == pytest.approx(2.0)
    assert {p.attempt_id for p in w.received} == {"att-1"}


def test_transient_gives_up_after_max(tmp_path):
    slept = []
    w = FakeWorker([err("overloaded")] * 4)
    res = run_with_retry(w, make_packet(tmp_path), sleep=slept.append, max_transient=3,
                         base_delay=1.0, max_delay=3.0)
    assert res.status == "error" and w.calls == 4
    assert slept == [1.0, 2.0, 3.0]                            # capped at max_delay
    assert "transient retry 3" in res.error


def test_schema_invalid_retries_once_with_the_error(tmp_path):
    bad = {"classification": "nope", "citation": None, "reasoning": "x"}
    w = FakeWorker([ok(result=bad), ok()])
    res = run_with_retry(w, make_packet(tmp_path), sleep=lambda s: pytest.fail("slept"))
    assert res.status == "ok" and w.calls == 2
    retry_prompt = w.received[1].prompt
    assert retry_prompt.startswith("do the thing")
    assert "previous result was rejected" in retry_prompt and "'nope'" in retry_prompt
    assert w.received[1].attempt_id == w.received[0].attempt_id


def test_schema_invalid_twice_is_returned(tmp_path):
    bad = {"classification": "nope", "citation": None, "reasoning": "x"}
    w = FakeWorker([ok(result=bad), WorkerResult(status="schema_invalid", error="still bad")])
    res = run_with_retry(w, make_packet(tmp_path))
    assert res.status == "schema_invalid" and w.calls == 2


def test_quota_stops_immediately_and_disables_vendor(tmp_path):
    health = VendorHealth()
    w = FakeWorker([WorkerResult(status="quota_exhausted", error="Credit balance is too low")])
    res = run_with_retry(w, make_packet(tmp_path), health=health, sleep=lambda s: pytest.fail("slept"))
    assert res.status == "quota_exhausted" and w.calls == 1
    assert not health.is_available("claude") and health.is_available("codex")
    # the next attempt for that family is not even sent
    res2 = run_with_retry(w, make_packet(tmp_path, attempt="att-2"), health=health)
    assert res2.status == "quota_exhausted" and w.calls == 1
    assert "disabled for this run" in res2.error


def test_quota_text_in_a_plain_error_is_promoted(tmp_path):
    w = FakeWorker([err("codex exited 1: You've hit your usage limit")])
    assert run_with_retry(w, make_packet(tmp_path)).status == "quota_exhausted"


def test_auth_error_is_not_retried_and_disables_vendor(tmp_path):
    health = VendorHealth()
    w = FakeWorker([err("Not logged in · Please run /login")])
    res = run_with_retry(w, make_packet(tmp_path), health=health, sleep=lambda s: pytest.fail("slept"))
    assert res.status == "error" and w.calls == 1
    assert health.reason("claude").startswith("auth:")


def test_permanent_error_is_not_retried(tmp_path):
    w = FakeWorker([err("error_max_budget_usd: Reached maximum budget ($0.25)")])
    res = run_with_retry(w, make_packet(tmp_path), sleep=lambda s: pytest.fail("slept"))
    assert res.status == "error" and w.calls == 1


def test_add_usage_cost_semantics():
    a = Usage(input_tokens=1, cost_usd=None, wall_seconds=1)
    assert add_usage(a, a).cost_usd is None
    b = Usage(input_tokens=2, cost_usd=0.5, cost_derived=True, wall_seconds=2)
    s = add_usage(a, b)
    assert (s.input_tokens, s.cost_usd, s.cost_derived, s.wall_seconds) == (3, 0.5, True, 3)


def test_render_prompt_lists_references_without_inlining(tmp_path):
    p = make_packet(tmp_path, references=("docs/digimon-rules/digest.md", "C:/base/DCGO/x.cs"))
    text = render_prompt(p)
    assert text.startswith("do the thing")
    assert "- docs/digimon-rules/digest.md" in text and "- C:/base/DCGO/x.cs" in text
    assert render_prompt(make_packet(tmp_path)) == "do the thing\n"


def test_with_validation_feedback_keeps_everything_but_the_prompt(tmp_path):
    p = make_packet(tmp_path)
    q = with_validation_feedback(p, "$.x: bad")
    assert dataclasses.replace(q, prompt=p.prompt) == p
    assert "$.x: bad" in q.prompt


def test_scrubbed_env_drops_parent_session_vars_only():
    env = scrubbed_env({"CLAUDECODE": "1", "CLAUDE_CODE_SESSION_ID": "s", "CLAUDE_CODE_MESSAGING_TOKEN": "t",
                        "CLAUDE_CODE_ENTRYPOINT": "claude-desktop", "CLAUDE_PID": "1",
                        "CLAUDE_CODE_OAUTH_TOKEN": "keep", "CLAUDE_CONFIG_DIR": "keep",
                        "ANTHROPIC_API_KEY": "keep", "PATH": "keep"})
    assert env == {"CLAUDE_CODE_OAUTH_TOKEN": "keep", "CLAUDE_CONFIG_DIR": "keep",
                   "ANTHROPIC_API_KEY": "keep", "PATH": "keep"}


# ----------------------------------------------------------------------------- fake worker


def test_fake_worker_is_a_worker():
    assert isinstance(FakeWorker(), Worker)


def test_fake_worker_lookup_precedence_and_recording(tmp_path):
    w = FakeWorker({
        ("triage", "clause:A"): ok(result={**VALID["triage"], "reasoning": "stage+item"}),
        "clause:A": ok(result={**VALID["triage"], "reasoning": "item"}),
        "triage": [ok(result={**VALID["triage"], "reasoning": "stage-1"}),
                   ok(result={**VALID["triage"], "reasoning": "stage-2"})],
    })
    r = [w.run(make_packet(tmp_path, item=i)).result["reasoning"]
         for i in ("clause:A", "clause:A", "clause:B", "clause:C")]
    assert r == ["stage+item", "item", "stage-1", "stage-2"]
    assert [p.item for p in w.received] == ["clause:A", "clause:A", "clause:B", "clause:C"]
    assert len(w.packets_for(item="clause:A")) == 2
    with pytest.raises(LookupError):
        w.run(make_packet(tmp_path, item="clause:D"))


def test_fake_worker_callable_and_default(tmp_path):
    w = FakeWorker({"review": lambda p: ok(result={"verdict": "accept", "directives": [],
                                                    "summary": p.item})},
                   default=err("unscripted"))
    assert w.run(make_packet(tmp_path, stage="review", item="card:X")).result["summary"] == "card:X"
    assert w.run(make_packet(tmp_path, stage="review", item="card:Y")).result["summary"] == "card:Y"
    assert w.run(make_packet(tmp_path, stage="triage")).error == "unscripted"


def test_fake_worker_family_mismatch(tmp_path):
    with pytest.raises(ValueError):
        FakeWorker([ok()], family="codex").run(make_packet(tmp_path))


def test_fake_worker_drives_run_with_retry_per_stage(tmp_path):
    w = FakeWorker({"triage": [err("overloaded"), ok()]})
    res = run_with_retry(w, make_packet(tmp_path), sleep=lambda s: None)
    assert res.status == "ok" and w.calls == 2


# ----------------------------------------------------------------------------- vendor health


def test_vendor_health_disable_once_and_notify():
    seen = []
    h = VendorHealth(clock=lambda: 42.0)
    h.subscribe(lambda fam, why: seen.append((fam, why)))
    assert h.available() == ("claude", "codex")
    assert h.disable("codex", "usage limit") is True
    assert h.disable("codex", "again") is False
    assert seen == [("codex", "usage limit")]
    assert h.available() == ("claude",)
    assert h.reason("codex") == "usage limit" and h.reason("claude") is None
    assert h.snapshot() == {"codex": {"reason": "usage limit", "at": 42.0}}
    with pytest.raises(ValueError):
        h.disable("gemini", "x")
    with pytest.raises(ValueError):
        h.is_available("gemini")


# ----------------------------------------------------------------------------- artifacts


def git(cwd, *args, check=True):
    return subprocess.run(["git", *args], cwd=cwd, capture_output=True, text=True, check=check).stdout


def init_repo(path: Path) -> str:
    path.mkdir(parents=True, exist_ok=True)
    git(path, "init", "-q")
    git(path, "config", "user.email", "t@example.invalid")
    git(path, "config", "user.name", "t")
    git(path, "config", "core.autocrlf", "false")
    (path / ".gitignore").write_text("ignored/\n")
    (path / "keep.txt").write_text("keep\n")
    (path / "edit.txt").write_text("before\n")
    (path / "gone.txt").write_text("bye\n")
    (path / "bin.dat").write_bytes(bytes(range(256)))
    git(path, "add", "-A")
    git(path, "commit", "-qm", "base")
    return git(path, "rev-parse", "HEAD").strip()


def test_capture_artifacts_full_worktree_vs_base(tmp_path):
    repo = tmp_path / "repo"
    base_sha = init_repo(repo)
    # a worker that commits something AND leaves staged, unstaged and untracked edits
    (repo / "committed.txt").write_text("c\n")
    git(repo, "add", "committed.txt")
    git(repo, "commit", "-qm", "worker commit")
    (repo / "edit.txt").write_text("after\n")
    (repo / "gone.txt").unlink()
    (repo / "new dir").mkdir()
    (repo / "new dir" / "new.yaml").write_bytes(b"x: 1\n")  # bytes: write_text adds CR on Windows
    (repo / "bin.dat").write_bytes(bytes(reversed(range(256))))
    (repo / "ignored").mkdir()
    (repo / "ignored" / "cache.bin").write_text("never captured")
    status_before = git(repo, "status", "--porcelain")

    out = capture_artifacts(repo, base_sha, tmp_path / "out")

    assert git(repo, "status", "--porcelain") == status_before   # real index untouched
    manifest = json.loads(Path(out["manifest"]).read_text())
    assert manifest["base_sha"] == base_sha
    by_path = {f["path"]: f for f in manifest["files"]}
    assert {p: f["status"] for p, f in by_path.items()} == {
        "bin.dat": "M", "committed.txt": "A", "edit.txt": "M", "gone.txt": "D", "new dir/new.yaml": "A"}
    assert by_path["new dir/new.yaml"]["sha256"] == hashlib.sha256(b"x: 1\n").hexdigest()
    assert by_path["gone.txt"]["sha256"] is None

    # the binary diff replays exactly onto a clean checkout of the base
    clean = tmp_path / "clean"
    git(tmp_path, "clone", "-q", str(repo), str(clean))
    git(clean, "checkout", "-q", base_sha)
    git(clean, "apply", "--binary", "--index", out["diff"])
    assert (clean / "edit.txt").read_text() == "after\n"
    assert (clean / "bin.dat").read_bytes() == bytes(reversed(range(256)))
    assert not (clean / "gone.txt").exists()
    assert not (clean / "ignored").exists()


def test_capture_artifacts_records_the_lf_normalised_hash_too(tmp_path):
    # A worker file with MIXED line endings matches no single-convention hash of
    # the LF-normalised diff after apply; `sha256_lf` is what the merge compares.
    repo = tmp_path / "repo"
    base_sha = init_repo(repo)
    mixed = b"a: 1\r\nb: 2\nc: 3\r\n"
    (repo / "mixed.yaml").write_bytes(mixed)
    out = capture_artifacts(repo, base_sha, tmp_path / "out")
    by_path = {f["path"]: f for f in json.loads(Path(out["manifest"]).read_text())["files"]}
    assert by_path["mixed.yaml"]["sha256"] == hashlib.sha256(mixed).hexdigest()
    assert by_path["mixed.yaml"]["sha256_lf"] == hashlib.sha256(b"a: 1\nb: 2\nc: 3\n").hexdigest()


def test_capture_artifacts_clean_worktree_is_empty(tmp_path):
    repo = tmp_path / "repo"
    base_sha = init_repo(repo)
    out = capture_artifacts(repo, base_sha, tmp_path / "out")
    assert Path(out["diff"]).read_bytes() == b""
    assert json.loads(Path(out["manifest"]).read_text())["files"] == []


def test_git_head(tmp_path):
    repo = tmp_path / "repo"
    assert base.git_head(tmp_path / "missing-dir") is None
    sha = init_repo(repo)
    assert base.git_head(repo) == sha
