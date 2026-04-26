"""Structured output contracts for AI agents."""

from __future__ import annotations

from typing import List, Literal

from pydantic import BaseModel, ConfigDict, Field


class ScriptFidelityOutput(BaseModel):
    model_config = ConfigDict(extra="forbid")

    faithful_to_card_text: bool
    engine_supported: bool
    issues: List[str] = Field(default_factory=list)
    suggested_fixes: List[str] = Field(default_factory=list)
    engine_requests: List[str] = Field(default_factory=list)


class ScriptAutofixFileEdit(BaseModel):
    model_config = ConfigDict(extra="forbid")

    path: str
    expected_hash: str
    new_content: str
    reason: str = ""


class ScriptAutofixOutput(BaseModel):
    model_config = ConfigDict(extra="forbid")

    summary: str = ""
    edits: List[ScriptAutofixFileEdit] = Field(default_factory=list)


class TranspilerGap(BaseModel):
    model_config = ConfigDict(extra="forbid")

    gap_id: str
    kind: str
    label: str
    api_count: int = 0
    dcgo_count: int = 0
    severity: Literal["low", "medium", "high", "critical"] = "medium"
    evidence: List[str] = Field(default_factory=list)


class TranspilerValidationMetrics(BaseModel):
    model_config = ConfigDict(extra="forbid")

    api_uncovered_keyword_count: int = 0
    api_uncovered_action_count: int = 0
    dcgo_unmapped_action_count: int = 0
    dcgo_no_action_effect_count: int = 0
    merged_gap_count: int = 0


class TranspilerRecommendation(BaseModel):
    model_config = ConfigDict(extra="forbid")

    recommendation_id: str = ""
    title: str
    recommendation_kind: str
    priority: Literal["low", "medium", "high", "critical"] = "medium"
    confidence: float = Field(0.0, ge=0.0, le=1.0)
    rationale: str = ""
    expected_impact: str = ""
    suggested_files: List[str] = Field(default_factory=list)
    patch_hints: List[str] = Field(default_factory=list)
    evidence: List[str] = Field(default_factory=list)
    auto_apply_eligible: bool = False


class TranspilerAuditOutput(BaseModel):
    model_config = ConfigDict(extra="forbid")

    set_id: str
    summary: str = ""
    recommendations: List[TranspilerRecommendation] = Field(default_factory=list)
    merged_gaps: List[TranspilerGap] = Field(default_factory=list)
    validation_metrics: TranspilerValidationMetrics


class TranspilerAutofixOutput(BaseModel):
    model_config = ConfigDict(extra="forbid")

    summary: str = ""
    recommendation_id: str = ""
    edits: List[ScriptAutofixFileEdit] = Field(default_factory=list)


class EngineCapabilityOutput(BaseModel):
    model_config = ConfigDict(extra="forbid")

    mechanic: str
    supported: bool
    notes: str = ""


class QATriageOutput(BaseModel):
    model_config = ConfigDict(extra="forbid")

    likely_root_cause: str
    reproducibility: Literal["low", "medium", "high"]
    classification: Literal["engine", "script", "invalid_report", "unknown"]
    suggested_debugging_area: str


class LLMTranspileOutput(BaseModel):
    """Output from LLM-based retranspilation of a card script."""

    model_config = ConfigDict(extra="forbid")

    script_content: str
    effects_implemented: list[str]
    effects_skipped: list[str]
    engine_gaps: list[str]
    reasoning: str


class TranspilerPatchSuggestion(BaseModel):
    """A suggested change to the transpiler codebase."""

    model_config = ConfigDict(extra="forbid")

    target_file: str
    description: str
    before_snippet: str
    after_snippet: str
    cards_affected: list[str]


class TranspilerLearnOutput(BaseModel):
    """Output from transpiler pattern learning."""

    model_config = ConfigDict(extra="forbid")

    cluster_summary: str
    patches: list[TranspilerPatchSuggestion]
    estimated_cards_fixed: int
    confidence: Literal["low", "medium", "high"]
