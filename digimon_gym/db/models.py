"""SQLAlchemy ORM models for all persistent entities."""

from __future__ import annotations

import uuid
from datetime import datetime, timezone

from sqlalchemy import (
    CheckConstraint,
    Column,
    DateTime,
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


# ── Refresh Tokens ──────────────────────────────────────────────────────

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
