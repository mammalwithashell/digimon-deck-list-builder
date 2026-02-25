"""Admin training management router: agents, jobs, gauntlets, deck library."""

from __future__ import annotations

import json
from typing import Any, Dict, List, Optional

from fastapi import APIRouter, Depends, HTTPException, Query, status
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from digimon_gym.agents.gauntlet import MetaGauntlet
from digimon_gym.db.auth import ROLE_ADMIN, require_roles
from digimon_gym.db.database import get_db
from digimon_gym.db.models import (
    Agent,
    Gauntlet,
    GauntletParticipant,
    MatchupResult,
    TrainingJob,
    User,
)
from digimon_gym.db.schemas import (
    AgentCreateRequest,
    AgentResponse,
    AgentUpdateRequest,
    ArchetypeInfoResponse,
    GauntletCreateRequest,
    GauntletDetailResponse,
    GauntletParticipantResponse,
    GauntletResponse,
    MatchupResultResponse,
    TrainingJobCreateRequest,
    TrainingJobResponse,
)

router = APIRouter(prefix="/admin/training", tags=["training"])


# ── Helpers ──────────────────────────────────────────────────────────────


def _load_json(raw: str | None, fallback: Any) -> Any:
    if not raw:
        return fallback
    try:
        return json.loads(raw)
    except json.JSONDecodeError:
        return fallback


def _agent_to_response(agent: Agent) -> AgentResponse:
    return AgentResponse(
        id=agent.id,
        name=agent.name,
        archetype=agent.archetype,
        algorithm=agent.algorithm,
        weights_path=agent.weights_path,
        deck_source=agent.deck_source,
        total_games=int(agent.total_games or 0),
        total_wins=int(agent.total_wins or 0),
        total_losses=int(agent.total_losses or 0),
        total_draws=int(agent.total_draws or 0),
        win_rate=float(agent.win_rate or 0.0),
        total_timesteps_trained=int(agent.total_timesteps_trained or 0),
        owner_id=agent.owner_id,
        created_at=agent.created_at,
        updated_at=agent.updated_at,
    )


def _job_to_response(job: TrainingJob) -> TrainingJobResponse:
    return TrainingJobResponse(
        id=job.id,
        job_type=job.job_type,
        status=job.status,
        agent_id=job.agent_id,
        opponent_agent_id=job.opponent_agent_id,
        opponent_type=job.opponent_type,
        gauntlet_id=job.gauntlet_id,
        gauntlet_stage=job.gauntlet_stage,
        total_games=int(job.total_games or 0),
        total_timesteps=int(job.total_timesteps or 0),
        completed_games=int(job.completed_games or 0),
        wins=int(job.wins or 0),
        losses=int(job.losses or 0),
        draws=int(job.draws or 0),
        win_rate=float(job.win_rate) if job.win_rate is not None else None,
        config=_load_json(job.config_json, {}),
        result=_load_json(job.result_json, None),
        error_text=job.error_text,
        worker_id=job.worker_id,
        device=job.device,
        created_by=job.created_by,
        started_at=job.started_at,
        completed_at=job.completed_at,
        created_at=job.created_at,
        updated_at=job.updated_at,
    )


def _participant_to_response(p: GauntletParticipant) -> GauntletParticipantResponse:
    return GauntletParticipantResponse(
        id=p.id,
        slot=p.slot,
        role=p.role,
        agent_id=p.agent_id,
        archetype_name=p.archetype_name,
        deck_source=p.deck_source,
        meta_share=float(p.meta_share or 0.0),
        threat_index=float(p.threat_index or 0.0),
        tournament_win_rate=float(p.tournament_win_rate) if p.tournament_win_rate is not None else None,
        expected_meta_win_rate=float(p.expected_meta_win_rate) if p.expected_meta_win_rate is not None else None,
    )


def _gauntlet_to_response(
    gauntlet: Gauntlet,
    participants: list[GauntletParticipant] | None = None,
) -> GauntletResponse:
    parts = participants if participants is not None else []
    return GauntletResponse(
        id=gauntlet.id,
        name=gauntlet.name,
        status=gauntlet.status,
        current_stage=int(gauntlet.current_stage or 0),
        stage1_games=int(gauntlet.stage1_games or 1000),
        stage2_games=int(gauntlet.stage2_games or 5000),
        stage3_games_per_matchup=int(gauntlet.stage3_games_per_matchup or 100),
        participants=[_participant_to_response(p) for p in parts],
        error_text=gauntlet.error_text,
        created_by=gauntlet.created_by,
        started_at=gauntlet.started_at,
        completed_at=gauntlet.completed_at,
        created_at=gauntlet.created_at,
        updated_at=gauntlet.updated_at,
    )


def _matchup_to_response(m: MatchupResult) -> MatchupResultResponse:
    return MatchupResultResponse(
        id=m.id,
        gauntlet_id=m.gauntlet_id,
        agent_a_id=m.agent_a_id,
        agent_b_id=m.agent_b_id,
        games_played=int(m.games_played or 0),
        a_wins=int(m.a_wins or 0),
        b_wins=int(m.b_wins or 0),
        draws=int(m.draws or 0),
        a_win_rate=float(m.a_win_rate or 0.0),
    )


# ── Agent CRUD ───────────────────────────────────────────────────────────


@router.post("/agents", response_model=AgentResponse, status_code=status.HTTP_201_CREATED)
async def create_agent(
    request: AgentCreateRequest,
    user: User = Depends(require_roles(ROLE_ADMIN)),
    db: AsyncSession = Depends(get_db),
):
    agent = Agent(
        name=request.name,
        archetype=request.archetype,
        algorithm=request.algorithm,
        deck_json=json.dumps(request.deck),
        deck_source=request.deck_source,
        weights_path="",
        owner_id=user.id,
    )
    db.add(agent)
    await db.commit()
    await db.refresh(agent)
    return _agent_to_response(agent)


@router.get("/agents", response_model=List[AgentResponse])
async def list_agents(
    archetype: Optional[str] = Query(None, min_length=1, max_length=100),
    limit: int = Query(100, ge=1, le=500),
    _: User = Depends(require_roles(ROLE_ADMIN)),
    db: AsyncSession = Depends(get_db),
):
    query = select(Agent).order_by(Agent.created_at.desc())
    if archetype:
        query = query.where(Agent.archetype == archetype)
    query = query.limit(limit)
    result = await db.execute(query)
    return [_agent_to_response(a) for a in result.scalars().all()]


@router.get("/agents/{agent_id}", response_model=AgentResponse)
async def get_agent(
    agent_id: str,
    _: User = Depends(require_roles(ROLE_ADMIN)),
    db: AsyncSession = Depends(get_db),
):
    result = await db.execute(select(Agent).where(Agent.id == agent_id))
    agent = result.scalar_one_or_none()
    if agent is None:
        raise HTTPException(status_code=404, detail="Agent not found")
    return _agent_to_response(agent)


@router.patch("/agents/{agent_id}", response_model=AgentResponse)
async def update_agent(
    agent_id: str,
    request: AgentUpdateRequest,
    _: User = Depends(require_roles(ROLE_ADMIN)),
    db: AsyncSession = Depends(get_db),
):
    result = await db.execute(select(Agent).where(Agent.id == agent_id))
    agent = result.scalar_one_or_none()
    if agent is None:
        raise HTTPException(status_code=404, detail="Agent not found")
    if request.name is not None:
        agent.name = request.name
    if request.archetype is not None:
        agent.archetype = request.archetype
    await db.commit()
    await db.refresh(agent)
    return _agent_to_response(agent)


@router.delete("/agents/{agent_id}")
async def delete_agent(
    agent_id: str,
    _: User = Depends(require_roles(ROLE_ADMIN)),
    db: AsyncSession = Depends(get_db),
):
    result = await db.execute(select(Agent).where(Agent.id == agent_id))
    agent = result.scalar_one_or_none()
    if agent is None:
        raise HTTPException(status_code=404, detail="Agent not found")
    await db.delete(agent)
    await db.commit()
    return {"id": agent_id, "deleted": True}


# ── Training Jobs ────────────────────────────────────────────────────────


_OPPONENT_TYPE_MAP = {
    "train_vs_greedy": "greedy",
    "train_vs_random": "random",
    "train_vs_agent": "agent",
    "evaluate": "agent",
}


@router.post("/jobs", response_model=TrainingJobResponse, status_code=status.HTTP_201_CREATED)
async def create_training_job(
    request: TrainingJobCreateRequest,
    user: User = Depends(require_roles(ROLE_ADMIN)),
    db: AsyncSession = Depends(get_db),
):
    # Validate agent exists
    agent = (await db.execute(select(Agent).where(Agent.id == request.agent_id))).scalar_one_or_none()
    if agent is None:
        raise HTTPException(status_code=404, detail="Agent not found")

    # Validate opponent agent if needed
    if request.job_type in ("train_vs_agent", "evaluate"):
        if not request.opponent_agent_id:
            raise HTTPException(
                status_code=400,
                detail=f"opponent_agent_id is required for job_type '{request.job_type}'",
            )
        opponent = (
            await db.execute(select(Agent).where(Agent.id == request.opponent_agent_id))
        ).scalar_one_or_none()
        if opponent is None:
            raise HTTPException(status_code=404, detail="Opponent agent not found")

    opponent_type = _OPPONENT_TYPE_MAP.get(request.job_type)

    job = TrainingJob(
        job_type=request.job_type,
        status="queued",
        agent_id=request.agent_id,
        opponent_agent_id=request.opponent_agent_id,
        opponent_type=opponent_type,
        total_timesteps=request.total_timesteps,
        total_games=request.total_games,
        config_json=json.dumps(request.config),
        created_by=user.id,
    )
    db.add(job)
    await db.commit()
    await db.refresh(job)
    return _job_to_response(job)


@router.get("/jobs", response_model=List[TrainingJobResponse])
async def list_training_jobs(
    status_filter: Optional[str] = Query(
        None, alias="status", pattern=r"^(queued|running|completed|failed|canceled)$"
    ),
    agent_id: Optional[str] = Query(None, min_length=1, max_length=128),
    gauntlet_id: Optional[str] = Query(None, min_length=1, max_length=128),
    limit: int = Query(100, ge=1, le=500),
    _: User = Depends(require_roles(ROLE_ADMIN)),
    db: AsyncSession = Depends(get_db),
):
    query = select(TrainingJob).order_by(TrainingJob.created_at.desc())
    if status_filter:
        query = query.where(TrainingJob.status == status_filter)
    if agent_id:
        query = query.where(TrainingJob.agent_id == agent_id)
    if gauntlet_id:
        query = query.where(TrainingJob.gauntlet_id == gauntlet_id)
    query = query.limit(limit)
    result = await db.execute(query)
    return [_job_to_response(j) for j in result.scalars().all()]


@router.get("/jobs/{job_id}", response_model=TrainingJobResponse)
async def get_training_job(
    job_id: str,
    _: User = Depends(require_roles(ROLE_ADMIN)),
    db: AsyncSession = Depends(get_db),
):
    result = await db.execute(select(TrainingJob).where(TrainingJob.id == job_id))
    job = result.scalar_one_or_none()
    if job is None:
        raise HTTPException(status_code=404, detail="Training job not found")
    return _job_to_response(job)


@router.post("/jobs/{job_id}/cancel")
async def cancel_training_job(
    job_id: str,
    _: User = Depends(require_roles(ROLE_ADMIN)),
    db: AsyncSession = Depends(get_db),
):
    result = await db.execute(select(TrainingJob).where(TrainingJob.id == job_id))
    job = result.scalar_one_or_none()
    if job is None:
        raise HTTPException(status_code=404, detail="Training job not found")
    if job.status not in ("queued", "running"):
        raise HTTPException(status_code=400, detail="Only queued or running jobs can be canceled")
    job.status = "canceled"
    await db.commit()
    return {"job_id": job_id, "status": "canceled"}


# ── Gauntlets ────────────────────────────────────────────────────────────


@router.post("/gauntlets", response_model=GauntletResponse, status_code=status.HTTP_201_CREATED)
async def create_gauntlet(
    request: GauntletCreateRequest,
    user: User = Depends(require_roles(ROLE_ADMIN)),
    db: AsyncSession = Depends(get_db),
):
    gauntlet = Gauntlet(
        name=request.name,
        status="configuring",
        stage1_games=request.stage1_games,
        stage2_games=request.stage2_games,
        stage3_games_per_matchup=request.stage3_games_per_matchup,
        config_json=json.dumps(request.config),
        created_by=user.id,
    )
    db.add(gauntlet)
    await db.flush()

    participants: list[GauntletParticipant] = []
    for slot, p in enumerate(request.participants):
        participant = GauntletParticipant(
            gauntlet_id=gauntlet.id,
            slot=slot,
            role=p.role,
            archetype_name=p.archetype_name,
            deck_json=json.dumps(p.deck),
            deck_source=p.deck_source,
            meta_share=p.meta_share,
            threat_index=p.threat_index,
        )
        db.add(participant)
        participants.append(participant)

    await db.commit()
    await db.refresh(gauntlet)
    for part in participants:
        await db.refresh(part)
    return _gauntlet_to_response(gauntlet, participants=participants)


@router.get("/gauntlets", response_model=List[GauntletResponse])
async def list_gauntlets(
    limit: int = Query(100, ge=1, le=500),
    _: User = Depends(require_roles(ROLE_ADMIN)),
    db: AsyncSession = Depends(get_db),
):
    result = await db.execute(
        select(Gauntlet).order_by(Gauntlet.created_at.desc()).limit(limit)
    )
    gauntlets = result.scalars().all()

    responses: list[GauntletResponse] = []
    for g in gauntlets:
        parts_result = await db.execute(
            select(GauntletParticipant)
            .where(GauntletParticipant.gauntlet_id == g.id)
            .order_by(GauntletParticipant.slot)
        )
        parts = parts_result.scalars().all()
        responses.append(_gauntlet_to_response(g, participants=parts))
    return responses


@router.get("/gauntlets/{gauntlet_id}", response_model=GauntletDetailResponse)
async def get_gauntlet_detail(
    gauntlet_id: str,
    _: User = Depends(require_roles(ROLE_ADMIN)),
    db: AsyncSession = Depends(get_db),
):
    result = await db.execute(select(Gauntlet).where(Gauntlet.id == gauntlet_id))
    gauntlet = result.scalar_one_or_none()
    if gauntlet is None:
        raise HTTPException(status_code=404, detail="Gauntlet not found")

    parts_result = await db.execute(
        select(GauntletParticipant)
        .where(GauntletParticipant.gauntlet_id == gauntlet_id)
        .order_by(GauntletParticipant.slot)
    )
    participants = parts_result.scalars().all()

    jobs_result = await db.execute(
        select(TrainingJob)
        .where(TrainingJob.gauntlet_id == gauntlet_id)
        .order_by(TrainingJob.created_at.desc())
    )
    jobs = jobs_result.scalars().all()

    matchup_matrix = _load_json(gauntlet.matchup_matrix_json, None)
    tournament_rankings = _load_json(gauntlet.tournament_rankings_json, None)

    return GauntletDetailResponse(
        gauntlet=_gauntlet_to_response(gauntlet, participants=participants),
        jobs=[_job_to_response(j) for j in jobs],
        matchup_matrix=matchup_matrix,
        tournament_rankings=tournament_rankings,
    )


@router.post("/gauntlets/{gauntlet_id}/start", response_model=GauntletResponse)
async def start_gauntlet(
    gauntlet_id: str,
    _: User = Depends(require_roles(ROLE_ADMIN)),
    db: AsyncSession = Depends(get_db),
):
    result = await db.execute(select(Gauntlet).where(Gauntlet.id == gauntlet_id))
    gauntlet = result.scalar_one_or_none()
    if gauntlet is None:
        raise HTTPException(status_code=404, detail="Gauntlet not found")
    if gauntlet.status != "configuring":
        raise HTTPException(status_code=400, detail="Gauntlet can only be started from configuring state")

    # Lazy import to avoid circular imports
    from digimon_gym.agents.gauntlet_orchestrator import gauntlet_orchestrator

    await gauntlet_orchestrator.start_gauntlet(db, gauntlet_id=gauntlet_id)
    await db.refresh(gauntlet)

    parts_result = await db.execute(
        select(GauntletParticipant)
        .where(GauntletParticipant.gauntlet_id == gauntlet_id)
        .order_by(GauntletParticipant.slot)
    )
    participants = parts_result.scalars().all()
    return _gauntlet_to_response(gauntlet, participants=participants)


@router.post("/gauntlets/{gauntlet_id}/cancel")
async def cancel_gauntlet(
    gauntlet_id: str,
    _: User = Depends(require_roles(ROLE_ADMIN)),
    db: AsyncSession = Depends(get_db),
):
    result = await db.execute(select(Gauntlet).where(Gauntlet.id == gauntlet_id))
    gauntlet = result.scalar_one_or_none()
    if gauntlet is None:
        raise HTTPException(status_code=404, detail="Gauntlet not found")

    gauntlet.status = "canceled"

    # Cancel all queued jobs for this gauntlet
    jobs_result = await db.execute(
        select(TrainingJob).where(
            TrainingJob.gauntlet_id == gauntlet_id,
            TrainingJob.status == "queued",
        )
    )
    for job in jobs_result.scalars().all():
        job.status = "canceled"

    await db.commit()
    return {"gauntlet_id": gauntlet_id, "status": "canceled"}


@router.get("/gauntlets/{gauntlet_id}/matchups", response_model=List[MatchupResultResponse])
async def get_gauntlet_matchups(
    gauntlet_id: str,
    _: User = Depends(require_roles(ROLE_ADMIN)),
    db: AsyncSession = Depends(get_db),
):
    # Verify gauntlet exists
    gauntlet = (
        await db.execute(select(Gauntlet).where(Gauntlet.id == gauntlet_id))
    ).scalar_one_or_none()
    if gauntlet is None:
        raise HTTPException(status_code=404, detail="Gauntlet not found")

    result = await db.execute(
        select(MatchupResult)
        .where(MatchupResult.gauntlet_id == gauntlet_id)
        .order_by(MatchupResult.created_at.desc())
    )
    return [_matchup_to_response(m) for m in result.scalars().all()]


# ── Deck Library ─────────────────────────────────────────────────────────


@router.get("/deck-library/archetypes", response_model=List[ArchetypeInfoResponse])
async def list_deck_library_archetypes(
    _: User = Depends(require_roles(ROLE_ADMIN)),
):
    mg = MetaGauntlet()
    mg.load()

    responses: list[ArchetypeInfoResponse] = []
    for stats in sorted(
        mg.archetypes.values(),
        key=lambda s: s.threat_index,
        reverse=True,
    ):
        sample = stats.decks[0].card_ids if stats.decks else []
        responses.append(
            ArchetypeInfoResponse(
                name=stats.archetype_name,
                display_card_id=stats.display_card_id,
                primary_color=stats.primary_color,
                meta_share=round(stats.digilab_meta_share, 4),
                threat_index=round(stats.threat_index, 4),
                decklists_count=len(stats.decks),
                sample_decklist=sample,
            )
        )
    return responses
