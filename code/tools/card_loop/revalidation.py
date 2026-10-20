"""Opt-in remeasurement of existing scenarios without rewriting historical state.

Queue definitions are immutable per source-manifest generation. Append-only
hash-chained events record new SIM/ORACLE attempts. An interrupted attempt is
held, never automatically replayed. Oracle runs retain recordings but do not
write verdicts/backfill; independent reviewed recording precedes acceptance.
"""
from __future__ import annotations

import copy
import hashlib
import json
import os
import time
from contextlib import contextmanager, nullcontext
from dataclasses import asdict
from pathlib import Path

from .driver_contracts import ItemRecord
from .ledger import new_attempt_id
from .source_snapshot import validate_snapshot
from .stages.base import StageDeferred
from .stages.sim import SimExecutor
from .stages.oracle import OracleExecutor


def _digest(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True, ensure_ascii=False).encode()).hexdigest()


def _file_hash(path):
    try:
        return hashlib.sha256(Path(path).read_bytes()).hexdigest()
    except OSError as error:
        raise StageDeferred(f"revalidation input unavailable: {path}") from error


def directory(ctx):
    pin = getattr(ctx.config, "source_manifest_sha256", None)
    if not getattr(ctx.config, "source_manifest", None) or not pin:
        raise StageDeferred("revalidation requires a pinned source manifest")
    return Path(ctx.run_dir) / "revalidation" / pin


@contextmanager
def _file_lock(path):
    """OS lock auto-releases on process death; persisted STARTED still holds work."""
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("a+b") as file:
        if file.tell() == 0:
            file.write(b"0")
            file.flush()
        file.seek(0)
        try:
            if os.name == "nt":
                import msvcrt
                msvcrt.locking(file.fileno(), msvcrt.LK_NBLCK, 1)
            else:
                import fcntl
                fcntl.flock(file.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
        except OSError as error:
            raise StageDeferred("revalidation generation already locked") from error
        try:
            yield
        finally:
            file.seek(0)
            if os.name == "nt":
                msvcrt.locking(file.fileno(), msvcrt.LK_UNLCK, 1)
            else:
                fcntl.flock(file.fileno(), fcntl.LOCK_UN)


def locked(ctx):
    return _file_lock(directory(ctx) / "dispatch.lock")


def _runtime(ctx):
    binary = getattr(ctx, "harness_bin", None) or getattr(ctx.config, "harness_bin", None)
    if not binary:
        raise StageDeferred("revalidation requires an explicit current-tree harness")
    path = (Path(ctx.repo) / binary).resolve()
    return {"path": str(path), "sha256": _file_hash(path),
            "execution_timeout": ctx.config.job_timeout_s, "oracle_wait": ctx.config.oracle_timeout_s,
            "harness_root": str(Path(ctx.config.harness_root).resolve())}


def _manifest(ctx):
    validate_snapshot(ctx)
    return json.loads((Path(ctx.repo) / ctx.config.source_manifest).read_text(encoding="utf-8"))


def _scenario(ctx, item, name, digest=None):
    from tools.clause_coverage.exam_binding import _parse_scenario_header
    path = (Path(ctx.repo) / name).resolve()
    if not path.is_relative_to(Path(ctx.repo).resolve()):
        raise StageDeferred("revalidation scenario outside run tree")
    if digest is not None and _file_hash(path) != digest:
        raise StageDeferred("revalidation scenario changed; review a new generation")
    validate_snapshot(ctx, scenario=name)
    h = _parse_scenario_header(path.read_text(encoding="utf-8"))
    kind, ident = item.split(":", 1)
    valid = (h.get("interaction") or {}).get("id") == ident if kind == "interaction" else ident in [h.get("clause"), *(h.get("covers") or [])]
    if not valid:
        raise StageDeferred("scenario does not cover the revalidation item")
    return _file_hash(path)


def create_queue(ctx, selected, *, reviewer, preserved_claims=None):
    """Explicit approval entry point; never called by driver startup."""
    if not reviewer or not selected:
        raise StageDeferred("nonempty reviewed queue required")
    manifest = _manifest(ctx)
    expected = manifest["expected"]
    allowed = {"clause:" + i for i in expected["clause_ids"]} | {"interaction:" + i for i in expected["gating_interaction_ids"]}
    entries = {}
    for item, scenarios in selected.items():
        if item not in allowed or item not in ctx.items or not scenarios:
            raise StageDeferred("queue item must belong to full focused denominator and have a scenario")
        rec = ctx.items[item]
        if rec.state not in {"CONFIRMED", "TERMINAL", "UNAVAILABLE", "ESCALATED"}:
            raise StageDeferred("ordinary live items must not also enter the revalidation queue")
        entries[item] = {"item": item, "previous_state": rec.state,
                         "previous_attempts": copy.deepcopy(rec.attempts),
                         "previous_data": copy.deepcopy(rec.data),
                         "scenarios": {p: _scenario(ctx, item, p) for p in scenarios}}
    definition = {"version": 1, "run_id": ctx.run_id, "generation": ctx.config.source_manifest_sha256,
                  "reviewer": reviewer, "runtime": _runtime(ctx), "entries": entries,
                  "preserved_claims": _claim_inventory(ctx, preserved_claims or {})}
    definition["digest"] = _digest(definition)
    with locked(ctx):
        path = directory(ctx) / "queue.json"
        if path.exists():
            if json.loads(path.read_text(encoding="utf-8")) != definition:
                raise StageDeferred("immutable revalidation queue already exists; do not replace history")
        else:
            with path.open("x", encoding="utf-8") as out:
                out.write(json.dumps(definition, ensure_ascii=False, indent=2) + "\n")
                out.flush()
                os.fsync(out.fileno())
    return definition


def read_queue(ctx):
    from .revalidation_epochs import read_active
    return read_active(ctx)


def _read_queue_at(ctx, root):
    path = root / "queue.json"
    if not path.exists():
        return None
    try:
        definition = json.loads(path.read_text(encoding="utf-8"))
        body = {k: v for k, v in definition.items() if k != "digest"}
        if definition["digest"] != _digest(body) or definition["generation"] != ctx.config.source_manifest_sha256 or definition["run_id"] != ctx.run_id:
            raise ValueError("queue identity changed")
        rows = {i: dict(copy.deepcopy(e), status="SIM" if e["previous_state"] == "CONFIRMED" else "HOLD", attempts={}, data={}) for i,e in definition["entries"].items()}
        previous = definition["digest"]
        events = []
        events_path = root / "events.jsonl"
        if events_path.exists():
            for line in events_path.read_text(encoding="utf-8").splitlines():
                event = json.loads(line)
                content = {k:v for k,v in event.items() if k != "digest"}
                if event["digest"] != _digest(content) or event["previous"] != previous or event["sequence"] != len(events)+1:
                    raise ValueError("revalidation event chain changed")
                row = rows[event["item"]]
                if event["kind"] == "start":
                    row["attempts"][event["stage"]] = row["attempts"].get(event["stage"],0)+1
                row.update(event.get("update", {}))
                previous = event["digest"]
                events.append(event)
        return {"definition": definition, "items": rows, "events": events, "tip": previous, "directory": str(root)}
    except (OSError, ValueError, KeyError, TypeError) as error:
        raise StageDeferred(f"invalid revalidation history: {error}") from error


def _append(ctx, item, kind, update, **detail):
    queue = read_queue(ctx)
    event = dict(sequence=len(queue["events"])+1, previous=queue["tip"], item=item, kind=kind,
                 update=update, ts=ctx.now(), **detail)
    event["digest"] = _digest(event)
    with (Path(queue["directory"])/"events.jsonl").open("a", encoding="utf-8") as out:
        out.write(json.dumps(event, ensure_ascii=False) + "\n")
        out.flush()
        os.fsync(out.fileno())


def _claim_inventory(ctx, inventory):
    root = (Path(ctx.config.harness_root) / "claimed").resolve()
    if not isinstance(inventory, dict):
        raise StageDeferred("invalid reviewed preserved claim inventory")
    for name, digest in inventory.items():
        if (not isinstance(name, str) or Path(name).name != name or
                not name.endswith(".json") or (root/name).resolve().parent != root or
                not isinstance(digest, str) or len(digest) != 64 or
                any(c not in "0123456789abcdef" for c in digest)):
            raise StageDeferred("invalid reviewed preserved claim inventory")
        if _file_hash(root/name) != digest:
            raise StageDeferred("preserved claim changed since independent review")
    return dict(inventory)


def _idle_context(ctx, queue):
    cloned = copy.copy(ctx)
    cloned.revalidation_preserved_claims = queue["definition"]["preserved_claims"]
    return cloned


def require_oracle_idle(ctx):
    root = Path(ctx.config.harness_root)
    heartbeat = root / "harness.heartbeat"
    if not heartbeat.exists() or time.time()-heartbeat.stat().st_mtime > 30 or heartbeat.read_text(encoding="utf-8").strip() != "idle":
        raise StageDeferred("shared oracle is not freshly idle")
    inventory = _claim_inventory(ctx, getattr(ctx, "revalidation_preserved_claims", {}))
    if list((root/"jobs").glob("*.json")) or {p.name for p in (root/"claimed").glob("*.json")} != set(inventory):
        raise StageDeferred("shared oracle queue or claim is occupied by unreviewed work")


class ReviewOracleExecutor(OracleExecutor):
    def _restart_if_still_stalled(self, ctx, stall):
        return "not restarted: revalidation preserves shared player and holds the attempt for review"


def _review_context(ctx, queue=None, row=None):
    cloned = copy.copy(ctx)
    run = ctx.run_command
    def recording_only(argv, cwd, timeout):
        if queue is not None and "--oracle" in argv:
            if (Path(ctx.run_dir)/"STOP").exists():
                raise StageDeferred("STOP received; drain active work without another dispatch")
            for entry in ([row] if row is not None else queue["items"].values()):
                for name, digest in entry["scenarios"].items():
                    _scenario(ctx, entry["item"], name, digest)
                for name, digest in entry.get("deck_book_pins", {}).items():
                    if _file_hash(Path(ctx.repo)/name) != digest:
                        raise StageDeferred("SIM deck book changed before oracle dispatch")
            require_oracle_idle(_idle_context(ctx, queue))
            _manifest(ctx)
            from .revalidation_epochs import validate_epoch_inputs
            validate_epoch_inputs(ctx, queue["definition"])
            if queue["definition"]["runtime"] != _runtime(ctx):
                raise StageDeferred("runtime changed before oracle dispatch")
        # Use the standard supported recording-only route. Review must precede
        # explicit recording to the verdict store, preserving prior raw rows.
        return run([a for a in argv if a not in {"--verdicts", "--backfill"}], cwd, timeout)
    cloned.run_command = recording_only
    return cloned


def advance(ctx):
    """At most one SIM or ORACLE attempt; no author, automatic retry or state edit."""
    if not (directory(ctx)/"queue.json").exists():
        return False
    with locked(ctx):
        _manifest(ctx)
        if (Path(ctx.run_dir)/"STOP").exists():
            return False
        queue = read_queue(ctx)
        from .revalidation_epochs import validate_epoch_inputs
        validate_epoch_inputs(ctx, queue["definition"])
        if queue["definition"]["runtime"] != _runtime(ctx):
            raise StageDeferred("revalidation runtime changed; independent review required")
        row = next((r for r in queue["items"].values() if r["status"] in {"SIM", "ORACLE"}), None)
        if row is None:
            return False
        for name,digest in row["scenarios"].items():
            _scenario(ctx,row["item"],name,digest)
        stage = row["status"]
        oracle_lock = _file_lock(Path(ctx.config.harness_root)/"revalidation-dispatch.lock") if stage == "ORACLE" else nullcontext()
        with oracle_lock:
            if stage == "ORACLE":
                require_oracle_idle(_idle_context(ctx, queue))
                for name, digest in row.get("deck_book_pins", {}).items():
                    if _file_hash(Path(ctx.repo)/name) != digest:
                        raise StageDeferred("SIM deck book changed before ORACLE")
            attempt = new_attempt_id(ctx.run_id)
            _append(ctx,row["item"],"start",{"status":"STARTED"},stage=stage,attempt_id=attempt)
            item = ItemRecord(row["item"],stage,data={"scenario_paths":list(row["scenarios"]), **row["data"]})
            executor = SimExecutor() if stage == "SIM" else ReviewOracleExecutor()
            # An exception leaves STARTED durably visible; never repeat an unknown call.
            result = executor.run(ctx if stage == "SIM" else _review_context(ctx, queue, row),item)
            if queue["definition"]["runtime"] != _runtime(ctx):
                raise StageDeferred("runtime changed during attempt; independent review required")
            for name,digest in row["scenarios"].items():
                _scenario(ctx,row["item"],name,digest)
            status = "ORACLE" if stage == "SIM" and result.next_state == "ORACLE" else "REVIEW"
            deck_pins = {name:_file_hash(Path(ctx.repo)/name) for name in (result.data.get("deck_books") or {}).values() if name}
            _append(ctx,row["item"],"result",{"status":status,"data":result.data,"outcome":result.next_state,"reason":result.reason,"deck_book_pins":deck_pins,
                    "escalation":asdict(result.escalation) if result.escalation is not None else None},stage=stage,attempt_id=attempt)
        return True


def authorize_retry(ctx, item, *, reviewer):
    """One new SIM->ORACLE attempt after diagnosis; never resets old counters."""
    if not reviewer:
        raise StageDeferred("explicit independent retry review required")
    with locked(ctx):
        _manifest(ctx)
        row = read_queue(ctx)["items"][item]
        if row["status"] == "STARTED":
            raise StageDeferred("unknown outcome: inspect recording before any retry")
        if row["status"] != "REVIEW" or row["previous_state"] != "CONFIRMED":
            raise StageDeferred("historical decision/question requires separate reviewed resolution")
        _append(ctx,item,"retry_review",{"status":"SIM","data":{}},reviewer=reviewer)


def review_hold(ctx, item, *, reviewer, evidence):
    """Authorize measurement of a held decision; the old question stays open."""
    if not reviewer or not evidence:
        raise StageDeferred("independent decision review with evidence is required")
    with locked(ctx):
        _manifest(ctx)
        row = read_queue(ctx)["items"][item]
        if row["status"] != "HOLD":
            raise StageDeferred("only a held historical decision can be reviewed here")
        _append(ctx,item,"measurement_review",{"status":"SIM","data":{}},reviewer=reviewer,evidence=evidence)


def review_interruption(ctx, item, *, reviewer, evidence):
    """Record inspected recovery evidence, never infer that a crashed call failed."""
    if not reviewer or not evidence:
        raise StageDeferred("independent interruption evidence is required")
    with locked(ctx):
        _manifest(ctx)
        queue = read_queue(ctx)
        row = queue["items"][item]
        if row["status"] != "STARTED":
            raise StageDeferred("item has no unresolved interrupted attempt")
        require_oracle_idle(_idle_context(ctx, queue))
        _append(ctx,item,"interruption_review",{"status":"REVIEW","reason":"unknown result inspected; prior attempt preserved"},reviewer=reviewer,evidence=evidence)


def current_binding(ctx):
    from tools.clause_coverage.exam_binding import bind, bind_interactions
    manifest = _manifest(ctx)
    root = Path(ctx.repo)
    book = json.loads((root/manifest["clause_text_json"]).read_text(encoding="utf-8"))
    args = dict(current_clauses=book["clauses"])
    clauses = bind(manifest["pool"],root/"qa/dcgo-exams",root/"qa/qa-reports/exam-verdicts",interaction_denominator=root/manifest["interaction_denominator"],**args)
    interactions = bind_interactions(manifest["pool"],root/manifest["interaction_denominator"],root/"qa/qa-reports/exam-verdicts",card_qa_path=root/"data/card_qa.json",**args)
    rows = {"clause:"+r["clause_id"]:r for card in clauses["cards"].values() for r in card["clauses"]}
    rows.update({"interaction:"+r["interaction_id"]:r for card in interactions["cards"].values() for r in card["interactions"]})
    expected = {"clause:"+i for i in manifest["expected"]["clause_ids"]} | {"interaction:"+i for i in manifest["expected"]["gating_interaction_ids"]}
    if set(rows) != expected:
        raise StageDeferred("current binding does not retain full clause/gating denominator")
    return rows


def accept(ctx, item, *, reviewer):
    """Close only after independent review and separately recorded current verdict."""
    if not reviewer:
        raise StageDeferred("independent recording review required")
    with locked(ctx):
        queue = read_queue(ctx)
        from .revalidation_epochs import validate_epoch_inputs
        validate_epoch_inputs(ctx, queue["definition"])
        if queue["definition"]["runtime"] != _runtime(ctx):
            raise StageDeferred("recording runtime changed; independent review required")
        row = queue["items"][item]
        current = current_binding(ctx)[item]
        jobs = {r.get("job_id") for r in (row.get("data",{}).get("oracle_results") or {}).values()} - {None}
        if row["status"] != "REVIEW" or row.get("outcome") != "CONFIRMED" or current["verdict"] != "confirmed" or current.get("stored_record",{}).get("job_id") not in jobs:
            raise StageDeferred("current reviewed source-bound recording is not established")
        for name,digest in row["scenarios"].items():
            _scenario(ctx,item,name,digest)
        _append(ctx,item,"accept",{"status":"DONE"},reviewer=reviewer)


def completion_blocker(ctx):
    """Read-only full denominator guard; no queue/state creation on startup."""
    if not getattr(ctx.config,"source_manifest",None):
        return None
    try:
        from tools.clause_coverage.readiness import clause_is_adjudicated, interaction_is_adjudicated
        rows = current_binding(ctx)
        outstanding = [i for i,r in rows.items() if not (clause_is_adjudicated(r) if i.startswith("clause:") else interaction_is_adjudicated(r))]
        queue = read_queue(ctx)
        pending = [i for i,r in queue["items"].items() if r["status"] != "DONE"] if queue else []
        questions = [i for i,r in getattr(ctx,"items",{}).items() if r.state == "ESCALATED"]
        questions += queue.get("unresolved_escalations", []) if queue else []
        if outstanding or pending or questions:
            return f"source-bound evidence incomplete: {len(outstanding)} clause/gating rows, {len(pending)} revalidations, {len(questions)} preserved escalations"
        return None
    except (OSError, ValueError, KeyError, TypeError, StageDeferred) as error:
        return f"source-bound completion refused: {error}"
