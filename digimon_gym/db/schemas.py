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


class ApproveSetIssuesRequest(BaseModel):
    set_id: str = Field(..., min_length=1, max_length=32)
    status_filter: str = Field("new", pattern=r"^(new|approved_for_ai|rejected|resolved)$")


class ApproveSetIssuesResponse(BaseModel):
    set_id: str
    matched_count: int
    approved_count: int
    skipped_count: int
    issue_ids: List[str] = Field(default_factory=list)


class QueueSetIssueFixesRequest(BaseModel):
    set_id: str = Field(..., min_length=1, max_length=32)
    queue_mode: Literal["approved_only", "new_and_approved"] = "approved_only"


class QueueSetIssueFixesResponse(BaseModel):
    set_id: str
    queue_mode: Literal["approved_only", "new_and_approved"]
    matched_issues: int
    queued_tasks: int
    skipped_duplicate_cards: int
    skipped_existing_task_cards: int
    queued_task_ids: List[str] = Field(default_factory=list)
    queued_card_ids: List[str] = Field(default_factory=list)
    skipped_cards: List[str] = Field(default_factory=list)


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
    set_run_id: Optional[str] = None
    set_id: Optional[str] = None
    work_id: str
    work_type: Literal["set_run", "batch", "task"]
    run_mode: Optional[str] = None
    scope_profile: Optional[str] = None
    started_at: Optional[datetime] = None
    completed_at: Optional[datetime] = None
    created_at: datetime
    updated_at: datetime
    apply_status: Optional[str] = None


class AITaskRetryResponse(BaseModel):
    task_id: str
    status: str


class AITaskApplyFixRequest(BaseModel):
    force: bool = False


class AITaskApplyFixResponse(BaseModel):
    task_id: str
    status: str
    applied_files: List[str] = Field(default_factory=list)
    commit_sha: Optional[str] = None
    pr_url: Optional[str] = None
    force_applied: bool = False


class AIFixApplyAuditResponse(BaseModel):
    id: str
    ai_task_id: Optional[str] = None
    batch_id: Optional[str] = None
    card_id: str
    scope_profile: str
    run_mode: str
    applied_files: List[str] = Field(default_factory=list)
    commit_sha: Optional[str] = None
    pr_url: Optional[str] = None
    status: str
    error_text: Optional[str] = None
    created_by: Optional[str] = None
    created_at: datetime


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
    set_run_id: Optional[str] = None
    work_id: Optional[str] = None
    work_type: Optional[Literal["set_run", "batch", "task"]] = None
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


class AISetRunCreateRequest(BaseModel):
    set_id: str = Field(..., min_length=1, max_length=32)
    run_mode: Literal["pr", "main"] = "pr"
    scope_profile: Literal["script", "script_engine", "script_engine_transpiler"] = "script"
    model_name: Optional[str] = None
    max_total_cost_usd: float = Field(5.0, ge=0.0)
    failure_rate_stop: float = Field(0.3, ge=0.0, le=1.0)
    max_fix_tasks: int = Field(0, ge=0, le=5000)
    max_fix_iterations: int = Field(3, ge=1, le=20)
    score_threshold: Optional[float] = Field(None, ge=0.0, le=1.0)


class AISetRunItemResponse(BaseModel):
    id: str
    set_run_id: str
    card_id: str
    set_id: str
    module_name: str
    review_faithful: Optional[bool] = None
    issue_id: Optional[str] = None
    qa_task_id: Optional[str] = None
    review_task_id: Optional[str] = None
    fix_task_id: Optional[str] = None
    fix_apply_status: str
    transpile_score: Optional[float] = None
    retranspile_task_id: Optional[str] = None
    commit_sha: Optional[str] = None
    pr_url: Optional[str] = None
    error_text: Optional[str] = None
    created_at: datetime
    updated_at: datetime

    model_config = {"from_attributes": True}


class AISetRunResponse(BaseModel):
    id: str
    set_id: str
    status: str
    stage: str
    run_mode: str
    scope_profile: str
    model_name: Optional[str] = None
    created_by: Optional[str] = None
    max_total_cost_usd: float
    failure_rate_stop: float
    max_fix_tasks: int
    fix_iteration: int = 0
    max_fix_iterations: int = 3
    qa_total: int
    qa_completed: int
    qa_failed: int
    review_total: int
    review_completed: int
    review_failed: int
    fix_total: int
    fix_completed: int
    fix_failed: int
    fix_applied: int
    score_threshold: Optional[float] = None
    retranspile_total: int = 0
    retranspile_completed: int = 0
    retranspile_failed: int = 0
    stopped_reason: Optional[str] = None
    pr_url: Optional[str] = None
    created_at: datetime
    updated_at: datetime
    completed_at: Optional[datetime] = None

    model_config = {"from_attributes": True}


class AISetRunDetailResponse(BaseModel):
    run: AISetRunResponse
    items: List[AISetRunItemResponse] = Field(default_factory=list)
    pr_queue: List[AISetRunItemResponse] = Field(default_factory=list)


class AISetRunCancelResponse(BaseModel):
    set_run_id: str
    status: str


class AITranspilerLearnCreateRequest(BaseModel):
    source_set_run_id: str
    min_cluster_size: int = Field(3, ge=1, le=50)


class AITranspilerLearnResponse(BaseModel):
    id: str
    source_set_run_id: Optional[str] = None
    status: str
    clusters_found: int
    patches_proposed: int
    pr_url: Optional[str] = None
    created_at: datetime
    completed_at: Optional[datetime] = None

    model_config = {"from_attributes": True}


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


# ── Agent Training ─────────────────────────────────────────────────────

class AgentCreateRequest(BaseModel):
    name: str = Field(..., min_length=1, max_length=100)
    archetype: str = Field(..., min_length=1, max_length=100)
    algorithm: Literal["mlp", "lstm"] = "mlp"
    deck: List[str]
    deck_source: Optional[str] = None
    deck_pool_id: Optional[str] = None


class AgentUpdateRequest(BaseModel):
    name: Optional[str] = Field(None, min_length=1, max_length=100)
    archetype: Optional[str] = Field(None, min_length=1, max_length=100)


class AgentResponse(BaseModel):
    id: str
    name: str
    archetype: str
    algorithm: str
    weights_path: str
    deck_source: Optional[str] = None
    deck_pool_id: Optional[str] = None
    total_games: int
    total_wins: int
    total_losses: int
    total_draws: int
    win_rate: float
    total_timesteps_trained: int
    owner_id: Optional[str] = None
    created_at: datetime
    updated_at: datetime


class TrainingJobCreateRequest(BaseModel):
    job_type: Literal["train_vs_greedy", "train_vs_random", "train_vs_agent", "evaluate"]
    agent_id: str
    opponent_agent_id: Optional[str] = None
    total_timesteps: int = Field(100_000, ge=1000, le=10_000_000)
    total_games: int = Field(0, ge=0, le=100_000)
    config: Dict[str, Any] = Field(default_factory=dict)


class TrainingJobResponse(BaseModel):
    id: str
    job_type: str
    status: str
    agent_id: Optional[str] = None
    opponent_agent_id: Optional[str] = None
    opponent_type: Optional[str] = None
    gauntlet_id: Optional[str] = None
    gauntlet_stage: Optional[int] = None
    total_games: int
    total_timesteps: int
    completed_games: int
    wins: int
    losses: int
    draws: int
    win_rate: Optional[float] = None
    config: Dict[str, Any] = Field(default_factory=dict)
    result: Optional[Dict[str, Any]] = None
    error_text: Optional[str] = None
    worker_id: Optional[str] = None
    device: Optional[str] = None
    created_by: Optional[str] = None
    started_at: Optional[datetime] = None
    completed_at: Optional[datetime] = None
    created_at: datetime
    updated_at: datetime


class GauntletParticipantInput(BaseModel):
    archetype_name: str = Field(..., min_length=1, max_length=100)
    deck: List[str]
    deck_source: Optional[str] = None
    deck_pool_id: Optional[str] = None
    role: Literal["core", "exploiter"] = "core"
    meta_share: float = Field(0.0, ge=0.0, le=1.0)
    threat_index: float = Field(0.0, ge=0.0)


class GauntletCreateRequest(BaseModel):
    name: str = Field(..., min_length=1, max_length=100)
    participants: List[GauntletParticipantInput] = Field(..., min_length=2, max_length=12)
    stage1_games: int = Field(1000, ge=100, le=100_000)
    stage2_games: int = Field(5000, ge=100, le=100_000)
    stage3_games_per_matchup: int = Field(100, ge=10, le=10_000)
    config: Dict[str, Any] = Field(default_factory=dict)


class GauntletParticipantResponse(BaseModel):
    id: str
    slot: int
    role: str
    agent_id: Optional[str] = None
    archetype_name: str
    deck_source: Optional[str] = None
    deck_pool_id: Optional[str] = None
    meta_share: float
    threat_index: float
    tournament_win_rate: Optional[float] = None
    expected_meta_win_rate: Optional[float] = None


class GauntletResponse(BaseModel):
    id: str
    name: str
    status: str
    current_stage: int
    stage1_games: int
    stage2_games: int
    stage3_games_per_matchup: int
    participants: List[GauntletParticipantResponse] = Field(default_factory=list)
    error_text: Optional[str] = None
    created_by: Optional[str] = None
    started_at: Optional[datetime] = None
    completed_at: Optional[datetime] = None
    created_at: datetime
    updated_at: datetime


class GauntletDetailResponse(BaseModel):
    gauntlet: GauntletResponse
    jobs: List[TrainingJobResponse] = Field(default_factory=list)
    matchup_matrix: Optional[Dict[str, Dict[str, float]]] = None
    tournament_rankings: Optional[List[Dict[str, Any]]] = None


class MatchupResultResponse(BaseModel):
    id: str
    gauntlet_id: str
    agent_a_id: Optional[str] = None
    agent_b_id: Optional[str] = None
    games_played: int
    a_wins: int
    b_wins: int
    draws: int
    a_win_rate: float


# ── Deck Pools ─────────────────────────────────────────────────────────

class DeckPoolCreateRequest(BaseModel):
    name: str = Field(..., min_length=1, max_length=100)
    archetype: str = Field(..., min_length=1, max_length=100)
    base_deck: List[str]
    egg_deck: List[str] = Field(default_factory=list)
    vary_eggs: bool = False
    side_cards: List[str] = Field(default_factory=list)
    core_overrides: Dict[str, bool] = Field(default_factory=dict)
    generation_mode: Literal["eager", "hybrid"] = "eager"
    target_variant_count: int = Field(4, ge=1, le=50)
    seed: Optional[int] = None
    hybrid_max_dynamic: int = Field(10, ge=0, le=100)


class DeckPoolUpdateRequest(BaseModel):
    name: Optional[str] = Field(None, min_length=1, max_length=100)
    archetype: Optional[str] = Field(None, min_length=1, max_length=100)
    side_cards: Optional[List[str]] = None
    core_overrides: Optional[Dict[str, bool]] = None
    generation_mode: Optional[Literal["eager", "hybrid"]] = None
    target_variant_count: Optional[int] = Field(None, ge=1, le=50)
    seed: Optional[int] = None
    vary_eggs: Optional[bool] = None
    hybrid_max_dynamic: Optional[int] = Field(None, ge=0, le=100)


class CoreAnalysisResponse(BaseModel):
    core_cards: Dict[str, int]
    flex_cards: Dict[str, int]
    total_main: int
    total_egg: int


class DeckPoolVariantResponse(BaseModel):
    index: int
    deck: List[str]
    changes_from_base: Dict[str, int]


class DeckPoolResponse(BaseModel):
    id: str
    name: str
    archetype: str
    base_deck: List[str]
    egg_deck: List[str]
    vary_eggs: bool
    core_overrides: Dict[str, bool]
    side_cards: List[str]
    generation_mode: str
    target_variant_count: int
    seed: Optional[int] = None
    variant_count: int
    hybrid_max_dynamic: int
    owner_id: Optional[str] = None
    created_at: datetime
    updated_at: datetime


class DeckPoolDetailResponse(BaseModel):
    pool: DeckPoolResponse
    core_analysis: CoreAnalysisResponse
    variants: List[DeckPoolVariantResponse] = Field(default_factory=list)


class DeckPoolGenerateRequest(BaseModel):
    count: Optional[int] = None
    seed: Optional[int] = None


class ArchetypeInfoResponse(BaseModel):
    name: str
    display_card_id: Optional[str] = None
    primary_color: Optional[str] = None
    meta_share: float = 0.0
    threat_index: float = 0.0
    decklists_count: int = 0
    sample_decklist: List[str] = Field(default_factory=list)
    scope: str = "global"
    local_times_played: Optional[int] = None


class StoreInfoResponse(BaseModel):
    store_id: int
    name: str
    city: str
    state: str
    scene_id: Optional[int] = None
    tournament_count: int = 0


class SceneInfoResponse(BaseModel):
    scene_id: int
    name: str
    display_name: str
    tournament_count: int = 0


# ── Patch Notes ─────────────────────────────────────────────────────────

class KnownIssueCreateRequest(BaseModel):
    title: str = Field(..., min_length=1, max_length=200)
    description: str = Field(default="", max_length=4000)


class KnownIssueUpdateRequest(BaseModel):
    title: Optional[str] = Field(None, min_length=1, max_length=200)
    description: Optional[str] = Field(None, max_length=4000)


class KnownIssueResponse(BaseModel):
    id: str
    title: str
    description: str
    created_at: datetime
    updated_at: datetime


class ReleaseCreateRequest(BaseModel):
    version: str = Field(..., min_length=1, max_length=64)
    release_date: datetime
    title: Optional[str] = Field(None, max_length=200)
    added: List[str] = Field(default_factory=list)
    changed: List[str] = Field(default_factory=list)
    fixed: List[str] = Field(default_factory=list)


class ReleaseUpdateRequest(BaseModel):
    version: Optional[str] = Field(None, min_length=1, max_length=64)
    release_date: Optional[datetime] = None
    title: Optional[str] = Field(None, max_length=200)
    added: Optional[List[str]] = None
    changed: Optional[List[str]] = None
    fixed: Optional[List[str]] = None


class ReleaseResponse(BaseModel):
    id: str
    version: str
    release_date: datetime
    title: Optional[str]
    added: List[str]
    changed: List[str]
    fixed: List[str]
    created_at: datetime
    updated_at: datetime


class PatchNotesResponse(BaseModel):
    known_issues: List[KnownIssueResponse]
    releases: List[ReleaseResponse]
