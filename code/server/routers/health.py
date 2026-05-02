"""Liveness + readiness endpoints.

`/health` (alias `/healthz`) is a cheap liveness probe — if the process is
answering, it's alive. `/readyz` is the readiness probe: returns 503 until
downstream dependencies the app can't serve without (database, model
storage) respond.
"""

from fastapi import APIRouter, Response, status
from sqlalchemy import text
from sqlalchemy.ext.asyncio import AsyncSession
from fastapi import Depends

from server.config import settings
from server.db.database import get_db

router = APIRouter(tags=["health"])


@router.get("/health")
@router.get("/healthz")
@router.get("/", include_in_schema=False)
def health_check():
    return {"status": "ok", "engine_version": settings.engine_version}


@router.get("/readyz")
async def readiness_check(response: Response, db: AsyncSession = Depends(get_db)):
    checks: dict[str, str] = {}

    try:
        await db.execute(text("SELECT 1"))
        checks["database"] = "ok"
    except Exception as exc:
        checks["database"] = f"error: {exc.__class__.__name__}"

    if settings.spaces_endpoint_url and settings.spaces_bucket:
        checks["model_storage"] = "configured"
    elif settings.environment == "development":
        checks["model_storage"] = "local"
    else:
        checks["model_storage"] = "misconfigured"

    ready = all(v == "ok" or v in ("configured", "local") for v in checks.values())
    if not ready:
        response.status_code = status.HTTP_503_SERVICE_UNAVAILABLE
    return {"ready": ready, "checks": checks, "engine_version": settings.engine_version}

