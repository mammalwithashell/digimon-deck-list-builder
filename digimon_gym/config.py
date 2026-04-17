"""Process-wide settings loaded from environment variables.

Importing `settings` is the only sanctioned way to read configuration from
environment variables. Hardcoded defaults in code (e.g. the former
`SECRET_KEY = "CHANGE-ME-IN-PRODUCTION"` in `db/auth.py`) are replaced by
fields here so production deployments fail loudly on misconfiguration.
"""

from __future__ import annotations

from functools import lru_cache
from typing import List, Literal, Optional

from pydantic import Field
from pydantic_settings import BaseSettings, SettingsConfigDict


PLACEHOLDER_SECRET_KEY = "CHANGE-ME-IN-PRODUCTION"


class Settings(BaseSettings):
    model_config = SettingsConfigDict(
        env_file=".env",
        env_file_encoding="utf-8",
        case_sensitive=False,
        extra="ignore",
    )

    environment: Literal["development", "staging", "production"] = Field(
        default="development",
        description="Deployment environment. Non-development rejects placeholder secrets.",
    )

    secret_key: str = Field(
        default=PLACEHOLDER_SECRET_KEY,
        description="HMAC key for JWT signing. MUST be overridden in staging/production.",
    )

    cors_origins_raw: str = Field(
        default="http://localhost:5173",
        alias="CORS_ORIGINS",
        description="Allowed CORS origins, comma-separated. Access via `cors_origins`.",
    )

    @property
    def cors_origins(self) -> List[str]:
        return [o.strip() for o in self.cors_origins_raw.split(",") if o.strip()]

    database_url: Optional[str] = Field(
        default=None,
        description="SQLAlchemy async URL. If unset, digimon_gym.db.database supplies a dev default.",
    )

    rate_limit_storage_uri: Optional[str] = Field(
        default=None,
        description="slowapi storage URI (e.g. redis://...). Unset falls back to in-memory.",
    )
    rate_limits_enabled: bool = Field(
        default=True,
        description="Kill switch for rate limiting (useful in tests). Production should always be true.",
    )

    invite_codes_required: bool = Field(
        default=False,
        description="When true, /auth/register requires a valid unused invite code.",
    )

    engine_version: str = Field(
        default="0.1.0",
        description="Semver engine version broadcast to WS clients; mismatch closes connection.",
    )

    # ── Model distribution (Part 2) ─────────────────────────────────────

    model_storage_backend: Literal["local", "spaces"] = Field(
        default="local",
        description="Backend for serving ONNX model blobs.",
    )
    model_storage_local_root: str = Field(
        default="data/models_store",
        description="Filesystem root for the `local` model storage backend.",
    )
    spaces_endpoint_url: Optional[str] = Field(
        default=None,
        description="DigitalOcean Spaces region endpoint, e.g. https://nyc3.digitaloceanspaces.com.",
    )
    spaces_region: Optional[str] = Field(default=None)
    spaces_bucket: Optional[str] = Field(default=None)
    spaces_access_key: Optional[str] = Field(default=None)
    spaces_secret_key: Optional[str] = Field(default=None)
    spaces_cdn_base_url: Optional[str] = Field(
        default=None,
        description="CDN base URL, e.g. https://bucket.nyc3.cdn.digitaloceanspaces.com.",
    )

    # ── Observability ───────────────────────────────────────────────────

    sentry_dsn: Optional[str] = Field(default=None)
    log_level: str = Field(default="INFO")

    def assert_production_ready(self) -> None:
        """Raise if the current settings would be unsafe to expose publicly.

        Called from the API lifespan. Dev environments are exempt so local
        iteration does not require setting every var.
        """
        if self.environment == "development":
            return
        problems: list[str] = []
        if self.secret_key == PLACEHOLDER_SECRET_KEY or not self.secret_key:
            problems.append("SECRET_KEY is unset or still the placeholder")
        if not self.database_url:
            problems.append("DATABASE_URL is unset")
        if self.model_storage_backend == "spaces":
            for name in ("spaces_endpoint_url", "spaces_bucket", "spaces_access_key", "spaces_secret_key"):
                if not getattr(self, name):
                    problems.append(f"{name.upper()} is unset")
        if problems:
            joined = "\n  - ".join(problems)
            raise RuntimeError(
                f"Unsafe configuration for environment={self.environment!r}:\n  - {joined}"
            )


@lru_cache(maxsize=1)
def get_settings() -> Settings:
    return Settings()


settings = get_settings()
