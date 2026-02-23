"""SQLAlchemy ORM models for all persistent entities."""

from __future__ import annotations

import uuid
from datetime import datetime, timezone

from sqlalchemy import (
    CheckConstraint,
    Column,
    DateTime,
    Float,
    ForeignKey,
    Index,
    Integer,
    String,
    Text,
    UniqueConstraint,
)
from sqlalchemy.orm import DeclarativeBase, relationship


def _utcnow() -> datetime:
    return datetime.now(timezone.utc)


def _new_uuid() -> str:
    return str(uuid.uuid4())


class Base(DeclarativeBase):
    pass


# ── Users ───────────────────────────────────────────────────────────────

class User(Base):
    __tablename__ = "users"

    id = Column(String, primary_key=True, default=_new_uuid)
    username = Column(String, unique=True, nullable=False, index=True)
    email = Column(String, unique=True, nullable=False, index=True)
    password_hash = Column(String, nullable=False)
    display_name = Column(String, nullable=True)
    avatar_url = Column(String, nullable=True)
    created_at = Column(DateTime(timezone=True), default=_utcnow, nullable=False)
    updated_at = Column(DateTime(timezone=True), default=_utcnow, onupdate=_utcnow, nullable=False)
    last_login_at = Column(DateTime(timezone=True), nullable=True)
    is_active = Column(Integer, default=1, nullable=False)

    # Relationships
    decks = relationship("Deck", back_populates="owner", cascade="all, delete-orphan")
    assets = relationship("UserAsset", back_populates="owner", cascade="all, delete-orphan")
    preferences = relationship("UserPreferences", back_populates="user", uselist=False, cascade="all, delete-orphan")
    refresh_tokens = relationship("RefreshToken", back_populates="user", cascade="all, delete-orphan")
    roles = relationship("Role", secondary="user_roles", back_populates="users")


class Role(Base):
    __tablename__ = "roles"
    __table_args__ = (
        Index("idx_roles_name", "name"),
    )

    id = Column(String, primary_key=True, default=_new_uuid)
    name = Column(String, unique=True, nullable=False)
    description = Column(Text, default="", nullable=False)
    created_at = Column(DateTime(timezone=True), default=_utcnow, nullable=False)

    users = relationship("User", secondary="user_roles", back_populates="roles")


class UserRole(Base):
    __tablename__ = "user_roles"
    __table_args__ = (
        UniqueConstraint("user_id", "role_id", name="uq_user_roles_user_role"),
        Index("idx_user_roles_user_id", "user_id"),
        Index("idx_user_roles_role_id", "role_id"),
    )

    id = Column(String, primary_key=True, default=_new_uuid)
    user_id = Column(String, ForeignKey("users.id", ondelete="CASCADE"), nullable=False)
    role_id = Column(String, ForeignKey("roles.id", ondelete="CASCADE"), nullable=False)
    created_at = Column(DateTime(timezone=True), default=_utcnow, nullable=False)


# ── Decks ───────────────────────────────────────────────────────────────

class Deck(Base):
    __tablename__ = "decks"
    __table_args__ = (
        CheckConstraint(
            "game_mode IN ('standard', 'edh_commander', 'titan', 'no_restriction')",
            name="ck_decks_game_mode",
        ),
        CheckConstraint(
            "(game_mode = 'edh_commander' AND commander_id IS NOT NULL) "
            "OR (game_mode != 'edh_commander' AND commander_id IS NULL)",
            name="ck_decks_commander",
        ),
        CheckConstraint(
            "(game_mode = 'titan' AND titan_role IS NOT NULL) "
            "OR (game_mode != 'titan' AND titan_role IS NULL)",
            name="ck_decks_titan_role",
        ),
        CheckConstraint(
            "titan_role IN ('titan', 'team') OR titan_role IS NULL",
            name="ck_decks_titan_role_values",
        ),
        Index("idx_decks_owner_id", "owner_id"),
        Index("idx_decks_game_mode", "game_mode"),
    )

    id = Column(String, primary_key=True, default=_new_uuid)
    owner_id = Column(String, ForeignKey("users.id", ondelete="CASCADE"), nullable=False)
    name = Column(String, nullable=False)
    description = Column(Text, default="")
    game_mode = Column(String, nullable=False)
    titan_role = Column(String, nullable=True)
    main_deck = Column(Text, nullable=False)  # JSON array of card ID strings
    egg_deck = Column(Text, default="[]")  # JSON array of card ID strings
    commander_id = Column(String, nullable=True)
    is_valid = Column(Integer, default=0, nullable=False)
    validation_errors = Column(Text, default="[]")  # JSON array of error strings
    is_public = Column(Integer, default=0, nullable=False)
    tags = Column(Text, default="[]")  # JSON array of tag strings
    created_at = Column(DateTime(timezone=True), default=_utcnow, nullable=False)
    updated_at = Column(DateTime(timezone=True), default=_utcnow, onupdate=_utcnow, nullable=False)

    owner = relationship("User", back_populates="decks")
    versions = relationship("DeckVersion", back_populates="deck", cascade="all, delete-orphan")


# ── Deck Versions ───────────────────────────────────────────────────────

class DeckVersion(Base):
    __tablename__ = "deck_versions"
    __table_args__ = (
        UniqueConstraint("deck_id", "version_number", name="uq_deck_version"),
        Index("idx_deck_versions_deck_id", "deck_id"),
    )

    id = Column(String, primary_key=True, default=_new_uuid)
    deck_id = Column(String, ForeignKey("decks.id", ondelete="CASCADE"), nullable=False)
    version_number = Column(Integer, nullable=False)
    main_deck = Column(Text, nullable=False)
    egg_deck = Column(Text, default="[]")
    commander_id = Column(String, nullable=True)
    change_note = Column(Text, default="")
    created_at = Column(DateTime(timezone=True), default=_utcnow, nullable=False)

    deck = relationship("Deck", back_populates="versions")


# ── Friendships ─────────────────────────────────────────────────────────

class Friendship(Base):
    __tablename__ = "friendships"
    __table_args__ = (
        CheckConstraint("user_id != friend_id", name="ck_friendships_no_self"),
        CheckConstraint(
            "status IN ('pending', 'accepted', 'blocked')",
            name="ck_friendships_status",
        ),
        Index("idx_friendships_friend_id", "friend_id"),
        Index("idx_friendships_status", "status"),
    )

    user_id = Column(String, ForeignKey("users.id", ondelete="CASCADE"), primary_key=True)
    friend_id = Column(String, ForeignKey("users.id", ondelete="CASCADE"), primary_key=True)
    status = Column(String, nullable=False)
    created_at = Column(DateTime(timezone=True), default=_utcnow, nullable=False)
    updated_at = Column(DateTime(timezone=True), default=_utcnow, onupdate=_utcnow, nullable=False)


# ── User Assets ─────────────────────────────────────────────────────────

class UserAsset(Base):
    __tablename__ = "user_assets"
    __table_args__ = (
        CheckConstraint(
            "asset_type IN ('card_back', 'board_skin', 'avatar')",
            name="ck_user_assets_type",
        ),
        Index("idx_user_assets_owner_id", "owner_id"),
    )

    id = Column(String, primary_key=True, default=_new_uuid)
    owner_id = Column(String, ForeignKey("users.id", ondelete="CASCADE"), nullable=False)
    asset_type = Column(String, nullable=False)
    name = Column(String, nullable=False)
    file_url = Column(String, nullable=False)
    thumbnail_url = Column(String, nullable=True)
    file_size_bytes = Column(Integer, nullable=True)
    mime_type = Column(String, nullable=True)
    created_at = Column(DateTime(timezone=True), default=_utcnow, nullable=False)

    owner = relationship("User", back_populates="assets")


# ── User Preferences ───────────────────────────────────────────────────

class UserPreferences(Base):
    __tablename__ = "user_preferences"

    user_id = Column(String, ForeignKey("users.id", ondelete="CASCADE"), primary_key=True)
    active_card_back_id = Column(
        String, ForeignKey("user_assets.id", ondelete="SET NULL"), nullable=True
    )
    active_board_skin_id = Column(
        String, ForeignKey("user_assets.id", ondelete="SET NULL"), nullable=True
    )
    updated_at = Column(DateTime(timezone=True), default=_utcnow, onupdate=_utcnow, nullable=False)

    user = relationship("User", back_populates="preferences")
    active_card_back = relationship("UserAsset", foreign_keys=[active_card_back_id])
    active_board_skin = relationship("UserAsset", foreign_keys=[active_board_skin_id])


# ── Game Sessions ───────────────────────────────────────────────────────

class GameSession(Base):
    __tablename__ = "game_sessions"
    __table_args__ = (
        CheckConstraint(
            "game_mode IN ('standard', 'edh_commander', 'titan', 'no_restriction')",
            name="ck_game_sessions_mode",
        ),
        CheckConstraint(
            "result_type IN ('completed', 'concession', 'timeout', 'deck_out', 'abandoned') "
            "OR result_type IS NULL",
            name="ck_game_sessions_result_type",
        ),
    )

    id = Column(String, primary_key=True, default=_new_uuid)
    game_mode = Column(String, nullable=False)
    started_at = Column(DateTime(timezone=True), default=_utcnow, nullable=False)
    ended_at = Column(DateTime(timezone=True), nullable=True)
    total_turns = Column(Integer, nullable=True)
    winner_id = Column(String, ForeignKey("users.id", ondelete="SET NULL"), nullable=True)
    result_type = Column(String, nullable=True)

    participants = relationship("GameParticipant", back_populates="game_session", cascade="all, delete-orphan")


# ── Game Participants ───────────────────────────────────────────────────

class GameParticipant(Base):
    __tablename__ = "game_participants"
    __table_args__ = (
        CheckConstraint(
            "player_type IN ('human', 'agent')",
            name="ck_game_participants_type",
        ),
        CheckConstraint(
            "result IN ('win', 'loss', 'draw', 'eliminated') OR result IS NULL",
            name="ck_game_participants_result",
        ),
        CheckConstraint(
            "titan_role IN ('titan', 'team') OR titan_role IS NULL",
            name="ck_game_participants_titan_role",
        ),
        Index("idx_game_participants_user", "user_id"),
    )

    game_id = Column(String, ForeignKey("game_sessions.id", ondelete="CASCADE"), primary_key=True)
    player_slot = Column(Integer, primary_key=True)
    user_id = Column(String, ForeignKey("users.id", ondelete="SET NULL"), nullable=True)
    player_type = Column(String, nullable=False)
    deck_id = Column(String, ForeignKey("decks.id", ondelete="SET NULL"), nullable=True)
    deck_snapshot = Column(Text, nullable=True)  # JSON snapshot of card list at game start
    titan_role = Column(String, nullable=True)
    result = Column(String, nullable=True)
    elimination_order = Column(Integer, nullable=True)

    game_session = relationship("GameSession", back_populates="participants")


# ── Game Recordings ────────────────────────────────────────────────

class GameRecording(Base):
    """Server-side recording of a headless (agent-vs-agent) game."""
    __tablename__ = "game_recordings"
    __table_args__ = (
        Index("idx_game_recordings_created", "created_at"),
    )

    id = Column(String, primary_key=True, default=_new_uuid)
    game_session_id = Column(
        String, ForeignKey("game_sessions.id", ondelete="SET NULL"), nullable=True
    )
    game_mode = Column(String, nullable=False, default="headless")
    recording_json = Column(Text, nullable=False)  # Full JSON: initial_state + actions + tensors
    total_steps = Column(Integer, nullable=False, default=0)
    has_tensors = Column(Integer, default=0, nullable=False)  # 1 if tensor snapshots included
    created_at = Column(DateTime(timezone=True), default=_utcnow, nullable=False)

    game_session = relationship("GameSession", backref="recording")


class GameRecordingReport(Base):
    """Client-submitted bug report with attached game recording from interactive play."""
    __tablename__ = "game_recording_reports"
    __table_args__ = (
        Index("idx_recording_reports_created", "created_at"),
        Index("idx_recording_reports_user", "user_id"),
    )

    id = Column(String, primary_key=True, default=_new_uuid)
    user_id = Column(
        String, ForeignKey("users.id", ondelete="SET NULL"), nullable=True
    )
    description = Column(Text, default="")
    recording_json = Column(Text, nullable=False)  # Full client recording bundle
    created_at = Column(DateTime(timezone=True), default=_utcnow, nullable=False)

    user = relationship("User")


# ── Refresh Tokens ──────────────────────────────────────────────────────

class Issue(Base):
    __tablename__ = "issues"
    __table_args__ = (
        CheckConstraint(
            "source IN ('player', 'judge', 'system')",
            name="ck_issues_source",
        ),
        CheckConstraint(
            "severity IN ('low', 'medium', 'high', 'critical')",
            name="ck_issues_severity",
        ),
        CheckConstraint(
            "status IN ('new', 'approved_for_ai', 'rejected', 'resolved')",
            name="ck_issues_status",
        ),
        Index("idx_issues_status", "status"),
        Index("idx_issues_card_id", "card_id"),
        Index("idx_issues_created_by", "created_by"),
    )

    id = Column(String, primary_key=True, default=_new_uuid)
    card_id = Column(String, nullable=False)
    description = Column(Text, nullable=False)
    source = Column(String, nullable=False)
    severity = Column(String, nullable=False, default="medium")
    status = Column(String, nullable=False, default="new")
    created_by = Column(String, ForeignKey("users.id", ondelete="SET NULL"), nullable=True)
    approved_by = Column(String, ForeignKey("users.id", ondelete="SET NULL"), nullable=True)
    triage_notes = Column(Text, default="", nullable=False)
    resolution_notes = Column(Text, default="", nullable=False)
    created_at = Column(DateTime(timezone=True), default=_utcnow, nullable=False)
    updated_at = Column(DateTime(timezone=True), default=_utcnow, onupdate=_utcnow, nullable=False)
    resolved_at = Column(DateTime(timezone=True), nullable=True)

    creator = relationship("User", foreign_keys=[created_by])
    approver = relationship("User", foreign_keys=[approved_by])


class AITask(Base):
    __tablename__ = "ai_tasks"
    __table_args__ = (
        CheckConstraint(
            "task_type IN ('review_batch', 'qa_analysis', 'engine_audit', 'script_autofix')",
            name="ck_ai_tasks_task_type",
        ),
        CheckConstraint(
            "status IN ('queued', 'running', 'completed', 'failed')",
            name="ck_ai_tasks_status",
        ),
        CheckConstraint(
            "run_mode IN ('pr', 'main') OR run_mode IS NULL",
            name="ck_ai_tasks_run_mode",
        ),
        CheckConstraint(
            "scope_profile IN ('script', 'script_engine', 'script_engine_transpiler') "
            "OR scope_profile IS NULL",
            name="ck_ai_tasks_scope_profile",
        ),
        Index("idx_ai_tasks_status", "status"),
        Index("idx_ai_tasks_task_type", "task_type"),
        Index("idx_ai_tasks_batch_id", "batch_id"),
        Index("idx_ai_tasks_created_at", "created_at"),
    )

    id = Column(String, primary_key=True, default=_new_uuid)
    task_type = Column(String, nullable=False)
    payload_json = Column(Text, nullable=False, default="{}")
    status = Column(String, nullable=False, default="queued")
    result_json = Column(Text, nullable=True)
    sanitized_input_json = Column(Text, nullable=False, default="{}")
    retrieval_refs_json = Column(Text, nullable=False, default="[]")
    model_name = Column(String, nullable=True)
    cost_estimate_usd = Column(Float, nullable=False, default=0.0)
    cost_actual_usd = Column(Float, nullable=False, default=0.0)
    input_tokens = Column(Integer, nullable=False, default=0)
    output_tokens = Column(Integer, nullable=False, default=0)
    error_text = Column(Text, nullable=True)
    attempts = Column(Integer, nullable=False, default=0)
    max_attempts = Column(Integer, nullable=False, default=3)
    created_by = Column(String, ForeignKey("users.id", ondelete="SET NULL"), nullable=True)
    batch_id = Column(String, ForeignKey("ai_fix_batches.id", ondelete="SET NULL"), nullable=True)
    run_mode = Column(String, nullable=True)
    scope_profile = Column(String, nullable=True)
    started_at = Column(DateTime(timezone=True), nullable=True)
    completed_at = Column(DateTime(timezone=True), nullable=True)
    created_at = Column(DateTime(timezone=True), default=_utcnow, nullable=False)
    updated_at = Column(DateTime(timezone=True), default=_utcnow, onupdate=_utcnow, nullable=False)

    creator = relationship("User")
    batch = relationship("AIFixBatch", back_populates="tasks")


class AIFixBatch(Base):
    __tablename__ = "ai_fix_batches"
    __table_args__ = (
        CheckConstraint(
            "run_mode IN ('pr', 'main')",
            name="ck_ai_fix_batches_run_mode",
        ),
        CheckConstraint(
            "scope_profile IN ('script', 'script_engine', 'script_engine_transpiler')",
            name="ck_ai_fix_batches_scope_profile",
        ),
        CheckConstraint(
            "status IN ('running', 'completed', 'failed', 'failed_no_changes', 'canceled')",
            name="ck_ai_fix_batches_status",
        ),
        Index("idx_ai_fix_batches_set_id", "set_id"),
        Index("idx_ai_fix_batches_status", "status"),
        Index("idx_ai_fix_batches_created_at", "created_at"),
    )

    id = Column(String, primary_key=True, default=_new_uuid)
    set_id = Column(String, nullable=False)
    run_mode = Column(String, nullable=False)
    scope_profile = Column(String, nullable=False)
    status = Column(String, nullable=False, default="running")
    created_by = Column(String, ForeignKey("users.id", ondelete="SET NULL"), nullable=True)
    model_name = Column(String, nullable=True)
    concurrency = Column(Integer, nullable=False, default=4)
    max_total_cost_usd = Column(Float, nullable=False, default=5.0)
    failure_rate_stop = Column(Float, nullable=False, default=0.3)
    max_tasks = Column(Integer, nullable=False, default=0)
    queued_count = Column(Integer, nullable=False, default=0)
    running_count = Column(Integer, nullable=False, default=0)
    completed_count = Column(Integer, nullable=False, default=0)
    failed_count = Column(Integer, nullable=False, default=0)
    applied_count = Column(Integer, nullable=False, default=0)
    commit_count = Column(Integer, nullable=False, default=0)
    stopped_reason = Column(Text, nullable=True)
    pr_url = Column(String, nullable=True)
    created_at = Column(DateTime(timezone=True), default=_utcnow, nullable=False)
    updated_at = Column(DateTime(timezone=True), default=_utcnow, onupdate=_utcnow, nullable=False)

    creator = relationship("User")
    items = relationship("AIFixBatchItem", back_populates="batch", cascade="all, delete-orphan")
    tasks = relationship("AITask", back_populates="batch")


class AIFixBatchItem(Base):
    __tablename__ = "ai_fix_batch_items"
    __table_args__ = (
        CheckConstraint(
            "status IN ('pending', 'queued', 'running', 'completed', 'failed', 'applied', 'canceled')",
            name="ck_ai_fix_batch_items_status",
        ),
        Index("idx_ai_fix_batch_items_batch_id", "batch_id"),
        Index("idx_ai_fix_batch_items_card_id", "card_id"),
        Index("idx_ai_fix_batch_items_status", "status"),
    )

    id = Column(String, primary_key=True, default=_new_uuid)
    batch_id = Column(String, ForeignKey("ai_fix_batches.id", ondelete="CASCADE"), nullable=False)
    issue_id = Column(String, ForeignKey("issues.id", ondelete="SET NULL"), nullable=True)
    card_id = Column(String, nullable=False)
    task_id = Column(String, ForeignKey("ai_tasks.id", ondelete="SET NULL"), nullable=True)
    status = Column(String, nullable=False, default="pending")
    error_text = Column(Text, nullable=True)
    applied_at = Column(DateTime(timezone=True), nullable=True)
    commit_sha = Column(String, nullable=True)
    created_at = Column(DateTime(timezone=True), default=_utcnow, nullable=False)
    updated_at = Column(DateTime(timezone=True), default=_utcnow, onupdate=_utcnow, nullable=False)

    batch = relationship("AIFixBatch", back_populates="items")
    issue = relationship("Issue")
    task = relationship("AITask")


class EngineBacklogItem(Base):
    __tablename__ = "engine_backlog_items"
    __table_args__ = (
        CheckConstraint(
            "status IN ('open', 'in_progress', 'closed')",
            name="ck_engine_backlog_status",
        ),
        Index("idx_engine_backlog_status", "status"),
        Index("idx_engine_backlog_source_task", "source_task_id"),
    )

    id = Column(String, primary_key=True, default=_new_uuid)
    title = Column(String, nullable=False)
    description = Column(Text, default="", nullable=False)
    mechanic = Column(String, default="", nullable=False)
    status = Column(String, nullable=False, default="open")
    source_task_id = Column(String, ForeignKey("ai_tasks.id", ondelete="SET NULL"), nullable=True)
    created_by = Column(String, ForeignKey("users.id", ondelete="SET NULL"), nullable=True)
    created_at = Column(DateTime(timezone=True), default=_utcnow, nullable=False)
    updated_at = Column(DateTime(timezone=True), default=_utcnow, onupdate=_utcnow, nullable=False)

    source_task = relationship("AITask")
    creator = relationship("User")


class ScriptPromotionAudit(Base):
    __tablename__ = "script_promotion_audits"
    __table_args__ = (
        Index("idx_script_promotions_card_id", "card_id"),
        Index("idx_script_promotions_created_at", "created_at"),
    )

    id = Column(String, primary_key=True, default=_new_uuid)
    card_id = Column(String, nullable=False)
    set_id = Column(String, nullable=False)
    module_name = Column(String, nullable=False)
    generated_hash = Column(String, nullable=False)
    frozen_hash = Column(String, nullable=False)
    manifest_version = Column(Integer, nullable=False, default=1)
    ai_task_id = Column(String, ForeignKey("ai_tasks.id", ondelete="SET NULL"), nullable=True)
    promoted_by = Column(String, ForeignKey("users.id", ondelete="SET NULL"), nullable=True)
    notes = Column(Text, default="", nullable=False)
    created_at = Column(DateTime(timezone=True), default=_utcnow, nullable=False)

    task = relationship("AITask")
    promoter = relationship("User")


class AIFixApplyAudit(Base):
    __tablename__ = "ai_fix_apply_audits"
    __table_args__ = (
        Index("idx_ai_fix_apply_audits_task_id", "ai_task_id"),
        Index("idx_ai_fix_apply_audits_batch_id", "batch_id"),
        Index("idx_ai_fix_apply_audits_card_id", "card_id"),
        Index("idx_ai_fix_apply_audits_created_at", "created_at"),
    )

    id = Column(String, primary_key=True, default=_new_uuid)
    ai_task_id = Column(String, ForeignKey("ai_tasks.id", ondelete="SET NULL"), nullable=True)
    batch_id = Column(String, ForeignKey("ai_fix_batches.id", ondelete="SET NULL"), nullable=True)
    card_id = Column(String, nullable=False)
    scope_profile = Column(String, nullable=False)
    run_mode = Column(String, nullable=False)
    applied_files_json = Column(Text, nullable=False, default="[]")
    check_outputs_json = Column(Text, nullable=False, default="[]")
    commit_sha = Column(String, nullable=True)
    status = Column(String, nullable=False, default="applied")
    error_text = Column(Text, nullable=True)
    created_by = Column(String, ForeignKey("users.id", ondelete="SET NULL"), nullable=True)
    created_at = Column(DateTime(timezone=True), default=_utcnow, nullable=False)

    task = relationship("AITask")
    batch = relationship("AIFixBatch")
    creator = relationship("User")


class RefreshToken(Base):
    __tablename__ = "refresh_tokens"
    __table_args__ = (
        Index("idx_refresh_tokens_user", "user_id"),
        Index("idx_refresh_tokens_hash", "token_hash"),
    )

    id = Column(String, primary_key=True, default=_new_uuid)
    user_id = Column(String, ForeignKey("users.id", ondelete="CASCADE"), nullable=False)
    token_hash = Column(String, unique=True, nullable=False)
    expires_at = Column(DateTime(timezone=True), nullable=False)
    created_at = Column(DateTime(timezone=True), default=_utcnow, nullable=False)
    revoked = Column(Integer, default=0, nullable=False)

    user = relationship("User", back_populates="refresh_tokens")
