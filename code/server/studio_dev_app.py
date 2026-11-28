"""Standalone local dev FastAPI app for Board-State Studio (engine-only, no DB/slowapi).

Run from repo root:
    .\\env\\Scripts\\python.exe -m uvicorn server.studio_dev_app:app --host 127.0.0.1 --port 8000
"""

from __future__ import annotations

from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware

from server.routers import debug_games, deck_tools, games, studio

app = FastAPI(title="Digimon Board-State Studio (Local Dev)")

app.add_middleware(
    CORSMiddleware,
    allow_origins=[
        "http://localhost:5173",
        "http://127.0.0.1:5173",
        "http://localhost:4173",
        "http://127.0.0.1:4173",
    ],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)


@app.get("/health")
def health_check():
    return {"status": "ok", "mode": "studio-dev"}


app.include_router(deck_tools.router)
app.include_router(debug_games.router)
app.include_router(studio.router)
app.include_router(games.router)
