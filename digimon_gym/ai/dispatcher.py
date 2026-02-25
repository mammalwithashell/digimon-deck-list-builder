"""AI task dispatcher: prompt assembly, retrieval, model invocation."""

from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from digimon_gym.ai.autofix_apply import (
    build_file_context_for_task,
    get_allowed_roots_for_scope,
    get_primary_script_path,
)
from digimon_gym.ai.client import ANTHROPIC_PRICING, MODEL_PRICING, LLMClient, create_llm_client
from digimon_gym.ai.contracts import (
    EngineCapabilityOutput,
    QATriageOutput,
    ScriptAutofixOutput,
    ScriptFidelityOutput,
)
from digimon_gym.ai.prompts import (
    build_engine_capability_messages,
    build_qa_triage_messages,
    build_script_autofix_messages,
    build_script_fidelity_messages,
)
from digimon_gym.ai.retrieval import LocalRAGIndex


PROJECT_ROOT = Path(__file__).resolve().parents[2]
CARDS_JSON_PATH = PROJECT_ROOT / "digimon_gym" / "engine" / "data" / "cards.json"
SCRIPTS_ROOT = PROJECT_ROOT / "digimon_gym" / "engine" / "data" / "scripts"


@dataclass
class DispatchOutcome:
    model_name: str
    result: dict[str, Any]
    sanitized_input: dict[str, Any]
    retrieval_refs: list[dict[str, Any]]
    input_tokens: int
    output_tokens: int
    cost_actual: float


def _load_cards_index() -> dict[str, dict]:
    if not CARDS_JSON_PATH.exists():
        return {}
    raw = json.loads(CARDS_JSON_PATH.read_text(encoding="utf-8"))
    if isinstance(raw, dict):
        return raw
    return {entry.get("card_id", ""): entry for entry in raw}


def _derive_set_and_module(card_id: str) -> tuple[str, str]:
    normalized = card_id.replace("-", "_").lower()
    if "_" not in normalized:
        return normalized, normalized
    set_id = normalized.split("_", 1)[0]
    return set_id, normalized


def _read_generated_script(set_id: str, module_name: str) -> str:
    path = SCRIPTS_ROOT / "generated" / set_id / f"{module_name}.py"
    if not path.exists():
        return ""
    return path.read_text(encoding="utf-8")


def _card_text_from_meta(meta: dict) -> str:
    return "\n".join(
        [
            meta.get("effect_description_eng", "") or "",
            meta.get("inherited_effect_description_eng", "") or "",
            meta.get("security_effect_description_eng", "") or "",
        ]
    ).strip()


class TaskDispatcher:
    def __init__(self, *, rag_index: LocalRAGIndex | None = None, client: LLMClient | None = None):
        self.rag_index = rag_index or LocalRAGIndex()
        self.client = client or create_llm_client()
        self.cards_index = _load_cards_index()

    def estimate_cost(self, task_type: str, payload: dict[str, Any], model_name: str | None = None) -> float:
        return self.client.estimate_cost(task_type, payload, model_name=model_name)

    def run(self, task_type: str, payload: dict[str, Any], model_name: str | None = None) -> DispatchOutcome:
        if task_type == "review_batch":
            return self._run_review_batch(payload, model_name=model_name)
        if task_type == "qa_analysis":
            return self._run_qa_analysis(payload, model_name=model_name)
        if task_type == "engine_audit":
            return self._run_engine_audit(payload, model_name=model_name)
        if task_type == "script_autofix":
            return self._run_script_autofix(payload, model_name=model_name)
        raise ValueError(f"Unsupported task_type: {task_type}")

    def _run_review_batch(self, payload: dict[str, Any], model_name: str | None) -> DispatchOutcome:
        cards_payload = payload.get("cards", [])
        if not isinstance(cards_payload, list) or not cards_payload:
            raise ValueError("review_batch payload must include non-empty 'cards' list")

        results: dict[str, Any] = {"cards": {}}
        sanitized_cards: list[dict[str, Any]] = []
        retrieval_refs: list[dict[str, Any]] = []
        input_tokens = 0
        output_tokens = 0
        resolved_model = ""

        for card in cards_payload:
            if isinstance(card, str):
                card_id = card
                set_id, module_name = _derive_set_and_module(card_id)
            elif isinstance(card, dict):
                card_id = str(card.get("card_id", "")).strip()
                set_id = str(card.get("set_id", "")).strip().lower() or _derive_set_and_module(card_id)[0]
                module_name = str(card.get("module_name", "")).strip().lower() or _derive_set_and_module(card_id)[1]
            else:
                continue

            if not card_id:
                continue

            card_meta = self.cards_index.get(card_id, {})
            card_text = _card_text_from_meta(card_meta)
            generated_script = _read_generated_script(set_id, module_name)
            query = f"{card_id} {card_meta.get('card_name_eng', '')} {card_text[:350]}"
            context = self.rag_index.retrieve(query, k=6)
            retrieval_refs.extend(
                {
                    "card_id": card_id,
                    "chunk_id": chunk.get("chunk_id"),
                    "source": chunk.get("source"),
                    "score": chunk.get("score"),
                }
                for chunk in context
            )

            system_prompt, user_prompt = build_script_fidelity_messages(
                card_id=card_id,
                card_text=card_text or "No card text available.",
                generated_script=generated_script or "No generated script found.",
                context_chunks=context,
            )
            run = self.client.run_structured(
                task_type="review_batch",
                system_prompt=system_prompt,
                user_prompt=user_prompt,
                schema_model=ScriptFidelityOutput,
                model_name=model_name,
            )
            resolved_model = run.model_name
            input_tokens += run.input_tokens
            output_tokens += run.output_tokens
            results["cards"][card_id] = run.output
            sanitized_cards.append(
                {
                    "card_id": card_id,
                    "set_id": set_id,
                    "module_name": module_name,
                }
            )

        return DispatchOutcome(
            model_name=resolved_model or self.client.resolve_model("review_batch", model_name),
            result=results,
            sanitized_input={"cards": sanitized_cards},
            retrieval_refs=retrieval_refs,
            input_tokens=input_tokens,
            output_tokens=output_tokens,
            cost_actual=self._cost_from_usage(resolved_model or self.client.resolve_model("review_batch", model_name), input_tokens, output_tokens),
        )

    def _run_qa_analysis(self, payload: dict[str, Any], model_name: str | None) -> DispatchOutcome:
        report_text = str(payload.get("report_text", "")).strip()
        card_id = str(payload.get("card_id", "")).strip()
        card_meta = self.cards_index.get(card_id, {})
        card_text = _card_text_from_meta(card_meta)
        script_text = str(payload.get("script_text", "")).strip()
        engine_version = str(payload.get("engine_version", "unknown"))
        query = f"{card_id} {report_text[:300]}"
        context = self.rag_index.retrieve(query, k=3)
        system_prompt, user_prompt = build_qa_triage_messages(
            report_text=report_text,
            card_text=card_text,
            script_text=script_text,
            engine_version=engine_version,
            context_chunks=context,
        )
        run = self.client.run_structured(
            task_type="qa_analysis",
            system_prompt=system_prompt,
            user_prompt=user_prompt,
            schema_model=QATriageOutput,
            model_name=model_name,
        )
        return DispatchOutcome(
            model_name=run.model_name,
            result=run.output,
            sanitized_input={"card_id": card_id, "engine_version": engine_version},
            retrieval_refs=context,
            input_tokens=run.input_tokens,
            output_tokens=run.output_tokens,
            cost_actual=self._cost_from_usage(run.model_name, run.input_tokens, run.output_tokens),
        )

    def _run_engine_audit(self, payload: dict[str, Any], model_name: str | None) -> DispatchOutcome:
        mechanics = payload.get("mechanics")
        if isinstance(mechanics, str):
            mechanics = [mechanics]
        if not isinstance(mechanics, list) or not mechanics:
            raise ValueError("engine_audit payload must include 'mechanics' list")

        outputs = []
        retrieval_refs = []
        input_tokens = 0
        output_tokens = 0
        resolved_model = ""
        for mechanic in mechanics:
            mechanic_text = str(mechanic).strip()
            if not mechanic_text:
                continue
            context = self.rag_index.retrieve(mechanic_text, k=5)
            system_prompt, user_prompt = build_engine_capability_messages(
                mechanic=mechanic_text,
                context_chunks=context,
            )
            run = self.client.run_structured(
                task_type="engine_audit",
                system_prompt=system_prompt,
                user_prompt=user_prompt,
                schema_model=EngineCapabilityOutput,
                model_name=model_name,
            )
            resolved_model = run.model_name
            outputs.append(run.output)
            retrieval_refs.extend(context)
            input_tokens += run.input_tokens
            output_tokens += run.output_tokens

        return DispatchOutcome(
            model_name=resolved_model or self.client.resolve_model("engine_audit", model_name),
            result={"mechanics": outputs},
            sanitized_input={"mechanics": [str(m) for m in mechanics]},
            retrieval_refs=retrieval_refs,
            input_tokens=input_tokens,
            output_tokens=output_tokens,
            cost_actual=self._cost_from_usage(resolved_model or self.client.resolve_model("engine_audit", model_name), input_tokens, output_tokens),
        )

    def _run_script_autofix(self, payload: dict[str, Any], model_name: str | None) -> DispatchOutcome:
        card_id = str(payload.get("card_id", "")).strip().upper()
        set_id = str(payload.get("set_id", "")).strip().lower()
        module_name = str(payload.get("module_name", "")).strip().lower()
        scope_profile = str(payload.get("scope_profile", "script")).strip().lower() or "script"
        issue_description = str(payload.get("issue_description", "")).strip()
        if not card_id or not set_id or not module_name:
            raise ValueError("script_autofix payload must include card_id, set_id, module_name")

        card_meta = self.cards_index.get(card_id, {})
        card_text = _card_text_from_meta(card_meta)
        file_contexts = build_file_context_for_task(
            set_id=set_id,
            module_name=module_name,
            scope_profile=scope_profile,
        )
        allowed_roots = get_allowed_roots_for_scope(scope_profile)
        primary_path = get_primary_script_path(set_id, module_name)
        allowed_paths = [ctx.get("path", "") for ctx in file_contexts if ctx.get("path")]
        if primary_path not in allowed_paths:
            allowed_paths.insert(0, primary_path)

        query = f"{card_id} {issue_description} {card_text[:300]}"
        context = self.rag_index.retrieve(query, k=6)
        system_prompt, user_prompt = build_script_autofix_messages(
            card_id=card_id,
            issue_description=issue_description or "No issue description supplied.",
            scope_profile=scope_profile,
            allowed_paths=allowed_paths,
            file_contexts=file_contexts,
            context_chunks=context,
        )
        run = self.client.run_structured(
            task_type="script_autofix",
            system_prompt=system_prompt,
            user_prompt=user_prompt,
            schema_model=ScriptAutofixOutput,
            model_name=model_name,
        )
        return DispatchOutcome(
            model_name=run.model_name,
            result=run.output,
            sanitized_input={
                "card_id": card_id,
                "set_id": set_id,
                "module_name": module_name,
                "scope_profile": scope_profile,
                "allowed_roots": allowed_roots,
            },
            retrieval_refs=context,
            input_tokens=run.input_tokens,
            output_tokens=run.output_tokens,
            cost_actual=self._cost_from_usage(run.model_name, run.input_tokens, run.output_tokens),
        )

    @staticmethod
    def _cost_from_usage(model_name: str, input_tokens: int, output_tokens: int) -> float:
        in_price, out_price = MODEL_PRICING.get(model_name) or ANTHROPIC_PRICING.get(model_name, (1.0, 4.0))
        return (
            (input_tokens / 1_000_000.0) * in_price
            + (output_tokens / 1_000_000.0) * out_price
        )
