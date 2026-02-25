"""Gauntlet orchestrator: manages the 3-stage training pipeline."""

from __future__ import annotations

import json
import logging
from datetime import datetime, timezone
from itertools import combinations

from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from digimon_gym.db.database import async_session
from digimon_gym.db.models import (
    Agent, Gauntlet, GauntletParticipant, MatchupResult, TrainingJob,
)

logger = logging.getLogger(__name__)

STEPS_PER_GAME_ESTIMATE = 50  # Conservative avg steps per game for timestep conversion


class GauntletOrchestrator:
    """Manages the 3-stage gauntlet pipeline by scheduling TrainingJobs."""

    async def start_gauntlet(self, gauntlet_id: str) -> None:
        """Transition from 'configuring' to 'stage_1' and schedule bootstrap jobs."""
        async with async_session() as db:
            result = await db.execute(select(Gauntlet).where(Gauntlet.id == gauntlet_id))
            gauntlet = result.scalar_one_or_none()
            if gauntlet is None:
                raise ValueError(f"Gauntlet {gauntlet_id} not found")
            if gauntlet.status != "configuring":
                raise ValueError(f"Gauntlet must be in 'configuring' status, got '{gauntlet.status}'")

            gauntlet.status = "stage_1"
            gauntlet.current_stage = 1
            gauntlet.started_at = datetime.now(timezone.utc)

            await self._schedule_stage1_jobs(db, gauntlet)
            await db.commit()

    async def on_job_finished(self, job_id: str) -> None:
        """Called by TrainingJobWorker when a gauntlet-linked job completes."""
        async with async_session() as db:
            result = await db.execute(select(TrainingJob).where(TrainingJob.id == job_id))
            job = result.scalar_one_or_none()
            if job is None or not job.gauntlet_id:
                return

            result = await db.execute(select(Gauntlet).where(Gauntlet.id == job.gauntlet_id))
            gauntlet = result.scalar_one_or_none()
            if gauntlet is None:
                return

            # Count remaining jobs for current stage
            pending_result = await db.execute(
                select(TrainingJob).where(
                    TrainingJob.gauntlet_id == gauntlet.id,
                    TrainingJob.gauntlet_stage == gauntlet.current_stage,
                    TrainingJob.status.in_(["queued", "running"]),
                )
            )
            remaining = pending_result.scalars().all()
            if remaining:
                return  # Not all jobs done yet

            # Check failure count
            failed_result = await db.execute(
                select(TrainingJob).where(
                    TrainingJob.gauntlet_id == gauntlet.id,
                    TrainingJob.gauntlet_stage == gauntlet.current_stage,
                    TrainingJob.status == "failed",
                )
            )
            failed_jobs = failed_result.scalars().all()
            total_result = await db.execute(
                select(TrainingJob).where(
                    TrainingJob.gauntlet_id == gauntlet.id,
                    TrainingJob.gauntlet_stage == gauntlet.current_stage,
                )
            )
            total_jobs = total_result.scalars().all()

            if total_jobs and len(failed_jobs) > len(total_jobs) // 2:
                gauntlet.status = "failed"
                gauntlet.error_text = f"Too many failures in stage {gauntlet.current_stage}: {len(failed_jobs)}/{len(total_jobs)}"
                gauntlet.completed_at = datetime.now(timezone.utc)
                await db.commit()
                return

            # Optimistic locking guard: prevent double stage transitions from parallel jobs
            original_stage = gauntlet.current_stage
            await db.refresh(gauntlet)
            if gauntlet.current_stage != original_stage:
                return

            # Transition to next stage
            if gauntlet.current_stage == 1:
                gauntlet.current_stage = 2
                gauntlet.status = "stage_2"
                await self._schedule_stage2_jobs(db, gauntlet)
            elif gauntlet.current_stage == 2:
                gauntlet.current_stage = 3
                gauntlet.status = "stage_3"
                await self._schedule_stage3_jobs(db, gauntlet)
            elif gauntlet.current_stage == 3:
                await self._finalize_stage3(db, gauntlet)
                gauntlet.status = "completed"
                gauntlet.completed_at = datetime.now(timezone.utc)

            await db.commit()

    async def _schedule_stage1_jobs(self, db: AsyncSession, gauntlet: Gauntlet) -> None:
        """For each participant: create Agent + TrainingJob(train_vs_greedy)."""
        result = await db.execute(
            select(GauntletParticipant)
            .where(GauntletParticipant.gauntlet_id == gauntlet.id)
            .order_by(GauntletParticipant.slot.asc())
        )
        participants = result.scalars().all()

        config = json.loads(gauntlet.config_json or "{}")
        algorithm = config.get("algorithm", "mlp")

        for p in participants:
            # Create agent
            agent = Agent(
                name=f"{p.archetype_name} Agent",
                archetype=p.archetype_name,
                algorithm=algorithm,
                weights_path="",
                deck_json=p.deck_json,
                deck_source=p.deck_source,
                owner_id=gauntlet.created_by,
            )
            db.add(agent)
            await db.flush()

            p.agent_id = agent.id

            # Create training job
            total_timesteps = gauntlet.stage1_games * STEPS_PER_GAME_ESTIMATE
            job = TrainingJob(
                job_type="train_vs_greedy",
                status="queued",
                agent_id=agent.id,
                opponent_type="greedy",
                gauntlet_id=gauntlet.id,
                gauntlet_stage=1,
                total_timesteps=total_timesteps,
                total_games=gauntlet.stage1_games,
                config_json=json.dumps(config.get("training_params", {})),
                created_by=gauntlet.created_by,
            )
            db.add(job)

        logger.info("Scheduled %d Stage 1 jobs for gauntlet %s", len(participants), gauntlet.id)

    async def _schedule_stage2_jobs(self, db: AsyncSession, gauntlet: Gauntlet) -> None:
        """Schedule agent-vs-agent training with meta-weighted/PFSP sampling."""
        result = await db.execute(
            select(GauntletParticipant)
            .where(GauntletParticipant.gauntlet_id == gauntlet.id)
            .order_by(GauntletParticipant.slot.asc())
        )
        participants = result.scalars().all()

        config = json.loads(gauntlet.config_json or "{}")
        total_timesteps = gauntlet.stage2_games * STEPS_PER_GAME_ESTIMATE

        # Build agent pool info
        agent_pool = {}
        for p in participants:
            if p.agent_id:
                agent_pool[p.agent_id] = {
                    "archetype": p.archetype_name,
                    "meta_share": p.meta_share,
                    "role": p.role,
                }

        for p in participants:
            if not p.agent_id:
                continue

            # Build opponent pool (exclude self)
            if p.role == "core":
                # Meta-weighted sampling: weight by meta_share
                opponent_weights = {}
                for aid, info in agent_pool.items():
                    if aid != p.agent_id:
                        opponent_weights[aid] = info["meta_share"] if info["meta_share"] > 0 else 0.01

                job_config = {
                    "sampling_mode": "meta_weighted",
                    "opponent_pool": opponent_weights,
                    **config.get("training_params", {}),
                }
            else:
                # PFSP: uniform initial weights over core agents only
                core_agents = {
                    aid: 1.0 / max(1, sum(1 for i in agent_pool.values() if i["role"] == "core"))
                    for aid, info in agent_pool.items()
                    if info["role"] == "core"
                }

                job_config = {
                    "sampling_mode": "pfsp",
                    "opponent_pool": core_agents,
                    **config.get("training_params", {}),
                }

            job = TrainingJob(
                job_type="train_vs_agent",
                status="queued",
                agent_id=p.agent_id,
                opponent_type="agent_pool",
                gauntlet_id=gauntlet.id,
                gauntlet_stage=2,
                total_timesteps=total_timesteps,
                total_games=gauntlet.stage2_games,
                config_json=json.dumps(job_config),
                created_by=gauntlet.created_by,
            )
            db.add(job)

        logger.info("Scheduled %d Stage 2 jobs for gauntlet %s", len(participants), gauntlet.id)

    async def _schedule_stage3_jobs(self, db: AsyncSession, gauntlet: Gauntlet) -> None:
        """Schedule all C(8,2) = 28 round-robin evaluation matchups between core agents."""
        result = await db.execute(
            select(GauntletParticipant).where(
                GauntletParticipant.gauntlet_id == gauntlet.id,
                GauntletParticipant.role == "core",
            ).order_by(GauntletParticipant.slot.asc())
        )
        core_participants = result.scalars().all()
        core_agents = [(p.agent_id, p) for p in core_participants if p.agent_id]

        job_count = 0
        for (a_id, _), (b_id, _) in combinations(core_agents, 2):
            job = TrainingJob(
                job_type="evaluate",
                status="queued",
                agent_id=a_id,
                opponent_agent_id=b_id,
                opponent_type="agent",
                gauntlet_id=gauntlet.id,
                gauntlet_stage=3,
                total_games=gauntlet.stage3_games_per_matchup,
                total_timesteps=0,
                config_json="{}",
                created_by=gauntlet.created_by,
            )
            db.add(job)
            job_count += 1

        logger.info("Scheduled %d Stage 3 evaluation jobs for gauntlet %s", job_count, gauntlet.id)

    async def _finalize_stage3(self, db: AsyncSession, gauntlet: Gauntlet) -> None:
        """Compute matchup matrix and expected tournament win rates."""
        # Load all matchup results for this gauntlet
        result = await db.execute(
            select(MatchupResult).where(MatchupResult.gauntlet_id == gauntlet.id)
        )
        matchups = result.scalars().all()

        # Load core participants for meta shares
        result = await db.execute(
            select(GauntletParticipant).where(
                GauntletParticipant.gauntlet_id == gauntlet.id,
                GauntletParticipant.role == "core",
            )
        )
        core_participants = result.scalars().all()

        # Build agent_id -> participant map
        agent_to_participant = {p.agent_id: p for p in core_participants if p.agent_id}
        agent_ids = list(agent_to_participant.keys())

        # Build matchup matrix
        matrix: dict[str, dict[str, float]] = {aid: {} for aid in agent_ids}
        for m in matchups:
            if m.agent_a_id and m.agent_b_id:
                matrix[m.agent_a_id][m.agent_b_id] = m.a_win_rate
                # Infer reverse
                b_win_rate = 1.0 - m.a_win_rate if m.draws == 0 else (m.b_wins / max(1, m.games_played))
                matrix[m.agent_b_id][m.agent_a_id] = b_win_rate

        # Compute Expected Tournament Win Rate
        # ETWR(A) = sum(wr(A,X) * meta_share(X)) / sum(meta_share(X)) for all X != A
        rankings = []
        for aid in agent_ids:
            p = agent_to_participant[aid]
            total_weighted_wr = 0.0
            total_meta_share = 0.0
            for other_aid in agent_ids:
                if other_aid == aid:
                    continue
                other_p = agent_to_participant[other_aid]
                wr = matrix.get(aid, {}).get(other_aid, 0.5)
                ms = other_p.meta_share if other_p.meta_share > 0 else 0.01
                total_weighted_wr += wr * ms
                total_meta_share += ms

            etwr = total_weighted_wr / total_meta_share if total_meta_share > 0 else 0.0

            # Also compute simple average matchup WR
            matchup_wrs = [matrix.get(aid, {}).get(oid, 0.5) for oid in agent_ids if oid != aid]
            avg_wr = sum(matchup_wrs) / len(matchup_wrs) if matchup_wrs else 0.0

            p.tournament_win_rate = avg_wr
            p.expected_meta_win_rate = etwr

            rankings.append({
                "agent_id": aid,
                "archetype": p.archetype_name,
                "avg_matchup_wr": round(avg_wr, 4),
                "expected_meta_wr": round(etwr, 4),
                "meta_share": round(p.meta_share, 4),
            })

        rankings.sort(key=lambda x: x["expected_meta_wr"], reverse=True)

        gauntlet.matchup_matrix_json = json.dumps(matrix)
        gauntlet.tournament_rankings_json = json.dumps(rankings)

        logger.info("Finalized gauntlet %s: %d rankings computed", gauntlet.id, len(rankings))


gauntlet_orchestrator = GauntletOrchestrator()
