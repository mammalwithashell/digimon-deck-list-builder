"""Admin AI/task/batch/promotion router."""

from __future__ import annotations

import asyncio
import hashlib
import json
import os
from pathlib import Path
from typing import Any, Dict, List, Optional

from fastapi import APIRouter, Depends, HTTPException, Query, status
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from digimon_gym.ai.autofix_apply import (
    apply_validated_edits,
    run_profile_checks,
    validate_edit_payload,
)
from digimon_gym.ai.batch_orchestrator import (
    BatchOrchestrationError,
    batch_orchestrator,
)
from digimon_gym.ai.dispatcher import TaskDispatcher
from digimon_gym.ai.git_adapter import GitAdapter, GitCommandError
from digimon_gym.ai.set_run_orchestrator import SetRunOrchestrationError, set_run_orchestrator
from digimon_gym.db.auth import ROLE_ADMIN, require_roles
from digimon_gym.db.database import get_db
from digimon_gym.db.models import (
    AIFixBatch,
    AIFixBatchItem,
    AIFixApplyAudit,
    AISetRun,
    AISetRunItem,
    AITask,
    AITranspilerLearnRun,
    EngineBacklogItem,
    Issue,
    ScriptPromotionAudit,
    User,
)
from digimon_gym.db.schemas import (
    AIFixApplyAuditResponse,
    AIFixBatchCancelResponse,
    AIFixBatchCreateRequest,
    AIFixBatchCreateResponse,
    AIFixBatchDetailResponse,
    AIFixBatchItemResponse,
    AIFixBatchResponse,
    AISetRunCancelResponse,
    AISetRunCreateRequest,
    AISetRunDetailResponse,
    AISetRunItemResponse,
    AISetRunResponse,
    AITaskApplyFixResponse,
    AITaskCreateRequest,
    AITaskResponse,
    AITaskRetryResponse,
    AITranspilerLearnCreateRequest,
    AITranspilerLearnResponse,
    ApproveSetIssuesRequest,
    ApproveSetIssuesResponse,
    EngineBacklogCreateRequest,
    EngineBacklogResponse,
    PromotionRequest,
    PromotionResponse,
    TaskPromotionRequest,
)
from digimon_gym.engine.data.script_promotion import (
    ScriptPromotionError,
    promote_script_from_generated,
    scripts_root,
)

router = APIRouter(prefix="/admin", tags=["admin"])
dispatcher = TaskDispatcher()
_git = GitAdapter()
MAX_TASK_COST_USD = float(os.getenv("AI_TASK_MAX_COST_USD", "5.0"))
PROJECT_ROOT = Path(__file__).resolve().parents[3]


def _load_json(raw: str | None, fallback: Any) -> Any:
    if not raw:
        return fallback
    try:
        return json.loads(raw)
    except json.JSONDecodeError:
        return fallback


def _task_to_response(task: AITask) -> AITaskResponse:
    return AITaskResponse(
        id=task.id,
        task_type=task.task_type,
        payload=_load_json(task.payload_json, {}),
        status=task.status,
        result=_load_json(task.result_json, None),
        sanitized_input=_load_json(task.sanitized_input_json, {}),
        retrieval_refs=_load_json(task.retrieval_refs_json, []),
        model_name=task.model_name,
        cost_estimate=float(task.cost_estimate_usd or 0.0),
        cost_actual=float(task.cost_actual_usd or 0.0),
        input_tokens=int(task.input_tokens or 0),
        output_tokens=int(task.output_tokens or 0),
        error_text=task.error_text,
        attempts=int(task.attempts or 0),
        max_attempts=int(task.max_attempts or 0),
        created_by=task.created_by,
        batch_id=task.batch_id,
        run_mode=task.run_mode,
        scope_profile=task.scope_profile,
        started_at=task.started_at,
        completed_at=task.completed_at,
        created_at=task.created_at,
        updated_at=task.updated_at,
    )


def _promotion_to_response(p: ScriptPromotionAudit, *, batch_id: str | None = None) -> PromotionResponse:
    return PromotionResponse(
        id=p.id,
        card_id=p.card_id,
        set_id=p.set_id,
        module_name=p.module_name,
        generated_hash=p.generated_hash,
        frozen_hash=p.frozen_hash,
        manifest_version=p.manifest_version,
        promoted_by=p.promoted_by,
        ai_task_id=p.ai_task_id,
        batch_id=batch_id,
        notes=p.notes or "",
        created_at=p.created_at,
    )


def _backlog_to_response(item: EngineBacklogItem) -> EngineBacklogResponse:
    return EngineBacklogResponse(
        id=item.id,
        title=item.title,
        description=item.description,
        mechanic=item.mechanic,
        status=item.status,
        source_task_id=item.source_task_id,
        created_by=item.created_by,
        created_at=item.created_at,
        updated_at=item.updated_at,
    )


def _batch_to_response(batch: AIFixBatch) -> AIFixBatchResponse:
    return AIFixBatchResponse(
        id=batch.id,
        set_id=batch.set_id,
        run_mode=batch.run_mode,
        scope_profile=batch.scope_profile,
        status=batch.status,
        created_by=batch.created_by,
        model_name=batch.model_name,
        concurrency=int(batch.concurrency or 0),
        max_total_cost_usd=float(batch.max_total_cost_usd or 0.0),
        failure_rate_stop=float(batch.failure_rate_stop or 0.0),
        max_tasks=int(batch.max_tasks or 0),
        queued_count=int(batch.queued_count or 0),
        running_count=int(batch.running_count or 0),
        completed_count=int(batch.completed_count or 0),
        failed_count=int(batch.failed_count or 0),
        applied_count=int(batch.applied_count or 0),
        commit_count=int(batch.commit_count or 0),
        stopped_reason=batch.stopped_reason,
        pr_url=batch.pr_url,
        created_at=batch.created_at,
        updated_at=batch.updated_at,
    )


def _batch_item_to_response(item: AIFixBatchItem) -> AIFixBatchItemResponse:
    return AIFixBatchItemResponse(
        id=item.id,
        batch_id=item.batch_id,
        issue_id=item.issue_id,
        card_id=item.card_id,
        task_id=item.task_id,
        status=item.status,
        error_text=item.error_text,
        applied_at=item.applied_at,
        commit_sha=item.commit_sha,
        created_at=item.created_at,
        updated_at=item.updated_at,
    )


def _set_run_to_response(run: AISetRun) -> AISetRunResponse:
    return AISetRunResponse(
        id=run.id,
        set_id=run.set_id,
        status=run.status,
        stage=run.stage,
        run_mode=run.run_mode,
        scope_profile=run.scope_profile,
        model_name=run.model_name,
        created_by=run.created_by,
        max_total_cost_usd=float(run.max_total_cost_usd or 0.0),
        failure_rate_stop=float(run.failure_rate_stop or 0.0),
        max_fix_tasks=int(run.max_fix_tasks or 0),
        qa_total=int(run.qa_total or 0),
        qa_completed=int(run.qa_completed or 0),
        qa_failed=int(run.qa_failed or 0),
        review_total=int(run.review_total or 0),
        review_completed=int(run.review_completed or 0),
        review_failed=int(run.review_failed or 0),
        fix_total=int(run.fix_total or 0),
        fix_completed=int(run.fix_completed or 0),
        fix_failed=int(run.fix_failed or 0),
        fix_applied=int(run.fix_applied or 0),
        score_threshold=run.score_threshold,
        retranspile_total=int(run.retranspile_total or 0),
        retranspile_completed=int(run.retranspile_completed or 0),
        retranspile_failed=int(run.retranspile_failed or 0),
        stopped_reason=run.stopped_reason,
        created_at=run.created_at,
        updated_at=run.updated_at,
        completed_at=run.completed_at,
    )


def _set_run_item_to_response(item: AISetRunItem) -> AISetRunItemResponse:
    return AISetRunItemResponse(
        id=item.id,
        set_run_id=item.set_run_id,
        card_id=item.card_id,
        set_id=item.set_id,
        module_name=item.module_name,
        review_faithful=(None if item.review_faithful is None else bool(item.review_faithful)),
        issue_id=item.issue_id,
        qa_task_id=item.qa_task_id,
        review_task_id=item.review_task_id,
        fix_task_id=item.fix_task_id,
        fix_apply_status=item.fix_apply_status,
        transpile_score=item.transpile_score,
        retranspile_task_id=item.retranspile_task_id,
        commit_sha=item.commit_sha,
        pr_url=item.pr_url,
        error_text=item.error_text,
        created_at=item.created_at,
        updated_at=item.updated_at,
    )


def _learn_run_to_response(run: AITranspilerLearnRun) -> AITranspilerLearnResponse:
    return AITranspilerLearnResponse(
        id=run.id,
        source_set_run_id=run.source_set_run_id,
        status=run.status,
        clusters_found=int(run.clusters_found or 0),
        patches_proposed=int(run.patches_proposed or 0),
        pr_url=run.pr_url,
        created_at=run.created_at,
        completed_at=run.completed_at,
    )


def _sha256(path: str) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def _card_to_set_module(card_id: str) -> tuple[str, str]:
    module = card_id.strip().replace("-", "_").lower()
    set_id = module.split("_", 1)[0]
    return set_id, module


@router.post("/issues/approve-set", response_model=ApproveSetIssuesResponse)
async def approve_set_issues(
    request: ApproveSetIssuesRequest,
    user: User = Depends(require_roles(ROLE_ADMIN)),
    db: AsyncSession = Depends(get_db),
):
    set_id = request.set_id.strip().upper()
    status_filter = request.status_filter.strip().lower()
    matched = (
        await db.execute(
            select(Issue)
            .where(
                Issue.card_id.like(f"{set_id}-%"),
                Issue.status == status_filter,
            )
            .order_by(Issue.created_at.asc())
        )
    ).scalars().all()

    approved_ids: list[str] = []
    for issue in matched:
        if issue.status != "new":
            continue
        issue.status = "approved_for_ai"
        issue.approved_by = user.id
        approved_ids.append(issue.id)

    await db.commit()
    return ApproveSetIssuesResponse(
        set_id=request.set_id.strip().lower(),
        matched_count=len(matched),
        approved_count=len(approved_ids),
        skipped_count=max(0, len(matched) - len(approved_ids)),
        issue_ids=approved_ids,
    )


@router.post("/issues/{issue_id}/queue-fix", response_model=AITaskResponse, status_code=status.HTTP_201_CREATED)
async def queue_single_issue_fix(
    issue_id: str,
    user: User = Depends(require_roles(ROLE_ADMIN)),
    db: AsyncSession = Depends(get_db),
):
    issue = (await db.execute(select(Issue).where(Issue.id == issue_id))).scalar_one_or_none()
    if issue is None:
        raise HTTPException(status_code=404, detail="Issue not found")
    if issue.status != "approved_for_ai":
        raise HTTPException(status_code=400, detail="Issue must be approved_for_ai")

    set_id, module_name = _card_to_set_module(issue.card_id)
    payload = {
        "card_id": issue.card_id,
        "set_id": set_id,
        "module_name": module_name,
        "issue_id": issue.id,
        "issue_description": issue.description,
        "scope_profile": "script",
    }
    estimate = dispatcher.estimate_cost("script_autofix", payload)
    if estimate > MAX_TASK_COST_USD:
        raise HTTPException(
            status_code=400,
            detail=f"Estimated cost ${estimate:.4f} exceeds cap ${MAX_TASK_COST_USD:.4f}",
        )

    task = AITask(
        task_type="script_autofix",
        payload_json=json.dumps(payload),
        status="queued",
        model_name=None,
        cost_estimate_usd=estimate,
        max_attempts=1,
        created_by=user.id,
        run_mode="pr",
        scope_profile="script",
    )
    db.add(task)
    await db.commit()
    await db.refresh(task)
    return _task_to_response(task)


@router.post("/set-runs", response_model=AISetRunResponse, status_code=status.HTTP_201_CREATED)
async def create_set_run(
    request: AISetRunCreateRequest,
    user: User = Depends(require_roles(ROLE_ADMIN)),
    db: AsyncSession = Depends(get_db),
):
    try:
        run = await set_run_orchestrator.create_set_run(
            db,
            created_by=user.id,
            set_id=request.set_id,
            run_mode=request.run_mode,
            scope_profile=request.scope_profile,
            model_name=request.model_name,
            max_total_cost_usd=request.max_total_cost_usd,
            failure_rate_stop=request.failure_rate_stop,
            max_fix_tasks=request.max_fix_tasks,
            score_threshold=request.score_threshold,
        )
    except SetRunOrchestrationError as exc:
        raise HTTPException(status_code=400, detail=str(exc))
    return _set_run_to_response(run)


@router.get("/set-runs", response_model=List[AISetRunResponse])
async def list_set_runs(
    set_id: Optional[str] = Query(None, min_length=1, max_length=32),
    status_filter: Optional[str] = Query(None, alias="status", pattern=r"^(running|completed|failed|canceled)$"),
    limit: int = Query(100, ge=1, le=500),
    _: User = Depends(require_roles(ROLE_ADMIN)),
    db: AsyncSession = Depends(get_db),
):
    rows = await set_run_orchestrator.list_set_runs(
        db,
        set_id=set_id,
        status_filter=status_filter,
        limit=limit,
    )
    return [_set_run_to_response(row) for row in rows]


@router.get("/set-runs/{set_run_id}", response_model=AISetRunDetailResponse)
async def get_set_run(
    set_run_id: str,
    _: User = Depends(require_roles(ROLE_ADMIN)),
    db: AsyncSession = Depends(get_db),
):
    run = await set_run_orchestrator.get_set_run(db, set_run_id=set_run_id)
    if run is None:
        raise HTTPException(status_code=404, detail="Set run not found")
    items = await set_run_orchestrator.list_set_run_items(db, set_run_id=set_run_id)
    responses = [_set_run_item_to_response(item) for item in items]
    pr_queue = [item for item in responses if item.pr_url]
    pr_queue.sort(key=lambda item: item.card_id)
    return AISetRunDetailResponse(
        run=_set_run_to_response(run),
        items=responses,
        pr_queue=pr_queue,
    )


@router.post("/set-runs/{set_run_id}/cancel", response_model=AISetRunCancelResponse)
async def cancel_set_run(
    set_run_id: str,
    _: User = Depends(require_roles(ROLE_ADMIN)),
    db: AsyncSession = Depends(get_db),
):
    run = await set_run_orchestrator.cancel_set_run(db, set_run_id=set_run_id)
    if run is None:
        raise HTTPException(status_code=404, detail="Set run not found")
    return AISetRunCancelResponse(set_run_id=run.id, status=run.status)


@router.post("/ai-batches", response_model=AIFixBatchCreateResponse, status_code=status.HTTP_201_CREATED)
async def create_ai_batch(
    request: AIFixBatchCreateRequest,
    user: User = Depends(require_roles(ROLE_ADMIN)),
    db: AsyncSession = Depends(get_db),
):
    try:
        batch, queued_task_ids, cards, eligible_count = await batch_orchestrator.create_batch(
            db,
            created_by=user.id,
            set_id=request.set_id,
            run_mode=request.run_mode,
            scope_profile=request.scope_profile,
            model_name=request.model_name,
            concurrency=request.concurrency,
            max_total_cost_usd=request.max_total_cost_usd,
            failure_rate_stop=request.failure_rate_stop,
            max_tasks=request.max_tasks,
            dry_run=request.dry_run,
        )
    except BatchOrchestrationError as exc:
        raise HTTPException(status_code=400, detail=str(exc))

    return AIFixBatchCreateResponse(
        batch=_batch_to_response(batch) if batch else None,
        eligible_count=eligible_count,
        selected_count=len(cards),
        queued_task_ids=queued_task_ids,
        preview_only=bool(request.dry_run),
        cards=cards,
    )


@router.get("/ai-batches/preview", response_model=AIFixBatchCreateResponse)
async def preview_ai_batch(
    set_id: str = Query(..., min_length=1, max_length=32),
    max_tasks: int = Query(0, ge=0, le=5000),
    _: User = Depends(require_roles(ROLE_ADMIN)),
    db: AsyncSession = Depends(get_db),
):
    issues = await batch_orchestrator.preview_eligible_issues(db, set_id=set_id, max_tasks=max_tasks)
    cards = [issue.card_id for issue in issues]
    return AIFixBatchCreateResponse(
        batch=None,
        eligible_count=len(issues),
        selected_count=len(cards),
        queued_task_ids=[],
        preview_only=True,
        cards=cards,
    )


@router.get("/ai-batches", response_model=List[AIFixBatchResponse])
async def list_ai_batches(
    limit: int = Query(100, ge=1, le=500),
    _: User = Depends(require_roles(ROLE_ADMIN)),
    db: AsyncSession = Depends(get_db),
):
    rows = await batch_orchestrator.list_batches(db, limit=limit)
    return [_batch_to_response(row) for row in rows]


@router.get("/ai-batches/{batch_id}", response_model=AIFixBatchDetailResponse)
async def get_ai_batch(
    batch_id: str,
    _: User = Depends(require_roles(ROLE_ADMIN)),
    db: AsyncSession = Depends(get_db),
):
    batch = await batch_orchestrator.get_batch(db, batch_id=batch_id)
    if batch is None:
        raise HTTPException(status_code=404, detail="AI batch not found")
    items = await batch_orchestrator.list_batch_items(db, batch_id=batch_id)
    return AIFixBatchDetailResponse(
        batch=_batch_to_response(batch),
        items=[_batch_item_to_response(item) for item in items],
    )


@router.post("/ai-batches/{batch_id}/cancel", response_model=AIFixBatchCancelResponse)
async def cancel_ai_batch(
    batch_id: str,
    _: User = Depends(require_roles(ROLE_ADMIN)),
    db: AsyncSession = Depends(get_db),
):
    batch = await batch_orchestrator.cancel_batch(db, batch_id=batch_id)
    if batch is None:
        raise HTTPException(status_code=404, detail="AI batch not found")
    return AIFixBatchCancelResponse(batch_id=batch.id, status=batch.status)


@router.post("/ai-tasks", response_model=AITaskResponse, status_code=status.HTTP_201_CREATED)
async def create_ai_task(
    request: AITaskCreateRequest,
    user: User = Depends(require_roles(ROLE_ADMIN)),
    db: AsyncSession = Depends(get_db),
):
    estimated_cost = float(request.cost_estimate)
    if estimated_cost <= 0:
        estimated_cost = dispatcher.estimate_cost(
            request.task_type,
            request.payload,
            model_name=request.model_name,
        )
    if estimated_cost > MAX_TASK_COST_USD:
        raise HTTPException(
            status_code=400,
            detail=(
                f"Estimated cost ${estimated_cost:.4f} exceeds cap ${MAX_TASK_COST_USD:.4f}. "
                "Split batch or reduce context."
            ),
        )

    task = AITask(
        task_type=request.task_type,
        payload_json=json.dumps(request.payload),
        status="queued",
        model_name=request.model_name,
        cost_estimate_usd=estimated_cost,
        max_attempts=request.max_attempts,
        created_by=user.id,
        batch_id=request.batch_id,
        run_mode=request.run_mode,
        scope_profile=request.scope_profile,
    )
    db.add(task)
    await db.commit()
    await db.refresh(task)
    return _task_to_response(task)


@router.get("/ai-tasks", response_model=List[AITaskResponse])
async def list_ai_tasks(
    status_filter: Optional[str] = Query(None, alias="status", pattern=r"^(queued|running|completed|failed)$"),
    task_type: Optional[str] = Query(None, pattern=r"^(review_batch|qa_analysis|engine_audit|script_autofix)$"),
    run_mode: Optional[str] = Query(None, pattern=r"^(pr|main)$"),
    scope_profile: Optional[str] = Query(None, pattern=r"^(script|script_engine|script_engine_transpiler)$"),
    batch_id: Optional[str] = Query(None, min_length=1, max_length=128),
    limit: int = Query(100, ge=1, le=500),
    _: User = Depends(require_roles(ROLE_ADMIN)),
    db: AsyncSession = Depends(get_db),
):
    query = select(AITask).order_by(AITask.created_at.desc())
    if status_filter:
        query = query.where(AITask.status == status_filter)
    if task_type:
        query = query.where(AITask.task_type == task_type)
    if run_mode:
        query = query.where(AITask.run_mode == run_mode)
    if scope_profile:
        query = query.where(AITask.scope_profile == scope_profile)
    if batch_id:
        query = query.where(AITask.batch_id == batch_id)
    query = query.limit(limit)

    result = await db.execute(query)
    return [_task_to_response(task) for task in result.scalars().all()]


@router.get("/ai-tasks/{task_id}", response_model=AITaskResponse)
async def get_ai_task(
    task_id: str,
    _: User = Depends(require_roles(ROLE_ADMIN)),
    db: AsyncSession = Depends(get_db),
):
    result = await db.execute(select(AITask).where(AITask.id == task_id))
    task = result.scalar_one_or_none()
    if task is None:
        raise HTTPException(status_code=404, detail="AI task not found")
    return _task_to_response(task)


@router.post("/ai-tasks/{task_id}/retry", response_model=AITaskRetryResponse)
async def retry_ai_task(
    task_id: str,
    _: User = Depends(require_roles(ROLE_ADMIN)),
    db: AsyncSession = Depends(get_db),
):
    result = await db.execute(select(AITask).where(AITask.id == task_id))
    task = result.scalar_one_or_none()
    if task is None:
        raise HTTPException(status_code=404, detail="AI task not found")

    if task.status not in {"failed", "completed"}:
        raise HTTPException(status_code=400, detail="Only failed/completed tasks can be retried")
    if task.attempts >= task.max_attempts:
        raise HTTPException(status_code=400, detail="Task max attempts exceeded")

    task.status = "queued"
    task.error_text = None
    task.result_json = None
    task.started_at = None
    task.completed_at = None
    await db.commit()
    return AITaskRetryResponse(task_id=task.id, status=task.status)


def _apply_and_commit_single_task(
    *,
    task_id: str,
    card_id: str,
    set_id: str,
    module_name: str,
    scope_profile: str,
    edits: list[dict],
) -> dict[str, Any]:
    """Blocking helper: apply edits in a worktree, commit, and open a draft PR.

    Returns dict with keys: applied_files, check_outputs, commit_sha, pr_url.
    """
    _git.preflight(run_mode="pr")
    ctx = _git.prepare_worktree_for_task(task_id=task_id, run_mode="pr")
    applied_files = apply_validated_edits(repo_root=ctx.worktree_path, edits=edits)
    check_outputs = run_profile_checks(
        repo_root=ctx.worktree_path,
        scope_profile=scope_profile,
        applied_files=applied_files,
    )
    commit_sha = _git.commit_files(
        worktree_path=ctx.worktree_path,
        files=applied_files,
        message=f"AI autofix {card_id}",
    )
    if commit_sha is None:
        raise RuntimeError("No diff produced after apply — nothing to commit")
    pr_url = _git.push_pr_branch_and_open_draft_pr(
        worktree_path=ctx.worktree_path,
        branch_name=ctx.branch_name,
        title=f"AI autofix {card_id}",
        body=f"Automated fix for **{card_id}** (`{set_id}/{module_name}`).\n\nTask: `{task_id}`",
    )
    return {
        "applied_files": applied_files,
        "check_outputs": check_outputs,
        "commit_sha": commit_sha,
        "pr_url": pr_url,
    }


@router.post("/ai-tasks/{task_id}/apply-fix", response_model=AITaskApplyFixResponse)
async def apply_task_fix(
    task_id: str,
    user: User = Depends(require_roles(ROLE_ADMIN)),
    db: AsyncSession = Depends(get_db),
):
    task = (await db.execute(select(AITask).where(AITask.id == task_id))).scalar_one_or_none()
    if task is None:
        raise HTTPException(status_code=404, detail="AI task not found")
    if task.task_type != "script_autofix":
        raise HTTPException(status_code=400, detail="Only script_autofix tasks can be applied")
    if task.status != "completed":
        raise HTTPException(status_code=400, detail="Task must be completed")

    payload = _load_json(task.payload_json, {})
    result_payload = _load_json(task.result_json, {})
    set_id = str(payload.get("set_id", "")).strip().lower()
    module_name = str(payload.get("module_name", "")).strip().lower()
    card_id = str(payload.get("card_id", "")).strip().upper()
    scope_profile = str(task.scope_profile or payload.get("scope_profile") or "script")
    run_mode = str(task.run_mode or "main")
    if not card_id or not set_id or not module_name:
        raise HTTPException(status_code=400, detail="Task payload missing card_id/set_id/module_name")

    try:
        edits = validate_edit_payload(
            result_payload=result_payload,
            scope_profile=scope_profile,
            set_id=set_id,
            module_name=module_name,
        )
    except Exception as exc:
        db.add(
            AIFixApplyAudit(
                ai_task_id=task.id, batch_id=task.batch_id, card_id=card_id,
                scope_profile=scope_profile, run_mode=run_mode,
                applied_files_json="[]", check_outputs_json=json.dumps([]),
                commit_sha=None, pr_url=None, status="failed",
                error_text=str(exc), created_by=user.id,
            )
        )
        await db.commit()
        raise HTTPException(status_code=400, detail=str(exc))

    if run_mode == "pr":
        # ── PR mode: worktree → commit → draft PR ──────────────────
        try:
            info = await asyncio.to_thread(
                _apply_and_commit_single_task,
                task_id=task.id,
                card_id=card_id,
                set_id=set_id,
                module_name=module_name,
                scope_profile=scope_profile,
                edits=edits,
            )
        except (GitCommandError, Exception) as exc:
            db.add(
                AIFixApplyAudit(
                    ai_task_id=task.id, batch_id=task.batch_id, card_id=card_id,
                    scope_profile=scope_profile, run_mode=run_mode,
                    applied_files_json="[]", check_outputs_json=json.dumps([]),
                    commit_sha=None, pr_url=None, status="failed",
                    error_text=str(exc), created_by=user.id,
                )
            )
            await db.commit()
            raise HTTPException(status_code=400, detail=str(exc))

        db.add(
            AIFixApplyAudit(
                ai_task_id=task.id, batch_id=task.batch_id, card_id=card_id,
                scope_profile=scope_profile, run_mode=run_mode,
                applied_files_json=json.dumps(info["applied_files"]),
                check_outputs_json=json.dumps(info["check_outputs"]),
                commit_sha=info["commit_sha"], pr_url=info["pr_url"],
                status="applied", created_by=user.id,
            )
        )
        await db.commit()
        return AITaskApplyFixResponse(
            task_id=task.id, status="applied",
            applied_files=info["applied_files"],
            commit_sha=info["commit_sha"], pr_url=info["pr_url"],
        )

    # ── Main mode: direct disk write (existing behaviour) ───────
    try:
        applied_files = apply_validated_edits(repo_root=PROJECT_ROOT, edits=edits)
        check_outputs = run_profile_checks(
            repo_root=PROJECT_ROOT,
            scope_profile=scope_profile,
            applied_files=applied_files,
        )
    except Exception as exc:
        db.add(
            AIFixApplyAudit(
                ai_task_id=task.id, batch_id=task.batch_id, card_id=card_id,
                scope_profile=scope_profile, run_mode=run_mode,
                applied_files_json="[]", check_outputs_json=json.dumps([]),
                commit_sha=None, pr_url=None, status="failed",
                error_text=str(exc), created_by=user.id,
            )
        )
        await db.commit()
        raise HTTPException(status_code=400, detail=str(exc))

    db.add(
        AIFixApplyAudit(
            ai_task_id=task.id, batch_id=task.batch_id, card_id=card_id,
            scope_profile=scope_profile, run_mode=run_mode,
            applied_files_json=json.dumps(applied_files),
            check_outputs_json=json.dumps(check_outputs),
            commit_sha=None, pr_url=None, status="applied",
            created_by=user.id,
        )
    )
    await db.commit()
    return AITaskApplyFixResponse(
        task_id=task.id, status="applied",
        applied_files=applied_files, commit_sha=None, pr_url=None,
    )


@router.get("/applied-cards", response_model=List[AIFixApplyAuditResponse])
async def list_applied_cards(
    status_filter: Optional[str] = Query(None, alias="status", pattern=r"^(applied|failed)$"),
    set_id: Optional[str] = Query(None, min_length=1, max_length=32),
    limit: int = Query(100, ge=1, le=500),
    _user: User = Depends(require_roles(ROLE_ADMIN)),
    db: AsyncSession = Depends(get_db),
):
    query = select(AIFixApplyAudit).order_by(AIFixApplyAudit.created_at.desc())
    if status_filter:
        query = query.where(AIFixApplyAudit.status == status_filter)
    if set_id:
        prefix = f"{set_id.strip().upper()}-"
        query = query.where(AIFixApplyAudit.card_id.like(prefix + "%"))
    query = query.limit(limit)
    audits = (await db.execute(query)).scalars().all()

    # For batch-originated audits missing pr_url, fall back to batch.pr_url
    batch_ids = {a.batch_id for a in audits if a.batch_id and not a.pr_url}
    batch_pr_map: Dict[str, str] = {}
    if batch_ids:
        batches = (await db.execute(
            select(AIFixBatch).where(AIFixBatch.id.in_(batch_ids))
        )).scalars().all()
        batch_pr_map = {b.id: b.pr_url for b in batches if b.pr_url}

    results: List[AIFixApplyAuditResponse] = []
    for a in audits:
        pr_url = a.pr_url or (batch_pr_map.get(a.batch_id) if a.batch_id else None)
        results.append(AIFixApplyAuditResponse(
            id=a.id,
            ai_task_id=a.ai_task_id,
            batch_id=a.batch_id,
            card_id=a.card_id,
            scope_profile=a.scope_profile,
            run_mode=a.run_mode,
            applied_files=_load_json(a.applied_files_json, []),
            commit_sha=a.commit_sha,
            pr_url=pr_url,
            status=a.status,
            error_text=a.error_text,
            created_by=a.created_by,
            created_at=a.created_at,
        ))
    return results


@router.post("/ai-tasks/{task_id}/promote", response_model=PromotionResponse, status_code=status.HTTP_201_CREATED)
async def promote_task_card(
    task_id: str,
    request: TaskPromotionRequest,
    user: User = Depends(require_roles(ROLE_ADMIN)),
    db: AsyncSession = Depends(get_db),
):
    result = await db.execute(select(AITask).where(AITask.id == task_id))
    task = result.scalar_one_or_none()
    if task is None:
        raise HTTPException(status_code=404, detail="AI task not found")
    if task.task_type not in {"review_batch", "script_autofix"}:
        raise HTTPException(status_code=400, detail="Task type does not support task-linked promotion")
    if task.status != "completed":
        raise HTTPException(status_code=400, detail="Task must be completed before promotion")

    card_id = request.card_id.strip().upper()

    # --- Quality Gate 1: review_batch must find card faithful to card text ---
    if task.task_type == "review_batch":
        result_data = _load_json(task.result_json, {})
        cards_results = result_data.get("cards", {}) if isinstance(result_data, dict) else {}
        card_review = cards_results.get(card_id, {}) if isinstance(cards_results, dict) else {}
        if not card_review.get("faithful_to_card_text", False):
            raise HTTPException(
                status_code=409,
                detail=f"Cannot promote: review found card {card_id} is not faithful to card text",
            )

    # --- Quality Gate 2: script_autofix must have a successful apply audit ---
    if task.task_type == "script_autofix":
        audit_result = await db.execute(
            select(AIFixApplyAudit).where(
                AIFixApplyAudit.ai_task_id == task.id,
                AIFixApplyAudit.status == "applied",
            ).limit(1)
        )
        if audit_result.scalar_one_or_none() is None:
            raise HTTPException(
                status_code=409,
                detail=f"Cannot promote: no successful apply audit found for task {task_id}",
            )

    sanitized = _load_json(task.sanitized_input_json, {})
    payload = _load_json(task.payload_json, {})
    cards = sanitized.get("cards", []) if isinstance(sanitized, dict) else []
    if not cards and task.task_type == "script_autofix":
        cards = [
            {
                "card_id": payload.get("card_id"),
                "set_id": payload.get("set_id"),
                "module_name": payload.get("module_name"),
            }
        ]
    if not cards:
        cards = payload.get("cards", []) if isinstance(payload, dict) else []
    if not isinstance(cards, list):
        cards = []

    selected: Optional[dict[str, Any]] = None
    for entry in cards:
        if not isinstance(entry, dict):
            continue
        if str(entry.get("card_id", "")).strip().upper() == card_id:
            selected = entry
            break
    if selected is None:
        raise HTTPException(status_code=400, detail=f"Card {card_id} not found in task payload/sanitized_input")

    set_id = str(selected.get("set_id", "")).strip().lower()
    module_name = str(selected.get("module_name", "")).strip().lower()
    if not set_id or not module_name:
        raise HTTPException(status_code=400, detail=f"Task card entry for {card_id} is missing set_id/module_name")

    generated_path = scripts_root() / "generated" / set_id / f"{module_name}.py"
    if not generated_path.exists():
        raise HTTPException(status_code=400, detail=f"Generated script not found: {generated_path}")
    expected_generated_hash = _sha256(str(generated_path))

    try:
        promotion = promote_script_from_generated(
            card_id=card_id,
            set_id=set_id,
            module_name=module_name,
            expected_generated_hash=expected_generated_hash,
        )
    except ScriptPromotionError as exc:
        raise HTTPException(status_code=400, detail=str(exc))

    audit = ScriptPromotionAudit(
        card_id=card_id,
        set_id=set_id,
        module_name=module_name,
        generated_hash=promotion["generated_hash"],
        frozen_hash=promotion["frozen_hash"],
        manifest_version=int(promotion["manifest_version"]),
        ai_task_id=task.id,
        promoted_by=user.id,
        notes=request.notes,
    )
    db.add(audit)
    await db.commit()
    await db.refresh(audit)
    return _promotion_to_response(audit, batch_id=task.batch_id)


@router.post("/promotions", response_model=PromotionResponse, status_code=status.HTTP_201_CREATED)
async def promote_script(
    request: PromotionRequest,
    user: User = Depends(require_roles(ROLE_ADMIN)),
    db: AsyncSession = Depends(get_db),
):
    try:
        result = promote_script_from_generated(
            card_id=request.card_id,
            set_id=request.set_id,
            module_name=request.module_name,
            expected_generated_hash=request.expected_generated_hash,
        )
    except ScriptPromotionError as exc:
        raise HTTPException(status_code=400, detail=str(exc))

    batch_id = None
    if request.ai_task_id:
        task = (
            await db.execute(select(AITask).where(AITask.id == request.ai_task_id))
        ).scalar_one_or_none()
        if task is not None:
            batch_id = task.batch_id

    audit = ScriptPromotionAudit(
        card_id=request.card_id,
        set_id=request.set_id,
        module_name=request.module_name,
        generated_hash=result["generated_hash"],
        frozen_hash=result["frozen_hash"],
        manifest_version=int(result["manifest_version"]),
        ai_task_id=request.ai_task_id,
        promoted_by=user.id,
        notes=request.notes,
    )
    db.add(audit)
    await db.commit()
    await db.refresh(audit)
    return _promotion_to_response(audit, batch_id=batch_id)


@router.get("/promotions", response_model=List[PromotionResponse])
async def list_promotions(
    limit: int = Query(100, ge=1, le=500),
    _: User = Depends(require_roles(ROLE_ADMIN)),
    db: AsyncSession = Depends(get_db),
):
    promotions = (
        await db.execute(
            select(ScriptPromotionAudit)
            .order_by(ScriptPromotionAudit.created_at.desc())
            .limit(limit)
        )
    ).scalars().all()

    task_ids = [p.ai_task_id for p in promotions if p.ai_task_id]
    task_map: dict[str, str | None] = {}
    if task_ids:
        tasks = (
            await db.execute(select(AITask).where(AITask.id.in_(task_ids)))
        ).scalars().all()
        task_map = {task.id: task.batch_id for task in tasks}

    return [
        _promotion_to_response(p, batch_id=task_map.get(p.ai_task_id) if p.ai_task_id else None)
        for p in promotions
    ]


@router.post("/engine-backlog", response_model=EngineBacklogResponse, status_code=status.HTTP_201_CREATED)
async def create_engine_backlog_item(
    request: EngineBacklogCreateRequest,
    user: User = Depends(require_roles(ROLE_ADMIN)),
    db: AsyncSession = Depends(get_db),
):
    item = EngineBacklogItem(
        title=request.title,
        description=request.description,
        mechanic=request.mechanic,
        source_task_id=request.source_task_id,
        created_by=user.id,
        status="open",
    )
    db.add(item)
    await db.commit()
    await db.refresh(item)
    return _backlog_to_response(item)


@router.get("/engine-backlog", response_model=List[EngineBacklogResponse])
async def list_engine_backlog_items(
    status_filter: Optional[str] = Query(None, alias="status", pattern=r"^(open|in_progress|closed)$"),
    limit: int = Query(100, ge=1, le=500),
    _: User = Depends(require_roles(ROLE_ADMIN)),
    db: AsyncSession = Depends(get_db),
):
    query = select(EngineBacklogItem).order_by(EngineBacklogItem.created_at.desc())
    if status_filter:
        query = query.where(EngineBacklogItem.status == status_filter)
    query = query.limit(limit)
    result = await db.execute(query)
    return [_backlog_to_response(item) for item in result.scalars().all()]


@router.post("/transpiler-learn", response_model=AITranspilerLearnResponse, status_code=status.HTTP_201_CREATED)
async def create_transpiler_learn_run(
    req: AITranspilerLearnCreateRequest,
    db: AsyncSession = Depends(get_db),
    current_user: User = Depends(require_roles(ROLE_ADMIN)),
):
    """Trigger pattern learning from a completed set run's autofixes."""
    from digimon_gym.ai.pattern_learner import create_learn_run
    learn_run = await create_learn_run(
        db,
        source_set_run_id=req.source_set_run_id,
        min_cluster_size=req.min_cluster_size,
    )
    return _learn_run_to_response(learn_run)


@router.get("/transpiler-learn/{learn_run_id}", response_model=AITranspilerLearnResponse)
async def get_transpiler_learn_run(
    learn_run_id: str,
    db: AsyncSession = Depends(get_db),
    current_user: User = Depends(require_roles(ROLE_ADMIN)),
):
    """Get transpiler learn run status."""
    run = await db.get(AITranspilerLearnRun, learn_run_id)
    if not run:
        raise HTTPException(404, "Learn run not found")
    return _learn_run_to_response(run)
