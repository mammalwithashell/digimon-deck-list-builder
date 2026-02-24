from contextlib import asynccontextmanager
import os

from digimon_gym.env import load_project_env

# Load .env before importing modules that read environment variables at import/init time.
load_project_env()

from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware

from digimon_gym.agents.training_worker import training_job_worker
from digimon_gym.ai.worker import ai_task_worker
from digimon_gym.db.database import init_db
from digimon_gym.db.routers import admin_ai as admin_ai_router
from digimon_gym.db.routers import training as training_router
from digimon_gym.db.routers import assets as assets_router
from digimon_gym.db.routers import auth as auth_router
from digimon_gym.db.routers import decks as decks_router
from digimon_gym.db.routers import friends as friends_router
from digimon_gym.db.routers import issues as issues_router
from digimon_gym.db.routers import users as users_router
from digimon_gym.routers import deck_tools
from digimon_gym.routers import games
from digimon_gym.routers import health
from digimon_gym.routers import recordings
from digimon_gym.routers import replays
from digimon_gym.routers import simulations


@asynccontextmanager
async def lifespan(app: FastAPI):
    await init_db()
    worker_enabled = os.getenv("AI_WORKER_DISABLED", "0") != "1"
    if worker_enabled:
        await ai_task_worker.start()
    training_enabled = os.getenv("TRAINING_WORKER_DISABLED", "0") != "1"
    if training_enabled:
        await training_job_worker.start()
    yield
    if training_enabled:
        await training_job_worker.stop()
    if worker_enabled:
        await ai_task_worker.stop()


app = FastAPI(lifespan=lifespan)

app.add_middleware(
    CORSMiddleware,
    allow_origins=["http://localhost:5173"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

# DB-backed routers
app.include_router(auth_router.router)
app.include_router(users_router.router)
app.include_router(decks_router.router)
app.include_router(friends_router.router)
app.include_router(assets_router.router)
app.include_router(issues_router.router)
app.include_router(admin_ai_router.router)
app.include_router(training_router.router)

# Domain routers (REST-first, with legacy aliases inside each module)
app.include_router(health.router)
app.include_router(simulations.router)
app.include_router(games.router)
app.include_router(recordings.router)
app.include_router(replays.router)
app.include_router(deck_tools.router)