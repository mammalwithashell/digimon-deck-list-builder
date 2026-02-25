"""Tests for RBAC, issue queue, AI task registry, promotions, and retrieval."""

from __future__ import annotations

import hashlib
import json

import pytest
from httpx import ASGITransport, AsyncClient
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession, async_sessionmaker, create_async_engine

import digimon_gym.ai.set_run_orchestrator as set_run_module
from digimon_gym.ai import retrieval as retrieval_module
from digimon_gym.ai.batch_orchestrator import batch_orchestrator
from digimon_gym.ai.client import OpenAIResponsesClient
from digimon_gym.ai.dispatcher import DispatchOutcome
from digimon_gym.ai.contracts import EngineCapabilityOutput, QATriageOutput, ScriptFidelityOutput
from digimon_gym.ai.retrieval import LocalRAGIndex, build_local_index, discover_source_files
from digimon_gym.ai.set_run_orchestrator import set_run_orchestrator
from digimon_gym.ai.worker import AITaskWorker
from digimon_gym.db.auth import (
    ROLE_ADMIN,
    ROLE_JUDGE,
    assign_role_to_user,
    get_user_role_names,
)
from digimon_gym.db.database import get_db
from digimon_gym.db.models import (
    AIFixBatch,
    AIFixBatchItem,
    AITask,
    AISetRunItem,
    Base,
    Issue,
    ScriptPromotionAudit,
    User,
)
from digimon_gym.engine.data import script_promotion


@pytest.fixture
async def db_engine():
    engine = create_async_engine(
        "sqlite+aiosqlite:///:memory:",
        echo=False,
        connect_args={"check_same_thread": False},
    )
    async with engine.begin() as conn:
        await conn.run_sync(Base.metadata.create_all)
    yield engine
    async with engine.begin() as conn:
        await conn.run_sync(Base.metadata.drop_all)
    await engine.dispose()


@pytest.fixture
async def session_factory(db_engine):
    return async_sessionmaker(db_engine, class_=AsyncSession, expire_on_commit=False)


@pytest.fixture
async def client(session_factory):
    from digimon_gym.api import app, ai_task_worker

    async def override_get_db():
        async with session_factory() as session:
            yield session

    # Disable background processing for deterministic endpoint tests.
    async def _noop():
        return None

    ai_task_worker.start = _noop  # type: ignore[assignment]
    ai_task_worker.stop = _noop  # type: ignore[assignment]

    app.dependency_overrides[get_db] = override_get_db
    transport = ASGITransport(app=app)
    async with AsyncClient(transport=transport, base_url="http://test") as c:
        yield c
    app.dependency_overrides.clear()


async def _register_and_login(client: AsyncClient, username: str) -> dict:
    await client.post(
        "/auth/register",
        json={
            "username": username,
            "email": f"{username}@example.com",
            "password": "secure-password-123",
        },
    )
    login = await client.post(
        "/auth/login",
        json={"username": username, "password": "secure-password-123"},
    )
    assert login.status_code == 200
    return login.json()


async def _grant_roles(session_factory, username: str, *roles: str) -> None:
    async with session_factory() as db:
        user = (
            await db.execute(select(User).where(User.username == username))
        ).scalar_one()
        for role in roles:
            await assign_role_to_user(db, user.id, role)
        await db.commit()


class TestRBACAndIssueFlow:
    async def test_register_assigns_player_role(self, client: AsyncClient, session_factory):
        tokens = await _register_and_login(client, "roleuser")
        me = await client.get("/users/me", headers={"Authorization": f"Bearer {tokens['access_token']}"})
        assert me.status_code == 200
        assert "player" in me.json()["roles"]
        async with session_factory() as db:
            user = (await db.execute(select(User).where(User.username == "roleuser"))).scalar_one()
            roles = await get_user_role_names(user.id, db)
            assert "player" in roles

    async def test_player_cannot_queue_admin_task(self, client: AsyncClient):
        tokens = await _register_and_login(client, "plainplayer")
        resp = await client.post(
            "/admin/ai-tasks",
            headers={"Authorization": f"Bearer {tokens['access_token']}"},
            json={
                "task_type": "engine_audit",
                "payload": {"mechanics": ["delayed deletion trigger"]},
            },
        )
        assert resp.status_code == 403

    async def test_issue_lifecycle_and_transition_rules(self, client: AsyncClient, session_factory):
        player_tokens = await _register_and_login(client, "issueplayer")
        judge_tokens = await _register_and_login(client, "issuejudge")
        await _grant_roles(session_factory, "issuejudge", ROLE_JUDGE)

        create_resp = await client.post(
            "/issues",
            headers={"Authorization": f"Bearer {player_tokens['access_token']}"},
            json={
                "card_id": "BT24-001",
                "description": "Effect timing seems wrong on delete trigger.",
                "source": "player",
                "severity": "high",
            },
        )
        assert create_resp.status_code == 201
        issue_id = create_resp.json()["id"]

        invalid = await client.patch(
            f"/issues/{issue_id}",
            headers={"Authorization": f"Bearer {judge_tokens['access_token']}"},
            json={"status": "resolved"},
        )
        assert invalid.status_code == 400

        approve = await client.patch(
            f"/issues/{issue_id}",
            headers={"Authorization": f"Bearer {judge_tokens['access_token']}"},
            json={"status": "approved_for_ai"},
        )
        assert approve.status_code == 200
        assert approve.json()["status"] == "approved_for_ai"

        resolve = await client.patch(
            f"/issues/{issue_id}",
            headers={"Authorization": f"Bearer {judge_tokens['access_token']}"},
            json={"status": "resolved", "resolution_notes": "Confirmed and fixed."},
        )
        assert resolve.status_code == 200
        assert resolve.json()["status"] == "resolved"


class TestAITasks:
    async def test_task_create_list_retry_flow(self, client: AsyncClient, session_factory):
        admin_tokens = await _register_and_login(client, "admintasks")
        await _grant_roles(session_factory, "admintasks", ROLE_ADMIN)

        create_resp = await client.post(
            "/admin/ai-tasks",
            headers={"Authorization": f"Bearer {admin_tokens['access_token']}"},
            json={
                "task_type": "engine_audit",
                "payload": {"mechanics": ["delayed deletion trigger"]},
                "cost_estimate": 0.1,
            },
        )
        assert create_resp.status_code == 201
        task_id = create_resp.json()["id"]

        list_resp = await client.get(
            "/admin/ai-tasks",
            headers={"Authorization": f"Bearer {admin_tokens['access_token']}"},
        )
        assert list_resp.status_code == 200
        assert any(task["id"] == task_id for task in list_resp.json())

        retry_before_fail = await client.post(
            f"/admin/ai-tasks/{task_id}/retry",
            headers={"Authorization": f"Bearer {admin_tokens['access_token']}"},
        )
        assert retry_before_fail.status_code == 400

        async with session_factory() as db:
            task = (await db.execute(select(AITask).where(AITask.id == task_id))).scalar_one()
            task.status = "failed"
            task.error_text = "forced fail"
            await db.commit()

        retry_resp = await client.post(
            f"/admin/ai-tasks/{task_id}/retry",
            headers={"Authorization": f"Bearer {admin_tokens['access_token']}"},
        )
        assert retry_resp.status_code == 200
        assert retry_resp.json()["status"] == "queued"

    async def test_worker_processes_one_task(self, session_factory, monkeypatch):
        # Point the worker module at the test DB sessionmaker.
        from digimon_gym.ai import worker as worker_module

        monkeypatch.setattr(worker_module, "async_session", session_factory)
        worker = AITaskWorker()

        async with session_factory() as db:
            task = AITask(
                task_type="engine_audit",
                payload_json=json.dumps({"mechanics": ["memory gain gating"]}),
                status="queued",
                cost_estimate_usd=0.01,
            )
            db.add(task)
            await db.commit()
            await db.refresh(task)
            task_id = task.id

        def fake_dispatch(task_type, payload, model_name):
            assert task_type == "engine_audit"
            return DispatchOutcome(
                model_name="gpt-4.1-mini",
                result={"mechanics": [{"mechanic": "memory gain gating", "supported": True, "notes": ""}]},
                sanitized_input={"mechanics": ["memory gain gating"]},
                retrieval_refs=[],
                input_tokens=100,
                output_tokens=40,
                cost_actual=0.0002,
            )

        monkeypatch.setattr(worker.dispatcher, "run", fake_dispatch)
        processed = await worker._process_one_task()
        assert processed is True

        async with session_factory() as db:
            task = (await db.execute(select(AITask).where(AITask.id == task_id))).scalar_one()
            assert task.status == "completed"
            assert task.result_json is not None
            assert task.input_tokens == 100
            assert task.output_tokens == 40

    async def test_worker_downgrades_model_on_tpm_overflow(self, session_factory, monkeypatch):
        from digimon_gym.ai import worker as worker_module

        monkeypatch.setattr(worker_module, "async_session", session_factory)
        worker = AITaskWorker()

        async with session_factory() as db:
            task = AITask(
                task_type="script_autofix",
                payload_json=json.dumps(
                    {
                        "card_id": "EX10-028",
                        "set_id": "ex10",
                        "module_name": "ex10_028",
                        "scope_profile": "script",
                    }
                ),
                status="queued",
                model_name="gpt-4.1",
                cost_estimate_usd=0.01,
                max_attempts=3,
            )
            db.add(task)
            await db.commit()
            await db.refresh(task)
            task_id = task.id

        calls = {"n": 0}

        def fake_dispatch(task_type, payload, model_name):
            calls["n"] += 1
            if calls["n"] == 1:
                raise RuntimeError(
                    "Error code: 429 - {'error': {'message': 'Request too large for gpt-4.1 on tokens per min (TPM): Limit 30000, Requested 40884', 'code': 'rate_limit_exceeded'}}"
                )
            assert model_name == "gpt-4.1-mini"
            return DispatchOutcome(
                model_name="gpt-4.1-mini",
                result={"summary": "ok", "edits": []},
                sanitized_input={"ok": True},
                retrieval_refs=[],
                input_tokens=10,
                output_tokens=5,
                cost_actual=0.00001,
            )

        monkeypatch.setattr(worker.dispatcher, "run", fake_dispatch)

        processed = await worker._process_one_task()
        assert processed is True
        async with session_factory() as db:
            task = (await db.execute(select(AITask).where(AITask.id == task_id))).scalar_one()
            assert task.status == "queued"
            assert task.model_name == "gpt-4.1-mini"

        processed = await worker._process_one_task()
        assert processed is True
        async with session_factory() as db:
            task = (await db.execute(select(AITask).where(AITask.id == task_id))).scalar_one()
            assert task.status == "completed"
            assert task.model_name == "gpt-4.1-mini"

    async def test_admin_engine_backlog_endpoints(self, client: AsyncClient, session_factory):
        admin_tokens = await _register_and_login(client, "adminbacklog")
        await _grant_roles(session_factory, "adminbacklog", ROLE_ADMIN)

        create_resp = await client.post(
            "/admin/engine-backlog",
            headers={"Authorization": f"Bearer {admin_tokens['access_token']}"},
            json={
                "title": "Delayed trigger queue",
                "description": "Engine needs delayed trigger scheduling support.",
                "mechanic": "delayed deletion trigger",
            },
        )
        assert create_resp.status_code == 201
        item_id = create_resp.json()["id"]

        list_resp = await client.get(
            "/admin/engine-backlog",
            headers={"Authorization": f"Bearer {admin_tokens['access_token']}"},
        )
        assert list_resp.status_code == 200
        assert any(item["id"] == item_id for item in list_resp.json())

    async def test_promote_completed_review_task_card(self, client: AsyncClient, session_factory, monkeypatch, tmp_path):
        from digimon_gym.db.routers import admin_ai as admin_ai_router

        admin_tokens = await _register_and_login(client, "adminpromotefromtask")
        await _grant_roles(session_factory, "adminpromotefromtask", ROLE_ADMIN)

        async with session_factory() as db:
            task = AITask(
                task_type="review_batch",
                payload_json=json.dumps(
                    {"cards": [{"card_id": "BT24-001", "set_id": "bt24", "module_name": "bt24_001"}]}
                ),
                sanitized_input_json=json.dumps(
                    {"cards": [{"card_id": "BT24-001", "set_id": "bt24", "module_name": "bt24_001"}]}
                ),
                result_json=json.dumps(
                    {
                        "cards": {
                            "BT24-001": {
                                "faithful_to_card_text": True,
                                "engine_supported": True,
                                "issues": [],
                                "suggested_fixes": [],
                                "engine_requests": [],
                            }
                        }
                    }
                ),
                status="completed",
                cost_estimate_usd=0.01,
            )
            db.add(task)
            await db.commit()
            await db.refresh(task)
            task_id = task.id

        def fake_promote_script_from_generated(*, card_id, set_id, module_name, expected_generated_hash):
            assert card_id == "BT24-001"
            assert set_id == "bt24"
            assert module_name == "bt24_001"
            assert expected_generated_hash == "abc123"
            return {
                "card_id": card_id,
                "set_id": set_id,
                "module_name": module_name,
                "generated_hash": "abc123",
                "frozen_hash": "def456",
                "manifest_version": 42,
            }

        monkeypatch.setattr(admin_ai_router, "_sha256", lambda _: "abc123")
        monkeypatch.setattr(admin_ai_router, "promote_script_from_generated", fake_promote_script_from_generated)

        # Make generated path exist for endpoint preflight check.
        test_scripts = tmp_path / "scripts"
        generated_dir = test_scripts / "generated" / "bt24"
        generated_dir.mkdir(parents=True, exist_ok=True)
        (generated_dir / "bt24_001.py").write_text("print('x')\n", encoding="utf-8")
        monkeypatch.setattr(admin_ai_router, "scripts_root", lambda: test_scripts)

        resp = await client.post(
            f"/admin/ai-tasks/{task_id}/promote",
            headers={"Authorization": f"Bearer {admin_tokens['access_token']}"},
            json={"card_id": "BT24-001", "notes": "approved"},
        )
        assert resp.status_code == 201
        body = resp.json()
        assert body["card_id"] == "BT24-001"
        assert body["ai_task_id"] == task_id
        assert body["manifest_version"] == 42

        async with session_factory() as db:
            rows = (await db.execute(select(ScriptPromotionAudit))).scalars().all()
            assert len(rows) == 1
            assert rows[0].card_id == "BT24-001"
            assert rows[0].ai_task_id == task_id


class TestAIFixBatches:
    async def test_preview_only_counts_approved_set_issues(self, client: AsyncClient, session_factory):
        admin_tokens = await _register_and_login(client, "adminbatchpreview")
        await _grant_roles(session_factory, "adminbatchpreview", ROLE_ADMIN)
        judge_tokens = await _register_and_login(client, "batchjudge")
        await _grant_roles(session_factory, "batchjudge", ROLE_JUDGE)

        for card_id, status_value in [
            ("BT24-001", "approved_for_ai"),
            ("BT24-002", "approved_for_ai"),
            ("BT23-001", "approved_for_ai"),
            ("BT24-003", "new"),
        ]:
            create = await client.post(
                "/issues",
                headers={"Authorization": f"Bearer {judge_tokens['access_token']}"},
                json={
                    "card_id": card_id,
                    "description": f"Issue for {card_id}",
                    "source": "judge",
                    "severity": "medium",
                },
            )
            assert create.status_code == 201
            issue_id = create.json()["id"]
            if status_value != "new":
                update = await client.patch(
                    f"/issues/{issue_id}",
                    headers={"Authorization": f"Bearer {judge_tokens['access_token']}"},
                    json={"status": status_value},
                )
                assert update.status_code == 200

        preview = await client.get(
            "/admin/ai-batches/preview",
            headers={"Authorization": f"Bearer {admin_tokens['access_token']}"},
            params={"set_id": "bt24"},
        )
        assert preview.status_code == 200
        payload = preview.json()
        assert payload["preview_only"] is True
        assert payload["eligible_count"] == 2
        assert sorted(payload["cards"]) == ["BT24-001", "BT24-002"]

    async def test_create_batch_queues_script_autofix_tasks(self, client: AsyncClient, session_factory, monkeypatch):
        admin_tokens = await _register_and_login(client, "adminbatchcreate")
        await _grant_roles(session_factory, "adminbatchcreate", ROLE_ADMIN)
        judge_tokens = await _register_and_login(client, "batchcreatejudge")
        await _grant_roles(session_factory, "batchcreatejudge", ROLE_JUDGE)

        monkeypatch.setattr(batch_orchestrator.git, "preflight", lambda **_: None)

        for card_id in ["BT24-010", "BT24-011", "BT24-012"]:
            create = await client.post(
                "/issues",
                headers={"Authorization": f"Bearer {judge_tokens['access_token']}"},
                json={
                    "card_id": card_id,
                    "description": f"Issue for {card_id}",
                    "source": "judge",
                    "severity": "high",
                },
            )
            issue_id = create.json()["id"]
            approve = await client.patch(
                f"/issues/{issue_id}",
                headers={"Authorization": f"Bearer {judge_tokens['access_token']}"},
                json={"status": "approved_for_ai"},
            )
            assert approve.status_code == 200

        create_batch = await client.post(
            "/admin/ai-batches",
            headers={"Authorization": f"Bearer {admin_tokens['access_token']}"},
            json={
                "set_id": "bt24",
                "run_mode": "pr",
                "scope_profile": "script",
                "concurrency": 2,
                "max_total_cost_usd": 5.0,
                "failure_rate_stop": 0.3,
                "max_tasks": 3,
                "dry_run": False,
            },
        )
        assert create_batch.status_code == 201
        body = create_batch.json()
        assert body["preview_only"] is False
        assert body["batch"] is not None
        assert len(body["queued_task_ids"]) == 2

        async with session_factory() as db:
            batch_id = body["batch"]["id"]
            batch = (await db.execute(select(AIFixBatch).where(AIFixBatch.id == batch_id))).scalar_one()
            assert batch.run_mode == "pr"
            assert batch.scope_profile == "script"
            tasks = (await db.execute(select(AITask).where(AITask.batch_id == batch_id))).scalars().all()
            assert all(task.task_type == "script_autofix" for task in tasks)
            items = (await db.execute(select(AIFixBatchItem).where(AIFixBatchItem.batch_id == batch_id))).scalars().all()
            assert len(items) == 3
            queued_or_pending = {item.status for item in items}
            assert queued_or_pending.issubset({"queued", "pending"})

    async def test_main_mode_blocked_without_env_flag(self, client: AsyncClient, session_factory, monkeypatch):
        admin_tokens = await _register_and_login(client, "adminbatchmain")
        await _grant_roles(session_factory, "adminbatchmain", ROLE_ADMIN)

        monkeypatch.delenv("AI_APPLY_MAIN_ALLOWED", raising=False)
        resp = await client.post(
            "/admin/ai-batches",
            headers={"Authorization": f"Bearer {admin_tokens['access_token']}"},
            json={
                "set_id": "bt24",
                "run_mode": "main",
                "scope_profile": "script",
                "concurrency": 1,
                "max_total_cost_usd": 5.0,
                "failure_rate_stop": 0.3,
                "max_tasks": 0,
                "dry_run": False,
            },
        )
        assert resp.status_code == 400
        assert "AI_APPLY_MAIN_ALLOWED" in resp.text

    async def test_cancel_batch_marks_pending_items_canceled(self, client: AsyncClient, session_factory, monkeypatch):
        admin_tokens = await _register_and_login(client, "adminbatchcancel")
        await _grant_roles(session_factory, "adminbatchcancel", ROLE_ADMIN)
        judge_tokens = await _register_and_login(client, "batchcanceljudge")
        await _grant_roles(session_factory, "batchcanceljudge", ROLE_JUDGE)

        monkeypatch.setattr(batch_orchestrator.git, "preflight", lambda **_: None)

        create = await client.post(
            "/issues",
            headers={"Authorization": f"Bearer {judge_tokens['access_token']}"},
            json={
                "card_id": "BT24-020",
                "description": "Issue for BT24-020",
                "source": "judge",
                "severity": "high",
            },
        )
        issue_id = create.json()["id"]
        await client.patch(
            f"/issues/{issue_id}",
            headers={"Authorization": f"Bearer {judge_tokens['access_token']}"},
            json={"status": "approved_for_ai"},
        )

        create_batch = await client.post(
            "/admin/ai-batches",
            headers={"Authorization": f"Bearer {admin_tokens['access_token']}"},
            json={
                "set_id": "bt24",
                "run_mode": "pr",
                "scope_profile": "script",
                "concurrency": 1,
                "max_total_cost_usd": 5.0,
                "failure_rate_stop": 0.3,
                "max_tasks": 1,
                "dry_run": False,
            },
        )
        batch_id = create_batch.json()["batch"]["id"]

        cancel_resp = await client.post(
            f"/admin/ai-batches/{batch_id}/cancel",
            headers={"Authorization": f"Bearer {admin_tokens['access_token']}"},
        )
        assert cancel_resp.status_code == 200
        assert cancel_resp.json()["status"] in {"canceled", "running", "failed_no_changes"}

        async with session_factory() as db:
            batch = (await db.execute(select(AIFixBatch).where(AIFixBatch.id == batch_id))).scalar_one()
            assert batch.status in {"canceled", "failed_no_changes"}


class TestSetRuns:
    async def test_approve_set_endpoint_bulk_approves_new(self, client: AsyncClient, session_factory):
        admin_tokens = await _register_and_login(client, "adminapproveset")
        await _grant_roles(session_factory, "adminapproveset", ROLE_ADMIN)
        judge_tokens = await _register_and_login(client, "approvesetjudge")
        await _grant_roles(session_factory, "approvesetjudge", ROLE_JUDGE)

        issue_ids: list[str] = []
        for card_id in ["BT13-001", "BT13-002", "BT21-001"]:
            create = await client.post(
                "/issues",
                headers={"Authorization": f"Bearer {judge_tokens['access_token']}"},
                json={
                    "card_id": card_id,
                    "description": f"Issue for {card_id}",
                    "source": "judge",
                    "severity": "medium",
                },
            )
            assert create.status_code == 201
            issue_ids.append(create.json()["id"])

        reject = await client.patch(
            f"/issues/{issue_ids[1]}",
            headers={"Authorization": f"Bearer {judge_tokens['access_token']}"},
            json={"status": "rejected"},
        )
        assert reject.status_code == 200

        resp = await client.post(
            "/admin/issues/approve-set",
            headers={"Authorization": f"Bearer {admin_tokens['access_token']}"},
            json={"set_id": "bt13", "status_filter": "new"},
        )
        assert resp.status_code == 200
        payload = resp.json()
        assert payload["set_id"] == "bt13"
        assert payload["matched_count"] == 1
        assert payload["approved_count"] == 1
        assert payload["skipped_count"] == 0

    def test_discover_set_card_counts_bt13_bt21(self):
        bt13 = set_run_orchestrator.discover_set_cards(set_id="bt13")
        bt21 = set_run_orchestrator.discover_set_cards(set_id="bt21")
        assert len(bt13) == 112
        assert len(bt21) == 103

    async def test_create_set_run_and_progress_to_applied_fix(
        self, client: AsyncClient, session_factory, monkeypatch
    ):
        monkeypatch.setattr(set_run_module, "async_session", session_factory)
        admin_tokens = await _register_and_login(client, "adminsetrun")
        await _grant_roles(session_factory, "adminsetrun", ROLE_ADMIN)
        judge_tokens = await _register_and_login(client, "setrunjudge")
        await _grant_roles(session_factory, "setrunjudge", ROLE_JUDGE)

        issue = await client.post(
            "/issues",
            headers={"Authorization": f"Bearer {judge_tokens['access_token']}"},
            json={
                "card_id": "BT13-001",
                "description": "QA report: effect text mismatch.",
                "source": "judge",
                "severity": "high",
            },
        )
        assert issue.status_code == 201

        create = await client.post(
            "/admin/set-runs",
            headers={"Authorization": f"Bearer {admin_tokens['access_token']}"},
            json={
                "set_id": "bt13",
                "run_mode": "pr",
                "scope_profile": "script",
                "max_fix_tasks": 1,
            },
        )
        assert create.status_code == 201
        set_run_id = create.json()["id"]

        detail = await client.get(
            f"/admin/set-runs/{set_run_id}",
            headers={"Authorization": f"Bearer {admin_tokens['access_token']}"},
        )
        assert detail.status_code == 200
        assert len(detail.json()["items"]) == 112

        async with session_factory() as db:
            qa_task = (
                await db.execute(
                    select(AITask).where(
                        AITask.set_run_id == set_run_id,
                        AITask.task_type == "qa_analysis",
                    )
                )
            ).scalar_one()
            qa_task.status = "completed"
            qa_task.result_json = json.dumps({"classification": "script"})
            await db.commit()
            qa_task_id = qa_task.id

        await set_run_orchestrator.on_task_finished(qa_task_id)

        async with session_factory() as db:
            review_task = (
                await db.execute(
                    select(AITask).where(
                        AITask.set_run_id == set_run_id,
                        AITask.task_type == "review_batch",
                    )
                )
            ).scalar_one()

            items = (
                await db.execute(select(AISetRunItem).where(AISetRunItem.set_run_id == set_run_id))
            ).scalars().all()

            cards_result = {}
            for item in items:
                cards_result[item.card_id] = {
                    "faithful_to_card_text": item.card_id != "BT13-001",
                    "engine_supported": True,
                    "issues": [] if item.card_id != "BT13-001" else ["Mismatch"],
                    "suggested_fixes": [],
                    "engine_requests": [],
                }
            review_task.status = "completed"
            review_task.result_json = json.dumps({"cards": cards_result})
            await db.commit()
            review_task_id = review_task.id

        monkeypatch.setattr(
            set_run_orchestrator,
            "_apply_fix_task_pr_mode",
            lambda **_: {
                "applied_files": ["digimon_gym/engine/data/scripts/generated/bt13/bt13_001.py"],
                "check_outputs": [],
                "commit_sha": "abc123",
                "pr_url": "https://example.test/pr/1",
            },
        )

        await set_run_orchestrator.on_task_finished(review_task_id)

        async with session_factory() as db:
            fix_task = (
                await db.execute(
                    select(AITask).where(
                        AITask.set_run_id == set_run_id,
                        AITask.task_type == "script_autofix",
                    )
                )
            ).scalar_one()
            fix_task.status = "completed"
            fix_task.result_json = json.dumps({"summary": "ok", "edits": []})
            await db.commit()
            fix_task_id = fix_task.id

        await set_run_orchestrator.on_task_finished(fix_task_id)

        detail_after = await client.get(
            f"/admin/set-runs/{set_run_id}",
            headers={"Authorization": f"Bearer {admin_tokens['access_token']}"},
        )
        assert detail_after.status_code == 200
        payload = detail_after.json()
        assert payload["run"]["status"] == "completed"
        assert any(item["card_id"] == "BT13-001" and item["pr_url"] for item in payload["items"])

    async def test_create_set_run_bt21_has_103_items(self, client: AsyncClient, session_factory):
        admin_tokens = await _register_and_login(client, "adminsetrunbt21")
        await _grant_roles(session_factory, "adminsetrunbt21", ROLE_ADMIN)

        create = await client.post(
            "/admin/set-runs",
            headers={"Authorization": f"Bearer {admin_tokens['access_token']}"},
            json={
                "set_id": "bt21",
                "run_mode": "pr",
                "scope_profile": "script",
                "max_fix_tasks": 0,
            },
        )
        assert create.status_code == 201
        set_run_id = create.json()["id"]

        detail = await client.get(
            f"/admin/set-runs/{set_run_id}",
            headers={"Authorization": f"Bearer {admin_tokens['access_token']}"},
        )
        assert detail.status_code == 200
        assert len(detail.json()["items"]) == 103


class TestPromotionAndRetrieval:
    def test_promote_generated_script_updates_manifest(self, tmp_path, monkeypatch):
        scripts_dir = tmp_path / "scripts"
        generated_dir = scripts_dir / "generated" / "bt24"
        generated_dir.mkdir(parents=True, exist_ok=True)
        generated_file = generated_dir / "bt24_001.py"
        generated_file.write_text("print('generated')\n", encoding="utf-8")
        expected_hash = hashlib.sha256(generated_file.read_bytes()).hexdigest()
        manifest_path = scripts_dir / "_frozen_manifest.json"

        monkeypatch.setattr(script_promotion, "scripts_root", lambda: scripts_dir)
        result = script_promotion.promote_script_from_generated(
            card_id="BT24-001",
            set_id="bt24",
            module_name="bt24_001",
            expected_generated_hash=expected_hash,
            manifest_path=manifest_path,
        )
        assert (scripts_dir / "bt24" / "bt24_001.py").exists()
        assert result["card_id"] == "BT24-001"

        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        assert "BT24-001" in manifest["cards"]

    def test_local_rag_index_build_and_retrieve(self, tmp_path):
        src = tmp_path / "sources"
        src.mkdir(parents=True, exist_ok=True)
        (src / "rules.txt").write_text("Delay effects trigger from trash on main phase.", encoding="utf-8")
        (src / "keywords.md").write_text("Blocker and Jamming are keyword abilities.", encoding="utf-8")
        index_path = tmp_path / "index.json"

        payload = build_local_index(output_path=index_path, source_paths=[src], chunk_size=80, overlap=10)
        assert payload["chunk_count"] >= 2

        idx = LocalRAGIndex(index_path)
        hits = idx.retrieve("How does Delay trigger from trash?", k=3)
        assert hits
        assert any("Delay" in h["text"] for h in hits)

    def test_local_rag_discovers_pdf_sources_and_keeps_pdf_path(self, tmp_path, monkeypatch):
        src = tmp_path / "sources"
        src.mkdir(parents=True, exist_ok=True)
        pdf_path = src / "general_rule.pdf"
        pdf_path.write_bytes(b"%PDF-1.4 fake")
        (src / "rules.txt").write_text("Delay effects trigger from trash on main phase.", encoding="utf-8")
        index_path = tmp_path / "index.json"

        source_files = discover_source_files([src])
        assert pdf_path in source_files

        monkeypatch.setattr(
            retrieval_module,
            "_read_pdf_file",
            lambda _: "Comprehensive rules: Delay triggers from trash in main phase.",
        )
        payload = build_local_index(output_path=index_path, source_paths=[src], chunk_size=80, overlap=10)
        assert any(str(source).endswith("general_rule.pdf") for source in payload["sources"])

        idx = LocalRAGIndex(index_path)
        hits = idx.retrieve("Delay triggers from trash", k=3)
        assert any(str(hit["source"]).endswith("general_rule.pdf") for hit in hits)


class TestStructuredContracts:
    def test_contract_schemas_are_strict(self):
        fidelity_schema = ScriptFidelityOutput.model_json_schema()
        capability_schema = EngineCapabilityOutput.model_json_schema()
        triage_schema = QATriageOutput.model_json_schema()

        assert fidelity_schema.get("additionalProperties") is False
        assert capability_schema.get("additionalProperties") is False
        assert triage_schema.get("additionalProperties") is False

    def test_review_batch_cost_estimate_scales_with_card_count(self):
        client = OpenAIResponsesClient()
        one_card_payload = {
            "cards": [{"card_id": "BT24-001", "set_id": "bt24", "module_name": "bt24_001"}]
        }
        five_card_payload = {
            "cards": [
                {"card_id": "BT24-001", "set_id": "bt24", "module_name": "bt24_001"},
                {"card_id": "BT24-002", "set_id": "bt24", "module_name": "bt24_002"},
                {"card_id": "BT24-003", "set_id": "bt24", "module_name": "bt24_003"},
                {"card_id": "BT24-004", "set_id": "bt24", "module_name": "bt24_004"},
                {"card_id": "BT24-005", "set_id": "bt24", "module_name": "bt24_005"},
            ]
        }
        one_cost = client.estimate_cost("review_batch", one_card_payload)
        five_cost = client.estimate_cost("review_batch", five_card_payload)

        assert five_cost > one_cost
        # review_batch dispatches one model call per card in current worker flow
        assert five_cost == pytest.approx(one_cost * 5, rel=0.05)

    def test_openai_strict_schema_has_all_required_fields(self):
        raw = ScriptFidelityOutput.model_json_schema()
        strict = OpenAIResponsesClient._to_openai_strict_json_schema(raw)
        expected = {
            "faithful_to_card_text",
            "engine_supported",
            "issues",
            "suggested_fixes",
            "engine_requests",
        }
        assert set(strict.get("required", [])) == expected
        assert strict.get("additionalProperties") is False

    def test_contract_models_validate_expected_shapes(self):
        fidelity = ScriptFidelityOutput.model_validate(
            {
                "faithful_to_card_text": True,
                "engine_supported": False,
                "issues": ["Delayed trigger missing"],
                "suggested_fixes": ["Add delayed trigger hook"],
                "engine_requests": ["Support delayed trigger queue"],
            }
        )
        capability = EngineCapabilityOutput.model_validate(
            {"mechanic": "delayed deletion trigger", "supported": False, "notes": "Missing scheduler"}
        )
        triage = QATriageOutput.model_validate(
            {
                "likely_root_cause": "Script effect misses once-per-turn flag",
                "reproducibility": "high",
                "classification": "script",
                "suggested_debugging_area": "generated bt24 effect builder",
            }
        )
        assert fidelity.engine_supported is False
        assert capability.supported is False
        assert triage.classification == "script"
