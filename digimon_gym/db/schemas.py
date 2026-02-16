"""Pydantic request/response schemas for all DB-backed API endpoints."""

from __future__ import annotations

from datetime import datetime
from typing import List, Optional

from pydantic import BaseModel, EmailStr, Field


# ── Auth ────────────────────────────────────────────────────────────────

class RegisterRequest(BaseModel):
    username: str = Field(..., min_length=3, max_length=32)
    email: EmailStr
    password: str = Field(..., min_length=8, max_length=128)
    display_name: Optional[str] = None


class LoginRequest(BaseModel):
    username: str
    password: str


class TokenResponse(BaseModel):
    access_token: str
    token_type: str = "bearer"
    refresh_token: str


class RefreshRequest(BaseModel):
    refresh_token: str


# ── Users ───────────────────────────────────────────────────────────────

class UserPublic(BaseModel):
    id: str
    username: str
    display_name: Optional[str] = None
    avatar_url: Optional[str] = None

    model_config = {"from_attributes": True}


class UserProfile(UserPublic):
    email: str
    created_at: datetime
    last_login_at: Optional[datetime] = None

    model_config = {"from_attributes": True}


class UpdateProfileRequest(BaseModel):
    display_name: Optional[str] = None
    avatar_url: Optional[str] = None


# ── Decks ───────────────────────────────────────────────────────────────

class CreateDeckRequest(BaseModel):
    name: str = Field(..., min_length=1, max_length=100)
    description: str = ""
    game_mode: str = Field(..., pattern=r"^(standard|edh_commander|titan|no_restriction)$")
    titan_role: Optional[str] = Field(None, pattern=r"^(titan|team)$")
    main_deck: List[str]  # Card ID strings
    egg_deck: List[str] = []
    commander_id: Optional[str] = None
    is_public: bool = False
    tags: List[str] = []


class UpdateDeckRequest(BaseModel):
    name: Optional[str] = Field(None, min_length=1, max_length=100)
    description: Optional[str] = None
    main_deck: Optional[List[str]] = None
    egg_deck: Optional[List[str]] = None
    commander_id: Optional[str] = None
    is_public: Optional[bool] = None
    tags: Optional[List[str]] = None
    change_note: str = ""  # For version history


class DeckResponse(BaseModel):
    id: str
    owner_id: str
    name: str
    description: str
    game_mode: str
    titan_role: Optional[str] = None
    main_deck: List[str]
    egg_deck: List[str]
    commander_id: Optional[str] = None
    is_valid: bool
    validation_errors: List[str]
    is_public: bool
    tags: List[str]
    created_at: datetime
    updated_at: datetime

    model_config = {"from_attributes": True}


class DeckSummary(BaseModel):
    id: str
    name: str
    game_mode: str
    is_valid: bool
    is_public: bool
    card_count: int
    created_at: datetime
    updated_at: datetime

    model_config = {"from_attributes": True}


# ── Friends ─────────────────────────────────────────────────────────────

class FriendshipResponse(BaseModel):
    user: UserPublic
    status: str
    created_at: datetime

    model_config = {"from_attributes": True}


class FriendRequestResponse(BaseModel):
    from_user: UserPublic
    created_at: datetime

    model_config = {"from_attributes": True}


# ── Assets ──────────────────────────────────────────────────────────────

class AssetResponse(BaseModel):
    id: str
    asset_type: str
    name: str
    file_url: str
    thumbnail_url: Optional[str] = None
    file_size_bytes: Optional[int] = None
    created_at: datetime

    model_config = {"from_attributes": True}


class PreferencesResponse(BaseModel):
    active_card_back_id: Optional[str] = None
    active_board_skin_id: Optional[str] = None

    model_config = {"from_attributes": True}


class UpdatePreferencesRequest(BaseModel):
    active_card_back_id: Optional[str] = None
    active_board_skin_id: Optional[str] = None


# ── Game History ────────────────────────────────────────────────────────

class GameSessionResponse(BaseModel):
    id: str
    game_mode: str
    started_at: datetime
    ended_at: Optional[datetime] = None
    total_turns: Optional[int] = None
    winner_id: Optional[str] = None
    result_type: Optional[str] = None

    model_config = {"from_attributes": True}


class GameParticipantResponse(BaseModel):
    player_slot: int
    user_id: Optional[str] = None
    player_type: str
    deck_id: Optional[str] = None
    titan_role: Optional[str] = None
    result: Optional[str] = None

    model_config = {"from_attributes": True}
