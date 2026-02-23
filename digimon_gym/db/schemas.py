"""Pydantic request/response schemas for all DB-backed API endpoints."""

from __future__ import annotations

from datetime import datetime
from typing import Any, Dict, List, Literal, Optional

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
    roles: List[str] = Field(default_factory=list)

    model_config = {"from_attributes": True}


class UserProfile(UserPublic):
    email: str
    created_at: datetime
    last_login_at: Optional[datetime] = None

    model_config = {"from_attributes": True}


class UpdateProfileRequest(BaseModel):
    display_name: Optional[str] = None
    avatar_url: Optional[str] = None


# -- AI Issue/Triage -----------------------------------------------------------

class IssueCreateRequest(BaseModel):
    card_id: str = Field(..., min_length=1, max_length=64)
    description: str = Field(..., min_length=1, max_length=5000)
    source: str = Field(..., pattern=r"^(player|judge|system)$")
    severity: str = Field("medium", pattern=r"^(low|medium|high|critical)$")


class IssueUpdateRequest(BaseModel):
    status: Optional[str] = Field(None, pattern=r"^(new|approved_for_ai|rejected|resolved)$")
    severity: Optional[str] = Field(None, pattern=r"^(low|medium|high|critical)$")
    triage_notes: Optional[str] = None
    resolution_notes: Optional[str] = None


class IssueResponse(BaseModel):
    id: str
    card_id: str
    description: str
    source: str
    severity: str
    status: str
    created_by: Optional[str] = None
    approved_by: Optional[str] = None
    triage_notes: str
    resolution_notes: str
    created_at: datetime
    updated_at: datetime
    resolved_at: Optional[datetime] = None

    model_config = {"from_attributes": True}


class AITaskCreateRequest(BaseModel):
    task_type: str = Field(..., pattern=r"^(review_batch|qa_analysis|engine_audit|script_autofix)$")
    payload: Dict[str, Any] = Field(default_factory=dict)
    model_name: Optional[str] = None
    cost_estimate: float = Field(0.0, ge=0.0)
    max_attempts: int = Field(3, ge=1, le=10)
    run_mode: Optional[Literal["pr", "main"]] = None
    scope_profile: Optional[Literal["script", "script_engine", "script_engine_transpiler"]] = None
    batch_id: Optional[str] = None


class AITaskResponse(BaseModel):
    id: str
    task_type: str
    payload: Dict[str, Any]
    status: str
    result: Optional[Dict[str, Any]] = None
    sanitized_input: Dict[str, Any] = Field(default_factory=dict)
    retrieval_refs: List[Dict[str, Any]] = Field(default_factory=list)
    model_name: Optional[str] = None
    cost_estimate: float
    cost_actual: float
    input_tokens: int
    output_tokens: int
    error_text: Optional[str] = None
    attempts: int
    max_attempts: int
    created_by: Optional[str] = None
    batch_id: Optional[str] = None
    run_mode: Optional[str] = None
    scope_profile: Optional[str] = None
    started_at: Optional[datetime] = None
    completed_at: Optional[datetime] = None
    created_at: datetime
    updated_at: datetime


class AITaskRetryResponse(BaseModel):
    task_id: str
    status: str


class AITaskApplyFixResponse(BaseModel):
    task_id: str
    status: str
    applied_files: List[str] = Field(default_factory=list)
    commit_sha: Optional[str] = None


class TaskPromotionRequest(BaseModel):
    card_id: str = Field(..., min_length=1, max_length=64)
    notes: str = ""


class PromotionRequest(BaseModel):
    card_id: str = Field(..., min_length=1, max_length=64)
    set_id: str = Field(..., min_length=1, max_length=32)
    module_name: str = Field(..., min_length=1, max_length=128)
    expected_generated_hash: str = Field(..., min_length=10, max_length=256)
    notes: str = ""
    ai_task_id: Optional[str] = None


class PromotionResponse(BaseModel):
    id: str
    card_id: str
    set_id: str
    module_name: str
    generated_hash: str
    frozen_hash: str
    manifest_version: int
    promoted_by: Optional[str] = None
    ai_task_id: Optional[str] = None
    batch_id: Optional[str] = None
    notes: str
    created_at: datetime


class AIFixBatchCreateRequest(BaseModel):
    set_id: str = Field(..., min_length=1, max_length=32)
    run_mode: Literal["pr", "main"] = "pr"
    scope_profile: Literal["script", "script_engine", "script_engine_transpiler"] = "script"
    model_name: Optional[str] = None
    concurrency: int = Field(4, ge=1, le=16)
    max_total_cost_usd: float = Field(5.0, ge=0.0)
    failure_rate_stop: float = Field(0.3, ge=0.0, le=1.0)
    max_tasks: int = Field(0, ge=0, le=5000)
    dry_run: bool = False


class AIFixBatchItemResponse(BaseModel):
    id: str
    batch_id: str
    issue_id: Optional[str] = None
    card_id: str
    task_id: Optional[str] = None
    status: str
    error_text: Optional[str] = None
    applied_at: Optional[datetime] = None
    commit_sha: Optional[str] = None
    created_at: datetime
    updated_at: datetime


class AIFixBatchResponse(BaseModel):
    id: str
    set_id: str
    run_mode: str
    scope_profile: str
    status: str
    created_by: Optional[str] = None
    model_name: Optional[str] = None
    concurrency: int
    max_total_cost_usd: float
    failure_rate_stop: float
    max_tasks: int
    queued_count: int
    running_count: int
    completed_count: int
    failed_count: int
    applied_count: int
    commit_count: int
    stopped_reason: Optional[str] = None
    pr_url: Optional[str] = None
    created_at: datetime
    updated_at: datetime


class AIFixBatchCreateResponse(BaseModel):
    batch: Optional[AIFixBatchResponse] = None
    eligible_count: int
    selected_count: int
    queued_task_ids: List[str] = Field(default_factory=list)
    preview_only: bool = False
    cards: List[str] = Field(default_factory=list)


class AIFixBatchDetailResponse(BaseModel):
    batch: AIFixBatchResponse
    items: List[AIFixBatchItemResponse] = Field(default_factory=list)


class AIFixBatchCancelResponse(BaseModel):
    batch_id: str
    status: str


class EngineBacklogCreateRequest(BaseModel):
    title: str = Field(..., min_length=1, max_length=200)
    description: str = ""
    mechanic: str = ""
    source_task_id: Optional[str] = None


class EngineBacklogResponse(BaseModel):
    id: str
    title: str
    description: str
    mechanic: str
    status: str
    source_task_id: Optional[str] = None
    created_by: Optional[str] = None
    created_at: datetime
    updated_at: datetime


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


# ── Game Recording ─────────────────────────────────────────────────

class BugReportRequest(BaseModel):
    """Client uploads a game recording bundle for bug reporting."""
    description: str = ""
    recording: Dict[str, Any]  # Full client-side recording bundle


class BugReportResponse(BaseModel):
    report_id: str
    status: str = "submitted"


class RecordingResponse(BaseModel):
    """Summary of a saved headless game recording."""
    id: str
    game_mode: str
    total_steps: int
    has_tensors: bool
    created_at: datetime

    model_config = {"from_attributes": True}


class RecordingSaveResponse(BaseModel):
    recording_id: str
    status: str = "saved"


# ── Replay ───────────────────────────────────────────────────────────────

class ReplayRequest(BaseModel):
    """Create a replay session from a recording dict."""
    recording: Dict[str, Any]
    verify: bool = False


class SeekRequest(BaseModel):
    """Jump to a specific step in a replay session."""
    step: int


class ReplayCreateResponse(BaseModel):
    """Response from creating a replay session."""
    replay_id: str
    total_steps: int
    initial_state: Dict[str, Any]


class ReplayStepResponse(BaseModel):
    """Response from a replay step or seek."""
    step_number: int
    action_id: int
    player_id: int
    phase_before: str
    phase_after: str
    memory_before: int
    memory_after: int
    turn_number: int
    is_game_over: bool
    winner_id: Optional[int] = None
    state: Dict[str, Any]
    verification_ok: Optional[bool] = None
    verification_errors: List[str] = []
