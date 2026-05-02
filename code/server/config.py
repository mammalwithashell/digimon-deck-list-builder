"""Process-wide settings loaded from environment variables."""

from __future__ import annotations

from functools import lru_cache
from typing import List, Literal, Optional

from pydantic import AliasChoices, Field
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
        validation_alias=AliasChoices("ENVIRONMENT", "DIGIMON_ENV"),
    )

    secret_key: str = Field(
        default=PLACEHOLDER_SECRET_KEY,
        validation_alias=AliasChoices("SECRET_KEY", "JWT_SECRET_KEY"),
    )

    cors_origins_raw: str = Field(
        default="http://localhost:5173",
        validation_alias=AliasChoices("CORS_ORIGINS", "FRONTEND_ORIGIN"),
    )

    database_url: Optional[str] = Field(
        default=None,
        validation_alias=AliasChoices("DATABASE_URL"),
    )

    rate_limit_storage_uri: Optional[str] = Field(default=None)
    rate_limits_enabled: bool = Field(default=True)
    invite_codes_required: bool = Field(default=False)
    engine_version: str = Field(default="0.1.0")

    spaces_endpoint_url: Optional[str] = Field(
        default=None,
        validation_alias=AliasChoices("SPACES_ENDPOINT", "SPACES_ENDPOINT_URL"),
    )
    spaces_region: Optional[str] = Field(default=None, validation_alias=AliasChoices("SPACES_REGION"))
    spaces_bucket: Optional[str] = Field(default=None, validation_alias=AliasChoices("SPACES_BUCKET"))
    spaces_access_key: Optional[str] = Field(default=None, validation_alias=AliasChoices("SPACES_KEY", "SPACES_ACCESS_KEY"))
    spaces_secret_key: Optional[str] = Field(default=None, validation_alias=AliasChoices("SPACES_SECRET", "SPACES_SECRET_KEY"))
    spaces_cdn_base_url: Optional[str] = Field(default=None, validation_alias=AliasChoices("SPACES_CDN_URL", "SPACES_CDN_BASE_URL"))

    sentry_dsn: Optional[str] = Field(default=None)
    log_level: str = Field(default="INFO")

    @property
    def cors_origins(self) -> List[str]:
        return [origin.strip() for origin in self.cors_origins_raw.split(",") if origin.strip()]

    def assert_production_ready(self) -> None:
        """Raise if the current settings are unsafe for a public deployment."""
        if self.environment == "development":
            return

        problems: list[str] = []
        if not self.secret_key or self.secret_key == PLACEHOLDER_SECRET_KEY:
            problems.append("SECRET_KEY/JWT_SECRET_KEY is unset or still the placeholder")
        if not self.database_url:
            problems.append("DATABASE_URL is unset")

        for name in (
            "spaces_endpoint_url",
            "spaces_bucket",
            "spaces_region",
            "spaces_access_key",
            "spaces_secret_key",
        ):
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
