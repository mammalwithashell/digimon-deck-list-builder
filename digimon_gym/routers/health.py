"""Health endpoints."""

from fastapi import APIRouter

router = APIRouter(tags=["health"])


@router.get("/health")
@router.get("/", include_in_schema=False)
def health_check():
    return {"status": "ok"}

